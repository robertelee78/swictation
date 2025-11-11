# 🧪 Comprehensive Verification and Testing Strategy
## Rust STT Implementation - Parakeet-TDT 1.1B Model

**Created by:** Tester Agent (Hive Mind)
**Date:** 2025-11-10
**Status:** ACTIVE
**Coordination:** Task ID `task-1762791854801-xf0ano457`

---

## 🎯 Executive Summary

Based on Hive Mind findings, the root cause is **encoder feature mismatch** producing a 40× blank probability gap. This strategy defines comprehensive tests to:

1. **Isolate the feature extraction bug** (6 dB offset, 460× linear scale factor)
2. **Verify any proposed fixes** through systematic comparison
3. **Establish confidence** in the Rust implementation before deployment

---

## 📊 Current State Analysis

### Symptoms
- **Output:** "mmhmm" gibberish (tokens: [19, 1010, 1005, 1010, 1010])
- **Blank rate:** 90.9% (expected: 50-70%)
- **Blank probability:** 81.30% vs 2.00% for best non-blank (40× gap)
- **Mel feature offset:** 6.13 dB constant offset (460× linear)
- **Correlation:** 0.86 (insufficient, need >0.95)

### Known Good Baseline
- **Python sherpa-onnx:** 95%+ accuracy
- **Model:** Same Parakeet-TDT 1.1B ONNX files
- **Version:** sherpa-onnx v1.12.15

### Verified Correct
- ✅ Decoder algorithm (matches C++ line-by-line)
- ✅ Blank token ID (1024)
- ✅ Audio loading and resampling
- ✅ Preemphasis (0.97) - tested both enabled/disabled
- ✅ Basic mel filterbank parameters (80 bins, 20-7600 Hz)

### Known Issues
- ❌ Mel features have 6 dB offset from Python
- ❌ Encoder produces wrong output features
- ❓ Unknown step causing 460× amplitude difference

---

## 🔬 Test Categories

### Category 1: Feature Extraction Verification ⚡ CRITICAL
**Priority:** HIGHEST
**Goal:** Find exact step where mel features diverge

### Category 2: Encoder Output Verification
**Priority:** HIGH
**Goal:** Verify encoder produces correct output for given input

### Category 3: Decoder Logic Verification
**Priority:** MEDIUM (already verified, but retest after fixes)
**Goal:** Ensure decoder state management works correctly

### Category 4: End-to-End Integration
**Priority:** HIGH
**Goal:** Verify complete inference pipeline

### Category 5: Performance & Regression
**Priority:** MEDIUM
**Goal:** Ensure fixes don't degrade performance

---

## 📋 Detailed Test Specifications

## Category 1: Feature Extraction Verification

### Test 1.1: Raw Audio Amplitude Comparison
**Objective:** Verify audio loading produces identical amplitudes

**Test Steps:**
1. Load `examples/en-short.mp3` with Python torchaudio
2. Load same file with Rust audio loader
3. Export first 1000 samples to CSV from both
4. Compare element-by-element

**Success Criteria:**
- RMS values match within 0.1%
- Max amplitude difference < 1e-6
- Mean difference < 1e-7

**Implementation:**
```bash
# Rust export
cargo run --example export_raw_audio examples/en-short.mp3 rust_audio.csv

# Python comparison
python scripts/compare_raw_audio.py rust_audio.csv examples/en-short.mp3
```

**Expected Output:**
```
✅ RMS match: Python=0.045716, Rust=0.045714 (diff=0.004%)
✅ Max amplitude match within 1e-6
✅ Mean match within 1e-7
```

**If FAILS:** Audio loading or resampling is broken. Investigate:
- Sample rate conversion
- Bit depth normalization
- Mono conversion

---

### Test 1.2: STFT Power Spectrum Comparison
**Objective:** Verify FFT and power computation before mel filterbank

**Test Steps:**
1. Extract power spectrum for first frame from Rust
2. Extract power spectrum for first frame from Python (bypass mel)
3. Compare first 257 frequency bins (N_FFT/2 + 1)

**Success Criteria:**
- Power values match within 1%
- No constant offset in log space
- Correlation > 0.99

