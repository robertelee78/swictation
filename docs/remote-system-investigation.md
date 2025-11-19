# Remote System Investigation - Dad's Laptop

**Date**: 2025-11-18
**System**: jrl@192.168.1.201:2222
**Investigation**: CUDA compatibility and swictation daemon failure

---

## Executive Summary

The swictation daemon is failing on dad's laptop due to **CUDA architecture incompatibility**. The NVIDIA Quadro M2200 GPU uses Maxwell architecture (compute capability 5.2), but the bundled ONNX Runtime CUDA provider was compiled only for newer architectures (Volta sm_70 and later). The system correctly detects this and falls back to CPU execution, but a separate ONNX Runtime initialization error is causing daemon crashes.

---

## System Configuration

### Hardware
- **GPU**: NVIDIA Quadro M2200
- **Compute Capability**: 5.2 (sm_52) - Maxwell architecture
- **VRAM**: 4096 MiB (4GB)
- **Driver**: 580.95.05 (CUDA 13.0 compatible)

### Software Environment
- **OS**: Ubuntu with Linux 6.17.0-6-generic
- **Node.js**: v20.19.4
- **npm**: 9.2.0
- **Swictation**: 0.4.15 (global npm install)
- **CUDA Toolkit**: NOT INSTALLED (no nvcc, no /usr/local/cuda)
- **cuDNN**: NOT INSTALLED (system), bundled cuDNN 9.x in ~/.local/share/swictation/gpu-libs/

---

## Critical Finding: CUDA Architecture Mismatch

### The Problem

The ONNX Runtime CUDA provider library (`libonnxruntime_providers_cuda.so`) was compiled for modern GPU architectures only:

**Supported architectures in the binary**:
```
sm_70  - Volta (Tesla V100, 2017)
sm_75  - Turing (RTX 20 series, 2018)
sm_80  - Ampere (A100, RTX 30 series, 2020)
sm_86  - Ampere (RTX 3090, 2020)
sm_89  - Ada Lovelace (RTX 40 series, 2022)
sm_90  - Hopper (H100, 2022)
sm_90a - Hopper variant
```

**Required architecture**:
```
sm_52 - Maxwell 2.0 (Quadro M2200, 2016) ❌ NOT INCLUDED
```

### Evidence from System

**GPU Detection**:
```bash
$ nvidia-smi --query-gpu=compute_cap --format=csv,noheader
5.2
```

**Binary Inspection**:
```bash
$ strings libonnxruntime_providers_cuda.so | grep "sm_"
.target sm_70
.target sm_75
.target sm_80
.target sm_86
.target sm_89
.target sm_90
.target sm_90a
```

**Daemon Logs**:
```
⚠️  Detected Maxwell GPU (compute capability 5.2) - forcing CPU execution for 0.6B model
    Maxwell GPU detected: disabling GPU acceleration for 0.6B model

⚠️  Detected Maxwell GPU (compute capability 5.2) with potential cuDNN 9.x incompatibility
Silero VAD: Detected Maxwell GPU (sm_50-52) - forcing CPU execution
            STT will still use GPU acceleration
```

The code **correctly detects** the incompatibility and falls back to CPU.

---

## CUDA Library Analysis

### Bundled Libraries Location
All CUDA libraries are bundled in: `~/.local/share/swictation/gpu-libs/`

**Complete inventory** (2.7 GB total):
```
libcublas.so.12                           105 MB
libcublasLt.so.12                         747 MB
libcudart.so.12                           736 KB
libcudnn.so.9                             125 KB
libcudnn_adv.so.9                         270 MB
libcudnn_cnn.so.9                         3.7 MB
libcudnn_engines_precompiled.so.9         502 MB
libcudnn_engines_runtime_compiled.so.9    29 MB
libcudnn_graph.so.9                       3.8 MB
libcudnn_heuristic.so.9                   54 MB
libcudnn_ops.so.9                         106 MB
libcufft.so.11                            289 MB
libcurand.so.10                           166 MB
libnvrtc.so.12                            106 MB
libonnxruntime_providers_cuda.so          345 MB  ⚠️ INCOMPATIBLE WITH sm_52
```

### Library Path Configuration

**Systemd Service** (`~/.config/systemd/user/swictation-daemon.service`):
```ini
Environment="LD_LIBRARY_PATH=/usr/local/cuda/lib64:/usr/local/cuda-12.9/lib64:~/.local/share/swictation/gpu-libs:~/.npm-global/lib/node_modules/swictation/lib/native"
```

**Status**:
- ❌ `/usr/local/cuda/lib64` - Does not exist (no toolkit installed)
- ❌ `/usr/local/cuda-12.9/lib64` - Does not exist
- ✅ `~/.local/share/swictation/gpu-libs` - **EXISTS** with all CUDA libraries
- ✅ `~/.npm-global/lib/node_modules/swictation/lib/native` - EXISTS

The bundled libraries are present and properly configured in LD_LIBRARY_PATH.

---

## Swictation Service Status

### Installation Details
- **Package**: swictation@0.4.15
- **Install Type**: Global npm (`npm install -g`)
- **Binary Location**: `/usr/local/bin/swictation` (symlink)
- **Daemon Binary**: `~/.npm-global/lib/node_modules/swictation/lib/native/swictation-daemon.bin`
- **Service Type**: systemd user service

### Current Service State
**Status**: ❌ FAILING

