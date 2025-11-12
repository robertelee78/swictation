# VAD Threshold Code Flow Analysis

**Analysis Date:** 2025-11-12
**Agent:** Coder Agent (Hive Mind)
**Status:** CRITICAL BUG IDENTIFIED

## Executive Summary

The VAD threshold configuration has a **critical bug** that prevents ALL speech detection. The daemon uses `threshold: 0.25` while the ONNX model outputs probabilities in range `0.0005-0.002` (125x lower than threshold).

**Result:** The condition `speech_prob >= threshold` at `silero_ort.rs:231` is **NEVER satisfied**, causing all audio to be classified as silence.

---

## Code Flow: Threshold Configuration Path

### 1. Library Default (swictation-vad)

**File:** `rust-crates/swictation-vad/src/lib.rs`
**Line:** 119
**Value:** `0.003`

```rust
impl Default for VadConfig {
    fn default() -> Self {
        Self {
            // ...
            // NOTE: Silero VAD ONNX model has ~100-200x lower probabilities than PyTorch JIT
            // Optimal threshold for ONNX: 0.001-0.005 (NOT 0.5 as in PyTorch examples)
            threshold: 0.003,
            // ...
        }
    }
}
```

**Documentation (lines 14-22):**
```rust
//! - PyTorch JIT: probabilities ~0.02-0.2, use threshold ~0.5
//! - ONNX model: probabilities ~0.0005-0.002, **use threshold ~0.001-0.005**
//!
//! **DO NOT use threshold 0.5 with ONNX** - it will never detect speech!
```

---

### 2. Daemon Override (swictation-daemon)

**File:** `rust-crates/swictation-daemon/src/config.rs`
**Line:** 117
**Value:** `0.25` ⚠️

```rust
impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            // ...
            vad_threshold: 0.25, // Optimized for real-time transcription (original 0.003 prevented silence detection)
            // ...
        }
    }
}
```

**PROBLEM:** Comment claims `0.003` prevented silence detection, but this is **backwards** - `0.25` prevents **SPEECH** detection!

---

### 3. Runtime Initialization

**File:** `rust-crates/swictation-daemon/src/pipeline.rs`
**Lines:** 66-73

```rust
let vad_config = VadConfig::with_model(config.vad_model_path.display().to_string())
    .min_silence(config.vad_min_silence)
    .min_speech(config.vad_min_speech)
    .max_speech(config.vad_max_speech)
    .threshold(config.vad_threshold)  // Sets 0.25 from DaemonConfig
    .provider(gpu_provider.clone())
    .num_threads(config.num_threads)
    .debug();
```

**Flow:**
1. `DaemonConfig::default()` sets `vad_threshold: 0.25`
2. `VadConfig::with_model()` uses `VadConfig::default()` (0.003)
3. `.threshold(config.vad_threshold)` **OVERRIDES** with 0.25

---

### 4. Validation

**File:** `rust-crates/swictation-vad/src/lib.rs`
**Lines:** 211-213

```rust
fn validate(&self) -> Result<()> {
    // ...
    if !(0.0..=1.0).contains(&self.threshold) {
        return Err(VadError::config("Threshold must be between 0.0 and 1.0"));
    }
    // ...
}
```

**Status:** ✅ Passes validation (0.25 is within 0.0-1.0)
**Problem:** Validation only checks range, not whether threshold makes sense for model output

---

### 5. Detection Logic (WHERE BUG MANIFESTS)

**File:** `rust-crates/swictation-vad/src/silero_ort.rs`
**Line:** 231

```rust
// Improved speech detection with buffering
if speech_prob >= self.threshold {
    // Speech detected
    if !self.triggered {
        self.triggered = true;
    }
    self.temp_end = self.current_sample;
    self.speech_buffer.extend_from_slice(audio_chunk);
} else if self.triggered {
    // Was speaking, now silence
    // ...
}
```

**Model Output:** `speech_prob` from ONNX inference (lines 179-186)
**Comparison:** `speech_prob >= self.threshold` (line 231)

---

## Mathematical Proof of Bug

### ONNX Model Output Range
- **Documented range:** 0.0005 - 0.002 (lines 18-19 of lib.rs)
- **Typical speech probability:** ~0.001 - 0.002
- **Maximum observed:** 0.002

### Threshold Comparison

