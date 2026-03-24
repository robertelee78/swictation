# ADR-032: Postinstall Script Logic Audit and Hardening

**Status:** Accepted
**Date:** 2026-03-24
**Authors:** Robert (Product Owner), Claude (Audit Lead)
**Deciders:** Robert
**Release:** v0.7.33 (bundled with ADR-031)
**Scope:** Comprehensive logic audit of both Linux and macOS postinstall scripts (`npm-package/postinstall.js` and `npm-package/package/postinstall.js`)

---

## Engineering Principles

> **DO NOT BE LAZY.** We have plenty of time to do it right. No shortcuts. Never make assumptions. Always dive deep and ensure you know the problem you're solving. Make use of search as needed. Measure 3x, cut once. No fallback. No stub (todo later) code. Just pure excellence, done the right way the entire time.
>
> **Chesterton's Fence:** Always understand the current implementation fully before changing it. If something looks wrong but has survived this long, investigate *why* it exists before removing or modifying it.

> **If you're unsure about anything, ask.** Don't guess. Don't assume. Ask the product owner before proceeding.

Every fix in this ADR must be implemented with these principles. No speculative changes. No "fix it later" stubs. Read the code, understand the problem, verify the fix, test the result.

---

## Context

Swictation is distributed via `npm install -g swictation` with a postinstall script that handles platform detection, binary permissions, GPU library downloads, ONNX Runtime setup, service generation (systemd on Linux, launchd on macOS), model downloads, and service activation.

Two postinstall scripts exist in the repository:

1. **Current script:** `npm-package/postinstall.js` (3320 lines, supports Linux + macOS)
2. **Legacy script:** `npm-package/package/postinstall.js` (2042 lines, Linux-only, version 0.6.1)

The current script evolved from the legacy script by adding macOS support. The legacy script remains as a complete, independently-publishable npm package with a wired postinstall hook in its own `package.json`.

A comprehensive logic audit was conducted on 2026-03-24 using three parallel review agents:

1. **Linux Reviewer** -- Audited the legacy Linux-only script for logic defects
2. **macOS Reviewer** -- Audited macOS-specific code paths in the current script
3. **Cross-Platform Reviewer** -- Audited shared logic, `main()` flow, and divergence between both scripts

### Audit Results Summary

| Severity | Found | Approved to Fix | Leave As-Is | Auto-Resolved (Legacy Deletion) |
|----------|-------|-----------------|-------------|-------------------------------|
| CRITICAL | 6     | 4               | 0           | 2                             |
| HIGH     | 8     | 6               | 1           | 1                             |
| MEDIUM   | 11    | 8               | 1           | 2                             |
| LOW      | 8     | 8               | 0           | 0                             |
| **Total**| **33**| **26**          | **2**       | **5**                         |

---

## Decision

We will implement 26 fixes across the postinstall script, delete the legacy `npm-package/package/` directory entirely, and leave 2 items as-is with documented rationale. 5 items are auto-resolved by the legacy deletion.

---

## Detailed Findings and Dispositions

### CRITICAL

#### C1. 16GB Hard Gate Rounding Bug on macOS
- **File:** `postinstall.js:2168-2174`
- **Problem:** `process.exit(1)` kills the npm install if `Math.round(totalMemoryMB / 1024) < 16`. The `Math.round()` causes 16GB Macs that report ~15.3-15.8GB (due to firmware memory reservation) to round down to 15 and fail the check, bricking installation on legitimate 16GB machines.
- **Decision:** Keep the 16GB gate (8GB Macs are genuinely unsupported), but compare raw MB against 14500 instead of using `Math.round` on GB.
- **Fix:**
  ```javascript
  // Before (buggy):
  gpuInfo.totalMemoryGB = Math.round(gpuInfo.totalMemoryMB / 1024);
  if (gpuInfo.totalMemoryGB < 16) { process.exit(1); }

  // After (fixed):
  gpuInfo.totalMemoryGB = Math.round(gpuInfo.totalMemoryMB / 1024);
  if (gpuInfo.totalMemoryMB < 14500) { process.exit(1); }
  ```

