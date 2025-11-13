# Testing Swictation v0.3.1

This guide provides comprehensive testing scenarios for v0.3.1 release validation.

---

## 🎯 What to Test in v0.3.1

### New Features
1. ✅ Real model test-loading during installation (~30-60s)
2. ✅ Old service cleanup on upgrades
3. ✅ Interactive config migration (pacman/apt-style)
4. ✅ Intelligent VRAM-based selection with verification
5. ✅ Fixed model thresholds (6GB for 1.1B, 4GB for 0.6B)
6. ✅ Robust error handling with graceful fallback

### Bug Fixes
- Model selection failure on 4GB VRAM GPUs
- Service installation conflicts during upgrades
- Config file conflicts on updates

---

## 📋 Test Environment Requirements

### Hardware Test Matrix

| GPU VRAM | Expected Model | Test Scenario |
|----------|----------------|---------------|
| 8GB+     | 1.1B (FP32)    | High-end GPU  |
| 4-6GB    | 0.6B           | Limited VRAM  |
| <4GB     | CPU-only       | Insufficient  |
| None     | CPU-only       | No GPU        |

### Test Systems
- **System A**: High-end GPU (RTX 3060, 12GB VRAM)
- **System B**: Mid-range GPU (RTX A1000, 4GB VRAM) ⭐ Critical test case
- **System C**: CPU-only (no GPU)
- **System D**: Headless/CI (no TTY, SKIP_MODEL_TEST=1)

---

## 🧪 Test Scenarios

### Scenario 1: Fresh Install (High-End GPU)

**System**: RTX 3060 (12GB VRAM)

**Steps**:
```bash
# Clean environment
sudo rm -rf ~/.config/swictation
sudo rm -f ~/.config/systemd/user/swictation-*.service

# Install
npm install -g swictation
```

**Expected Output**:
```
🚀 Setting up Swictation...

═══ Phase 1: Service Cleanup ═══
✓ No old service files found

═══ Phase 2: Configuration ═══
✓ Created config file: ~/.config/swictation/config.toml

═══ Phase 3: GPU Detection ═══
🎮 Detecting GPU capabilities...
✓ GPU Detected: NVIDIA GeForce RTX 3060
  VRAM: 12GB (12288MB)
  Driver: 535.xx
  CUDA: 12.2
  ✓ Sufficient VRAM for 1.1B model (best quality)

🧪 Testing model loading...
  Testing 1.1b-gpu model...
  ✓ Model 1.1b test-loaded successfully
    Your system can load and run the 1.1b-gpu model

✓ GPU acceleration enabled!

═══ Phase 4: Service Installation ═══
✓ Generated daemon service: ~/.config/systemd/user/swictation-daemon.service
✓ Installed UI service: ~/.config/systemd/user/swictation-ui.service

✨ Swictation installed successfully!

📊 System Detection:
   GPU: NVIDIA GeForce RTX 3060 (12GB VRAM)
   Driver: 535.xx
   CUDA: 12.2

🎯 Recommended Model:
   1.1B - Best quality - Full GPU acceleration with FP32 precision
```

**Verification**:
```bash
# Check GPU info
cat ~/.config/swictation/gpu-info.json
# Should show: "recommendedModel": "1.1b"

# Check services
ls -l ~/.config/systemd/user/swictation-*.service

# Start daemon
systemctl --user daemon-reload
systemctl --user start swictation-daemon
systemctl --user status swictation-daemon

# Check logs for model loading
journalctl --user -u swictation-daemon -n 50 | grep -i model
# Should show: Loading 1.1B model
```

**Pass Criteria**:
- ✅ 1.1B model recommended
- ✅ Model test-loaded successfully
- ✅ Services installed
- ✅ Daemon starts without errors
- ✅ Config created with correct model

---

### Scenario 2: Fresh Install (Limited VRAM) ⭐ CRITICAL

**System**: RTX A1000 (4GB VRAM)

**Steps**:
```bash
# Clean environment
sudo rm -rf ~/.config/swictation
sudo rm -f ~/.config/systemd/user/swictation-*.service

# Install
npm install -g swictation
```

**Expected Output**:
```
═══ Phase 3: GPU Detection ═══
✓ GPU Detected: NVIDIA RTX A1000 Laptop GPU
  VRAM: 4GB (4096MB)
  Driver: 535.xx
  CUDA: 12.2
  ⚠️  Limited VRAM - Recommending 0.6B model
     (1.1B model requires ~6GB VRAM)

🧪 Testing model loading...
  Testing 0.6b-gpu model...
  ✓ Model 0.6b test-loaded successfully
    Your system can load and run the 0.6b-gpu model

🎯 Recommended Model:
   0.6B - Lighter model for limited VRAM systems
```

**Verification**:
```bash
# Check GPU info
cat ~/.config/swictation/gpu-info.json
# Should show: "recommendedModel": "0.6b", "vramMB": 4096

# Start daemon and check logs
systemctl --user start swictation-daemon
journalctl --user -u swictation-daemon -n 50 | grep -i model
# Should show: Loading 0.6B model (NOT 1.1B)

# Verify VRAM usage doesn't exceed ~3.5GB
nvidia-smi
# VRAM usage should be ~3.5GB, NOT ~6GB
```

