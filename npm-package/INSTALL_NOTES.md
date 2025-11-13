# Swictation v0.3.1 Installation Notes

## ✅ What Works

1. **Libraries are bundled** - All native libraries (.so files) and binaries are correctly included in the npm package
2. **Daemon execution** - The daemon binary runs correctly when system dependencies are met
3. **Library path resolution** - LD_LIBRARY_PATH is correctly set in postinstall and service files
4. **CPU-only mode** - Works on systems without NVIDIA GPU

## System Requirements

### Required Dependencies
- **Node.js** >= 18.0.0
- **libasound2t64** (Ubuntu 24.04+) or **libasound2** (older systems)
  ```bash
  # Ubuntu 24.04+
  sudo apt-get install libasound2t64

  # Ubuntu 22.04 and older
  sudo apt-get install libasound2
  ```

### Optional Dependencies (for full functionality)
- **NVIDIA GPU** + **CUDA 12.x** (for GPU acceleration)
- **systemd** (for background service)
- **wtype** (Wayland text injection)
- **xdotool** (X11 text injection)
- **huggingface_hub[cli]** (model downloads)
  ```bash
  pip install "huggingface_hub[cli]"
  ```

## GPU Acceleration

GPU acceleration libraries (209MB) are downloaded separately during postinstall:
- `libonnxruntime_providers_cuda.so` (~200MB)
- `libonnxruntime_providers_tensorrt.so`
- `libonnxruntime_providers_shared.so`

These are downloaded from:
```
https://github.com/robertelee78/swictation/releases/download/v{version}/swictation-gpu-libs.tar.gz
```

**Note**: The v0.3.1 release must be created on GitHub before publishing to npm.

## Installation Flow

1. **Pre-flight checks** - Platform, permissions, directories
2. **Service cleanup** - Remove old/conflicting service files
3. **Config migration** - Interactive config update (if existing config found)
4. **GPU detection** - Detect NVIDIA GPU and VRAM
5. **GPU libs download** - Download CUDA providers (if GPU detected)
6. **Model testing** - Test-load models to verify they work (if GPU detected)
7. **Service generation** - Create systemd service files with correct paths

## Known Issues & Workarounds

### Issue 1: Postinstall output hidden by default
**Symptom**: No output visible during `npm install -g`
**Workaround**: Use `--foreground-scripts` flag:
```bash
sudo npm install -g swictation --foreground-scripts
```

**Fix needed**: Make output visible by default (console.log vs log())

### Issue 2: GPU libs download fails (v0.3.1)
**Symptom**: `gzip: stdin: not in gzip format` error during postinstall
**Root cause**: v0.3.1 GitHub release doesn't exist yet (404)
**Fix**: Create GitHub release before publishing to npm

### Issue 3: Model test-loading on dev box
**Symptom**: `libsherpa-onnx-c-api.so: cannot open shared object file`
**Root cause**: LD_LIBRARY_PATH not set in test environment
**Status**: Fixed - Docker tests confirm library path is correct

## Testing Results

### Docker Test Results (Ubuntu 24.04 + Node 20)
- ✅ Package installs successfully
- ✅ All libraries bundled correctly
- ✅ Daemon binary executable
- ✅ CLI works
- ✅ All dependencies satisfied (with libasound2t64)
- ✅ Daemon runs with --help flag

### Test Commands
```bash
# Quick smoke test
./tests/docker-quick-test.sh

# Full matrix test (Node 18, 20, 22)
./tests/docker-test.sh

# Debug postinstall
./tests/docker-postinstall-debug.sh

# Full install with dependencies
./tests/docker-full-install-test.sh
```

## Next Steps Before npm Publish

1. **Create GitHub v0.3.1 release**
   - Upload `swictation-gpu-libs.tar.gz` (209MB)
   - Tag: `v0.3.1`

2. **Make postinstall output visible**
   - Change `log()` calls to use `console.log()` for important messages
   - Or document `--foreground-scripts` flag requirement

3. **Test on target system (192.168.1.133)**
   - Install from real npm registry
   - Verify GPU acceleration works
   - Test model test-loading with NVIDIA GPU

4. **Document system requirements** in README
   - libasound2t64 requirement
   - CUDA 12.x for GPU support
   - Wayland/X11 for text injection

## Installation Verification

After installation, verify:
```bash
# 1. CLI works
swictation help

# 2. Daemon binary exists
ls /usr/local/lib/node_modules/swictation/lib/native/swictation-daemon.bin

# 3. Libraries bundled
ls /usr/local/lib/node_modules/swictation/lib/native/*.so

# 4. Daemon can run
cd /usr/local/lib/node_modules/swictation/lib/native
export LD_LIBRARY_PATH="$(pwd):$LD_LIBRARY_PATH"
./swictation-daemon.bin --help

# 5. Service files created
ls ~/.config/systemd/user/swictation-*
```

## Files Included in npm Package

Total: 22 files (35.4 MB)
- `bin/` - CLI wrappers
- `lib/native/` - Binaries and libraries (35 MB)
  - `swictation-daemon.bin` (8.6 MB)
  - `libonnxruntime.so` (22 MB)
  - `libsherpa-onnx-c-api.so` (3.8 MB)
  - `libsherpa-onnx-cxx-api.so` (84 KB)
  - `libonnxruntime_providers_shared.so` (15 KB)
- `src/` - UI scripts
- `config/` - Config templates
- `templates/` - Service templates
- `postinstall.js` - Setup script
- `README.md`, `package.json`

## Excluded (Too large for npm)
- `swictation-gpu-libs.tar.gz` (209 MB) - Downloaded during postinstall
