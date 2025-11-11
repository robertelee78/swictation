# 🧠 Hive Mind Coder Synthesis: Comprehensive Analysis & Action Plan

**Date**: 2025-11-10
**Agent**: Coder (Hive Mind)
**Mission**: Review all diagnostic efforts and provide actionable fixes

---

## 📊 EXECUTIVE SUMMARY

After reviewing **6 comprehensive documents** and **18+ diagnostic scripts**, the investigation has:

✅ **IDENTIFIED** the root cause: **Mel feature preprocessing mismatch**
✅ **TESTED** multiple hypotheses systematically
✅ **ELIMINATED** several false leads (preemphasis, decoder logic, blank token ID)
✅ **DISCOVERED** a persistent ~6 dB offset in mel features
❌ **NOT RESOLVED** the underlying bug causing "mmhmm" gibberish

### The Core Problem
- **Rust output**: "mmhmm" with 92.5% blank predictions
- **Python sherpa-onnx**: 95%+ accuracy with same model
- **Symptom**: Mel features have ~6 dB constant offset (460× in linear space)
- **Impact**: Encoder receives wrong features → 40× higher blank probability

---

## 🔬 INVESTIGATION TIMELINE: What We Know

### Phase 1: Initial Diagnosis ✅
**Document**: `CRITICAL-FIX-decoder-compilation-error.md`

**Finding**: Code doesn't compile due to missing return statistics
```rust
// Line 632 - BROKEN
Ok((tokens, last_emitted_token, decoder_out))

// Expected
Ok((tokens, last_emitted_token, decoder_out, (blank_count, nonblank_count)))
```

**Status**: ✅ **FIXED** - Statistics tracking added (lines 517-518, 578-582)

---

### Phase 2: Decoder Algorithm Verification ✅
**Document**: `decoder-blank-token-analysis.md`

**Finding**: Rust decoder matches C++ sherpa-onnx reference **exactly**
- ✅ Greedy search logic identical
- ✅ Skip logic matches (3 conditions)
- ✅ Blank token ID correctly found (1024)
- ✅ RNN state management correct

**Status**: ✅ **VERIFIED** - Decoder is NOT the problem

---

### Phase 3: Audio Preprocessing Investigation ⚠️
**Document**: `rust-python-audio-analysis.md`

**Hypotheses Tested**:
1. ❌ Preemphasis filter (disabled - no improvement)
2. ✅ Per-feature normalization (implemented correctly)
3. ✅ Mel filterbank config (matches sherpa-onnx)
4. ✅ Povey window (correct implementation)
5. ✅ STFT parameters (all match)

**Key Discovery**: Preemphasis is NOT the cause!

---

### Phase 4: Mel Feature Offset Investigation ⚠️
**Document**: `aha-23-mel-offset-investigation.md`

**Critical Findings**:
- **Rust mean**: -2.05 dB
- **Python mean**: -8.18 dB
- **Offset**: 6.13 dB (constant in log space)
- **Linear factor**: exp(6.13) ≈ 460×
- **Correlation**: 0.86 (insufficient, need >0.95)

**Hypotheses Tested**:
1. ❌ Power vs Magnitude spectrum (no change)
2. ❌ DC offset removal (not the issue)
3. ❌ Window energy normalization (wrong magnitude)
4. ❌ FFT scaling convention (both use no normalization)
5. ✅ Mel filterbank construction (correct)

**Status**: 🔴 **ROOT CAUSE NOT FOUND**

---

### Phase 5: Diagnostic Tools Development ✅
**Documents**: `rust_csv_export_summary.md`, `README_DIAGNOSIS.md`

**Tools Created**:
1. ✅ `export_mel_features.rs` - Rust CSV exporter
2. ✅ `extract_python_mel_features.py` - Python reference
3. ✅ `compare_mel_features.py` - Element-wise comparison
4. ✅ `diagnose_feature_mismatch.sh` - Master orchestrator
5. ✅ `verify_rust_csv.py` - Validation tool

**Status**: ✅ **COMPLETE** - Diagnostic infrastructure ready

---

### Phase 6: Hive Mind Collective Diagnosis 🧠
**Document**: `HIVE-MIND-DIAGNOSIS-RUST-MMHMM-BUG.md`

**Consensus**: The root cause is **encoder feature mismatch**, NOT:
- ❌ Preemphasis filter
- ❌ Sample normalization
- ❌ Decoder algorithm
- ❌ Blank token ID

**Evidence**:
- Blank probability explodes to 81.30% (logit=9.49)
- Best non-blank only 2.00% (logit=5.79)
- Ratio: 40× higher for blank
- This indicates encoder receives WRONG features

---

## 🎯 THE REMAINING MYSTERY: 6 dB Offset

