# PRD: Swictation v0.7.29 Release — macOS Native Completion

**Author:** John (PM) with team input
**Date:** 2026-03-23
**Status:** Draft
**Target Version:** 0.7.29

---

## 1. Overview

Swictation v0.7.29 is a milestone release completing the macOS native port. The headline feature — native CoreML STT inference via the `coreml-native` crate — is already implemented. This PRD covers the remaining work to reach release-ready status: a native macOS menu bar status icon, Tauri UI verification, documentation updates, version synchronization, and release process execution.

### 1.1 Background

Since v0.7.1 (the last documented release), 27 versions of incremental macOS work have shipped internally, culminating in:

- Native CoreML STT backend (`recognizer_coreml.rs` + `coreml-native` crate)
- TDT model ported to CoreML format
- Hotkey support via `RunApplicationEventLoop`
- Text injection fixes (duplicate elimination, keystroke delay tuning)
- Default microphone selection on macOS
- Full Apple Silicon readiness (permissions, memory detection, FFI)

### 1.2 Goals

1. Complete macOS feature parity with the Linux tray/status icon experience
2. Verify the Tauri UI builds and runs correctly on macOS
3. Synchronize all version numbers to 0.7.29
4. Update all user-facing documentation to reflect macOS native support
5. Execute the release checklist for both platforms

### 1.3 Non-Goals

- No new features beyond macOS parity (no "nice to haves")
- No Python dependencies on macOS (Python tray app is Linux-only)
- No Intel Mac support (Apple Silicon only)
- No changes to Linux functionality

---

## 2. Feature: macOS Native Menu Bar Status Icon

### 2.1 Problem

On Linux (Sway/Wayland), a Python/PySide6 system tray app provides visual daemon state, toggle controls, and Tauri UI launching. macOS has no equivalent — there is no indicator when the daemon is running, recording, or stopped.

### 2.2 Solution

Implement a native macOS menu bar status icon using **Tauri 2's built-in tray system** (`tauri::tray::TrayIconBuilder`). This replaces the Python tray app entirely for macOS. The Tauri UI already has tray icon assets and the `tray-icon` feature enabled in Cargo.toml.

### 2.3 Requirements — Feature Parity with Linux Tray App

#### 2.3.1 Visual States

| State | Icon Treatment | Description |
|-------|---------------|-------------|
| Idle | Base icon (normal) | Daemon running, not recording |
| Recording | Red overlay on icon | Actively capturing audio |
| Off/Disconnected | Grayed or absent | Daemon not running / socket unreachable |

#### 2.3.2 Click Interactions

| Input | Action |
|-------|--------|
| Left-click | Toggle recording (send `toggle` command via Unix socket) |
| Middle-click | Toggle Tauri metrics UI (open if closed, close if open) |
| Right-click | Show context menu |

Note: macOS menu bar items typically use left-click for menu. The implementation should follow macOS conventions — left-click may show the menu with toggle as the primary action, since middle-click is uncommon on Mac trackpads.

#### 2.3.3 Context Menu

| Menu Item | Action |
|-----------|--------|
| Show Metrics | Launch Tauri UI window |
| Toggle Recording | Send toggle command to daemon |
| --- | Separator |
| Quit | Exit the tray app |

#### 2.3.4 Background Behavior

- Poll daemon state via Unix socket every 1 second
- Send `{"action": "status"}` and parse response for state field
- Auto-update icon on state change
- Show system notifications on recording state transitions ("Recording started" / "Recording stopped")
- Track Tauri UI window lifecycle (open/close state)

### 2.4 Implementation Notes

- Use existing tray icon assets in `tauri-ui/src-tauri/icons/` (tray-icon.png, tray-mono.png, etc.)
- Daemon IPC uses the same Unix socket protocol as Linux (`swictation.sock`)
- On macOS the socket path follows the same logic: `$HOME/.local/share/swictation/swictation.sock` or equivalent
- The tray icon code should be cross-platform in Tauri but conditionally replace the Python tray on macOS
- On Linux, the Python tray app remains the default (no changes to Linux behavior)

