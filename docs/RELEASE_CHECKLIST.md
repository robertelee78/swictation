# Swictation Release Checklist

This checklist ensures that both Linux and macOS platforms are properly supported in every release.

## Pre-Release Build

### Linux (x86_64-unknown-linux-gnu)
- [ ] Build Rust binaries: `npm run build:linux` or `cargo build --release --target x86_64-unknown-linux-gnu`
- [ ] Verify binary: `file rust-crates/target/x86_64-unknown-linux-gnu/release/swictation-daemon`
- [ ] Test binary runs: `./rust-crates/target/x86_64-unknown-linux-gnu/release/swictation-daemon --help`
- [ ] Copy to npm package:
  - `cp rust-crates/target/x86_64-unknown-linux-gnu/release/swictation-daemon npm-package/bin/swictation-daemon`
  - `cp rust-crates/target/x86_64-unknown-linux-gnu/release/swictation-daemon npm-package/lib/native/swictation-daemon.bin`

### macOS (aarch64-apple-darwin)
- [ ] Build Rust binaries: `npm run build:macos` or `./npm-package/scripts/build-macos-release.sh`
- [ ] Verify binary: `file npm-package/bin/swictation-daemon-macos`
- [ ] Test binary runs: `./npm-package/bin/swictation-daemon-macos --help`
- [ ] Verify CoreML support: `otool -L npm-package/bin/swictation-daemon-macos | grep onnxruntime`

## ONNX Runtime Libraries

### Linux GPU Libraries
- [ ] Verify CUDA libraries in `npm-package/lib/native/`:
  - `libonnxruntime.so`
  - `libonnxruntime_providers_cuda.so`
  - `libonnxruntime_providers_shared.so`
  - `libonnxruntime_providers_tensorrt.so`
- [ ] Create tarball: `tar -czf cuda-libs-cuda12.tar.gz -C npm-package/lib/native/ lib*.so`
- [ ] Test download: Verify `postinstall.js` can download from GitHub release

### macOS CoreML Library
- [ ] Download ONNX Runtime for macOS:
  ```bash
  wget https://github.com/microsoft/onnxruntime/releases/download/v1.23.2/onnxruntime-osx-arm64-1.23.2.tgz
  tar -xzf onnxruntime-osx-arm64-1.23.2.tgz
  cp onnxruntime-osx-arm64-1.23.2/lib/libonnxruntime.1.23.2.dylib ./libonnxruntime.dylib
  ```
- [ ] Verify dylib: `file libonnxruntime.dylib` (should show Mach-O 64-bit arm64)
- [ ] Test download: Verify `postinstall.js` can download from GitHub release

## GitHub Releases

### GPU Libraries Release (if updated)
```bash
# Only create if CUDA/ONNX Runtime versions changed
gh release create gpu-libs-v1.2.0 \
  --title "GPU Libraries v1.2.0" \
  --notes "ONNX Runtime 1.23.2, CUDA 12.9, cuDNN 9.15.1"

gh release upload gpu-libs-v1.2.0 cuda-libs-cuda12.tar.gz
```

### macOS ONNX Runtime Release (if updated)
```bash
# Only create if ONNX Runtime version changed
gh release create onnx-runtime-macos-v1.23.2 \
  --title "ONNX Runtime macOS CoreML v1.23.2" \
  --notes "ONNX Runtime 1.23.2 with CoreML support for Apple Silicon"

gh release upload onnx-runtime-macos-v1.23.2 libonnxruntime.dylib
```

### Main Package Release
```bash
# Bump version in package.json first
npm version patch  # or minor, or major

# Create git tag and GitHub release
git push --follow-tags

gh release create v0.7.2 \
  --title "v0.7.2: Bug fixes and improvements" \
  --notes "See CHANGELOG.md for details"
```

## NPM Package Publishing

### Pre-Publish Tests
- [ ] Test on Linux:
  ```bash
  # On Linux machine or VM
  npm install -g /path/to/swictation --foreground-scripts
  swictation start
  # Test dictation works
  swictation stop
  npm uninstall -g swictation
  ```

- [ ] Test on macOS:
  ```bash
  # On macOS machine (Apple Silicon)
  npm install -g /path/to/swictation --foreground-scripts
  swictation start
  # Test dictation works
  swictation stop
  npm uninstall -g swictation
  ```

### Publish to NPM
```bash
# Login to npm
npm login

# Publish package
npm publish

# Test installation from npm
npm install -g swictation  # on both platforms
```

## Post-Release Verification

### Linux
- [ ] Install from npm: `npm install -g swictation`
- [ ] Verify postinstall downloads GPU libraries
- [ ] Start daemon: `swictation start`
- [ ] Test basic dictation
- [ ] Check GPU usage in `nvidia-smi`
- [ ] Stop daemon: `swictation stop`

### macOS
- [ ] Install from npm: `npm install -g swictation`
- [ ] Verify postinstall downloads ONNX Runtime dylib
- [ ] Start daemon: `swictation start`
- [ ] Grant Accessibility permissions
- [ ] Test basic dictation
- [ ] Check GPU usage in Activity Monitor
- [ ] Stop daemon: `swictation stop`

## Documentation Updates

- [ ] Update `README.md`:
  - Supported platforms (Linux x86_64, macOS ARM64)
  - Installation instructions for both platforms
  - Platform-specific requirements
  - Known limitations

- [ ] Update `CHANGELOG.md`:
  - Version number and date
  - New features
  - Bug fixes
  - Breaking changes
  - Platform-specific notes

- [ ] Update `docs/INSTALLATION.md`:
  - Platform-specific setup steps
  - Troubleshooting for both platforms

## Rollback Plan

If issues are discovered after release:

1. **NPM Package:**
   ```bash
   npm unpublish swictation@X.Y.Z  # within 72 hours
   # or
   npm deprecate swictation@X.Y.Z "Use version X.Y.Z+1 instead"
   ```

2. **GitHub Release:**
   ```bash
   gh release delete vX.Y.Z --yes
   git tag -d vX.Y.Z
   git push origin :refs/tags/vX.Y.Z
   ```

3. **Hotfix:**
   - Fix the issue
   - Increment patch version
   - Follow this checklist again

## Automation (Future)

Consider creating `scripts/prepare-release.sh` to automate:
- Platform detection
- Binary building
- Library verification
- Tarball creation
- Checksum generation
- Pre-publish tests

## Notes

- **ONNX Runtime Versions:** Update `ORT_VERSION` in `postinstall.js` if upgrading
- **CUDA Versions:** Update `GPU_LIBS_VERSION` in `postinstall.js` if upgrading
- **Platform Detection:** `postinstall.js` uses `os.platform()` and `os.arch()`
- **Binary Naming:**
  - Linux: `swictation-daemon` (no suffix)
  - macOS: `swictation-daemon-macos`
