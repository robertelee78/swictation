# 🔧 QUICK FIX CHECKLIST: Rust "mmhmm" Bug

**Problem**: Rust produces "mmhmm" gibberish, Python works perfectly
**Root Cause**: 6.13 dB mel feature offset (460× in linear space)
**Status**: Diagnosed, ready to fix

---

## ⚡ IMMEDIATE ACTIONS (Do These First)

### 1️⃣ Add Debug Logging to Audio Loading

**File**: `/opt/swictation/rust-crates/swictation-stt/src/audio.rs`

**Location**: Line ~268 in `extract_mel_features()` function

**Add BEFORE any processing**:
```rust
// DEBUG: Raw audio amplitude check
let rms = (samples.iter().map(|&x| x * x).sum::<f32>() / samples.len() as f32).sqrt();
eprintln!("🔊 RAW AUDIO: samples={}, RMS={:.6}, mean={:.6}, min={:.6}, max={:.6}",
         samples.len(), rms,
         samples.iter().sum::<f32>() / samples.len() as f32,
         samples.iter().fold(f32::INFINITY, |a, &b| a.min(b)),
         samples.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b)));
```

**Expected Python RMS**: 0.045716
**If Rust RMS differs significantly**: Audio loading is wrong!

---

### 2️⃣ Test Without Resampling

**Create 16kHz test file**:
```bash
ffmpeg -i examples/en-short.mp3 -ar 16000 examples/en-short-16k.wav
```

**Test Rust**:
```bash
cd rust-crates/swictation-stt
cargo build --release --example export_mel_features
./target/release/examples/export_mel_features ../../examples/en-short-16k.wav /tmp/rust_16k.csv
```

**Test Python**:
```bash
python3.12 scripts/extract_python_mel_features.py examples/en-short-16k.wav /tmp/python_16k.csv
```

**Compare**:
```bash
python3.12 scripts/compare_mel_features.py /tmp/rust_16k.csv /tmp/python_16k.csv
```

**If offset disappears**: Resampling is the bug!
**If offset persists**: Continue to step 3

---

### 3️⃣ Verify Mel Filterbank Coefficients

**Add to audio.rs** after mel filterbank creation (line ~332):
```rust
// DEBUG: Verify mel filterbank
eprintln!("🔬 MEL FILTERBANK:");
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
mel_fb = torchaudio.functional.melscale_fbanks(
    n_freqs=257, n_mels=80, sample_rate=16000,
    f_min=20.0, f_max=7600.0, norm='slaney'
)
for i in range(5):
    print(f"  Bin {i}: sum={mel_fb[i].sum():.6f}, max={mel_fb[i].max():.6f}")
```

**If sums differ**: Filterbank scaling is wrong!

---

## 📋 FULL DIAGNOSTIC WORKFLOW

### Step 1: Build & Test Current State
```bash
cd /opt/swictation
cargo build --release --manifest-path rust-crates/swictation-stt/Cargo.toml
./scripts/diagnose_feature_mismatch.sh examples/en-short.mp3
```

**Expected**: ~6 dB offset detected

---

### Step 2: Apply Debug Logging Patches

Edit these files:
- `/opt/swictation/rust-crates/swictation-stt/src/audio.rs` (add logging)
- Rebuild: `cargo build --release`

---

### Step 3: Rerun with Logging
```bash
RUST_LOG=debug cargo run --release --example export_mel_features examples/en-short.mp3 /tmp/rust_debug.csv 2>&1 | tee /tmp/rust_debug.log
```

**Check log for**:
- RAW AUDIO RMS value
- Sample count
- Mel filterbank stats

---

### Step 4: Compare with Python Reference
```bash
python3.12 scripts/extract_python_mel_features.py examples/en-short.mp3 /tmp/python_ref.csv 2>&1 | tee /tmp/python_debug.log
```

**Check log for**:
- Audio RMS value
- Sample count
- Any preprocessing mentions

