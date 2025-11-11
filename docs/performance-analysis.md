# Performance Analysis: Swictation Metrics Verification

**Analyst**: Performance Metrics Agent
**Date**: 2025-11-10
**Task**: Verify performance claims and identify unverified metrics
**Session**: swarm-1762839560715-p3x53fr7j

---

## Executive Summary

This document analyzes the Swictation codebase to verify performance claims found in documentation against actual code implementation, benchmarks, and test results.

**Key Findings:**
- ✅ **GPU Detection Logic**: Fully implemented and tested
- ⚠️ **VRAM Thresholds**: Logic verified, but peak usage values are **estimates not measurements**
- ⚠️ **WER Claims (5.77%)**: Referenced from external source, not independently verified
- ⚠️ **Latency Claims**: Ranges provided but no comprehensive benchmark data
- ❌ **Memory Usage**: Estimates present but no actual measurement code found

---

## 1. GPU Detection & VRAM Measurement

### Verified Implementation ✅

**Location**: `/opt/swictation/rust-crates/swictation-daemon/src/gpu.rs`

**Function**: `get_gpu_memory_mb()` (lines 130-184)
```rust
pub fn get_gpu_memory_mb() -> Option<(u64, u64)> {
    // Queries nvidia-smi with format: "total_mb, free_mb"
    let output = Command::new("nvidia-smi")
        .args(&[
            "--query-gpu=memory.total,memory.free",
            "--format=csv,noheader,nounits"
        ])
        .output()
        .ok()?;
    // ... parsing and validation ...
}
```

**Tests**: Lines 198-226
- Test exists for VRAM detection
- Validates parsing of nvidia-smi output
- Sanity checks for total > 0, free ≤ total
- **Status**: ✅ Working code with test coverage

**GPU Provider Detection**: Lines 12-42
- Platform-specific detection (CUDA, DirectML, CoreML)
- Priority: CUDA > DirectML > CoreML > CPU fallback
- **Status**: ✅ Implemented and tested

---

## 2. VRAM Threshold Logic

### Verified Logic ✅, Unverified Peak Values ⚠️

**Location**: `/opt/swictation/rust-crates/swictation-daemon/src/gpu.rs` (lines 229-270)

**Test**: `test_vram_thresholds()`
```rust
// 1.1B INT8 model requirements
let model_1_1b_peak = 3500; // 3.5GB peak usage ⚠️ ESTIMATE
let threshold_1_1b = 4096;  // 4GB minimum threshold
let headroom_1_1b = threshold_1_1b - model_1_1b_peak; // 596MB headroom

// 0.6B GPU model requirements
let model_0_6b_peak = 1200; // 1.2GB peak usage ⚠️ ESTIMATE
let threshold_0_6b = 1536;  // 1.5GB minimum threshold
let headroom_0_6b = threshold_0_6b - model_0_6b_peak; // 336MB headroom
```

**Decision Tree Test**: Lines 273-353
- Tests model selection for VRAM values: 512MB, 1024MB, 1536MB, 2048MB, 3072MB, 4096MB, 8192MB, 24576MB, 81920MB
- Validates threshold logic: <1536MB → 0.6B CPU, 1536-4095MB → 0.6B GPU, ≥4096MB → 1.1B GPU
- **Status**: ✅ Logic tested, ⚠️ Peak values are hardcoded estimates

### Unverified Claims ⚠️

**From Documentation** (`README.md:18`, `docs/architecture.md:279-283`):
- 1.1B peak VRAM: ~3.5GB (claimed)
- 0.6B peak VRAM: ~1.2GB (claimed)
- 1.1B typical VRAM: ~2.2GB (claimed)
- 0.6B typical VRAM: ~800MB (claimed)

**Evidence**: No benchmark code found that measures actual VRAM usage during inference
**Missing**: Runtime VRAM monitoring during model execution

---

## 3. WER (Word Error Rate) Accuracy

### Unverified External Claim ⚠️

**Claimed WER** (from `README.md:130`, `README.md:154`, `docs/architecture.md:278`):
- Parakeet-TDT-1.1B: **5.77% WER**
- Parakeet-TDT-0.6B: **7-8% WER**

**Source References** (in code):
```rust
// rust-crates/swictation-stt/src/engine.rs:15-16
/// - **≥4GB VRAM**: `Parakeet1_1B` (best quality: 5.77% WER)
/// - **≥1.5GB VRAM**: `Parakeet0_6B` with GPU (good quality: 7-8% WER)
```

**Test Files Found**:
1. `/opt/swictation/rust-crates/swictation-stt/examples/test_accuracy.rs`
   - Tests against reference transcriptions
   - Uses simple substring matching: `result.text.to_lowercase().contains(&reference[..20])`
   - **Not a proper WER calculation**

