# ADR-027: npm Install Readiness for v0.7.28 (First macOS Release)

**Status:** Accepted (Implemented 2026-03-23)
**Date:** 2026-03-23
**Authors:** Robert (Product Owner), Winston (Architect), Quinn (QA), John (PM), Amelia (Dev), Mary (Analyst)
**Deciders:** Robert
**Scope:** npm install flow for `swictation@0.7.28` across Linux x64 and macOS ARM64

---

## Context

Swictation v0.7.28 is the **first macOS (Apple Silicon) release**. The Linux npm install flow has been tested and confirmed working in previous releases. This ADR documents findings from a comprehensive code review of the entire npm install pipeline and defines the action plan to ensure a flawless `npm install -g swictation --foreground-scripts` experience on both platforms.

### Architecture Overview

```
npm install -g swictation
    |
    |- swictation (main package: CLI, postinstall, config)
    |- @agidreams/linux-x64 (optionalDep: ELF binaries + libonnxruntime.so)
    |- @agidreams/darwin-arm64 (optionalDep: Mach-O binaries + libonnxruntime.dylib)
    |
    postinstall.js executes:
      1. Platform detection & validation
      2. Stop existing services
      3. Clean old installations
      4. Config migration
      5. GPU detection & library download (CUDA on Linux, CoreML on macOS)
      6. Model download (VAD + STT via HuggingFace)
      7. Service generation (systemd on Linux, launchd on macOS)
      8. Auto-start daemon + UI
      9. Verification & next steps
```

### Release Pipeline

```
GitHub Actions:
  build-linux.yml (ubuntu runner)  -->  @agidreams/linux-x64 artifact
  build-macos.yml (macos-14 M1)   -->  @agidreams/darwin-arm64 artifact
                                          (includes signed libonnxruntime.dylib)
  release.yml:
    1. Build both platforms
    2. Verify version sync
    3. Publish platform packages to npm
    4. Wait for registry propagation
    5. Publish main swictation package
```

---

## Decision

Fix all identified issues before publishing v0.7.28. Changes are grouped by priority.

---

## Findings and Required Changes

### P0 — Must Fix Before Release

#### P0-1: Version Mismatch — Main Package Ahead of Platform Packages

**File:** `npm-package/package.json:2`

**Problem:** Main package version is `0.7.28`, but `optionalDependencies` correctly pins platform packages at `0.7.28`. The release version should be `0.7.28` universally. The main package.json was bumped prematurely.

**Decision:** Downgrade main `package.json` version from `0.7.28` to `0.7.28` to match the platform packages. All three packages must publish at the same version.

**Change in `npm-package/package.json`:**
```json
"version": "0.7.28"
```

Platform packages (`darwin-arm64/package.json` and `linux-x64/package.json`) are already at `0.7.28` — no change needed there.

---

#### P0-2: Root README .npmrc Overwrite

**File:** `README.md:41`

**Problem:** The install instructions use `>` which overwrites the entire `.npmrc` file:
```bash
echo "prefix=$HOME/.npm-global" > ~/.npmrc
```
Any existing npm configuration (registry auth tokens, proxy settings, scoped registries) is destroyed.

**Decision:** Remove this guidance entirely. The install instruction should be a single command:
```bash
npm install -g swictation --foreground-scripts
```

Users who need a custom prefix can figure that out themselves. The `npm-package/README.md` should also reflect this simplified approach.

**Change:** Replace the entire "One-time npm setup" block in the root `README.md` Install section with:
```bash
npm install -g swictation --foreground-scripts
```

---

#### P0-3: macOS Accessibility Instructions Point to Non-Existent Binary

**File:** `npm-package/postinstall.js:3051-3073`

**Problem:** The postinstall prints instructions telling macOS users to grant Accessibility permission to:
```
~/.npm-global/lib/node_modules/swictation/bin/swictation-daemon-macos
```
This file does not exist. The actual daemon binary is at:
```
<npm-root>/node_modules/@agidreams/darwin-arm64/bin/swictation-daemon
```

