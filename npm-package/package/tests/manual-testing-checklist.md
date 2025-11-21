# Manual Testing Checklist - Swictation 0.3.0

## Overview
This document provides step-by-step manual testing procedures for validating the Swictation npm installation process. Use this checklist for comprehensive validation before release.

---

## Pre-Testing Setup

### Environment Preparation
- [ ] Test machines available:
  - [ ] Ubuntu 24.04 LTS (clean install)
  - [ ] Ubuntu 24.04 LTS (with NVIDIA GPU 6GB+)
  - [ ] Ubuntu 24.04 LTS (with NVIDIA GPU 2-4GB)
  - [ ] Ubuntu 24.04 LTS (CPU-only)
  - [ ] Ubuntu 22.04 LTS (for compatibility warning test)
- [ ] Node.js 18+ installed on all test machines
- [ ] npm configured and working
- [ ] Internet connection verified
- [ ] Screen capture software ready

### Documentation Preparation
- [ ] Create test results folder with timestamp
- [ ] Prepare screenshots folder
- [ ] Open text editor for notes
- [ ] Have this checklist printed or on second screen

---

## Test Session 1: Fresh Installation (Ubuntu 24.04 + GPU 6GB+)

### Test Environment
- **OS**: Ubuntu 24.04 LTS
- **GPU**: NVIDIA GPU with 6GB+ VRAM
- **Date**: ________________
- **Tester**: ________________
- **Duration**: ________________

### Pre-Test System Check
```bash
# Document system information
uname -a
cat /etc/os-release | grep PRETTY_NAME
nvidia-smi --query-gpu=name,memory.total --format=csv
node --version
npm --version
ldd --version | head -1
```

**System Info** (fill in):
- Kernel: ________________
- OS: ________________
- GPU: ________________
- GPU VRAM: ________________
- Node: ________________
- npm: ________________
- GLIBC: ________________

### Installation Steps

#### Step 1: Clean Environment
```bash
# Remove any existing installation
npm uninstall -g swictation 2>/dev/null || true
rm -rf ~/.config/swictation
rm -rf ~/.local/share/swictation
rm -rf ~/.cache/swictation
rm -rf ~/.config/systemd/user/swictation*.service
```

**Verification**:
- [ ] No swictation command found: `which swictation` returns nothing
- [ ] No directories exist
- [ ] No service files exist

#### Step 2: Install Package
```bash
# Install from npm
npm install -g swictation@0.3.0
```

**Time Started**: ________________
**Time Completed**: ________________
**Duration**: ________________

**Observations During Install**:
- [ ] Platform check message shown
- [ ] GLIBC check passed (no warnings)
- [ ] Binary permissions messages shown (✓ for each binary)
- [ ] Directory creation messages shown (✓ for each directory)
- [ ] GPU detection message shown: "✓ NVIDIA GPU detected!"
- [ ] GPU library download initiated
- [ ] Download URL displayed
- [ ] Download progress or completion message
- [ ] Extraction message: "✓ Extracted GPU libraries"
- [ ] GPU acceleration enabled message
- [ ] ORT detection message: "✓ Found ONNX Runtime (GPU-enabled)"
- [ ] Service file generation messages
- [ ] Dependency check output
- [ ] System detection summary shown
- [ ] Model recommendation shown: **1.1B**
- [ ] Next steps displayed

**Screenshot**: `01-installation-output.png`

#### Step 3: Verify Console Output

**Expected Messages** (check all that appear):
- [ ] "🚀 Setting up Swictation..."
- [ ] "✓ Set execute permissions for swictation-daemon"
- [ ] "✓ Set execute permissions for swictation-ui"
- [ ] "✓ Set execute permissions for swictation"
- [ ] "✓ Set execute permissions for swictation-daemon.bin"
- [ ] "✓ Created directory: ~/.config/swictation"
- [ ] "✓ Created directory: ~/.local/share/swictation"
- [ ] "✓ Created directory: ~/.local/share/swictation/models"
- [ ] "✓ Created directory: ~/.cache/swictation"
- [ ] "✓ NVIDIA GPU detected!"
- [ ] "📦 Downloading GPU acceleration libraries..."
- [ ] "✓ Downloaded GPU libraries"
- [ ] "✓ Extracted GPU libraries"
- [ ] "✓ GPU acceleration enabled!"
- [ ] "🔍 Detecting ONNX Runtime library path..."
- [ ] "✓ Found ONNX Runtime (GPU-enabled)"
- [ ] "⚙️ Generating systemd service files..."
- [ ] "✓ Generated daemon service"
- [ ] "✓ Installed UI service"
- [ ] "✨ Swictation installed successfully!"
- [ ] "📊 System Detection:"
- [ ] GPU name and VRAM shown
- [ ] "🎯 Recommended Model: 1.1B"
- [ ] "62x realtime speed on GPU"
- [ ] Next steps with commands