#### C2. `stopExistingServices()` Has No macOS/launchd Fallback
- **File:** `postinstall.js:170-201`
- **Problem:** On macOS upgrade, if the `swictation` CLI isn't in PATH, the function falls through to `systemctl --user stop` which doesn't exist on macOS. The running daemon is never stopped before files are replaced, risking corrupt state or stale dylib handles.
- **Decision:** Add a `process.platform === 'darwin'` branch using `launchctl bootout`, `pkill` fallback, and 2s wait (matching current Linux behavior).
- **Fix:** Add macOS branch:
  ```javascript
  if (process.platform === 'darwin') {
    try {
      execSync('launchctl bootout gui/$(id -u) com.swictation.daemon 2>/dev/null || true', { shell: '/bin/bash' });
      execSync('launchctl bootout gui/$(id -u) com.swictation.ui 2>/dev/null || true', { shell: '/bin/bash' });
      log('green', 'Stopped services via launchctl');
      stopped = true;
    } catch (e) { /* ignore */ }
    // Safety net for manually launched daemons
    try {
      execSync('pkill -f swictation-daemon 2>/dev/null || true', { stdio: 'ignore' });
    } catch (e) { /* ignore */ }
    if (stopped) await new Promise(resolve => setTimeout(resolve, 2000));
  }
  ```

#### C3. `cleanOldServices()` Runs Linux Systemd Cleanup Unconditionally on macOS
- **File:** `postinstall.js:285-355`
- **Problem:** Only checks systemd service file paths. On macOS, runs `systemctl` commands that fail silently. Never cleans stale LaunchAgent plists from prior macOS installations.
- **Decision:** Add macOS branch that globs `com.swictation.*.plist` in `~/Library/LaunchAgents/`, unloads and removes them. Guard all systemd code behind `process.platform === 'linux'`.
- **Fix:** Add macOS plist cleanup:
  ```javascript
  if (process.platform === 'darwin') {
    const agentsDir = path.join(os.homedir(), 'Library', 'LaunchAgents');
    if (fs.existsSync(agentsDir)) {
      const plists = fs.readdirSync(agentsDir).filter(f => f.startsWith('com.swictation.') && f.endsWith('.plist'));
      for (const plist of plists) {
        const plistPath = path.join(agentsDir, plist);
        const label = plist.replace('.plist', '');
        try {
          execSync(`launchctl bootout gui/$(id -u) ${label} 2>/dev/null || true`, { shell: '/bin/bash' });
        } catch (e) { /* ignore */ }
        try {
          fs.unlinkSync(plistPath);
          log('green', `  Removed old plist: ${plist}`);
          foundOldServices = true;
        } catch (e) { /* ignore */ }
      }
    }
  }
  ```

#### C4. Command Injection Vectors via Shell String Interpolation
- **File:** `postinstall.js` (various `execSync` calls with string interpolation)
- **Problem:** `execSync(`sudo rm -rf "${path}"`)` and `installPackage()` passing unsanitized `packageName` to `sudo apt install -y ${packageName}`. While inputs are currently hardcoded, the pattern is dangerous.
- **Decision:** Delete the legacy `package/` directory entirely (which has the worst instances). In the current script, harden ALL `execSync` calls with shell interpolation — use `fs.rmSync()` where possible and `execFileSync` with argument arrays for commands requiring `sudo`. Full hardening, not just security-sensitive paths. Inputs are hardcoded today, but clean patterns signal we take security seriously.
- **Fix:** Replace shell-interpolated `rm` calls:
  ```javascript
  // Before:
  execSync(`sudo rm -rf "${oldPath}" 2>/dev/null || rm -rf "${oldPath}"`, { stdio: 'ignore' });

  // After:
  try {
    fs.rmSync(oldPath, { recursive: true, force: true });
  } catch (err) {
    // May need elevated permissions for system paths
    try {
      execFileSync('sudo', ['rm', '-rf', oldPath], { stdio: 'ignore' });
    } catch (sudoErr) {
      log('yellow', `Could not remove ${oldPath}: ${sudoErr.message}`);
    }
  }
  ```

#### C5. Legacy Script Still Has a Wired Postinstall Hook (Auto-Resolved)
- **File:** `npm-package/package/package.json:54`
- **Problem:** The legacy `package/` directory is a complete npm package (version 0.6.1) with `"postinstall": "node postinstall.js"`. If accidentally built/published, runs old script with no checksums, single-redirect downloads, and no macOS support.
- **Decision:** Delete the entire `npm-package/package/` directory. It's all in git history if needed.
- **Safety verification:** Confirmed no CI workflows, build scripts, or runtime code references `npm-package/package/` (only `npm-package/packages/` plural). Only historical docs/ADRs mention it. The `.npmignore` entry for `npm-package/package/config/detected-environment.json` will be cleaned up.
- **Note:** ADR-030 items S6 and S11 reference `npm-package/package/preuninstall.js` — these are superseded by this deletion.