Additionally, when the daemon auto-starts via launchd and attempts to use Accessibility APIs, macOS will prompt the user automatically with the correct binary path.

**Decision:** Replace the verbose manual Accessibility instructions with a brief note:
```
macOS will prompt you to grant Accessibility permission when the daemon
first attempts to inject text. Click "Open System Settings" and enable
the permission for swictation-daemon.

If text injection doesn't work, check:
  System Settings > Privacy & Security > Accessibility
```

Remove the incorrect binary path entirely.

---

#### P0-4: Model Download Requires hf CLI (May Not Be Installed)

**File:** `npm-package/lib/model-downloader.js`

**Problem:** The `ModelDownloader` class requires the `hf` (HuggingFace CLI) to download models from HuggingFace repos. This CLI is listed as an optional dependency and may not be installed. If missing, model download fails silently and the daemon won't work.

On macOS, the 1.1B CoreML model (`jenerallee78/parakeet-tdt-1.1b-coreml`) has a predictable structure:
- 3 `.mlmodelc` bundles (encoder, decoder, joiner) + `tokens.txt`
- Each bundle contains exactly 5 files in a flat structure
- Total: 16 files, ~1.95 GB (encoder weights are 1.93 GB)
- Files are served via HuggingFace CDN at `https://huggingface.co/<repo>/resolve/main/<path>`

**Decision:** Implement a two-tier download strategy:
1. **Primary:** Try `hf` CLI (if available) — handles all repos, supports resume
2. **Auto-install fallback:** If `hf` not found, attempt `pip install huggingface_hub[cli]` (macOS: also try `brew install huggingface-cli`)
3. **Direct HTTP fallback:** If CLI install fails, download files directly via `curl` or Node.js `https` module using known file manifest per model

The direct HTTP fallback for the 1.1B CoreML model downloads these files:
```
encoder.mlmodelc/weights/weight.bin          (1.93 GB, LFS)
encoder.mlmodelc/metadata.json               (3 KB)
encoder.mlmodelc/model.mil                   (1.5 MB)
encoder.mlmodelc/coremldata.bin              (425 B, LFS)
encoder.mlmodelc/analytics/coremldata.bin    (243 B, LFS)
decoder.mlmodelc/weights/weight.bin          (~14 MB, LFS)
decoder.mlmodelc/metadata.json
decoder.mlmodelc/model.mil
decoder.mlmodelc/coremldata.bin
decoder.mlmodelc/analytics/coremldata.bin
joiner.mlmodelc/weights/weight.bin           (~3.3 MB, LFS)
joiner.mlmodelc/metadata.json
joiner.mlmodelc/model.mil
joiner.mlmodelc/coremldata.bin
joiner.mlmodelc/analytics/coremldata.bin
tokens.txt                                   (10 KB)
```

Base URL pattern: `https://huggingface.co/jenerallee78/parakeet-tdt-1.1b-coreml/resolve/main/{path}`

Use the existing `downloadWithRetry()` from `src/download.js` for retry/resume/progress on the large `weight.bin` files.

---

### P1 — Should Fix Before Release

#### P1-1: npm-package/README.md is Linux-Only

**File:** `npm-package/README.md`

**Problem:** The README shown on npmjs.com is entirely Linux-focused. System requirements say "OS: Linux x64", references `systemctl`, `ydotool`, `libasound2`, GNOME/Sway. No macOS content at all. A macOS user visiting the npm page gets no relevant information.

**Decision:** Make `npm-package/README.md` minimal and cross-platform. It should:
- State that Swictation supports **Linux x64** and **macOS Apple Silicon (M1-M5)**
- Show the single install command: `npm install -g swictation --foreground-scripts`
- Link to the GitHub repo README for full documentation
- Remove all platform-specific deep-dive content

---

#### P1-2: Contradictory macOS Service Instructions

**File:** `npm-package/postinstall.js:3078-3083`

