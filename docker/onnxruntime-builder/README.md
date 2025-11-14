# ONNX Runtime Multi-Architecture Builder

Docker-based build system for creating ONNX Runtime libraries with custom CUDA compute capability support.

## Quick Start

```bash
# 1. Build Docker image (once, ~10-15 minutes)
./docker-build.sh build-image

# 2. Test build (verify environment, ~45 minutes)
./docker-build.sh test

# 3. Build production packages (2-3.5 hours total)
./docker-build.sh legacy    # sm_50-70 for Maxwell/Pascal/Volta
./docker-build.sh modern    # sm_75-86 for Turing/Ampere
./docker-build.sh latest    # sm_89-90 for Ada/Hopper/Blackwell
```

## Architecture Packages

### Legacy Package (sm_50,52,60,61,70)
**Target GPUs:**
- GTX 750, GTX 750 Ti (Maxwell GM107)
- GTX 900 series (Maxwell GM204/206)
- GTX 1000 series (Pascal GP102/104/106/107)
- Quadro M/P series
- Titan V, V100 (Volta GV100)

**Build time:** ~60 minutes
**Binary size:** ~150 MB (libonnxruntime_providers_cuda.so)

### Modern Package (sm_75,80,86)
**Target GPUs:**
- GTX 1600 series (Turing TU116/117)
- RTX 2000 series (Turing TU102/104/106)
- RTX 3000 series (Ampere GA102/104)
- A100, A6000 (Ampere GA100)

**Build time:** ~50 minutes
**Binary size:** ~180 MB

### Latest Package (sm_89,90,100,120)
**Target GPUs:**
- RTX 4090 (Ada Lovelace AD102) - sm_89
- H100 (Hopper GH100) - sm_90
- **Blackwell GPUs (native support):**
  - B100, B200 (Blackwell GB100/GB200) - sm_100
  - RTX PRO 6000 Blackwell - sm_120
  - RTX 50 series (5090, 5080) - sm_120

**Build time:** ~50 minutes
**Binary size:** ~180 MB

**Note:** CUDA 12.9 provides native sm_120 compilation while maintaining sm_50 support (CUDA 13.0+ drops sm_50).

## Build Output Structure

```
output/
├── test/           # Test build (sm_90 only)
│   ├── libonnxruntime.so
│   ├── libonnxruntime_providers_cuda.so
│   ├── libonnxruntime_providers_shared.so
│   └── libonnxruntime_providers_tensorrt.so
├── legacy/         # Legacy package files
├── modern/         # Modern package files
└── latest/         # Latest package files
```

## System Requirements

- Docker with NVIDIA Container Toolkit
- 30+ GB free disk space
- NVIDIA GPU (for --gpus all flag, optional)
- 16+ GB RAM

## Advanced Usage

### Custom Architecture Build
```bash
./docker-build.sh custom "52;70;86"
```

### Build Inside Container (manual)
```bash
docker run -it --gpus all onnxruntime-builder:cuda12.6 /bin/bash
# Inside container:
/workspace/build-onnxruntime.sh "90"
```

### Extract Artifacts Manually
```bash
docker run --rm \
  -v $(pwd)/output:/output \
  onnxruntime-builder:cuda12.6 \
  bash -c 'cp /workspace/onnxruntime/build/Linux/Release/*.so /output/'
```

## Verification

### Check Supported Architectures
```bash
cuobjdump --list-ptx output/legacy/libonnxruntime_providers_cuda.so | grep "PTX file"
```

### Check CUDA Dependencies
```bash
ldd output/legacy/libonnxruntime_providers_cuda.so | grep cuda
```

## Troubleshooting

### Docker Build Fails
- Ensure you have 30+ GB free space
- Check CUDA 12.6 base image is available
- Verify internet connection for downloads

### Build Takes Too Long
- Use `--skip_tests` (already enabled by default)
- Build on system with more CPU cores
- Close other applications to free RAM

### Out of Memory During Build
- Reduce parallel jobs: Edit build-onnxruntime.sh, remove `--parallel`
- Increase Docker memory limit: Docker Desktop → Resources → Memory

## References

- [ONNX Runtime Build Docs](https://onnxruntime.ai/docs/build/)
- [CUDA Compute Capabilities](https://developer.nvidia.com/cuda-gpus)
- [Research: `/opt/swictation/docs/research/onnxruntime-cuda-build-guide.md`](../../docs/research/onnxruntime-cuda-build-guide.md)

## License

ONNX Runtime: MIT License
This build system: Apache 2.0