#### C6. `finalLdLibraryPath` Uninitialized in Legacy Script (Auto-Resolved)
- **File:** `package/postinstall.js:859, 913`
- **Problem:** `let finalLdLibraryPath;` with no default; `.trim()` call can throw TypeError.
- **Decision:** Auto-resolved by deleting the legacy script (C5).

---

### HIGH

#### H1. Legacy `downloadFile()` -- No Status Code Check, Single Redirect, No Checksums (Auto-Resolved)
- **File:** `package/postinstall.js:565-593`
- **Problem:** 404/500 responses piped to disk as valid files. Only one redirect followed. No checksum verification on GPU library downloads. Supply-chain security gap.
- **Decision:** Auto-resolved by deleting the legacy script (C5).

#### H2. `selectGPUPackageVariant()` Has SM Version Gaps (sm_71-74, sm_87-88)
- **File:** `postinstall.js:601-636`
- **Problem:** Compute capabilities sm_71-74 (Jetson Xavier) and sm_87-88 (Jetson Orin) fall through to "unsupported" variant. These are real NVIDIA devices.
- **Decision:** **Leave as-is.** Jetson embedded devices are out of scope for Swictation. The gaps only affect NVIDIA Jetson SoCs which are not part of the target user base.

#### H3. Phase Counter Shows `[9/8]`
- **File:** `postinstall.js:75`
- **Problem:** `_totalPhases = 8` but 9 `phaseLog()` calls exist in `main()` (one conditional). The comment "Adjusted dynamically" is false -- no dynamic adjustment exists.
- **Decision:** Set `_totalPhases = 9`. The conditional "Downloading speech models" phase may cause a phase number skip when not triggered, but `[5/9]` jumping to `[7/9]` is less jarring than `[9/8]`.
- **Fix:**
  ```javascript
  let _totalPhases = 9;
  ```

#### H4. `gpuInfo` and `ortLibPath` Uninitialized -- Crash Risk
- **File:** `postinstall.js:3138-3139`
- **Problem:** `let gpuInfo;` and `let ortLibPath;` have no defaults. If `checkPlatform()` is ever changed to warn-and-continue (as it already does for incompatible GLIBC), these hit TypeError at line 3174.
- **Decision:** Add safe defaults and an explicit else-throw guard after the platform branches.
- **Fix:**
  ```javascript
  let gpuInfo = { hasGPU: false, recommendedModel: 'cpu-only' };
  let ortLibPath = null;

  if (process.platform === 'linux') {
    // ... existing Linux code ...
  } else if (process.platform === 'darwin') {
    // ... existing macOS code ...
  } else {
    throw new Error(`Unsupported platform: ${process.platform} (should have been caught by checkPlatform)`);
  }
  ```

#### H5. GLIBC Regex May Match Wrong Number on Non-glibc Systems
- **File:** `postinstall.js:100-101`
- **Problem:** `/(\d+)\.(\d+)/` on `ldd --version` output could match a year ("2024") on musl-based systems like Alpine Linux, giving a false positive on the version check.
- **Decision:** Tighten regex to `/GLIBC\s+(\d+)\.(\d+)/i` to anchor to the actual GLIBC identifier.
- **Fix:**
  ```javascript
  const versionMatch = glibcVersion.match(/GLIBC\s+(\d+)\.(\d+)/i);
  ```

#### H6. SIP Makes `DYLD_LIBRARY_PATH` in Plist Dead Code
- **File:** `postinstall.js:1848-1852` and plist template
- **Problem:** macOS SIP (System Integrity Protection) strips `DYLD_*` environment variables from launchd processes. The wrapper script was created as the workaround (line 1759 comment), but the plist template still sets `DYLD_LIBRARY_PATH` -- this is dead code that causes debugging confusion.
- **Decision:** Remove `DYLD_LIBRARY_PATH` from the plist template. The wrapper script is the sole reliable mechanism.
- **Fix:** Remove `{{DYLD_LIBRARY_PATH}}` from `templates/macos/com.swictation.daemon.plist` and remove the replacement at line 1852.