**Warnings/Errors** (should be none):
- [ ] No red error messages
- [ ] No unexpected warnings

#### Step 4: Verify File System

```bash
# Check directories
ls -la ~/.config/swictation
ls -la ~/.local/share/swictation
ls -la ~/.local/share/swictation/models
ls -la ~/.cache/swictation

# Check binaries
NPM_ROOT=$(npm root -g)
ls -la $NPM_ROOT/swictation/bin/
ls -la $NPM_ROOT/swictation/lib/native/

# Check permissions
stat -c '%A %n' $NPM_ROOT/swictation/bin/swictation
stat -c '%A %n' $NPM_ROOT/swictation/bin/swictation-daemon
stat -c '%A %n' $NPM_ROOT/swictation/bin/swictation-ui
stat -c '%A %n' $NPM_ROOT/swictation/lib/native/swictation-daemon.bin

# Check service files
ls -la ~/.config/systemd/user/swictation*.service
cat ~/.config/systemd/user/swictation-daemon.service
```

**File System Verification**:
- [ ] All directories created: `~/.config/swictation`, `~/.local/share/swictation`, models, cache
- [ ] Binaries present in `bin/` directory
- [ ] Binaries have execute permission (755): `-rwxr-xr-x`
- [ ] Native library present: `lib/native/swictation-daemon.bin`
- [ ] GPU libraries present: `lib/native/libonnxruntime.so` and CUDA libraries
- [ ] Service files exist: `swictation-daemon.service`, `swictation-ui.service`

**Screenshot**: `02-filesystem-verification.png`

#### Step 5: Verify Service File Content

```bash
# Check daemon service
cat ~/.config/systemd/user/swictation-daemon.service | grep -E "ExecStart|ORT_DYLIB_PATH|LD_LIBRARY_PATH"
```

**Service File Verification**:
- [ ] `ExecStart` points to correct npm installation path (no `__INSTALL_DIR__`)
- [ ] `ORT_DYLIB_PATH` is set to actual path (no `__ORT_DYLIB_PATH__` placeholder)
- [ ] ORT_DYLIB_PATH points to bundled library in npm package
- [ ] `LD_LIBRARY_PATH` includes CUDA paths: `/usr/local/cuda/lib64`, `/usr/local/cuda-12.9/lib64`
- [ ] `LD_LIBRARY_PATH` includes npm native library path
- [ ] `CUDA_HOME` is set to `/usr/local/cuda`
- [ ] `RUST_LOG` is set to `info`

**Screenshot**: `03-service-file-content.png`

#### Step 6: Test Command Availability

```bash
# Test CLI commands
swictation --version
swictation help
which swictation
which swictation-daemon
which swictation-ui
```

**Command Verification**:
- [ ] `swictation --version` shows version 0.3.0
- [ ] `swictation help` displays help information
- [ ] All binaries found in PATH
- [ ] No permission errors

**Screenshot**: `04-command-verification.png`

#### Step 7: Document Model Recommendation

**Model Recommendation Details**:
- Detected GPU: ________________
- Detected VRAM: ________________
- Recommended Model: ________________
- Reason Given: ________________
- Download Size: ________________
- Performance Note: ________________

**Expectations Met**:
- [ ] Model recommendation is 1.1B for 6GB+ GPU
- [ ] Reason mentions GPU name and VRAM
- [ ] Performance note mentions "62x realtime speed on GPU"

### Test Result

**Overall Result**: [ ] PASS  [ ] FAIL  [ ] PARTIAL

**Issues Found**:
1. ________________
2. ________________
3. ________________