---

### Step 5: Analyze Differences

Compare debug logs side-by-side:
```bash
diff /tmp/rust_debug.log /tmp/python_debug.log
```

Look for:
- RMS mismatch → Audio loading bug
- Sample count mismatch → Padding/chunking bug
- Filterbank mismatch → Mel construction bug

---

## 🎯 SUCCESS CRITERIA

### ✅ Phase 1: Diagnosis
- [x] Mel offset quantified: 6.13 dB
- [x] Tools created: export_mel_features, compare scripts
- [x] Correlation measured: 0.86

### ⏳ Phase 2: Root Cause (In Progress)
- [ ] Identify exact step causing offset
- [ ] Audio RMS matches Python
- [ ] Sample counts match
- [ ] Mel filterbank coefficients match

### 🎯 Phase 3: Fix Validated
- [ ] Max mel difference < 0.001
- [ ] Correlation > 0.999
- [ ] Rust transcription >95% accurate
- [ ] No more "mmhmm" gibberish!

---

## 🚨 LIKELY CULPRITS (Ranked by Probability)

1. **Resampling** (60% confidence) - Test with 16kHz input
2. **Audio loading** (25% confidence) - Check RMS values
3. **Hidden torchaudio constant** (10% confidence) - Read source
4. **Mel filterbank scaling** (5% confidence) - Verify coefficients

---

## 📞 IF STUCK

### Option 1: Use sherpa-onnx Feature Extraction
```rust
// Instead of custom audio.rs processing, use sherpa-onnx's feature extractor
// This guarantees bit-exact match with Python
```

### Option 2: Direct FFT Comparison
```bash
# Export FFT output before mel filterbank
# Compare with Python FFT output
# Find divergence point
```

### Option 3: Ask for Help
```bash
# Post to sherpa-onnx GitHub with:
# - Rust debug log
# - Python debug log
# - Comparison report
# - This checklist
```

---

## 📁 KEY FILES

### Documentation
- `docs/HIVE-CODER-SYNTHESIS.md` - Full analysis (this is comprehensive!)
- `docs/aha-23-mel-offset-investigation.md` - Offset investigation
- `docs/HIVE-MIND-DIAGNOSIS-RUST-MMHMM-BUG.md` - Collective diagnosis

### Code
- `rust-crates/swictation-stt/src/audio.rs` - Audio processing (needs debug logging)
- `rust-crates/swictation-stt/src/recognizer_ort.rs` - Decoder (verified correct ✅)

### Diagnostic Tools
- `scripts/diagnose_feature_mismatch.sh` - Master orchestrator
- `scripts/compare_mel_features.py` - Element-wise comparison
- `scripts/extract_python_mel_features.py` - Python reference

### Test Files
- `examples/en-short.mp3` - Test audio (6.17 seconds)
- `examples/en-short-16k.wav` - Need to create (no resampling)

---

## ⏱️ ESTIMATED TIME

- **Adding debug logging**: 5 minutes
- **Testing with 16kHz input**: 10 minutes
- **Analyzing results**: 15 minutes
- **Implementing fix**: 30-60 minutes
- **Validation**: 15 minutes

**Total**: ~1.5-2 hours to complete fix

---

## 🏆 WHAT SUCCESS LOOKS LIKE

**Before (Current)**:
```
Frame 0: token=1024 ('<blk>'), duration=1
Frame 1: token=1024 ('<blk>'), duration=1
...
Result: "mmhmm" (92.5% blank)
```

**After (Fixed)**:
```
Frame 0: token=19 ('▁'), duration=1
Frame 1: token=518 ('T'), duration=2
Frame 5: token=615 ('he'), duration=1
...
Result: "The quick brown fox..." (95%+ accuracy)
```

---

**Created**: 2025-11-10
**By**: Coder Agent (Hive Mind)
**Status**: ✅ Ready to execute

*Let's fix this! 🚀*
