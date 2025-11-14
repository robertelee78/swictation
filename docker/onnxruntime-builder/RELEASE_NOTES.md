# GPU Libraries v1.1.0 - Multi-Architecture CUDA Support

## What's New in v1.1.0

### Native Blackwell sm_120 Support 🎉
- **CUDA 12.9** with native compilation for NVIDIA Blackwell architecture (sm_120)
- First release to support RTX PRO 6000 Blackwell and upcoming RTX 50 series GPUs
- Full backward compatibility: sm_50 (Maxwell 2014) through sm_120 (Blackwell 2024)

### Three Optimized Architecture Packages
Instead of a single large package, v1.1.0 provides three optimized variants. Users download only what their GPU needs:

1. **cuda-libs-legacy.tar.gz** (1.5GB compressed, 2.3GB uncompressed)
   - **Architecture support:** sm_50, sm_52, sm_60, sm_61, sm_70
   - **Target GPUs:** Maxwell, Pascal, Volta (2014-2017)
   - **Examples:** GTX 750/900/1000, Quadro M/P series, Titan V, V100

2. **cuda-libs-modern.tar.gz** (1.5GB compressed, 2.3GB uncompressed)
   - **Architecture support:** sm_75, sm_80, sm_86
   - **Target GPUs:** Turing, Ampere (2018-2021)
   - **Examples:** GTX 16/RTX 20/30 series, A100, RTX A1000-A6000

3. **cuda-libs-latest.tar.gz** (1.5GB compressed, 2.3GB uncompressed)
   - **Architecture support:** sm_89, sm_90, sm_100, sm_120
   - **Target GPUs:** Ada Lovelace, Hopper, Blackwell (2022-2024)
   - **Examples:** RTX 4090, H100, B100/B200, RTX PRO 6000 Blackwell, RTX 50 series

## Package Contents

Each package includes:

### ONNX Runtime 1.23.2
- `libonnxruntime.so` - Core runtime library
- `libonnxruntime_providers_cuda.so` - CUDA execution provider (architecture-specific)
- `libonnxruntime_providers_shared.so` - Shared provider utilities

### CUDA 12.9 Runtime Libraries
- `libcublas.so.12` (101MB) - CUDA Basic Linear Algebra Subroutines
- `libcublasLt.so.12` (713MB) - cuBLAS with tensor cores
- `libcudart.so.12` (720KB) - CUDA runtime
- `libcufft.so.11` (277MB) - CUDA Fast Fourier Transform
- `libcurand.so.10` (160MB) - CUDA random number generation
- `libnvrtc.so.12` (102MB) - NVIDIA runtime compilation

### cuDNN 9.15.1 Deep Learning Libraries
- `libcudnn.so.9` - Main cuDNN library
- `libcudnn_adv.so.9` (259MB) - Advanced operations
- `libcudnn_cnn.so.9` (4MB) - Convolutional neural networks
- `libcudnn_engines_precompiled.so.9` (479MB) - Precompiled kernels
- `libcudnn_engines_runtime_compiled.so.9` (28MB) - Runtime compilation
- `libcudnn_graph.so.9` (4MB) - Graph API
- `libcudnn_heuristic.so.9` (53MB) - Heuristics engine
- `libcudnn_ops.so.9` (102MB) - Core operations

## Which Package Should I Download?

### Check Your GPU Compute Capability

Run this command to check your GPU:
```bash
nvidia-smi --query-gpu=name,compute_cap --format=csv
```

### Package Selection Guide

| Compute Capability | Package | Download |
|-------------------|---------|----------|
| sm_50 - sm_70 | Legacy | cuda-libs-legacy.tar.gz |
| sm_75 - sm_86 | Modern | cuda-libs-modern.tar.gz |
| sm_89 - sm_120 | Latest | cuda-libs-latest.tar.gz |

## Installation

### Automatic (with swictation npm package)
The swictation npm package will automatically detect your GPU and download the correct package during `npm install`.

### Manual Installation

