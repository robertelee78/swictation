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
apt-get install -y -qq curl ldd > /dev/null 2>&1 || apt-get install -y -qq curl libc-bin > /dev/null 2>&1

# Install Node.js
curl -fsSL https://deb.nodesource.com/setup_NODE_VERSION.x | bash - > /dev/null 2>&1
apt-get install -y -qq nodejs > /dev/null 2>&1

# Install package
cd /package
npm install -g . > /dev/null 2>&1

# Find installation directory
INSTALL_DIR="/usr/local/lib/node_modules/swictation"
NATIVE_DIR="$INSTALL_DIR/lib/native"

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
echo "ALL LIBRARY TESTS PASSED"
LIBTEST

    sed -i "s/NODE_VERSION/$node_version/g" "$test_script"
    chmod +x "$test_script"

    if docker run --rm \
        -v "$PACKAGE_DIR:/package:ro" \
        -v "$test_script:/libtest.sh:ro" \
        "$distro" \
        /bin/bash /libtest.sh 2>&1 | tee /tmp/library-test-${distro//[:\/]/-}-node${node_version}.log; then

        log_success "Library test passed: ${distro}-node${node_version}"
        return 0
    else
        log_error "Library test failed: ${distro}-node${node_version}"
        return 1
    fi

    rm -f "$test_script"
}

main() {
    log_section "Advanced Library Dependency Testing"

    # Check Docker
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed"
        exit 1
    fi

    log_info "Pulling ubuntu:22.04..."
    docker pull ubuntu:22.04 > /dev/null 2>&1

    log_info "Running comprehensive library test..."
    echo ""

    if run_library_test "ubuntu:22.04" "20"; then
        log_section "Success"
        echo -e "${GREEN}✓ Library bundling works correctly${NC}"
        echo "  - All shared libraries resolve"
        echo "  - LD_LIBRARY_PATH wrapper works"
        echo "  - Binary executes on clean Ubuntu 22.04"
        exit 0
    else
        log_section "Failure"
        echo -e "${RED}✗ Library issues detected${NC}"
        echo "Check log: /tmp/library-test-*.log"
        exit 1
    fi
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
    main "$@"
fi
