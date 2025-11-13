# Building and Releasing Swictation v0.3.1

This guide covers building, testing, and publishing the v0.3.1 npm package.

---

## 🎯 What's New in v0.3.1

1. **Real model test-loading during installation** (~30-60s)
2. **Old service cleanup** on upgrades (prevents conflicts)
3. **Interactive config migration** (pacman/apt-style prompts)
4. **Intelligent VRAM-based selection** with verification
5. **Fixed model thresholds** (empirically validated)
6. **Robust error handling** (graceful fallback)

---

## 📋 Prerequisites

### System Requirements
- **Linux** with Sway/Wayland compositor (Ubuntu 24.04+ recommended)
- **NVIDIA GPU** with 4GB+ VRAM (RTX A1000/3050/4060 or better)
- **Node.js** 18+
- **npm** 8+
- **Rust** toolchain (stable)
- **CUDA** 11.8+ with nvidia-smi

### Build Tools
```bash
# Ubuntu/Debian
sudo apt install build-essential pkg-config wtype wl-clipboard pipewire nvidia-cuda-toolkit

# Arch/Manjaro
sudo pacman -S base-devel wtype wl-clipboard pipewire cuda
```

---

## 🔨 Building from Source

### 1. Clone Repository
```bash
git clone --recurse-submodules https://github.com/robertelee78/swictation.git
cd swictation
```

### 2. Build Rust Daemon
```bash
cd rust-crates
cargo build --release

# Verify binary
ls -lh target/release/swictation-daemon
file target/release/swictation-daemon
```

### 3. Build Tauri UI
```bash
cd ../tauri-ui
npm install
npm run tauri build

# Verify binary
ls -lh src-tauri/target/release/swictation-ui
```

### 4. Prepare npm Package
```bash
cd ../npm-package

# Copy binaries
mkdir -p bin lib/native
cp ../rust-crates/target/release/swictation-daemon bin/
cp ../rust-crates/target/release/swictation-daemon lib/native/swictation-daemon.bin
cp ../tauri-ui/src-tauri/target/release/swictation-ui bin/

# Copy GPU libraries (if building on GPU system)
mkdir -p lib/native
cp /usr/lib/x86_64-linux-gnu/libonnxruntime.so* lib/native/

# Set permissions
chmod +x bin/*
chmod +x lib/native/*
```

---

## 🧪 Testing Locally

### 1. Test Installation (Local npm Package)
```bash
cd /opt/swictation/npm-package

# Create test tarball
npm pack

# Install locally (in a test directory)
cd /tmp
mkdir swictation-test && cd swictation-test
npm install /opt/swictation/npm-package/swictation-0.3.1.tgz

# This will run postinstall.js which:
# - Detects GPU and VRAM
# - Recommends optimal model
# - Downloads GPU libraries
# - Attempts to test-load model (~30-60s)
# - Sets up systemd services
# - Migrates config files
```

### 2. Verify Installation
```bash
# Check binaries
ls -lh /tmp/swictation-test/node_modules/swictation/bin/
file /tmp/swictation-test/node_modules/swictation/bin/swictation-daemon

# Check GPU detection
cat ~/.config/swictation/gpu-info.json

# Check service files
ls -l ~/.config/systemd/user/swictation-*.service

# Try starting daemon
systemctl --user daemon-reload
systemctl --user start swictation-daemon
systemctl --user status swictation-daemon

# Check logs
journalctl --user -u swictation-daemon -n 50
```

### 3. Test Scenarios

#### Scenario A: Fresh Install (No Previous Installation)
```bash
# Expected behavior:
# - GPU detected and VRAM measured
# - Model recommended based on VRAM
# - Model test-loaded successfully (~30-60s)
# - Services installed
# - Config created from template
```

#### Scenario B: Upgrade from v0.3.0
```bash
# Expected behavior:
# - Old service files detected and cleaned up
# - Config migration prompt (or "Keep" in non-interactive)
# - GPU re-detected
# - Model re-tested with new thresholds
# - Services reinstalled
```

#### Scenario C: Limited VRAM GPU (4GB)
```bash
# Expected behavior:
# - GPU detected: RTX A1000 (4GB)
# - Recommends 0.6B model (not 1.1B)
# - Test-loads 0.6B successfully
# - Service configured with 0.6B
```

#### Scenario D: CI/Headless Environment
```bash
SKIP_MODEL_TEST=1 npm install -g swictation

# Expected behavior:
# - No interactive prompts
# - No model test-loading
# - Services installed
# - Config defaulted to "Keep"
```

### 4. Test Cleanup
```bash
# Remove test installation
rm -rf /tmp/swictation-test

# Remove services (if installed)
systemctl --user stop swictation-daemon swictation-ui
systemctl --user disable swictation-daemon swictation-ui
rm -f ~/.config/systemd/user/swictation-*.service
systemctl --user daemon-reload

# Optional: Remove config
rm -rf ~/.config/swictation
```

---

## 📦 Building npm Package

### 1. Verify Version
```bash
cd /opt/swictation/npm-package

# Check package.json
grep '"version"' package.json
# Should show: "version": "0.3.1"

# Update if needed
npm version 0.3.1 --no-git-tag-version
```