1. **Download the appropriate package** for your GPU architecture
2. **Extract to your swictation installation:**
   ```bash
   tar -xzf cuda-libs-<variant>.tar.gz
   cd <variant>/libs
   cp *.so ~/.local/share/swictation/gpu-libs/
   ```

3. **Verify installation:**
   ```bash
   npx swictation test-gpu
   ```

## Technical Details

### Build Configuration
- **Base Image:** NVIDIA CUDA 12.9.0-devel-ubuntu22.04
- **ONNX Runtime:** v1.23.2 (built from source)
- **CMake:** 3.28.3
- **Compiler:** GCC 11.4.0, nvcc 12.9

### Build Flags
```cmake
CMAKE_CUDA_ARCHITECTURES=<architectures>
CMAKE_BUILD_TYPE=Release
--build_shared_lib
--use_cuda --cuda_version 12.9
--cudnn_home=/usr
--skip_tests
--parallel
```

### Architecture-Specific Sizes

| Package | CUDA Provider Size | Total Uncompressed |
|---------|-------------------|-------------------|
| Legacy (5 archs) | 223 MB | 2.3 GB |
| Modern (3 archs) | 368 MB | 2.3 GB |
| Latest (4 archs) | 682 MB | 2.3 GB |

## Verification

### Verify CUDA Architectures in Package
```bash
# Extract package
tar -xzf cuda-libs-latest.tar.gz

# Check compiled architectures
docker run --rm -v $(pwd):/data nvidia/cuda:12.9.0-devel-ubuntu22.04 \
  cuobjdump --list-elf /data/latest/libs/libonnxruntime_providers_cuda.so | grep sm_
```

Expected output for latest package:
```
sm_89
sm_90
sm_100
sm_120
```

## Changes from v1.0.1

### Added
- ✅ Native sm_120 (Blackwell) support via CUDA 12.9
- ✅ sm_100 (Blackwell B100/B200) support
- ✅ Three optimized architecture packages (was single package)
- ✅ Automatic GPU detection in npm postinstall
- ✅ Docker-based reproducible build system

### Improved
- ⬆️ CUDA 12.6 → 12.9 (native Blackwell compilation)
- ⬆️ cuDNN 9.x → 9.15.1 (latest stable)
- 📦 Package size reduction (download only what you need)
- 🚀 Build time: 51 minutes parallel (was ~2.5 hours sequential)

### Technical
- Full sm_50-120 range support in a single CUDA version (12.9)
- Last CUDA version supporting sm_50 before deprecation in CUDA 13.0
- Native compilation for all architectures (no PTX forward-compatibility needed)

## Compatibility

### Minimum Requirements
- **CUDA Driver:** 12.0+ (NVIDIA driver ≥525.60.13)
- **ONNX Runtime:** 1.23.0+
- **Linux:** Ubuntu 20.04+, similar distributions

### Tested On
- ✅ Quadro M2200 (sm_50) - Maxwell
- ✅ RTX A1000 (sm_86) - Ampere
- ✅ RTX PRO 6000 Blackwell (sm_120) - Blackwell

## Build Reproduction

To build these packages yourself:

```bash
cd docker/onnxruntime-builder

# Build Docker image (once)
./docker-build.sh build-image

# Build all three packages in parallel (~51 minutes)
./docker-build.sh legacy &
./docker-build.sh modern &
./docker-build.sh latest &
wait

# Package with CUDA runtime libraries
./package-cuda-libs.sh
```

See [docker/onnxruntime-builder/README.md](../../docker/onnxruntime-builder/README.md) for detailed build instructions.

## License

- **ONNX Runtime:** MIT License
- **CUDA Runtime:** NVIDIA Deep Learning Container License
- **cuDNN:** NVIDIA cuDNN Software License Agreement
- **Build System:** Apache 2.0

## Support

- **Issues:** https://github.com/robertelee78/swictation/issues
- **Documentation:** https://github.com/robertelee78/swictation#readme
- **Build Logs:** Available in release artifacts

---

**Note:** These packages are optimized for the swictation speech recognition system but can be used with any ONNX Runtime application requiring CUDA support.
