# Installation Fixes Required for v0.2.2

## What Works Now (11/11/2025)

✅ Daemon v0.2.2 running with CUDA GPU acceleration
✅ Silero VAD loading with correct k2-fsa model (x/h/c tensor format)
✅ Parakeet-TDT 1.1B INT8 model with GPU
✅ Auto model selection based on VRAM (97GB detected)
✅ IPC server listening on /tmp/swictation.sock

## What Had to Be Fixed Manually

### 1. **Rust Binary Version Mismatch**
- **Problem**: Daemon binary was v0.1.0, npm package was v0.2.2
- **Root Cause**: Cargo.toml files had version = "0.1.0"
- **Fix**: Updated all `rust-crates/swictation-*/Cargo.toml` to `version = "0.2.2"`
- **Command**: `sed -i 's/^version = "0.1.0"/version = "0.2.2"/' swictation-*/Cargo.toml`
- **Rebuild**: `cargo build --release --bin swictation-daemon`
- **Copy**: `cp rust-crates/target/release/swictation-daemon npm-package/lib/native/swictation-daemon.bin`

### 2. **Config File Format**
- **Problem**: Config had `[general]` section, but Rust struct expects flat TOML
- **Root Cause**: TOML deserializer doesn't match struct with `#[serde(rename)]`
- **Fix**: Remove `[general]` section header, keep only `[hotkeys]`
- **Also Added**: Missing `stt_model_override = "auto"` field required by DaemonConfig struct

**Correct config format:**
```toml
socket_path = "/tmp/swictation.sock"
vad_model_path = "/home/robert/.local/share/swictation/models/silero-vad/silero_vad.onnx"
vad_min_silence = 0.5
vad_min_speech = 0.25
vad_max_speech = 30.0
vad_threshold = 0.003
stt_model_override = "auto"
stt_0_6b_model_path = "/home/robert/.local/share/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx"
stt_1_1b_model_path = "/home/robert/.local/share/swictation/models/parakeet-tdt-1.1b-onnx"
num_threads = 4

[hotkeys]
toggle = "Super+Shift+D"
push_to_talk = "Super+Space"
```

### 3. **CUDA Provider Libraries Missing**
- **Problem**: ONNX Runtime couldn't find `libonnxruntime_providers_cuda.so`
- **Root Cause**: CUDA libs (330MB) not included in npm package
- **Fix**: Copy CUDA providers to `lib/native/`:
  ```bash
  sudo cp rust-crates/target/release/libonnxruntime_providers_*.so \
           /usr/local/lib/node_modules/swictation/lib/native/
  ```
- **Files**:
  - `libonnxruntime_providers_cuda.so` (330MB)
  - `libonnxruntime_providers_shared.so` (15KB)
  - `libonnxruntime_providers_tensorrt.so` (787KB)

### 4. **1.1B Model Downloaded**
- **Downloaded from**: https://huggingface.co/jenerallee78/parakeet-tdt-1.1b-onnx
- **Size**: ~12GB (includes INT8 + FP32 + weights)
- **Command**: `node lib/model-downloader.js --model=1.1b --force`

## What Needs to Be Fixed in npm Package

### Fix 1: Versioning
**File**: `rust-crates/swictation-*/Cargo.toml` (6 files)
**Change**: Keep Rust versions in sync with npm `package.json`
**Solution**: Add pre-publish script to sync versions

### Fix 2: Config Template
**File**: `config/swictation-config-template.toml` (create this)
**Change**: Ship correct config template without `[general]` section
**Include**: `stt_model_override = "auto"` field

### Fix 3: CUDA Libraries Download
**File**: `postinstall.js` (already has GPU download logic!)
**Current**: Lines 182-228 download GPU libs from GitHub release
**Status**: Code exists, but needs GitHub release created
**Required**: Create `swictation-gpu-libs.tar.gz` at:
  `https://github.com/robertelee78/swictation/releases/download/v0.2.2/swictation-gpu-libs.tar.gz`

### Fix 4: Model Downloads
**File**: `lib/model-downloader.js`
**Status**: ✅ Already working correctly
- VAD: Direct URL from k2-fsa/sherpa-onnx
- 1.1B: HuggingFace jenerallee78/parakeet-tdt-1.1b-onnx

## Build Script Needed

Create `scripts/prepare-npm-package.sh`:
```bash
#!/bin/bash
set -e

echo "📦 Preparing swictation npm package..."

# 1. Sync versions
VERSION=$(grep "version" package.json | head -1 | sed 's/.*: "\(.*\)".*/\1/')
echo "Setting Rust crates to version $VERSION"
find ../rust-crates -name Cargo.toml -exec sed -i "s/^version = .*/version = \"$VERSION\"/" {} \;

# 2. Build Rust binaries
echo "Building Rust binaries..."
cd ../rust-crates
cargo build --release --bin swictation-daemon
cd ../npm-package

# 3. Copy binaries
echo "Copying binaries..."
mkdir -p lib/native
cp ../rust-crates/target/release/swictation-daemon lib/native/swictation-daemon.bin
cp ../rust-crates/target/release/libonnxruntime.so lib/native/

# 4. Create GPU libs tarball (for GitHub release)
echo "Creating GPU libs tarball..."
tar -czf swictation-gpu-libs.tar.gz -C ../rust-crates/target/release \
    libonnxruntime_providers_cuda.so \
    libonnxruntime_providers_shared.so \
    libonnxruntime_providers_tensorrt.so

echo "✅ Package prepared!"
echo "⚠️  Remember to:"
echo "   1. Upload swictation-gpu-libs.tar.gz to GitHub release v$VERSION"
echo "   2. Test: sudo npm install -g ."
echo "   3. Publish: npm publish"
```

## Testing Checklist

- [ ] Fresh npm install: `sudo npm install -g .`
- [ ] Models download automatically
- [ ] GPU libs download from GitHub release
- [ ] Daemon starts successfully
- [ ] CUDA provider loads
- [ ] 1.1B model selected on GPU
- [ ] VAD works with k2-fsa model
- [ ] Config created with correct format

## Files Modified in This Session

1. `npm-package/lib/model-downloader.js` - k2-fsa VAD URL
2. `npm-package/CHANGELOG.md` - v0.2.2 release notes
3. `npm-package/package.json` - version 0.2.2
4. `rust-crates/swictation-*/Cargo.toml` - version 0.2.2
5. `~/.config/swictation/config.toml` - correct format

## Ready for npm Publish?

**NO** - Need to:
1. Create GitHub release v0.2.2
2. Upload `swictation-gpu-libs.tar.gz` to release
3. Add prepare script to package.json
4. Test fresh install on clean system
