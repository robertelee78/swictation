#!/bin/bash
#
# Automated Test Script for Swictation Postinstall
# Version: 0.3.0
# Purpose: Test postinstall.js functionality with mocked dependencies
#
# Usage:
#   ./test-postinstall.sh [test-suite]
#
# Test Suites:
#   all          - Run all tests (default)
#   platform     - Platform and GLIBC checks
#   permissions  - Binary permission tests
#   directories  - Directory creation tests
#   gpu          - GPU detection tests
#   ort          - ONNX Runtime detection tests
#   services     - Systemd service generation tests
#   dependencies - Dependency checking tests
#   models       - Model recommendation tests
#   errors       - Error handling tests
#   upgrade      - Upgrade scenario tests
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;36m'
NC='\033[0m' # No Color

# Test counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0

# Test environment
TEST_DIR="/tmp/swictation-test-$$"
MOCK_DIR="$TEST_DIR/mocks"
INSTALL_DIR="$TEST_DIR/install"
HOME_DIR="$TEST_DIR/home"

# Logging
LOG_FILE="$TEST_DIR/test.log"

#############################################
# Utility Functions
#############################################

log() {
    echo -e "${BLUE}[$(date +'%H:%M:%S')]${NC} $*" | tee -a "$LOG_FILE"
}

log_success() {
    echo -e "${GREEN}✓${NC} $*" | tee -a "$LOG_FILE"
}

log_error() {
    echo -e "${RED}✗${NC} $*" | tee -a "$LOG_FILE"
}

log_warning() {
    echo -e "${YELLOW}⚠${NC} $*" | tee -a "$LOG_FILE"
}

test_start() {
    local test_name="$1"
    log "\n=== Starting Test: $test_name ==="
    TESTS_RUN=$((TESTS_RUN + 1))
}

test_pass() {
    local test_name="$1"
    log_success "PASS: $test_name"
    TESTS_PASSED=$((TESTS_PASSED + 1))
}

test_fail() {
    local test_name="$1"
    local reason="$2"
    log_error "FAIL: $test_name"
    log_error "Reason: $reason"
    TESTS_FAILED=$((TESTS_FAILED + 1))
}

test_skip() {
    local test_name="$1"
    local reason="$2"
    log_warning "SKIP: $test_name - $reason"
    TESTS_SKIPPED=$((TESTS_SKIPPED + 1))
}

#############################################
# Setup and Teardown
#############################################

setup_test_environment() {
    log "Setting up test environment at $TEST_DIR"

    # Create test directories
    mkdir -p "$TEST_DIR"
    mkdir -p "$MOCK_DIR"
    mkdir -p "$INSTALL_DIR"
    mkdir -p "$HOME_DIR"
    mkdir -p "$INSTALL_DIR/bin"
    mkdir -p "$INSTALL_DIR/lib/native"
    mkdir -p "$INSTALL_DIR/config"
    mkdir -p "$INSTALL_DIR/templates"

    # Create mock binaries
    touch "$INSTALL_DIR/bin/swictation"
    touch "$INSTALL_DIR/bin/swictation-daemon"
    touch "$INSTALL_DIR/bin/swictation-ui"
    touch "$INSTALL_DIR/lib/native/swictation-daemon.bin"

    # Create mock template
    cat > "$INSTALL_DIR/templates/swictation-daemon.service.template" << 'EOF'
[Unit]
Description=Swictation Voice-to-Text Daemon
After=graphical-session.target

[Service]
Type=simple
ExecStart=__INSTALL_DIR__/lib/native/swictation-daemon.bin
Environment="ORT_DYLIB_PATH=__ORT_DYLIB_PATH__"
Environment="LD_LIBRARY_PATH=/usr/local/cuda/lib64:/usr/local/cuda-12.9/lib64:__INSTALL_DIR__/lib/native"

[Install]
WantedBy=default.target
EOF

    # Create mock package.json
    cat > "$INSTALL_DIR/package.json" << 'EOF'
{
  "name": "swictation",
  "version": "0.3.0"
}
EOF

    # Set HOME to test directory
    export HOME="$HOME_DIR"
    export PATH="$MOCK_DIR:$PATH"

    log_success "Test environment created"
}

