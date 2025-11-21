# Installation Test Scenarios

## Test Environment Information
- **Version**: 0.3.0
- **Platform**: Linux x64 only
- **Required GLIBC**: 2.39+
- **Test Date**: 2025-11-13
- **Tester**: _____________

---

## Test Scenario 1: Fresh Installation (No Previous Install)

### Test ID: TS-001
### Objective
Verify clean installation on a system with no previous Swictation installation.

### Prerequisites
- Clean Ubuntu 24.04 LTS system
- No previous Swictation installation
- Node.js 18+ installed
- No existing `~/.config/swictation` directory

### Test Steps
1. Run: `npm install -g swictation@0.3.0`
2. Observe console output during postinstall
3. Check created directories
4. Verify binary permissions
5. Check systemd service files generated

### Expected Results
- ✅ All directories created:
  - `~/.config/swictation`
  - `~/.local/share/swictation`
  - `~/.local/share/swictation/models`
  - `~/.cache/swictation`
- ✅ Binary permissions set to 755:
  - `bin/swictation-daemon`
  - `bin/swictation-ui`
  - `bin/swictation`
  - `lib/native/swictation-daemon.bin`
- ✅ Systemd services generated:
  - `~/.config/systemd/user/swictation-daemon.service`
  - `~/.config/systemd/user/swictation-ui.service`
- ✅ ORT library detected and configured
- ✅ GPU libraries downloaded (if NVIDIA GPU present)
- ✅ Platform check passed (Linux x64)
- ✅ GLIBC version check passed (2.39+)

### Pass Criteria
- All files created successfully
- No errors in console output
- Services generated with correct paths
- Appropriate model recommendation shown

---

## Test Scenario 2: GPU Detection and Library Download

### Test ID: TS-002
### Objective
Verify correct GPU detection and GPU library download process.

### Sub-Scenario 2A: NVIDIA GPU Present (6GB+ VRAM)

#### Prerequisites
- System with NVIDIA GPU (6GB+ VRAM)
- `nvidia-smi` command available
- Internet connection for library download

#### Test Steps
1. Run: `npm install -g swictation@0.3.0`
2. Verify GPU detection message
3. Check GPU library download
4. Verify extraction to `lib/native/`
5. Check model recommendation

#### Expected Results
- ✅ Message: "✓ NVIDIA GPU detected!"
- ✅ GPU library download initiated
- ✅ Download URL shown: `https://github.com/robertelee78/swictation/releases/download/v0.3.0/swictation-gpu-libs.tar.gz`
- ✅ Libraries extracted to: `lib/native/`
- ✅ Message: "✓ GPU acceleration enabled!"
- ✅ Recommended model: **1.1B** (Best quality - Full GPU acceleration)
- ✅ Performance note: "62x realtime speed on GPU"

### Sub-Scenario 2B: NVIDIA GPU Present (2-4GB VRAM)

#### Prerequisites
- System with NVIDIA GPU (2-4GB VRAM)
- `nvidia-smi` command available

#### Expected Results
- ✅ GPU detected
- ✅ GPU libraries downloaded
- ✅ Recommended model: **0.6B** (Lighter model for lower VRAM)
- ✅ Reason shown: "GPU detected but limited VRAM (2-4GB)"

### Sub-Scenario 2C: No GPU Present

#### Prerequisites
- System without NVIDIA GPU OR `nvidia-smi` not available

#### Expected Results
- ✅ Message: "ℹ No NVIDIA GPU detected - skipping GPU library download"
- ✅ Message: "CPU-only mode will be used"
- ✅ No GPU library download attempted
- ✅ Model recommendation based on CPU cores and RAM

---

## Test Scenario 3: ONNX Runtime Library Detection

### Test ID: TS-003
### Objective
Verify correct ONNX Runtime library detection and configuration.

### Sub-Scenario 3A: Bundled GPU-Enabled Library Present

#### Prerequisites
- GPU libraries downloaded successfully
- File exists: `lib/native/libonnxruntime.so`

#### Test Steps
1. Complete installation
2. Check console output for ORT detection
3. Verify service file configuration

#### Expected Results
- ✅ Message: "✓ Found ONNX Runtime (GPU-enabled): [path]/lib/native/libonnxruntime.so"
- ✅ Message: "Using bundled GPU-enabled library with CUDA provider support"
- ✅ Service file contains: `Environment="ORT_DYLIB_PATH=[path]/lib/native/libonnxruntime.so"`
- ✅ No fallback to Python installation

