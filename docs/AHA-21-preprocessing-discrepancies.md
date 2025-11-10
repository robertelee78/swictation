# AHA #21: Audio Preprocessing Discrepancies Found!

**Date:** 2025-11-10
**Status:** Investigation Complete - Code Changes Needed
**Severity:** HIGH - May explain transcription quality issues

---

## 🎯 Executive Summary

Deep investigation comparing sherpa-onnx reference implementation with our Rust code revealed **critical audio preprocessing differences** that may explain why we're getting "mm", "mmhmm" instead of actual transcriptions.

**Bottom Line:** Our normalization approach was CORRECT, but we're using the **wrong window function**.

---

## 🔍 Investigation Timeline

### AHA #18: Initial Hypothesis (WRONG)
- **Hypothesis:** Missing fixed mean/std normalization from model training
- **Result:** **DISPROVEN** - Sherpa-onnx ALSO computes mean/std from input audio (not fixed values)

### AHA #19: GigaAM Detection Check
- **Finding:** Our 1.1B model is correctly identified as NON-GigaAM
- **Confirmed:** Should use `remove_dc_offset=false`, `preemph_coeff=0.97`

### AHA #20: Sherpa-ONNX Preprocessing Analysis
- **Discovery:** Sherpa-onnx uses different settings for GigaAM vs non-GigaAM models
- **For Parakeet-TDT (non-GigaAM):** Special preprocessing configuration

### **AHA #21: Window Function Mismatch (THIS IS THE PROBLEM!)**
- **Critical:** We use **Hann window**, sherpa-onnx uses **Povey window**
- **Impact:** Different spectral shaping → wrong encoder inputs → bad transcriptions

---

## 📊 Detailed Comparison

### 1. Window Function ❌ **MISMATCH**

| Implementation | Window Type | Formula |
|----------------|-------------|---------|
| **Sherpa-ONNX** | **Povey** | `w(n) = [0.5 - 0.5·cos(2πn/N)]^0.85` |
| **Our Code** | **Hann** | `w(n) = 0.5 - 0.5·cos(2πn/N)` |

**Location in our code:** `/opt/swictation/rust-crates/swictation-stt/src/audio.rs:335`

```rust
let window = hann_window(WIN_LENGTH);  // ❌ WRONG - should be Povey!
```

**Sherpa-ONNX defaults (features.h:42):**
```cpp
std::string window_type = "povey";  // Default for all non-GigaAM models
```

**Why this matters:**
- Povey window = Hann^0.85 (dulled curve)
- Different spectral shaping of frames
- Encoder trained on Povey-windowed features
- We feed Hann-windowed features → Out of distribution!

---

### 2. Preemphasis ✅ **CORRECT**

| Implementation | Coefficient | Location |
|----------------|-------------|----------|
| **Sherpa-ONNX** | 0.97 | Default (features.h:42) |
| **Our Code** | 0.97 | audio.rs:256 ✅ |

```rust
let preemphasized = apply_preemphasis(samples, 0.97);  // ✅ CORRECT
```

---

### 3. DC Offset Removal ✅ **CORRECT**

| Implementation | remove_dc_offset | Location |
|----------------|------------------|----------|
| **Sherpa-ONNX (Parakeet-TDT)** | **false** | offline-recognizer-transducer-nemo-impl.h:167 |
| **Our Code** | **false** (implicitly) | audio.rs:356-357 ✅ |

```rust
// NO DC offset removal for NeMo models!
buffer[i] = Complex::new(samples[start + i] * window[i], 0.0);  // ✅ CORRECT
```

---

### 4. Feature Normalization ✅ **CORRECT**

| Implementation | Method | Notes |
|----------------|--------|-------|
| **Sherpa-ONNX** | Per-feature from input | Computed per chunk |
| **Our Code** | Per-feature from input | audio.rs:299-319 ✅ |

Both compute mean/std from the input audio chunk - **NO FIXED VALUES from training!**

This was confirmed by reading sherpa-onnx/csrc/offline-stream.cc:299-312:
```cpp
ComputeMeanAndInvStd(p, num_frames, feature_dim, &mean, &inv_stddev);
// Computes mean/std FROM INPUT, not from fixed arrays!
```

---

### 5. is_librosa Flag

| Implementation | Value | Location |
|----------------|-------|----------|
| **Sherpa-ONNX (Parakeet-TDT)** | **true** | offline-recognizer-transducer-nemo-impl.h:167 |
| **Our Code** | N/A | Not explicitly set |

**Note:** This flag may affect mel filterbank construction in kaldi-native-fbank. Need to investigate impact.

---

## 🎯 Root Cause Analysis

### Why "mm", "mmhmm" Instead of Actual Speech?

