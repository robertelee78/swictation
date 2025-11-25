# macOS Daemon Diagnostic Report
**Date:** 2025-11-25
**Machine:** eddie@192.168.1.17
**OS:** macOS 15.6.1 (24G90)
**Architecture:** ARM64 (Apple Silicon)

---

## Executive Summary

**Critical Issue Identified:** The swictation daemon fails to start due to a **missing dynamic library path configuration**. The ONNX Runtime library (`libonnxruntime.dylib`) exists in the npm package but is not in the daemon's search path.

**Status:** ❌ Daemon NOT running
**Root Cause:** Missing RPATH configuration in daemon binary
**Severity:** High - Prevents daemon from starting

---

## Detailed Findings

### 1. Daemon Process Status
**Result:** No running daemon process found
```bash
pgrep -fl swictation
# Returns: (empty)
```

### 2. Daemon Crash Analysis

**Latest Crash:** 2025-11-24 20:03:38 PST

**Panic Message:**
```
thread 'main' (9973441) panicked at /Users/eddie/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ort-2.0.0-rc.10/src/lib.rs:111:33:
An error occurred while attempting to load the ONNX Runtime binary at `libonnxruntime.dylib`:
dlopen(libonnxruntime.dylib, 0x0005): tried:
  'libonnxruntime.dylib' (no such file),
  '/System/Volumes/Preboot/Cryptexes/OSlibonnxruntime.dylib' (no such file),
  '/usr/lib/libonnxruntime.dylib' (no such file, not in dyld cache),
  'libonnxruntime.dylib' (no such file)
```

**Crash Reports Found:**
- `/Users/eddie/Library/Logs/DiagnosticReports/swictation-daemon-macos-2025-11-24-200338.ips`
- `/Users/eddie/Library/Logs/DiagnosticReports/swictation-daemon-macos-2025-11-24-154014.ips`

### 3. Library Location Verification

**ONNX Runtime Library EXISTS:**
```bash
Location: ~/.npm-global/lib/node_modules/swictation/lib/native/libonnxruntime.dylib
Size:     32MB
Type:     Mach-O 64-bit dynamically linked shared library arm64
Status:   ✅ Valid library file
```

**Additional Libraries Found:**
- `libonnxruntime.so` (Linux version)
- `libonnxruntime_providers_shared.so` (Linux version)

### 4. Binary Analysis

**Daemon Binary:**
```bash
Location: ~/.npm-global/lib/node_modules/swictation/bin/swictation-daemon-macos
Version:  0.7.1
Size:     7.2MB (7,539,840 bytes)
```

**Dynamic Library Dependencies:**
```
System Libraries:
  ✅ CoreGraphics.framework
  ✅ ApplicationServices.framework
  ✅ Foundation.framework
  ✅ Metal.framework
  ✅ AudioUnit.framework
  ✅ CoreAudio.framework
  ✅ CoreML.framework
  ✅ AppKit.framework
  ❌ libonnxruntime.dylib (missing from RPATH)
```

**RPATH Configuration:**
```bash
otool -l swictation-daemon-macos | grep -A 3 RPATH
# Returns: (empty) - NO RPATH SET
```

### 5. Workaround Verification

**Setting DYLD_LIBRARY_PATH works:**
```bash
DYLD_LIBRARY_PATH=~/.npm-global/lib/node_modules/swictation/lib/native:$DYLD_LIBRARY_PATH \
./swictation-daemon-macos --version-info

# Output:
swictation-daemon 0.7.1
Build Information:
  ONNX Runtime: 1.23.2
  Target:       aarch64-apple-darwin
  Profile:      release
  Build Date:   2025-11-24 22:47:01 UTC
  Features:     sway-integration
```

### 6. System Environment

**Audio System:**
- ✅ MacBook Pro Microphone detected
- ✅ MacBook Pro Speakers configured
- ✅ Sample Rate: 48000 Hz

**CoreML Framework:**
- ✅ Present at `/System/Library/Frameworks/CoreML.framework/CoreML`
- ✅ Apple Silicon GPU detection working

**Daemon Configuration:**
- ✅ Config file loaded from `~/Library/Application Support/com.swictation.daemon/config.toml`
- ✅ GPU detection: CoreML (Apple Silicon)

### 7. Socket Files

**Found in /tmp:**
```bash
/tmp/swictation_flushed_audio.wav         (20 KB)
/tmp/swictation-daemon-macos-arm64.tar.gz (3.1 MB)
/tmp/swictation-daemon-package/           (directory with daemon binary)
```

**No active socket file** (would be created if daemon runs successfully)

---

## Root Cause Analysis

### Problem
The daemon binary (`swictation-daemon-macos`) is built **without RPATH** pointing to the bundled ONNX Runtime library. When the binary starts:

