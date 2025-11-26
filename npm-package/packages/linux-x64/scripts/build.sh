#!/bin/bash
# Linux x64 platform package build script
# Builds Rust binaries and packages them for @swictation/linux-x64
# Designed to run in CI/CD (GitHub Actions) or locally
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$PACKAGE_DIR/../../.." && pwd)"
PLATFORM_BIN_DIR="$PACKAGE_DIR/bin"
PLATFORM_LIB_DIR="$PACKAGE_DIR/lib"

echo "🔨 Building @swictation/linux-x64 platform package..."
echo ""
echo "📁 Directories:"
echo "   Repository root: $REPO_ROOT"
echo "   Package root:    $PACKAGE_DIR"
echo "   Output bin/:     $PLATFORM_BIN_DIR"
echo "   Output lib/:     $PLATFORM_LIB_DIR"
echo ""

# ============================================================================
# STEP 1: Platform Validation
# ============================================================================
echo "🔍 Step 1: Platform validation..."
PLATFORM=$(uname -s)
if [ "$PLATFORM" != "Linux" ]; then
    echo "❌ ERROR: This script must run on Linux"
    echo "   Current platform: $PLATFORM"
    exit 1
fi
echo "✓ Platform: Linux"

ARCH=$(uname -m)
if [ "$ARCH" != "x86_64" ]; then
    echo "❌ ERROR: This script requires x86_64 architecture"
    echo "   Current architecture: $ARCH"
    exit 1
fi
echo "✓ Architecture: x86_64"
echo ""

# ============================================================================
# STEP 2: Check Rust Toolchain
# ============================================================================
echo "🦀 Step 2: Checking Rust toolchain..."
if ! command -v cargo &> /dev/null; then
    echo "❌ ERROR: cargo not found. Install Rust from https://rustup.rs"
    exit 1
fi

RUST_VERSION=$(rustc --version)
echo "✓ Rust toolchain: $RUST_VERSION"

# Ensure x86_64-unknown-linux-gnu target is available
if ! rustup target list --installed | grep -q "x86_64-unknown-linux-gnu"; then
    echo "   Installing target x86_64-unknown-linux-gnu..."
    rustup target add x86_64-unknown-linux-gnu
fi
echo "✓ Rust target: x86_64-unknown-linux-gnu"
echo ""

# ============================================================================
# STEP 3: Load Component Versions from versions.json
# ============================================================================
echo "📋 Step 3: Loading component versions from versions.json..."
VERSIONS_FILE="$REPO_ROOT/npm-package/versions.json"

if [ ! -f "$VERSIONS_FILE" ]; then
    echo "❌ ERROR: versions.json not found at $VERSIONS_FILE"
    exit 1
fi

# Extract versions using jq (fallback to sed if jq not available)
if command -v jq &> /dev/null; then
    DISTRIBUTION_VERSION=$(jq -r '.distribution' "$VERSIONS_FILE")
    DAEMON_VERSION=$(jq -r '.components.daemon.version' "$VERSIONS_FILE")
    UI_VERSION=$(jq -r '.components.ui.version' "$VERSIONS_FILE")
    ONNX_VERSION=$(jq -r '.libraries.onnxruntime."linux-gpu"' "$VERSIONS_FILE")
else
    # Fallback to grep/sed
    DISTRIBUTION_VERSION=$(grep '"distribution"' "$VERSIONS_FILE" | sed 's/.*: *"\([^"]*\)".*/\1/')
    DAEMON_VERSION=$(grep -A 1 '"daemon"' "$VERSIONS_FILE" | grep '"version"' | sed 's/.*: *"\([^"]*\)".*/\1/')
    UI_VERSION=$(grep -A 1 '"ui"' "$VERSIONS_FILE" | grep '"version"' | sed 's/.*: *"\([^"]*\)".*/\1/')
    ONNX_VERSION=$(grep '"linux-gpu"' "$VERSIONS_FILE" | sed 's/.*: *"\([^"]*\)".*/\1/')
fi

echo "   Distribution: $DISTRIBUTION_VERSION"
echo "   Daemon:       $DAEMON_VERSION"
echo "   UI:           $UI_VERSION"
echo "   ONNX Runtime: $ONNX_VERSION"
echo ""