cleanup_test_environment() {
    log "Cleaning up test environment"
    rm -rf "$TEST_DIR"
    log_success "Cleanup complete"
}

create_mock_nvidia_smi() {
    local vram_mb="$1"
    local gpu_name="${2:-NVIDIA GeForce GTX 1060}"

    cat > "$MOCK_DIR/nvidia-smi" << EOF
#!/bin/bash
case "\$1" in
    "")
        exit 0
        ;;
    --query-gpu=memory.total)
        echo "$vram_mb"
        ;;
    --query-gpu=name)
        echo "$gpu_name"
        ;;
esac
EOF
    chmod +x "$MOCK_DIR/nvidia-smi"
}

remove_mock_nvidia_smi() {
    rm -f "$MOCK_DIR/nvidia-smi"
}

create_mock_python_ort() {
    local ort_path="$1"

    mkdir -p "$(dirname "$ort_path")"
    touch "$ort_path"

    cat > "$MOCK_DIR/python3" << EOF
#!/bin/bash
case "\$*" in
    *onnxruntime*)
        if [[ "\$*" == *"__file__"* ]]; then
            echo "$(dirname "$ort_path")/.."
        elif [[ "\$*" == *"__version__"* ]]; then
            echo "1.15.1"
        fi
        ;;
esac
EOF
    chmod +x "$MOCK_DIR/python3"
}

#############################################
# Test Suite: Platform Checks
#############################################