**Screenshots Captured**:
- [ ] 01-installation-output.png
- [ ] 02-filesystem-verification.png
- [ ] 03-service-file-content.png
- [ ] 04-command-verification.png

**Sign-off**:
- Tester: ________________
- Date: ________________
- Time: ________________

---

## Test Session 2: Fresh Installation (Ubuntu 24.04 + GPU 2-4GB)

### Test Environment
- **OS**: Ubuntu 24.04 LTS
- **GPU**: NVIDIA GPU with 2-4GB VRAM
- **Date**: ________________
- **Tester**: ________________

### Expected Differences from Session 1
- Model recommendation should be **0.6B** instead of 1.1B
- Reason should mention "limited VRAM"

### Installation Steps

(Follow same steps as Session 1)

**Key Verification Points**:
- [ ] GPU detected correctly
- [ ] VRAM amount shown correctly (2-4GB)
- [ ] Recommended model: **0.6B**
- [ ] Reason: "GPU detected but limited VRAM"
- [ ] Description: "Lighter model for lower VRAM systems"

### Test Result

**Overall Result**: [ ] PASS  [ ] FAIL  [ ] PARTIAL

**Sign-off**:
- Tester: ________________
- Date: ________________

---

## Test Session 3: Fresh Installation (Ubuntu 24.04 CPU-Only)

### Test Environment
- **OS**: Ubuntu 24.04 LTS
- **GPU**: None (or nvidia-smi not available)
- **CPU Cores**: ________________
- **RAM**: ________________
- **Date**: ________________
- **Tester**: ________________

### Expected Behavior
- No GPU detection
- CPU/RAM-based model recommendation
- No GPU library download
- Fallback to Python ONNX Runtime detection

### Installation Steps

(Follow same steps as Session 1)

**Key Verification Points**:
- [ ] Message: "ℹ No NVIDIA GPU detected - skipping GPU library download"
- [ ] Message: "CPU-only mode will be used"
- [ ] No GPU library download attempted
- [ ] Model recommendation based on CPU cores and RAM
- [ ] If 8+ cores and 16GB+ RAM: Recommended model is **1.1B (INT8)**
- [ ] If fewer cores/RAM: Recommended model is **0.6B**
- [ ] Reason mentions CPU cores and RAM

**ORT Detection**:
- [ ] Bundled GPU library not found (expected)
- [ ] Warning: "Falling back to system Python installation"
- [ ] Python ONNX Runtime path detected OR
- [ ] Warning to install: `pip3 install onnxruntime-gpu`

### Test Result

**Overall Result**: [ ] PASS  [ ] FAIL  [ ] PARTIAL

**Sign-off**:
- Tester: ________________
- Date: ________________

---

## Test Session 4: Ubuntu 22.04 Compatibility Warning

### Test Environment
- **OS**: Ubuntu 22.04 LTS
- **GLIBC**: 2.35 (expected)
- **Date**: ________________
- **Tester**: ________________

### Expected Behavior
- Installation proceeds but with GLIBC warning
- Warning about Ubuntu 22.04 not supported

### Installation Steps

```bash
# Install on Ubuntu 22.04
npm install -g swictation@0.3.0
```

**Key Verification Points**:
- [ ] Warning: "⚠ INCOMPATIBLE GLIBC VERSION"
- [ ] Message: "Detected GLIBC 2.35 (need 2.39+)"
- [ ] Message: "Swictation requires Ubuntu 24.04 LTS or newer"
- [ ] Message: "Ubuntu 22.04 is NOT supported due to GLIBC 2.35"
- [ ] List of supported distributions shown
- [ ] Message: "Installation will continue but binaries may not work"
- [ ] Installation completes (exit code 0)
- [ ] All files created normally

### Test Result

**Overall Result**: [ ] PASS  [ ] FAIL  [ ] PARTIAL

**Sign-off**:
- Tester: ________________
- Date: ________________

---

## Test Session 5: Upgrade from 0.2.x to 0.3.0

### Test Environment
- **OS**: Ubuntu 24.04 LTS
- **Previous Version**: 0.2.x
- **Date**: ________________
- **Tester**: ________________

### Pre-Upgrade Setup

```bash
# Install old version first (if available)
npm install -g swictation@0.2.2

# Verify old installation
swictation --version
ls ~/.config/systemd/user/swictation*.service
```

