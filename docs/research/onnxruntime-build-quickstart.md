# ONNX Runtime CUDA Multi-Architecture Build - Quick Reference

## TL;DR - Essential Commands

### Prerequisites
```bash
# CUDA 12.6/12.9 (REQUIRED for sm_50 support)
# cuDNN 9.x
# CMake 3.28+
# Python 3.10+
```

### Build Command (Linux)
```bash
export CUDA_HOME=/usr/local/cuda-12.6
export CUDNN_HOME=/usr/lib/x86_64-linux-gnu/
export CUDACXX=$CUDA_HOME/bin/nvcc

git clone --recursive https://github.com/microsoft/onnxruntime.git
cd onnxruntime

./build.sh \
  --config Release \
  --build_shared_lib \
  --parallel \
  --use_cuda \
  --cuda_version 12.6 \
  --cuda_home $CUDA_HOME \
  --cudnn_home $CUDNN_HOME \
  --build_wheel \
  --skip_tests \
  --cmake_extra_defines CMAKE_CUDA_ARCHITECTURES="50;70;80;90" \
  --cmake_extra_defines CMAKE_CUDA_COMPILER=$CUDACXX
```

### Build Command (Windows)
```cmd
set CUDA_HOME=C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.6
set CUDNN_HOME=C:\Program Files\NVIDIA\CUDNN\v9.0

git clone --recursive https://github.com/microsoft/onnxruntime.git
cd onnxruntime

.\build.bat ^
  --config Release ^
  --build_shared_lib ^
  --parallel ^
  --use_cuda ^
  --cuda_version 12.6 ^
  --cuda_home "%CUDA_HOME%" ^
  --cudnn_home "%CUDNN_HOME%" ^
  --build_wheel ^
  --skip_tests ^
  --cmake_extra_defines CMAKE_CUDA_ARCHITECTURES="50;70;80;90" ^
  --cmake_generator "Visual Studio 17 2022"
```

---

## Quick Answers

### 1. How to build with specific CUDA architectures?

Use `--cmake_extra_defines CMAKE_CUDA_ARCHITECTURES="XX;YY;ZZ"`

**Examples:**
- Maxwell + Volta + Ampere + Hopper: `"50;70;80;90"`
- Modern GPUs only (Volta+): `"70;75;80;86;89;90"`
- Single architecture (A100): `"80"`

### 2. What are build requirements for CUDA 12.x?

**Required:**
- CUDA Toolkit 12.x (12.6 or 12.9 recommended for sm_50 support)
- cuDNN 9.x (NOT compatible with cuDNN 8.x)
- CMake 3.28+
- Python 3.10+
- Zlib (Linux only)

**Important:** CUDA 13.0+ will NOT support Maxwell (sm_50)!

### 3. Exact CMake flags for sm_50, sm_70, sm_80, sm_90?

```bash
--cmake_extra_defines CMAKE_CUDA_ARCHITECTURES="50;70;80;90"
```

**Optional additional flags:**
```bash
# Enable profiling
--cmake_extra_defines onnxruntime_ENABLE_NVTX_PROFILE=ON

# Enable debugging symbols for CUDA kernels
--cmake_extra_defines onnxruntime_ENABLE_CUDA_LINE_NUMBER_INFO=ON

# Enable NHWC optimizations
--cmake_extra_defines onnxruntime_USE_CUDA_NHWC_OPS=ON

# Skip unit tests (faster builds)
--cmake_extra_defines onnxruntime_BUILD_UNIT_TESTS=OFF
```

### 4. Binary sizes for multi-architecture builds?

| Configuration | Approximate Size |
|---------------|-----------------|
| Single arch (sm_80 only) | 240-300 MB (wheel) |
| Multi-arch (4 SMs: 50,70,80,90) | 500-800 MB (library) |
| With all dependencies (CUDA libs) | ~2.6 GB |

**Per-architecture overhead:** ~50-150 MB each

### 5. Performance implications?

**✅ NO runtime penalty:**
- CUDA runtime auto-selects optimal code for detected GPU
- Each GPU uses its native architecture code
- Startup time impact: ~10-50ms (one-time)

**✅ Full performance on each architecture:**
- sm_50 code optimized for Maxwell GPUs
- sm_80 code optimized for Ampere GPUs
- No compromise on per-GPU performance

**❌ DO NOT use old builds on new GPUs:**
- Running sm_70 code on sm_90 GPU: 2-10x slower
- Always include native SM code for target GPUs

### 6. Recommended CUDA toolkit version for sm_50?

**CUDA 12.6 or CUDA 12.9** (latest in 12.x series)

**Why:**
- Last major CUDA version supporting Maxwell (sm_50, sm_52)
- CUDA 13.0+ will DROP Maxwell/Pascal/Volta support
- Mature and stable
- Compatible with cuDNN 9.x

**Timeline:**
- sm_50 introduced: CUDA 6.0 (2014)
- sm_50 deprecated: CUDA 11.x (2020)
- sm_50 last supported: CUDA 12.x (2020-present)
- sm_50 removed: CUDA 13.0+ (future)

---

## Architecture Quick Reference