**Latest Error** (from journalctl):
```
Exception during initialization: /onnxruntime_src/onnxruntime/core/optimizer/initializer.cc:33
onnxruntime::Initializer::Initializer(const onnx::TensorProto&, const std::filesystem::__cxx11::path&)
!model_path.empty() was false. model_path must not be empty.
Ensure that a path is provided when the model is created or loaded.

swictation-daemon.service: Main process exited, code=dumped, status=6/ABRT
swictation-daemon.service: Failed with result 'core-dump'.
```

**Analysis**: This is a **separate issue** from CUDA compatibility. The model path is not being passed correctly to ONNX Runtime during initialization.

---

## Root Cause Analysis

### Issue 1: GPU Acceleration Disabled (Expected Behavior)
- **Cause**: ONNX Runtime provider compiled without Maxwell (sm_52) support
- **Impact**: CPU fallback for inference (slower but functional)
- **Status**: **WORKING AS DESIGNED** - Code correctly detects and handles this

### Issue 2: Daemon Crash (Active Bug)
- **Cause**: Model path not provided to ONNX Runtime initializer
- **Impact**: Service won't start
- **Status**: **CRITICAL BUG** - Needs immediate fix

### Issue 3: cuDNN 9.x Compatibility
- **Potential Issue**: cuDNN 9.x may have deprecated Maxwell support
- **Status**: Not confirmed, but code already handles with CPU fallback

---

## Solutions & Recommendations

### Immediate Fix (Issue 2)
**Fix model initialization bug**:
- Investigate why model_path is empty during ONNX Runtime initialization
- Likely a recent regression in swictation code
- Check model loading code in daemon binary

### GPU Acceleration (Issue 1)

#### Option A: Accept CPU Fallback (EASIEST)
- No changes needed
- Performance penalty but system is functional
- Document Maxwell GPU limitation

#### Option B: Rebuild ONNX Runtime with Maxwell Support
**Requirements**:
1. Install CUDA Toolkit 11.8 (last version with full Maxwell support)
   ```bash
   wget https://developer.download.nvidia.com/compute/cuda/11.8.0/local_installers/cuda_11.8.0_520.61.05_linux.run
   sudo sh cuda_11.8.0_520.61.05_linux.run
   ```

2. Install cuDNN 8.9 (compatible with CUDA 11.8 and Maxwell)
   ```bash
   # Download from NVIDIA Developer
   # Install to /usr/local/cuda-11.8
   ```

3. Rebuild ONNX Runtime with Maxwell support:
   ```bash
   cmake -DCMAKE_CUDA_ARCHITECTURES="52;70;75;80;86" \
         -Donnxruntime_USE_CUDA=ON \
         -DCUDA_TOOLKIT_ROOT_DIR=/usr/local/cuda-11.8
   ```

4. Replace `libonnxruntime_providers_cuda.so` in `~/.local/share/swictation/gpu-libs/`

**Complexity**: HIGH - Requires CUDA development environment

#### Option C: Install Newer GPU
- Upgrade to GPU with sm_70+ (Volta or newer)
- Most RTX series cards qualify
- **Cost**: $200-$2000 depending on model

---

## Verification Checklist

### Completed ✅
- [x] GPU model and compute capability confirmed (Quadro M2200, sm_52)
- [x] Driver version verified (580.95.05, CUDA 13.0)
- [x] CUDA toolkit status checked (not installed)
- [x] cuDNN system libraries checked (not installed)
- [x] Bundled CUDA libraries inventory (2.7 GB in gpu-libs/)
- [x] ONNX Runtime architecture support analyzed (sm_70+, missing sm_52)
- [x] Service logs reviewed (Maxwell detection working, model init failing)
- [x] LD_LIBRARY_PATH configuration verified
- [x] Node.js/npm versions documented

### To Investigate 🔍
- [ ] Model initialization failure root cause
- [ ] Model file locations and paths
- [ ] sherpa-rs library integration
- [ ] ONNX model loading flow in daemon
- [ ] Recent code changes that may have broken model loading

---

## Next Steps

1. **Priority 1**: Fix model initialization crash
   - Review model loading code
   - Check file paths and permissions
   - Test with minimal ONNX model

2. **Priority 2**: Document Maxwell GPU limitations
   - Update README with GPU compatibility matrix
   - Add detection to installer/postinstall
   - Provide clear CPU fallback messaging

3. **Priority 3**: Decide on GPU acceleration approach
   - Evaluate performance impact of CPU fallback
   - Assess cost/benefit of CUDA toolkit installation
   - Consider supporting Maxwell in future builds

---

## Technical Details

### System Information
```
Hostname: 7520
Kernel: Linux 6.17.0-6-generic
Architecture: x86_64
GCC: 15.2.0
```

### NVIDIA Driver
```
Driver Version: 580.95.05
CUDA Version: 13.0
NVRM Version: NVIDIA UNIX x86_64 Kernel Module 580.95.05
```

### Service File Location
```
~/.config/systemd/user/swictation-daemon.service
```

### Log Commands
```bash
# View daemon logs
journalctl --user -u swictation-daemon --no-pager -n 100

# Check service status
systemctl --user status swictation-daemon

# Monitor real-time
journalctl --user -u swictation-daemon -f
```

---

**Report Generated**: 2025-11-18
**Investigator**: Coder Agent (Hive Mind)
**Coordination**: Memory stored at `hive/remote/system_info`