### Sub-Scenario 3B: Fallback to Python Installation

#### Prerequisites
- Bundled library not present
- Python3 with onnxruntime-gpu installed

#### Expected Results
- ✅ Warning: "⚠️ Warning: GPU-enabled ONNX Runtime not found at [path]"
- ✅ Message: "Falling back to system Python installation (may be CPU-only)"
- ✅ Python ORT path detected via: `python3 -c "import onnxruntime..."`
- ✅ Warning: "Using Python ONNX Runtime: [path]"
- ✅ Warning: "Note: This may be CPU-only and lack CUDA support"
- ✅ Config file created: `config/detected-environment.json`

### Sub-Scenario 3C: No ONNX Runtime Available

#### Prerequisites
- No bundled library
- No Python onnxruntime-gpu installed

#### Expected Results
- ✅ Warning: "⚠️ Could not detect ONNX Runtime:"
- ✅ Message: "📦 Please install onnxruntime-gpu for optimal performance:"
- ✅ Installation command shown: `pip3 install onnxruntime-gpu`
- ✅ Warning: "The daemon will not work correctly without this library!"
- ✅ Service file contains placeholder: `Environment="ORT_DYLIB_PATH=__ORT_DYLIB_PATH__"`

---

## Test Scenario 4: Platform and GLIBC Compatibility Checks

### Test ID: TS-004
### Objective
Verify platform compatibility checks work correctly.

### Sub-Scenario 4A: Ubuntu 24.04 LTS (GLIBC 2.39+)

#### Expected Results
- ✅ Platform check passes
- ✅ GLIBC version check passes
- ✅ No compatibility warnings
- ✅ Installation proceeds normally

### Sub-Scenario 4B: Ubuntu 22.04 LTS (GLIBC 2.35)