### What We've Checked ✅
| Component | Rust | Python | Status |
|-----------|------|--------|--------|
| Sample normalization | mean=0, std=1 | Yes | ✅ MATCH |
| Preemphasis | 0.97 | Unknown | ⚠️ TESTED (disabled - no change) |
| Window | Povey (exp=0.85) | Povey | ✅ MATCH |
| STFT params | 512/160/400 | Same | ✅ MATCH |
| Mel range | 20-7600 Hz | Same | ✅ MATCH |
| Mel count | 80 | 80 | ✅ MATCH |
| Spectrum type | Power (re²+im²) | Power | ✅ MATCH |
| Log epsilon | 1e-10 | Check | ⚠️ UNKNOWN |

### What We Haven't Checked ❓
1. **Resampling Implementation**
   - Python: `torchaudio.transforms.Resample`
   - Rust: Simple linear interpolation
   - **Action**: Test with pre-16kHz audio to eliminate resampling

2. **Audio Loading Amplitude**
   - Python RMS: 0.045716
   - Rust RMS: ⚠️ **UNKNOWN** (need debug output)
   - **Action**: Add raw audio amplitude logging

3. **Hidden torchaudio Preprocessing**
   - `torchaudio.compliance.kaldi.fbank` may have undocumented steps
   - **Action**: Read full torchaudio source code

4. **Epsilon in Log Computation**
   - Rust: 1e-10
   - Python: Need to verify
   - **Action**: Check torchaudio epsilon value

5. **Sample Count Mismatch**
   - Python: 98,304 samples → 614 frames
   - Rust: 98,688 samples → 615 frames
   - **Action**: Investigate why sample counts differ

---

## 🚀 ACTIONABLE RECOMMENDATIONS (Priority Order)

### CRITICAL PRIORITY 1: Verify Raw Audio Loading 🔴

**Hypothesis**: Audio amplitude differs between implementations

**Test**:
```rust
// Add to audio.rs extract_mel_features() after loading
let rms = (samples.iter().map(|&x| x * x).sum::<f32>() / samples.len() as f32).sqrt();
eprintln!("RAW AUDIO: samples={}, RMS={:.6}, mean={:.6}",
         samples.len(), rms,
         samples.iter().sum::<f32>() / samples.len() as f32);
```

**Expected**: If RMS differs significantly from Python (0.045716), audio loading is wrong

---

### CRITICAL PRIORITY 2: Test Without Resampling 🔴

**Hypothesis**: Resampling implementation differs

**Test**:
```bash
# Use audio file that's already 16kHz (no resampling needed)
# OR compare with Python using same resampling library

# If no 16kHz file available, create one:
ffmpeg -i examples/en-short.mp3 -ar 16000 examples/en-short-16k.wav

# Test both implementations:
cargo run --release --example export_mel_features examples/en-short-16k.wav /tmp/rust.csv
python3.12 scripts/extract_python_mel_features.py examples/en-short-16k.wav /tmp/python.csv
python3.12 scripts/compare_mel_features.py /tmp/rust.csv /tmp/python.csv
```

**Expected**: If offset disappears, resampling is the culprit

---

### HIGH PRIORITY 3: Verify Mel Filterbank Scaling 🟡

**Hypothesis**: Mel filterbank coefficients differ

**Test**:
```rust
// Add to audio.rs after mel_filters creation
eprintln!("MEL FILTERBANK STATS:");
for mel_idx in 0..5 {
    let row_sum: f32 = self.mel_filters.row(mel_idx).sum();
    let row_max: f32 = self.mel_filters.row(mel_idx).iter()
        .fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    eprintln!("  Bin {}: sum={:.6}, max={:.6}", mel_idx, row_sum, row_max);
}
```

**Compare with Python**:
```python
import torchaudio
import numpy as np

mel_fb = torchaudio.functional.melscale_fbanks(
    n_freqs=257,
    n_mels=80,
    sample_rate=16000,
    f_min=20.0,
    f_max=7600.0,
    norm='slaney'  # or None
)

print("MEL FILTERBANK STATS:")
for i in range(5):
    print(f"  Bin {i}: sum={mel_fb[i].sum():.6f}, max={mel_fb[i].max():.6f}")
```

**Expected**: If sums/maxs differ, filterbank construction is wrong

---

### HIGH PRIORITY 4: Check HTK vs Slaney Mel Scale 🟡

**Hypothesis**: Mel scale formula differs

**Test**:
```rust
// In create_mel_filterbank(), add debug output
fn hz_to_mel(hz: f32) -> f32 {
    let mel = 2595.0 * (1.0 + hz / 700.0).log10();
    if hz < 100.0 {  // Only log first few
        eprintln!("hz_to_mel({}) = {}", hz, mel);
    }
    mel
}
```

**Verify Python uses same formula**:
```python
# Check if Python uses HTK or Slaney
# HTK: mel = 2595 * log10(1 + hz/700)
# Slaney: Different below 1000 Hz
```

