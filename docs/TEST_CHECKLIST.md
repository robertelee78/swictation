# ✅ Verification Test Checklist
## Quick Reference for Implementation Team

**Purpose:** Track progress on systematic bug isolation
**Goal:** Identify exact step causing 6 dB mel feature offset
**Status:** Ready to execute

---

## 🎯 Critical Path Tests (Must Pass Before Fix)

### Phase 1: Isolate Root Cause

- [ ] **Test 1.1: Raw Audio Loading**
  - [ ] Create `examples/export_raw_audio.rs`
  - [ ] Add `export_raw_samples()` to `audio.rs`
  - [ ] Create `scripts/compare_raw_audio.py`
  - [ ] Run: `cargo run --example export_raw_audio --release examples/en-short.mp3 rust_raw_audio.csv`
  - [ ] Run: `python scripts/compare_raw_audio.py rust_raw_audio.csv examples/en-short.mp3`
  - [ ] **Expected:** ✅ PASS (RMS diff < 0.1%)
  - [ ] **If FAIL:** Issue is in audio loading/resampling

- [ ] **Test 1.2: Power Spectrum**
  - [ ] Add `export_power_spectrum_frame()` to `audio.rs`
  - [ ] Add `create_povey_window()` helper to `audio.rs`
  - [ ] Create `examples/export_power_spectrum.rs`
  - [ ] Create `scripts/compare_power_spectrum.py`
  - [ ] Run: `cargo run --example export_power_spectrum --release examples/en-short.mp3 0 rust_power_spectrum.csv`
  - [ ] Run: `python scripts/compare_power_spectrum.py rust_power_spectrum.csv examples/en-short.mp3 0`
  - [ ] **Expected:** ✅ PASS (log diff < 0.1 dB) OR ❌ FAIL (6 dB offset here)
  - [ ] **If FAIL:** Issue is in FFT/windowing/power computation

- [ ] **Test 1.3: Mel Filterbank Weights**
  - [ ] Add `export_mel_filterbank()` to `audio.rs`
  - [ ] Create `examples/export_mel_filterbank.rs`
  - [ ] Create `scripts/verify_mel_filterbank.py`
  - [ ] Run: `cargo run --example export_mel_filterbank rust_mel_filterbank.csv`
  - [ ] Run: `python scripts/verify_mel_filterbank.py rust_mel_filterbank.csv`
  - [ ] **Expected:** ✅ PASS (weights match < 1e-4) OR ❌ FAIL (filterbank construction wrong)
  - [ ] **If FAIL:** Issue is in HTK formula or triangular filter construction

- [ ] **Test 1.4: Full Mel Pipeline** (Already exists)
  - [ ] Run: `./scripts/diagnose_feature_mismatch.sh`
  - [ ] **Current:** ❌ FAIL (6.13 dB offset)
  - [ ] **After fix:** ✅ PASS (offset < 0.1 dB, correlation > 0.99)

---

## 🔧 Phase 2: Validate Fix (After Root Cause Fixed)

- [ ] **Test 2.1: Encoder Input Validation**
  - [ ] Add debug logging to `recognizer_ort.rs` before encoder
  - [ ] Verify input shape: (1, 80, N)
  - [ ] Verify normalization: mean ≈ 0, std ≈ 1
  - [ ] Log first 5 features

- [ ] **Test 2.2: Encoder Output Comparison**
  - [ ] Add encoder output export to `recognizer_ort.rs`
  - [ ] Create `scripts/compare_encoder_outputs.py`
  - [ ] Verify outputs match Python within 1e-4

- [ ] **Test 3.1: Decoder State Management**
  - [ ] Add state change logging to `decode_frames_with_state()`
  - [ ] Verify states evolve after emission
  - [ ] Verify no state freezing

- [ ] **Test 3.2: Blank Probability Analysis**
  - [ ] Add joiner logit logging
  - [ ] Verify blank probability < 80%
  - [ ] Verify balanced non-blank distribution
  - [ ] Compare with Python sherpa-onnx

- [ ] **Test 4.1: End-to-End Transcription** ⚡ CRITICAL
  - [ ] Run: `cargo run --example test_1_1b_direct examples/en-short.mp3`
  - [ ] **Expected:** "hey there how are you doing today"
  - [ ] **Current:** "mmhmm"
  - [ ] **Success:** Correct transcription with >95% word accuracy

---

## 📊 Phase 3: Comprehensive Validation (Final)

- [ ] **Test 4.2: Edge Cases**
  - [ ] Silence (5 sec) → no crash
  - [ ] Music → no hallucination
  - [ ] Multi-speaker → graceful handling
  - [ ] Very short (<1 sec) → no crash