#### H7. `codesign -dv` Is Display-Only, Not Verification
- **File:** `postinstall.js:1021-1035`
- **Problem:** `codesign -dv` only confirms a code signature exists (including ad-hoc signatures). It does not verify signature validity or chain of trust. An attacker replacing the dylib with an ad-hoc signed malicious binary would pass this check.
- **Decision:** Upgrade to `codesign --verify --deep --strict`. Tested on the actual installed ONNX Runtime dylib (v1.22.0, Team ID `3T2D2YNTVW`) — passes strict verification. Team ID pinning deferred to a future signing overhaul.
- **Fix:**
  ```javascript
  function verifyDylibSignature(dylibToCheck) {
    try {
      // --verify: actually validate the signature (not just display)
      // --deep: verify nested code
      // --strict: enforce strict validation rules
      execSync(`codesign --verify --deep --strict "${dylibToCheck}" 2>&1`, { encoding: 'utf8' });
      // If verification passes, get the Team ID for metadata
      const sigInfo = execSync(`codesign -dv "${dylibToCheck}" 2>&1`, { encoding: 'utf8' });
      let teamId = null;
      const teamMatch = sigInfo.match(/TeamIdentifier=(\S+)/);
      if (teamMatch && teamMatch[1] !== 'not set') {
        teamId = teamMatch[1];
      }
      return { valid: true, teamId };
    } catch (err) {
      return { valid: false, teamId: null };
    }
  }
  ```

#### H8. Variable Shadowing -- `daemonPlistPath` Declared Twice
- **File:** `postinstall.js:1662, 1860`
- **Problem:** Outer `const daemonPlistPath` at line 1662, inner `const daemonPlistPath` at line 1860. Same path, different JavaScript variables. A third variable `daemonPlistFinal` at line 1914 points to the same path. Confusing and error-prone during refactoring.
- **Decision:** Remove inner redeclarations. Use the outer `daemonPlistPath` and `uiPlistPath` throughout the function.

---

### MEDIUM

#### M1. `checkDependencies()` Lists Linux Tools on macOS
- **File:** `postinstall.js:499-541`
- **Problem:** The tools list includes `systemctl`, `wtype`, and `xdotool` with no platform guard. macOS users see misleading "missing optional dependency" warnings for tools that don't apply to their platform.
- **Decision:** Skip `checkDependencies()` entirely on macOS. macOS has no optional tool dependencies worth checking at install time.
- **Fix:**
  ```javascript
  function checkDependencies() {
    if (process.platform === 'darwin') return; // No optional deps to check on macOS
    // ... existing Linux code ...
  }
  ```

#### M2. `interactiveConfigMigration()` Shows Dead UI Menu
- **File:** `postinstall.js:2014-2037`
- **Problem:** Displays K/N/M/D/S options but always defaults to "Keep existing config" even when `process.stdin.isTTY` is true. The interactive prompt was never implemented. Lines 2031-2037 explicitly say "Interactive mode not available during postinstall."
- **Decision:** Remove the dead interactive UI entirely. On upgrade, overwrite config with the new template and back up the existing config to `config.toml.old`. No interactive prompts — they are unreliable in npm postinstall hooks across different package managers and CI environments.
- **Fix:**
  ```javascript
  async function interactiveConfigMigration() {
    const configDir = getConfigDir();
    const configPath = path.join(configDir, 'config.toml');
    const newConfigTemplate = path.join(__dirname, 'config', 'config.toml');

    if (!fs.existsSync(newConfigTemplate)) return;

    if (fs.existsSync(configPath)) {
      // Back up existing config before overwriting
      const backupPath = path.join(configDir, 'config.toml.old');
      try {
        fs.copyFileSync(configPath, backupPath);
        log('cyan', `  Backed up existing config to ${backupPath}`);
      } catch (err) {
        log('yellow', `  Could not back up config: ${err.message}`);
      }
    }

    try {
      fs.mkdirSync(configDir, { recursive: true });
      fs.copyFileSync(newConfigTemplate, configPath);
      log('green', `  Config written to ${configPath}`);
    } catch (err) {
      log('yellow', `  Could not write config: ${err.message}`);
    }
  }
  ```

