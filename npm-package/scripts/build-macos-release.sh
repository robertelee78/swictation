#!/bin/bash
# macOS build script for Apple Silicon - compile Rust binaries for aarch64-apple-darwin
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
NPM_NATIVE_DIR="$REPO_ROOT/npm-package/lib/native"
NPM_BIN_DIR="$REPO_ROOT/npm-package/bin"

echo "🍎 Building Swictation binaries for macOS (Apple Silicon)..."
echo ""

# 1. Platform validation - ensure we're on macOS
echo "🔍 Checking platform..."
PLATFORM=$(uname -s)
if [ "$PLATFORM" != "Darwin" ]; then
    echo "❌ ERROR: This script must run on macOS (Darwin)"
    echo "   Current platform: $PLATFORM"
    exit 1
fi
echo "✓ Platform: macOS ($PLATFORM)"

# 2. Architecture validation - ensure Apple Silicon (ARM64)
ARCH=$(uname -m)
if [ "$ARCH" != "arm64" ]; then
    echo "⚠️  WARNING: This script is designed for Apple Silicon (arm64)"
    echo "   Current architecture: $ARCH"
    echo "   Build may fail or produce incorrect binaries"
fi
echo "✓ Architecture: $ARCH"
echo ""

# 3. Check Rust toolchain for aarch64-apple-darwin
echo "🦀 Checking Rust toolchain..."
if ! rustup target list --installed | grep -q "aarch64-apple-darwin"; then
    echo "⚠️  Target aarch64-apple-darwin not installed"
    echo "   Installing with: rustup target add aarch64-apple-darwin"
    rustup target add aarch64-apple-darwin
fi
echo "✓ Rust target: aarch64-apple-darwin"
echo ""

# 4. Build Rust workspace in release mode for Apple Silicon
echo "📦 Building Rust workspace for aarch64-apple-darwin..."
cd "$REPO_ROOT/rust-crates"
cargo build --release --target aarch64-apple-darwin --workspace --features coreml-native
echo "✓ Rust build complete"
echo ""

# 5. Create output directories
echo "📁 Preparing npm package directories..."
mkdir -p "$NPM_BIN_DIR"
mkdir -p "$NPM_NATIVE_DIR"
echo "✓ Directories ready"
echo ""

# 6. Copy binaries to npm package
echo "📋 Copying macOS binaries to npm package..."

# Daemon - copy to bin/ (used by CLI)
DAEMON_SOURCE="$REPO_ROOT/rust-crates/target/aarch64-apple-darwin/release/swictation-daemon"
cp "$DAEMON_SOURCE" "$NPM_BIN_DIR/swictation-daemon-macos"
chmod +x "$NPM_BIN_DIR/swictation-daemon-macos"
echo "✓ Copied swictation-daemon to bin/swictation-daemon-macos"

# Daemon - also copy to lib/native/ for backwards compatibility
cp "$DAEMON_SOURCE" "$NPM_NATIVE_DIR/swictation-daemon-macos.bin"
chmod +x "$NPM_NATIVE_DIR/swictation-daemon-macos.bin"
echo "✓ Copied swictation-daemon to lib/native/swictation-daemon-macos.bin"
echo ""

# 7. Check for ONNX Runtime dylibs (CoreML-enabled)
echo "🔍 Checking for ONNX Runtime dylibs..."
if [ ! -f "$NPM_NATIVE_DIR/libonnxruntime.dylib" ]; then
    echo "⚠️  libonnxruntime.dylib not found in npm package"
    echo "   This library is required for CoreML GPU acceleration"
    echo "   Run: npm run download-onnx-macos"
else
    echo "✓ Found libonnxruntime.dylib (CoreML support)"
fi
echo ""

# 8. Verify binary checksums match
echo "🔍 Verifying binary integrity..."
DAEMON_BIN="$NPM_BIN_DIR/swictation-daemon-macos"
DAEMON_NATIVE="$NPM_NATIVE_DIR/swictation-daemon-macos.bin"

# Use shasum on macOS (sha256sum equivalent)
SOURCE_HASH=$(shasum -a 256 "$DAEMON_SOURCE" | awk '{print $1}')
BIN_HASH=$(shasum -a 256 "$DAEMON_BIN" | awk '{print $1}')
NATIVE_HASH=$(shasum -a 256 "$DAEMON_NATIVE" | awk '{print $1}')

if [ "$SOURCE_HASH" != "$BIN_HASH" ] || [ "$SOURCE_HASH" != "$NATIVE_HASH" ]; then
    echo "❌ ERROR: Binary checksums don't match!"
    echo "   Source:  $SOURCE_HASH"
    echo "   bin/:    $BIN_HASH"
    echo "   native/: $NATIVE_HASH"
    exit 1
fi

echo "✓ Binary checksums match (bin/ and lib/native/ are identical to source)"
echo ""

# 9. Show binary info
echo "📊 Binary info:"
ls -lh "$DAEMON_BIN"
ls -lh "$DAEMON_NATIVE"
echo ""

# 10. Check dynamic library dependencies with otool
echo "🔗 Checking dynamic library dependencies..."
echo "Dependencies for swictation-daemon-macos:"
otool -L "$DAEMON_BIN" | tail -n +2 | sed 's/^/  /'
echo ""

# 11. Verify CoreML/Metal support compiled in
echo "🔍 Verifying CoreML/Metal support..."
if otool -L "$DAEMON_BIN" | grep -q "libonnxruntime"; then
    echo "✓ Binary links to libonnxruntime (CoreML support detected)"
elif strings "$DAEMON_BIN" | grep -i "coreml" > /dev/null 2>&1; then
    echo "✓ Found CoreML references in binary"
else
    echo "⚠️  Warning: CoreML support not clearly visible in binary"
    echo "   This may be normal if using static linking or feature flags"
fi
echo ""

# 12. Sanity check - verify binary can run
echo "🧪 Sanity check - testing binary..."
if "$DAEMON_BIN" --help > /dev/null 2>&1; then
    echo "✓ Binary executes successfully"
else
    echo "⚠️  Warning: Binary --help command failed"
    echo "   This may indicate missing dependencies or compilation issues"
fi
echo ""

# 13. Create checksums file for npm package
echo "📝 Creating build-checksums.txt..."
{
    echo "# macOS Build Checksums (Apple Silicon - aarch64-apple-darwin)"
    echo "# Generated: $(date -u +"%Y-%m-%d %H:%M:%S UTC")"
    echo "# Build host: $(hostname)"
    echo "# Architecture: $(uname -m)"
    echo ""
    echo "swictation-daemon-macos:"
    shasum -a 256 "$DAEMON_BIN" | sed 's|.*/||'
    echo ""
    echo "swictation-daemon-macos.bin:"
    shasum -a 256 "$DAEMON_NATIVE" | sed 's|.*/||'
} > "$REPO_ROOT/npm-package/build-checksums-macos.txt"
echo "✓ Checksums saved to build-checksums-macos.txt"
echo ""

# 14. Final summary
echo "✅ macOS build complete!"
echo ""
echo "📦 Build artifacts:"
echo "   - swictation-daemon-macos (bin/)"
echo "   - swictation-daemon-macos.bin (lib/native/)"
echo ""
echo "🚀 Next steps:"
echo "   1. Test binary on macOS: $DAEMON_BIN --help"
echo "   2. Verify CoreML GPU acceleration works"
echo "   3. Update npm package.json to include macOS binaries"
echo "   4. Test installation: npm install -g ."
echo ""
