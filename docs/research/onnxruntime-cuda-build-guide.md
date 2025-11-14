# ONNX Runtime CUDA Multi-Architecture Build Guide

## Executive Summary

This guide provides comprehensive instructions for building ONNX Runtime from source with custom CUDA compute capability support, including multi-architecture builds supporting sm_50, sm_70, sm_80, and sm_90.

**Key Findings:**
- CUDA 12.x is the last major version supporting Maxwell (sm_50)
- Multi-architecture builds increase binary size by ~2-3x
- Build time varies significantly based on target architectures
- CUDA 12.x with cuDNN 9 is the current recommended configuration
- Maxwell support (sm_50) requires CUDA Toolkit 12.x or earlier

---

## 1. CUDA Compute Capability Support Matrix

### Target Architectures

| Architecture | Compute Capability | CUDA Support Status | GPU Examples |
|--------------|-------------------|---------------------|--------------|
| Maxwell | sm_50, sm_52 | Deprecated in CUDA 11+, **last supported in CUDA 12.x** | GTX 750, GTX 900 series |
| Pascal | sm_60, sm_61 | Deprecated in CUDA 11+, **last supported in CUDA 12.x** | GTX 1000 series, Tesla P100 |
| Volta | sm_70, sm_72 | Deprecated in CUDA 11+, **last supported in CUDA 12.x** | Tesla V100, Titan V |
| Turing | sm_75 | Fully supported | RTX 2000 series, GTX 1600 series |
| Ampere | sm_80, sm_86 | Fully supported | A100, RTX 3000 series |
| Hopper | sm_90, sm_90a | Fully supported | H100 |
| Blackwell | sm_120 | Supported in CUDA 13+ | B100, B200 |

### Critical Notes on sm_50 Support

**CUDA Version Requirements:**
- **Initial Support:** CUDA 6.0 (2014)
- **Deprecated:** CUDA 11.x (2020)
- **Last Major Support:** CUDA 12.x series (12.0 - 12.9)
- **Support Removed:** CUDA 13.0+ (cannot target Maxwell/Pascal/Volta)

**Recommendation:** For sm_50 support, use **CUDA 12.6** or **CUDA 12.9** (latest in 12.x series).

---

## 2. Build Requirements and Dependencies

### System Requirements

**Minimum Versions:**
- CMake: 3.28+ (increased from 3.26)
- Python: 3.10+ (for building from source)
- CUDA Toolkit: 12.x (for sm_50 support)
- cuDNN: 9.x (for CUDA 12.x)

### CUDA 12.x Build Dependencies

#### Core Dependencies
```bash
# CUDA Toolkit 12.x
CUDA_HOME=/usr/local/cuda-12.6  # or 12.9
CUDA_PATH=/usr/local/cuda-12.6

# cuDNN 9.x
cuDNN_PATH=/usr/lib/x86_64-linux-gnu/
# OR for custom installation:
cuDNN_PATH=/usr/local/cudnn-9.x

# Zlib (Linux only, required by cuDNN 9.x)
# Usually available via: apt-get install zlib1g-dev
```

#### Environment Variables
```bash
# Set before building
export CUDA_HOME=/usr/local/cuda-12.6
export CUDA_PATH=/usr/local/cuda-12.6
export CUDNN_HOME=/usr/local/cudnn-9.x
export CUDACXX=/usr/local/cuda-12.6/bin/nvcc
export PATH=/usr/local/cuda-12.6/bin:$PATH
export LD_LIBRARY_PATH=/usr/local/cuda-12.6/lib64:$LD_LIBRARY_PATH
```

### Version Compatibility Matrix

| ONNX Runtime | CUDA Version | cuDNN Version | Python | Notes |
|--------------|--------------|---------------|--------|-------|
| 1.19+ | 12.x | 9.x | 3.10+ | Default for PyPI packages |
| 1.18 | 11.8 | 8.x | 3.8+ | Legacy CUDA 11 support |
| 1.24+ | 12.x / 13.x | 9.x | 3.10+ | Blackwell support (sm_120) |

