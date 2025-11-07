#!/bin/bash
# Build script to prepare npm package with pre-compiled binaries

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
NPM_PACKAGE_DIR="$SCRIPT_DIR"

echo "🔨 Building Swictation NPM package..."

# Build Rust binaries in release mode
echo "📦 Building Rust binaries..."
cd "$REPO_ROOT/rust-crates"
cargo build --release

# Build Tauri UI
echo "🎨 Building Tauri UI..."
cd "$REPO_ROOT/tauri-ui/src-tauri"
cargo build --release

# Copy binaries to npm package
echo "📋 Copying binaries to npm package..."
mkdir -p "$NPM_PACKAGE_DIR/bin"

# Copy daemon binary
if [ -f "$REPO_ROOT/rust-crates/target/release/swictation-daemon" ]; then
    cp "$REPO_ROOT/rust-crates/target/release/swictation-daemon" "$NPM_PACKAGE_DIR/bin/"
    chmod +x "$NPM_PACKAGE_DIR/bin/swictation-daemon"
    echo "✓ Copied swictation-daemon"
else
    echo "⚠️  Warning: swictation-daemon binary not found"
fi

# Copy UI binary
if [ -f "$REPO_ROOT/tauri-ui/src-tauri/target/release/swictation-ui" ]; then
    cp "$REPO_ROOT/tauri-ui/src-tauri/target/release/swictation-ui" "$NPM_PACKAGE_DIR/bin/"
    chmod +x "$NPM_PACKAGE_DIR/bin/swictation-ui"
    echo "✓ Copied swictation-ui"
else
    echo "⚠️  Warning: swictation-ui binary not found"
fi

# Strip binaries to reduce size
echo "🔧 Stripping binaries..."
if command -v strip &> /dev/null; then
    strip "$NPM_PACKAGE_DIR/bin/swictation-daemon" 2>/dev/null || true
    strip "$NPM_PACKAGE_DIR/bin/swictation-ui" 2>/dev/null || true
    echo "✓ Binaries stripped"
else
    echo "⚠️  strip command not found, skipping"
fi

# Check binary sizes
echo ""
echo "📊 Binary sizes:"
ls -lh "$NPM_PACKAGE_DIR/bin/" | grep -E "swictation-daemon|swictation-ui"

# Create tarball for npm publish (optional)
echo ""
echo "📦 Creating package tarball..."
cd "$NPM_PACKAGE_DIR"
npm pack

echo ""
echo "✨ Build complete! Package is ready for publishing."
echo ""
echo "To test locally:"
echo "  cd $NPM_PACKAGE_DIR"
echo "  npm link"
echo "  swictation help"
echo ""
echo "To publish to npm:"
echo "  cd $NPM_PACKAGE_DIR"
echo "  npm publish"