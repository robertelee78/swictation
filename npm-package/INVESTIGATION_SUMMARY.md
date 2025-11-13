# v0.3.1 npm Installation Investigation Summary

**Date**: November 13, 2025
**Task**: aa21b713-0664-4f80-92d7-009f2dc47a24
**Status**: ✅ COMPLETE - Ready for Review

---

## 🎯 Executive Summary

**Bottom Line**: The npm package IS working correctly. All native libraries are properly bundled, and the daemon executes successfully when system dependencies are met.

We identified **3 actionable items** that must be completed before publishing to npm:
1. Create GitHub v0.3.1 release with GPU libs (BLOCKING)
2. Make postinstall output visible by default (HIGH)
3. Document libasound2t64 system requirement (MEDIUM)

---

## ✅ What We Proved Works

### 1. Library Bundling ✓
- All .so files and binaries correctly included (35 MB total)
- `lib/native/` directory properly packaged
- Verified with `npm pack` and tarball inspection
- Files array in package.json is correct

### 2. Library Path Resolution ✓
- LD_LIBRARY_PATH correctly set in postinstall
- Service files have correct ORT_DYLIB_PATH
- Daemon finds libsherpa-onnx-c-api.so and libonnxruntime.so
- Confirmed with `ldd` in Docker tests

### 3. Daemon Execution ✓
- Binary runs with `--help`, `--test-model`, `--dry-run`
- CLI wrapper functional
- All shared library dependencies satisfied (with libasound2t64)

### 4. Docker Test Matrix ✓
| Component | Node 18 | Node 20 | Node 22 |
|-----------|---------|---------|---------|
| Package install | ✅ | ✅ | ✅ |
| Libraries bundled | ✅ | ✅ | ✅ |
| Daemon executable | ✅ | ✅ | ✅ |
| CLI works | ✅ | ✅ | ✅ |

---

## 🔍 Issues Identified

### Issue #1: Postinstall Output Hidden
**Symptom**: User saw no output during `npm install -g`
**Root Cause**: npm hides postinstall stdout by default
**Status**: ✅ IDENTIFIED

**Evidence**:
```bash
$ npm install -g ./swictation-0.3.1.tgz
# (silent - no output)

$ npm install -g ./swictation-0.3.1.tgz --foreground-scripts
# (full colorized output visible)
```

**Options to Fix**:
- **A)** Use `console.log()` instead of custom `log()` function
- **B)** Document `--foreground-scripts` flag requirement in README
- **C)** Add to package.json: `"config": {"foreground-scripts": true}`

### Issue #2: GPU Libs Download Fails (404)
**Symptom**: `gzip: stdin: not in gzip format` during postinstall
**Root Cause**: GitHub v0.3.1 release doesn't exist
**Status**: ✅ IDENTIFIED - BLOCKING

**URL That Fails**:
```
https://github.com/robertelee78/swictation/releases/download/v0.3.1/swictation-gpu-libs.tar.gz
HTTP/2 404
```

**Required Action**:
1. Create GitHub tag: `git tag v0.3.1 && git push origin v0.3.1`
2. Create release on GitHub
3. Upload `swictation-gpu-libs.tar.gz` (209 MB)
4. Verify URL returns 200 before npm publish

### Issue #3: Missing System Dependency
**Symptom**: `libasound.so.2: cannot open shared object file`
**Root Cause**: ALSA library not documented
**Status**: ✅ IDENTIFIED

**Required Package**:
- Ubuntu 24.04+: `libasound2t64`
- Ubuntu 22.04 and older: `libasound2`

**Fix**: Add to README:
```markdown
## System Requirements
- libasound2t64 (Ubuntu 24.04+)
  ```bash
  sudo apt-get install libasound2t64
  ```
```

---

## 📁 Deliverables Created

### Documentation (713 lines)
1. **INSTALL_NOTES.md**
   - System requirements
   - Known issues & workarounds
   - Installation verification
   - Test results

2. **NPM_BUILD_PROCESS.md**
   - Complete build workflow
   - Pre-publish checklist
   - Testing procedures
   - Rollback process

3. **INVESTIGATION_SUMMARY.md** (this file)
   - Executive summary
   - Findings and action items
   - Next steps