# ============================================================================
# STEP 4: Build Rust Daemon (swictation-daemon)
# ============================================================================
echo "📦 Step 4: Building Rust daemon workspace..."
cd "$REPO_ROOT/rust-crates"

echo "   Running: cargo build --release --target x86_64-unknown-linux-gnu --workspace"
cargo build --release --target x86_64-unknown-linux-gnu --workspace

DAEMON_SOURCE="$REPO_ROOT/rust-crates/target/x86_64-unknown-linux-gnu/release/swictation-daemon"
if [ ! -f "$DAEMON_SOURCE" ]; then
    echo "❌ ERROR: Daemon binary not found at $DAEMON_SOURCE"
    exit 1
fi

echo "✓ Daemon build complete"
echo "   Source: $DAEMON_SOURCE"
echo "   Size: $(du -h "$DAEMON_SOURCE" | cut -f1)"
echo ""

# ============================================================================
# STEP 5: Build Tauri UI (swictation-ui)
# ============================================================================
echo "🖥️  Step 5: Building Tauri UI..."
cd "$REPO_ROOT/tauri-ui"

# Check if Tauri CLI is available
if ! command -v cargo &> /dev/null || ! cargo tauri --version &> /dev/null 2>&1; then
    echo "⚠️  Warning: Tauri CLI not found, attempting to install..."
    if ! cargo install tauri-cli --version '^2.0.0'; then
        echo "❌ ERROR: Failed to install Tauri CLI"
        exit 1
    fi
fi

echo "   Running: cargo tauri build --target x86_64-unknown-linux-gnu"
cargo tauri build --target x86_64-unknown-linux-gnu

UI_SOURCE="$REPO_ROOT/tauri-ui/src-tauri/target/x86_64-unknown-linux-gnu/release/swictation-ui"
if [ ! -f "$UI_SOURCE" ]; then
    echo "❌ ERROR: UI binary not found at $UI_SOURCE"
    exit 1
fi

echo "✓ UI build complete"
echo "   Source: $UI_SOURCE"
echo "   Size: $(du -h "$UI_SOURCE" | cut -f1)"
echo ""

# ============================================================================
# STEP 6: Prepare Output Directories
# ============================================================================
echo "📁 Step 6: Preparing output directories..."
mkdir -p "$PLATFORM_BIN_DIR"
mkdir -p "$PLATFORM_LIB_DIR"
echo "✓ Directories created"
echo ""

# ============================================================================
# STEP 7: Copy Binaries to Platform Package
# ============================================================================
echo "📋 Step 7: Copying binaries to platform package..."

# Copy daemon
cp "$DAEMON_SOURCE" "$PLATFORM_BIN_DIR/swictation-daemon"
chmod +x "$PLATFORM_BIN_DIR/swictation-daemon"
echo "✓ Copied swictation-daemon"

# Copy UI
cp "$UI_SOURCE" "$PLATFORM_BIN_DIR/swictation-ui"
chmod +x "$PLATFORM_BIN_DIR/swictation-ui"
echo "✓ Copied swictation-ui"
echo ""

# ============================================================================
# STEP 8: Copy ONNX Runtime Library
# ============================================================================
echo "📚 Step 8: Copying ONNX Runtime library..."

# Try to find libonnxruntime.so in known locations
ONNX_LIB=""
SEARCH_PATHS=(
    "$REPO_ROOT/docker/onnxruntime-builder/output/latest/libs/libonnxruntime.so"
    "$REPO_ROOT/docker/onnxruntime-builder/output/modern/libs/libonnxruntime.so"
    "$REPO_ROOT/npm-package/lib/native/libonnxruntime.so"
    "/usr/local/lib/libonnxruntime.so"
    "/usr/lib/x86_64-linux-gnu/libonnxruntime.so"
)

for path in "${SEARCH_PATHS[@]}"; do
    if [ -f "$path" ]; then
        ONNX_LIB="$path"
        echo "   Found libonnxruntime.so at: $path"
        break
    fi
done

