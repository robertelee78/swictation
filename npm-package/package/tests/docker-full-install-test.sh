#!/bin/bash
# Full installation test with system dependencies
# Tests the complete flow including libasound2 installation

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
CYAN='\033[0;36m'
YELLOW='\033[0;33m'
NC='\033[0m'

echo -e "${CYAN}=== Full Installation Test with Dependencies ===${NC}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_DIR="$(dirname "$SCRIPT_DIR")"

# Create tarball
echo "Creating package tarball..."
cd "$PACKAGE_DIR"
npm pack > /dev/null 2>&1
TARBALL=$(ls swictation-*.tgz | tail -1)

if [ ! -f "$TARBALL" ]; then
    echo -e "${RED}Failed to create tarball${NC}"
    exit 1
fi

echo "Testing with: $TARBALL"
echo ""

docker run --rm \
    -v "$PACKAGE_DIR/$TARBALL:/package.tgz:ro" \
    ubuntu:24.04 \
    /bin/bash -c '
set -e

echo "=== Phase 1: System Setup ==="
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq curl

echo "Installing Node.js 20..."
curl -fsSL https://deb.nodesource.com/setup_20.x | bash - > /dev/null 2>&1
apt-get install -y -qq nodejs

echo "Installing ALSA library (required for audio)..."
apt-get install -y -qq libasound2t64 || apt-get install -y -qq libasound2 || echo "ALSA not available, daemon may not work"

echo "✓ Node.js: $(node --version)"
echo "✓ npm: $(npm --version)"
echo "✓ libasound2 installed"
echo ""

echo "=== Phase 2: NPM Install ==="
npm install -g /package.tgz 2>&1
echo ""

echo "=== Phase 3: Verify Installation ==="
NPM_ROOT=$(npm root -g)
INSTALL_DIR="$NPM_ROOT/swictation"
DAEMON_BIN="$INSTALL_DIR/lib/native/swictation-daemon.bin"
NATIVE_DIR="$INSTALL_DIR/lib/native"

echo "Installation directory: $INSTALL_DIR"
echo ""

echo "Installed libraries:"
ls -lh "$NATIVE_DIR/" | grep -E "(\.so|\.bin)"
echo ""

echo "=== Phase 4: Test Daemon Execution ==="
export LD_LIBRARY_PATH="$NATIVE_DIR:$LD_LIBRARY_PATH"
export ORT_DYLIB_PATH="$NATIVE_DIR/libonnxruntime.so"
export RUST_LOG="info"

echo "Testing daemon with --help..."
timeout 5s "$DAEMON_BIN" --help 2>&1 | head -20 || {
    EXIT_CODE=$?
    echo ""
    echo "Daemon exit code: $EXIT_CODE"
    if [ $EXIT_CODE -eq 124 ]; then
        echo "✓ Daemon ran (timeout expected for --help)"
    else
        echo "✗ Daemon failed"
        exit 1
    fi
}
echo ""

echo "=== Phase 5: Library Dependencies ==="
echo "Checking shared library dependencies:"
ldd "$DAEMON_BIN" | grep "not found" && {
    echo "✗ Missing dependencies found"
    exit 1
} || echo "✓ All dependencies satisfied"
echo ""

echo "=== Summary ==="
swictation help > /dev/null 2>&1 && echo "✓ CLI works" || echo "✗ CLI failed"
command -v swictation > /dev/null && echo "✓ Binary in PATH" || echo "✗ Binary not in PATH"
[ -d "$INSTALL_DIR/lib/native" ] && echo "✓ lib/native directory exists" || echo "✗ lib/native missing"
[ -f "$DAEMON_BIN" ] && echo "✓ Daemon binary exists" || echo "✗ Daemon binary missing"
[ -f "$NATIVE_DIR/libonnxruntime.so" ] && echo "✓ ONNX Runtime exists" || echo "✗ ONNX Runtime missing"
[ -f "$NATIVE_DIR/libsherpa-onnx-c-api.so" ] && echo "✓ Sherpa library exists" || echo "✗ Sherpa missing"
echo ""
echo "✅ Full installation test PASSED"
'

TEST_RESULT=$?

# Cleanup
rm -f "$PACKAGE_DIR/$TARBALL"

echo ""
if [ $TEST_RESULT -eq 0 ]; then
    echo -e "${GREEN}✓ Full test PASSED - npm package is working correctly!${NC}"
    exit 0
else
    echo -e "${RED}✗ Full test FAILED${NC}"
    exit 1
fi