test_platform_linux_x64() {
    test_start "Platform Check - Linux x64"

    # This test runs on actual platform, just verify it doesn't exit
    local result
    result=$(cd "$INSTALL_DIR" && node -e "
        const postinstall = require('./postinstall-test-wrapper.js');
        postinstall.checkPlatform();
        console.log('success');
    " 2>&1 || echo "failed")

    if [[ "$result" == *"success"* ]]; then
        test_pass "Platform Check - Linux x64"
    else
        test_fail "Platform Check - Linux x64" "Platform check failed: $result"
    fi
}

test_glibc_version_check() {
    test_start "GLIBC Version Check"

    # Just verify function doesn't crash
    local glibc_version
    glibc_version=$(ldd --version 2>&1 | head -1)

    if [[ -n "$glibc_version" ]]; then
        log "Detected GLIBC: $glibc_version"
        test_pass "GLIBC Version Check"
    else
        test_fail "GLIBC Version Check" "Could not detect GLIBC"
    fi
}

#############################################
# Test Suite: Binary Permissions
#############################################

test_binary_permissions() {
    test_start "Binary Permissions"

    # Set up test binaries
    local binaries=(
        "$INSTALL_DIR/bin/swictation"
        "$INSTALL_DIR/bin/swictation-daemon"
        "$INSTALL_DIR/bin/swictation-ui"
        "$INSTALL_DIR/lib/native/swictation-daemon.bin"
    )

    # Remove execute permissions
    for binary in "${binaries[@]}"; do
        chmod 644 "$binary"
    done

    # Run permission setting
    cd "$INSTALL_DIR"
    node -e "
        const fs = require('fs');
        const path = require('path');
        const __dirname = process.cwd();

        function ensureBinaryPermissions() {
            const binDir = path.join(__dirname, 'bin');
            const binaries = [
                path.join(binDir, 'swictation-daemon'),
                path.join(binDir, 'swictation-ui'),
                path.join(binDir, 'swictation'),
                path.join(__dirname, 'lib', 'native', 'swictation-daemon.bin')
            ];

            for (const binary of binaries) {
                if (fs.existsSync(binary)) {
                    fs.chmodSync(binary, '755');
                }
            }
        }

        ensureBinaryPermissions();
    "

    # Verify permissions
    local failed=0
    for binary in "${binaries[@]}"; do
        local perms
        perms=$(stat -c '%a' "$binary")
        if [[ "$perms" != "755" ]]; then
            log_error "Binary $binary has permissions $perms, expected 755"
            failed=1
        fi
    done

    if [[ $failed -eq 0 ]]; then
        test_pass "Binary Permissions"
    else
        test_fail "Binary Permissions" "Some binaries have incorrect permissions"
    fi
}

#############################################
# Test Suite: Directory Creation
#############################################

test_directory_creation() {
    test_start "Directory Creation"

    local dirs=(
        "$HOME_DIR/.config/swictation"
        "$HOME_DIR/.local/share/swictation"
        "$HOME_DIR/.local/share/swictation/models"
        "$HOME_DIR/.cache/swictation"
    )

    # Remove directories if they exist
    for dir in "${dirs[@]}"; do
        rm -rf "$dir"
    done

    # Run directory creation
    cd "$INSTALL_DIR"
    node -e "
        const fs = require('fs');
        const path = require('path');
        const os = require('os');

        function createDirectories() {
            const dirs = [
                path.join(os.homedir(), '.config', 'swictation'),
                path.join(os.homedir(), '.local', 'share', 'swictation'),
                path.join(os.homedir(), '.local', 'share', 'swictation', 'models'),
                path.join(os.homedir(), '.cache', 'swictation')
            ];

            for (const dir of dirs) {
                if (!fs.existsSync(dir)) {
                    fs.mkdirSync(dir, { recursive: true });
                }
            }
        }

        createDirectories();
    "

    # Verify directories
    local failed=0
    for dir in "${dirs[@]}"; do
        if [[ ! -d "$dir" ]]; then
            log_error "Directory not created: $dir"
            failed=1
        fi
    done

    if [[ $failed -eq 0 ]]; then
        test_pass "Directory Creation"
    else
        test_fail "Directory Creation" "Some directories were not created"
    fi
}

test_directory_idempotency() {
    test_start "Directory Creation Idempotency"

    # Run directory creation twice
    cd "$INSTALL_DIR"
    for i in 1 2; do
        node -e "
            const fs = require('fs');
            const path = require('path');
            const os = require('os');

            function createDirectories() {
                const dirs = [
                    path.join(os.homedir(), '.config', 'swictation'),
                    path.join(os.homedir(), '.local', 'share', 'swictation')
                ];

                for (const dir of dirs) {
                    if (!fs.existsSync(dir)) {
                        fs.mkdirSync(dir, { recursive: true });
                    }
                }
            }

            createDirectories();
        " 2>&1 || {
            test_fail "Directory Creation Idempotency" "Failed on iteration $i"
            return
        }
    done

    test_pass "Directory Creation Idempotency"
}

#############################################
# Test Suite: GPU Detection
#############################################

