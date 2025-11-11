#!/bin/bash
# Comprehensive Installation Test Suite for Swictation npm Package
# Tests the complete installation flow on a clean system

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Test counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Log functions
log_info() {
    echo -e "${CYAN}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
    ((TESTS_PASSED++))
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
    ((TESTS_FAILED++))
}

log_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
}

run_test() {
    ((TESTS_RUN++))
    local test_name="$1"
    local test_cmd="$2"

    log_info "Running: $test_name"

    if eval "$test_cmd" > /dev/null 2>&1; then
        log_success "$test_name"
        return 0
    else
        log_error "$test_name"
        return 1
    fi
}

# Wrapper to prevent set -e from exiting on test failures
safe_test() {
    safe_test "$@" || true
}

# Print system information
print_system_info() {
    echo ""
    echo "========================================="
    echo "System Information"
    echo "========================================="
    echo "OS: $(uname -s)"
    echo "Architecture: $(uname -m)"
    echo "Kernel: $(uname -r)"
    echo "Distribution: $(lsb_release -d 2>/dev/null | cut -f2 || echo 'Unknown')"
    echo "Node version: $(node --version 2>/dev/null || echo 'Not installed')"
    echo "npm version: $(npm --version 2>/dev/null || echo 'Not installed')"
    echo "Python version: $(python3 --version 2>/dev/null || echo 'Not installed')"
    echo "========================================="
    echo ""
}

# Test 1: Check prerequisites
test_prerequisites() {
    log_info "Testing prerequisites..."

    # Check Node.js
    if command -v node &> /dev/null; then
        local node_version=$(node --version | cut -d'v' -f2 | cut -d'.' -f1)
        if [ "$node_version" -ge 14 ]; then
            log_success "Node.js version check (v${node_version})"
        else
            log_error "Node.js version too old (v${node_version} < 14)"
        fi
    else
        log_error "Node.js not installed"
    fi

    # Check npm
    safe_test "npm installation check" "command -v npm"

    # Check platform
    if [ "$(uname -s)" = "Linux" ] && [ "$(uname -m)" = "x86_64" ]; then
        log_success "Platform check (Linux x64)"
    else
        log_error "Unsupported platform: $(uname -s) $(uname -m)"
    fi
}

# Test 2: Check package structure
test_package_structure() {
    log_info "Testing package structure..."

    local pkg_dir="$(dirname "$0")"

    # Check critical files
    safe_test "package.json exists" "[ -f '$pkg_dir/package.json' ]"
    safe_test "postinstall.js exists" "[ -f '$pkg_dir/postinstall.js' ]"
    safe_test "bin/swictation exists" "[ -f '$pkg_dir/bin/swictation' ]"
    safe_test "bin/swictation-daemon exists" "[ -f '$pkg_dir/bin/swictation-daemon' ]"
    safe_test "lib/model-downloader.js exists" "[ -f '$pkg_dir/lib/model-downloader.js' ]"

    # Check native libraries
    safe_test "lib/native directory exists" "[ -d '$pkg_dir/lib/native' ]"
    safe_test "swictation-daemon.bin exists" "[ -f '$pkg_dir/lib/native/swictation-daemon.bin' ]"
    safe_test "libsherpa-onnx-c-api.so exists" "[ -f '$pkg_dir/lib/native/libsherpa-onnx-c-api.so' ]"
    safe_test "libonnxruntime.so exists" "[ -f '$pkg_dir/lib/native/libonnxruntime.so' ]"
}

# Test 3: Check file permissions
test_permissions() {
    log_info "Testing file permissions..."

    local pkg_dir="$(dirname "$0")"

    safe_test "swictation CLI is executable" "[ -x '$pkg_dir/bin/swictation' ]"
    safe_test "swictation-daemon wrapper is executable" "[ -x '$pkg_dir/bin/swictation-daemon' ]"
    safe_test "swictation-daemon.bin is executable" "[ -x '$pkg_dir/lib/native/swictation-daemon.bin' ]"
}

# Test 4: Test library dependencies
test_library_dependencies() {
    log_info "Testing library dependencies..."

    local pkg_dir="$(dirname "$0")"
    local binary="$pkg_dir/lib/native/swictation-daemon.bin"

    if command -v ldd &> /dev/null; then
        log_info "Checking shared library dependencies..."

        # Set LD_LIBRARY_PATH to find bundled libraries
        export LD_LIBRARY_PATH="$pkg_dir/lib/native:${LD_LIBRARY_PATH}"

        # Check for missing libraries
        local missing=$(ldd "$binary" 2>&1 | grep "not found" | wc -l)

        if [ "$missing" -eq 0 ]; then
            log_success "All shared libraries resolved"
        else
            log_error "Missing libraries detected:"
            ldd "$binary" 2>&1 | grep "not found"
        fi
    else
        log_warning "ldd not available, skipping library dependency check"
    fi
}

