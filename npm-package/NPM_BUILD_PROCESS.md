# Swictation npm Build & Release Process

## Overview

This document defines the complete build and release process for publishing swictation to npm registry.

## Build Process

### Phase 1: Upstream Preparation

#### 1.1 Rust Binary Build
```bash
cd /opt/swictation/rust-crates/swictation-daemon
cargo build --release --target x86_64-unknown-linux-gnu

# Output: target/x86_64-unknown-linux-gnu/release/swictation-daemon
# Size: ~8.6 MB
# Copy to: npm-package/lib/native/swictation-daemon.bin
```

#### 1.2 Native Libraries Collection
Collect these libraries into `npm-package/lib/native/`:

**Bundled in npm package** (35 MB total):
- `libonnxruntime.so` (22 MB) - ONNX Runtime base
- `libsherpa-onnx-c-api.so` (3.8 MB) - Speech recognition C API
- `libsherpa-onnx-cxx-api.so` (84 KB) - Speech recognition C++ API
- `libonnxruntime_providers_shared.so` (15 KB) - Shared provider code

**Separate GPU libs tarball** (209 MB):
```bash
cd /opt/swictation/npm-package
tar -czf swictation-gpu-libs.tar.gz \
  libonnxruntime_providers_cuda.so \
  libonnxruntime_providers_tensorrt.so \
  libonnxruntime_providers_shared.so

# Upload to GitHub release: v{version}/swictation-gpu-libs.tar.gz
```

#### 1.3 Tauri UI Build
```bash
cd /opt/swictation/src/tauri-ui
npm install
npm run build

# Output: src-tauri/target/release/swictation-ui
# Size: ~15 MB
# Copy to: npm-package/bin/swictation-ui
```

#### 1.4 Python Scripts
Copy to `npm-package/src/ui/`:
- `swictation_tray.py` - System tray icon
- Any other UI scripts

### Phase 2: npm Package Preparation

#### 2.1 Update Version
```bash
cd /opt/swictation/npm-package
# Edit package.json version field
npm version patch  # or minor, major
```

#### 2.2 Verify package.json Files Array
```json
{
  "files": [
    "bin/",           // CLI wrappers + UI binary
    "lib/",           // Native binaries and libraries
    "config/",        // Config templates
    "src/",           // Python/JS source
    "docs/",          // Documentation
    "templates/",     // Service file templates
    "postinstall.js", // Setup script
    "README.md"
  ]
}
```

#### 2.3 Verify .npmignore
```
# Build files
build.sh
*.tar.gz (EXCEPT swictation-gpu-libs.tar.gz in releases)
*.tgz

# Development files
.git/
tests/ (keep for users)
*.test.js

# Documentation source (keep minimal docs)
docs/development/

# IDE files
.vscode/
```

### Phase 3: Testing

#### 3.1 Local Build Test
```bash
npm pack
# Creates: swictation-{version}.tgz
# Verify size: ~35 MB (without GPU libs)

# Inspect contents
tar -tzf swictation-{version}.tgz | less

# Check for lib/native files
tar -tzf swictation-{version}.tgz | grep "lib/native"
```

#### 3.2 Docker Tests
```bash
# Quick smoke test
./tests/docker-quick-test.sh

# Full matrix (Node 18, 20, 22)
./tests/docker-test.sh

# Postinstall debug
./tests/docker-postinstall-debug.sh

# Full with dependencies
./tests/docker-full-install-test.sh
```

**All tests must PASS before continuing**.

#### 3.3 Local System Test
```bash
# Test on development system
sudo npm install -g ./swictation-{version}.tgz --foreground-scripts

# Verify installation
swictation help
ls /usr/local/lib/node_modules/swictation/lib/native/

# Test daemon
cd /usr/local/lib/node_modules/swictation/lib/native
export LD_LIBRARY_PATH="$(pwd):$LD_LIBRARY_PATH"
./swictation-daemon.bin --help

# Cleanup
sudo npm uninstall -g swictation
```

### Phase 4: GitHub Release Creation

