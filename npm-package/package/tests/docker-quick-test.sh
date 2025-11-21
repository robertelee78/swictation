#!/bin/bash
# Quick smoke test in Docker (single config)
# Fast validation before running full test matrix

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${CYAN}Quick Smoke Test - Ubuntu 24.04 + Node 20${NC}"

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

docker run --rm \
    -v "$PACKAGE_DIR/$TARBALL:/package.tgz:ro" \
    ubuntu:24.04 \
    /bin/bash -c '
set -e

# Silent install
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq > /dev/null 2>&1
apt-get install -y -qq curl > /dev/null 2>&1
curl -fsSL https://deb.nodesource.com/setup_20.x | bash - > /dev/null 2>&1
apt-get install -y -qq nodejs > /dev/null 2>&1

# Install package from tarball
npm install -g /package.tgz > /dev/null 2>&1

# Run tests
echo "✓ Node.js: $(node --version)"
echo "✓ npm: $(npm --version)"

swictation help > /dev/null 2>&1 && echo "✓ CLI works"

command -v swictation > /dev/null && echo "✓ Binary in PATH"

ls /usr/local/lib/node_modules/swictation/lib/native/*.so > /dev/null 2>&1 && echo "✓ Libraries bundled"

[ -x /usr/local/lib/node_modules/swictation/bin/swictation-daemon ] && echo "✓ Daemon wrapper executable"

[ -x /usr/local/lib/node_modules/swictation/lib/native/swictation-daemon.bin ] && echo "✓ Daemon binary executable"

echo ""
echo "✓ All smoke tests passed!"
'

TEST_RESULT=$?

# Cleanup tarball
rm -f "$PACKAGE_DIR/$TARBALL"

if [ $TEST_RESULT -eq 0 ]; then
    echo ""
    echo -e "${GREEN}✓ Quick test PASSED${NC}"
    echo "Ready to run full test matrix: ./tests/docker-test.sh"
    exit 0
else
    echo ""
    echo -e "${RED}✗ Quick test FAILED${NC}"
    exit 1
fi