if [ -z "$ONNX_LIB" ]; then
    echo "⚠️  Warning: libonnxruntime.so not found in standard locations"
    echo "   The library will need to be provided separately or downloaded during postinstall"
    echo "   Expected locations:"
    for path in "${SEARCH_PATHS[@]}"; do
        echo "     - $path"
    done
    echo ""
    echo "   Skipping library copy (GPU libraries will be downloaded by postinstall.js)"
else
    cp "$ONNX_LIB" "$PLATFORM_LIB_DIR/libonnxruntime.so"
    echo "✓ Copied libonnxruntime.so"
    echo "   Size: $(du -h "$PLATFORM_LIB_DIR/libonnxruntime.so" | cut -f1)"
fi
echo ""

# ============================================================================
# STEP 9: Generate Checksums
# ============================================================================
echo "🔐 Step 9: Generating checksums..."

CHECKSUMS_FILE="$PACKAGE_DIR/CHECKSUMS.txt"
{
    echo "# Linux x64 Build Checksums"
    echo "# Generated: $(date -u +"%Y-%m-%d %H:%M:%S UTC")"
    echo "# Platform: $(uname -s) $(uname -m)"
    echo "# Distribution: $DISTRIBUTION_VERSION"
    echo "# Daemon: $DAEMON_VERSION"
    echo "# UI: $UI_VERSION"
    echo ""
    echo "## Binaries"
    sha256sum "$PLATFORM_BIN_DIR/swictation-daemon" | sed "s|$PLATFORM_BIN_DIR/||"
    sha256sum "$PLATFORM_BIN_DIR/swictation-ui" | sed "s|$PLATFORM_BIN_DIR/||"

    if [ -f "$PLATFORM_LIB_DIR/libonnxruntime.so" ]; then
        echo ""
        echo "## Libraries"
        sha256sum "$PLATFORM_LIB_DIR/libonnxruntime.so" | sed "s|$PLATFORM_LIB_DIR/||"
    fi
} > "$CHECKSUMS_FILE"

echo "✓ Checksums saved to CHECKSUMS.txt"
cat "$CHECKSUMS_FILE"
echo ""

# ============================================================================
# STEP 10: Update package.json Metadata
# ============================================================================
echo "📝 Step 10: Updating package.json metadata..."

PACKAGE_JSON="$PACKAGE_DIR/package.json"
BUILD_DATE=$(date -u +"%Y-%m-%dT%H:%M:%S.000Z")

# Update metadata using jq if available, otherwise use sed
if command -v jq &> /dev/null; then
    # Use jq for clean JSON manipulation
    TMP_JSON=$(mktemp)
    jq --arg dist "$DISTRIBUTION_VERSION" \
       --arg daemon "$DAEMON_VERSION" \
       --arg ui "$UI_VERSION" \
       --arg onnx "$ONNX_VERSION" \
       --arg date "$BUILD_DATE" \
       '.version = $dist |
        .metadata.distribution = $dist |
        .metadata.daemon = $daemon |
        .metadata.ui = $ui |
        .metadata.onnxruntime = $onnx |
        .metadata.buildDate = $date' \
       "$PACKAGE_JSON" > "$TMP_JSON"
    mv "$TMP_JSON" "$PACKAGE_JSON"
    echo "✓ Updated package.json metadata with jq"
else
    # Fallback to sed (less reliable but works without jq)
    sed -i "s/\"version\": \"[^\"]*\"/\"version\": \"$DISTRIBUTION_VERSION\"/" "$PACKAGE_JSON"
    sed -i "s/\"distribution\": \"[^\"]*\"/\"distribution\": \"$DISTRIBUTION_VERSION\"/" "$PACKAGE_JSON"
    sed -i "s/\"daemon\": \"[^\"]*\"/\"daemon\": \"$DAEMON_VERSION\"/" "$PACKAGE_JSON"
    sed -i "s/\"ui\": \"[^\"]*\"/\"ui\": \"$UI_VERSION\"/" "$PACKAGE_JSON"
    sed -i "s/\"onnxruntime\": \"[^\"]*\"/\"onnxruntime\": \"$ONNX_VERSION\"/" "$PACKAGE_JSON"
    sed -i "s/\"buildDate\": null/\"buildDate\": \"$BUILD_DATE\"/" "$PACKAGE_JSON"
    sed -i "s/\"buildDate\": \"[^\"]*\"/\"buildDate\": \"$BUILD_DATE\"/" "$PACKAGE_JSON"
    echo "✓ Updated package.json metadata with sed"