**CRITICAL**: Must be done BEFORE npm publish!

#### 4.1 Create Git Tag
```bash
git tag v{version}
git push origin v{version}
```

#### 4.2 Create GitHub Release
1. Go to https://github.com/robertelee78/swictation/releases/new
2. Tag: `v{version}` (e.g., v0.3.1)
3. Title: `Swictation v{version}`
4. Description: Release notes
5. Upload assets:
   - `swictation-gpu-libs.tar.gz` (209 MB)
6. Publish release

#### 4.3 Verify GPU Libs Download
```bash
VERSION="0.3.1"
curl -I "https://github.com/robertelee78/swictation/releases/download/v${VERSION}/swictation-gpu-libs.tar.gz"
# Should return: HTTP/2 200 (not 404)
```

### Phase 5: npm Publish

#### 5.1 npm Login
```bash
npm login
# Enter credentials for npm registry
```

#### 5.2 Publish to npm
```bash
npm publish
# Package will be available at: https://www.npmjs.com/package/swictation
```

#### 5.3 Verify npm Package
```bash
# Wait 1-2 minutes for propagation
npm view swictation

# Check version
npm view swictation version

# Check files included
npm view swictation dist.tarball
```

### Phase 6: Production Testing

#### 6.1 Test Fresh Install from npm
```bash
# On clean system (192.168.1.133)
sudo npm install -g swictation --foreground-scripts

# Verify GPU libs downloaded
ls /usr/local/lib/node_modules/swictation/lib/native/libonnxruntime_providers_cuda.so

# Download models
pip install "huggingface_hub[cli]"
swictation download-model 1.1b

# Setup
swictation setup

# Start service
swictation start

# Test dictation
swictation toggle
```

## Pre-Publish Checklist

- [ ] Rust daemon binary built and copied
- [ ] Native libraries collected
- [ ] Tauri UI binary built and copied
- [ ] Python scripts copied
- [ ] package.json version updated
- [ ] Git committed and pushed
- [ ] Docker tests PASSED
- [ ] Local install test PASSED
- [ ] Git tag created and pushed
- [ ] GitHub release created with GPU libs
- [ ] GPU libs download URL verified (200, not 404)
- [ ] Ready to `npm publish`

## Post-Publish Checklist

- [ ] npm package visible on registry
- [ ] Fresh install test on production system
- [ ] GPU acceleration working
- [ ] Model downloads working
- [ ] Service starts successfully
- [ ] Dictation functional
- [ ] Update documentation
- [ ] Announce release

## Troubleshooting

### Issue: "Package size too large"
- npm has 10 MB tarball limit for unpaid accounts
- Our package is ~35 MB, requires npm Plus/Pro account
- Or split into multiple packages

### Issue: "GPU libs 404 during postinstall"
- GitHub release wasn't created before npm publish
- **FIX**: Create GitHub release first!

### Issue: "Libraries not found after install"
- Check `files` array in package.json includes `lib/`
- Verify .npmignore doesn't exclude .so files
- Test with `npm pack` and inspect tarball

### Issue: "Postinstall doesn't run"
- Check for `--ignore-scripts` flag
- Verify `postinstall` script in package.json
- Use `--foreground-scripts` to see output

### Issue: "Daemon fails with missing libraries"
- System needs libasound2t64 (Ubuntu 24.04+)
- Install: `sudo apt-get install libasound2t64`
- Update README with system requirements

## Rollback Procedure

If critical issue discovered after publish:

```bash
# 1. Unpublish bad version (within 72 hours)
npm unpublish swictation@{version}

# 2. Or deprecate
npm deprecate swictation@{version} "Critical bug, use v{fixed-version} instead"

# 3. Publish fixed version
npm version patch
npm publish
```

## Version Numbering

- **Major** (1.0.0): Breaking changes, incompatible API
- **Minor** (0.3.0): New features, backwards compatible
- **Patch** (0.3.1): Bug fixes, backwards compatible

Current: **0.3.1** - Patch release with installation fixes
