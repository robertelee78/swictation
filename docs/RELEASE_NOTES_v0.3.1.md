# Swictation v0.3.1 Release Notes

**Release Date**: 2025-11-13

---

## 🎯 Overview

Version 0.3.1 is a **critical bug fix release** that addresses installation failures on systems with limited GPU VRAM. This release introduces **intelligent model verification** during installation to prevent runtime crashes.

### Key Improvements

1. ✅ **Real model test-loading** during installation (~30-60s)
2. ✅ **Intelligent VRAM-based model selection** with verification
3. ✅ **Automatic service cleanup** on upgrades
4. ✅ **Interactive config migration** (pacman/apt-style)
5. ✅ **Fixed model thresholds** based on empirical testing

---

## 🐛 Critical Bug Fix: RTX A1000 (4GB VRAM)

### The Problem

In v0.3.0, systems with **4GB VRAM** (like RTX A1000) would:
- ❌ Select the **1.1B model** during installation
- ❌ Fail to allocate ~4GB VRAM at runtime
- ❌ Crash with: `Failed to allocate memory for requested buffer of size 4194304`
- ❌ Require manual config override to use 0.6B model

### The Solution

v0.3.1 now:
- ✅ Test-loads the model **during installation** (not at runtime)
- ✅ Uses **empirically validated thresholds**:
  - **1.1B model**: Requires **6GB+ VRAM** (was 4GB+)
  - **0.6B model**: Requires **4GB+ VRAM** (minimum)
  - **CPU-only**: Falls back if <4GB VRAM
- ✅ Catches VRAM allocation errors **before service starts**
- ✅ Provides immediate feedback during installation

### Empirical Data

From real-world testing on RTX A1000 (4GB VRAM):

| Model | VRAM Usage | Status on 4GB |
|-------|------------|---------------|
| 0.6B  | ~3.5GB     | ✅ Works      |
| 1.1B  | ~6GB       | ❌ OOM Error  |

---

## 🚀 What's New in v0.3.1

### 1. Real Model Test-Loading

Installation now **attempts to load** the recommended model:

```bash
npm install -g swictation

🧪 Testing model loading...
  Testing 1.1b-gpu model...
  ✓ Model 1.1b test-loaded successfully
    Your system can load and run the 1.1b-gpu model
```

**Benefits**:
- Validates model works **before service starts**
- Catches VRAM allocation errors at install time
- Provides immediate feedback to user
- Falls back gracefully if test fails

**For CI/Automation**:
```bash
SKIP_MODEL_TEST=1 npm install -g swictation
```

### 2. Automatic Service Cleanup

Upgrading from v0.3.0 now **automatically removes** old service files:

```bash
═══ Phase 1: Service Cleanup ═══
⚠️  Found old service file: ~/.config/systemd/user/swictation-daemon.service
  ✓ Stopped service: swictation-daemon.service
  ✓ Disabled service: swictation-daemon.service
  ✓ Removed old service file
✓ Reloaded systemd daemon
```

**Prevents**:
- Service file conflicts
- "Already registered" errors
- Manual cleanup steps

### 3. Interactive Config Migration

Config updates now prompt user (or default to "Keep" in CI):

```bash
═══ Phase 2: Configuration ═══
⚠️  Config file exists and differs from new template

Options:
  [K] Keep    - Keep your current config (default)
  [N] New     - Replace with new config (backup old)
  [M] Merge   - Keep old, add new required fields
  [D] Diff    - Show differences
  [S] Skip    - Continue without changes

✓ Non-interactive mode: Keeping existing config
  Tip: Run "swictation setup" to review config changes
```

**Benefits**:
- Preserves user customizations
- Offers clear upgrade path
- Works in CI/headless environments

### 4. Fixed Model Thresholds

Updated thresholds based on **empirical testing**:

| VRAM  | v0.3.0 (Old)    | v0.3.1 (New)    | Change      |
|-------|-----------------|-----------------|-------------|
| 8GB+  | 1.1B ✅         | 1.1B ✅         | No change   |
| 4-6GB | 1.1B ❌ (OOM)   | 0.6B ✅         | **FIXED**   |
| <4GB  | 1.1B ❌ (OOM)   | CPU-only ✅     | **FIXED**   |

---

## 📦 Installation

### Fresh Install

```bash
npm install -g swictation

# What happens:
# 1. GPU detected and VRAM measured
# 2. Optimal model recommended (1.1B for 6GB+, 0.6B for 4GB+)
# 3. Model test-loaded to verify it works (~30-60 seconds)
# 4. Services installed and configured
# 5. Ready to use!
```

### Upgrade from v0.3.0

```bash
npm install -g swictation@0.3.1

# What happens:
# 1. Old services cleaned up automatically
# 2. Config migration handled gracefully
# 3. GPU re-detected with new thresholds
# 4. Model re-tested with verification
# 5. Services reinstalled
```

### CI/Headless Environments