**Important:** ONNX Runtime built with cuDNN 8.x is NOT compatible with cuDNN 9.x, and vice versa.

---

## 3. CMake Configuration for Multi-Architecture Builds

### CMAKE_CUDA_ARCHITECTURES Syntax

ONNX Runtime uses CMake 3.18+ `CMAKE_CUDA_ARCHITECTURES` property to specify target GPU architectures.

#### Syntax Options

**Option 1: Semicolon-separated list (Recommended)**
```bash
--cmake_extra_defines CMAKE_CUDA_ARCHITECTURES="50;70;80;90"
```

**Option 2: Space-separated with quotes**
```bash
--cmake_extra_defines CMAKE_CUDA_ARCHITECTURES="50 70 80 90"
```

**Option 3: Single architecture**
```bash
--cmake_extra_defines CMAKE_CUDA_ARCHITECTURES=80
```

**Option 4: Native architecture (auto-detect)**
```bash
--cmake_extra_defines CMAKE_CUDA_ARCHITECTURES=native
```

### Complete Build Flag Examples

#### Full Multi-Architecture Build (sm_50, sm_70, sm_80, sm_90)

**Linux:**
```bash
./build.sh \
  --config Release \
  --build_shared_lib \
  --parallel \
  --use_cuda \
  --cuda_version 12.6 \
  --cuda_home /usr/local/cuda-12.6 \
  --cudnn_home /usr/lib/x86_64-linux-gnu/ \
  --build_wheel \
  --skip_tests \
  --cmake_extra_defines CMAKE_CUDA_ARCHITECTURES="50;70;80;90" \
  --cmake_extra_defines CMAKE_CUDA_COMPILER=/usr/local/cuda-12.6/bin/nvcc \
  --cmake_extra_defines onnxruntime_BUILD_UNIT_TESTS=OFF
```

**Windows:**
```cmd
.\build.bat ^
  --config Release ^
  --build_shared_lib ^
  --parallel ^
  --use_cuda ^
  --cuda_version 12.6 ^
  --cuda_home "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.6" ^
  --cudnn_home "C:\Program Files\NVIDIA\CUDNN\v9.0" ^
  --build_wheel ^
  --skip_tests ^
  --cmake_extra_defines CMAKE_CUDA_ARCHITECTURES="50;70;80;90" ^
  --cmake_generator "Visual Studio 17 2022"
```

#### Optimized Build (Modern GPUs Only: sm_70+)

If Maxwell (sm_50) support is not required:
```bash
./build.sh \
  --config Release \
  --build_shared_lib \
  --parallel \
  --use_cuda \
  --cuda_version 12.6 \
  --cuda_home /usr/local/cuda-12.6 \
  --cudnn_home /usr/lib/x86_64-linux-gnu/ \
  --build_wheel \
  --skip_tests \
  --cmake_extra_defines CMAKE_CUDA_ARCHITECTURES="70;80;90" \
  --cmake_extra_defines onnxruntime_BUILD_UNIT_TESTS=OFF
```

#### Additional Useful CMake Flags

```bash
# Enable NVTX profiling support (Nsight Systems)
--cmake_extra_defines onnxruntime_ENABLE_NVTX_PROFILE=ON

# Enable CUDA line number information (debugging)
--cmake_extra_defines onnxruntime_ENABLE_CUDA_LINE_NUMBER_INFO=ON

# Enable NHWC (channels-last) optimizations
--cmake_extra_defines onnxruntime_USE_CUDA_NHWC_OPS=ON

# Custom install prefix
--cmake_extra_defines CMAKE_INSTALL_PREFIX=/usr/local/onnxruntime

# Build with TensorRT support
--use_tensorrt --tensorrt_home /usr/src/tensorrt
```

---

## 4. Binary Size Analysis

### Expected Binary Sizes

#### Core ONNX Runtime Library