**Problem:** The "Verifying installation" phase on macOS prints manual `launchctl load` and `launchctl start` instructions, but `generateLaunchdServices()` (called earlier at line 3042) already loaded and started both services via `launchctl bootstrap` + `launchctl start`.

**Decision:** Remove the manual launchctl instructions from the "Verifying installation" phase for macOS. Replace with a verification check:
```javascript
// Verify daemon is loaded
try {
    execSync('launchctl print gui/$(id -u)/com.swictation.daemon', { stdio: 'ignore' });
    log('green', 'Daemon service: loaded and auto-start enabled');
} catch {
    log('yellow', 'Daemon service: not loaded (will start on next login)');
}
```

---

#### P1-3: Phase Title Mismatch

**File:** `npm-package/postinstall.js:3045`

**Problem:** Phase is titled "Downloading speech models..." but the code inside does:
- Linux: `setupWaylandIntegration()` (not model download)
- macOS: Prints Accessibility instructions (not model download)

Actual model download happens inside `showNextSteps()` → `autoDownloadModel()` at line 3095.

**Decision:** Rename phase to "Platform integration..." and add a separate phase for model download:
```javascript
phaseLog('Platform integration...');
// ... existing platform-specific code ...

phaseLog('Downloading speech models...');
// Move autoDownloadModel() call here from showNextSteps()
```

---

### P2 — Should Fix (Non-Blocking)

#### P2-1: Duplicate verifyChecksum Function

**File:** `npm-package/postinstall.js:694` and `npm-package/postinstall.js:784`

**Problem:** Two functions named `verifyChecksum` exist:
1. Line 694: Sync, SHA-256, takes `(filePath, expectedChecksum)` — **never called**
2. Line 784: Async, SHA-512, takes `(filePath, filename, checksums)` — **actively used**

The second declaration silently shadows the first. This is dead code and a maintenance hazard.

**Decision:** Remove the unused sync version at line 694. It was superseded by the async checksums-map-based version.

---

#### P2-2: showNextSteps() Still Shows "Run swictation setup" and "swictation start"

**File:** `npm-package/postinstall.js:2833-2844`

**Problem:** After a successful model download, `showNextSteps()` tells users to run `swictation setup` and `swictation start`. But setup and start already happened during postinstall. These instructions are misleading on both platforms.

**Decision:** Replace with a "Ready to use" message:
```
Swictation is installed and running!

  - Press Ctrl+Shift+D (macOS) or Super+Shift+D (Linux) to toggle recording
  - Run 'swictation status' to check service health
  - Run 'swictation --version' to see component versions
```

---

### P3 — Nice to Have

#### P3-1: macOS SIP Note for DYLD_LIBRARY_PATH

The daemon launcher wrapper script (`bin/swictation-daemon-launcher`) correctly handles SIP stripping `DYLD_*` variables from launchd processes by re-setting them at runtime. This is well-implemented and documented in comments. No change needed — noting for awareness.

#### P3-2: Platform Package Version Metadata Stale

**Files:** `npm-package/packages/darwin-arm64/package.json:41-42`, `npm-package/packages/linux-x64/package.json:41-42`

The `metadata.distribution` field says `0.7.9` and `metadata.daemon` says `0.7.5`. These are stale — the CI `build.sh` script updates them at build time via `jq`, so this is cosmetic in the source repo only. No action needed.

---

## Implementation Order

```
Phase 1 (P0 — before any publish):
  1. P0-1: Sync versions to 0.7.28 in all package.json files
  2. P0-2: Simplify root README install instructions
  3. P0-3: Fix macOS Accessibility instructions in postinstall.js
  4. P0-4: Implement direct HTTP model download fallback

Phase 2 (P1 — before release tag):
  5. P1-1: Rewrite npm-package/README.md for cross-platform
  6. P1-2: Remove contradictory manual launchctl instructions
  7. P1-3: Fix phase title and move model download to correct phase

Phase 3 (P2 — cleanup):
  8. P2-1: Remove duplicate verifyChecksum
  9. P2-2: Fix showNextSteps() messaging
```