#### Expected Results
- ⚠️ Warning: "⚠ INCOMPATIBLE GLIBC VERSION"
- ⚠️ Message: "Detected GLIBC 2.35 (need 2.39+)"
- ⚠️ Message: "Swictation requires Ubuntu 24.04 LTS or newer"
- ⚠️ Message: "Ubuntu 22.04 is NOT supported due to GLIBC 2.35"
- ⚠️ Supported distributions listed
- ⚠️ Message: "Installation will continue but binaries may not work."
- ✅ Installation continues (doesn't exit)

### Sub-Scenario 4C: Non-Linux Platform

#### Prerequisites
- macOS or Windows system

#### Expected Results
- ⚠️ Message: "Note: Swictation currently only supports Linux x64"
- ⚠️ Message: "Skipping postinstall for non-Linux platform"
- ✅ Installation exits cleanly (exit code 0)

### Sub-Scenario 4D: Non-x64 Architecture

#### Prerequisites
- Linux system with ARM or other architecture

#### Expected Results
- ⚠️ Message: "Note: Swictation currently only supports x64 architecture"
- ✅ Installation exits cleanly (exit code 0)

---

## Test Scenario 5: Directory Creation and Permissions

### Test ID: TS-005
### Objective
Verify all required directories are created with correct permissions.

### Test Steps
1. Remove any existing Swictation directories
2. Run installation
3. Check directory creation
4. Verify permissions

### Expected Results
All directories created with correct permissions:

| Directory | Purpose | Owner | Permissions |
|-----------|---------|-------|-------------|
| `~/.config/swictation` | Configuration files | user:user | 755 |
| `~/.local/share/swictation` | Application data | user:user | 755 |
| `~/.local/share/swictation/models` | AI models | user:user | 755 |
| `~/.cache/swictation` | Cache files | user:user | 755 |

### Error Handling
- ✅ If directory already exists: No error, skip creation
- ✅ If permission denied: Warning shown, installation continues
- ✅ Directory creation failures logged but don't halt installation

---

## Test Scenario 6: Systemd Service Generation

### Test ID: TS-006
### Objective
Verify systemd service files are generated correctly from templates.

### Test Steps
1. Complete installation
2. Check service file generation
3. Verify template substitution
4. Validate service file syntax

### Expected Results

#### Daemon Service
- ✅ File created: `~/.config/systemd/user/swictation-daemon.service`
- ✅ Template placeholders replaced:
  - `__INSTALL_DIR__` → actual npm installation path
  - `__ORT_DYLIB_PATH__` → detected ONNX Runtime library path
- ✅ Environment variables configured:
  - `RUST_LOG=info`
  - `ORT_DYLIB_PATH=[detected path]`
  - `LD_LIBRARY_PATH=/usr/local/cuda/lib64:/usr/local/cuda-12.9/lib64:[install_dir]/lib/native`
  - `CUDA_HOME=/usr/local/cuda`
- ✅ Service configuration:
  - `Type=simple`
  - `Restart=on-failure`
  - `RestartSec=5`
  - `Wants=swictation-ui.service`

#### UI Service
- ✅ File copied: `~/.config/systemd/user/swictation-ui.service`
- ✅ No template substitution needed (copied directly)

### Service Validation
```bash
# Validate service file syntax
systemd-analyze verify ~/.config/systemd/user/swictation-daemon.service
systemd-analyze verify ~/.config/systemd/user/swictation-ui.service
```

### Error Cases
- ✅ Template not found: Warning shown, service generation skipped
- ✅ No write permission: Warning shown, installation continues
- ✅ ORT path not detected: Placeholder left in file, warning shown

---

## Test Scenario 7: Dependency Checking

### Test ID: TS-007
### Objective
Verify optional and required dependencies are detected correctly.

### Test Steps
1. Install with various dependency combinations
2. Check console output for dependency status
3. Verify installation continues appropriately

### Dependency Matrix

| Dependency | Type | Package | Detection Command |
|------------|------|---------|------------------|
| systemctl | Optional | systemd | `which systemctl` |
| nc | Optional | netcat | `which nc` |
| wtype | Optional | wtype | `which wtype` |
| xdotool | Optional | xdotool | `which xdotool` |
| hf | Optional | huggingface_hub[cli] | `which hf` |

### Expected Results

#### All Dependencies Present
- ✅ No dependency warnings
- ✅ Installation completes normally

#### Missing Optional Dependencies
- ⚠️ Message: "📦 Optional dependencies for full functionality:"
- ⚠️ List of missing dependencies shown
- ⚠️ Installation packages listed
- ✅ Installation continues

#### Missing Required Dependencies (if any added in future)
- ❌ Message: "⚠ Required dependencies missing:"
- ❌ List of required dependencies shown
- ❌ Installation commands provided
- ❌ Installation exits with code 1

---

## Test Scenario 8: Model Recommendation Logic

### Test ID: TS-008
### Objective
Verify correct AI model recommendations based on system hardware.

### Hardware Scenarios and Expected Recommendations

| Hardware Configuration | Recommended Model | Reason |
|----------------------|-------------------|--------|
| **GPU: 6GB+ VRAM** | 1.1B (FP32) | Best quality - Full GPU acceleration |
| **GPU: 4-6GB VRAM** | 1.1B (FP32) | Best quality - Full GPU acceleration |
| **GPU: 2-4GB VRAM** | 0.6B | Lighter model for lower VRAM |
| **CPU: 8+ cores, 16GB+ RAM** | 1.1B (INT8) | Powerful CPU can handle quantized model |
| **CPU: 4-8 cores, 8-16GB RAM** | 0.6B | Lighter model for CPU-only |
| **CPU: <4 cores, <8GB RAM** | 0.6B | Optimized for CPU |

### Test Steps
1. Run installation on various hardware configurations
2. Check system detection output
3. Verify model recommendation
4. Validate reasoning displayed

### Expected Output Format
```
📊 System Detection:
   [Hardware detection reason]

🎯 Recommended Model:
   [MODEL] - [Description]
   Size: [Download size]
   Performance: [Performance note]

Next steps:
  1. Download recommended AI model:
     swictation download-model [model]
```

### Validation Points
- ✅ Correct GPU detection (using `nvidia-smi`)
- ✅ GPU memory read correctly (MB → GB conversion)
- ✅ GPU name displayed
- ✅ CPU core count accurate
- ✅ RAM detection correct (bytes → GB conversion)
- ✅ Model recommendation matches hardware profile
- ✅ Download size accurate
- ✅ Performance expectations clear

---

## Test Scenario 9: Error Handling and Edge Cases

### Test ID: TS-009
### Objective
Verify graceful error handling for various failure conditions.

### Sub-Scenario 9A: Network Failure During GPU Library Download

#### Test Steps
1. Simulate network failure (disconnect network)
2. Run installation with GPU present
3. Verify fallback behavior

#### Expected Results
- ⚠️ Message: "⚠ Failed to download GPU libraries: [error]"
- ✅ Message: "Continuing with CPU-only mode"
- ✅ Manual download URL provided
- ✅ Installation continues (exit code 0)

### Sub-Scenario 9B: Insufficient Disk Space

#### Test Steps
1. Fill disk to near capacity
2. Attempt installation

#### Expected Results
- ⚠️ Warnings for directory creation failures
- ⚠️ Warnings for file write failures
- ✅ Installation continues where possible
- ✅ Clear error messages shown

### Sub-Scenario 9C: No Write Permission to Home Directory

#### Test Steps
1. Remove write permission from `~/.config`
2. Run installation

#### Expected Results
- ⚠️ Warning: "Could not create [directory]: EACCES"
- ✅ Installation continues
- ✅ User advised to fix permissions manually

### Sub-Scenario 9D: Invalid Template Files

#### Test Steps
1. Corrupt template file
2. Run installation

#### Expected Results
- ⚠️ Warning: "Template not found at [path]"
- ⚠️ Message: "Skipping daemon service generation"
- ✅ Installation continues
- ✅ User can manually create later

### Sub-Scenario 9E: Python Not Available

#### Test Steps
1. Run installation without Python installed
2. Bundled ORT library missing

#### Expected Results
- ⚠️ Warning: "Could not detect ONNX Runtime"
- ✅ Installation command provided: `pip3 install onnxruntime-gpu`
- ⚠️ Warning: "The daemon will not work correctly without this library!"
- ✅ Installation completes (exit code 0)

---

## Test Scenario 10: Upgrade Scenarios

### Test ID: TS-010
### Objective
Verify behavior when upgrading from previous versions.

### Sub-Scenario 10A: Upgrade from 0.2.x to 0.3.0

#### Prerequisites
- Swictation 0.2.x installed
- Old service files exist
- Old configuration exists

#### Test Steps
1. Run: `npm install -g swictation@0.3.0`
2. Check for version-specific migrations
3. Verify old services detected
4. Check configuration handling

#### Expected Results
- ✅ Old directories preserved
- ✅ Old service files detected (future: migration prompt)
- ✅ New service files generated alongside old ones
- ✅ Configuration preserved
- ✅ Model files preserved
- ⚠️ User notified about service file updates

### Sub-Scenario 10B: Reinstall Same Version

#### Test Steps
1. Install version 0.3.0
2. Run `npm install -g swictation@0.3.0` again

#### Expected Results
- ✅ No errors about existing files
- ✅ Permissions re-applied to binaries
- ✅ Service files regenerated with latest paths
- ✅ GPU libraries re-downloaded if missing
- ✅ Idempotent operation (safe to run multiple times)

---

## Test Scenario 11: Console Output and User Experience

### Test ID: TS-011
### Objective
Verify console output is clear, informative, and properly formatted.

### Output Quality Checklist

#### Visual Formatting
- ✅ Colors used appropriately (green=success, yellow=warning, red=error)
- ✅ Emoji/symbols used for clarity (✓, ✨, 📦, 🚀, etc.)
- ✅ Proper indentation and spacing
- ✅ Section headers clear and distinct
- ✅ Progress indicators present

#### Information Completeness
- ✅ Each step clearly announced
- ✅ File paths shown for generated files
- ✅ Hardware detection results displayed
- ✅ Model recommendation with rationale
- ✅ Next steps clearly outlined
- ✅ Installation commands shown
- ✅ Help command referenced

#### Error Messages
- ✅ Clear error descriptions
- ✅ Actionable solutions provided
- ✅ No technical jargon without explanation
- ✅ Links to documentation when relevant

### Example Expected Output
```
🚀 Setting up Swictation...

✓ Set execute permissions for swictation-daemon
✓ Set execute permissions for swictation-ui
✓ Set execute permissions for swictation
✓ Created directory: /home/user/.config/swictation
✓ Created directory: /home/user/.local/share/swictation/models

✓ NVIDIA GPU detected!
📦 Downloading GPU acceleration libraries...
  Downloading from: https://github.com/...
  ✓ Downloaded GPU libraries
  Extracting...
  ✓ Extracted GPU libraries
✓ GPU acceleration enabled!

🔍 Detecting ONNX Runtime library path...
✓ Found ONNX Runtime (GPU-enabled): [path]/lib/native/libonnxruntime.so

⚙️ Generating systemd service files...
✓ Created /home/user/.config/systemd/user
✓ Generated daemon service: [path]/swictation-daemon.service
✓ Installed UI service: [path]/swictation-ui.service

✨ Swictation installed successfully!

📊 System Detection:
   GPU detected: NVIDIA GeForce RTX 3060 (6GB VRAM)

🎯 Recommended Model:
   1.1B - Best quality - Full GPU acceleration with FP32 precision
   Size: ~75MB download (FP32 + INT8 versions)
   Performance: 62x realtime speed on GPU

Next steps:
  1. Download recommended AI model:
     pip install "huggingface_hub[cli]"
     swictation download-model 1.1b

  2. Run initial setup:
     swictation setup

  3. Start the service:
     swictation start

  4. Toggle recording with:
     swictation toggle

For more information:
  swictation help
```

---

## Test Scenario 12: Binary Permissions and Execution

### Test ID: TS-012
### Objective
Verify all binaries have correct permissions and are executable.

### Binaries to Check

| Binary Path | Required Permission | Executable |
|-------------|-------------------|------------|
| `bin/swictation` | 755 | ✅ |
| `bin/swictation-daemon` | 755 | ✅ |
| `bin/swictation-ui` | 755 | ✅ |
| `lib/native/swictation-daemon.bin` | 755 | ✅ |

### Test Steps
1. Complete installation
2. Check file permissions: `ls -la [file]`
3. Verify executability: `file [file]`
4. Test execution: `[file] --help` (where applicable)

### Expected Results
- ✅ All binaries have read+execute for owner, group, others (755)
- ✅ `file` command shows: "ELF 64-bit LSB executable"
- ✅ Binaries are executable from any directory
- ✅ Help command works (where implemented)

### Error Handling
- ⚠️ If chmod fails: Warning shown but installation continues
- ⚠️ If binary missing: Warning shown with path

---

## Test Scenario 13: Configuration File Generation

### Test ID: TS-013
### Objective
Verify configuration files are generated correctly when needed.

### Configuration Files

#### detected-environment.json
**Location**: `config/detected-environment.json`

**Created When**: ONNX Runtime detected via Python fallback

**Expected Content**:
```json
{
  "ORT_DYLIB_PATH": "/usr/lib/python3/dist-packages/onnxruntime/capi/libonnxruntime.so.1.15.1",
  "detected_at": "2025-11-13T18:00:00.000Z",
  "onnxruntime_version": "1.15.1",
  "warning": "Using Python pip installation - may be CPU-only"
}
```

**Validation**:
- ✅ Valid JSON format
- ✅ ORT_DYLIB_PATH is absolute path
- ✅ Version string matches installed package
- ✅ Timestamp in ISO format
- ✅ Warning message present

---

## Test Scenario 14: Cleanup and Idempotency

### Test ID: TS-014
### Objective
Verify installation can be run multiple times safely.

### Test Steps
1. Complete fresh installation
2. Run installation again immediately
3. Verify no errors or conflicts
4. Check all files updated correctly

### Expected Behavior
- ✅ Existing directories not recreated (no error)
- ✅ Binary permissions reset to 755
- ✅ Service files regenerated with latest values
- ✅ GPU libraries re-downloaded if missing
- ✅ ORT detection runs again
- ✅ No duplicate or conflicting files
- ✅ Installation completes successfully (exit code 0)

### Idempotency Validation
- ✅ Running installation N times = same result as running once
- ✅ No accumulation of files or duplicates
- ✅ Safe to run after partial installation failure

---

## Test Coverage Summary

### Function Coverage

| Function | Scenarios Testing |
|----------|------------------|
| `checkPlatform()` | TS-004 (A, B, C, D) |
| `ensureBinaryPermissions()` | TS-001, TS-005, TS-012, TS-014 |
| `createDirectories()` | TS-001, TS-005, TS-014 |
| `checkDependencies()` | TS-007 |
| `detectNvidiaGPU()` | TS-002 (A, B, C) |
| `downloadFile()` | TS-002, TS-009-A |
| `downloadGPULibraries()` | TS-002 (A, B, C), TS-009-A |
| `detectOrtLibrary()` | TS-003 (A, B, C) |
| `generateSystemdService()` | TS-006, TS-009-D |
| `detectSystemCapabilities()` | TS-008 |
| `recommendOptimalModel()` | TS-008 |
| `showNextSteps()` | TS-011 |
| `main()` | All scenarios |

### Error Path Coverage
- ✅ Network failures (TS-009-A)
- ✅ Disk space issues (TS-009-B)
- ✅ Permission errors (TS-009-C)
- ✅ Missing files (TS-009-D)
- ✅ Missing dependencies (TS-009-E)
- ✅ Platform incompatibility (TS-004-B, C, D)
- ✅ GLIBC version mismatch (TS-004-B)

### Edge Cases Coverage
- ✅ No GPU present (TS-002-C)
- ✅ Low VRAM GPU (TS-002-B)
- ✅ No Python installed (TS-009-E)
- ✅ Corrupted template (TS-009-D)
- ✅ Existing installation (TS-010, TS-014)
- ✅ Multiple GPUs (covered in TS-002-A)
- ✅ No systemd (TS-007)

---

## Test Execution Checklist

### Before Testing
- [ ] Clean test environment prepared
- [ ] All prerequisites documented
- [ ] Test data/mocks prepared
- [ ] Version numbers verified
- [ ] Network connection verified (for download tests)

### During Testing
- [ ] Console output captured
- [ ] Screenshots taken for visual verification
- [ ] Timestamps recorded
- [ ] Error messages documented verbatim
- [ ] File contents verified
- [ ] Permissions checked

### After Testing
- [ ] Test results documented
- [ ] Failures analyzed and categorized
- [ ] Bug reports filed (if needed)
- [ ] Success criteria validation
- [ ] Test artifacts archived
- [ ] Summary report created

---

## Test Results Template

### Test Execution Record

**Test ID**: _____________
**Test Name**: _____________
**Date**: _____________
**Tester**: _____________
**Environment**: _____________
**Version**: _____________

**Result**: [ ] PASS  [ ] FAIL  [ ] BLOCKED  [ ] SKIP

**Actual Results**:
```
[Paste console output or describe behavior]
```

**Deviations from Expected**:
```
[List any differences from expected results]
```

**Screenshots/Logs**:
- [ ] Console output: _____________
- [ ] File listings: _____________
- [ ] Service files: _____________
- [ ] Error logs: _____________

**Notes**:
```
[Additional observations or comments]
```

**Defects Found**:
1. [Bug ID]: [Description]
2. [Bug ID]: [Description]

---

## Appendix A: Test Data Setup

### Creating Test Environments

#### Docker Test Environment (Clean Ubuntu 24.04)
```bash
docker run -it --rm ubuntu:24.04 /bin/bash
apt update && apt install -y nodejs npm python3 python3-pip
```

#### Mock GPU Environment
```bash
# Create mock nvidia-smi for testing
cat > /usr/local/bin/nvidia-smi << 'EOF'
#!/bin/bash
echo "6144" # Mock 6GB VRAM
EOF
chmod +x /usr/local/bin/nvidia-smi
```

#### Mock Low VRAM GPU
```bash
cat > /usr/local/bin/nvidia-smi << 'EOF'
#!/bin/bash
echo "2048" # Mock 2GB VRAM
EOF
```

#### Remove GPU for CPU-only Testing
```bash
rm /usr/local/bin/nvidia-smi
```

---

## Appendix B: Verification Commands

### Quick Verification Script
```bash
#!/bin/bash
# Quick verification of installation

echo "=== Directory Check ==="
ls -ld ~/.config/swictation
ls -ld ~/.local/share/swictation
ls -ld ~/.cache/swictation

echo -e "\n=== Binary Permissions ==="
ls -la $(npm root -g)/swictation/bin/

echo -e "\n=== Service Files ==="
ls -la ~/.config/systemd/user/swictation*.service

echo -e "\n=== GPU Libraries ==="
ls -la $(npm root -g)/swictation/lib/native/

echo -e "\n=== Service File Content ==="
grep "ORT_DYLIB_PATH" ~/.config/systemd/user/swictation-daemon.service

echo -e "\n=== Binary Execution ==="
swictation --help
```

### Full System Check
```bash
#!/bin/bash
echo "GPU Detection:"
nvidia-smi --query-gpu=name,memory.total --format=csv

echo -e "\nPlatform:"
uname -a

echo -e "\nGLIBC Version:"
ldd --version | head -1

echo -e "\nNode Version:"
node --version

echo -e "\nPython ONNX Runtime:"
python3 -c "import onnxruntime; print(onnxruntime.__version__)" 2>/dev/null || echo "Not installed"

echo -e "\nDependencies:"
for cmd in systemctl nc wtype xdotool hf; do
    which $cmd &>/dev/null && echo "✓ $cmd" || echo "✗ $cmd"
done
```

---

**Document Version**: 1.0
**Last Updated**: 2025-11-13
**Status**: Ready for Testing