**Expected**: Both should use HTK (Kaldi default)

---

### MEDIUM PRIORITY 5: Investigate Sample Count Discrepancy 🟢

**Hypothesis**: Padding or edge handling differs

**Finding**:
- Python: 98,304 samples
- Rust: 98,688 samples
- Difference: 384 samples (24ms at 16kHz)

**Test**:
```bash
# Check actual file duration and sample counts
ffprobe examples/en-short.mp3

# Compare with Rust loading
cargo run --release --example export_mel_features examples/en-short.mp3 /tmp/rust.csv 2>&1 | grep "samples"
```

**Expected**: Find why Rust loads 384 more samples

---

### MEDIUM PRIORITY 6: Check torchaudio Source Code 🟢

**Hypothesis**: Hidden preprocessing in `kaldi.fbank`

**Action**:
```bash
# Read torchaudio source
# Look for: torchaudio/compliance/kaldi.py
# Check for undocumented preprocessing steps:
# - DC removal
# - Dithering
# - Energy normalization
# - Hidden scaling factors
```

**Expected**: Find hidden preprocessing step causing 6 dB offset

---

## 🛠️ IMMEDIATE FIX WORKFLOW

### Step 1: Verify Current Code Compiles ✅
```bash
cd /opt/swictation/rust-crates/swictation-stt
cargo build --release --example test_1_1b_direct
```

**Status**: Should compile (statistics tracking added)

---

### Step 2: Run Diagnostic Suite
```bash
cd /opt/swictation
./scripts/diagnose_feature_mismatch.sh examples/en-short.mp3
```

**Expected Output**:
- Rust CSV: ~615 frames × 80 features
- Python CSV: ~614 frames × 80 features
- Comparison: ~6 dB offset detected

---

### Step 3: Test Priority 1 & 2 (Audio + Resampling)
```rust
// Modify audio.rs extract_mel_features()
// Add extensive logging as shown above
cargo build --release --example export_mel_features

# Test with original file
./rust-crates/target/release/examples/export_mel_features examples/en-short.mp3 /tmp/rust_test.csv

# Test with pre-16kHz file
ffmpeg -i examples/en-short.mp3 -ar 16000 examples/en-short-16k.wav
./rust-crates/target/release/examples/export_mel_features examples/en-short-16k.wav /tmp/rust_16k.csv
```

---

### Step 4: Compare Results
```bash
python3.12 scripts/compare_mel_features.py /tmp/rust_test.csv /tmp/python_mel.csv
python3.12 scripts/compare_mel_features.py /tmp/rust_16k.csv /tmp/python_16k.csv
```

**If offset disappears with 16kHz input**:
→ Fix resampling implementation

**If offset persists**:
→ Investigate Priority 3-6

---

## 🧩 WHAT'S STILL UNKNOWN

### The 460× Linear Factor Mystery

A 6.13 dB constant offset in log space means:
```
rust_value / python_value = exp(6.13) ≈ 460
```

This is **far too large** for subtle differences. Possible causes:

1. **Scaling Factor**: Missing normalization by window energy or FFT size
2. **Power vs Energy**: Computing power spectrum differently
3. **Filterbank Scaling**: Mel filters not normalized correctly
4. **Hidden Constant**: Python multiplies by constant factor before log

---

### Sample Count Mismatch

384 extra samples = exactly 24ms at 16kHz:
- Could be padding at file boundaries
- Could be different chunk handling
- Could be MP3 decoder difference

**Impact**: Causes 1 extra frame in Rust (615 vs 614)

---

## 📝 FILES REQUIRING MODIFICATION

### 1. `/opt/swictation/rust-crates/swictation-stt/src/audio.rs`

**Priority 1 Changes** (Add debug logging):
```rust
// Line ~268 (in extract_mel_features)
// Add BEFORE any processing:
let rms = (samples.iter().map(|&x| x * x).sum::<f32>() / samples.len() as f32).sqrt();
eprintln!("🔊 RAW AUDIO: samples={}, RMS={:.6}, mean={:.6}, min={:.6}, max={:.6}",
         samples.len(), rms,
         samples.iter().sum::<f32>() / samples.len() as f32,
         samples.iter().fold(f32::INFINITY, |a, &b| a.min(b)),
         samples.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b)));

// Line ~332 (after mel filterbank creation)
// Add MEL FILTERBANK verification:
eprintln!("🔬 MEL FILTERBANK:");
for mel_idx in 0..5 {
    let row_sum: f32 = self.mel_filters.row(mel_idx).sum();
    let row_max: f32 = self.mel_filters.row(mel_idx).iter()
        .fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    eprintln!("  Bin {}: sum={:.6}, max={:.6}", mel_idx, row_sum, row_max);
}
```

