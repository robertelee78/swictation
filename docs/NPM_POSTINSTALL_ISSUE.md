# NPM Postinstall Issue - Critical Documentation

## Problem

When installing globally with `sudo npm install -g swictation`, npm **does not automatically run** the postinstall.js script. This causes:

1. ❌ Service files not installed to `~/.config/systemd/user/`
2. ❌ CUDA provider libraries not downloaded (330M `libonnxruntime_providers_cuda.so`)
3. ❌ GPU functionality completely broken

## Root Cause

This is a known npm behavior when using `sudo` with global installs. The postinstall script runs as root but the service files need to be in the user's home directory.

## Solution: Manual Postinstall

After installing, users MUST run postinstall manually:

```bash
# Install package
sudo npm install -g swictation

# Run postinstall manually (REQUIRED!)
cd /usr/local/lib/node_modules/swictation
node postinstall.js
```

## What postinstall.js Does

1. **Detects NVIDIA GPU** - Checks for nvidia-smi
2. **Downloads CUDA libraries** (330M) from GitHub releases
   - `libonnxruntime_providers_cuda.so`
   - `libonnxruntime_providers_shared.so`
   - `libonnxruntime_providers_tensorrt.so`
3. **Detects ONNX Runtime** - Prefers bundled GPU library over Python pip
4. **Generates service files**:
   - `~/.config/systemd/user/swictation-daemon.service`
   - `~/.config/systemd/user/swictation-ui.service`
5. **Sets correct paths** - Replaces template placeholders

## Verification After Install

```bash
# 1. Check service files exist
ls -la ~/.config/systemd/user/swictation*.service

# 2. Check CUDA provider exists
ls -lh /usr/local/lib/node_modules/swictation/lib/native/*cuda*

# 3. Check service has correct ORT path
grep "ORT_DYLIB_PATH" ~/.config/systemd/user/swictation-daemon.service
# Should show: /usr/local/lib/node_modules/swictation/lib/native/libonnxruntime.so

# 4. Check CUDA environment
grep -E "cuda-12.9|CUDA_HOME" ~/.config/systemd/user/swictation-daemon.service

# 5. Start and verify
systemctl --user daemon-reload
systemctl --user start swictation-daemon swictation-ui

# 6. Check GPU working
journalctl --user -u swictation-daemon --since "1 min ago" | grep "Successfully registered.*CUDA"
journalctl --user -u swictation-daemon --since "1 min ago" | grep "cuDNN version"
journalctl --user -u swictation-daemon --since "1 min ago" | grep "Using FP32 model"

# 7. Check GPU memory usage
nvidia-smi | grep swictation
```

## Expected Output When Working

```
✓ Found ONNX Runtime (GPU-enabled): /usr/local/lib/node_modules/swictation/lib/native/libonnxruntime.so
  Using bundled GPU-enabled library with CUDA provider support
✓ Generated daemon service: /home/user/.config/systemd/user/swictation-daemon.service
✓ Installed UI service: /home/user/.config/systemd/user/swictation-ui.service
```

Logs should show:
```
Successfully registered `CUDAExecutionProvider`
cuDNN version: 91500
Using FP32 model for GPU: encoder.onnx
Using FP32 model for GPU: decoder.onnx
Using FP32 model for GPU: joiner.onnx
GPU memory monitoring enabled: NVIDIA RTX PRO 6000
```

GPU memory usage should be ~6GB for 1.1B model.

## Alternative: Include in README

Add to package README.md:

```markdown
## Post-Installation Setup (REQUIRED)

After installing, you MUST run the post-install setup:

\`\`\`bash
# Run postinstall to configure services and download GPU libraries
cd /usr/local/lib/node_modules/swictation
node postinstall.js
\`\`\`

This downloads CUDA libraries (~330MB) and sets up systemd services.
```

## Future Fix Options

1. **Make postinstall run automatically** - Research npm/systemd interaction
2. **Create setup command** - `swictation setup` calls postinstall
3. **Better error messaging** - Detect when postinstall didn't run and show clear instructions
4. **Wrapper script** - Create install script that handles both steps

## Related Files

- `/opt/swictation/npm-package/postinstall.js` - The critical setup script
- `/opt/swictation/scripts/build-npm-package.sh` - Build and verification script
- `/opt/swictation/docs/NPM_PACKAGE_CHECKLIST.md` - Complete build checklist

## Last Updated

2025-11-12 - After discovering postinstall doesn't auto-run with sudo npm install -g