1. It attempts to load `libonnxruntime.dylib`
2. macOS searches standard system paths (not the npm package path)
3. Library not found → panic → crash

### Why It Happens
The Rust build process (likely using `cargo build --release`) creates the binary but doesn't embed the correct `@rpath` or `@loader_path` reference for the bundled dylib.

### Evidence
- **Binary has no RPATH:** `otool -l` shows no LC_RPATH load commands
- **Library exists:** The dylib is present in `lib/native/`
- **Workaround works:** Setting `DYLD_LIBRARY_PATH` allows daemon to start

---

## Recommended Solutions

### Immediate Fix (For Users)
Create a wrapper script that sets the library path:

```bash
#!/bin/bash
# ~/.npm-global/lib/node_modules/swictation/bin/swictation-daemon-wrapper

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
LIB_DIR="$(cd "$SCRIPT_DIR/../lib/native" && pwd)"

export DYLD_LIBRARY_PATH="$LIB_DIR:$DYLD_LIBRARY_PATH"
exec "$SCRIPT_DIR/swictation-daemon-macos" "$@"
```

### Long-term Fix (Build System)
Modify the Rust build process to embed RPATH:

**Option 1: Cargo Config**
```toml
# .cargo/config.toml
[target.aarch64-apple-darwin]
rustflags = [
    "-C", "link-arg=-Wl,-rpath,@loader_path/../lib/native"
]
```

**Option 2: Build Script**
```rust
// build.rs
fn main() {
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path/../lib/native");
    }
}
```

**Option 3: Post-Build Patching**
```bash
# After build
install_name_tool -add_rpath @loader_path/../lib/native \
    target/release/swictation-daemon-macos
```

### NPM Package Fix
Update the npm package's binary wrapper to include library path:

```javascript
// In npm-package/bin/swictation-daemon
const { spawn } = require('child_process');
const path = require('path');

const libDir = path.join(__dirname, '../lib/native');
const env = {
  ...process.env,
  DYLD_LIBRARY_PATH: `${libDir}:${process.env.DYLD_LIBRARY_PATH || ''}`
};

const daemon = spawn(
  path.join(__dirname, 'swictation-daemon-macos'),
  process.argv.slice(2),
  { env, stdio: 'inherit' }
);
```

---

## Testing Recommendations

1. **Verify RPATH after build:**
   ```bash
   otool -l target/release/swictation-daemon-macos | grep -A 3 RPATH
   ```

2. **Test library loading:**
   ```bash
   otool -L target/release/swictation-daemon-macos
   ```

3. **Validate dylib resolution:**
   ```bash
   DYLD_PRINT_LIBRARIES=1 ./swictation-daemon-macos --version
   ```

4. **End-to-end test:**
   ```bash
   npm pack && npm install -g swictation-*.tgz
   swictation-daemon --version-info
   ```

---

## Impact Assessment

**Current State:**
- ❌ Daemon cannot start on macOS
- ❌ No transcription functionality
- ❌ Users must manually set DYLD_LIBRARY_PATH
- ❌ LaunchD service will fail to start

**After Fix:**
- ✅ Daemon starts automatically
- ✅ No environment variables needed
- ✅ Standard npm installation works
- ✅ LaunchD service operates correctly

---

## Additional Notes

### npm Installation Path
```
Global: ~/.npm-global/lib/node_modules/swictation/
Binary: ~/.npm-global/lib/node_modules/swictation/bin/swictation-daemon-macos
Native: ~/.npm-global/lib/node_modules/swictation/lib/native/libonnxruntime.dylib
```

### Build Information
```
ONNX Runtime: 1.23.2
Target:       aarch64-apple-darwin
Build Date:   2025-11-24 22:47:01 UTC
Features:     sway-integration
```

### Model Compatibility
- Parakeet-TDT-0.6B-V3 (ONNX)
- Parakeet-TDT-1.1B-V3 (ONNX)

---

## Next Steps

1. **Immediate:** Implement wrapper script in npm package
2. **Short-term:** Add RPATH to build configuration
3. **Testing:** Validate on clean macOS installation
4. **Documentation:** Update installation instructions with troubleshooting
5. **CI/CD:** Add library path verification to build pipeline

---

## Related Files

**Source:**
- `rust-crates/swictation-daemon/Cargo.toml`
- `rust-crates/swictation-daemon/build.rs` (may not exist)
- `npm-package/bin/swictation-daemon`

**Testing:**
- Crash reports in `~/Library/Logs/DiagnosticReports/`
- Daemon logs in `~/Library/Logs/swictation-daemon.log`
- Config in `~/Library/Application Support/com.swictation.daemon/`

---

**Report Generated:** 2025-11-25T16:23:00-08:00
**Diagnostics Collected By:** Research Agent
**Priority:** High - Core Functionality Blocked
