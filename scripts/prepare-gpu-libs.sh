#!/usr/bin/env bash
# Create GPU libraries tarball for GitHub release
# This bundle includes CUDA providers that are too large for npm package

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
VERSION=$(grep '"version"' "$PROJECT_ROOT/npm-package/package.json" | head -1 | sed 's/.*: "\(.*\)".*/\1/')

echo "📦 Preparing GPU libraries tarball for v$VERSION"
echo ""

# GPU libraries are copied to target/release during build
LIB_DIR="$PROJECT_ROOT/rust-crates/target/release"

if [ ! -d "$LIB_DIR" ]; then
    echo "❌ Error: Could not find Rust build directory"
    echo "   Expected: $LIB_DIR"
    echo "   Please run: cd rust-crates && cargo build --release --bin swictation-daemon"
    exit 1
fi

echo "📂 Found libraries in: $LIB_DIR"
echo ""

# Check for required files
REQUIRED_FILES=(
    "libonnxruntime_providers_cuda.so"
    "libonnxruntime_providers_shared.so"
)

OPTIONAL_FILES=(
    "libonnxruntime_providers_tensorrt.so"
)

echo "🔍 Checking for required files..."
for file in "${REQUIRED_FILES[@]}"; do
    if [ -f "$LIB_DIR/$file" ]; then
        SIZE=$(du -h "$LIB_DIR/$file" | cut -f1)
        echo "  ✓ $file ($SIZE)"
    else
        echo "  ❌ $file - NOT FOUND"
        exit 1
    fi
done

echo ""
echo "🔍 Checking for optional files..."
for file in "${OPTIONAL_FILES[@]}"; do
    if [ -f "$LIB_DIR/$file" ]; then
        SIZE=$(du -h "$LIB_DIR/$file" | cut -f1)
        echo "  ✓ $file ($SIZE)"
    else
        echo "  ⚠  $file - not found (optional)"
    fi
done

# Create tarball
TARBALL="$PROJECT_ROOT/swictation-gpu-libs.tar.gz"
echo ""
echo "📦 Creating tarball: $TARBALL"

cd "$LIB_DIR"
tar -czf "$TARBALL" \
    libonnxruntime_providers_cuda.so \
    libonnxruntime_providers_shared.so \
    $([ -f libonnxruntime_providers_tensorrt.so ] && echo "libonnxruntime_providers_tensorrt.so" || true)

cd "$PROJECT_ROOT"

if [ -f "$TARBALL" ]; then
    TARBALL_SIZE=$(du -h "$TARBALL" | cut -f1)
    echo "✓ Created: $TARBALL ($TARBALL_SIZE)"
    echo ""
    echo "📋 Contents:"
    tar -tzf "$TARBALL"
    echo ""
    echo "✅ GPU libraries tarball ready!"
    echo ""
    echo "📤 Next steps:"
    echo "  1. Create GitHub release: gh release create v$VERSION"
    echo "  2. Upload tarball: gh release upload v$VERSION $TARBALL"
    echo ""
    echo "  Or manually upload to:"
    echo "  https://github.com/robertelee78/swictation/releases/tag/v$VERSION"
else
    echo "❌ Failed to create tarball"
    exit 1
fi