### 2. Bundle GPU Libraries
```bash
# Create GPU libraries tarball (do this on a GPU system with CUDA installed)
cd /opt/swictation
mkdir -p tmp/gpu-libs

# Copy ONNX Runtime with CUDA support
cp /usr/lib/x86_64-linux-gnu/libonnxruntime.so* tmp/gpu-libs/
cp /usr/local/cuda-12.9/lib64/libcudnn*.so* tmp/gpu-libs/ || true

# Create tarball
cd tmp
tar -czf swictation-gpu-libs.tar.gz gpu-libs/
mv swictation-gpu-libs.tar.gz ../npm-package/lib/native/

# Verify
ls -lh ../npm-package/lib/native/swictation-gpu-libs.tar.gz
```

### 3. Create Package Tarball
```bash
cd /opt/swictation/npm-package

# Clean previous builds
rm -f swictation-*.tgz

# Create tarball
npm pack

# Verify contents
tar -tzf swictation-0.3.1.tgz | head -20

# Check size
ls -lh swictation-0.3.1.tgz
# Should be ~50-100MB with GPU libraries
```

---

## 🚀 Publishing to npm

### 1. Pre-publish Checklist
- [ ] Version updated in package.json (0.3.1)
- [ ] CHANGELOG.md updated with v0.3.1 changes
- [ ] README.md reflects new behavior
- [ ] All binaries built and included
- [ ] GPU libraries bundled (if applicable)
- [ ] Local testing completed successfully
- [ ] Test scenarios verified (fresh, upgrade, limited VRAM, CI)
- [ ] Git committed and pushed

### 2. Publish to npm
```bash
cd /opt/swictation/npm-package

# Login to npm (if not already)
npm login

# Publish (dry-run first)
npm publish --dry-run

# Verify output, then publish for real
npm publish

# Verify publication
npm view swictation version
npm view swictation
```

### 3. Create GitHub Release
```bash
cd /opt/swictation

# Tag release
git tag -a v0.3.1 -m "Release v0.3.1: Real model test-loading, service cleanup, config migration"
git push origin v0.3.1

# Create release on GitHub
gh release create v0.3.1 \
  --title "v0.3.1 - Intelligent Installation with Model Verification" \
  --notes-file docs/RELEASE_NOTES_v0.3.1.md \
  npm-package/swictation-0.3.1.tgz \
  npm-package/lib/native/swictation-gpu-libs.tar.gz
```

---

## 🐛 Troubleshooting Build Issues

### Binary Size Too Large
```bash
# Strip debug symbols
cd rust-crates
cargo build --release
strip target/release/swictation-daemon

# Verify size reduction
ls -lh target/release/swictation-daemon
```

### Missing GPU Libraries
```bash
# Check ONNX Runtime installation
python3 -c "import onnxruntime; print(onnxruntime.__file__)"

# Find library
find /usr -name "libonnxruntime.so*" 2>/dev/null

# Copy to package
cp <found-path> npm-package/lib/native/
```

### CUDA Libraries Not Found
```bash
# Check CUDA installation
nvidia-smi

# Find CUDA libraries
find /usr/local/cuda* -name "libcudnn*.so*" 2>/dev/null

# Set LD_LIBRARY_PATH for build
export LD_LIBRARY_PATH=/usr/local/cuda-12.9/lib64:$LD_LIBRARY_PATH
```

### Test-Loading Fails During Build Testing
```bash
# Skip test-loading for build testing
SKIP_MODEL_TEST=1 npm install -g swictation

# Or test manually
/path/to/swictation-daemon --test-model=0.6b-gpu --dry-run
```

---

## 📝 Build Artifacts

After successful build, you should have:

```
npm-package/
├── bin/
│   ├── swictation              # CLI wrapper script
│   ├── swictation-daemon       # Rust daemon (release)
│   └── swictation-ui           # Tauri UI (release)
├── lib/
│   └── native/
│       ├── swictation-daemon.bin       # Daemon binary (copy)
│       ├── libonnxruntime.so*          # ONNX Runtime (GPU)
│       └── swictation-gpu-libs.tar.gz  # GPU libraries bundle
├── config/
│   ├── config.toml             # Default config template
│   ├── swictation-ui.service   # UI systemd service
│   └── detected-environment.json
├── templates/
│   └── swictation-daemon.service.template
├── package.json                # Version 0.3.1
├── postinstall.js              # Installation script
└── swictation-0.3.1.tgz        # Final npm package
```

---

## 📊 Package Statistics

Expected sizes:
- **swictation-daemon**: ~15-25MB (stripped)
- **swictation-ui**: ~10-20MB (Tauri binary)
- **libonnxruntime.so**: ~30-50MB (GPU-enabled)
- **GPU libraries tarball**: ~50-100MB (CUDA + cuDNN)
- **Final npm package**: ~100-200MB

---

## 🔄 Continuous Integration

### GitHub Actions Workflow (Suggested)
```yaml
name: Build and Test

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v4
        with:
          submodules: recursive

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Build daemon
        run: |
          cd rust-crates
          cargo build --release

      - name: Build UI
        run: |
          cd tauri-ui
          npm install
          npm run tauri build

      - name: Package npm
        run: |
          cd npm-package
          npm pack

      - name: Upload artifact
        uses: actions/upload-artifact@v3
        with:
          name: swictation-package
          path: npm-package/swictation-*.tgz
```

---

## 📞 Support

For build issues:
- Check [Issues](https://github.com/robertelee78/swictation/issues)
- Review [TROUBLESHOOTING.md](/opt/swictation/docs/TROUBLESHOOTING.md)
- Ask in [Discussions](https://github.com/robertelee78/swictation/discussions)

---

**Status**: Ready for v0.3.1 Release
**Last Updated**: 2025-11-13