#### M3. VRAM Thresholds Inconsistent Between Detection Functions
- **File:** `postinstall.js` -- `detectGPUVRAM()` vs `recommendOptimalModel()`
- **Problem:** `detectGPUVRAM()` uses 6000MB for 1.1B and 3500MB for 0.6B (based on empirical testing). `recommendOptimalModel()` uses 4000MB for 1.1B. A 5GB VRAM GPU gets different recommendations depending on which function is consulted.
- **Decision:** `detectGPUVRAM()` thresholds are authoritative (empirically calibrated). Align `recommendOptimalModel()` to match.
- **Fix:** Change `recommendOptimalModel()`:
  ```javascript
  if (capabilities.gpuMemoryMB >= 6000) {
    // 1.1B model
  } else if (capabilities.gpuMemoryMB >= 3500) {
    // 0.6B model
  }
  ```

#### M4. `cleanupOldOnnxRuntime()` Only Detects Version "1.20"
- **File:** `postinstall.js:228`
- **Problem:** `ortFiles[0].includes('1.20')` only matches version 1.20.x. Other conflicting old versions (1.18, 1.19, etc.) are not cleaned up. Also only checks the first file alphabetically.
- **Decision:** Parse the version from the filename and remove anything older than 1.22 (the minimum supported version). Check all matching files, not just `[0]`.
- **Fix:**
  ```javascript
  const ortFiles = fs.readdirSync(capiDir).filter(f => f.includes('libonnxruntime'));
  const versionMatch = ortFiles.find(f => {
    const m = f.match(/(\d+)\.(\d+)/);
    if (m) {
      const ver = parseInt(m[1]) * 100 + parseInt(m[2]);
      return ver < 122; // anything older than 1.22
    }
    return false;
  });
  if (versionMatch) {
    // Remove the conflicting installation
  }
  ```

#### M5. `cleanupOldOnnxRuntime()` Checks `.so` Extension on macOS
- **File:** `postinstall.js:228`
- **Problem:** Filters for `libonnxruntime.so` which is Linux-only. On macOS the extension is `.dylib`. Additionally, macOS pip paths (`~/Library/Python/3.XX/`) are not checked.
- **Decision:** Check `.dylib` on macOS and add macOS pip library paths.
- **Fix:** Use platform-aware extension and paths:
  ```javascript
  const isMacOS = process.platform === 'darwin';
  const ortExtension = isMacOS ? 'libonnxruntime' : 'libonnxruntime.so';
  const pythonLibDirs = isMacOS
    ? [/* ~/Library/Python/3.XX/lib/python/site-packages/onnxruntime paths */]
    : [/* existing ~/.local/lib paths */];
  ```

#### M6. SM Version Calculation Breaks for Double-Digit Minor Versions
- **File:** `postinstall.js:583-584`
- **Problem:** `major * 10 + minor` gives 130 for "12.10" instead of 1210. NVIDIA has never used double-digit minors, but this is a cheap defensive fix.
- **Decision:** Use string concatenation for future-proofing.
- **Fix:**
  ```javascript
  result.smVersion = parseInt(`${major}${minor}`);
  ```

#### M7. `cleanupOldNpmInstallations()` Misses Homebrew ARM64 Path
- **File:** `postinstall.js:257-261`
- **Problem:** Missing `/opt/homebrew/lib/node_modules/swictation` from the cleanup list. This is the default Homebrew path on Apple Silicon Macs.
- **Decision:** Add the Homebrew ARM64 path.
- **Fix:**
  ```javascript
  const oldInstallPaths = [
    '/usr/local/lib/node_modules/swictation',
    '/usr/local/nodejs/lib/node_modules/swictation',
    '/usr/lib/node_modules/swictation',
    '/opt/homebrew/lib/node_modules/swictation', // Homebrew ARM64 (macOS Apple Silicon)
  ];
  ```

#### M8. Temp Directories Not Cleaned on Download Failure
- **File:** `postinstall.js` -- `downloadGPULibraries()` and `downloadONNXRuntimeCoreML()`
- **Problem:** If download or extraction fails after creating the temp directory, the temp files (potentially 1.5GB GPU tarball) are left on disk. Catch blocks do not clean up.
- **Decision:** Use `fs.mkdtempSync()` for unique temp directories and clean up in `finally` blocks.
- **Fix:**
  ```javascript
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'swictation-gpu-'));
  try {
    // ... download and extract ...
  } catch (err) {
    // ... error handling ...
  } finally {
    try { fs.rmSync(tmpDir, { recursive: true, force: true }); } catch { /* ignore */ }
  }
  ```