**Pass Criteria**:
- ✅ 0.6B model recommended (NOT 1.1B)
- ✅ Model test-loaded successfully
- ✅ Daemon starts without OOM errors
- ✅ VRAM usage stays under 3.8GB
- ✅ No "Failed to allocate memory" errors

**This is the bug we fixed!** Previously this would have:
- ❌ Recommended 1.1B model
- ❌ Failed to allocate ~4GB during model loading
- ❌ Crashed daemon with OOM error

---

### Scenario 3: Upgrade from v0.3.0

**System**: Any GPU system with v0.3.0 installed

**Initial State**:
```bash
# Verify v0.3.0 installed
npm list -g swictation
# Should show: swictation@0.3.0

# Check existing services
systemctl --user status swictation-daemon
# Should be running

# Check existing config
cat ~/.config/swictation/config.toml
# Should have v0.3.0 config
```

**Upgrade Steps**:
```bash
# Upgrade to v0.3.1
npm install -g swictation@0.3.1
```

**Expected Output**:
```
═══ Phase 1: Service Cleanup ═══
🧹 Checking for old service files...
⚠️  Found old service file: ~/.config/systemd/user/swictation-daemon.service
  ✓ Stopped service: swictation-daemon.service
  ✓ Disabled service: swictation-daemon.service
  ✓ Removed old service file
✓ Reloaded systemd daemon

═══ Phase 2: Configuration ═══
📝 Checking configuration files...
⚠️  Config file exists and differs from new template

Options:
  [K] Keep    - Keep your current config (default)
  [N] New     - Replace with new config (backup old)
  [M] Merge   - Keep old, add new required fields
  [D] Diff    - Show differences
  [S] Skip    - Continue without changes

⚠️  Interactive mode not available during postinstall
   Defaulting to: Keep existing config
   New config template available at: [path]

✓ Kept existing config

═══ Phase 3: GPU Detection ═══
[GPU detection and model test-loading...]

═══ Phase 4: Service Installation ═══
✓ Generated daemon service: [new service with updated paths]
✓ Installed UI service
```

**Verification**:
```bash
# Check version
npm list -g swictation
# Should show: swictation@0.3.1

# Check services updated
systemctl --user daemon-reload
systemctl --user status swictation-daemon
# Should show new service file path

# Config preserved
diff ~/.config/swictation/config.toml ~/.config/swictation/config.toml.backup
# Should show no differences (config kept)

# Restart daemon
systemctl --user restart swictation-daemon
systemctl --user status swictation-daemon
# Should start successfully
```

**Pass Criteria**:
- ✅ Old services cleaned up before new install
- ✅ Config migration handled gracefully
- ✅ New services installed successfully
- ✅ Daemon restarts without errors
- ✅ No service file conflicts

---

### Scenario 4: CPU-Only System

**System**: No NVIDIA GPU

**Steps**:
```bash
npm install -g swictation
```

**Expected Output**:
```
═══ Phase 3: GPU Detection ═══
🎮 Detecting GPU capabilities...
  No NVIDIA GPU detected - CPU mode will be used

ℹ No NVIDIA GPU detected - skipping GPU library download
  CPU-only mode will be used

🎯 Recommended Model:
   CPU-optimized models
   Multiple sizes available (0.6B - 1.1B)
```

**Pass Criteria**:
- ✅ Detects no GPU correctly
- ✅ Skips GPU library download
- ✅ Recommends CPU models
- ✅ Services installed
- ✅ Daemon can run in CPU mode

---

### Scenario 5: Headless/CI Environment

**System**: Any, but non-interactive (CI/automation)

**Steps**:
```bash
SKIP_MODEL_TEST=1 npm install -g swictation
```

**Expected Output**:
```
[No interactive prompts]
[No TTY warnings]

✓ Non-interactive mode: Keeping existing config
  Tip: Run "swictation setup" to review config changes

ℹ️  Skipping model test-loading (SKIP_MODEL_TEST=1 set)
  Models will be validated at runtime
```

**Pass Criteria**:
- ✅ No interactive prompts
- ✅ No TTY errors
- ✅ Installation completes without hanging
- ✅ Config defaults to "Keep"
- ✅ Model test-loading skipped
- ✅ Services installed

---

### Scenario 6: Model Test-Loading Failure

**System**: Any GPU, but simulate failure

**Setup**:
```bash
# Temporarily remove model files to force failure
rm -rf ~/.cache/swictation/models/*
```

**Steps**:
```bash
npm install -g swictation
```

**Expected Output**:
```
🧪 Testing model loading...
  Testing 1.1b-gpu model...
  ⚠️  Model test-loading failed (will use runtime fallback)
    Test timed out - model may be downloading or system is slow
    The daemon will handle model loading at runtime

✨ Swictation installed successfully!
```