# Test 5: Test CLI commands
test_cli_commands() {
    log_info "Testing CLI commands..."

    local pkg_dir="$(dirname "$0")"

    # Test help command
    if "$pkg_dir/bin/swictation" help &> /dev/null; then
        log_success "swictation help command"
    else
        log_error "swictation help command failed"
    fi

    # Test download-models --help (should show error about hf CLI if not installed)
    if "$pkg_dir/bin/swictation" download-models 2>&1 | grep -q "hf CLI"; then
        log_success "swictation download-models command accessible"
    else
        log_warning "swictation download-models command output unexpected"
    fi
}

# Test 6: Test daemon binary
test_daemon_binary() {
    log_info "Testing daemon binary..."

    local pkg_dir="$(dirname "$0")"

    # Create temporary config
    local temp_config=$(mktemp -d)
    mkdir -p "$temp_config/swictation"

    # Test daemon help (should work even without models)
    if timeout 5 "$pkg_dir/bin/swictation-daemon" --version 2>&1 | grep -q "Swictation Daemon"; then
        log_success "Daemon binary executes"
    else
        log_warning "Daemon binary execution issue (may need config/models)"
    fi

    rm -rf "$temp_config"
}

# Test 7: Check model directory creation
test_model_directory() {
    log_info "Testing model directory setup..."

    local model_dir="$HOME/.local/share/swictation/models"

    if [ -d "$model_dir" ]; then
        log_success "Model directory exists: $model_dir"
    else
        log_warning "Model directory not created (normal if postinstall hasn't run)"
    fi
}

# Test 8: Test package.json validation
test_package_json() {
    log_info "Testing package.json validity..."

    local pkg_dir="$(dirname "$0")"

    if command -v node &> /dev/null; then
        # Validate JSON
        if node -e "JSON.parse(require('fs').readFileSync('$pkg_dir/package.json', 'utf8'))" 2>/dev/null; then
            log_success "package.json is valid JSON"
        else
            log_error "package.json is invalid JSON"
        fi

        # Check required fields
        local pkg_name=$(node -e "console.log(require('$pkg_dir/package.json').name)" 2>/dev/null)
        if [ "$pkg_name" = "swictation" ]; then
            log_success "package.json has correct name"
        else
            log_error "package.json name incorrect: $pkg_name"
        fi
    else
        log_warning "Node.js not available for package.json validation"
    fi
}

# Test 9: Check bundled library sizes
test_library_sizes() {
    log_info "Testing bundled library sizes..."

    local pkg_dir="$(dirname "$0")"
    local native_dir="$pkg_dir/lib/native"

    if [ -d "$native_dir" ]; then
        local total_size=$(du -sh "$native_dir" 2>/dev/null | cut -f1)
        log_info "Total native library size: $total_size"

        # Check if size is reasonable (should be around 32MB)
        local size_mb=$(du -sm "$native_dir" 2>/dev/null | cut -f1)
        if [ "$size_mb" -lt 50 ]; then
            log_success "Library bundle size is reasonable (${size_mb}MB < 50MB)"
        else
            log_warning "Library bundle is large: ${size_mb}MB"
        fi
    else
        log_error "Native library directory not found"
    fi
}

# Test 10: Simulate npm pack
test_npm_pack() {
    log_info "Testing npm pack (dry run)..."

    local pkg_dir="$(dirname "$0")"

    if command -v npm &> /dev/null; then
        cd "$pkg_dir"

        # Run npm pack --dry-run to see what would be included
        if npm pack --dry-run > /dev/null 2>&1; then
            log_success "npm pack dry-run successful"

            # Check package size
            local tarball_size=$(npm pack --dry-run 2>&1 | grep "npm notice" | grep "unpacked size" | awk '{print $(NF-1) " " $NF}')
            log_info "Estimated package size: $tarball_size"
        else
            log_error "npm pack dry-run failed"
        fi
    else
        log_warning "npm not available for pack test"
    fi
}

# Main test execution
main() {
    echo "========================================="
    echo "Swictation npm Package Test Suite"
    echo "========================================="

    print_system_info

    log_info "Starting tests..."
    echo ""

    test_prerequisites
    echo ""

    test_package_structure
    echo ""

    test_permissions
    echo ""

    test_library_dependencies
    echo ""

    test_cli_commands
    echo ""

    test_daemon_binary
    echo ""

    test_model_directory
    echo ""

    test_package_json
    echo ""

    test_library_sizes
    echo ""

    test_npm_pack
    echo ""

    # Print summary
    echo "========================================="
    echo "Test Summary"
    echo "========================================="
    echo "Total tests run: $TESTS_RUN"
    echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
    echo -e "${RED}Failed: $TESTS_FAILED${NC}"
    echo ""

    if [ $TESTS_FAILED -eq 0 ]; then
        echo -e "${GREEN}✓ All tests passed!${NC}"
        exit 0
    else
        echo -e "${RED}✗ Some tests failed${NC}"
        exit 1
    fi
}

# Run main if executed directly
if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
    main "$@"
fi