#### M9. TOCTOU Races in Security-Sensitive Paths
- **File:** `postinstall.js` -- checksum verification and codesign paths
- **Problem:** `existsSync()` + operation pattern used throughout. Most are low-risk in a single-user postinstall context, but checksum verification and codesign checks are security-sensitive.
- **Decision:** Fix TOCTOU in security-critical paths only (checksum verify and codesign check). Accept the risk for non-security paths.
- **Implementation Notes:**
  - For checksum verification: Operate on the file directly in a try/catch rather than checking existence first
  - For codesign: Verify the final target file, not an intermediate copy

#### M10. `nvidia-smi` Called 6+ Times Redundantly
- **File:** `postinstall.js` -- multiple `detectNvidiaGPU()` callers
- **Problem:** `nvidia-smi` spawned 6+ times during a single postinstall run. Each invocation is a separate process with GPU driver initialization.
- **Decision:** **Leave as-is.** `nvidia-smi` is fast enough on modern systems, and the caching would add complexity for minimal benefit.

#### M11. Legacy Model Download Happens After Model Test (Auto-Resolved)
- **File:** `package/postinstall.js` -- legacy-only
- **Problem:** Tests a model that isn't downloaded yet. Fixed in current script but not backported.
- **Decision:** Auto-resolved by deleting the legacy script (C5).

---

### LOW

#### L1. `detectActualNpmInstallPath()` Is Trivial Wrapper
- **File:** `postinstall.js:647-654`
- **Problem:** Function does nothing but return `__dirname`. Obscures intent and adds indirection.
- **Decision:** Inline `__dirname` directly. Remove the function and `detectNpmNativeLibPath()` which depends on it.

#### L2. `checkDependencies()` Required-Deps Branch Is Unreachable
- **File:** `postinstall.js:526-533`
- **Problem:** All tools in the `tools` array are `type: 'optional'`. The `required` array is always empty, making `process.exit(1)` at line 532 dead code.
- **Decision:** Remove the dead `required` handling code. If required dependencies are added in the future, re-add the handling.

#### L3. `waylandResults` and `serviceResults` Captured But Never Used
- **File:** `postinstall.js:3223, 3238`
- **Problem:** Return values assigned to variables but never referenced.
- **Decision:** Remove the variable assignments (keep the function calls).

#### L4. Install Log Uses Linux XDG Path on macOS
- **File:** `postinstall.js:35`
- **Problem:** Install log goes to `~/.local/share/swictation/install.log` on macOS. macOS convention is `~/Library/Logs/`. Service logs already go to `~/Library/Logs/swictation/`.
- **Decision:** Use `~/Library/Logs/swictation/install.log` on macOS for consistency with service logs.

#### L5. Data/Cache Dirs Use XDG Paths on macOS
- **File:** `postinstall.js:416-418`
- **Problem:** `~/.local/share/swictation` and `~/.cache/swictation` used on macOS. Config dir correctly uses `~/Library/Application Support/swictation/` via `getConfigDir()`, creating a split personality.
- **Decision:** Use macOS-native paths: `~/Library/Application Support/swictation/` for data, `~/Library/Caches/swictation/` for cache.
- **Note:** This requires updating all code that references these paths to use platform-aware helper functions (similar to `getConfigDir()`). Model download paths, GPU info paths, and service file template variables will need updates.

#### L6. `minor` Captured But Unused in macOS Version Check
- **File:** `postinstall.js:137`
- **Problem:** `const minor = parseInt(versionMatch[2])` assigned but never used. Only `major` is compared against 14.
- **Decision:** Remove the unused `minor` variable.

#### L7. `Math.round` for VRAM GB Inflates Display Values
- **File:** `postinstall.js` (and legacy)
- **Problem:** `Math.round(gpuInfo.vramMB / 1024)` means 3584MB displays as "4GB", potentially misleading users about model compatibility.
- **Decision:** Use `Math.floor` for VRAM GB display to avoid inflation.

