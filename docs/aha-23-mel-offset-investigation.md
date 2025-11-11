# AHA #23: Comprehensive Mel Feature Offset Investigation

**Date:** 2025-11-10
**Status:** IN PROGRESS - Root cause not yet identified
**Impact:** CRITICAL - Prevents 1.1B model from working (produces "mmhmm" gibberish)

## Problem Statement

Rust mel-filterbank features have a persistent ~6 dB offset compared to Python/torchaudio implementation:
- **Rust mean:** -2.05 dB
- **Python mean:** -8.18 dB
- **Offset:** 6.13 dB (constant in log space)
- **Linear factor:** exp(6.13) ≈ 460×
- **Correlation:** 0.86 (insufficient, need >0.95)
- **Result:** 92.5% blank predictions → "mmhmm" transcription

## Hypotheses Tested & Results

### ❌ Hypothesis 1: Power vs Magnitude Spectrum
**Theory:** Rust using |FFT|² while Python uses |FFT|
**Test:** Changed Rust to magnitude spectrum: `(c.re² + c.im²).sqrt()`
**Result:** NO DIFFERENCE - Features identical to before
**Conclusion:** NOT the issue

### ❌ Hypothesis 2: DC Offset Removal
**Theory:** Python doing per-frame DC removal, Rust not
**Test:** Added `remove_dc_offset=False` to Python script
**Result:** NO DIFFERENCE - Offset persists
**Conclusion:** NOT the issue

### ❌ Hypothesis 3: Window Energy Normalization
**Theory:** Povey window needs sum(w²)=1.0 normalization
**Test:** Calculated Povey window sum(w²) = 160.57
**Analysis:** Would cause -22 dB difference, not -6 dB
**Conclusion:** NOT the issue

### ❌ Hypothesis 4: FFT Scaling Convention
**Theory:** Different FFT libraries use different 1/N scaling
**Research:** RustFFT and PyTorch both use NO normalization by default
**Conclusion:** NOT the issue

### ✅ Hypothesis 5: Mel Filterbank Construction
**Verification:** Both use height normalization (peak=1.0), NOT area normalization
**Conclusion:** Implementation is CORRECT

## What We've Verified as Correct

| Component | Rust | Python | Status |
|-----------|------|--------|--------|
| Window type | Povey (exp=0.85) | Povey (exp=0.85) | ✅ MATCH |
| Frequency range | 20-7600 Hz | 20-7600 Hz | ✅ MATCH |
| Mel feature count | 80 | 80 | ✅ MATCH |
| Preemphasis | 0.97 | 0.97 | ✅ MATCH |
| Frame length | 25ms | 25ms | ✅ MATCH |
| Frame shift | 10ms | 10ms | ✅ MATCH |
| Spectrum type | Power (re²+im²) | Power (default) | ✅ MATCH |
| FFT normalization | None | None (default) | ✅ MATCH |
| DC offset removal | False (NeMo) | False (set) | ✅ MATCH |

## Observations

1. **Constant offset:** Mean differs by 6.13 dB, but std devs similar (3.61 vs 3.16)
2. **High correlation:** 0.86 suggests shapes are similar, just scaled/offset
3. **High-frequency emphasis:** Bins 75-79 show worst mismatch (7+ dB)
4. **Sample count mismatch:** Python loads 98,304 samples, Rust loads 98,688 samples
5. **Frame count mismatch:** Python produces 614 frames, Rust produces 615 frames

## Mathematical Analysis

In log space:
```
log(rust) - log(python) = 6.13
log(rust/python) = 6.13
rust/python = exp(6.13) ≈ 460
```

This means Rust values are **460× larger** in linear space before log computation.

This is far too large for subtle differences. Points to fundamental processing difference.

## Remaining Investigation Paths

### 1. Audio Loading/Amplitude
- ✅ Python RMS: 0.045716
- ⚠️ Rust RMS: Unknown (need to add debug output)
- **Action:** Compare raw audio amplitudes before any processing

### 2. Resampling Implementation
- Python uses torchaudio.transforms.Resample
- Rust uses simple linear interpolation
- **Action:** Test with identical resampling or skip resampling (use 16kHz source)

### 3. HTK vs Slaney Mel Scale
- Both implementations claim to use HTK formula
- **Action:** Verify mel scale formula: `mel = 2595 * log10(1 + hz/700)`

### 4. Log Computation Details
- Both use natural log (ln)
- Both add epsilon to avoid log(0)
- Rust epsilon: 1e-10
- Python epsilon: Check torchaudio source
- **Action:** Verify epsilon values match

### 5. Hidden Preprocessing
- torchaudio.compliance.kaldi.fbank may have undocumented preprocessing
- **Action:** Read full torchaudio source code for hidden steps

## Diagnostic Tools Created

1. `export_mel_features.rs` - Rust mel feature exporter
2. `extract_python_mel_features.py` - Python reference implementation
3. `compare_mel_features.py` - Element-by-element comparison
4. `diagnose_feature_mismatch.sh` - Master diagnostic orchestrator

## Files Modified

- `rust-crates/swictation-stt/src/audio.rs`
  - Fixed mel feature count: 128 → 80
  - Removed incorrect per-feature normalization
  - Re-enabled preemphasis (0.97)
  - Added power vs magnitude testing (reverted)

- `scripts/extract_python_mel_features.py`
  - Added `remove_dc_offset=False` parameter

## Next Steps (Priority Order)

1. **Add raw audio debug logging** to Rust to compare amplitudes
2. **Test with pre-16kHz audio** to eliminate resampling differences
3. **Verify mel scale formula** implementation in both
4. **Check torchaudio source** for undocumented preprocessing
5. **Compare epsilon values** in log computation
6. **Test with different audio files** to see if offset is file-specific

## References

- [torchaudio.compliance.kaldi.fbank docs](https://pytorch.org/audio/stable/generated/torchaudio.compliance.kaldi.fbank.html)
- [RustFFT normalization](https://docs.rs/rustfft/)
- [Kaldi mel filterbank](https://kaldi-asr.org/doc/feat.html)
- sherpa-onnx source: offline-recognizer-transducer-nemo-impl.h