### 2.5 Acceptance Criteria

- [ ] Menu bar icon appears when Tauri UI or tray process launches on macOS
- [ ] Icon visually changes between idle, recording, and disconnected states
- [ ] Left-click or menu item toggles recording via socket IPC
- [ ] "Show Metrics" opens the Tauri UI metrics window
- [ ] State polling updates icon within 1-2 seconds of daemon state change
- [ ] Notifications appear on recording start/stop
- [ ] No Python dependencies required on macOS

---

## 3. Tauri UI macOS Verification

### 3.1 Requirements

- [ ] `npm run tauri build` succeeds targeting `aarch64-apple-darwin`
- [ ] App bundle (.app) launches on macOS 14+ (Sonoma)
- [ ] Tray icon renders in macOS menu bar
- [ ] Metrics display connects to daemon socket and shows real-time data
- [ ] All UI tabs function (Dashboard, History, Corrections, Settings)
- [ ] Window management works (minimize, close, reopen from tray)
- [ ] No PySide6/Python dependency required

### 3.2 Known Risks

- Tauri UI has never been tested on macOS (only Linux builds confirmed)
- Socket path differences between platforms may cause connection failures
- macOS sandboxing or notarization requirements may block socket access

---

## 4. Version Synchronization

Bump all version numbers to **0.7.29**:

| File | Current Version | Target |
|------|----------------|--------|
| `npm-package/package.json` | 0.7.28 | 0.7.29 |
| `rust-crates/swictation-daemon/Cargo.toml` | 0.7.5 | 0.7.29 |
| `rust-crates/swictation-stt/Cargo.toml` | 0.2.2 | 0.7.29 |
| `rust-crates/swictation-vad/Cargo.toml` | 0.2.2 | 0.7.29 |
| `rust-crates/swictation-audio/Cargo.toml` | 0.2.2 | 0.7.29 |
| `rust-crates/swictation-metrics/Cargo.toml` | 0.2.2 | 0.7.29 |
| `rust-crates/swictation-broadcaster/Cargo.toml` | 0.2.2 | 0.7.29 |
| `rust-crates/swictation-context-learning/Cargo.toml` | 0.1.0 | 0.7.29 |
| `rust-crates/swictation-paths/Cargo.toml` | 0.1.0 | 0.7.29 |
| `rust-crates/swictation-wasm-utils/Cargo.toml` | 0.1.0 | 0.7.29 |
| `tauri-ui/src-tauri/Cargo.toml` | 0.1.0 | 0.7.29 |
| `tauri-ui/package.json` | 0.1.0 | 0.7.29 |

---

## 5. Documentation Updates

### 5.1 CHANGELOG.md

Create a single consolidated entry for v0.7.29 covering all changes from v0.7.2 through v0.7.29:

**Key sections:**
- Added: Native CoreML STT backend, coreml-native crate integration, macOS menu bar status icon, Tauri tray on macOS
- Changed: Hotkey (macOS: `Ctrl+Shift+D`), model format (CoreML .mlmodelc), inference backend selection
- Fixed: Duplicate text injection, keystroke delay, stride bug in CoreML, default mic selection, hotkey support via RunApplicationEventLoop
- Technical Details: coreml-rs, recognizer_coreml.rs, FP16 conversion pipeline, NeuralNetwork format

### 5.2 README.md

Updates needed:
- [ ] Fix macOS hotkey: `Cmd+Shift+D` -> `Ctrl+Shift+D`
- [ ] Keep Linux hotkey: `Super+Shift+D` (unchanged)
- [ ] Add macOS troubleshooting section (accessibility permissions, CoreML model loading, mic selection)
- [ ] Update architecture section to mention CoreML native backend
- [ ] Document coreml-native crate dependency for macOS
- [ ] Update performance numbers if CoreML native differs from ONNX CoreML EP