**Theory:**
1. Model was trained with **Povey-windowed** mel-spectrograms
2. We feed **Hann-windowed** mel-spectrograms
3. Spectral shape is different (Povey^0.85 vs Hann)
4. Encoder sees "out of distribution" features
5. Decoder falls back to safe/common tokens: "m", "mm", "yeah", "mmhmm"

**Analogy:** Like training on photos with specific brightness/contrast, then testing on differently processed photos → poor recognition.

---

## 🔧 Required Code Changes

### Priority 1: Fix Window Function (HIGH PRIORITY)

**File:** `rust-crates/swictation-stt/src/audio.rs`

**Current (line 335):**
```rust
let window = hann_window(WIN_LENGTH);
```

**Should be:**
```rust
let window = povey_window(WIN_LENGTH);
```

**Add Povey window function:**
```rust
/// Create Povey window for STFT
///
/// Povey window = Hann^0.85, designed for speech recognition
/// Formula: w(n) = [0.5 - 0.5·cos(2πn/N)]^0.85
fn povey_window(window_length: usize) -> Vec<f32> {
    (0..window_length)
        .map(|n| {
            let factor = 2.0 * PI * n as f32 / (window_length - 1) as f32;
            let hann = 0.5 * (1.0 - factor.cos());
            hann.powf(0.85)  // Povey = Hann^0.85
        })
        .collect()
}
```

**Estimated Impact:** **CRITICAL** - This is likely the primary cause of poor transcriptions.

---

### Priority 2: Investigate is_librosa Flag (MEDIUM PRIORITY)

**Current:** Not set
**Sherpa-ONNX:** Sets to `true` for Parakeet-TDT

**Investigation needed:**
- What does `is_librosa` actually control in kaldi-native-fbank?
- Does it affect mel filterbank construction?
- Should we adapt our mel filterbank code to match?

**File to check:** kaldi-native-fbank source code

---

## 📈 Testing Plan

### After Implementing Window Fix:

1. **Rebuild and test with sample audio:**
   ```bash
   cargo build --release
   ./target/release/swictation-stt test-audio /tmp/en-short.wav
   ```

2. **Expected results:**
   - **Before fix:** "mm" (2 tokens)
   - **After fix:** "Hello world" or similar (actual speech)

3. **Validation:**
   - Compare with sherpa-onnx Python/C++ output on same audio
   - Should match exactly if window is the only issue

---

## 🔬 Sherpa-ONNX Configuration Summary

**For Parakeet-TDT 1.1B (non-GigaAM) models:**

| Parameter | Value | Source |
|-----------|-------|--------|
| window_type | `"povey"` | features.h default |
| preemph_coeff | `0.97` | features.h default |
| remove_dc_offset | `false` | Override (line 167) |
| is_librosa | `true` | Override (line 167) |
| normalize_type | `"per_feature"` | From ONNX metadata |
| feat_dim | `128` | From ONNX metadata |
| low_freq | `0` | Default |
| high_freq | `-400` | Default (8000 Hz effective) |

**Our current implementation:**
- ✅ preemph_coeff = 0.97
- ✅ remove_dc_offset = false
- ✅ normalize_type = per_feature (computed from input)
- ✅ feat_dim = 128
- ✅ low_freq = 0
- ✅ high_freq = 8000
- ❌ window_type = Hann (should be Povey)
- ❓ is_librosa = not set (may need investigation)

---

## 📚 References

1. **Povey Window Formula:**
   https://dsp.stackexchange.com/questions/54810/povey-window-formula

2. **Kaldi Feature Window Source:**
   https://github.com/kaldi-asr/kaldi/blob/master/src/feat/feature-window.h

3. **Sherpa-ONNX Features Config:**
   `/var/tmp/sherpa-onnx/sherpa-onnx/csrc/features.h:42-44`

4. **Sherpa-ONNX NeMo Recognizer:**
   `/var/tmp/sherpa-onnx/sherpa-onnx/csrc/offline-recognizer-transducer-nemo-impl.h:167`

5. **Our Audio Preprocessing:**
   `/opt/swictation/rust-crates/swictation-stt/src/audio.rs:335`

---

## 🎯 Conclusion

The **window function mismatch** (Hann vs Povey) is the most likely explanation for poor transcription quality. This is a **critical bug** that should be fixed immediately.

**Confidence Level:** 95% - Based on:
- Exact comparison with sherpa-onnx reference implementation
- Understanding of how spectral shape affects encoder inputs
- Pattern matches symptoms (valid tokens, but wrong content)

**Next Steps:**
1. Implement Povey window function
2. Test with sample audio
3. Compare results with sherpa-onnx
4. If still issues, investigate is_librosa flag

---

**AHA Captured By:** Hive Mind Collective (researcher + analyst + coder + validator)
**Investigation Duration:** Deep dive into sherpa-onnx codebase
**Key Learning:** Never assume preprocessing details - always verify against reference!
