# Parakeet-TDT Model Selection Guide

## Quick Decision Matrix

| Audio Length | Accuracy Need | Model Choice | Acceleration | Expected Speed |
|--------------|---------------|--------------|--------------|----------------|
| < 3 seconds | Standard | 0.6B int8 | CPU | ~130ms |
| < 3 seconds | High | 110M float32 | CPU | ~70ms |
| > 3 seconds | Standard | 110M float32 | **GPU** | ~20-50ms/s |
| > 3 seconds | High | 0.6B float32* | **GPU** | ~10-30ms/s |
| > 3 seconds | Highest | 1.1B float32* | **GPU** | ~15-40ms/s |

\* *Requires conversion from NeMo (see below)*

## Model Characteristics

### 110M Float32 (sherpa-onnx-nemo-parakeet_tdt_transducer_110m-en-36000)

**Status**: ✅ Available from k2-fsa/sherpa-onnx

**Specs**:
- Parameters: 110 million
- Size: 456 MB
- Quantization: float32 (full precision)
- Language: English
- GPU: ✅ **7.86x speedup** on long audio

**Best For**:
- Production GPU deployment TODAY
- Long audio transcription (>3 seconds)
- Balance between speed and accuracy

**Trade-offs**:
- Lower accuracy than 0.6B/1.1B models
- GPU slower on very short audio (<3s) due to overhead

**Usage**:
```rust
let recognizer = Recognizer::new(
    "/opt/swictation/models/sherpa-onnx-nemo-parakeet_tdt_transducer_110m-en-36000",
    true  // use GPU
)?;
```

### 0.6B INT8 (sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8)

**Status**: ✅ Available from k2-fsa/sherpa-onnx

**Specs**:
- Parameters: 600 million (quantized to 8-bit)
- Size: ~600 MB
- Quantization: int8
- Language: 25 European languages (v3) or English (v2)
- GPU: ❌ Falls back to CPU (3.8x SLOWER)

**Best For**:
- CPU-only deployments
- Short audio (<3 seconds)
- Batch processing where latency isn't critical
- Multilingual support (v3)

**Usage**:
```rust
let recognizer = Recognizer::new(
    "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8",
    false  // CPU only
)?;
```

### 0.6B Float32 (Must Convert)

**Status**: ⏳ Not available - requires NeMo→ONNX conversion

**Expected Specs**:
- Parameters: 600 million
- Size: ~2.4 GB
- Quantization: float32
- Language: English (v2) or 25 languages (v3)
- GPU: ✅ Expected 5-10x speedup on long audio

**Best For**:
- Production GPU deployment (higher accuracy than 110M)
- Long audio transcription
- When accuracy is critical

**How to Get**:
```bash
# Requires: pip install nemo_toolkit[asr] onnx onnxruntime
python3 /opt/swictation/rust-crates/scripts/convert-parakeet-to-onnx.py 0.6b-v3
```

### 1.1B Float32 (Must Convert)

**Status**: ⏳ Not available - requires NeMo→ONNX conversion

**Expected Specs**:
- Parameters: 1.1 billion
- Size: ~4.4 GB
- Quantization: float32
- Language: English
- GPU: ✅ Expected 10-15x speedup on long audio

**Best For**:
- Maximum accuracy requirements
- Long audio transcription
- Research/benchmarking

**How to Get**:
```bash
# Requires: pip install nemo_toolkit[asr] onnx onnxruntime
python3 /opt/swictation/rust-crates/scripts/convert-parakeet-to-onnx.py 1.1b
```

## Implementation Strategies

### Strategy 1: Single Model (Simple)

**Use 110M float32 with GPU for everything**

```rust
pub fn create_recognizer() -> Result<Recognizer> {
    Recognizer::new(
        "/opt/swictation/models/sherpa-onnx-nemo-parakeet_tdt_transducer_110m-en-36000",
        true  // GPU
    )
}
```

**Pros**: Simple, fast for long audio
**Cons**: Slower for short audio, lower accuracy

### Strategy 2: Hybrid CPU/GPU (Recommended)

**Dynamic selection based on audio length**

```rust
pub fn create_recognizer(audio_duration_seconds: f64) -> Result<Recognizer> {
    let (model_path, use_gpu) = if audio_duration_seconds > 3.0 {
        // Long audio: Use GPU with float32
        (
            "/opt/swictation/models/sherpa-onnx-nemo-parakeet_tdt_transducer_110m-en-36000",
            true
        )
    } else {
        // Short audio: Use CPU with int8
        (
            "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8",
            false
        )
    };

    Recognizer::new(model_path, use_gpu)
}
```

**Pros**: Optimal performance for all audio lengths
**Cons**: More complex, requires audio length estimation

### Strategy 3: Quality vs Speed

**User-selectable quality levels**

```rust
pub enum Quality {
    Fast,      // 110M float32 GPU
    Balanced,  // 0.6B float32 GPU (after conversion)
    Best,      // 1.1B float32 GPU (after conversion)
}

pub fn create_recognizer(quality: Quality, use_gpu: bool) -> Result<Recognizer> {
    let model_path = match quality {
        Quality::Fast =>
            "/opt/swictation/models/sherpa-onnx-nemo-parakeet_tdt_transducer_110m-en-36000",
        Quality::Balanced =>
            "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3",
        Quality::Best =>
            "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-1.1b",
    };

    Recognizer::new(model_path, use_gpu)
}
```

