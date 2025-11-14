# CUDA Version Constraints and Blackwell Support Strategy

## The CUDA Version Dilemma

### Problem Statement
We need to support GPUs from 2014 (Maxwell sm_50) through 2024 (Blackwell sm_120), but CUDA toolkit versions have conflicting requirements:

- **CUDA 12.6** (last version supporting sm_50):
  - ✅ Supports: sm_50 through sm_90
  - ❌ Missing: sm_100, sm_120 (Blackwell)

- **CUDA 13.0+** (adds Blackwell support):
  - ✅ Supports: sm_70 through sm_120
  - ❌ Drops: sm_50, sm_52 (Maxwell)

### Our Solution: Forward Compatibility via PTX

NVIDIA's PTX (Parallel Thread Execution) intermediate representation allows newer GPUs to run code compiled for older architectures through JIT compilation at runtime.

**How it works:**
1. Compile ONNX Runtime with `CMAKE_CUDA_ARCHITECTURES=89;90`
2. CUDA toolkit generates PTX code for sm_90
3. Blackwell GPU (sm_120) JIT-compiles PTX at first run
4. Compiled code is cached for future use

**Performance impact:**
- First run: ~2-5 second JIT compilation delay
- Subsequent runs: Near-native performance (~5-10% slower than native sm_120)
- Acceptable tradeoff for compatibility

## Package Architecture Strategy

### Package 1: Legacy (CUDA 12.6)
```
Architectures: sm_50,52,60,61,70
Target GPUs:
  - GTX 750/900 (Maxwell GM107/204)
  - GTX 1000 series (Pascal)
  - Quadro M2200, M4000, M6000
  - Titan V, V100 (Volta)
Build time: ~60 minutes
Binary size: ~150 MB
```

### Package 2: Modern (CUDA 12.6)
```
Architectures: sm_75,80,86
Target GPUs:
  - GTX 1600 series (Turing)
  - RTX 2000/3000 series
  - A100, A6000 (Ampere)
Build time: ~50 minutes
Binary size: ~180 MB
```

### Package 3: Latest (CUDA 12.6 with Forward Compatibility)
```
Architectures: sm_89,90
Target GPUs:
  - RTX 4090 (Ada Lovelace sm_89)
  - H100 (Hopper sm_90)
  - Blackwell via PTX forward-compat:
    * B100, B200 (sm_100)
    * RTX PRO 6000 Blackwell (sm_120)
    * RTX 50 series (sm_120)
Build time: ~45 minutes
Binary size: ~150 MB
```

## Verification Commands

### Check CUDA toolkit supported architectures:
```bash
nvcc --list-gpu-arch
```

### Check GPU compute capability:
```bash
nvidia-smi --query-gpu=compute_cap --format=csv,noheader
```

### Verify PTX code in compiled binary:
```bash
cuobjdump --list-ptx libonnxruntime_providers_cuda.so | grep "PTX file"
```

Expected output for latest package:
```
PTX file    1: compute_89
PTX file    2: compute_90
```

## Testing Plan

1. **Dad's Quadro M2200 (sm_50):**
   - Use legacy package
   - Test native sm_50 performance

2. **User's RTX A1000 Laptop (sm_86):**
   - Use modern package
   - Test native sm_86 performance

3. **User's RTX PRO 6000 Blackwell (sm_120):**
   - Use latest package
   - Test forward-compat PTX JIT
   - Measure first-run compilation delay
   - Verify subsequent runs use cached code

## Alternative: Future Native Blackwell Support

When CUDA 13.x becomes stable and ONNX Runtime supports it:

**Option A: Maintain legacy support**
- Package 1: CUDA 12.6, sm_50-70
- Package 2: CUDA 12.6, sm_75-90
- Package 3: CUDA 13.x, sm_89-120 (native Blackwell)
- Total: 3 packages, full compatibility

**Option B: Drop legacy support**
- Package 1: CUDA 13.x, sm_70-120
- Total: 1 package, simpler but loses Maxwell support
- Users with GTX 750-900 series must use CPU mode

Current recommendation: **Stay with CUDA 12.6 + PTX forward-compat** until CUDA 13.x is widely adopted and stable.

## References

- [NVIDIA CUDA Compatibility Guide](https://docs.nvidia.com/deploy/cuda-compatibility/)
- [CUDA Compute Capabilities](https://developer.nvidia.com/cuda-gpus)
- [PTX ISA Documentation](https://docs.nvidia.com/cuda/parallel-thread-execution/)
- [ONNX Runtime CUDA Build Guide](https://onnxruntime.ai/docs/build/eps.html#cuda)