**Implementation:**
```rust
// In audio.rs, add debug export
pub fn export_power_spectrum(&self, audio: &[f32], frame_idx: usize) -> Array1<f32> {
    let frame = self.extract_frame(audio, frame_idx);
    let windowed = self.apply_povey_window(&frame);
    let fft_out = self.compute_fft(&windowed);
    // Return power: re^2 + im^2
    fft_out.iter().map(|c| c.re*c.re + c.im*c.im).collect()
}
```

**Expected Output:**
```
Frame 0, Bin 0: Python=1234.56, Rust=1234.57 (diff=0.001%)
Frame 0, Bin 50: Python=789.12, Rust=789.10 (diff=0.003%)
...
✅ Correlation: 0.998
✅ Max offset: 0.02 dB
```

**If FAILS:** FFT or windowing issue. Check:
- Povey window calculation
- FFT normalization
- Power vs magnitude

---

### Test 1.3: Mel Filterbank Application
**Objective:** Verify mel filterbank weights and application

**Test Steps:**
1. Export mel filterbank weights from Rust (80×257 matrix)
2. Create identical filterbank in Python using same formula
3. Apply both filterbanks to SAME power spectrum
4. Compare mel feature vectors

**Success Criteria:**
- Filterbank weights identical (diff < 1e-6)
- Applied mel features match within 0.1 dB
- No systematic offset

**Implementation:**
```rust
// In audio.rs
pub fn export_mel_filterbank(&self, path: &str) -> Result<()> {
    let mut file = File::create(path)?;
    for (i, row) in self.mel_filters.outer_iter().enumerate() {
        writeln!(file, "# Mel bin {}", i)?;
        for val in row.iter() {
            write!(file, "{:.10},", val)?;
        }
        writeln!(file)?;
    }
    Ok(())
}
```

**Expected Output:**
```
Filterbank comparison:
✅ All 80 filters match Python within 1e-6
✅ Peak normalization correct (max=1.0)
✅ HTK mel scale formula verified

Applied to same spectrum:
✅ Mel bin 0: Python=-12.34, Rust=-12.35 (diff=0.01 dB)
✅ Mel bin 79: Python=-8.76, Rust=-8.77 (diff=0.01 dB)
```

**If FAILS:** Mel filterbank construction issue. Check:
- HTK mel scale: `mel = 2595 * log10(1 + hz/700)`
- Frequency bin mapping
- Triangular filter shape
- Peak vs area normalization

---

### Test 1.4: Complete Mel Feature Pipeline
**Objective:** End-to-end mel feature extraction comparison

**Test Steps:**
1. Run full mel extraction on `en-short.mp3` with Rust
2. Run identical extraction with Python torchaudio
3. Export both to CSV (frames × 80 features)
4. Statistical comparison

**Success Criteria:**
- Mean offset < 0.1 dB
- Std deviation match within 5%
- Correlation > 0.99
- Per-bin correlation > 0.95

**Implementation:**
```bash
# Already exists
./scripts/diagnose_feature_mismatch.sh
```

**Current Results (FAILING):**
```
❌ Mean offset: 6.13 dB (should be <0.1 dB)
❌ Linear scale factor: 460× (should be ~1.0)
❌ Correlation: 0.86 (should be >0.99)
```

**When PASSES:** Root cause is fixed. Proceed to Category 2.

---

## Category 2: Encoder Output Verification

### Test 2.1: Encoder Input Tensor Validation
**Objective:** Verify encoder receives correct input shape and values

**Test Steps:**
1. Load known-good mel features from Python
2. Feed to Rust encoder via ONNX Runtime
3. Log input tensor shape, mean, std
4. Compare with Python sherpa-onnx encoder input

**Success Criteria:**
- Tensor shape: (1, 80, N) or (1, N, 80) depending on model
- Mean close to 0 (after normalization)
- Std close to 1 (after normalization)

**Implementation:**
```rust
// In recognizer_ort.rs, add before encoder.run()
info!("Encoder input shape: {:?}", mel_features.shape());
info!("Input mean: {:.6}, std: {:.6}",
    mel_features.mean().unwrap(),
    mel_features.std(0.0));
let first_5 = mel_features.slice(s![0, 0..5, 0]);
info!("First 5 features: {:?}", first_5);
```

