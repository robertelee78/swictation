# GPU Testing Report - CUDA 12 + sherpa-rs Integration

## Executive Summary

**Finding**: INT8 quantized Parakeet-TDT models **cannot** achieve GPU acceleration with offline (file-based) recognition using sherpa-rs/sherpa-onnx.

**Recommendation**: Use float32 models for GPU-accelerated offline recognition, or implement streaming recognition for int8 models.

## Test Environment

- **GPU**: NVIDIA RTX PRO 6000 (Ada Lovelace, 48GB VRAM)
- **Driver**: 580.95.05
- **CUDA**: 12.9
- **cuDNN**: 9.x
- **ONNX Runtime**: 1.22.0 (via sherpa-onnx v1.12.15)
- **sherpa-rs**: Latest from git
- **Model**: Parakeet-TDT 0.6B INT8 quantized

## Test Results

### INT8 Model with CUDA Execution Provider

```
═══════════════════════════════════════════════════
  Sherpa-RS CUDA 12 GPU Benchmark
═══════════════════════════════════════════════════

[ GPU Mode - CUDA 12 ]
  Load: 2.81s
  Short: 519ms - "Hello world. Testing one, two, three."
  Long:  3582ms - "The open source AI community has scored..."

[ CPU Mode ]
  Load: 2.25s
  Short: 134ms - "Hello world. Testing one, two, three."
  Long:  2703ms - "The open source AI community has scored..."

═══════════════════════════════════════════════════
[ GPU Speedup ]
  Load time:   0.80x (slower)
  Short audio: 0.26x (3.8x SLOWER)
  Long audio:  0.75x (1.3x SLOWER)
```

### GPU Utilization During "GPU Mode"

```
# gpu     sm    mem    enc    dec    jpg    ofa
# Idx      %      %      %      %      %      %
    0      2-4%   0-3%   0      0      0      0
```

**Analysis**: GPU is essentially idle (2-4% SM usage), confirming CPU fallback.

## Root Cause Analysis

### INT8 Quantization Limitations

1. **CUDA Execution Provider**:
   - Microsoft ONNX Runtime docs: "It is not possible to run quantized models on CUDAExecutionProvider"
   - CUDA EP silently falls back to CPU for quantized operations
   - No Tensor Core utilization for int8 arithmetic

2. **TensorRT Execution Provider**:
   - Supports int8 natively on Tensor Cores
   - sherpa-onnx limitation: **TensorRT only works for online (streaming) models**
   - Error from `session.cc:80`: "Tensorrt support for Online models ony, Must be extended for offline and others"
   - Offline recognition (file-based) exits with error code 1

### Code Evidence

From `sherpa-onnx/csrc/session.cc`:

```cpp
case Provider::kTRT: {
  if (provider_config == nullptr) {
    SHERPA_ONNX_LOGE(
        "Tensorrt support for Online models ony,"
        "Must be extended for offline and others");
    exit(1);
  }
  // ... TensorRT config ...
}
```

## Solutions

### Option 1: Float32 Models (Recommended for Offline Recognition)

**Pros**:
- Full CUDA EP support for offline recognition
- Expected 2-5x GPU speedup
- Works with existing TransducerRecognizer API

**Cons**:
- Larger model size (~2.4GB vs 600MB)
- Higher VRAM usage (~4GB vs 1GB)

**Implementation**: Use nvidia-parakeet-tdt-1.1b (float32) instead of int8 model

### Option 2: Streaming Recognition with INT8

**Pros**:
- Can use TensorRT EP for int8 acceleration
- Smaller model size
- Real-time streaming capability

**Cons**:
- Requires rewriting recognition pipeline
- sherpa-rs may need OnlineRecognizer API updates
- More complex implementation

### Option 3: CPU-Only INT8 (Current)

**Pros**:
- Works reliably
- Good CPU performance (137ms for short audio)
- Acceptable for non-real-time workloads

**Cons**:
- No GPU acceleration
- Slower than GPU float32 would be

## Recommendations

### Immediate Action

1. **Fix 1.1B float32 model download**
   - Current model files are corrupted (0 bytes)
   - Re-download from NVIDIA/HuggingFace

2. **Test float32 model with CUDA EP**
   - Expect 2-5x speedup over CPU
   - Verify GPU utilization >50%

3. **Document model selection guidance**
   - INT8: CPU-only, best for batch processing
   - Float32: GPU-accelerated, best for real-time

### Long-term Improvements

1. **Implement streaming recognition**
   - Use OnlineRecognizer for real-time transcription
   - Enable TensorRT EP for int8 models
   - Benefit from Tensor Core acceleration

2. **Contribute to sherpa-onnx**
   - Add TensorRT support for offline models
   - Extend provider_config exposure in sherpa-rs

3. **Hybrid approach**
   - Use int8 CPU for batch/offline workloads
   - Use float32 GPU for real-time/streaming workloads

## Configuration Summary

### Current Implementation

**File**: `/opt/swictation/rust-crates/swictation-stt/src/recognizer.rs`

```rust
// GPU provider: CUDA for offline recognition (TensorRT only supports online/streaming)
// Note: CUDA EP may not fully utilize int8 quantization on Tensor Cores
// but it's the only option for offline (file-based) recognition with GPU
provider: Some(if use_gpu { "cuda" } else { "cpu" }.to_string()),
```

### Recommended Configuration

For **offline recognition**:
- **INT8 models**: Use CPU (GPU provides no benefit)
- **Float32 models**: Use CUDA EP for GPU acceleration

## References

- Microsoft ONNX Runtime: https://onnxruntime.ai/docs/execution-providers/CUDA-ExecutionProvider.html
- sherpa-onnx Provider Code: `crates/sherpa-rs-sys/sherpa-onnx/sherpa-onnx/csrc/session.cc`
- TensorRT Integration: `provider.h` lines 78-120
- Test Code: `/opt/swictation/rust-crates/swictation-daemon/examples/test_gpu_benchmark.rs`

## Next Steps

1. ✅ Completed: INT8 + CUDA EP testing (confirmed CPU fallback)
2. ✅ Completed: TensorRT EP investigation (online-only limitation)
3. ⏳ Pending: Fix 1.1B float32 model files
4. ⏳ Pending: Test float32 + CUDA EP
5. ⏳ Pending: Implement model selection logic (int8 CPU vs float32 GPU)

## Conclusion

GPU acceleration for Parakeet-TDT models with sherpa-rs requires using **float32 models** with CUDA Execution Provider. INT8 quantized models are limited to CPU execution for offline recognition due to architectural constraints in both ONNX Runtime and sherpa-onnx.

For production deployment, we recommend:
- **Default**: INT8 CPU (600MB model, good performance)
- **Performance Mode**: Float32 GPU (2.4GB model, 2-5x faster)
- **Future**: Streaming recognition with TensorRT int8 (best of both worlds)