| SM Code | Architecture | Example GPUs | CUDA Support |
|---------|-------------|--------------|--------------|
| sm_50 | Maxwell | GTX 750, GTX 900 | 6.0 - 12.x ⚠️ |
| sm_52 | Maxwell | GTX Titan X | 6.5 - 12.x ⚠️ |
| sm_60 | Pascal | Tesla P100 | 8.0 - 12.x ⚠️ |
| sm_61 | Pascal | GTX 1080 | 8.0 - 12.x ⚠️ |
| sm_70 | Volta | Tesla V100 | 9.0 - 12.x ⚠️ |
| sm_75 | Turing | RTX 2080 | 10.0+ ✅ |
| sm_80 | Ampere | A100 | 11.0+ ✅ |
| sm_86 | Ampere | RTX 3090 | 11.1+ ✅ |
| sm_89 | Ada | RTX 4090 | 11.8+ ✅ |
| sm_90 | Hopper | H100 | 11.8+ ✅ |
| sm_120 | Blackwell | B100/B200 | 13.0+ ✅ |

⚠️ = Deprecated, will be removed in CUDA 13.0

---

## Build Time Estimates

| Configuration | 4-Core CPU | 8-Core CPU | 16-Core CPU |
|---------------|-----------|-----------|-------------|
| Single arch (sm_80) | 15-25 min | 10-15 min | 8-12 min |
| 4 archs (50,70,80,90) | 45-70 min | 25-40 min | 15-25 min |
| 7 archs (all modern) | 70-120 min | 40-60 min | 25-40 min |

**Optimization tips:**
- Use `--parallel` for multi-core builds
- Use `--skip_tests` to save 30-50% time
- Use SSD instead of HDD
- Ensure 16GB+ RAM available

---

## Common Issues & Solutions

### Build can't find CUDA
```bash
# Set explicit paths
export CUDA_HOME=/usr/local/cuda-12.6
export PATH=$CUDA_HOME/bin:$PATH
--cuda_home $CUDA_HOME
```

### CMAKE_CUDA_ARCHITECTURES ignored
```bash
# Explicitly set CUDA compiler
export CUDACXX=/usr/local/cuda-12.6/bin/nvcc
--cmake_extra_defines CMAKE_CUDA_COMPILER=$CUDACXX
```

### cuDNN version mismatch
```bash
# Verify cuDNN 9.x installation
ls -la /usr/lib/x86_64-linux-gnu/libcudnn.so*
# Should show version 9.x.x
```

### Out of memory during build
```bash
# Reduce parallel jobs
./build.sh --parallel 4  # instead of auto-detect
```

### Many warnings about old SM versions
```bash
# Normal for sm_50/sm_52 on CUDA 12.x - build will succeed
# Optional: suppress with --cmake_extra_defines CMAKE_CUDA_FLAGS="-w"
```

---

## Verification Commands

### Check CUDA installation
```bash
nvcc --version
# Should show: cuda_12.6 or cuda_12.9
```

### Check build output
```bash
ls -lh build/Linux/Release/libonnxruntime.so*
# Expected: 500-800 MB for multi-arch build
```

### Verify CUDA provider in Python
```python
import onnxruntime as ort
print(ort.get_available_providers())
# Should include: 'CUDAExecutionProvider'
```

### Check GPU compute capability
```bash
nvidia-smi --query-gpu=compute_cap --format=csv,noheader
# Shows your GPU's SM version (e.g., 8.0 for A100)
```

---

## Production Build Script

```bash
#!/bin/bash
# Production-ready ONNX Runtime build script

set -e

# Configuration
CUDA_VERSION=12.6
TARGET_ARCHS="50;70;75;80;86;89;90"

# Setup environment
export CUDA_HOME=/usr/local/cuda-${CUDA_VERSION}
export CUDNN_HOME=/usr/lib/x86_64-linux-gnu/
export CUDACXX=${CUDA_HOME}/bin/nvcc
export PATH=${CUDA_HOME}/bin:$PATH

# Clone
git clone --recursive --branch v1.24.0 \
    https://github.com/microsoft/onnxruntime.git
cd onnxruntime

# Build
./build.sh \
  --config Release \
  --build_shared_lib \
  --parallel \
  --use_cuda \
  --cuda_version ${CUDA_VERSION} \
  --cuda_home ${CUDA_HOME} \
  --cudnn_home ${CUDNN_HOME} \
  --build_wheel \
  --skip_tests \
  --cmake_extra_defines CMAKE_CUDA_ARCHITECTURES="${TARGET_ARCHS}" \
  --cmake_extra_defines CMAKE_CUDA_COMPILER=${CUDACXX} \
  --cmake_extra_defines onnxruntime_BUILD_UNIT_TESTS=OFF

echo "✅ Build complete!"
echo "📦 Wheel location:"
find build -name "*.whl"
```

---

## Docker Build (Recommended)

```dockerfile
FROM nvidia/cuda:12.6.0-cudnn9-devel-ubuntu22.04

RUN apt-get update && apt-get install -y \
    build-essential cmake git python3.10 python3-pip zlib1g-dev

RUN git clone --recursive https://github.com/microsoft/onnxruntime.git
WORKDIR /onnxruntime

RUN ./build.sh \
    --config Release \
    --build_shared_lib \
    --parallel \
    --use_cuda \
    --cuda_version 12.6 \
    --build_wheel \
    --cmake_extra_defines CMAKE_CUDA_ARCHITECTURES="50;70;80;90"
```

Build with:
```bash
docker build -t onnxruntime-cuda:12.6 .
docker run --gpus all -it onnxruntime-cuda:12.6 bash
```

---

## References

- Full guide: `/opt/swictation/docs/research/onnxruntime-cuda-build-guide.md`
- Official docs: https://onnxruntime.ai/docs/build/eps.html
- CUDA docs: https://docs.nvidia.com/cuda/
- ONNX Runtime repo: https://github.com/microsoft/onnxruntime

---

**Last Updated:** 2025-11-13