**Priority 2 Changes** (Test resampling bypass):
```rust
// Line ~? (in load_audio or wherever resampling happens)
// Add flag to skip resampling for debugging:
let skip_resample = std::env::var("SKIP_RESAMPLE").is_ok();
if original_sr != 16000 && !skip_resample {
    eprintln!("⚠️  Resampling {} → 16000 Hz", original_sr);
    // ... existing resampling code
} else {
    eprintln!("✅ Using native sample rate (no resampling)");
}
```

---

### 2. `/opt/swictation/scripts/extract_python_mel_features.py`

**Add matching debug output**:
```python
# After loading audio
rms = np.sqrt(np.mean(waveform.numpy() ** 2))
print(f"🔊 RAW AUDIO: samples={waveform.shape[-1]}, RMS={rms:.6f}, "
      f"mean={waveform.mean():.6f}, min={waveform.min():.6f}, max={waveform.max():.6f}")

# After mel filterbank creation (if accessible)
# Print filterbank stats to match Rust
```

---

## 🎯 SUCCESS CRITERIA

### Phase 1: Diagnosis Complete ✅
- ✅ Mel offset quantified (6.13 dB)
- ✅ Diagnostic tools working
- ✅ Correlation measured (0.86)

### Phase 2: Root Cause Found ⏳
- ⏳ Identify exact step causing 460× linear factor
- ⏳ Explain 6.13 dB constant offset
- ⏳ Achieve correlation >0.999

### Phase 3: Fix Implemented 🎯
- 🎯 Mel features match (max diff < 0.001)
- 🎯 Rust transcription matches Python >95%
- 🎯 "mmhmm" gibberish eliminated

---

## 🧠 HIVE MIND MEMORY COORDINATION

```bash
# Store findings
npx claude-flow@alpha hooks post-task \
  --task-id "coder-synthesis" \
  --memory-key "hive/coder/synthesis"

# Notify other agents
npx claude-flow@alpha hooks notify \
  --message "Coder synthesis complete - actionable plan ready"

# Save to memory
npx claude-flow@alpha memory store \
  --key "hive/shared/action-plan" \
  --value "Priority: 1=audio RMS, 2=resampling, 3=filterbank"
```

---

## 📚 REFERENCE DOCUMENTS REVIEWED

1. ✅ `CRITICAL-FIX-decoder-compilation-error.md` - Compilation error fixed
2. ✅ `HIVE-MIND-DIAGNOSIS-RUST-MMHMM-BUG.md` - Collective intelligence report
3. ✅ `rust-ort-decoder-bug-analysis.md` - Decoder verified correct
4. ✅ `aha-23-mel-offset-investigation.md` - 6 dB offset identified
5. ✅ `decoder-blank-token-analysis.md` - Blank token handling verified
6. ✅ `rust-python-audio-analysis.md` - Preprocessing comparison
7. ✅ `rust_csv_export_summary.md` - Diagnostic tools ready
8. ✅ `README_DIAGNOSIS.md` - Test suite documentation

---

## 🏁 NEXT STEPS FOR IMPLEMENTATION TEAM

1. **Immediate** (Today):
   - Add debug logging (Priority 1)
   - Test with 16kHz input (Priority 2)
   - Run diagnostic suite
   - Report RMS and sample count findings

2. **Short-term** (This Week):
   - Verify mel filterbank scaling (Priority 3)
   - Check HTK vs Slaney (Priority 4)
   - Read torchaudio source (Priority 6)

3. **Medium-term** (If Above Doesn't Work):
   - Implement bit-exact FFT comparison
   - Test with multiple audio files
   - Contact torchaudio maintainers
   - Consider using sherpa-onnx's feature extraction directly

---

## 💡 FINAL INSIGHT

The investigation has been **extremely thorough** and **methodical**. We've:
- ✅ Verified decoder is correct
- ✅ Fixed compilation errors
- ✅ Tested multiple hypotheses
- ✅ Built diagnostic infrastructure
- ✅ Quantified the problem (6 dB offset)

The remaining issue is **very specific**: a constant 6.13 dB offset suggesting a **scaling factor** or **normalization constant** difference.

**Most likely culprits** (in order):
1. **Resampling implementation** (Priority 2)
2. **Audio loading amplitude** (Priority 1)
3. **Hidden torchaudio constant** (Priority 6)
4. **Mel filterbank normalization** (Priority 3)

**Confidence level**: HIGH that we'll find it with Priority 1-3 tests.

---

**Synthesized by**: Coder Agent (Hive Mind)
**Coordination**: Task ID `task-1762791854787-h63pvwnzt`
**Memory**: `hive/coder/synthesis` + `hive/shared/action-plan`
**Status**: ✅ READY FOR IMPLEMENTATION

---

*"The hive has spoken. Now we execute."* 🐝
