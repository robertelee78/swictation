#!/bin/bash
# Debug postinstall script in clean Docker environment
# This tests the exact npm install flow including library loading

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
CYAN='\033[0;36m'
YELLOW='\033[0;33m'
NC='\033[0m'

echo -e "${CYAN}=== Postinstall Debug Test ===${NC}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_DIR="$(dirname "$SCRIPT_DIR")"

# Create tarball for testing
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

echo "✓ Node.js: $(node --version)"
echo "✓ npm: $(npm --version)"
echo ""

echo "=== Phase 2: NPM Install with --foreground-scripts ==="
npm install -g /package.tgz --foreground-scripts 2>&1 | tee /tmp/install-output.log
echo ""

echo "=== Phase 3: Analyze Installation ==="
echo "Postinstall output:"
grep -A 5 "postinstall" /tmp/install-output.log || echo "No postinstall marker found"
echo ""

# Find where npm actually installed (can be /usr/lib or /usr/local/lib)
NPM_ROOT=$(npm root -g)
INSTALL_DIR="$NPM_ROOT/swictation"
echo "npm root -g: $NPM_ROOT"
echo "Installation directory: $INSTALL_DIR"
echo ""

echo "Checking installed files:"
if [ -d "$INSTALL_DIR/lib/native" ]; then
    ls -lh "$INSTALL_DIR/lib/native/" | grep -E "(\.so|\.bin)" || echo "No binaries found"
else
    echo "lib/native directory not found"
fi
echo ""

echo "=== Phase 4: Library Loading Test ==="
DAEMON_BIN="$INSTALL_DIR/lib/native/swictation-daemon.bin"
NATIVE_DIR="$INSTALL_DIR/lib/native"

echo "Testing ldd without LD_LIBRARY_PATH:"
ldd "$DAEMON_BIN" 2>&1 | grep -E "(sherpa|onnx|not found)" || echo "All libraries found"
echo ""

echo "Testing ldd WITH LD_LIBRARY_PATH:"
export LD_LIBRARY_PATH="$NATIVE_DIR:$LD_LIBRARY_PATH"
ldd "$DAEMON_BIN" 2>&1 | grep -E "(sherpa|onnx|not found)" || echo "All libraries found"
echo ""

echo "=== Phase 5: Daemon Execution Test ==="
echo "Attempting to run daemon with proper environment:"
export ORT_DYLIB_PATH="$NATIVE_DIR/libonnxruntime.so"
export LD_LIBRARY_PATH="$NATIVE_DIR:$LD_LIBRARY_PATH"
export RUST_LOG="info"

# Try to run with a simple flag
timeout 5s "$DAEMON_BIN" --help 2>&1 | head -10 || {
    echo ""
    echo "ERROR: Daemon failed to execute"
    echo "Exit code: $?"
}
echo ""

echo "=== Phase 6: Check for GPU libs download attempt ==="
grep -i "gpu.*download\|gpu.*extract" /tmp/install-output.log || echo "No GPU download messages"
echo ""

echo "=== Summary ==="
swictation help > /dev/null 2>&1 && echo "✓ CLI works" || echo "✗ CLI failed"
command -v swictation > /dev/null && echo "✓ Binary in PATH" || echo "✗ Binary not in PATH"
[ -d "$INSTALL_DIR/lib/native" ] && echo "✓ lib/native directory exists" || echo "✗ lib/native directory missing"
[ -f "$DAEMON_BIN" ] && echo "✓ Daemon binary exists" || echo "✗ Daemon binary missing"
[ -f "$NATIVE_DIR/libonnxruntime.so" ] && echo "✓ ONNX Runtime exists" || echo "✗ ONNX Runtime missing"
[ -f "$NATIVE_DIR/libsherpa-onnx-c-api.so" ] && echo "✓ Sherpa library exists" || echo "✗ Sherpa library missing"
[ -f "$NATIVE_DIR/libonnxruntime_providers_cuda.so" ] && echo "✓ CUDA provider exists (GPU libs downloaded)" || echo "ℹ  CUDA provider missing (CPU-only or download failed)"
'

TEST_RESULT=$?

# Cleanup tarball
rm -f "$PACKAGE_DIR/$TARBALL"

echo ""
if [ $TEST_RESULT -eq 0 ]; then
    echo -e "${GREEN}✓ Debug test completed${NC}"
    exit 0
else
    echo -e "${RED}✗ Debug test failed${NC}"
    exit 1
fi
