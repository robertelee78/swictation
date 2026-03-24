# ADR-033: Config Path Migration & Dry-Run Detection Fix

- **Status:** Accepted
- **Date:** 2026-03-24
- **Issue:** v0.7.33 regression — model migration moves files but config retains stale paths; dry-run falsely reports success
- **Release:** v0.7.34

## Mantra

> DO NOT BE LAZY. No shortcuts. Never make assumptions. Always dive deep.
> Measure 3x, cut once. Chesterton's fence.

## Context

v0.7.33 (ADR-032, Item L5) introduced macOS-native data directory paths, migrating model files from `~/.local/share/swictation/models/` to `~/Library/Application Support/swictation/models/`. The migration moves the files correctly but does not update the existing `config.toml` which contains absolute paths pointing to the old location. The daemon reads stale paths, finds no models, and crash-loops.

This regression was masked by a second bug: the dry-run success detection in `postinstall.js` matches `"Would load:"` in the daemon output, but the daemon prints this string **before** checking if model files exist on disk. A failing dry-run still contains `"Would load:"`, so the postinstall falsely reports `"1.1b-coreml verified (dry-run passed)"`.

A third issue compounds the problem: `interactiveConfigMigration()` was designed to overwrite the config from a template file, but `config/config.toml` does not exist in the npm package. The function silently returns without modifying the config.

### Execution trace on upgrade from v0.7.32

1. `createDirectories()` (line ~510) — moves model files from XDG to Application Support
2. `interactiveConfigMigration()` (line ~2064) — silently returns (no template file)
3. `testLoadModel()` (line ~2299) — config already exists, skips generation; dry-run daemon reads config with stale paths, fails to find models, but postinstall reports success due to Bug 2
4. `updateConfigWithTestedModel()` (line ~3082) — only modifies `stt_model_override`, not model paths
5. **Result:** Config has correct `stt_model_override = "1.1b-coreml"` but all model paths point to `~/.local/share/swictation/models/` where files no longer exist

### How the dry-run falsely passed

Daemon dry-run output (lines 453-505 of `main.rs`):
```
Override active: 1.1b-coreml
Would load: Parakeet-TDT-1.1B (CoreML, forced)    <-- printed BEFORE file check
  Path: /Users/robert/.local/share/swictation/models/parakeet-tdt-1.1b-coreml
  Model files NOT found at expected path            <-- file check fails
```
Daemon exits non-zero. `execSync` throws. Catch block checks `err.stdout.includes('Would load:')` — **true**. Reports success.

## Decision

Four fixes for v0.7.34.

## Fix Items

### Item 1: Config path rewrite after model migration

**Why:** `createDirectories()` moves models but leaves config pointing to old paths. The daemon reads the config, follows stale paths, crashes.

**What:** After moving model files in the macOS migration block (`createDirectories()`), read the existing `config.toml` and do a blanket string replace of the old data directory prefix with the new one.

**File:** `npm-package/postinstall.js` — `createDirectories()` migration block (~line 544-567)

**Fix:**
```javascript
// After model migration, update config paths to match
const configPath = path.join(getConfigDir(), 'config.toml');
if (fs.existsSync(configPath)) {
  try {
    const oldPrefix = path.join(os.homedir(), '.local', 'share', 'swictation');
    const newPrefix = getDataDir();
    let config = fs.readFileSync(configPath, 'utf8');
    if (config.includes(oldPrefix)) {
      config = config.split(oldPrefix).join(newPrefix);
      fs.writeFileSync(configPath, config);
      log('cyan', '  Updated config paths to match migrated data directory');
    }
  } catch (err) {
    log('yellow', `  Warning: Could not update config paths: ${err.message}`);
  }
}
```

### Item 2: Fix dry-run success detection

**Why:** The postinstall matches `"Would load:"` in daemon output to detect success, but the daemon prints this before checking file existence. A failing dry-run contains the string, causing a false positive.

**What:** On `execSync` success (no throw), check for `"Dry-run complete"` in output. On `execSync` failure (throw/catch), treat it as failure — do not rescue by parsing output. The daemon only prints `"Dry-run complete"` after all verifications pass.

**File:** `npm-package/postinstall.js` — `testLoadModel()` (~line 2380-2410)

**Fix:**
```javascript
try {
  const output = execSync(cmd, { encoding: 'utf8', timeout: 30000 });
  // Exit code 0 — check for the definitive success marker
  if (output.includes('Dry-run complete')) {
    log('green', `    \u2713 ${modelName} verified (dry-run passed)`);
    return { success: true, model: modelName };
  }
  // Exit 0 but no success marker — treat as failure
  log('yellow', `    \u26a0\ufe0f ${modelName} dry-run inconclusive`);
  return { success: false, model: modelName, reason: 'no success marker in output' };
} catch (err) {
  // Non-zero exit — this is a genuine failure, do NOT parse output for partial matches
  const output = (err.stdout || '') + (err.stderr || '');
  log('yellow', `    \u26a0\ufe0f ${modelName} dry-run failed`);
  return { success: false, model: modelName, reason: output.slice(0, 200) };
}
```

### Item 3: Extract shared generateDefaultConfig() function

**Why:** Config generation logic exists inline in `testLoadModel()` (line ~2315) and should also be used by `interactiveConfigMigration()`. Currently `interactiveConfigMigration()` depends on a template file that doesn't exist and never will. Two sources of config generation = two watches.

**What:** Extract a `generateDefaultConfig()` function that returns the config string using `getDataDir()`, `getIpcSocketPath()`, and platform-appropriate hotkeys. Both `testLoadModel()` and `interactiveConfigMigration()` call it. Remove the template file dependency from `interactiveConfigMigration()`.

**File:** `npm-package/postinstall.js`

### Item 4: Daemon error message includes paths tried

**Why:** When the daemon fails with "AI models not found", it doesn't show which paths it tried. Users can't diagnose config/path mismatches.

**What:** Include the actual paths from config in the error message so the mismatch is immediately visible.

**File:** `rust-crates/swictation-daemon/src/main.rs` — the "AI models not found" error block

**Fix:** Add path info to the error message:
```rust
error!("Looked for models at:");
error!("  0.6B: {}", config.stt_0_6b_model_path.display());
error!("  1.1B: {}", config.stt_1_1b_model_path.display());
error!("  CoreML: {}", config.stt_coreml_model_path.display());
error!("  VAD: {}", config.vad_model_path.display());
error!("If paths look wrong, check your config at:");
error!("  {}", config.config_path.display());
```

## Files Changed

| File | Items |
|------|-------|
| `npm-package/postinstall.js` | #1, #2, #3 |
| `rust-crates/swictation-daemon/src/main.rs` | #4 |

## Verification

1. **Upgrade test (macOS):** Install v0.7.33, then upgrade to v0.7.34. Verify:
   - Models migrated to `~/Library/Application Support/swictation/models/`
   - Config paths updated to match
   - Daemon starts successfully
   - `Ctrl+Shift+D` hotkey works
2. **Fresh install test (macOS):** Verify config generated with correct paths from `generateDefaultConfig()`
3. **Dry-run failure test:** Temporarily remove a model file, verify dry-run correctly reports failure (not false positive)
4. **Daemon error test:** Point config to nonexistent path, verify error message shows the path tried
5. **Linux test:** Verify no regression — no migration should occur, paths unchanged

## References

- ADR-031: Model override mismatch and config path fix
- ADR-032: Postinstall logic audit and hardening (introduced L5 migration)
