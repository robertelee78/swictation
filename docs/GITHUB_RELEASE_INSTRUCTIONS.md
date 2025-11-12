# GitHub Release Instructions for Swictation

## Creating a New Release

When creating a new release (e.g., v0.3.0), you MUST upload the GPU libraries tarball.

### 1. Build the NPM Package

```bash
cd /opt/swictation
./scripts/build-npm-package.sh
```

This creates `npm-package/swictation-X.Y.Z.tgz`

### 2. Create GPU Libraries Tarball

```bash
cd /opt/swictation
tar -czf swictation-gpu-libs.tar.gz \
  -C rust-crates/target/release \
  libonnxruntime_providers_cuda.so \
  libonnxruntime_providers_shared.so \
  libonnxruntime_providers_tensorrt.so
```

**Files included:**
- `libonnxruntime_providers_cuda.so` (~330MB) - CUDA execution provider
- `libonnxruntime_providers_shared.so` (~15KB) - Shared provider base
- `libonnxruntime_providers_tensorrt.so` (~787KB) - TensorRT provider

### 3. Verify Tarball

```bash
# Check size
ls -lh swictation-gpu-libs.tar.gz

# Verify contents
tar -tzf swictation-gpu-libs.tar.gz

# Test extraction
mkdir /tmp/test && cd /tmp/test
tar -xzf /opt/swictation/swictation-gpu-libs.tar.gz
ls -lh
rm -rf /tmp/test
```

### 4. Create GitHub Release

1. Go to: https://github.com/robertelee78/swictation/releases/new

2. **Tag**: v0.3.0 (must match package.json version)

3. **Title**: Swictation v0.3.0

4. **Description**:
   ```markdown
   ## What's New in v0.3.0

   ### Critical Fixes
   - ✅ Fixed GPU acceleration (FP32 model selection)
   - ✅ Fixed cuDNN 9 loading (cuda-12.9 path)
   - ✅ Fixed service file generation with correct paths
   - ✅ Fixed postinstall ONNX Runtime detection
   - ✅ Added UI service installation

   ### Performance
   - GPU memory usage: ~6GB for 1.1B model
   - Performance: 62x realtime speed on GPU
   - cuDNN 9.1.500 support

   ### Installation

   ```bash
   # Step 1: Install package
   sudo npm install -g swictation

   # Step 2: Run post-install setup (REQUIRED!)
   cd /usr/local/lib/node_modules/swictation
   node postinstall.js
   ```

   **Important**: The post-install script downloads GPU libraries (~330MB) from this release.

   ### Verification

   After installation, verify GPU is working:

   ```bash
   systemctl --user restart swictation-daemon
   journalctl --user -u swictation-daemon --since "1 min ago" | grep "Successfully registered.*CUDA"
   journalctl --user -u swictation-daemon --since "1 min ago" | grep "cuDNN"
   journalctl --user -u swictation-daemon --since "1 min ago" | grep "Using FP32"
   nvidia-smi | grep swictation
   ```

   ### Documentation

   - [Complete Installation Guide](docs/NPM_INSTALL_COMPLETE_GUIDE.md)
   - [GPU Environment Fix](docs/GPU_ENVIRONMENT_FIX.md)
   - [Postinstall Issue Documentation](docs/NPM_POSTINSTALL_ISSUE.md)

   ### Known Issues

   - `npm install -g` with sudo does not automatically run postinstall (documented workaround above)
   ```

5. **Upload Assets**:
   - ✅ `swictation-gpu-libs.tar.gz` (REQUIRED - ~331MB)
   - Optional: `swictation-0.3.0.tgz` (npm package for reference)

6. **Pre-release**: ☐ No (unless testing)

7. **Publish Release**: Click "Publish release"

### 5. Verify Release Assets

After publishing, verify the download URL works:

```bash
wget https://github.com/robertelee78/swictation/releases/download/v0.3.0/swictation-gpu-libs.tar.gz
tar -tzf swictation-gpu-libs.tar.gz
rm swictation-gpu-libs.tar.gz
```

### 6. Test Fresh Install

On a clean system or after uninstalling:

```bash
# Uninstall current
sudo npm uninstall -g swictation
rm -rf ~/.config/systemd/user/swictation*.service

# Install new version
sudo npm install -g swictation

# Run postinstall
cd /usr/local/lib/node_modules/swictation
node postinstall.js

# Should show:
# ✓ NVIDIA GPU detected!
# 📦 Downloading GPU acceleration libraries...
# ✓ Downloaded GPU libraries
# ✓ Found ONNX Runtime (GPU-enabled)
# ✓ Generated daemon service
# ✓ Installed UI service

# Verify GPU working
systemctl --user start swictation-daemon
journalctl --user -u swictation-daemon --since "1 min ago" | grep CUDA
```

## Why the GPU Libraries are Separate

**npm package size limit**: npm recommends packages under 50MB. Our GPU libraries are 331MB.

**Solution**:
1. Base package (~17MB) contains everything except GPU libraries
2. GPU libraries tarball uploaded to GitHub releases
3. postinstall.js automatically downloads tarball during installation

## postinstall.js Download Logic

```javascript
const version = require('./package.json').version;
const releaseUrl = `https://github.com/robertelee78/swictation/releases/download/v${version}/swictation-gpu-libs.tar.gz`;
```

**Critical**: The release tag MUST match package.json version!

## Troubleshooting

### Download Fails

If postinstall can't download the GPU libraries:

```bash
# Manual download
cd /usr/local/lib/node_modules/swictation/lib/native
wget https://github.com/robertelee78/swictation/releases/download/v0.3.0/swictation-gpu-libs.tar.gz
tar -xzf swictation-gpu-libs.tar.gz
rm swictation-gpu-libs.tar.gz
```

### Wrong Version

If postinstall downloads from wrong version:
1. Check package.json version matches release tag
2. Ensure release exists and has swictation-gpu-libs.tar.gz asset

### File Already Exists Error

If tar extraction fails with "File exists":
1. Use `--overwrite` flag in extraction (already in postinstall.js)
2. Or manually remove old files first

## Checklist for New Release

- [ ] Update package.json version
- [ ] Run `./scripts/build-npm-package.sh`
- [ ] Create `swictation-gpu-libs.tar.gz`
- [ ] Verify tarball contents
- [ ] Create GitHub release with matching tag
- [ ] Upload GPU libraries tarball
- [ ] Test fresh install
- [ ] Verify GPU working
- [ ] Publish npm package (optional)

## Related Files

- `/opt/swictation/npm-package/postinstall.js` - Downloads GPU libraries
- `/opt/swictation/scripts/build-npm-package.sh` - Build automation
- `/opt/swictation/docs/NPM_INSTALL_COMPLETE_GUIDE.md` - Installation guide