| Configuration | Size (Approximate) | Notes |
|---------------|-------------------|-------|
| Minimal build (CPU only) | 1.5 - 7.5 MB | No CUDA dependencies |
| Single architecture (sm_80) | 240 - 300 MB | Wheel package size |
| Multi-arch (sm_50,70,80,90) | 500 - 800 MB | libonnxruntime.so |
| Full CUDA dependencies | 2.6 GB | Includes cuBLAS, cuDNN, TensorRT |

#### Size Impact by Component

**Per-architecture overhead:** ~50-150 MB additional per compute capability
- Each SM version requires separate cubin code generation
- CUDA libraries (cuBLAS, cuDNN) contain multi-arch code

**Python Wheel Sizes (onnxruntime-gpu from PyPI):**
- Linux (manylinux): ~300 MB
- Windows: ~245 MB
- Does NOT include CUDA runtime (must install separately)

#### Binary Size Optimization Strategies

1. **Minimal Architecture Set:**
   - Only include architectures you actually need
   - Example: If only targeting RTX 3000+ series, use `sm_86;80;90`

2. **Separate CUDA Dependencies:**
   - Don't bundle CUDA libraries with the binary
   - Require users to install CUDA Toolkit separately
   - Reduces distribution size significantly

3. **Build Configurations:**
   ```bash
   # Minimal release build (smallest binary)
   --config MinSizeRel

   # Standard release build
   --config Release

   # Release with debug info (largest)
   --config RelWithDebInfo
   ```

---

## 5. Performance Implications

### Multi-Architecture Binary Performance

**Runtime Performance:**
- **NO runtime penalty** - CUDA runtime selects appropriate cubin for detected GPU
- Optimal performance on each supported architecture
- Binary contains multiple cubin variants, GPU picks its native version

**Startup Performance:**
- Minimal impact (~10-50ms) for library loading
- CUDA runtime performs one-time architecture detection
- No ongoing overhead after initialization

**Memory Impact:**
- Larger binaries consume more disk space and memory when loaded
- CUDA runtime only loads code for detected architecture
- Inactive architecture code remains in disk cache

### Forward/Backward Compatibility

**Forward Compatibility (Running old binaries on new GPUs):**
- ❌ **NOT recommended** - Extremely slow performance
- Virtual architecture codes (compute_XX) provide basic compatibility
- Example: sm_70 code running on sm_90 GPU is 2-10x slower

**Backward Compatibility (New binaries on old GPUs):**
- ✅ **Fully supported** if architecture included in build
- Example: Binary with sm_50;80;90 runs optimally on GTX 750 (sm_50)

**Best Practice:** Always include specific SM code for target GPUs

### Architecture-Specific Optimizations

```bash
# Comprehensive coverage (Maxwell → Hopper)
CMAKE_CUDA_ARCHITECTURES="50;70;75;80;86;89;90"

# Server GPUs only (Volta → Hopper)
CMAKE_CUDA_ARCHITECTURES="70;80;90"

# Consumer GPUs (Pascal → Ada)
CMAKE_CUDA_ARCHITECTURES="61;75;86;89"
```

---

## 6. Recommended CUDA Toolkit Versions

### For sm_50 (Maxwell) Support

**Recommended: CUDA 12.6 or CUDA 12.9**

Rationale:
- Last major CUDA version supporting Maxwell (sm_50, sm_52)
- Mature, stable release with extensive testing
- Compatible with modern cuDNN 9.x
- CUDA 13.0+ will NOT support Maxwell/Pascal/Volta

### Installation Instructions

#### CUDA 12.6 Installation (Ubuntu/Debian)

```bash
# Download and install CUDA 12.6
wget https://developer.download.nvidia.com/compute/cuda/12.6.0/local_installers/cuda_12.6.0_560.28.03_linux.run
sudo sh cuda_12.6.0_560.28.03_linux.run

# Verify installation
nvcc --version
# Expected: Cuda compilation tools, release 12.6
```

#### cuDNN 9.x Installation