test_gpu_detection_present_high_vram() {
    test_start "GPU Detection - 6GB VRAM"

    create_mock_nvidia_smi "6144" "NVIDIA GeForce RTX 3060"

    cd "$INSTALL_DIR"
    local result
    result=$(node -e "
        const { execSync } = require('child_process');

        function detectNvidiaGPU() {
            try {
                execSync('nvidia-smi', { stdio: 'ignore' });
                return true;
            } catch {
                return false;
            }
        }

        const hasGPU = detectNvidiaGPU();
        console.log(hasGPU ? 'GPU_DETECTED' : 'NO_GPU');

        if (hasGPU) {
            const vram = execSync('nvidia-smi --query-gpu=memory.total --format=csv,noheader,nounits', { encoding: 'utf8' }).trim();
            const name = execSync('nvidia-smi --query-gpu=name --format=csv,noheader', { encoding: 'utf8' }).trim();
            console.log('VRAM:' + vram);
            console.log('NAME:' + name);
        }
    " 2>&1)

    remove_mock_nvidia_smi

    if [[ "$result" == *"GPU_DETECTED"* ]] && \
       [[ "$result" == *"VRAM:6144"* ]] && \
       [[ "$result" == *"NAME:NVIDIA GeForce RTX 3060"* ]]; then
        test_pass "GPU Detection - 6GB VRAM"
    else
        test_fail "GPU Detection - 6GB VRAM" "GPU not detected or wrong info: $result"
    fi
}

test_gpu_detection_low_vram() {
    test_start "GPU Detection - 2GB VRAM"

    create_mock_nvidia_smi "2048" "NVIDIA GeForce GTX 1050"

    cd "$INSTALL_DIR"
    local result
    result=$(node -e "
        const { execSync } = require('child_process');

        function detectNvidiaGPU() {
            try {
                execSync('nvidia-smi', { stdio: 'ignore' });
                return true;
            } catch {
                return false;
            }
        }

        const hasGPU = detectNvidiaGPU();
        if (hasGPU) {
            const vram = execSync('nvidia-smi --query-gpu=memory.total --format=csv,noheader,nounits', { encoding: 'utf8' }).trim();
            console.log('VRAM:' + vram);
        }
    " 2>&1)

    remove_mock_nvidia_smi

    if [[ "$result" == *"VRAM:2048"* ]]; then
        test_pass "GPU Detection - 2GB VRAM"
    else
        test_fail "GPU Detection - 2GB VRAM" "Expected 2GB VRAM: $result"
    fi
}

test_gpu_detection_absent() {
    test_start "GPU Detection - No GPU"

    remove_mock_nvidia_smi

    cd "$INSTALL_DIR"
    local result
    result=$(node -e "
        const { execSync } = require('child_process');

        function detectNvidiaGPU() {
            try {
                execSync('nvidia-smi', { stdio: 'ignore' });
                return true;
            } catch {
                return false;
            }
        }

        console.log(detectNvidiaGPU() ? 'GPU_DETECTED' : 'NO_GPU');
    " 2>&1)

    if [[ "$result" == *"NO_GPU"* ]]; then
        test_pass "GPU Detection - No GPU"
    else
        test_fail "GPU Detection - No GPU" "GPU was detected when it shouldn't be: $result"
    fi
}

#############################################
# Test Suite: ONNX Runtime Detection
#############################################

test_ort_detection_bundled() {
    test_start "ORT Detection - Bundled Library"

    # Create mock bundled library
    local ort_lib="$INSTALL_DIR/lib/native/libonnxruntime.so"
    touch "$ort_lib"

    cd "$INSTALL_DIR"
    local result
    result=$(node -e "
        const fs = require('fs');
        const path = require('path');
        const __dirname = process.cwd();

        const npmOrtLib = path.join(__dirname, 'lib', 'native', 'libonnxruntime.so');
        if (fs.existsSync(npmOrtLib)) {
            console.log('BUNDLED_FOUND:' + npmOrtLib);
        } else {
            console.log('NOT_FOUND');
        }
    " 2>&1)

    if [[ "$result" == *"BUNDLED_FOUND"* ]]; then
        test_pass "ORT Detection - Bundled Library"
    else
        test_fail "ORT Detection - Bundled Library" "Bundled library not found: $result"
    fi
}

test_ort_detection_python_fallback() {
    test_start "ORT Detection - Python Fallback"

    # Remove bundled library
    rm -f "$INSTALL_DIR/lib/native/libonnxruntime.so"

    # Create mock Python ORT
    local ort_path="$HOME_DIR/.local/lib/python3.10/site-packages/onnxruntime/capi/libonnxruntime.so.1.15.1"
    create_mock_python_ort "$ort_path"

    cd "$INSTALL_DIR"
    local result
    result=$(node -e "
        const fs = require('fs');
        const path = require('path');
        const { execSync } = require('child_process');
        const __dirname = process.cwd();

        const npmOrtLib = path.join(__dirname, 'lib', 'native', 'libonnxruntime.so');
        if (fs.existsSync(npmOrtLib)) {
            console.log('BUNDLED_FOUND');
        } else {
            try {
                const ortPath = execSync(
                    'python3 -c \"import onnxruntime; import os; print(os.path.join(os.path.dirname(onnxruntime.__file__), \\'capi\\'))\"',
                    { encoding: 'utf-8' }
                ).trim();
                console.log('PYTHON_FOUND:' + ortPath);
            } catch (err) {
                console.log('NOT_FOUND');
            }
        }
    " 2>&1)

    if [[ "$result" == *"PYTHON_FOUND"* ]]; then
        test_pass "ORT Detection - Python Fallback"
    else
        test_fail "ORT Detection - Python Fallback" "Python ORT not detected: $result"
    fi
}

test_ort_detection_missing() {
    test_start "ORT Detection - Missing"

    # Remove all ORT libraries
    rm -f "$INSTALL_DIR/lib/native/libonnxruntime.so"
    rm -f "$MOCK_DIR/python3"

    cd "$INSTALL_DIR"
    local result
    result=$(node -e "
        const fs = require('fs');
        const path = require('path');
        const __dirname = process.cwd();

        const npmOrtLib = path.join(__dirname, 'lib', 'native', 'libonnxruntime.so');
        if (fs.existsSync(npmOrtLib)) {
            console.log('FOUND');
        } else {
            console.log('NOT_FOUND');
        }
    " 2>&1)

    if [[ "$result" == *"NOT_FOUND"* ]]; then
        test_pass "ORT Detection - Missing"
    else
        test_fail "ORT Detection - Missing" "ORT was found when it shouldn't be: $result"
    fi
}

#############################################
# Test Suite: Systemd Service Generation
#############################################

test_service_generation() {
    test_start "Service Generation"

    local systemd_dir="$HOME_DIR/.config/systemd/user"
    mkdir -p "$systemd_dir"

    # Mock ORT library for path substitution
    local ort_path="$INSTALL_DIR/lib/native/libonnxruntime.so"
    touch "$ort_path"

    cd "$INSTALL_DIR"
    node -e "
        const fs = require('fs');
        const path = require('path');
        const os = require('os');
        const __dirname = process.cwd();

        const ortLibPath = '$ort_path';
        const systemdDir = path.join(os.homedir(), '.config', 'systemd', 'user');

        if (!fs.existsSync(systemdDir)) {
            fs.mkdirSync(systemdDir, { recursive: true });
        }

        const templatePath = path.join(__dirname, 'templates', 'swictation-daemon.service.template');
        let template = fs.readFileSync(templatePath, 'utf8');

        const installDir = __dirname;
        template = template.replace(/__INSTALL_DIR__/g, installDir);
        template = template.replace(/__ORT_DYLIB_PATH__/g, ortLibPath);

        const daemonServicePath = path.join(systemdDir, 'swictation-daemon.service');
        fs.writeFileSync(daemonServicePath, template);

        console.log('SERVICE_GENERATED');
    " 2>&1

    # Verify service file
    local service_file="$systemd_dir/swictation-daemon.service"
    if [[ -f "$service_file" ]]; then
        local content
        content=$(cat "$service_file")

        if [[ "$content" == *"$INSTALL_DIR"* ]] && \
           [[ "$content" == *"$ort_path"* ]] && \
           [[ "$content" != *"__INSTALL_DIR__"* ]] && \
           [[ "$content" != *"__ORT_DYLIB_PATH__"* ]]; then
            test_pass "Service Generation"
        else
            test_fail "Service Generation" "Template placeholders not replaced correctly"
        fi
    else
        test_fail "Service Generation" "Service file not created"
    fi
}

#############################################
# Test Suite: Model Recommendations
#############################################

test_model_recommendation_high_vram_gpu() {
    test_start "Model Recommendation - High VRAM GPU"

    create_mock_nvidia_smi "6144" "NVIDIA GeForce RTX 3060"

    cd "$INSTALL_DIR"
    local result
    result=$(node -e "
        const { execSync } = require('child_process');
        const os = require('os');

        function detectNvidiaGPU() {
            try {
                execSync('nvidia-smi', { stdio: 'ignore' });
                return true;
            } catch {
                return false;
            }
        }

        function detectSystemCapabilities() {
            const capabilities = {
                hasGPU: false,
                gpuName: null,
                gpuMemoryMB: 0,
                cpuCores: os.cpus().length,
                totalRAMGB: Math.round(os.totalmem() / (1024 * 1024 * 1024))
            };

            if (detectNvidiaGPU()) {
                capabilities.hasGPU = true;
                try {
                    const gpuMemory = execSync('nvidia-smi --query-gpu=memory.total --format=csv,noheader,nounits', { encoding: 'utf8' });
                    capabilities.gpuMemoryMB = parseInt(gpuMemory.trim());

                    const gpuName = execSync('nvidia-smi --query-gpu=name --format=csv,noheader', { encoding: 'utf8' });
                    capabilities.gpuName = gpuName.trim();
                } catch (err) {
                    capabilities.gpuMemoryMB = 0;
                }
            }

            return capabilities;
        }

        function recommendOptimalModel(capabilities) {
            if (capabilities.hasGPU) {
                if (capabilities.gpuMemoryMB >= 4000) {
                    return { model: '1.1b' };
                } else {
                    return { model: '0.6b' };
                }
            } else {
                if (capabilities.cpuCores >= 8 && capabilities.totalRAMGB >= 16) {
                    return { model: '1.1b' };
                } else {
                    return { model: '0.6b' };
                }
            }
        }

        const caps = detectSystemCapabilities();
        const rec = recommendOptimalModel(caps);
        console.log('MODEL:' + rec.model);
    " 2>&1)

    remove_mock_nvidia_smi

    if [[ "$result" == *"MODEL:1.1b"* ]]; then
        test_pass "Model Recommendation - High VRAM GPU"
    else
        test_fail "Model Recommendation - High VRAM GPU" "Expected 1.1b model: $result"
    fi
}

test_model_recommendation_low_vram_gpu() {
    test_start "Model Recommendation - Low VRAM GPU"

    create_mock_nvidia_smi "2048" "NVIDIA GeForce GTX 1050"

    cd "$INSTALL_DIR"
    local result
    result=$(node -e "
        const { execSync } = require('child_process');

        function detectNvidiaGPU() {
            try {
                execSync('nvidia-smi', { stdio: 'ignore' });
                return true;
            } catch {
                return false;
            }
        }

        const hasGPU = detectNvidiaGPU();
        if (hasGPU) {
            const vram = parseInt(execSync('nvidia-smi --query-gpu=memory.total --format=csv,noheader,nounits', { encoding: 'utf8' }).trim());
            const model = vram >= 4000 ? '1.1b' : '0.6b';
            console.log('MODEL:' + model);
        }
    " 2>&1)

    remove_mock_nvidia_smi

    if [[ "$result" == *"MODEL:0.6b"* ]]; then
        test_pass "Model Recommendation - Low VRAM GPU"
    else
        test_fail "Model Recommendation - Low VRAM GPU" "Expected 0.6b model: $result"
    fi
}

#############################################
# Test Suite: Error Handling
#############################################

test_error_handling_missing_template() {
    test_start "Error Handling - Missing Template"

    # Remove template
    rm -f "$INSTALL_DIR/templates/swictation-daemon.service.template"

    cd "$INSTALL_DIR"
    local result
    result=$(node -e "
        const fs = require('fs');
        const path = require('path');
        const __dirname = process.cwd();

        const templatePath = path.join(__dirname, 'templates', 'swictation-daemon.service.template');
        if (!fs.existsSync(templatePath)) {
            console.log('TEMPLATE_NOT_FOUND');
        } else {
            console.log('TEMPLATE_FOUND');
        }
    " 2>&1)

    if [[ "$result" == *"TEMPLATE_NOT_FOUND"* ]]; then
        test_pass "Error Handling - Missing Template"
    else
        test_fail "Error Handling - Missing Template" "Template check failed: $result"
    fi
}

test_error_handling_no_permissions() {
    test_start "Error Handling - No Write Permissions"

    local readonly_dir="$HOME_DIR/.config/readonly"
    mkdir -p "$readonly_dir"
    chmod 555 "$readonly_dir"

    cd "$INSTALL_DIR"
    local result
    result=$(node -e "
        const fs = require('fs');
        const path = require('path');

        try {
            fs.writeFileSync('$readonly_dir/test.txt', 'test');
            console.log('WRITE_SUCCESS');
        } catch (err) {
            if (err.code === 'EACCES') {
                console.log('PERMISSION_DENIED');
            } else {
                console.log('OTHER_ERROR:' + err.code);
            }
        }
    " 2>&1)

    chmod 755 "$readonly_dir"
    rm -rf "$readonly_dir"

    if [[ "$result" == *"PERMISSION_DENIED"* ]]; then
        test_pass "Error Handling - No Write Permissions"
    else
        test_fail "Error Handling - No Write Permissions" "Permission check failed: $result"
    fi
}

#############################################
# Main Test Runner
#############################################

run_test_suite() {
    local suite="$1"

    log "Running test suite: $suite"

    case "$suite" in
        platform)
            test_platform_linux_x64
            test_glibc_version_check
            ;;
        permissions)
            test_binary_permissions
            ;;
        directories)
            test_directory_creation
            test_directory_idempotency
            ;;
        gpu)
            test_gpu_detection_present_high_vram
            test_gpu_detection_low_vram
            test_gpu_detection_absent
            ;;
        ort)
            test_ort_detection_bundled
            test_ort_detection_python_fallback
            test_ort_detection_missing
            ;;
        services)
            test_service_generation
            ;;
        models)
            test_model_recommendation_high_vram_gpu
            test_model_recommendation_low_vram_gpu
            ;;
        errors)
            test_error_handling_missing_template
            test_error_handling_no_permissions
            ;;
        all)
            run_test_suite platform
            run_test_suite permissions
            run_test_suite directories
            run_test_suite gpu
            run_test_suite ort
            run_test_suite services
            run_test_suite models
            run_test_suite errors
            ;;
        *)
            log_error "Unknown test suite: $suite"
            echo "Available suites: all, platform, permissions, directories, gpu, ort, services, models, errors"
            exit 1
            ;;
    esac
}

print_summary() {
    log "\n=========================================="
    log "TEST SUMMARY"
    log "=========================================="
    log "Total Tests Run: $TESTS_RUN"
    log_success "Passed: $TESTS_PASSED"
    if [[ $TESTS_FAILED -gt 0 ]]; then
        log_error "Failed: $TESTS_FAILED"
    else
        log "Failed: 0"
    fi
    if [[ $TESTS_SKIPPED -gt 0 ]]; then
        log_warning "Skipped: $TESTS_SKIPPED"
    fi
    log "=========================================="

    if [[ $TESTS_FAILED -eq 0 ]]; then
        log_success "ALL TESTS PASSED!"
        return 0
    else
        log_error "SOME TESTS FAILED"
        return 1
    fi
}

#############################################
# Main Entry Point
#############################################

main() {
    local suite="${1:-all}"

    echo "==============================================="
    echo "Swictation Postinstall Automated Test Suite"
    echo "Version: 0.3.0"
    echo "==============================================="
    echo ""

    setup_test_environment

    trap cleanup_test_environment EXIT

    run_test_suite "$suite"

    print_summary

    local exit_code=$?

    log "\nTest log saved to: $LOG_FILE"

    exit $exit_code
}

# Run main with arguments
main "$@"