**Expected Output:**
```
Encoder input shape: [1, 80, 615]
Input mean: -0.00234, std: 0.98765
First 5 features: [-0.456, 0.123, -0.789, 0.234, -0.567]
```

**If FAILS:** Check:
- Normalization formula (subtract mean, divide by std)
- Axis for mean/std calculation
- Input transpose requirement

---

### Test 2.2: Encoder Output Comparison
**Objective:** Verify encoder produces correct embeddings

**Test Steps:**
1. Feed identical mel features to both implementations
2. Export encoder outputs (1 × T × 640)
3. Compare embeddings element-by-element

**Success Criteria:**
- Output shapes match
- Values match within 1e-4 (FP32 precision)
- No NaN or Inf values

**Implementation:**
```rust
// After encoder.run()
info!("Encoder output shape: {:?}", encoder_out.shape());
info!("Encoder output range: [{:.6}, {:.6}]",
    encoder_out.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(),
    encoder_out.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap());

// Export to CSV
self.export_encoder_output(&encoder_out, "encoder_out_rust.csv")?;
```

**Expected Output:**
```
✅ Shape match: [1, 615, 640]
✅ Value match within 1e-4
✅ No NaN/Inf values
```

**If FAILS:** ONNX Runtime loading issue. Check:
- Model file integrity
- Execution provider (CPU vs CUDA)
- ONNX Runtime version
- INT8 quantization handling

---

### Test 2.3: Encoder-to-Decoder Data Flow
**Objective:** Verify encoder output correctly feeds decoder

**Test Steps:**
1. Use Python encoder output as input to Rust decoder
2. Compare with full Python pipeline
3. Verify token predictions match

**Success Criteria:**
- Decoder accepts encoder output
- Token predictions match Python
- Blank rate normalized

**Implementation:**
```python
# Load Rust encoder output
rust_encoder_out = np.load("encoder_out_rust.npy")

# Feed to Python decoder
recognizer = sherpa.OfflineRecognizer(...)
# Manually inject encoder output
tokens = recognizer.decode_encoder_output(rust_encoder_out)

print(f"Tokens: {tokens}")
print(f"Text: {recognizer.tokens_to_text(tokens)}")
```

**Expected Output:**
```
If Rust encoder is correct:
✅ Tokens match Python encoder → Python decoder

If Rust encoder is wrong:
❌ Tokens differ, proving encoder is the issue
```

---

## Category 3: Decoder Logic Verification

### Test 3.1: Decoder State Management
**Objective:** Verify RNN states update correctly after emission

**Test Steps:**
1. Run decoder on known sequence: [19, 1010, 1005]
2. Log decoder_out after each token
3. Verify states change appropriately
4. Compare with C++ sherpa-onnx behavior

**Success Criteria:**
- decoder_out changes after each emission
- States evolve (not frozen)
- Blank probability doesn't explode

**Implementation:**
```rust
// In decode_frames_with_state()
if y != blank_id {
    let prev_state = self.decoder_state1.clone();
    decoder_out = self.run_decoder(&[y])?;
    let new_state = self.decoder_state1.clone();

    let state_diff = new_state.iter()
        .zip(prev_state.unwrap().iter())
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>();

    info!("State changed by: {:.6}", state_diff);
}
```

**Expected Output:**
```
Token 19: State changed by: 12.345678
Token 1010: State changed by: 8.765432
Token 1005: State changed by: 9.876543

✅ States evolving normally
```

**If FAILS:** State update bug. Check:
- States extracted from correct output index
- States assigned back to self
- No accidental state reset

---

### Test 3.2: Blank Token Probability Analysis
**Objective:** Understand why blank probability explodes

**Test Steps:**
1. Log joiner output logits for first 10 frames
2. Track blank logit vs best non-blank logit
3. Compare with Python sherpa-onnx logits
4. Identify divergence point

**Success Criteria:**
- Blank probability reasonable (< 80%)
- Non-blank tokens have competitive probabilities
- Distribution matches Python