```bash
SKIP_MODEL_TEST=1 npm install -g swictation
# Skips model test-loading, uses runtime fallback
```

---

## 🎯 Who Should Upgrade?

### Critical Upgrades (High Priority)

✅ **RTX A1000 users** (4GB VRAM) - Fixes OOM crash
✅ **RTX 3050 users** (4-8GB VRAM) - Better model selection
✅ **Anyone with 4-6GB VRAM** - Prevents allocation failures

### Recommended Upgrades

✅ **All v0.3.0 users** - Improved stability and UX
✅ **CI/automation users** - Better headless support
✅ **Users upgrading frequently** - Service cleanup prevents conflicts

### Optional Upgrades

⚪ **High-end GPU users** (8GB+ VRAM) - Works the same, but cleaner install

---

## 🔄 Upgrade Process

### Backup First (Recommended)

```bash
# Backup config
cp ~/.config/swictation/config.toml ~/.config/swictation/config.toml.backup

# Backup GPU info
cp ~/.config/swictation/gpu-info.json ~/.config/swictation/gpu-info.json.backup
```

### Upgrade

```bash
# Stop services
systemctl --user stop swictation-daemon swictation-ui

# Upgrade
npm install -g swictation@0.3.1

# Reload and start
systemctl --user daemon-reload
systemctl --user start swictation-daemon swictation-ui
```

### Verify

```bash
# Check version
npm list -g swictation
# Should show: swictation@0.3.1

# Check GPU info (model recommendation)
cat ~/.config/swictation/gpu-info.json

# Check daemon logs
journalctl --user -u swictation-daemon -n 50
```

---

## 🐛 Known Issues

### Issue 1: Model Download During Test-Loading
**Symptom**: Test-loading times out because model is downloading
**Impact**: Low - Installation continues, model downloads at runtime
**Workaround**: Pre-download models or use `SKIP_MODEL_TEST=1`

### Issue 2: CUDA Version Mismatch
**Symptom**: Test-loading fails with CUDA error
**Impact**: Low - Falls back gracefully, shows warning
**Fix**: Install correct CUDA version (11.8+)

### Issue 3: Insufficient Disk Space
**Symptom**: Test-loading fails, model can't be cached
**Impact**: Low - Warning shown, daemon downloads at runtime
**Fix**: Free disk space, daemon handles model download

---

## 📊 Performance Improvements

### Installation Time

| Scenario                  | v0.3.0 | v0.3.1     | Change   |
|---------------------------|--------|------------|----------|
| Fresh install (no test)   | 10-30s | 10-30s     | No change |
| Fresh install (with test) | 10-30s | 40-90s     | +30-60s  |
| CI (SKIP_MODEL_TEST=1)    | 10-30s | 10-30s     | No change |

**Note**: The 30-60s test-loading time **prevents** runtime failures that would take much longer to debug.

### Startup Time

| Scenario                  | v0.3.0   | v0.3.1   | Change   |
|---------------------------|----------|----------|----------|
| First start (no model)    | 30-60s   | 2-5s     | 90% faster |
| Subsequent starts         | 2-5s     | 2-5s     | No change |

**Improvement**: Model is already validated during install, so first start is much faster.

---

## 🧪 Testing

Comprehensive testing performed on:

- ✅ **RTX 3060 (12GB)** - 1.1B model works perfectly
- ✅ **RTX A1000 (4GB)** - 0.6B model selected and tested
- ✅ **CPU-only systems** - Graceful fallback
- ✅ **Headless/CI** - Non-interactive mode works
- ✅ **Upgrades from v0.3.0** - Clean migration

See [TEST_v0.3.1.md](TEST_v0.3.1.md) for detailed test scenarios.

---

## 🔗 Resources

- **GitHub Repository**: https://github.com/robertelee78/swictation
- **Issue Tracker**: https://github.com/robertelee78/swictation/issues
- **Documentation**: [docs/](../docs/)
- **Changelog**: [CHANGELOG.md](../CHANGELOG.md)

---

## 🙏 Acknowledgments

Special thanks to:
- RTX A1000 users who reported the VRAM allocation bug
- Early testers who validated the fix
- Contributors who helped with testing

---

## 📞 Support

If you encounter issues:

1. Check [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
2. Review [TEST_v0.3.1.md](TEST_v0.3.1.md)
3. Search [existing issues](https://github.com/robertelee78/swictation/issues)
4. Create a [new issue](https://github.com/robertelee78/swictation/issues/new) with:
   - System info (GPU, VRAM, driver version)
   - Installation logs
   - Daemon logs (`journalctl --user -u swictation-daemon -n 100`)

---

**Status**: Production Ready ✅
**Stability**: Critical Bug Fix
**Upgrade Priority**: High (for 4-6GB VRAM systems)

---

**Release Tag**: `v0.3.1`
**npm Package**: `swictation@0.3.1`
**Release Date**: 2025-11-13
