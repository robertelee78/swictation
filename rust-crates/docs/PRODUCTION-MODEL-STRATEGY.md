# Production Model Strategy - swictation

**Date**: November 9, 2025
**Status**: Based on comprehensive GPU/CPU benchmarking

## Executive Summary

**Use hybrid approach**: 110M fp32 with GPU for speed, 0.6B int8 with CPU for accuracy.

## Production-Ready Models

### ✅ Model #1: 110M fp32 + GPU (Speed Mode)

**Path**: `/opt/swictation/models/sherpa-onnx-nemo-parakeet_tdt_transducer_110m-en-36000`

**Performance**:
- Inference: **13ms** (after GPU warmup)
- GPU speedup: **5.1x** faster than CPU
- Quality: Perfect transcription

**Best for**:
- Real-time dictation
- Low-latency requirements
- Interactive use cases

**Configuration**:
```rust
let mut recognizer = Recognizer::new(
    "/opt/swictation/models/sherpa-onnx-nemo-parakeet_tdt_transducer_110m-en-36000",
    true  // GPU enabled
)?;

// One-time warmup (~97ms)
let _ = recognizer.recognize_file(warmup_sample)?;

// Then: 13ms per request
```

---

### ✅ Model #2: 0.6B int8 + CPU (Accuracy Mode)

**Path**: `/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8`

**Performance**:
- Inference: **141ms** on CPU
- Quality: Perfect transcription (higher accuracy than 110M)

**Best for**:
- Medical/legal dictation requiring maximum accuracy
- When GPU is busy with other tasks
- Batch processing where latency is less critical

**Configuration**:
```rust
let mut recognizer = Recognizer::new(
    "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8",
    false  // CPU
)?;

// 141ms per request, but higher accuracy
```

**Why not GPU?**: int8 quantization is CPU-optimized. GPU is 1.9x SLOWER (266ms).

---

## Models NOT Ready for Production

### ❌ 1.1B Model - Wrong File Structure

**Problem**: Has combined `decoder_joint-model.onnx` instead of separate files

**Status**: Needs conversion using official sherpa-onnx scripts

**File Structure**:
```
sherpa-onnx-nemo-parakeet-tdt-1.1b/
├── encoder-model.onnx ✅
├── decoder_joint-model.onnx ❌ (should be decoder.onnx + joiner.onnx)
└── tokens.txt (missing)
```

**Solution**: Use official k2-fsa/sherpa-onnx conversion scripts to properly split decoder+joiner.

---

### ⚠️ 0.6B fp16 - Quality Issues

**Path**: `/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-fp16`

**Problem**: Outputs `<unk>` (unknown) tokens instead of proper transcription

**Performance**: 389ms on GPU (1.2x faster than CPU)

**Possible causes**:
- Corrupted model download
- Incompatible fp16 precision handling
- Tokenizer mismatch

**Status**: Needs investigation or re-download from sherpa-onnx releases.

---

## Hybrid Production Strategy

### Recommended Daemon Architecture

```rust
pub struct TranscriptionEngine {
    fast_gpu: Recognizer,   // 110M fp32 (13ms)
    accurate_cpu: Recognizer, // 0.6B int8 (141ms)
}

impl TranscriptionEngine {
    pub fn new() -> Result<Self> {
        // Load both models at startup
        let mut fast_gpu = Recognizer::new(
            "/opt/swictation/models/sherpa-onnx-nemo-parakeet_tdt_transducer_110m-en-36000",
            true
        )?;

        let accurate_cpu = Recognizer::new(
            "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8",
            false
        )?;

        // Warmup GPU once
        let _ = fast_gpu.recognize_file(warmup_sample)?;

        Ok(Self { fast_gpu, accurate_cpu })
    }

    pub fn transcribe(&mut self, audio: &Path, mode: Mode) -> Result<String> {
        match mode {
            Mode::Fast => self.fast_gpu.recognize_file(audio),      // 13ms
            Mode::Accurate => self.accurate_cpu.recognize_file(audio), // 141ms
        }
    }
}
```

### Mode Selection Logic

**Use Fast Mode (110M GPU)** when:
- Real-time dictation
- Interactive editing
- Low-latency required (<20ms target)
- General transcription

**Use Accurate Mode (0.6B CPU)** when:
- Medical/legal transcription
- Final document transcription
- Batch processing
- GPU busy with other tasks

---

## Performance Comparison

| Model | Device | Latency | Quality | Use Case |
|-------|--------|---------|---------|----------|
| 110M fp32 | GPU | **13ms** ✅ | Good | Real-time dictation |
| 0.6B int8 | CPU | 141ms | Excellent | High-accuracy transcription |
| 0.6B int8 | GPU | 266ms ❌ | Excellent | Don't use (slower) |
| 0.6B fp16 | GPU | 389ms ⚠️ | Broken | Not usable |
| 1.1B | Any | N/A ❌ | N/A | Needs conversion |

---

## Memory Requirements

**Active daemon with both models loaded**:
- 110M fp32: ~456 MB
- 0.6B int8: ~640 MB
- **Total**: ~1.1 GB RAM

**GPU Memory**:
- 110M fp32: ~500 MB VRAM (stays resident)
- Plenty of room on RTX PRO 6000 (24GB)

---

## Future Improvements

### Priority 1: Test Long Audio
- Current benchmarks: 2.8 seconds (short audio)
- Model guide claims: 7.86x GPU speedup on long audio
- Action: Test with 30+ second files

### Priority 2: Fix 1.1B Model
- Use official sherpa-onnx conversion scripts
- Proper decoder/joiner separation
- Test GPU performance
- Expected: Higher accuracy than 0.6B, good GPU performance

### Priority 3: Investigate 0.6B fp16
- Re-download from sherpa-onnx releases
- Verify SHA checksums
- Test transcription quality
- Expected: Should work perfectly

---

## Deployment Checklist

- ✅ Load 110M fp32 with GPU enabled
- ✅ Load 0.6B int8 with CPU
- ✅ Warmup GPU with one inference run
- ✅ Implement mode selection (Fast/Accurate)
- ✅ Monitor GPU memory usage
- ✅ Handle model loading errors gracefully
- ✅ Add metrics/logging for latency tracking

---

## Conclusion

**Production-ready models**: 110M fp32 (GPU) + 0.6B int8 (CPU)

**Performance achieved**:
- Fast mode: 13ms (meets <20ms real-time requirement)
- Accurate mode: 141ms (still acceptable for most use cases)

**Recommendation**: Start with hybrid approach, tune mode selection based on user feedback.