```bash
# Download cuDNN 9.x for CUDA 12.x from NVIDIA Developer Portal
# Requires NVIDIA Developer account

# Extract and copy to CUDA directory
tar -xvf cudnn-linux-x86_64-9.x.x.x_cuda12-archive.tar.xz
sudo cp cudnn-*-archive/include/cudnn*.h /usr/local/cuda-12.6/include
sudo cp -P cudnn-*-archive/lib/libcudnn* /usr/local/cuda-12.6/lib64
sudo chmod a+r /usr/local/cuda-12.6/include/cudnn*.h /usr/local/cuda-12.6/lib64/libcudnn*
```

#### Windows Installation

```powershell
# Download CUDA 12.6 installer from NVIDIA
# https://developer.nvidia.com/cuda-12-6-0-download-archive

# Install to default location:
# C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.6

# Download and extract cuDNN 9.x
# Copy files to CUDA installation directory
```

### Alternative: Docker-based Build

```dockerfile
# Dockerfile for ONNX Runtime CUDA 12.6 build
FROM nvidia/cuda:12.6.0-cudnn9-devel-ubuntu22.04

# Install dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    git \
    python3.10 \
    python3.10-dev \
    python3-pip \
    zlib1g-dev

# Clone ONNX Runtime
RUN git clone --recursive https://github.com/microsoft/onnxruntime.git
WORKDIR /onnxruntime

# Build with multi-architecture support
RUN ./build.sh \
    --config Release \
    --build_shared_lib \
    --parallel \
    --use_cuda \
    --cuda_version 12.6 \
    --cuda_home /usr/local/cuda \
    --cudnn_home /usr/lib/x86_64-linux-gnu/ \
    --build_wheel \
    --cmake_extra_defines CMAKE_CUDA_ARCHITECTURES="50;70;80;90"
```

---

## 7. Complete Build Instructions

### Prerequisites Checklist

- [ ] CUDA Toolkit 12.6 or 12.9 installed
- [ ] cuDNN 9.x installed and configured
- [ ] CMake 3.28+ installed
- [ ] Python 3.10+ installed
- [ ] Git with submodule support
- [ ] C++ compiler (GCC 9+, MSVC 2019+, or Clang 10+)
- [ ] Zlib development headers (Linux only)

### Step-by-Step Build Process

#### Step 1: Clone Repository

```bash
# Clone with submodules (required)
git clone --recursive https://github.com/microsoft/onnxruntime.git
cd onnxruntime

# If already cloned without --recursive:
git submodule update --init --recursive
```

#### Step 2: Set Environment Variables

**Linux:**
```bash
export CUDA_HOME=/usr/local/cuda-12.6
export CUDNN_HOME=/usr/lib/x86_64-linux-gnu/
export CUDACXX=/usr/local/cuda-12.6/bin/nvcc
export PATH=/usr/local/cuda-12.6/bin:$PATH
export LD_LIBRARY_PATH=/usr/local/cuda-12.6/lib64:$LD_LIBRARY_PATH
```

**Windows (PowerShell):**
```powershell
$env:CUDA_HOME="C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.6"
$env:CUDNN_HOME="C:\Program Files\NVIDIA\CUDNN\v9.0"
$env:PATH="$env:CUDA_HOME\bin;$env:PATH"
```

#### Step 3: Build ONNX Runtime

**Linux - Full Multi-Architecture Build:**
```bash
./build.sh \
  --config Release \
  --build_shared_lib \
  --parallel \
  --update \
  --build \
  --use_cuda \
  --cuda_version 12.6 \
  --cuda_home $CUDA_HOME \
  --cudnn_home $CUDNN_HOME \
  --build_wheel \
  --skip_tests \
  --cmake_extra_defines CMAKE_CUDA_ARCHITECTURES="50;70;75;80;86;89;90" \
  --cmake_extra_defines CMAKE_CUDA_COMPILER=$CUDACXX \
  --cmake_extra_defines onnxruntime_BUILD_UNIT_TESTS=OFF \
  --cmake_extra_defines onnxruntime_ENABLE_NVTX_PROFILE=ON
```

**Expected Build Time:**
- Single architecture: 15-30 minutes (4-core CPU)
- Multi-architecture (4+ SM versions): 45-90 minutes (4-core CPU)
- With parallel build on 8+ cores: 20-40 minutes