**Pros**: User control, clear quality tiers
**Cons**: Requires converted models for Balanced/Best

## Conversion Instructions

### Prerequisites

```bash
pip install nemo_toolkit[asr] onnx onnxruntime
```

### Convert 0.6B v3 (Multilingual)

```bash
python3 /opt/swictation/rust-crates/scripts/convert-parakeet-to-onnx.py 0.6b-v3
```

Expected output: `/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3/`
- `encoder.onnx` (~2.2 GB)
- `decoder.onnx` (~15 MB)
- `joiner.onnx` (~3 MB)
- `tokens.txt` (~10 KB)

### Convert 1.1B (English)

```bash
python3 /opt/swictation/rust-crates/scripts/convert-parakeet-to-onnx.py 1.1b
```

Expected output: `/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-1.1b/`
- `encoder.onnx` (~4 GB)
- `decoder.onnx` (~15 MB)
- `joiner.onnx` (~3 MB)
- `tokens.txt` (~10 KB)

### Verify Conversion

```bash
cd /opt/swictation/rust-crates
cargo run --release --example test_gpu_benchmark
```

Expected results:
- **0.6B**: 5-8x GPU speedup on long audio
- **1.1B**: 8-12x GPU speedup on long audio

## Performance Benchmarks

### 110M Float32 (Tested ✅)

```
Audio: 30 seconds of speech

GPU (CUDA EP):
- Load: 1.52s
- Inference: 170ms (0.17s)
- Total: 1.69s
- RTF: 0.0056 (176x real-time)

CPU:
- Load: 1.48s
- Inference: 1336ms (1.34s)
- Total: 2.82s
- RTF: 0.044 (23x real-time)

Speedup: 7.86x
```

### 0.6B INT8 (Tested ✅)

```
Audio: 30 seconds of speech

GPU (CUDA EP - Falls back to CPU):
- Load: 2.81s
- Inference: 3582ms (3.58s)
- Total: 6.39s
- RTF: 0.119 (8.4x real-time)

CPU:
- Load: 2.25s
- Inference: 2703ms (2.70s)
- Total: 4.95s
- RTF: 0.090 (11x real-time)

Speedup: 0.77x (GPU SLOWER)
```

### 0.6B Float32 (Estimated)

```
Audio: 30 seconds of speech

GPU (CUDA EP - Estimated):
- Load: ~2.0s
- Inference: ~120ms (0.12s)
- Total: ~2.12s
- RTF: ~0.004 (250x real-time)

Estimated speedup: 5-8x vs CPU
```

## Resource Requirements

| Model | Disk Size | VRAM (GPU) | RAM (CPU) | GPU Utilization |
|-------|-----------|------------|-----------|-----------------|
| 110M float32 | 456 MB | ~1 GB | ~1 GB | 40-60% |
| 0.6B int8 | 600 MB | N/A | ~2 GB | 2-4% (fallback) |
| 0.6B float32 | ~2.4 GB | ~3 GB | ~4 GB | 50-70% |
| 1.1B float32 | ~4.4 GB | ~6 GB | ~8 GB | 60-80% |

## Recommendations Summary

### Immediate Deployment (Today)

**Use 110M float32 with GPU**:
- Available now from k2-fsa/sherpa-onnx
- 7.86x speedup on long audio
- Good balance of speed and accuracy
- Simple implementation

**Implementation**:
```rust
let recognizer = Recognizer::new(
    "/opt/swictation/models/sherpa-onnx-nemo-parakeet_tdt_transducer_110m-en-36000",
    audio_duration > 3.0  // GPU for long, CPU for short
)?;
```

### Production Deployment (Recommended)

**Convert 0.6B v3 to float32 for GPU**:
- Better accuracy than 110M
- Expected 5-8x speedup
- Multilingual support
- Worth the conversion effort

**Implementation**:
1. Run conversion script: `python3 scripts/convert-parakeet-to-onnx.py 0.6b-v3`
2. Test with: `cargo run --example test_gpu_benchmark`
3. Deploy with hybrid strategy (GPU for long, CPU for short)

### Future Optimization

**Convert 1.1B for maximum accuracy**:
- Best accuracy available
- Expected 10-15x speedup
- For critical applications
- Higher resource requirements

## Troubleshooting

### GPU shows low utilization (2-4%)

**Problem**: INT8 model being used with GPU
**Solution**: Use float32 model instead

### GPU slower than CPU

**Problem**: Audio too short (<3 seconds)
**Solution**: Use CPU for short audio

### Model produces garbage output

**Problem**: FP16 v2 model is broken
**Solution**: Use float32 or int8 instead

### cuDNN library not found

**Problem**: cuDNN not in library path
**Solution**:
```bash
sudo ldconfig /usr/local/cuda-12/lib /usr/local/cuda-12.9/lib64
```

## References

- GPU Success Report: `/opt/swictation/rust-crates/docs/gpu-acceleration-success-report.md`
- INT8 Testing Report: `/opt/swictation/rust-crates/docs/gpu-testing-report.md`
- Conversion Script: `/opt/swictation/rust-crates/scripts/convert-parakeet-to-onnx.py`
- NeMo Conversion Issue: https://github.com/NVIDIA/NeMo/issues/13085
- sherpa-onnx Models: https://k2-fsa.github.io/sherpa/onnx/pretrained_models/
