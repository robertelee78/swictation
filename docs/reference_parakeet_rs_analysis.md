# Reference Implementation Analysis: parakeet-rs (0.6B Model)

**Research Date**: 2025-11-10
**Researcher**: Hive Mind Research Agent
**Status**: ✅ Complete
**Reference Implementation**: `/var/tmp/parakeet-rs`

## Executive Summary

The parakeet-rs implementation provides a WORKING reference for the 0.6B Parakeet-TDT model. This analysis identifies **12 critical architectural differences** between the working 0.6B implementation and our broken 1.1B implementation.

## Critical Finding: Root Cause Hypothesis

**THE KEY DIFFERENCE**: The reference implementation uses:
1. ✅ **Per-feature normalization of mel-spectrograms** (our 1.1B removed this!)
2. ✅ **Standard Hann window** (not Povey window)
3. ✅ **Frequency range 0 Hz to Nyquist** (not 20-7600 Hz)
4. ✅ **Feature size 128** for 0.6B (vs 80 for 1.1B)

## Top 5 Critical Differences

### 1. 🔴 MEL FEATURE NORMALIZATION (MOST CRITICAL)

**Reference (0.6B - WORKS)**:
```rust
// Per-feature normalization AFTER log-mel
for feat_idx in 0..num_features {
    let mut column = mel_spectrogram.column_mut(feat_idx);
    let mean: f32 = column.iter().sum::<f32>() / num_frames as f32;
    let variance: f32 = column.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / num_frames as f32;
    let std = variance.sqrt().max(1e-10);
    for val in column.iter_mut() {
        *val = (*val - mean) / std;
    }
}
```

**Our Implementation (1.1B - BROKEN)**:
```rust
// CRITICAL FIX: DO NOT apply per-mel-bin normalization!
// NeMo models expect RAW log-mel features without per-feature normalization
```

**Impact**: We REMOVED this normalization, but 0.6B NEEDS IT! This is likely the root cause.

---

### 2. 🟡 FREQUENCY RANGE

| Implementation | Low Freq | High Freq | Comment |
|---------------|----------|-----------|---------|
| Reference 0.6B | 0 Hz | 8000 Hz (Nyquist) | Standard full spectrum |
| Our 1.1B | 20 Hz | 7600 Hz | Restricted sherpa-onnx range |

**Impact**: Missing low and high frequency information

---

### 3. 🟡 WINDOW FUNCTION

| Implementation | Window Type | Formula |
|---------------|-------------|---------|
| Reference 0.6B | Hann | `0.5 - 0.5 * cos(2πn/(N-1))` |
| Our 1.1B | Povey | `[0.5 - 0.5*cos(...)]^0.85` |

**Impact**: Different frequency resolution and spectral leakage

---

### 4. 🟢 ENCODER INPUT FORMAT

| Implementation | Shape | Transposed? |
|---------------|-------|-------------|
| Reference 0.6B | (batch, 128 features, time) | ✅ YES |
| Our 1.1B | (batch, time, 80 features) | ❌ NO |

**Impact**: Auto-detected correctly in our implementation

---

### 5. 🟢 MODEL ARCHITECTURE

| Component | Reference 0.6B | Our 1.1B |
|-----------|---------------|----------|
| Encoder | encoder.onnx | encoder.onnx |
| Decoder | decoder_joint.onnx (combined) | decoder.onnx + joiner.onnx (separate) |
| Feature Size | 128 mel bins | 80 mel bins |
| Vocab Size | 8193 tokens | 1025 tokens |
| Duration Prediction | Yes (5 classes) | Unknown (needs verification) |

**Impact**: Different model architectures require different inference code

---

## Complete Architectural Comparison

### Audio Preprocessing Pipeline

#### Reference 0.6B (WORKS):
```
1. Load audio → mono, 16kHz
2. Apply preemphasis (coef=0.97)
3. STFT with Hann window
4. Mel filterbank (0-8000 Hz, 128 bins)
5. Log scaling
6. ✅ PER-FEATURE NORMALIZATION (mean=0, std=1)
```

#### Our 1.1B (BROKEN):
```
1. Load audio → mono, 16kHz
2. ✅ AUDIO SAMPLE NORMALIZATION (mean=0, std=1)
3. Apply preemphasis (coef=0.97)
4. STFT with Povey window
5. Mel filterbank (20-7600 Hz, 80 bins)
6. Log scaling
7. ❌ NO PER-FEATURE NORMALIZATION
```

**Key Difference**: Order and type of normalization!

---

## Decoder Greedy Search Comparison

### Reference 0.6B (Single decoder_joint model):
```rust
while t < time_steps {
    // 1. Get encoder frame
    let frame = encoder_out.slice(s![0, .., t]);

    // 2. Run decoder_joint with encoder + target + states
    let outputs = decoder_joint.run(inputs!(
        "encoder_outputs" => frame,
        "targets" => [last_token],
        "target_length" => [1],
        "input_states_1" => state_h,
        "input_states_2" => state_c
    ));

    // 3. Split output into vocab + duration logits
    let vocab_logits = outputs[0..vocab_size];
    let duration_logits = outputs[vocab_size..];

    // 4. Greedy select token and duration
    let token = argmax(vocab_logits);
    let duration = argmax(duration_logits);

    // 5. If non-blank, update states
    if token != blank_id {
        state_h = outputs.state_h;
        state_c = outputs.state_c;
        emit(token);
    }

    // 6. Advance frame pointer by duration
    t += duration.max(1);
}
```