**Implementation:**
```rust
// In decode_frames_with_state(), after joiner
let blank_logit = joiner_out[blank_id as usize];
let nonblank_logits: Vec<_> = (0..blank_id)
    .map(|i| (i, joiner_out[i as usize]))
    .collect();
let best_nonblank = nonblank_logits.iter()
    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
    .unwrap();

info!("Frame {}: blank_logit={:.4}, best_nonblank=({}, {:.4})",
    t, blank_logit, best_nonblank.0, best_nonblank.1);
```

**Expected Output:**
```
Frame 0: blank_logit=5.23, best_nonblank=(19, 6.78)
Frame 1: blank_logit=4.56, best_nonblank=(1010, 5.12)
...

✅ Balanced distribution
```

**If FAILS (current state):**
```
Frame 5: blank_logit=9.49, best_nonblank=(1010, 5.79)
❌ 40× probability gap indicates wrong encoder features
```

---

## Category 4: End-to-End Integration

### Test 4.1: Known-Good Audio Samples
**Objective:** Establish accuracy baseline on standard datasets

**Test Samples:**
1. **Short clear speech** (5 seconds) - `en-short.mp3`
   - Expected: "hey there how are you doing today"
   - Difficulty: EASY

2. **Medium speech with pauses** (15 seconds)
   - Expected: Full sentence with natural pauses
   - Difficulty: MEDIUM

3. **Long conversation** (60 seconds)
   - Expected: Multi-speaker dialogue
   - Difficulty: HARD

**Success Criteria:**
- Short: 100% word accuracy
- Medium: >95% word accuracy
- Long: >90% word accuracy

**Implementation:**
```bash
# Python baseline
python scripts/test_1_1b_with_sherpa.py examples/en-short.mp3

# Rust implementation
cargo run --example test_1_1b_direct examples/en-short.mp3

# Compare outputs
./scripts/compare_transcriptions.sh
```

---

### Test 4.2: Edge Case Handling
**Objective:** Verify robustness on challenging inputs

**Test Cases:**
1. **Silence** (5 seconds of quiet)
   - Expected: Empty string or silence token

2. **Music** (pure instrumental)
   - Expected: Empty or minimal output (no hallucination)

3. **Multiple speakers** (overlapping)
   - Expected: Graceful degradation, no crash

4. **Non-English** (Spanish/French)
   - Expected: Attempt transcription or reject cleanly

5. **Very short utterance** (< 1 second)
   - Expected: Handle gracefully, no crash

**Success Criteria:**
- No crashes or panics
- No infinite loops
- No memory leaks
- Reasonable output or clear rejection

---

### Test 4.3: Reference Implementation Comparison
**Objective:** Prove Rust matches Python sherpa-onnx

**Test Steps:**
1. Run both on 100 diverse samples
2. Compute Word Error Rate (WER) for each
3. Statistical comparison

**Success Criteria:**
- WER matches within 2% absolute
- No systematic bias (not consistently worse on certain phonemes)
- Performance within 20% of Python

**Implementation:**
```bash
# Generate test set
python scripts/prepare_test_set.py --output test_samples.txt

# Run both implementations
./scripts/batch_test_rust_vs_python.sh test_samples.txt
```

---

## Category 5: Performance & Regression

### Test 5.1: Memory Usage
**Objective:** Ensure no memory leaks or excessive allocation

**Test Steps:**
1. Transcribe 1000 files in loop
2. Monitor RSS memory
3. Verify no unbounded growth

**Success Criteria:**
- Memory usage stable after warmup
- No leaks detected by valgrind
- Peak memory < 2GB

---

### Test 5.2: Inference Speed
**Objective:** Verify performance is acceptable

**Benchmarks:**
- **CPU:** >1.0× real-time
- **GPU:** >5.0× real-time

---

## 🎯 Verification Checklist for Proposed Fixes

When implementing fixes for the 6 dB offset issue, verify:

### Pre-Fix Validation
- [ ] Test 1.1: Raw audio amplitudes match Python (baseline)
- [ ] Test 1.2: Power spectrum documented (before state)
- [ ] Test 1.4: Mel features exported (failing state documented)

