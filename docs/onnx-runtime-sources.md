# ONNX Runtime Official Binaries

This document describes where to obtain official ONNX Runtime binaries for Swictation platform packages.

## Official Release Source

**GitHub Releases:** https://github.com/microsoft/onnxruntime/releases

Microsoft provides pre-built binaries for multiple platforms and configurations.

## Linux (x86_64) - GPU Acceleration

**Version:** 1.23.2
**Purpose:** CUDA 12.9 GPU acceleration for NVIDIA GPUs
**Download URL:**
```
https://github.com/microsoft/onnxruntime/releases/download/v1.23.2/onnxruntime-linux-x64-gpu-1.23.2.tgz
```

**Contents:**
- `libonnxruntime.so` - Core ONNX Runtime library
- `libonnxruntime_providers_cuda.so` - CUDA execution provider
- `libonnxruntime_providers_shared.so` - Shared provider library
- `libonnxruntime_providers_tensorrt.so` - TensorRT execution provider (optional)

**Target GPUs:**
- **Modern GPUs** (sm_75-90): RTX 2000/3000/4000 series, A100, H100
- **Latest GPUs** (sm_89-120): RTX 4090, H100, Blackwell (B100/B200)
- Requires CUDA 12.9 and cuDNN 9.15.1

**Why 1.23.2:**
- Native CUDA 12.9 support
- Compute capability sm_50-120 range (Maxwell through Blackwell)
- Improved performance for Transformer models

**Legacy GPU Support:**
For older GPUs (Maxwell/Pascal/Volta), Swictation provides custom builds with CUDA 11.8:
- ONNX Runtime 1.23.2 rebuilt with CUDA 11.8
- Located in `~/.local/share/swictation/gpu-libs/`
- See `/opt/swictation/docker/onnxruntime-builder/` for build scripts

## macOS (Apple Silicon) - CoreML Acceleration

**Version:** 1.22.0
**Purpose:** CoreML GPU/Neural Engine acceleration for M1/M2/M3 chips
**Download URL:**
```
https://github.com/microsoft/onnxruntime/releases/download/v1.22.0/onnxruntime-osx-arm64-1.22.0.tgz
```

**Contents:**
- `libonnxruntime.dylib` - Core ONNX Runtime with CoreML provider built-in

**Target Hardware:**
- **Apple Silicon only:** M1, M1 Pro, M1 Max, M1 Ultra
- M2, M2 Pro, M2 Max, M2 Ultra
- M3, M3 Pro, M3 Max

**Why 1.22.0 (not 1.23.x):**
ONNX Runtime 1.23.x has a regression affecting external data + CoreML execution:
- Issue: https://github.com/microsoft/onnxruntime/issues/26261
- Swictation models use external data format (weights in separate .bin file)
- 1.22.0 is the last stable version supporting this configuration

**CoreML vs CUDA:**
- CoreML uses Apple's Metal Performance Shaders for GPU acceleration
- No CUDA support on macOS (Apple Silicon does not support NVIDIA GPUs)
- CoreML can dispatch to CPU, GPU (Metal), or Neural Engine automatically

## GitHub Actions Caching

Both workflows cache ONNX Runtime libraries to speed up builds:

### Linux Workflow
```yaml
- name: Cache ONNX Runtime libraries
  uses: actions/cache@v4
  with:
    path: ~/.cache/onnxruntime
    key: onnxruntime-linux-gpu-1.23.2
```

### macOS Workflow
```yaml
- name: Cache ONNX Runtime libraries
  uses: actions/cache@v4
  with:
    path: ~/.cache/onnxruntime
    key: onnxruntime-macos-arm64-coreml-1.22.0
```

**Cache Benefits:**
- First build: ~30 seconds to download (80-120 MB compressed)
- Cached builds: Instant library retrieval
- Shared across workflow runs

## Version Update Process

When updating ONNX Runtime versions:

1. **Update versions.json:**
   ```json
   "libraries": {
     "onnxruntime": {
       "linux-gpu": "1.23.2",
       "macos-coreml": "1.22.0"
     }
   }
   ```

2. **Update GitHub Actions workflows:**
   - `.github/workflows/build-linux.yml` - Update download URL and cache key
   - `.github/workflows/build-macos.yml` - Update download URL and cache key

3. **Update this documentation** with new version numbers and URLs

4. **Test thoroughly:**
   - Verify libraries load correctly with daemon
   - Check model inference works on both platforms
   - Validate GPU acceleration is active

## Alternative Sources

### Custom Builds (Advanced)

For specialized requirements, build ONNX Runtime from source:

**Location:** `/opt/swictation/docker/onnxruntime-builder/`

**Use cases:**
- Custom CUDA compute capabilities
- Legacy GPU support (CUDA 11.8)
- Specific operator optimization
- Debug builds

**Build time:** 45-70 minutes per architecture

### CPU-Only Versions

For CPU-only deployments, use standard CPU releases:
- Linux CPU: `onnxruntime-linux-x64-{version}.tgz`
- macOS CPU: `onnxruntime-osx-arm64-{version}.tgz` (without GPU providers)

## Verification

After downloading, verify library integrity:

### Linux
```bash
# Check ELF format
file libonnxruntime.so
# Expected: ELF 64-bit LSB shared object, x86-64

# Check for CUDA symbols
nm libonnxruntime_providers_cuda.so | grep -i cuda | head -5
```

### macOS
```bash
# Check Mach-O format
file libonnxruntime.dylib
# Expected: Mach-O 64-bit dynamically linked shared library arm64

# Check for CoreML symbols
nm libonnxruntime.dylib | grep -i coreml | head -5
```

## Support and Issues

- **ONNX Runtime Issues:** https://github.com/microsoft/onnxruntime/issues
- **Swictation Issues:** https://github.com/robertelee78/swictation/issues
- **Documentation:** https://onnxruntime.ai/docs/

---

**Last Updated:** 2025-11-26
**Maintained By:** Swictation Project