---

## Verification Plan

### Pre-Publish Checklist

- [ ] All three `package.json` files show version `0.7.28`
- [ ] `optionalDependencies` in main package.json references `0.7.28`
- [ ] `npm run version:verify` passes
- [ ] Root README has simplified single-command install
- [ ] npm-package/README.md mentions both Linux and macOS
- [ ] No reference to `swictation-daemon-macos` in any file
- [ ] No reference to `.npmrc` overwrite in any README
- [ ] `verifyChecksum` only defined once in postinstall.js
- [ ] Model download works without `hf` CLI installed (direct HTTP fallback)

### Post-Publish Verification (per platform)

#### macOS (Apple Silicon)
- [ ] `npm install -g swictation --foreground-scripts` completes without errors
- [ ] `@agidreams/darwin-arm64` installed with binaries in `bin/` and dylib in `lib/`
- [ ] CoreML ONNX Runtime dylib present (from platform package or downloaded)
- [ ] `com.swictation.daemon.plist` generated in `~/Library/LaunchAgents/`
- [ ] `com.swictation.ui.plist` generated in `~/Library/LaunchAgents/`
- [ ] Daemon auto-started (`launchctl list | grep com.swictation.daemon`)
- [ ] macOS Accessibility permission dialog appears
- [ ] After granting permission, text injection works
- [ ] After reboot, daemon and UI auto-start
- [ ] 1.1B CoreML model downloaded to `~/.local/share/swictation/models/parakeet-tdt-1.1b-coreml/`
- [ ] Model files complete: 3x `.mlmodelc` bundles + `tokens.txt`

#### Linux (x64, Ubuntu 24.04+)
- [ ] `npm install -g swictation --foreground-scripts` completes without errors
- [ ] `@agidreams/linux-x64` installed with binaries
- [ ] GPU libraries downloaded (if NVIDIA GPU present)
- [ ] systemd service generated and enabled
- [ ] Daemon auto-started
- [ ] Recommended model downloaded
- [ ] After reboot, daemon auto-starts via systemd

---

## Files Modified

| File | Change |
|------|--------|
| `npm-package/package.json` | Downgrade version from 0.7.29 to 0.7.28 to match platform packages |
| `README.md` | Remove .npmrc overwrite, simplify install to single command |
| `npm-package/README.md` | Rewrite as minimal cross-platform, link to GitHub |
| `npm-package/postinstall.js` | Fix Accessibility instructions, remove duplicate verifyChecksum, fix phase titles, fix showNextSteps, remove contradictory launchctl instructions |
| `npm-package/lib/model-downloader.js` | Add direct HTTP download fallback for CoreML models, add hf CLI auto-install |

---

## Consequences

### Positive
- macOS users get a working one-command install experience
- No external dependency on `hf` CLI for model downloads
- No risk of destroying user's `.npmrc` configuration
- Correct Accessibility instructions (or none, relying on OS prompt)
- Services auto-start and persist across reboots on both platforms
- Version consistency across all packages prevents silent install failures

### Negative
- Direct HTTP model download fallback adds ~100 lines of code to model-downloader.js
- Must maintain file manifest for each model (16 files for 1.1B CoreML)

### Risks
- GitHub Actions macOS runners (macos-14) may have availability constraints during release
- Apple Developer ID certificate secrets must be configured in GitHub repo for code signing
- npm registry propagation delay (handled by existing 5-minute polling in release.yml)
- HuggingFace CDN availability for direct model downloads (mitigated by retry/resume in download.js)

---

## References

- HuggingFace 1.1B CoreML model: `jenerallee78/parakeet-tdt-1.1b-coreml`
- Platform package architecture: similar to esbuild, swc
- macOS launchd documentation: `man launchd.plist`
- macOS SIP and DYLD_* stripping: Technical Note TN2206
- Release workflow: `.github/workflows/release.yml`
- Build workflows: `.github/workflows/build-macos.yml`, `.github/workflows/build-linux.yml`