#### L8. Legacy `package.json` Version Is 0.6.1 vs Current 0.7.33 (Auto-Resolved)
- **File:** `package/package.json:3`
- **Problem:** Wildly out-of-date version confirms the legacy package is abandoned.
- **Decision:** Auto-resolved by deleting the legacy script (C5).

---

## Implementation Plan

### Phase 1: Legacy Deletion (C4, C5 -- resolves C6, H1, M11, L8)
1. Delete `npm-package/package/` directory entirely
2. Verify no other files reference the legacy package directory
3. Update `.gitignore` and `.npmignore` if they reference `package/` paths

### Phase 2: Critical macOS Fixes (C1, C2, C3)
1. Fix 16GB memory gate rounding (compare raw MB against 14500)
2. Add launchctl + pkill + wait to `stopExistingServices()`
3. Add macOS plist glob cleanup to `cleanOldServices()`
4. Guard all systemd code behind `process.platform === 'linux'`

### Phase 3: High-Priority Fixes (H3-H8)
1. Set `_totalPhases = 9`
2. Initialize `gpuInfo` and `ortLibPath` with defaults + else-throw guard
3. Tighten GLIBC regex to `/GLIBC\s+(\d+)\.(\d+)/i`
4. Remove `DYLD_LIBRARY_PATH` from plist template
5. Upgrade codesign to `--verify --deep --strict`
6. Remove variable shadowing in `generateLaunchdServices()`

### Phase 4: Medium and Low Fixes (M1-M9, L1-L7)
1. Skip `checkDependencies()` on macOS
2. Replace dead interactive config migration with overwrite + `.old` backup
3. Align VRAM thresholds in `recommendOptimalModel()`
4. Broaden ORT cleanup to remove any version < 1.22
5. Fix `.dylib` extension and macOS pip paths in ORT cleanup
6. Fix SM version calculation for future-proofing
7. Add Homebrew ARM64 path to cleanup list
8. Use `mkdtempSync` + `finally` for temp directory cleanup
9. Fix TOCTOU in checksum and codesign paths
10. Inline trivial wrappers, remove dead code, fix macOS paths
11. Use `Math.floor` for VRAM display
12. Command injection hardening (`fs.rmSync` / `execFileSync`)

### Phase 5: Verification
1. Test Linux fresh install
2. Test Linux upgrade (with running services)
3. Test macOS fresh install on 16GB Mac
4. Test macOS upgrade from v0.7.32 (with running launchd services, stale `~/.config/swictation/` from ADR-031)
5. Test macOS install attempt on 8GB Mac (should fail gracefully with clear message)
6. Test non-interactive install (CI environment, no TTY)
7. Verify phase counter displays correctly in all scenarios
8. Verify `config.toml.old` backup created on upgrade
9. Verify `codesign --verify --deep --strict` passes on downloaded ORT dylib
10. Verify L5 model directory migration (move from XDG to Application Support)

---

## Items Intentionally Left As-Is

| ID | Issue | Rationale |
|----|-------|-----------|
| H2 | SM version gaps (sm_71-74, sm_87-88) | Jetson embedded devices are out of scope. No desktop/laptop/server GPUs use these compute capabilities. |
| M10 | `nvidia-smi` called 6+ times | Fast enough on modern systems; caching adds complexity for minimal benefit. |

---

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Deleting legacy `package/` may break unknown consumers | Verified: no CI/build scripts reference it. Only historical docs. All in git history. |
| L5 (macOS native paths) requires updating many path references | Implement `getDataDir()` and `getCacheDir()` helper functions, similar to existing `getConfigDir()`. Also requires coordinating with Rust `swictation_paths` crate for model directory paths. |
| `codesign --verify --strict` may reject ad-hoc signed dylibs | Tested on actual installed ORT dylib — passes strict verification with Team ID `3T2D2YNTVW`. Not ad-hoc. |
| Config overwrite on upgrade loses user customizations | Existing config backed up to `config.toml.old` before overwriting. Users can diff and restore manually. |
| L5 model directory migration (~2GB) on macOS | Move (not copy) model files from `~/.local/share/swictation/models/` to `~/Library/Application Support/swictation/models/`. Fall back to re-download if move fails. |

---

## References

- ADR-028: Postinstall CoreML sequencing fix
- ADR-029: Upgrade crash loop, launcher paths, chunking dedup
- ADR-030: Security audit findings and hardening plan
- ADR-031: Model override mismatch and config path fix
