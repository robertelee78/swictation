#!/bin/bash
# Pre-release build script - ensures all binaries are fresh before npm publish
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
NPM_NATIVE_DIR="$REPO_ROOT/npm-package/lib/native"

echo "🔨 Building Swictation binaries for release..."
echo ""

# 1. Build all Rust binaries in release mode
echo "📦 Building Rust workspace in release mode..."
cd "$REPO_ROOT/rust-crates"
cargo build --release --workspace
echo "✓ Rust build complete"
echo ""

# 2. Copy binaries to npm package
echo "📋 Copying binaries to npm package..."
mkdir -p "$NPM_NATIVE_DIR"

# Daemon
cp "$REPO_ROOT/rust-crates/target/release/swictation-daemon" \
   "$NPM_NATIVE_DIR/swictation-daemon.bin"
chmod +x "$NPM_NATIVE_DIR/swictation-daemon.bin"
echo "✓ Copied swictation-daemon"

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
DAEMON_NPM="$NPM_NATIVE_DIR/swictation-daemon.bin"

SOURCE_HASH=$(sha256sum "$DAEMON_SOURCE" | awk '{print $1}')
NPM_HASH=$(sha256sum "$DAEMON_NPM" | awk '{print $1}')

if [ "$SOURCE_HASH" != "$NPM_HASH" ]; then
    echo "❌ ERROR: Binary checksums don't match!"
    echo "   This should never happen - cp just ran!"
    exit 1
fi

echo "✓ Binary checksums match"
echo ""

# 4. Show binary info
echo "📊 Binary info:"
ls -lh "$DAEMON_NPM"
echo ""

# 5. Verify key strings in binary (sanity check)
echo "🔍 Sanity check - looking for recent code changes..."
if strings "$DAEMON_NPM" | grep -q "# Swictation"; then
    echo "✓ Found simplified Sway config format"
else
    echo "⚠️  Warning: Simplified config format not found in binary"
fi

echo ""
echo "✅ Pre-release build complete!"
echo "   Ready for npm publish"