**Old Installation Verified**:
- [ ] Version 0.2.2 installed
- [ ] Old service files exist
- [ ] Old configuration exists

### Upgrade Steps

```bash
# Upgrade to 0.3.0
npm install -g swictation@0.3.0
```

**Key Verification Points**:
- [ ] Old directories preserved
- [ ] Old configuration files preserved
- [ ] New service files generated
- [ ] Old service files still present (or replaced)
- [ ] No data loss
- [ ] Version command shows 0.3.0
- [ ] All new features available

**Check for Conflicts**:
- [ ] No file permission errors
- [ ] No directory conflicts
- [ ] Service files updated correctly
- [ ] Binary paths updated

### Test Result

**Overall Result**: [ ] PASS  [ ] FAIL  [ ] PARTIAL

**Sign-off**:
- Tester: ________________
- Date: ________________

---

## Test Session 6: Reinstall (Idempotency Test)

### Test Environment
- **OS**: Ubuntu 24.04 LTS
- **Date**: ________________
- **Tester**: ________________

### Test Steps

```bash
# Install once
npm install -g swictation@0.3.0

# Capture state
ls -la ~/.config/swictation
stat ~/.config/systemd/user/swictation-daemon.service

# Reinstall immediately
npm install -g swictation@0.3.0

# Compare state
ls -la ~/.config/swictation
stat ~/.config/systemd/user/swictation-daemon.service
```

**Idempotency Verification**:
- [ ] No errors on second installation
- [ ] No "file exists" errors
- [ ] No duplicate files created
- [ ] Service files regenerated correctly
- [ ] Binary permissions reset to 755
- [ ] Same result as first installation
- [ ] Exit code 0

### Test Result

**Overall Result**: [ ] PASS  [ ] FAIL  [ ] PARTIAL

**Sign-off**:
- Tester: ________________
- Date: ________________

---

## Test Session 7: Error Condition Testing

### Test 7A: Network Failure During GPU Library Download

**Setup**:
```bash
# Disconnect network or use firewall to block GitHub
sudo iptables -A OUTPUT -d github.com -j DROP
```

**Test Steps**:
```bash
npm install -g swictation@0.3.0
```

**Expected Behavior**:
- [ ] Warning: "⚠ Failed to download GPU libraries"
- [ ] Message: "Continuing with CPU-only mode"
- [ ] Manual download URL provided
- [ ] Installation completes successfully (exit code 0)
- [ ] No GPU libraries in lib/native/
- [ ] Rest of installation proceeds normally

**Cleanup**:
```bash
sudo iptables -D OUTPUT -d github.com -j DROP
```

**Result**: [ ] PASS  [ ] FAIL

---

### Test 7B: No Write Permission to Config Directory

**Setup**:
```bash
mkdir -p ~/.config
chmod 555 ~/.config  # Read and execute only
```

**Test Steps**:
```bash
npm install -g swictation@0.3.0
```

**Expected Behavior**:
- [ ] Warning about unable to create `~/.config/swictation`
- [ ] Installation continues
- [ ] Other directories created successfully
- [ ] Exit code 0 (success)
- [ ] User advised to fix permissions manually

**Cleanup**:
```bash
chmod 755 ~/.config
```

**Result**: [ ] PASS  [ ] FAIL

---

### Test 7C: Missing Python (No ORT Fallback)

**Setup**:
```bash
# Remove bundled ORT library to force Python detection
NPM_ROOT=$(npm root -g)
rm -f $NPM_ROOT/swictation/lib/native/libonnxruntime.so

# Ensure Python is not in PATH (or rename python3)
sudo mv /usr/bin/python3 /usr/bin/python3.bak
```

**Test Steps**:
```bash
npm install -g swictation@0.3.0
```

**Expected Behavior**:
- [ ] Warning: "⚠️ Could not detect ONNX Runtime"
- [ ] Message: "📦 Please install onnxruntime-gpu for optimal performance"
- [ ] Installation command provided: `pip3 install onnxruntime-gpu`
- [ ] Warning: "The daemon will not work correctly without this library!"
- [ ] Installation completes (exit code 0)
- [ ] Service file contains placeholder: `__ORT_DYLIB_PATH__`

