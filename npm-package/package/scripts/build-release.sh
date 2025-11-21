#!/bin/bash
# Pre-release build script - ensures all binaries are fresh before npm publish
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
NPM_NATIVE_DIR="$REPO_ROOT/npm-package/package/lib/native"

echo "🔨 Building Swictation binaries for release..."
echo ""

# 1. Build all Rust binaries in release mode
echo "📦 Building Rust workspace in release mode..."
cd "$REPO_ROOT/rust-crates"
cargo build --release --workspace
echo "✓ Rust build complete"
echo ""

# 2. Copy binaries to npm package (BOTH bin/ and lib/native/)
echo "📋 Copying binaries to npm package..."
NPM_BIN_DIR="$REPO_ROOT/npm-package/package/bin"
mkdir -p "$NPM_BIN_DIR"
mkdir -p "$NPM_NATIVE_DIR"

# Daemon - copy to bin/ (used by CLI)
cp "$REPO_ROOT/rust-crates/target/release/swictation-daemon" \
   "$NPM_BIN_DIR/swictation-daemon"
chmod +x "$NPM_BIN_DIR/swictation-daemon"
echo "✓ Copied swictation-daemon to bin/"

# Daemon - also copy to lib/native/ for backwards compatibility
cp "$REPO_ROOT/rust-crates/target/release/swictation-daemon" \
   "$NPM_NATIVE_DIR/swictation-daemon.bin"
chmod +x "$NPM_NATIVE_DIR/swictation-daemon.bin"
echo "✓ Copied swictation-daemon to lib/native/"

# Shared libraries (already in npm package, just verify they exist)
if [ ! -f "$NPM_NATIVE_DIR/libonnxruntime.so" ]; then
    echo "⚠️  Warning: libonnxruntime.so not found in npm package"
fi
if [ ! -f "$NPM_NATIVE_DIR/libsherpa-onnx-c-api.so" ]; then
    echo "⚠️  Warning: libsherpa-onnx-c-api.so not found in npm package"
fi

echo ""

# 3. Verify binary content matches (use checksums instead of timestamps)
echo "🔍 Verifying binary integrity..."
DAEMON_SOURCE="$REPO_ROOT/rust-crates/target/release/swictation-daemon"
DAEMON_BIN="$NPM_BIN_DIR/swictation-daemon"
DAEMON_NATIVE="$NPM_NATIVE_DIR/swictation-daemon.bin"

SOURCE_HASH=$(sha256sum "$DAEMON_SOURCE" | awk '{print $1}')
BIN_HASH=$(sha256sum "$DAEMON_BIN" | awk '{print $1}')
NATIVE_HASH=$(sha256sum "$DAEMON_NATIVE" | awk '{print $1}')

if [ "$SOURCE_HASH" != "$BIN_HASH" ] || [ "$SOURCE_HASH" != "$NATIVE_HASH" ]; then
    echo "❌ ERROR: Binary checksums don't match!"
    echo "   Source:  $SOURCE_HASH"
    echo "   bin/:    $BIN_HASH"
    echo "   native/: $NATIVE_HASH"
    exit 1
fi

echo "✓ Binary checksums match (bin/ and lib/native/ are identical to source)"
echo ""

# 4. Show binary info
echo "📊 Binary info:"
ls -lh "$DAEMON_BIN"
ls -lh "$DAEMON_NATIVE"
echo ""

# 5. Verify key strings in binary (sanity check)
echo "🔍 Sanity check - looking for recent code changes..."
if strings "$DAEMON_BIN" | grep -q "# Swictation"; then
    echo "✓ Found simplified Sway config format"
else
    echo "⚠️  Warning: Simplified config format not found in binary"
fi

echo ""
echo "✅ Pre-release build complete!"
echo "   Ready for npm publish"