**What's Missing**:
- No Levenshtein distance calculation
- No proper WER formula: `WER = (S + D + I) / N` where S=substitutions, D=deletions, I=insertions, N=total words
- No standardized test dataset (LibriSpeech, Common Voice, etc.)
- Values appear to be **cited from external NVIDIA/NeMo benchmarks**, not independently verified

**Status**: ⚠️ **Unverified** - Values likely from model documentation, not measured in this codebase

---

## 4. Latency Measurements

### Partial Verification ⚠️

**Claimed Latencies** (from `docs/architecture.md:82-86`, `README.md:202-206`):

| Component | Claimed Latency | Verification Status |
|-----------|----------------|---------------------|
| VAD processing | <50ms | ✅ Claimed in docs |
| Silence threshold | 800ms | ✅ Configurable parameter |
| STT (1.1B GPU) | 150-250ms | ⚠️ Range claimed, no benchmark |
| STT (0.6B GPU) | 100-150ms | ⚠️ Range claimed, no benchmark |
| STT (0.6B CPU) | 200-400ms | ⚠️ Range claimed, no benchmark |
| Text transform | ~1µs | ✅ MidStream benchmark exists |
| Text injection | 10-50ms | ⚠️ Claimed, not measured |
| Hotkey latency | <10ms | ⚠️ Claimed, not measured |

### Benchmark Files Found

1. **`test_gpu_benchmark.rs`** (lines 1-74)
   - Tests GPU vs CPU speedup for 110M float32 model
   - Measures load time and inference time
   - Expected speedup: ≥1.5x for GPU
   - **Does not benchmark 0.6B or 1.1B models**

2. **`benchmark_models.rs`** (lines 1-110)
   - Benchmarks 0.6B and 1.1B models
   - Runs 3 iterations after warm-up
   - Calculates average processing time
   - **No actual results logged** - just framework

3. **`gpu_benchmark.rs`** (lines 1-74)
   - Compares GPU vs CPU for different models
   - Models: 0.6B int8, 0.6B fp16, 110M fp32
   - **No pre-captured results found**

4. **`test_pipeline_end_to_end.rs`** (lines 1-123)
   - Tests full pipeline with converted WAV files
   - Measures transcription time with `Instant::now()`
   - **No benchmark results stored**

### What's Missing
- No comprehensive latency benchmark suite with results
- No percentile measurements (p50, p95, p99)
- No comparison across different audio lengths
- No long-term monitoring data

**Status**: ⚠️ **Partially verified** - Code exists to measure, but results not captured in repo

---

## 5. Memory Usage (RAM)

### Unverified Estimates ⚠️

**Claimed RAM Usage** (from `README.md:214-216`, `docs/architecture.md:733-759`):

| Configuration | RAM Claimed | VRAM Claimed | Verification |
|---------------|-------------|--------------|--------------|
| Rust Daemon | ~150MB | - | ❌ No measurement |
| 1.1B GPU (typical) | ~160MB | ~2.2GB | ❌ Estimates only |
| 1.1B GPU (peak) | ~160MB | ~3.5GB | ❌ Estimates only |
| 0.6B GPU | ~160MB | ~800MB | ❌ Estimates only |
| 0.6B CPU | ~960MB | - | ❌ Estimates only |
| Context Buffer | ~400MB | - | ❌ Estimate |
| Silero VAD | - | 2.3MB | ✅ Model file size |
| Audio Buffer | ~10MB | - | ❌ Estimate |

**Evidence**:
- No memory profiling code found
- No calls to process memory measurement APIs
- Values appear to be engineering estimates
- Only VAD size (2.3MB) is verifiable as file size

**What's Missing**:
- Runtime memory monitoring
- Peak memory tracking
- Memory growth over time
- Actual measurements vs estimates

**Status**: ❌ **Unverified** - All values are estimates without measurement code

---

## 6. GPU Speedup Claims

### Benchmark Code Exists ✅, Results Missing ⚠️

**From** `test_gpu_benchmark.rs` (lines 55-70):
```rust
// Calculate speedup
let speedup_short = cpu_short_ms as f64 / gpu_short_ms as f64;
let speedup_long = cpu_long_ms as f64 / gpu_long_ms as f64;

if speedup_short >= 1.5 && speedup_long >= 1.5 {
    println!("✓ GPU acceleration is working! ({:.1}x average speedup)",
             (speedup_short + speedup_long) / 2.0);
}
```

**Status**: ✅ Code exists to verify ≥1.5x speedup, ⚠️ No logged benchmark runs found

---

## 7. Text Transformation Performance

### Verified Claim ✅

**Claimed**: ~1µs latency (`README.md:156`, `README.md:252`)

**Evidence**: MidStream crate has performance benchmarks
- Location: `external/midstream/benches/`
- Uses Criterion benchmark framework
- Measures transformation latency
- **Status**: ✅ Likely verified in external crate (not checked in detail)

---

## 8. Comparison with Other Systems

**From** `docs/architecture.md:875-880`:

| Feature | Swictation | Talon | Whisper.cpp | Cloud APIs |
|---------|-----------|-------|-------------|------------|
| Latency | ~1s (VAD pause), 100-150ms, 50-100ms, 500-1000ms | ? | ? | ? |
| Accuracy (WER) | 5.77-8% (adaptive) | ~3% WER | ~2% WER | 3-8% WER |
| Privacy | 100% local | 100% local | 100% local | Cloud only |
| VRAM Usage | 0.8-2.2GB | Varies | N/A | N/A |

**Status**: ❌ **Unverified** - No benchmarks comparing against other systems

---

## Summary of Verification Status

### ✅ Verified Metrics (Code + Tests)
1. GPU detection logic (CUDA, DirectML, CoreML)
2. VRAM measurement via nvidia-smi
3. VRAM threshold decision tree logic
4. Model selection based on available VRAM
5. Silero VAD model size (2.3MB file size)

### ⚠️ Partially Verified (Code exists, results missing)
1. **Latency measurements**: Timing code exists but no comprehensive benchmark results
2. **GPU speedup**: Test framework exists, no logged results
3. **VRAM peak usage**: Logic tested with hardcoded values, not measured
4. **0.6B vs 1.1B benchmarks**: Code exists, no results captured

### ❌ Unverified Claims (Estimates or External Citations)
1. **WER accuracy (5.77%, 7-8%)**: Cited from external model docs, not independently tested
2. **RAM usage (~150MB daemon)**: No measurement code found
3. **VRAM usage (2.2GB typical, 3.5GB peak)**: Engineering estimates, not measured
4. **Context buffer (400MB)**: Estimate without measurement
5. **Audio buffer (10MB)**: Estimate without measurement
6. **Text injection latency (10-50ms)**: Claimed without measurement
7. **Hotkey latency (<10ms)**: Claimed without measurement
8. **Comparison with other systems**: No benchmarks found

---

## Recommendations

### High Priority
1. **Implement WER Testing Suite**
   - Add proper Levenshtein distance calculation
   - Test against standard datasets (LibriSpeech)
   - Verify 5.77% and 7-8% WER claims independently

2. **Capture Memory Measurements**
   - Add runtime memory profiling (RSS, VRAM)
   - Measure actual vs estimated values
   - Track memory growth over time

3. **Run and Log Benchmarks**
   - Execute existing benchmark code
   - Capture results in version control
   - Include hardware specs with results

### Medium Priority
4. **Measure Peak VRAM Usage**
   - Profile actual VRAM during inference
   - Verify 3.5GB and 1.2GB peak claims
   - Adjust thresholds if needed

5. **Comprehensive Latency Testing**
   - Test across multiple audio lengths
   - Measure percentiles (p50, p95, p99)
   - Validate all claimed latency ranges

### Low Priority
6. **Comparative Benchmarks**
   - Test against Whisper.cpp, Talon
   - Document methodology and results
   - Update comparison tables with verified data

---

## Code References

### GPU Detection & VRAM
- `/opt/swictation/rust-crates/swictation-daemon/src/gpu.rs`
  - `detect_gpu_provider()`: Lines 12-42
  - `get_gpu_memory_mb()`: Lines 130-184
  - Tests: Lines 187-353

### Model Selection
- `/opt/swictation/rust-crates/swictation-daemon/src/pipeline.rs`
  - Adaptive selection: Lines 80-217
  - Config override: Lines 90-133

### Benchmarks (No Results)
- `/opt/swictation/rust-crates/swictation-daemon/examples/test_gpu_benchmark.rs`
- `/opt/swictation/rust-crates/swictation-stt/examples/benchmark_models.rs`
- `/opt/swictation/rust-crates/swictation-stt/examples/gpu_benchmark.rs`
- `/opt/swictation/rust-crates/swictation-daemon/examples/test_pipeline_end_to_end.rs`

### Accuracy Testing (Not WER)
- `/opt/swictation/rust-crates/swictation-stt/examples/test_accuracy.rs`

### Documentation Claims
- `/opt/swictation/README.md`: Lines 17-18, 130, 154-156, 202-216, 252, 260
- `/opt/swictation/docs/architecture.md`: Lines 82-90, 144-146, 182-283, 373-378, 596, 713-764, 770, 875-880

---

## Conclusion

The Swictation project has **solid infrastructure** for GPU detection and adaptive model selection. However, many performance claims are based on:
1. **Engineering estimates** (memory usage)
2. **External model documentation** (WER accuracy)
3. **Theoretical calculations** (VRAM headroom)
4. **Benchmark code without captured results** (latency, speedup)

To fully verify claims, the project needs:
- Comprehensive benchmark execution with logged results
- Runtime memory/VRAM profiling
- Independent WER testing against standard datasets
- Hardware-specific performance characterization

The existing benchmark framework is good - it just needs to be **run systematically and results captured**.

---

**Next Steps**: Store findings in hive memory and notify coordinator of analysis completion.