### Our 1.1B (Separate decoder + joiner):
```rust
while t < num_frames {
    // 1. Get encoder frame
    let encoder_frame = encoder_out.slice(s![0, .., t]);

    // 2. Get decoder output (separate model)
    let decoder_out = run_decoder([last_token], states);

    // 3. Run joiner to combine encoder + decoder
    let logits = run_joiner(encoder_frame, decoder_out);

    // 4. Greedy select token
    let token = argmax(logits);

    // 5. If non-blank, update decoder
    if token != blank_id {
        decoder_out = run_decoder([token], states);
        emit(token);
    }

    // 6. Advance by skip amount
    t += skip.max(1);
}
```

**Key Difference**: Combined vs separated decoder/joiner architecture

---

## Blank Token Handling

| Aspect | Reference 0.6B | Our 1.1B |
|--------|---------------|----------|
| Blank ID | `vocab_size - 1` (8192) | Searched from tokens.txt (1024) |
| Location | Last token in vocab | Found by name "<blk>" or "<blank>" |
| Hardcoded | ✅ Yes | ❌ No (dynamic lookup) |

---

## Duration Prediction

### Reference 0.6B:
- ✅ Explicit 5-class duration prediction
- Output size: `vocab_size + 5` (8198 total)
- Duration affects frame skipping directly

### Our 1.1B:
- ❓ Unknown if duration prediction exists
- Output size: Needs verification (likely vocab_size only = 1030)
- Skip logic based on blank/non-blank + token count

---

## Recommended Actions (Priority Order)

### 🔴 CRITICAL - Priority 1: Feature Extraction
1. **Restore per-feature normalization** in 1.1B implementation
2. Test with/without to determine which matches 1.1B training
3. Export mel features from both implementations for numerical comparison
4. Verify frequency range (test 0-8000 vs 20-7600)
5. Test Hann vs Povey window effect

### 🟡 HIGH - Priority 2: Decoder Verification
1. Verify joiner output size (1030 vs 1030+5)
2. Check if 1.1B uses duration prediction
3. Validate blank token ID location (1024 vs last)
4. Compare decoder state handling

### 🟢 MEDIUM - Priority 3: Integration Testing
1. Create side-by-side comparison tool
2. Test both implementations on same audio
3. Measure encoder output similarity
4. Validate decoder behavior matches

---

## Key Files for Reference

### Working Implementation (0.6B):
```
/var/tmp/parakeet-rs/
├── src/
│   ├── audio.rs           ← Per-feature normalization HERE!
│   ├── model_tdt.rs       ← Combined decoder_joint logic
│   ├── decoder_tdt.rs     ← Token-to-text conversion
│   ├── config.rs          ← PreprocessorConfig with feature_size=128
│   └── vocab.rs           ← Vocabulary handling
└── examples/
    └── transcribe.rs      ← Usage example
```

### Our Implementation (1.1B):
```
/opt/swictation/rust-crates/swictation-stt/
├── src/
│   ├── audio.rs           ← NO per-feature normalization
│   └── recognizer_ort.rs  ← Separate decoder + joiner
```

---

## Experimental Plan

### Experiment 1: Restore Per-Feature Normalization
```rust
// Add to audio.rs after log-mel extraction
pub fn extract_mel_features(&mut self, samples: &[f32]) -> Result<Array2<f32>> {
    // ... existing code ...
    let log_mel = mel_spec.mapv(|x| (x + 1e-10).ln());

    // EXPERIMENTAL: Add per-feature normalization like 0.6B
    let num_frames = log_mel.nrows();
    let num_features = log_mel.ncols();

    for feat_idx in 0..num_features {
        let mut column = log_mel.column_mut(feat_idx);
        let mean: f32 = column.sum() / num_frames as f32;
        let variance: f32 = column.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f32>() / num_frames as f32;
        let std = variance.sqrt().max(1e-10);

        for val in column.iter_mut() {
            *val = (*val - mean) / std;
        }
    }

    Ok(log_mel)
}
```

**Expected Result**: If this fixes the "mmhmm" bug, we've found the root cause!

---

## Conclusion

The reference parakeet-rs implementation reveals that **per-feature normalization** is critical for the 0.6B model. We removed this normalization for 1.1B based on sherpa-onnx investigation, but this may have been incorrect.

**Hypothesis**: The 1.1B model was also trained with per-feature normalized mel-spectrograms, and removing this preprocessing step causes the model to output gibberish.

**Next Steps**:
1. Test per-feature normalization on 1.1B model
2. Compare mel features numerically with Python implementation
3. Verify all other preprocessing parameters
4. Document which configuration works for 1.1B

---

**Research Status**: ✅ COMPLETE
**Confidence**: HIGH (direct code comparison from working implementation)
**Impact**: CRITICAL (explains core feature extraction differences)
**Actionable**: YES (clear experimental path forward)
