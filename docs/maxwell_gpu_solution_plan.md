# Maxwell GPU Support - Solution Plan

## Current State Analysis

### Architecture
```
SttEngine enum:
├── Parakeet0_6B(Recognizer)      ← Uses sherpa-rs wrapper
│   └── sherpa_rs::TransducerRecognizer
│       └── libsherpa-onnx-c-api.so (requires VERS_1.22.0)
│           └── libonnxruntime.so 1.22.0
│
└── Parakeet1_1B(OrtRecognizer)   ← Direct ONNX Runtime via ort crate ✅
    └── ort::Session (uses latest ONNX Runtime)
        └── libonnxruntime.so (any version, loaded at runtime via ORT_DYLIB_PATH)
```

### Problem Statement
**Maxwell GPUs (Quadro M2200, GTX 750/900 series) crash with current setup:**

1. **Root Cause**: cuDNN 9.15.1 (used in Modern/Latest GPU packages) dropped RNN/LSTM support for compute capability <6.0 (Maxwell = 5.0-5.2)

2. **Crash Location**: `cudnnSetDropoutDescriptor()` during Silero VAD LSTM initialization

3. **Version Lock Issue**:
   - npm package contains pre-compiled sherpa-onnx binaries linked against ONNX Runtime 1.22.0
   - Symbol versioning prevents using ONNX Runtime 1.23.x
   - We need GPU libs with ONNX Runtime 1.22.0 OR bypass sherpa-onnx entirely

## Solution Options

### Option A: Build GPU Libs with ONNX Runtime 1.22.0 (Quick Fix)
**Pros:**
- Minimal code changes
- Keeps sherpa-onnx integration
- Straightforward build process

**Cons:**
- Locked to ONNX Runtime 1.22.0 permanently
- Still depends on external sherpa-onnx binaries
- Cannot use latest ONNX Runtime features/optimizations
- Requires matching exact versions for all future updates

**Effort:** 2-3 hours (rebuild + test)

### Option B: Extend OrtRecognizer for 0.6B Model (Recommended)
**Pros:**
- No version locking - can use latest ONNX Runtime 1.23.2+
- Consistent architecture (both models use same code path)
- Full control over ONNX Runtime configuration
- Easier to maintain long-term
- Already proven working for 1.1B model
- Reduces binary dependencies

**Cons:**
- More code changes required
- Need to implement TDT decoder logic
- More testing required

**Effort:** 6-8 hours (implementation + testing)

## Recommended Approach: Option B

### Implementation Plan

#### Phase 1: Extend OrtRecognizer to Support 0.6B Model
**File:** `/opt/swictation/rust-crates/swictation-stt/src/recognizer_ort.rs`

1. **Add model detection** (0.6B vs 1.1B):
   - Check model directory for encoder size or metadata
   - Auto-detect decoder hidden size (512 for 0.6B, 640 for 1.1B)

2. **Support multiple quantization formats**:
   - FP32 (encoder.onnx, decoder.onnx, joiner.onnx)
   - FP16 (encoder.fp16.onnx, etc.) ← **Best for GPU**
   - INT8 (encoder.int8.onnx, etc.)

3. **Handle different model shapes**:
   - 0.6B: decoder state (2, batch, 512)
   - 1.1B: decoder state (2, batch, 640)

4. **Add model-specific configuration**:
   ```rust
   pub struct ModelConfig {
       decoder_hidden_size: usize,  // 512 or 640
       has_external_weights: bool,   // 1.1B yes, 0.6B no
       model_type: ModelType,        // Enum: Parakeet0_6B | Parakeet1_1B
   }
   ```

#### Phase 2: Update SttEngine to Use OrtRecognizer for 0.6B
**File:** `/opt/swictation/rust-crates/swictation-daemon/src/pipeline.rs`

Replace all `Recognizer::new()` calls with `OrtRecognizer::new()` for 0.6B model:
```rust
// Before:
let recognizer = Recognizer::new(&config.stt_0_6b_model_path, true)?;
SttEngine::Parakeet0_6B(recognizer)

// After:
let ort_recognizer = OrtRecognizer::new(&config.stt_0_6b_model_path, true)?;
SttEngine::Parakeet0_6B_Ort(ort_recognizer)
```

#### Phase 3: Build GPU Libraries with Latest ONNX Runtime
1. **Update Dockerfile.cuda11** to use ONNX Runtime 1.23.2 (already done)
2. **Build with CUDA 11.8 + cuDNN 8.9.7** for Maxwell support
3. **Package libraries** for Legacy variant

#### Phase 4: Testing
1. **Unit tests**: Verify OrtRecognizer works with 0.6B model files
2. **Integration tests**: End-to-end pipeline on Maxwell GPU
3. **Performance validation**: Compare latency/accuracy vs sherpa-rs
4. **Regression testing**: Ensure 1.1B still works

### Dependencies

**Rust crates** (already in Cargo.toml):
- `ort` = "2.0.0-rc.10" (ONNX Runtime bindings)
- `ndarray` (tensor operations)
- `tracing` (logging)

**System libraries** (need to build):
- ONNX Runtime 1.23.2 with CUDA 11.8 + cuDNN 8.9.7
- CUDA Runtime 11.8 libraries
- cuDNN 8.9.7 libraries

### Model Files Required

For 0.6B model directory:
```
/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b/
├── encoder.fp16.onnx    (or encoder.onnx for FP32)
├── decoder.fp16.onnx
├── joiner.fp16.onnx
└── tokens.txt
```

### Fallback Strategy

If Option B proves too complex during implementation:
1. Fall back to Option A (build with ONNX Runtime 1.22.0)
2. Document version lock in README
3. Plan migration to Option B for next major release

### Success Criteria

1. ✅ Maxwell GPU (Quadro M2200) runs daemon without crashes
2. ✅ VAD + STT pipeline works end-to-end
3. ✅ Latency within 10% of current sherpa-rs implementation
4. ✅ WER matches or improves on current 0.6B results
5. ✅ No regression on Pascal/Turing/Ampere GPUs
6. ✅ CPU fallback still works
7. ✅ 1.1B model unaffected

### Timeline

- **Phase 1** (Code): 3-4 hours
- **Phase 2** (Integration): 1-2 hours
- **Phase 3** (Build): 1 hour
- **Phase 4** (Testing): 2 hours

**Total: 7-9 hours** (Option B)
**Fallback: 2-3 hours** (Option A if needed)

## Decision Point

**Proceed with Option B?** Yes/No

If Yes → Start with Phase 1
If No → Implement Option A as quick fix