**Cleanup**:
```bash
sudo mv /usr/bin/python3.bak /usr/bin/python3
```

**Result**: [ ] PASS  [ ] FAIL

---

## Test Session 8: Console Output Quality Review

### Test Environment
- **Date**: ________________
- **Tester**: ________________

### Visual Quality Checklist

Run a standard installation and evaluate:

**Color Usage**:
- [ ] Green (✓) used for success messages
- [ ] Yellow (⚠) used for warnings
- [ ] Red (✗) used for errors (if any)
- [ ] Cyan/Blue used for informational messages
- [ ] Colors appropriate and not excessive

**Formatting**:
- [ ] Clear section headers (🚀, 📦, ⚙️, etc.)
- [ ] Consistent indentation
- [ ] Proper spacing between sections
- [ ] Progress indicators clear
- [ ] File paths clearly shown

**Information Completeness**:
- [ ] Every action clearly announced
- [ ] File paths shown for created files
- [ ] Hardware detection results displayed
- [ ] Model recommendation with clear rationale
- [ ] Next steps with copy-pasteable commands
- [ ] Help reference provided

**Readability**:
- [ ] No walls of text
- [ ] Important information stands out
- [ ] Technical jargon explained
- [ ] Commands formatted distinctly
- [ ] Summary at end is clear

**User Experience**:
- [ ] Installation feels professional
- [ ] Progress is apparent
- [ ] User knows what's happening
- [ ] Errors (if any) have solutions
- [ ] Next steps are actionable

**Overall Score**: ___/10

**Comments**:
________________
________________
________________

### Test Result

**Overall Result**: [ ] PASS  [ ] FAIL  [ ] NEEDS_IMPROVEMENT

**Sign-off**:
- Tester: ________________
- Date: ________________

---

## Test Session 9: Service File Validation

### Test Environment
- **OS**: Ubuntu 24.04 LTS
- **Date**: ________________
- **Tester**: ________________

### Validation Steps

```bash
# Install package
npm install -g swictation@0.3.0

# Validate service file syntax
systemd-analyze verify ~/.config/systemd/user/swictation-daemon.service
systemd-analyze verify ~/.config/systemd/user/swictation-ui.service

# Try loading services (don't start them)
systemctl --user daemon-reload
systemctl --user status swictation-daemon.service
systemctl --user status swictation-ui.service
```

**Service File Validation**:
- [ ] `systemd-analyze verify` passes for daemon service
- [ ] `systemd-analyze verify` passes for UI service
- [ ] No syntax errors reported
- [ ] Services recognized by systemd
- [ ] Status shows "loaded" (not necessarily active)
- [ ] No missing dependencies reported

**Manual Service File Inspection**:
- [ ] All paths are absolute (no relative paths)
- [ ] No placeholder variables remain (`__INSTALL_DIR__`, `__ORT_DYLIB_PATH__`)
- [ ] ExecStart points to valid binary
- [ ] Environment variables properly formatted
- [ ] Restart policy is reasonable
- [ ] Service dependencies correct (Wants=swictation-ui.service)

### Test Result

**Overall Result**: [ ] PASS  [ ] FAIL  [ ] PARTIAL

**Sign-off**:
- Tester: ________________
- Date: ________________

---

## Test Session 10: Full Integration Test

### Test Environment
- **OS**: Ubuntu 24.04 LTS
- **GPU**: NVIDIA GPU (any VRAM)
- **Date**: ________________
- **Tester**: ________________

### Complete Workflow Test

This test validates the entire installation → setup → usage workflow.

#### Step 1: Clean Installation
```bash
npm install -g swictation@0.3.0
```

**Verified**: [ ] Installation successful

#### Step 2: Download Model
```bash
# Install huggingface_hub if needed
pip3 install "huggingface_hub[cli]"

# Download recommended model (use what was recommended)
swictation download-model 1.1b  # or 0.6b based on recommendation
```

**Verified**:
- [ ] Model download command works
- [ ] Model downloaded to `~/.local/share/swictation/models/`
- [ ] No errors during download

#### Step 3: Run Setup
```bash
swictation setup
```

**Verified**:
- [ ] Setup command runs without errors
- [ ] Configuration created/updated
- [ ] Services registered with systemd