- [ ] **Test 4.3: Reference Comparison**
  - [ ] Create `scripts/batch_test_rust_vs_python.sh`
  - [ ] Test on 100 diverse samples
  - [ ] Compute WER for each
  - [ ] **Success:** WER within 2% of Python

- [ ] **Test 5.1: Memory Usage**
  - [ ] Run 1000 file loop
  - [ ] Monitor RSS memory
  - [ ] **Success:** No unbounded growth, peak < 2GB

- [ ] **Test 5.2: Performance**
  - [ ] Benchmark CPU and GPU
  - [ ] **Success:** CPU >1.0× RT, GPU >5.0× RT

---

## 📋 File Creation Checklist

### Rust Examples (in `rust-crates/swictation-stt/examples/`)
- [ ] `export_raw_audio.rs` - Export raw audio samples to CSV
- [ ] `export_power_spectrum.rs` - Export power spectrum for single frame
- [ ] `export_mel_filterbank.rs` - Export mel filterbank weights

### Rust Code Modifications (in `rust-crates/swictation-stt/src/audio.rs`)
- [ ] `export_raw_samples()` - Export audio samples with statistics
- [ ] `export_power_spectrum_frame()` - Export power spectrum for frame
- [ ] `create_povey_window()` - Generate Povey window coefficients
- [ ] `export_mel_filterbank()` - Export mel filterbank matrix

### Python Scripts (in `scripts/`)
- [ ] `compare_raw_audio.py` - Compare Rust vs Python audio loading
- [ ] `compare_power_spectrum.py` - Compare Rust vs Python power spectrum
- [ ] `verify_mel_filterbank.py` - Verify mel filterbank weights
- [ ] `compare_encoder_outputs.py` - Compare encoder outputs (later)
- [ ] `batch_test_rust_vs_python.sh` - Batch comparison (later)

### Master Diagnostic (in `scripts/`)
- [ ] `diagnose_root_cause.sh` - Unified diagnostic runner

---

## 🎯 Success Metrics

### Phase 1 Complete When:
✅ We know which test fails (1.1, 1.2, or 1.3)
✅ Root cause identified with confidence

### Phase 2 Complete When:
✅ Test 1.4 passes (mel features match Python)
✅ Test 4.1 passes (correct transcription)
✅ Blank rate normalized (50-70%)

### Phase 3 Complete When:
✅ All edge cases handled
✅ WER matches Python baseline
✅ No memory leaks
✅ Performance acceptable

---

## 🚨 Red Flags (Stop and Investigate)

- ❌ Test 1.1 fails → Audio loading is broken (unlikely but critical)
- ❌ Test 1.2 fails with ~6 dB offset → FFT or windowing is root cause
- ❌ Test 1.3 fails → Mel filterbank construction is root cause
- ❌ Tests 1.1-1.3 pass but 1.4 fails → Normalization or log computation issue
- ❌ Fix implemented but 4.1 still fails → Encoder or decoder issue

---

## 📞 Coordination Protocol

### Before Starting Each Test
```bash
npx claude-flow@alpha hooks pre-task --description "Running Test X.Y: [description]"
```

### After Completing Each Test
```bash
npx claude-flow@alpha hooks post-task --memory-key "hive/tester/test-X-Y-results"
npx claude-flow@alpha hooks notify --message "Test X.Y: [PASS/FAIL] - [key finding]"
```

### After Phase Completion
```bash
npx claude-flow@alpha hooks notify --message "Phase N complete: [summary of findings]"
```

---

## 📈 Progress Tracking

**Current Status:** Phase 1 ready to start

| Phase | Status | Completion |
|-------|--------|------------|
| Phase 1: Isolate | 🟡 Ready | 0% |
| Phase 2: Validate Fix | ⚪ Blocked | 0% |
| Phase 3: Comprehensive | ⚪ Blocked | 0% |

**Updated:** 2025-11-10 by Tester Agent

---

## 🔗 Related Documents

- [VERIFICATION_TESTING_STRATEGY.md](./VERIFICATION_TESTING_STRATEGY.md) - Full strategy
- [IMMEDIATE_TEST_PLAN.md](./IMMEDIATE_TEST_PLAN.md) - Detailed implementation guide
- [HIVE-MIND-DIAGNOSIS-RUST-MMHMM-BUG.md](./HIVE-MIND-DIAGNOSIS-RUST-MMHMM-BUG.md) - Original diagnosis

---

**Next Action:** Start with Test 1.1 (Raw Audio Loading)
**Time Estimate:** 30 minutes for first test
**Blocker:** None