### Post-Fix Validation (CRITICAL PATH)
- [ ] Test 1.2: Power spectrum matches Python (if fixed at FFT level)
- [ ] Test 1.3: Mel filterbank application correct
- [ ] Test 1.4: Full mel features match within 0.1 dB ⚡
- [ ] Test 2.2: Encoder output reasonable (no NaN/Inf)
- [ ] Test 3.2: Blank probability normalized (<80%)
- [ ] Test 4.1: "en-short.mp3" transcribes correctly ⚡
- [ ] Test 4.3: WER matches Python baseline

### Regression Prevention
- [ ] Test 5.1: No memory leaks
- [ ] Test 5.2: Performance maintained
- [ ] Test 4.2: Edge cases still handled
- [ ] Test 3.1: Decoder state management still correct

---

## 📂 Test Infrastructure

### Required Scripts (Already Exist)
- ✅ `scripts/diagnose_feature_mismatch.sh` - Master orchestrator
- ✅ `scripts/extract_python_mel_features.py` - Python reference
- ✅ `scripts/compare_mel_features.py` - Statistical comparison
- ✅ `scripts/verify_rust_csv.py` - Rust CSV validator
- ✅ `rust-crates/swictation-stt/examples/export_mel_features.rs` - Rust exporter

### Scripts to Create
- [ ] `scripts/compare_raw_audio.py` - Test 1.1
- [ ] `scripts/export_power_spectrum.rs` - Test 1.2 (Rust side)
- [ ] `scripts/compare_power_spectrum.py` - Test 1.2 (comparison)
- [ ] `scripts/export_mel_filterbank.rs` - Test 1.3 (Rust side)
- [ ] `scripts/compare_encoder_outputs.py` - Test 2.2
- [ ] `scripts/batch_test_rust_vs_python.sh` - Test 4.3
- [ ] `scripts/compare_transcriptions.sh` - Test 4.1 helper

### Test Data Repository
```
tests/
├── audio/
│   ├── en-short.mp3 (already exists)
│   ├── en-medium.mp3 (to create)
│   ├── en-long.mp3 (to create)
│   ├── silence.mp3
│   ├── music.mp3
│   └── multi-speaker.mp3
├── expected/
│   ├── en-short.txt ("hey there...")
│   ├── en-medium.txt
│   └── en-long.txt
└── reference/
    ├── python_mel_features/ (CSV dumps)
    ├── python_encoder_outputs/
    └── python_transcriptions/
```

---

## 🔄 Testing Workflow

### Phase 1: Isolate Root Cause (IN PROGRESS)
1. Run Test 1.1 (raw audio) → **ESTABLISH BASELINE**
2. Run Test 1.2 (power spectrum) → **FIND DIVERGENCE POINT**
3. Run Test 1.3 (mel filterbank) → **NARROW DOWN ISSUE**
4. Run Test 1.4 (full pipeline) → **CONFIRM DIAGNOSIS**

**Status:** Stuck at Test 1.4 (6 dB offset)

### Phase 2: Validate Fix (NEXT)
1. Implement fix based on Phase 1 findings
2. Re-run Tests 1.2-1.4 → **VERIFY OFFSET GONE**
3. Run Test 2.2 (encoder output) → **CHECK PROPAGATION**
4. Run Test 4.1 (en-short.mp3) → **END-TO-END PROOF**

**Exit Criteria:** Test 4.1 produces correct transcription

### Phase 3: Comprehensive Validation (FINAL)
1. Run full Test Category 2 (encoder)
2. Run full Test Category 3 (decoder)
3. Run full Test Category 4 (integration)
4. Run Test Category 5 (performance)

**Exit Criteria:** All tests pass, ready for production

---

## 🚨 Critical Success Metrics

### Mandatory for Release
- ✅ Test 1.4: Mel features match Python (<0.1 dB offset)
- ✅ Test 4.1: "en-short.mp3" transcribes correctly
- ✅ Test 4.3: WER within 2% of Python
- ✅ No crashes on 1000+ test files

### Confidence Indicators
- ✅ Correlation >0.99 between Rust and Python mel features
- ✅ Blank rate 50-70% (not 90%)
- ✅ Encoder outputs match within 1e-4
- ✅ State evolution verified

---

## 📊 Current Test Results Summary

### Passing ✅
- Decoder algorithm verification (matches C++ line-by-line)
- Blank token ID (1024)
- Basic mel parameters (80 bins, 20-7600 Hz)