### Test Scripts
1. **docker-postinstall-debug.sh**
   - Shows postinstall output in detail
   - Tests library loading with ldd
   - Verifies daemon execution
   - Runtime: ~40s

2. **docker-full-install-test.sh**
   - Installs system dependencies
   - Runs complete validation
   - All tests passing
   - Runtime: ~50s

### Git Commit
```
commit 9c9e76d7
docs: Add comprehensive npm installation testing and build process documentation
```

---

## 📊 Test Results

### Library Verification
```bash
$ ls -lh /usr/lib/node_modules/swictation/lib/native/
-rw-r--r--  22M libonnxruntime.so
-rw-r--r--  15K libonnxruntime_providers_shared.so
-rw-r--r-- 3.8M libsherpa-onnx-c-api.so
-rw-r--r--  84K libsherpa-onnx-cxx-api.so
-rwxr-xr-x 8.6M swictation-daemon.bin
✅ All present
```

### Daemon Execution
```bash
$ /usr/lib/node_modules/swictation/lib/native/swictation-daemon.bin --help
Voice-to-text dictation daemon with adaptive model selection

Usage: swictation-daemon.bin [OPTIONS]

Options:
      --test-model <MODEL>  Override STT model selection
      --dry-run             Dry-run: show model selection
  -h, --help                Print help
✅ Works correctly
```

### Shared Library Dependencies
```bash
$ ldd swictation-daemon.bin | grep "not found"
(no output with libasound2t64 installed)
✅ All dependencies satisfied
```

---

## 🚦 Pre-Publish Checklist

### BLOCKING (Must Complete)
- [ ] **Create GitHub v0.3.1 release**
  - Create tag: `git tag v0.3.1 && git push origin v0.3.1`
  - Create release on GitHub
  - Upload swictation-gpu-libs.tar.gz (209 MB)
  - Verify URL accessible (200, not 404)

### HIGH Priority
- [ ] **Make postinstall output visible**
  - Choose option A, B, or C above
  - Test with fresh install

- [ ] **Update README system requirements**
  - Add libasound2t64 requirement
  - Document --foreground-scripts flag

### MEDIUM Priority
- [ ] **Test on local dev system**
  - Install with: `sudo npm install -g ./swictation-0.3.1.tgz --foreground-scripts`
  - Verify GPU libs download (after GitHub release created)
  - Test model test-loading on NVIDIA RTX PRO 6000

### After npm Publish
- [ ] **Production system test** (192.168.1.133)
  - Fresh install from npm registry
  - Verify GPU acceleration works
  - Test dictation functionality

---

## 🎓 Key Learnings

1. **npm hides postinstall by default** - Need `--foreground-scripts` or console.log()
2. **Docker paths differ from dev** - Use `npm root -g` for portable tests
3. **System dependencies critical** - libasound2t64 required, must document
4. **GitHub release must exist first** - Can't download GPU libs if URL 404s
5. **Package structure is correct** - Libraries bundled, paths resolved
6. **Docker tests are essential** - Caught real issues in clean environment

---

## 📞 Next Actions

### Immediate
1. Review this investigation with team
2. Decide on postinstall output solution (A, B, or C)
3. Create GitHub v0.3.1 release
4. Update README

### Before npm Publish
1. Complete pre-publish checklist
2. Test on local system
3. Verify all action items closed

### After npm Publish
1. Test on production system (192.168.1.133)
2. Verify end-to-end functionality
3. Update Archon task to 'done'
4. Celebrate! 🎉

---

## 📚 Reference Files

- **Task**: Archon aa21b713-0664-4f80-92d7-009f2dc47a24
- **Logs**: `/tmp/install-foreground.log`, `/tmp/fresh-install.log`
- **Tests**: `npm-package/tests/docker-*.sh`
- **Docs**: `npm-package/INSTALL_NOTES.md`, `npm-package/NPM_BUILD_PROCESS.md`
- **Git**: Commit 9c9e76d7, branch main

---

**Investigation Status**: ✅ COMPLETE
**Package Status**: ✅ WORKING
**Ready for Publish**: ⏳ After action items completed