### 5.3 docs/architecture.md

Updates needed:
- [ ] Add CoreML native backend as alternative to ONNX Runtime on macOS
- [ ] Document `recognizer_coreml.rs` and its role
- [ ] Add `coreml-native` crate (external: https://github.com/robertelee78/coreml-native) to component diagram
- [ ] Update system overview diagram (currently says "wtype text injection (Wayland)" only)
- [ ] Document macOS text injection via Accessibility API
- [ ] Add macOS menu bar tray architecture

---

## 6. Release Process

Execute `docs/RELEASE_CHECKLIST.md` for both platforms:

### 6.1 Pre-Release Build

- [ ] Build Linux binary: `cargo build --release --target x86_64-unknown-linux-gnu`
- [ ] Build macOS binary: `./npm-package/scripts/build-macos-release.sh`
- [ ] Verify both binaries run with `--help`
- [ ] Build Tauri UI for macOS (.app bundle)

### 6.2 Testing

- [ ] Linux: Install from npm, start daemon, test dictation, verify GPU
- [ ] macOS: Install from npm, start daemon, grant accessibility, test dictation
- [ ] macOS: Verify CoreML model loading and inference
- [ ] macOS: Verify menu bar tray icon states and interactions
- [ ] macOS: Verify Tauri UI launches and displays metrics
- [ ] Linux: Regression check — no breakage from macOS changes

### 6.3 Cleanup

- [ ] Ensure Python tray app is not installed/launched on macOS
- [ ] Verify postinstall.js handles macOS CoreML model download path
- [ ] Verify launchd plist for macOS daemon auto-start

### 6.4 Publish

- [ ] Git tag v0.7.29
- [ ] GitHub release with release notes
- [ ] npm publish
- [ ] Post-release verification on both platforms

---

## 7. Dependency: coreml-native Crate

| Property | Value |
|----------|-------|
| Repository | https://github.com/robertelee78/coreml-native |
| Local Path | /opt/coreml-native |
| Purpose | Native CoreML inference for Apple Silicon (ANE acceleration) |
| Relationship | External crate, used by swictation-stt on macOS |

This crate is maintained in a separate repository. The swictation build on macOS depends on it for CoreML model loading and inference. The README and architecture docs should reference it as an external dependency.

---

## 8. Work Items Summary

| # | Item | Category | Priority | Estimate |
|---|------|----------|----------|----------|
| 1 | macOS native menu bar status icon (Tauri tray) | Feature | P0 | Large |
| 2 | Tauri UI build + test on macOS | Testing | P0 | Medium |
| 3 | Version bump all files to 0.7.29 | Release | P0 | Small |
| 4 | CHANGELOG.md — consolidated v0.7.29 entry | Docs | P0 | Medium |
| 5 | README.md — hotkey fix, macOS troubleshooting, CoreML | Docs | P0 | Medium |
| 6 | Architecture doc — CoreML native backend, coreml-native | Docs | P1 | Medium |
| 7 | Python tray app — exclude from macOS install path | Cleanup | P1 | Small |
| 8 | macOS postinstall.js — CoreML model download verification | Testing | P1 | Small |
| 9 | Linux regression testing | Testing | P1 | Medium |
| 10 | Release checklist execution (both platforms) | Release | P1 | Large |

---

## 9. Success Criteria

v0.7.29 is ready to ship when:

1. A macOS user can `npm install -g swictation`, run `swictation start`, see a menu bar icon, press `Ctrl+Shift+D`, dictate, and see text appear
2. The menu bar icon reflects daemon state (idle/recording/off) within 2 seconds
3. The Tauri UI opens from the menu bar and shows live metrics
4. All documentation accurately reflects the macOS experience
5. Linux functionality is unchanged — no regressions
6. All version numbers read 0.7.29