#### Step 4: Start Service
```bash
swictation start
```

**Verified**:
- [ ] Service starts successfully
- [ ] No immediate crashes
- [ ] `systemctl --user status swictation-daemon.service` shows active

#### Step 5: Check Service Status
```bash
swictation status
systemctl --user status swictation-daemon.service
journalctl --user -u swictation-daemon.service -n 50
```

**Verified**:
- [ ] Daemon is running
- [ ] No error messages in logs
- [ ] Service is stable (not restarting)

#### Step 6: Test Toggle (if applicable)
```bash
swictation toggle
# Wait a few seconds
swictation toggle
```

**Verified**:
- [ ] Toggle command works
- [ ] Recording starts/stops
- [ ] No crashes

#### Step 7: Stop Service
```bash
swictation stop
```

**Verified**:
- [ ] Service stops cleanly
- [ ] No error messages
- [ ] Status shows inactive

### Test Result

**Overall Result**: [ ] PASS  [ ] FAIL  [ ] PARTIAL

**Issues Found**:
________________
________________
________________

**Sign-off**:
- Tester: ________________
- Date: ________________

---

## Final Summary

### Test Session Results

| Session | Description | Result | Tester | Date |
|---------|-------------|--------|--------|------|
| 1 | Fresh Install (GPU 6GB+) | [ ] P [ ] F | _____ | _____ |
| 2 | Fresh Install (GPU 2-4GB) | [ ] P [ ] F | _____ | _____ |
| 3 | Fresh Install (CPU-Only) | [ ] P [ ] F | _____ | _____ |
| 4 | Ubuntu 22.04 Warning | [ ] P [ ] F | _____ | _____ |
| 5 | Upgrade 0.2.x → 0.3.0 | [ ] P [ ] F | _____ | _____ |
| 6 | Reinstall Idempotency | [ ] P [ ] F | _____ | _____ |
| 7A | Network Failure | [ ] P [ ] F | _____ | _____ |
| 7B | Permission Error | [ ] P [ ] F | _____ | _____ |
| 7C | Missing Python | [ ] P [ ] F | _____ | _____ |
| 8 | Console Output Quality | [ ] P [ ] F | _____ | _____ |
| 9 | Service Validation | [ ] P [ ] F | _____ | _____ |
| 10 | Full Integration | [ ] P [ ] F | _____ | _____ |

### Overall Assessment

**Total Tests Run**: _____
**Tests Passed**: _____
**Tests Failed**: _____
**Pass Rate**: _____%

### Critical Issues Found

1. ________________
2. ________________
3. ________________

### Non-Critical Issues Found

1. ________________
2. ________________
3. ________________

### Recommendations

________________
________________
________________

### Release Readiness

**Is version 0.3.0 ready for release?** [ ] YES  [ ] NO  [ ] CONDITIONAL

**Conditions** (if conditional):
________________
________________

### Sign-Off

**QA Lead**: ________________
**Date**: ________________
**Signature**: ________________

**Development Lead**: ________________
**Date**: ________________
**Signature**: ________________

---

## Appendix: Quick Command Reference

### Installation Commands
```bash
npm install -g swictation@0.3.0
npm uninstall -g swictation
```

### Verification Commands
```bash
swictation --version
swictation help
which swictation
ls -la ~/.config/swictation
ls -la ~/.local/share/swictation
systemctl --user status swictation-daemon.service
```

### Cleanup Commands
```bash
npm uninstall -g swictation
rm -rf ~/.config/swictation
rm -rf ~/.local/share/swictation
rm -rf ~/.cache/swictation
rm -f ~/.config/systemd/user/swictation*.service
systemctl --user daemon-reload
```

### Diagnostic Commands
```bash
# System info
uname -a
cat /etc/os-release
ldd --version | head -1
node --version
npm --version

# GPU info
nvidia-smi
nvidia-smi --query-gpu=name,memory.total --format=csv

# Service logs
journalctl --user -u swictation-daemon.service -n 100
journalctl --user -u swictation-ui.service -n 100

# File locations
npm root -g
readlink -f $(which swictation)
```

---

**Document Version**: 1.0
**Last Updated**: 2025-11-13
**Status**: Ready for Manual Testing
