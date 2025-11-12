#!/bin/bash
# Comprehensive NPM package build script with GPU fix verification
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"

echo "🚀 Building Swictation NPM Package with GPU Fixes..."
echo "=================================================="
echo ""

# Step 1: Build Rust binaries
echo "1️⃣  Building Rust daemon with FP32 support..."
cd "$REPO_ROOT"
cargo build --release --manifest-path rust-crates/swictation-daemon/Cargo.toml

if [ ! -f "rust-crates/target/release/swictation-daemon" ]; then
  echo "❌ ERROR: Rust build failed - binary not found!"
  exit 1
fi
echo "   ✓ Rust daemon built successfully"
echo ""

# Step 2: Verify FP32 logic in source code
echo "2️⃣  Verifying FP32 model selection code..."
if grep -q 'info!("Using FP32 model for GPU:' rust-crates/swictation-stt/src/recognizer_ort.rs; then
  echo "   ✓ FP32 selection logic present in recognizer_ort.rs"
else
  echo "   ❌ WARNING: FP32 selection logic not found!"
  echo "   This may cause INT8 models to load instead of FP32"
fi
echo ""

# Step 3: Copy binary to npm package
echo "3️⃣  Copying binary to npm-package/lib/native/..."
mkdir -p "$REPO_ROOT/npm-package/lib/native"
cp rust-crates/target/release/swictation-daemon \
   npm-package/lib/native/swictation-daemon.bin
chmod +x npm-package/lib/native/swictation-daemon.bin

BINARY_SIZE=$(du -h npm-package/lib/native/swictation-daemon.bin | cut -f1)
echo "   ✓ Binary copied (size: $BINARY_SIZE)"
echo ""

# Step 4: Verify service files have CUDA environment
echo "4️⃣  Verifying CUDA environment in service files..."

check_service_file() {
  local file=$1
  local errors=0

  if [ ! -f "$file" ]; then
    echo "   ❌ ERROR: File not found: $file"
    return 1
  fi

  if ! grep -q "cuda-12.9" "$file"; then
    echo "   ❌ ERROR: Missing cuda-12.9 path in $file"
    errors=$((errors + 1))
  fi

  if ! grep -q "CUDA_HOME" "$file"; then
    echo "   ❌ ERROR: Missing CUDA_HOME in $file"
    errors=$((errors + 1))
  fi

  if ! grep -q "ORT_DYLIB_PATH" "$file"; then
    echo "   ❌ ERROR: Missing ORT_DYLIB_PATH in $file"
    errors=$((errors + 1))
  fi

  if [ $errors -eq 0 ]; then
    echo "   ✓ $file has all required environment variables"
  else
    echo "   ❌ $file is missing critical environment variables!"
    return 1
  fi
}

check_service_file "npm-package/config/swictation-daemon.service" || exit 1
check_service_file "npm-package/templates/swictation-daemon.service.template" || exit 1

if [ ! -f "npm-package/config/swictation-ui.service" ]; then
  echo "   ❌ WARNING: npm-package/config/swictation-ui.service not found"
  echo "   UI service will not be installed by postinstall script"
else
  echo "   ✓ npm-package/config/swictation-ui.service exists"
fi
echo ""

# Step 5: Verify postinstall script
echo "5️⃣  Checking postinstall.js configuration..."
if grep -q "generateSystemdService" npm-package/postinstall.js; then
  echo "   ✓ postinstall.js will generate daemon service from template"
else
  echo "   ❌ WARNING: postinstall.js may not generate service files correctly"
fi
echo ""

# Step 6: Create package
echo "6️⃣  Creating npm package tarball..."
cd "$REPO_ROOT/npm-package"

CURRENT_VERSION=$(node -p "require('./package.json').version")
echo "   Current version: $CURRENT_VERSION"

# Create tarball
npm pack

TARBALL=$(ls -1t swictation-*.tgz | head -1)
TARBALL_SIZE=$(du -h "$TARBALL" | cut -f1)

echo "   ✓ Package created: $TARBALL (size: $TARBALL_SIZE)"
echo ""

# Step 7: Summary and next steps
echo "=================================================="
echo "✅ NPM Package Built Successfully!"
echo "=================================================="
echo ""
echo "📦 Package: $TARBALL"
echo "🔢 Version: $CURRENT_VERSION"
echo "📏 Size: $TARBALL_SIZE"
echo ""
echo "🧪 To test locally:"
echo "   sudo npm uninstall -g swictation"
echo "   sudo npm install -g $REPO_ROOT/npm-package/$TARBALL"
echo "   swictation --version"
echo ""
echo "🔍 To verify GPU fixes after install:"
echo "   cat ~/.config/systemd/user/swictation-daemon.service | grep cuda-12.9"
echo "   systemctl --user restart swictation-daemon"
echo "   journalctl --user -u swictation-daemon --since '1 min ago' | grep FP32"
echo ""
echo "📤 To publish to npm registry:"
echo "   cd $REPO_ROOT/npm-package"
echo "   npm publish $TARBALL"
echo ""