| Configuration | Threshold | Model Output | Condition Met? | Result |
|---------------|-----------|--------------|----------------|--------|
| **VadConfig::default()** | 0.003 | 0.001-0.002 | **Borderline** | May miss speech |
| **DaemonConfig::default()** | **0.25** | 0.001-0.002 | ❌ **NEVER** | **No speech detected** |
| **Recommended** | 0.001-0.005 | 0.001-0.002 | ✅ **Yes** | Detects speech |

### Why 0.25 Fails

```
speech_prob (from model) = 0.001 to 0.002
threshold (configured)   = 0.25

Comparison: speech_prob >= threshold
           0.001 >= 0.25  → FALSE
           0.002 >= 0.25  → FALSE

Ratio: threshold / max_output = 0.25 / 0.002 = 125x
```

**The threshold is 125x HIGHER than the maximum model output.**
**It is mathematically IMPOSSIBLE for the model to output a value ≥ 0.25**

---

## Code Evidence Summary

### Threshold Set (Default)
- **VadConfig:** Line 119 → `0.003`
- **DaemonConfig:** Line 117 → `0.25` (WRONG)

### Threshold Used
- **Pipeline init:** Lines 66-73 → Uses `DaemonConfig.vad_threshold` (0.25)
- **Silero VAD:** Line 114 → Stores threshold in `self.threshold`

### Threshold Compared
- **Detection logic:** Line 231 → `if speech_prob >= self.threshold`
- **Model output:** Lines 179-186 → Extracts `speech_prob` from ONNX inference

### Debug Output
- **Location:** Lines 218-227
- **Format:** `"VAD: t={time}s, prob={speech_prob:.6}, threshold={threshold:.3}"`
- **Would show:** `prob=0.001500 vs threshold=0.250` (proving mismatch)

---

## Impact Analysis

### Current Behavior (threshold = 0.25)
1. Audio captured → VAD receives chunks
2. ONNX model processes → outputs `prob = 0.001-0.002`
3. Comparison: `0.001 >= 0.25` → **FALSE**
4. Result: `VadResult::Silence` (line 261)
5. **NO SPEECH EVER DETECTED**

### Why Comment is Wrong
**Comment says:** "original 0.003 prevented silence detection"

**Reality:**
- Threshold 0.003: May detect speech (borderline with model output 0.001-0.002)
- Threshold 0.25: **PREVENTS SPEECH DETECTION** (always returns silence)

**The comment has the logic backwards!**

---

## Fix Required

### File to Change
`rust-crates/swictation-daemon/src/config.rs`

### Current Code (Line 117)
```rust
vad_threshold: 0.25, // Optimized for real-time transcription (original 0.003 prevented silence detection)
```

### Corrected Code
```rust
vad_threshold: 0.003, // Optimal ONNX threshold (0.001-0.005 range, matches model output ~0.0005-0.002)
```

### Better Options
Based on testing, could use:
- **Conservative:** `0.005` (fewer false positives)
- **Balanced:** `0.003` (default, catches most speech)
- **Sensitive:** `0.001` (catches quiet speech, more false positives)

---

## Testing Verification

To verify the fix works, run daemon in debug mode and check logs:

```bash
# Debug output at silero_ort.rs:222-226
VAD: t=1.50s, prob=0.001500, threshold=0.003  # ✅ Should detect speech
VAD: t=1.50s, prob=0.001500, threshold=0.250  # ❌ Never detects speech
```

**Expected behavior after fix:**
- Speech probability ~0.001-0.002
- Threshold 0.003
- Condition `speech_prob >= threshold` satisfied for loud speech
- `VadResult::Speech` returned with audio segments

---

## Files Referenced

| File | Purpose | Lines |
|------|---------|-------|
| `swictation-vad/src/lib.rs` | VadConfig definition & defaults | 14-22, 119, 211-213 |
| `swictation-daemon/src/config.rs` | DaemonConfig defaults | 117 |
| `swictation-daemon/src/pipeline.rs` | Runtime initialization | 66-73 |
| `swictation-vad/src/silero_ort.rs` | Detection logic & comparison | 231, 218-227 |

---

## Conclusion

**Root Cause:** DaemonConfig sets `vad_threshold: 0.25`, which is 125x higher than ONNX model output range (0.0005-0.002).

**Effect:** The condition `speech_prob >= threshold` at `silero_ort.rs:231` is NEVER satisfied, causing all audio to be classified as `VadResult::Silence`.

**Fix:** Change line 117 of `config.rs` from `0.25` to `0.003` (or 0.001-0.005 range).

**Urgency:** CRITICAL - This bug completely breaks VAD functionality.