**Windows - Full Multi-Architecture Build:**
```cmd
.\build.bat ^
  --config Release ^
  --build_shared_lib ^
  --parallel ^
  --update ^
  --build ^
  --use_cuda ^
  --cuda_version 12.6 ^
  --cuda_home "%CUDA_HOME%" ^
  --cudnn_home "%CUDNN_HOME%" ^
  --build_wheel ^
  --skip_tests ^
  --cmake_extra_defines CMAKE_CUDA_ARCHITECTURES="50;70;75;80;86;89;90" ^
  --cmake_generator "Visual Studio 17 2022" ^
  --cmake_extra_defines onnxruntime_BUILD_UNIT_TESTS=OFF
```

#### Step 4: Verify Build

```bash
# Check build output
ls -lh build/Linux/Release/libonnxruntime.so*
# Expected: ~500-800 MB for multi-arch build

# Install Python wheel
pip install build/Linux/Release/dist/onnxruntime_gpu-*.whl

# Test CUDA provider
python3 -c "import onnxruntime as ort; print(ort.get_available_providers())"
# Expected output should include: 'CUDAExecutionProvider'
```

#### Step 5: Test CUDA Execution

```python
import onnxruntime as ort
import numpy as np

# Create session with CUDA provider
session = ort.InferenceSession(
    "model.onnx",
    providers=[
        ('CUDAExecutionProvider', {
            'device_id': 0,
            'gpu_mem_limit': 2 * 1024 * 1024 * 1024,  # 2GB
            'arena_extend_strategy': 'kNextPowerOfTwo',
        }),
        'CPUExecutionProvider'
    ]
)

# Verify CUDA provider is active
print("Active providers:", session.get_providers())
# Expected: ['CUDAExecutionProvider', 'CPUExecutionProvider']

# Check compute capability (requires nvidia-smi)
import subprocess
result = subprocess.run(['nvidia-smi', '--query-gpu=compute_cap', '--format=csv,noheader'],
                       capture_output=True, text=True)
print(f"GPU Compute Capability: {result.stdout.strip()}")
```

---

## 8. Build Time Estimates

### Estimated Build Times

Based on research and community reports:

| Configuration | 4-Core CPU | 8-Core CPU | 16-Core CPU | Notes |
|---------------|-----------|-----------|-------------|-------|
| Single arch (sm_80) | 15-25 min | 10-15 min | 8-12 min | Baseline |
| Multi-arch (4 SMs) | 45-70 min | 25-40 min | 15-25 min | ~2-3x single |
| Multi-arch (7 SMs) | 70-120 min | 40-60 min | 25-40 min | ~3-4x single |
| Full build + tests | 120-180 min | 60-90 min | 40-60 min | With unit tests |

**Factors affecting build time:**
- Number of target architectures (primary factor)
- CPU cores and parallel build settings
- Disk I/O speed (SSD vs HDD)
- Available RAM (16GB+ recommended)
- Build configuration (Release vs RelWithDebInfo)

### Build Time Optimization

```bash
# Use all available cores
--parallel

# Skip tests (saves 30-50% build time)
--skip_tests

# Use Ninja generator (faster than Make)
--cmake_generator Ninja

# Use ccache for incremental builds
export CMAKE_C_COMPILER_LAUNCHER=ccache
export CMAKE_CXX_COMPILER_LAUNCHER=ccache

# Minimal debug symbols
--config Release  # instead of RelWithDebInfo
```

---

## 9. Troubleshooting Common Build Issues

### Issue 1: CMAKE_CUDA_ARCHITECTURES Not Working

**Problem:** CMake ignoring `CMAKE_CUDA_ARCHITECTURES` setting

**Solution:**
```bash
# Explicitly set CUDACXX environment variable
export CUDACXX=/usr/local/cuda-12.6/bin/nvcc

# Use cmake_extra_defines with explicit compiler
--cmake_extra_defines CMAKE_CUDA_COMPILER=/usr/local/cuda-12.6/bin/nvcc \
--cmake_extra_defines CMAKE_CUDA_ARCHITECTURES="50;70;80;90"
```

