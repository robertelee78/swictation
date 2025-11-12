# Complete NPM Package Installation & Verification Guide

## Summary of All Fixes Applied

This document summarizes ALL fixes that were applied to ensure GPU functionality persists in npm packages:

### 1. **FP32 Model Selection for GPU** ✅
- **File**: `rust-crates/swictation-stt/src/recognizer_ort.rs` (lines 110-115)
- **Fix**: Code prefers FP32 models over INT8 when GPU is detected
- **Status**: Already implemented, just needed binary rebuild

### 2. **CUDA Environment in Service Files** ✅
- **Files**:
  - `/opt/swictation/config/swictation-daemon.service`
  - `/opt/swictation/npm-package/config/swictation-daemon.service`
  - `/opt/swictation/npm-package/templates/swictation-daemon.service.template`
- **Fix**: Added complete CUDA environment:
  ```systemd
  Environment="LD_LIBRARY_PATH=/usr/local/cuda/lib64:/usr/local/cuda-12.9/lib64:..."
  Environment="CUDA_HOME=/usr/local/cuda"
  Environment="ORT_DYLIB_PATH=/usr/local/lib/node_modules/swictation/lib/native/libonnxruntime.so"
  ```
- **Critical**: Must include `/usr/local/cuda-12.9/lib64` for cuDNN 9 support

### 3. **postinstall.js UI Service Installation** ✅
- **File**: `/opt/swictation/npm-package/postinstall.js` (lines 333-342)
- **Fix**: Added UI service installation after daemon service generation
- **Result**: Both services now installed by postinstall

### 4. **postinstall.js ONNX Runtime Detection** ✅
- **File**: `/opt/swictation/npm-package/postinstall.js` (lines 233-243)
- **Fix**: Check bundled GPU library FIRST before falling back to Python pip
- **Result**: Always uses GPU-enabled library with CUDA provider support

### 5. **Build Script Updates** ✅
- **File**: `/opt/swictation/scripts/build-npm-package.sh`
- **Added**:
  - Binary copy to `lib/native/swictation-daemon.bin`
  - CUDA library references (for documentation)
  - Service file verification
  - Complete end-to-end checks

## Critical Discovery: NPM Postinstall Doesn't Auto-Run

**THE BIGGEST ISSUE**: When installing with `sudo npm install -g`, the postinstall.js script **does NOT run automatically**.

### Why This Happens
- npm with sudo has complex permission handling
- Postinstall runs as root but needs to access user home directory
- This is a known npm behavior, not a bug in our package

### The Solution

Users MUST manually run postinstall after installing:

```bash
# Step 1: Install
sudo npm install -g swictation

# Step 2: Run postinstall (REQUIRED!)
cd /usr/local/lib/node_modules/swictation
node postinstall.js
```

## What postinstall.js Does (When Run)

1. ✅ Detects NVIDIA GPU
2. ✅ Downloads CUDA libraries from GitHub (330MB)
   - `libonnxruntime_providers_cuda.so`
   - `libonnxruntime_providers_shared.so`
   - `libonnxruntime_providers_tensorrt.so`
3. ✅ Detects ONNX Runtime (prefers bundled GPU library)
4. ✅ Generates systemd service files with correct paths
5. ✅ Installs both daemon and UI services

## Complete Verification Checklist

After installing and running postinstall, verify:

### ✓ Service Files Exist
```bash
ls -la ~/.config/systemd/user/swictation*.service
# Should show: swictation-daemon.service and swictation-ui.service
```

### ✓ Correct ORT Path (GPU-Enabled Library)
```bash
grep "ORT_DYLIB_PATH" ~/.config/systemd/user/swictation-daemon.service
# Should show: /usr/local/lib/node_modules/swictation/lib/native/libonnxruntime.so
# NOT: Python pip location
```

### ✓ CUDA Environment Complete
```bash
grep -E "cuda-12.9|CUDA_HOME" ~/.config/systemd/user/swictation-daemon.service
# Should show both cuda-12.9 path and CUDA_HOME
```

### ✓ CUDA Provider Library Exists
```bash
ls -lh /usr/local/lib/node_modules/swictation/lib/native/*cuda*
# Should show: libonnxruntime_providers_cuda.so (~330M)
```

### ✓ Services Start Successfully
```bash
systemctl --user daemon-reload
systemctl --user start swictation-daemon swictation-ui
systemctl --user status swictation-daemon swictation-ui
# Both should show: Active: active (running)
```