**Pass Criteria**:
- ✅ Test-loading times out gracefully (30s)
- ✅ Installation continues (doesn't fail)
- ✅ Warning shown but not error
- ✅ Services still installed
- ✅ Daemon will download model at first run

---

## 🐛 Regression Tests

### Test 1: RTX A1000 (4GB) - Original Bug

**Bug**: v0.3.0 selected 1.1B model for 4GB VRAM, causing OOM crash

**Test**:
```bash
# Install v0.3.1 on RTX A1000 (4GB)
npm install -g swictation
```

**Expected**: 0.6B model selected, no OOM errors
**Pass**: ✅ 0.6B selected, daemon starts successfully

---

### Test 2: Service File Conflicts on Upgrade

**Bug**: Old Python services conflicted with new Node.js services

**Test**:
```bash
# Create old service file
mkdir -p ~/.config/systemd/user
cat > ~/.config/systemd/user/swictation-daemon.service <<EOF
[Unit]
Description=Old Python Service
[Service]
ExecStart=/usr/bin/python3 /old/path/daemon.py
EOF

# Upgrade to v0.3.1
npm install -g swictation@0.3.1
```

**Expected**: Old service cleaned up, new service installed
**Pass**: ✅ Old service removed, new service installed

---

### Test 3: Config Conflicts on Update

**Bug**: New config overwrote user config without asking

**Test**:
```bash
# Modify config
echo "# My custom config" >> ~/.config/swictation/config.toml

# Upgrade
npm install -g swictation@0.3.1
```

**Expected**: Prompt to keep/replace/merge (or default to keep in CI)
**Pass**: ✅ Config preserved, prompt shown

---

## 📊 Performance Tests

### Test 1: Installation Time

**Measure**: Time from `npm install` start to completion

**Expected**:
- Without test-loading: 10-30s
- With test-loading: 40-90s (30-60s for model test)
- With SKIP_MODEL_TEST=1: 10-30s

**Pass**: ✅ Times within expected range

---

### Test 2: Model Test-Loading Time

**Measure**: Time for test-loading phase

**Expected**:
- 0.6B model: 20-40s
- 1.1B model: 30-60s
- Timeout: 30s max if fails

**Pass**: ✅ Times within expected range

---

### Test 3: Service Startup Time

**Measure**: Time from `systemctl start` to "Ready" state

**Command**:
```bash
time systemctl --user start swictation-daemon
systemctl --user status swictation-daemon
```

**Expected**: 2-5s (model already loaded during install test)

**Pass**: ✅ Starts within 5s

---

## ✅ Test Report Template

### Test Execution Checklist

**Tester**: _________________
**Date**: _________________
**System**: _________________

| Scenario | System | Pass | Notes |
|----------|--------|------|-------|
| Fresh Install (High-End) | RTX 3060 (12GB) | ☐ | |
| Fresh Install (Limited) | RTX A1000 (4GB) | ☐ | ⭐ Critical |
| Upgrade from v0.3.0 | Any GPU | ☐ | |
| CPU-Only | No GPU | ☐ | |
| Headless/CI | Any (non-TTY) | ☐ | |
| Test-Loading Failure | Any GPU | ☐ | |
| RTX A1000 Regression | 4GB VRAM | ☐ | Bug fix |
| Service Conflict Regression | Upgrade | ☐ | Bug fix |
| Config Conflict Regression | Upgrade | ☐ | Bug fix |

### Observations

**Issues Found**:
-

**Performance**:
- Installation time: _____s
- Test-loading time: _____s
- Service startup: _____s

**Recommendations**:
-

---

## 🚨 Known Issues to Watch

### Issue 1: Model Download During Test-Loading
**Symptom**: Test-loading times out because model is downloading
**Expected**: Warning shown, installation continues
**Fix**: User downloads model manually after install

### Issue 2: CUDA Version Mismatch
**Symptom**: Test-loading fails with CUDA error
**Expected**: Falls back gracefully, shows warning
**Fix**: User installs correct CUDA version

### Issue 3: Insufficient Disk Space
**Symptom**: Test-loading fails, model can't be cached
**Expected**: Warning shown, installation continues
**Fix**: User frees disk space, daemon downloads model at runtime

---

## 📞 Reporting Issues

If you encounter problems during testing:

1. **Gather logs**:
   ```bash
   journalctl --user -u swictation-daemon -n 100 > daemon.log
   cat ~/.config/swictation/gpu-info.json > gpu-info.log
   npm install -g swictation 2>&1 | tee install.log
   ```

2. **System info**:
   ```bash
   nvidia-smi > nvidia.log
   uname -a > system.log
   cat /etc/os-release > distro.log
   ```

3. **Create issue**:
   - Go to: https://github.com/robertelee78/swictation/issues
   - Title: `[v0.3.1] Brief description`
   - Attach logs and system info
   - Include test scenario that failed

---

## 🎉 Success Criteria

v0.3.1 is ready for release when:

- ✅ All 9 test scenarios pass
- ✅ All 3 regression tests pass
- ✅ Performance within expected ranges
- ✅ No critical bugs found
- ✅ Works on all GPU VRAM sizes (8GB+, 4-6GB, <4GB, none)
- ✅ Graceful fallback on test-loading failure
- ✅ Clean upgrades from v0.3.0
- ✅ CI/headless environments work

---

**Status**: Ready for Testing
**Last Updated**: 2025-11-13