### Issue 2: Build Warnings for Old SM Versions

**Problem:** Many warnings about deprecated sm_50, sm_52

**Expected Behavior:** These are warnings, not errors. Build will succeed.

**Suppress warnings (optional):**
```bash
--cmake_extra_defines CMAKE_CUDA_FLAGS="-w"
```

### Issue 3: cuDNN 9.x Compatibility

**Problem:** Runtime error about cuDNN version mismatch

**Solution:** Ensure cuDNN 9.x is properly installed and environment variables point to correct location:
```bash
# Verify cuDNN installation
ls -la $CUDNN_HOME/lib/libcudnn.so*
# Should show version 9.x.x

# Check runtime linking
ldd build/Linux/Release/libonnxruntime.so | grep cudnn
```

### Issue 4: Out of Memory During Build

**Problem:** Compiler crashes with OOM error

**Solution:**
```bash
# Reduce parallel jobs
./build.sh --parallel 4  # instead of using all cores

# Or limit memory per job
export CMAKE_BUILD_PARALLEL_LEVEL=4
```

### Issue 5: CUDA Not Found

**Problem:** Build script cannot find CUDA installation

**Solution:**
```bash
# Verify CUDA installation
nvcc --version

# Explicitly set paths
--cuda_home /usr/local/cuda-12.6 \
--cudnn_home /usr/lib/x86_64-linux-gnu/

# Ensure nvcc is in PATH
export PATH=/usr/local/cuda-12.6/bin:$PATH
```

---

## 10. Production Recommendations

### Recommended Build Configuration

For production deployment supporting Maxwell through Hopper GPUs:

```bash
#!/bin/bash
# Production ONNX Runtime build script

set -e

# Configuration
CUDA_VERSION=12.6
ONNX_RT_VERSION=v1.24.0  # or latest stable
BUILD_CONFIG=Release
TARGET_ARCHS="50;70;75;80;86;89;90"

# Environment
export CUDA_HOME=/usr/local/cuda-${CUDA_VERSION}
export CUDNN_HOME=/usr/lib/x86_64-linux-gnu/
export CUDACXX=${CUDA_HOME}/bin/nvcc
export PATH=${CUDA_HOME}/bin:$PATH

# Clone and checkout
git clone --recursive --branch ${ONNX_RT_VERSION} \
    https://github.com/microsoft/onnxruntime.git
cd onnxruntime

# Build
./build.sh \
  --config ${BUILD_CONFIG} \
  --build_shared_lib \
  --parallel \
  --update \
  --build \
  --use_cuda \
  --cuda_version ${CUDA_VERSION} \
  --cuda_home ${CUDA_HOME} \
  --cudnn_home ${CUDNN_HOME} \
  --build_wheel \
  --skip_tests \
  --cmake_extra_defines CMAKE_CUDA_ARCHITECTURES="${TARGET_ARCHS}" \
  --cmake_extra_defines CMAKE_CUDA_COMPILER=${CUDACXX} \
  --cmake_extra_defines onnxruntime_BUILD_UNIT_TESTS=OFF \
  --cmake_extra_defines onnxruntime_ENABLE_NVTX_PROFILE=ON \
  --cmake_extra_defines CMAKE_INSTALL_PREFIX=/opt/onnxruntime

echo "Build complete! Wheel location:"
find build -name "*.whl"
```

### Deployment Considerations

**Binary Distribution:**
- Distribute Python wheel (.whl) for easy installation
- Document CUDA 12.x + cuDNN 9.x runtime requirements
- Provide Docker images for consistent environment

**Runtime Requirements:**
- CUDA Toolkit 12.x (runtime libraries only, full toolkit not needed)
- cuDNN 9.x runtime libraries
- NVIDIA driver supporting CUDA 12.x (Driver version 525+)

**License Compliance:**
- ONNX Runtime: MIT License
- CUDA Toolkit: NVIDIA CUDA EULA
- cuDNN: NVIDIA cuDNN License

---

## 11. Performance Benchmarking

### Testing Build Performance

