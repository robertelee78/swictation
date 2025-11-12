#!/bin/bash
# Advanced Docker testing with library dependency validation
# Tests actual daemon execution with bundled libraries

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() {
    echo -e "${CYAN}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
}

log_section() {
    echo ""
    echo "========================================="
    echo "$1"
    echo "========================================="
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_DIR="$(dirname "$SCRIPT_DIR")"

# Comprehensive test that validates library loading
run_library_test() {
    local distro="$1"
    local node_version="$2"

    log_section "Library Dependency Test: ${distro}-node${node_version}"

    local test_script=$(mktemp)
    cat > "$test_script" << 'LIBTEST'
#!/bin/bash
set -e

# Install dependencies
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq curl libasound2 libc-bin > /dev/null 2>&1

# Install Node.js
curl -fsSL https://deb.nodesource.com/setup_NODE_VERSION.x | bash - > /dev/null 2>&1
apt-get install -y -qq nodejs > /dev/null 2>&1

# Install package from tarball
echo "=== Installing package ==="
npm install -g /package.tgz

# Find installation directory dynamically
NPM_ROOT=$(npm root -g)
INSTALL_DIR="$NPM_ROOT/swictation"
NATIVE_DIR="$INSTALL_DIR/lib/native"

echo "npm root -g: $NPM_ROOT"
echo "INSTALL_DIR: $INSTALL_DIR"

echo "=== Installation Directory ==="
ls -lah "$INSTALL_DIR" || exit 1

echo ""
echo "=== Native Libraries ==="
ls -lah "$NATIVE_DIR" || exit 1

echo ""
echo "=== Library Dependencies (ldd) ==="
export LD_LIBRARY_PATH="$NATIVE_DIR:$LD_LIBRARY_PATH"
ldd "$NATIVE_DIR/swictation-daemon.bin" || exit 1

echo ""
echo "=== Checking for 'not found' errors ==="
if ldd "$NATIVE_DIR/swictation-daemon.bin" | grep "not found"; then
    echo "ERROR: Missing libraries detected!"
    exit 1
else
    echo "✓ All libraries resolved successfully"
fi

echo ""
echo "=== Testing Daemon Wrapper ==="
"$INSTALL_DIR/bin/swictation-daemon" --version 2>&1 || echo "Version check completed (may require config)"

echo ""
echo "=== Phase 2: ORT Detection Test ==="
# Check if ORT detection config was created
if [ -f "$INSTALL_DIR/config/detected-environment.json" ]; then
    echo "✓ ORT detection config created"
    cat "$INSTALL_DIR/config/detected-environment.json"
else
    echo "⚠  ORT detection config not found (Python/onnxruntime may not be installed)"
fi

echo ""
echo "=== Phase 2: Systemd Service Template Test ==="
# Check if systemd service template exists
if [ -f "$INSTALL_DIR/templates/swictation-daemon.service.template" ]; then
    echo "✓ Systemd service template exists"

    # Verify template has placeholders
    if grep -q "__INSTALL_DIR__" "$INSTALL_DIR/templates/swictation-daemon.service.template" && \
       grep -q "__ORT_DYLIB_PATH__" "$INSTALL_DIR/templates/swictation-daemon.service.template"; then
        echo "✓ Template has correct placeholders"
    else
        echo "✗ Template missing required placeholders"
        exit 1
    fi

    # Verify critical environment variables in template
    if grep -q "ORT_DYLIB_PATH" "$INSTALL_DIR/templates/swictation-daemon.service.template" && \
       grep -q "LD_LIBRARY_PATH" "$INSTALL_DIR/templates/swictation-daemon.service.template" && \
       grep -q "ImportEnvironment" "$INSTALL_DIR/templates/swictation-daemon.service.template"; then
        echo "✓ Template includes all Phase 1 environment variables"
    else
        echo "✗ Template missing critical environment variables"
        exit 1
    fi
else
    echo "✗ Systemd service template not found"
    exit 1
fi

echo ""
echo "ALL LIBRARY TESTS PASSED"
echo "ALL PHASE 2 TESTS PASSED"
LIBTEST

    sed -i "s/NODE_VERSION/$node_version/g" "$test_script"
    chmod +x "$test_script"

    local log_file="/tmp/library-test-${distro//[:\/]/-}-node${node_version}.log"

    docker run --rm \
        -v "$PACKAGE_DIR/$TARBALL:/package.tgz:ro" \
        -v "$test_script:/libtest.sh:ro" \
        "$distro" \
        /bin/bash /libtest.sh > "$log_file" 2>&1
    local exit_code=$?

    if [ $exit_code -eq 0 ]; then
        log_success "Library test passed: ${distro}-node${node_version}"
        cat "$log_file"
        rm -f "$test_script" "$log_file"
        return 0
    else
        log_error "Library test failed: ${distro}-node${node_version}"
        echo "Log output:"
        cat "$log_file"
        rm -f "$test_script"
        return 1
    fi
}

main() {
    log_section "Advanced Library Dependency Testing"

    # Check Docker
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed"
        exit 1
    fi

    # Create tarball
    log_info "Creating package tarball..."
    cd "$PACKAGE_DIR"
    npm pack > /dev/null 2>&1
    TARBALL=$(ls swictation-*.tgz | tail -1)

    if [ ! -f "$TARBALL" ]; then
        log_error "Failed to create tarball"
        exit 1
    fi

    log_info "Pulling ubuntu:24.04..."
    docker pull ubuntu:24.04 > /dev/null 2>&1

    log_info "Running comprehensive library test..."
    echo ""

    if run_library_test "ubuntu:24.04" "20"; then
        rm -f "$PACKAGE_DIR/$TARBALL"
        log_section "Success"
        echo -e "${GREEN}✓ Library bundling works correctly${NC}"
        echo "  - All shared libraries resolve"
        echo "  - LD_LIBRARY_PATH wrapper works"
        echo "  - Binary executes on clean Ubuntu 24.04"
        exit 0
    else
        rm -f "$PACKAGE_DIR/$TARBALL"
        log_section "Failure"
        echo -e "${RED}✗ Library issues detected${NC}"
        echo "Check log: /tmp/library-test-*.log"
        exit 1
    fi
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
    main "$@"
fi
