# NPM Package Build Checklist

## Critical: Ensure All GPU Fixes Persist in NPM Package

This checklist guarantees that GPU environment fixes, FP32 model loading, and UI service configuration are included in every npm package build.

---

## Pre-Build Verification

### 1. Code Changes ✅
- [x] `rust-crates/swictation-stt/src/recognizer_ort.rs` - FP32 model selection for GPU (lines 110-115)
- [x] All systemd service files updated with CUDA environment variables

### 2. Service File Templates ✅
- [x] `npm-package/config/swictation-daemon.service` - Has CUDA paths
- [x] `npm-package/config/swictation-ui.service` - Exists
- [x] `npm-package/templates/swictation-daemon.service.template` - Has CUDA paths
- [x] `config/swictation-daemon.service` - Source of truth with CUDA paths

#### Required Environment Variables in Service Files:
```bash
Environment="RUST_LOG=info"
Environment="ORT_DYLIB_PATH=..."
Environment="LD_LIBRARY_PATH=/usr/local/cuda/lib64:/usr/local/cuda-12.9/lib64:..."
Environment="CUDA_HOME=/usr/local/cuda"
ImportEnvironment=
```

### 3. Postinstall Script ✅
- [x] `npm-package/postinstall.js` uses template system
- [x] Copies systemd-daemon.service from template (line 290-339)
- [ ] ⚠️ **ISSUE**: Does NOT install swictation-ui.service!

---

## Build Process

### Step 1: Build Rust Binaries with FP32 Fix
```bash
cd /opt/swictation
cargo build --release --manifest-path rust-crates/swictation-daemon/Cargo.toml
```

**Verify FP32 logic exists:**
```bash
grep -A5 "if use_gpu" rust-crates/swictation-stt/src/recognizer_ort.rs | grep "FP32"
# Should show: info!("Using FP32 model for GPU: {}.onnx", name);
```

### Step 2: Copy Binary to NPM Package
```bash
# CRITICAL: Binary goes to lib/native/ NOT bin/!
cp rust-crates/target/release/swictation-daemon \
   npm-package/lib/native/swictation-daemon.bin
chmod +x npm-package/lib/native/swictation-daemon.bin
```

### Step 3: Verify Service Files Have CUDA Paths
```bash
# Check all three locations
for file in \
  config/swictation-daemon.service \
  npm-package/config/swictation-daemon.service \
  npm-package/templates/swictation-daemon.service.template
do
  echo "=== $file ==="
  grep -E "LD_LIBRARY_PATH|CUDA_HOME|cuda-12.9" "$file" || echo "❌ MISSING CUDA PATHS!"
done
```

### Step 4: Update Postinstall to Install UI Service
**Add this to postinstall.js after generateSystemdService():**
```javascript
function installUIService() {
  log('cyan', '\n⚙️  Installing UI service file...');
  try {
    const uiServiceSource = path.join(__dirname, 'config', 'swictation-ui.service');
    const systemdDir = path.join(os.homedir(), '.config', 'systemd', 'user');
    const uiServiceDest = path.join(systemdDir, 'swictation-ui.service');

    if (fs.existsSync(uiServiceSource)) {
      fs.copyFileSync(uiServiceSource, uiServiceDest);
      log('green', `✓ Installed UI service: ${uiServiceDest}`);
    } else {
      log('yellow', `⚠️  UI service not found at ${uiServiceSource}`);
    }
  } catch (err) {
    log('yellow', `⚠️  Failed to install UI service: ${err.message}`);
  }
}

// Call in main() after generateSystemdService()
```

### Step 5: Version Bump
```bash
cd npm-package
npm version patch  # or minor/major
# This updates version in package.json
```

### Step 6: Create Package
```bash
cd npm-package
npm pack
# Creates swictation-X.Y.Z.tgz
```

---

## Testing Before Publish

### Test 1: Local Install
```bash
# Uninstall current version
sudo npm uninstall -g swictation

# Install from tarball
sudo npm install -g npm-package/swictation-*.tgz

# Verify binary was updated
stat /usr/local/lib/node_modules/swictation/lib/native/swictation-daemon.bin
# Check date is TODAY
```

### Test 2: Service Files Installed
```bash
# Check daemon service has CUDA paths
cat ~/.config/systemd/user/swictation-daemon.service | grep "cuda-12.9"
# Should show the cuda-12.9 path

# Check UI service installed
ls -la ~/.config/systemd/user/swictation-ui.service
# Should exist
```

### Test 3: FP32 Models Load
```bash
systemctl --user daemon-reload
systemctl --user restart swictation-daemon
sleep 3
journalctl --user -u swictation-daemon --since "10 seconds ago" | grep "Using FP32"
# Should show: "Using FP32 model for GPU: encoder.onnx"
```

### Test 4: GPU Environment Active
```bash
systemctl --user show swictation-daemon | grep LD_LIBRARY_PATH
# Should include: /usr/local/cuda/lib64:/usr/local/cuda-12.9/lib64
```

### Test 5: UI Service Running
```bash
systemctl --user status swictation-ui.service
# Should be: Active: active (running)
```

---

## Automated Build Script

Create `/opt/swictation/scripts/build-npm-package.sh`:

```bash
#!/bin/bash
set -e

echo "🚀 Building NPM Package with GPU Fixes..."
cd /opt/swictation

echo "1️⃣  Building Rust daemon with FP32 support..."
cargo build --release --manifest-path rust-crates/swictation-daemon/Cargo.toml

echo "2️⃣  Copying binary to npm-package..."
cp rust-crates/target/release/swictation-daemon \
   npm-package/lib/native/swictation-daemon.bin
chmod +x npm-package/lib/native/swictation-daemon.bin

echo "3️⃣  Verifying service files have CUDA environment..."
if ! grep -q "cuda-12.9" npm-package/config/swictation-daemon.service; then
  echo "❌ ERROR: npm-package/config/swictation-daemon.service missing cuda-12.9 path!"
  exit 1
fi

if ! grep -q "CUDA_HOME" npm-package/templates/swictation-daemon.service.template; then
  echo "❌ ERROR: npm-package/templates/swictation-daemon.service.template missing CUDA_HOME!"
  exit 1
fi

echo "4️⃣  Creating npm package..."
cd npm-package
npm pack

echo ""
echo "✅ Package built successfully!"
echo ""
echo "📦 Tarball: $(ls -1 swictation-*.tgz | tail -1)"
echo ""
echo "Test locally with:"
echo "  sudo npm install -g swictation-*.tgz"
echo ""
echo "Publish with:"
echo "  npm publish swictation-*.tgz"
```

---

## Final Verification After Publish

After publishing to npm, verify on a CLEAN system:

```bash
# Fresh install
sudo npm install -g swictation

# Check binary date
stat /usr/local/lib/node_modules/swictation/lib/native/swictation-daemon.bin

# Check service has CUDA
cat ~/.config/systemd/user/swictation-daemon.service | grep cuda-12.9

# Test GPU loading
systemctl --user restart swictation-daemon
journalctl --user -u swictation-daemon | grep "FP32"
```

---

## Known Issues to Fix

1. **postinstall.js doesn't install UI service** - Need to add installUIService()
2. **build.sh is outdated** - Copies to bin/ instead of lib/native/
3. **No automated verification** - Should add checks to build script

---

**Last Updated**: 2025-11-12
**Commits**: 4ff9e314, 3716278f (GPU environment fixes)