### ✓ GPU Provider Registered
```bash
journalctl --user -u swictation-daemon --since "1 min ago" | grep "Successfully registered.*CUDA"
# Should show: Successfully registered `CUDAExecutionProvider`
```

### ✓ cuDNN Loaded
```bash
journalctl --user -u swictation-daemon --since "1 min ago" | grep "cuDNN"
# Should show: cuDNN version: 91500 (appears 4 times for each model component)
```

### ✓ FP32 Models Loading
```bash
journalctl --user -u swictation-daemon --since "1 min ago" | grep "Using FP32"
# Should show:
#   Using FP32 model for GPU: encoder.onnx
#   Using FP32 model for GPU: decoder.onnx
#   Using FP32 model for GPU: joiner.onnx
```

### ✓ GPU Memory Usage
```bash
nvidia-smi | grep swictation
# Should show ~6GB usage for 1.1B model
```

### ✓ UI Service with Compositor Detection
```bash
systemctl --user status swictation-ui | grep "Sway"
# Should show: Sway compositor detected
```

## Expected Log Output When Working

```
✓ NVIDIA GPU detected!
📦 Downloading GPU acceleration libraries...
  ✓ Downloaded GPU libraries
✓ Found ONNX Runtime (GPU-enabled): /usr/local/lib/node_modules/swictation/lib/native/libonnxruntime.so
  Using bundled GPU-enabled library with CUDA provider support
⚙️  Generating systemd service files...
✓ Generated daemon service: /home/user/.config/systemd/user/swictation-daemon.service
✓ Installed UI service: /home/user/.config/systemd/user/swictation-ui.service

📊 System Detection:
   GPU detected: NVIDIA RTX PRO 6000 Blackwell Workstation Edition (96GB VRAM)

🎯 Recommended Model:
   1.1B - Best quality - Full GPU acceleration with FP32 precision
   Size: ~75MB download (FP32 + INT8 versions)
   Performance: 62x realtime speed on GPU
```

Daemon logs:
```
Successfully registered `CUDAExecutionProvider`
cuDNN version: 91500
Using FP32 model for GPU: encoder.onnx
Using FP32 model for GPU: decoder.onnx
Using FP32 model for GPU: joiner.onnx
✓ Parakeet-TDT-1.1B-INT8 loaded successfully (GPU, forced)
GPU memory monitoring enabled: NVIDIA RTX PRO 6000 Blackwell Workstation Edition
```

## Files Changed/Created

### Modified Files
1. `/opt/swictation/npm-package/postinstall.js` - Detection and installation logic
2. `/opt/swictation/scripts/build-npm-package.sh` - Build automation
3. `/opt/swictation/npm-package/README.md` - Installation instructions
4. `/opt/swictation/config/swictation-daemon.service` - CUDA environment
5. `/opt/swictation/npm-package/config/swictation-daemon.service` - CUDA environment
6. `/opt/swictation/npm-package/templates/swictation-daemon.service.template` - CUDA environment

### New Documentation
1. `/opt/swictation/docs/NPM_POSTINSTALL_ISSUE.md` - Postinstall problem documentation
2. `/opt/swictation/docs/NPM_INSTALL_COMPLETE_GUIDE.md` - This file
3. `/opt/swictation/docs/GPU_ENVIRONMENT_FIX.md` - Original GPU fix documentation

## Future Improvements

1. **Auto-run postinstall** - Research npm/sudo interaction to make it automatic
2. **Setup command** - `swictation setup` could call postinstall automatically
3. **Better error detection** - Detect when services missing and show clear instructions
4. **GitHub release automation** - Automate CUDA library tarball creation

## Testing Process

To test a fresh install:
```bash
# 1. Stop and uninstall current version
systemctl --user stop swictation-daemon swictation-ui
sudo npm uninstall -g swictation
rm -f ~/.config/systemd/user/swictation*.service

# 2. Install from tarball
sudo npm install -g /opt/swictation/npm-package/swictation-0.3.0.tgz

# 3. Run postinstall
cd /usr/local/lib/node_modules/swictation
node postinstall.js

# 4. Verify (run all checks above)
```

## Success Criteria

✅ All verification checks pass  
✅ GPU memory usage ~6GB  
✅ cuDNN version 91500 shown 4 times  
✅ FP32 models loading for all components  
✅ Both services active  
✅ UI detects compositor  
✅ No "cannot open shared object file" errors  
✅ No "CUDA execution provider is not enabled" errors  

## Last Updated

2025-11-12 - After complete end-to-end verification and postinstall discovery

---

**All fixes are now in place. The package is ready for publishing with proper documentation.**