fi

echo "   Build date: $BUILD_DATE"
echo ""

# ============================================================================
# STEP 11: Verify Binary Integrity
# ============================================================================
echo "🔍 Step 11: Verifying binary integrity..."

# Check that binaries are ELF 64-bit
DAEMON_FILE_TYPE=$(file "$PLATFORM_BIN_DIR/swictation-daemon")
UI_FILE_TYPE=$(file "$PLATFORM_BIN_DIR/swictation-ui")

if echo "$DAEMON_FILE_TYPE" | grep -q "ELF 64-bit"; then
    echo "✓ swictation-daemon: ELF 64-bit (correct format)"
else
    echo "❌ ERROR: swictation-daemon is not ELF 64-bit"
    echo "   File type: $DAEMON_FILE_TYPE"
    exit 1
fi

if echo "$UI_FILE_TYPE" | grep -q "ELF 64-bit"; then
    echo "✓ swictation-ui: ELF 64-bit (correct format)"
else
    echo "❌ ERROR: swictation-ui is not ELF 64-bit"
    echo "   File type: $UI_FILE_TYPE"
    exit 1
fi

# Check dynamic library dependencies
echo ""
echo "   Daemon dependencies:"
ldd "$PLATFORM_BIN_DIR/swictation-daemon" | head -10 | sed 's/^/     /'

echo ""
echo "   UI dependencies:"
ldd "$PLATFORM_BIN_DIR/swictation-ui" | head -10 | sed 's/^/     /'
echo ""

# ============================================================================
# STEP 12: Sanity Check - Test Binaries
# ============================================================================
echo "🧪 Step 12: Sanity check - testing binaries..."

if "$PLATFORM_BIN_DIR/swictation-daemon" --version > /dev/null 2>&1; then
    DAEMON_VER=$("$PLATFORM_BIN_DIR/swictation-daemon" --version 2>&1 | head -1)
    echo "✓ Daemon executes successfully: $DAEMON_VER"
else
    echo "⚠️  Warning: Daemon --version command failed"
    echo "   Binary may have missing dependencies"
fi

if "$PLATFORM_BIN_DIR/swictation-ui" --version > /dev/null 2>&1; then
    UI_VER=$("$PLATFORM_BIN_DIR/swictation-ui" --version 2>&1 | head -1)
    echo "✓ UI executes successfully: $UI_VER"
else
    echo "⚠️  Warning: UI --version command failed"
    echo "   Binary may have missing dependencies"
fi
echo ""

# ============================================================================
# STEP 13: Build Summary
# ============================================================================
echo "✅ Linux x64 platform package build complete!"
echo ""
echo "📦 Build artifacts in $PACKAGE_DIR:"
echo "   bin/"
ls -lh "$PLATFORM_BIN_DIR" | tail -n +2 | sed 's/^/     /'
echo ""
echo "   lib/"
if [ -d "$PLATFORM_LIB_DIR" ] && [ "$(ls -A "$PLATFORM_LIB_DIR")" ]; then
    ls -lh "$PLATFORM_LIB_DIR" | tail -n +2 | sed 's/^/     /'
else
    echo "     (empty - GPU libraries will be downloaded by postinstall.js)"
fi
echo ""
echo "📊 Package info:"
echo "   Distribution: $DISTRIBUTION_VERSION"
echo "   Daemon:       $DAEMON_VERSION"
echo "   UI:           $UI_VERSION"
echo "   ONNX Runtime: $ONNX_VERSION"
echo "   Build date:   $BUILD_DATE"
echo ""
echo "🚀 Next steps:"
echo "   1. Test package locally: cd $PACKAGE_DIR && npm pack"
echo "   2. Test installation: npm install -g $PACKAGE_DIR/*.tgz"
echo "   3. Verify binaries: swictation --version"
echo "   4. Run in CI/CD: This script is ready for GitHub Actions"
echo ""