### Failing ❌
- **Test 1.4:** 6 dB offset (460× linear) - ROOT CAUSE
- **Test 4.1:** "mmhmm" output instead of correct transcription
- **Test 3.2:** 90.9% blank rate, 40× probability gap

### Not Yet Run ⏸️
- Test 1.1 (raw audio comparison)
- Test 1.2 (power spectrum)
- Test 1.3 (mel filterbank isolation)
- Test 2.x (encoder verification)
- Tests 4.2, 4.3, 5.x (edge cases, comparison, performance)

---

## 🎓 Testing Philosophy

### Principles
1. **Isolate then integrate** - Fix components before testing full pipeline
2. **Compare with ground truth** - Always have Python baseline
3. **Automate everything** - Scripts should run with one command
4. **Document deviations** - Any difference from Python must be explained
5. **No false positives** - Test must actually verify what it claims

### Red Flags
- ⚠️ Tests that "mostly work" - should be 100% or documented failure
- ⚠️ Hard-coded test data - use diverse samples
- ⚠️ Ignoring small differences - 0.1 dB matters at scale
- ⚠️ No baseline comparison - always compare with working implementation

---

## 🔗 References

### Existing Documentation
- `/opt/swictation/docs/HIVE-MIND-DIAGNOSIS-RUST-MMHMM-BUG.md` - Main diagnosis
- `/opt/swictation/docs/aha-23-mel-offset-investigation.md` - Feature extraction investigation
- `/opt/swictation/docs/decoder-blank-token-analysis.md` - Decoder verification
- `/opt/swictation/docs/rust-ort-decoder-bug-analysis.md` - Debug analysis

### External References
- sherpa-onnx C++ implementation
- torchaudio.compliance.kaldi documentation
- NVIDIA NeMo Parakeet-TDT model docs

---

## 🚀 Next Immediate Actions

### For Implementation Team (Priority Order)

1. **Add raw audio export** to verify loading (Test 1.1)
   ```rust
   // In examples/export_raw_audio.rs
   pub fn export_raw_samples(audio: &[f32], path: &str, count: usize)
   ```

2. **Add power spectrum export** before mel filterbank (Test 1.2)
   ```rust
   // In audio.rs
   pub fn export_power_spectrum_csv(...)
   ```

3. **Create power spectrum comparison script** (Python)
   ```python
   # scripts/compare_power_spectrum.py
   ```

4. **Run diagnostic pipeline** with new tests
   ```bash
   ./scripts/diagnose_feature_mismatch.sh --verbose --all-tests
   ```

### Expected Discovery Path
- Test 1.1 → ✅ Audio loading is fine
- Test 1.2 → ❌ Power spectrum has offset OR ✅ FFT is fine
- Test 1.3 → ❌ Mel filterbank application is wrong (LIKELY)

Once Test 1.2/1.3 identifies the exact step, implement targeted fix and validate through entire test suite.

---

## 📈 Success Criteria

**Definition of Done:**
- All Category 1 tests pass (feature extraction correct)
- All Category 2 tests pass (encoder correct)
- Test 4.1 passes ("en-short.mp3" transcribes correctly)
- Test 4.3 passes (WER matches Python baseline)
- No regressions (Category 5 tests pass)

**Confidence Level:** HIGH
- We have clear metrics (0.1 dB tolerance)
- We have working baseline (Python sherpa-onnx)
- We have isolated the issue (mel features, not decoder)
- We have diagnostic tools (CSV comparison scripts)

**Timeline Estimate:**
- Phase 1 (isolate): 2-4 hours (Tests 1.1-1.3)
- Phase 2 (fix + validate): 4-8 hours (implement + retest)
- Phase 3 (comprehensive): 2-4 hours (full test suite)

**Total:** 8-16 hours for complete verification

---

**Status:** Ready for systematic debugging with Test 1.1-1.3
**Blocker:** Need to identify which step causes 6 dB offset
**Next:** Run Test 1.1 (raw audio comparison) to establish baseline

**Hive Mind Coordination:**
- Memory key: `hive/tester/strategy`
- Session ID: `swarm-1762761203503-wfwa5u13a`
- Dependencies: Findings from researcher, analyst, coder agents
