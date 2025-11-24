# macOS ONNX Runtime Metal Provider Research

**Research Date:** 2025-11-23
**Purpose:** Document ONNX Runtime GPU acceleration on Apple Silicon for Swictation

## Executive Summary

**CRITICAL FINDING:** ONNX Runtime does **not** have a direct Metal Performance Shaders (MPS) execution provider. Instead, Apple Silicon GPU acceleration is achieved through the **CoreML Execution Provider**, which internally may use Metal/MPS for GPU compute.

**Key Decision:** Use CoreML Execution Provider (not "Metal" provider) for GPU acceleration on Apple Silicon.

---

## Table of Contents

1. [ONNX Runtime on Apple Silicon](#onnx-runtime-on-apple-silicon)
2. [CoreML Execution Provider](#coreml-execution-provider)
3. [INT8 Quantization Support](#int8-quantization-support)
4. [Unified Memory Architecture](#unified-memory-architecture)
5. [Performance Expectations](#performance-expectations)
6. [Implementation Guide](#implementation-guide)

---

## ONNX Runtime on Apple Silicon

### Available Execution Providers

ONNX Runtime supports the following execution providers on macOS:

1. **CPU Execution Provider** - Default, always available
2. **CoreML Execution Provider** - GPU/ANE acceleration (recommended)
3. ~~Metal Execution Provider~~ - **Does not exist** as standalone provider

### CoreML vs "Metal"

**CoreML Execution Provider:**
- ✅ Official ONNX Runtime provider for Apple hardware
- ✅ Can dispatch to CPU, GPU (Metal), or Neural Engine
- ✅ Supported since ONNX Runtime 1.7+
- ✅ Works on macOS 10.15+ (requires 14+ for best support)
- ⚠️ Limited operator coverage (not all ONNX ops supported)
- ⚠️ Static shapes preferred for best performance

**Direct Metal Provider:**
- ❌ Does not exist in ONNX Runtime
- ❌ Not available as execution provider option
- 🔄 All Metal acceleration goes through CoreML

### Why CoreML Instead of Metal?

Apple's CoreML framework provides:
- **Automatic hardware selection** (CPU/GPU/ANE based on model and device)
- **Model compilation and optimization** for Apple Silicon
- **Unified API** across iOS, macOS, watchOS, tvOS
- **Better operator coverage** than raw Metal compute kernels

---

## CoreML Execution Provider

### Configuration

```rust
use ort::{
    execution_providers::CoreMLExecutionProvider,
    Session,
};

// Create session with CoreML provider
let session = Session::builder()?
    .with_execution_providers([
        CoreMLExecutionProvider::default()
            .with_ane_only(false)              // Allow GPU + ANE
            .with_subgraphs(true)              // Enable in control flow
            .build(),
    ])?
    .commit_from_file("parakeet_tdt_1.1b.onnx")?;
```

### Supported Operators

CoreML EP supports most common ONNX operators, but **not all**:

**Supported (relevant to Parakeet-TDT):**
- ✅ Conv, ConvTranspose
- ✅ MatMul, Gemm
- ✅ Add, Sub, Mul, Div
- ✅ Relu, Sigmoid, Tanh, Gelu
- ✅ Softmax, LayerNormalization
- ✅ Reshape, Transpose, Concat
- ✅ AveragePool, MaxPool
- ✅ BatchNormalization
- ✅ Attention (via decomposition)

**Unsupported (will fallback to CPU):**
- ⚠️ Some dynamic shape operations
- ⚠️ Complex control flow (Loop, If, Scan)
- ⚠️ Custom/contrib operators

### Performance Tuning

**For best CoreML performance:**

1. **Static Input Shapes** - CoreML compiles models; dynamic shapes force recompilation
2. **MLProgram Format** - Use `COREML_FLAG_CREATE_MLPROGRAM` for macOS 12+ (better than NeuralNetwork)
3. **Profiling** - Enable `ProfileComputePlan` to see which ops run on GPU vs CPU

```rust
CoreMLExecutionProvider::default()
    .with_mlprogram(true)                 // Use MLProgram format (macOS 12+)
    .with_static_input_shapes(true)       // Optimize for static shapes
    .with_compute_units(ComputeUnits::All) // CPU + GPU + ANE
    .build()
```

---

## INT8 Quantization Support

### CRITICAL LIMITATION

**CoreML EP has LIMITED support for INT8 quantized ONNX models:**

1. **Quantized operators often NOT accelerated:**
   - INT8 Conv, MatMul may run on CPU
   - Dequantize operations before execution
   - Negates most performance benefit

2. **Recommended approach:**
   - Use **FP16 models** instead of INT8 for Apple Silicon
   - CoreML natively supports FP16 on GPU
   - Better acceleration than quantized INT8

3. **Alternative:**
   - Convert ONNX → CoreML format
   - Use CoreML's own quantization tools
   - But: Conversion pipeline more complex

### Performance Comparison

| Model Type | CoreML EP | Recommendation |
|-----------|-----------|----------------|
| FP32      | Good      | Use for CPU   |
| FP16      | **Excellent** | **✅ Best for GPU** |
| INT8      | Poor      | ❌ Avoid on macOS |

**For Swictation:**
- Keep FP16 version of Parakeet-TDT for macOS
- INT8 models for Linux CUDA only
- Accept ~2x model size vs 4x better GPU performance

---

## Unified Memory Architecture

### How Apple Silicon Memory Works

Apple Silicon (M1/M2/M3/M4) uses **unified memory architecture (UMA):**

```
┌─────────────────────────────────────┐
│     Physical RAM (shared)           │
│  ┌──────────┬──────────┬─────────┐  │
│  │   CPU    │   GPU    │   ANE   │  │
│  │  Access  │  Access  │  Access │  │
│  └──────────┴──────────┴─────────┘  │
│         No memory copying!          │
└─────────────────────────────────────┘
```

**Key Differences from NVIDIA CUDA:**

| Feature | NVIDIA (Discrete) | Apple Silicon (Unified) |
|---------|------------------|------------------------|
| Memory | Separate VRAM | Shared system RAM |
| CPU↔GPU Copy | Required | **Not needed** |
| Memory Bandwidth | GPU: ~1TB/s, CPU: ~50GB/s | Unified: ~200-400GB/s |
| VRAM Detection | Query GPU | Query **system memory** |

### GPU Memory Detection on macOS

**Wrong approach (Linux-style):**
```rust
// This doesn't exist on macOS!
get_nvidia_gpu_vram() // ❌
```

**Correct approach (unified memory):**
```rust
/// Get available memory for ML workloads on macOS
///
/// Apple Silicon uses unified memory - GPU shares system RAM
pub fn get_gpu_memory_mb() -> Option<(u64, u64)> {
    // Query system memory (not separate VRAM)
    let total_mb = sysinfo::System::total_memory() / (1024 * 1024);

    // Reserve memory for OS and other apps (30-40%)
    let available_mb = (total_mb as f64 * 0.65) as u64;

    Some((total_mb, available_mb))
}
```

### Memory Usage Strategy

**For model selection:**
- M1 (8GB): Use 0.6B model (70% of 8GB = ~5GB available)
- M1 Pro/Max (16-32GB): Use 1.1B model
- M1 Ultra (64-128GB): Use 1.1B model (overkill, but works)

**Memory calculation:**
```rust
// Estimate model memory usage
let model_size_mb = if is_int8 { 600 } else { 1200 }; // FP16
let required_mb = model_size_mb + 500; // Model + activation memory

let (total, available) = get_gpu_memory_mb()?;
if required_mb > available {
    warn!("Insufficient memory for model");
}
```

---

## Performance Expectations

### Benchmark Comparison

Based on research findings from PyTorch/spaCy benchmarks:

**Transformer inference speed (words per second):**

| Device | CPU AMX | GPU Metal | vs CPU |
|--------|---------|-----------|--------|
| M1 (8 GPU cores) | 1,180 | 2,202 | **1.9x** |
| M2 (10 GPU cores) | 1,242 | 3,362 | **2.7x** |
| M1 Pro (14 GPU) | 1,631 | 4,661 | **2.9x** |
| M1 Max (32 GPU) | 1,821 | 8,648 | **4.7x** |
| M1 Ultra (48 GPU) | 2,197 | 12,073 | **5.5x** |

**For comparison:**
- NVIDIA RTX 3090: 18,845 WPS (**10x** vs CPU)
- M1 Max GPU: ~50% of RTX 3090 performance

### Expected Swictation Performance

**Parakeet-TDT 1.1B model (FP16):**

| Hardware | Expected RTF | Latency (1s audio) |
|----------|--------------|-------------------|
| M1 CPU | ~0.15 | 150ms |
| M1 GPU (CoreML) | ~0.05-0.08 | **50-80ms** |
| M1 Max GPU | ~0.03-0.05 | **30-50ms** |
| M2/M3 GPU | ~0.04-0.06 | **40-60ms** |

**Conclusion:** CoreML GPU acceleration should provide **2-3x speedup** over CPU on M1/M2/M3 Macs.

---

## Implementation Guide

### Step 1: Update Cargo.toml

```toml
[dependencies]
ort = { version = "2.0.0-rc.10", features = ["coreml"] }

[target.'cfg(target_os = "macos")'.dependencies]
# No additional dependencies needed - CoreML is in ort
```

### Step 2: Update STT Engine

**File:** `rust-crates/swictation-stt/src/recognizer_ort.rs`

```rust
#[cfg(target_os = "macos")]
fn create_session(model_path: &Path) -> Result<Session> {
    use ort::execution_providers::CoreMLExecutionProvider;

    Session::builder()?
        .with_execution_providers([
            // Try CoreML first (GPU/ANE acceleration)
            CoreMLExecutionProvider::default()
                .with_mlprogram(true)
                .with_compute_units(ComputeUnits::All)
                .build(),
            // Fallback to CPU if CoreML unavailable
            ort::execution_providers::CPUExecutionProvider::default(),
        ])?
        .commit_from_file(model_path)
}

#[cfg(target_os = "linux")]
fn create_session(model_path: &Path) -> Result<Session> {
    // Existing Linux CUDA logic
}
```

### Step 3: Platform-Specific Model Selection

```rust
#[cfg(target_os = "macos")]
fn select_model_for_hardware() -> &'static str {
    let (total_mb, _) = get_gpu_memory_mb().unwrap_or((8192, 5000));

    if total_mb >= 16_000 {
        "parakeet_tdt_1.1b_fp16.onnx"  // FP16, not INT8!
    } else {
        "parakeet_tdt_0.6b_fp16.onnx"
    }
}
```

### Step 4: GPU Detection

**File:** `rust-crates/swictation-daemon/src/gpu_macos.rs`

```rust
use sysinfo::{System, SystemExt};

/// Get GPU memory info for macOS (unified memory)
pub fn get_gpu_memory_mb() -> Option<(u64, u64)> {
    let mut system = System::new_all();
    system.refresh_memory();

    let total_mb = system.total_memory() / (1024 * 1024);

    // Reserve 30-40% for OS and other apps
    let available_mb = (total_mb as f64 * 0.65) as u64;

    Some((total_mb, available_mb))
}

/// Check if running on Apple Silicon
pub fn is_apple_silicon() -> bool {
    #[cfg(target_arch = "aarch64")]
    {
        true
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        false
    }
}
```

---

## Testing Checklist

- [ ] CoreML EP initializes successfully on M1/M2/M3
- [ ] Model loads with FP16 weights (not INT8)
- [ ] Inference uses GPU (verify with Activity Monitor → GPU History)
- [ ] Latency meets targets (<50ms for 1s audio on M1 Max)
- [ ] Unified memory usage correct (no separate VRAM reported)
- [ ] Fallback to CPU works if CoreML unavailable
- [ ] Performance comparable to expectations (2-3x vs CPU)

---

## References

1. **ONNX Runtime CoreML EP Documentation:**
   https://onnxruntime.ai/docs/execution-providers/CoreML-ExecutionProvider.html

2. **Metal Performance Shaders with PyTorch:**
   https://explosion.ai/blog/metal-performance-shaders

3. **ort (Rust ONNX Runtime bindings):**
   https://ort.pyke.io

4. **Apple Silicon GPU Specs:**
   - M1: 8 GPU cores, 2.6 TFLOPS
   - M2: 10 GPU cores, 3.6 TFLOPS
   - M1 Pro: 14-16 GPU cores
   - M1 Max: 24-32 GPU cores
   - M1 Ultra: 48-64 GPU cores

5. **CoreML Operator Support:**
   Limited coverage; check docs for specific op support

---

## Task Updates Required

Based on these findings, the following Archon tasks need updates:

### Task 79ef114c (This research task)
- ✅ Mark as complete
- Document: Use CoreML EP, not "Metal" provider
- Document: FP16 models preferred over INT8
- Document: Unified memory architecture

### Task b5847c7a (Integrate ONNX Runtime with Metal)
- ❌ Title misleading - should be "Integrate ONNX Runtime with CoreML"
- Update implementation to use CoreML EP
- Remove references to "Metal provider"
- Add FP16 model preference

### Task f1fe2ed4 (Create macOS GPU detection)
- Update to query **system memory**, not separate VRAM
- Document unified memory architecture
- Add 65% available memory calculation

### Task 34416d1a (Download ONNX Runtime dylibs)
- Specify: Need CoreML-enabled build
- Verify: ort crate with "coreml" feature sufficient
- No separate Metal dylibs needed
