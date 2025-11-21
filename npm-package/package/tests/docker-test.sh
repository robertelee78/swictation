#!/bin/bash
# Docker-based comprehensive npm package testing
# Tests installation across multiple distributions and Node.js versions

set -eo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# Test results tracking
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
FAILED_CONFIGS=()

log_info() {
    echo -e "${CYAN}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
    ((PASSED_TESTS++)) || true
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
    ((FAILED_TESTS++)) || true
}

log_section() {
    echo ""
    echo "========================================="
    echo "$1"
    echo "========================================="
}

# Get package directory (parent of tests/)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_DIR="$(dirname "$SCRIPT_DIR")"

# Test configurations: distro:node_version
# NOTE: Ubuntu 22.04 removed - GLIBC 2.35 too old (need 2.39+)
TEST_CONFIGS=(
    "ubuntu:24.04:18"
    "ubuntu:24.04:20"
    "ubuntu:24.04:22"
)

# Future: Test on newer Ubuntu versions
# "ubuntu:25.04:22"
# "ubuntu:25.10:22"

run_docker_test() {
    local distro="$1"
    local node_version="$2"
    local test_name="${distro}-node${node_version}"

    ((TOTAL_TESTS++)) || true

    log_section "Testing: $test_name"

    # Create temporary test script
    local test_script=$(mktemp)
    cat > "$test_script" << 'TESTEOF'
#!/bin/bash
set -e

# Install Node.js and npm
if [ -f /etc/debian_version ]; then
    # Ubuntu/Debian
    export DEBIAN_FRONTEND=noninteractive
    apt-get update -qq
    apt-get install -y -qq curl > /dev/null 2>&1

    # Install Node.js via NodeSource
    curl -fsSL https://deb.nodesource.com/setup_NODE_VERSION.x | bash - > /dev/null 2>&1
    apt-get install -y -qq nodejs > /dev/null 2>&1
elif [ -f /etc/arch-release ]; then
    # Arch Linux
    pacman -Sy --noconfirm nodejs npm > /dev/null 2>&1
fi

# Verify Node.js installation
node --version || exit 1
npm --version || exit 1

# Install package from tarball
npm install -g /package.tgz > /dev/null 2>&1 || exit 1

# Verify binaries are installed
command -v swictation || exit 1

# Test CLI help
swictation help > /dev/null 2>&1 || exit 1

# Test download-models command exists
swictation download-models 2>&1 | grep -q "hf CLI" || exit 1

# Get actual npm root and check installation
INSTALL_DIR=$(npm root -g)/swictation

# Verify library bundling
ls -la "$INSTALL_DIR/lib/native/" || exit 1

# Check daemon wrapper exists
[ -x "$INSTALL_DIR/bin/swictation-daemon" ] || exit 1

# Check actual binary exists
[ -x "$INSTALL_DIR/lib/native/swictation-daemon.bin" ] || exit 1

echo "ALL TESTS PASSED"
TESTEOF

    # Replace NODE_VERSION placeholder
    sed -i "s/NODE_VERSION/$node_version/g" "$test_script"
    chmod +x "$test_script"

    # Run test in Docker container
    if docker run --rm \
        -v "$PACKAGE_DIR/$TARBALL:/package.tgz:ro" \
        -v "$test_script:/test.sh:ro" \
        "$distro" \
        /bin/bash /test.sh > /tmp/docker-test-$test_name.log 2>&1; then

        log_success "$test_name"
        rm -f /tmp/docker-test-$test_name.log
    else
        log_error "$test_name - Check logs: /tmp/docker-test-$test_name.log"
        FAILED_CONFIGS+=("$test_name")
        cat /tmp/docker-test-$test_name.log
    fi

    rm -f "$test_script"
}

# Main execution
main() {
    log_section "Docker-based npm Package Testing"

    # Check Docker is available
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed or not in PATH"
        exit 1
    fi

    # Check Docker is running
    if ! docker ps &> /dev/null; then
        log_error "Docker daemon is not running"
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

    log_success "Created $TARBALL"

    log_info "Package directory: $PACKAGE_DIR"
    log_info "Testing ${#TEST_CONFIGS[@]} configurations..."
    echo ""

    # Pull base images first
    log_info "Pulling Docker images..."
    docker pull ubuntu:24.04 > /dev/null 2>&1
    log_success "Docker images ready"
    echo ""

    # Run all test configurations
    for config in "${TEST_CONFIGS[@]}"; do
        IFS=':' read -r distro version node_version <<< "$config"
        run_docker_test "${distro}:${version}" "$node_version"
    done

    # Cleanup tarball
    rm -f "$PACKAGE_DIR/$TARBALL"

    # Print summary
    log_section "Test Summary"
    echo "Total configurations tested: $TOTAL_TESTS"
    echo -e "${GREEN}Passed: $PASSED_TESTS${NC}"
    echo -e "${RED}Failed: $FAILED_TESTS${NC}"
    echo ""

    if [ ${#FAILED_CONFIGS[@]} -gt 0 ]; then
        echo -e "${RED}Failed configurations:${NC}"
        for config in "${FAILED_CONFIGS[@]}"; do
            echo "  - $config"
        done
        echo ""
        exit 1
    else
        echo -e "${GREEN}✓ All tests passed!${NC}"
        echo ""
        echo "Package is ready for npm publish"
        exit 0
    fi
}

# Run if executed directly
if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
    main "$@"
fi