```python
#!/usr/bin/env python3
"""Benchmark ONNX Runtime CUDA performance across architectures"""

import onnxruntime as ort
import numpy as np
import time

def benchmark_inference(model_path, input_shape, num_runs=100):
    """Benchmark inference performance"""

    # Create session with CUDA
    session = ort.InferenceSession(
        model_path,
        providers=[('CUDAExecutionProvider', {'device_id': 0})]
    )

    # Prepare input
    input_name = session.get_inputs()[0].name
    x = np.random.randn(*input_shape).astype(np.float32)

    # Warmup
    for _ in range(10):
        session.run(None, {input_name: x})

    # Benchmark
    start = time.perf_counter()
    for _ in range(num_runs):
        session.run(None, {input_name: x})
    end = time.perf_counter()

    avg_time = (end - start) / num_runs * 1000  # ms
    throughput = num_runs / (end - start)

    print(f"Average inference time: {avg_time:.2f} ms")
    print(f"Throughput: {throughput:.2f} inferences/sec")

    return avg_time, throughput

if __name__ == "__main__":
    # Check CUDA provider availability
    providers = ort.get_available_providers()
    print(f"Available providers: {providers}")

    if 'CUDAExecutionProvider' not in providers:
        print("ERROR: CUDA provider not available!")
        exit(1)

    # Run benchmark
    benchmark_inference("model.onnx", (1, 3, 224, 224))
```

---

## 12. Additional Resources

### Official Documentation
- **ONNX Runtime Build Guide:** https://onnxruntime.ai/docs/build/
- **CUDA Execution Provider:** https://onnxruntime.ai/docs/execution-providers/CUDA-ExecutionProvider.html
- **NVIDIA CUDA Documentation:** https://docs.nvidia.com/cuda/
- **cuDNN Documentation:** https://docs.nvidia.com/deeplearning/cudnn/

### Community Resources
- **ONNX Runtime GitHub:** https://github.com/microsoft/onnxruntime
- **ONNX Runtime Issues:** https://github.com/microsoft/onnxruntime/issues
- **NVIDIA Developer Forums:** https://forums.developer.nvidia.com/

### Compute Capability Reference
- **GPU Compute Capability Table:** https://developer.nvidia.com/cuda-gpus
- **Architecture Matching Guide:** https://arnon.dk/matching-sm-architectures-arch-and-gencode-for-various-nvidia-cards/

---

## 13. Conclusion

### Key Takeaways

1. **CUDA 12.6 or 12.9 is required for sm_50 (Maxwell) support** - Future CUDA 13+ will not support Maxwell/Pascal/Volta

2. **Multi-architecture builds are essential for broad GPU compatibility** - Include only the architectures you need to minimize binary size

3. **Binary size scales with number of architectures** - Each SM version adds ~50-150 MB to the final binary

4. **No runtime performance penalty** - CUDA runtime automatically selects optimal code path for detected GPU

5. **Build time increases linearly with architectures** - Plan for 45-90 minutes for 4+ architecture builds on typical hardware

### Recommended Configuration

For maximum compatibility (Maxwell → Hopper):
```bash
CMAKE_CUDA_ARCHITECTURES="50;70;75;80;86;89;90"
CUDA_VERSION=12.6 or 12.9
cuDNN_VERSION=9.x
```

For modern GPUs only (Volta → Hopper):
```bash
CMAKE_CUDA_ARCHITECTURES="70;75;80;86;89;90"
CUDA_VERSION=12.6+ or 13.x
cuDNN_VERSION=9.x
```

### Next Steps

1. Install CUDA 12.6/12.9 and cuDNN 9.x
2. Clone ONNX Runtime repository with submodules
3. Set environment variables for CUDA paths
4. Run build script with appropriate CMAKE_CUDA_ARCHITECTURES
5. Test with target GPU hardware
6. Deploy with clear runtime requirements documentation

---

**Document Version:** 1.0
**Last Updated:** 2025-11-13
**Research Source:** Web search and official ONNX Runtime documentation
**Target ONNX Runtime Version:** 1.24.0+
