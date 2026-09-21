# macOS native-release host checklist

Updated: 2026-09-21. This checklist records required work; no item is a claim of a
completed proof. The first native release and clean-host qualification are pending.
Pair this with the [release checklist](RELEASE_CHECKLIST.md) and
[ADR-038](adr/ADR-038-native-distribution-lifecycle.md).

## Evidence identity

Record the source commit, exact tag, archive/installer digests, macOS version, hardware,
unified memory, user profile, commands, exit codes and sanitized logs. Use the final
signed/notarized archive. A fixture installer or source binary is not interchangeable.

## Installation and migration

- [ ] Prove the macOS 14+ Apple Silicon target and 16 GiB CoreML setup floor.
- [ ] Verify Intel/unsupported hosts fail with accurate diagnostics.
- [ ] On a clean user account without Node/npm/Rust, run the public installer from
  [the setup guide](macos-setup.md) and verify the installed version.
- [ ] Verify Developer ID, Team ID, hardened runtime, timestamp and notarization
  against the exact installed executable, app and nested components.
- [ ] Verify the launcher selects the whole `<data>/install/current` release.
- [ ] Installation leaves setup explicit; it does not claim models or permissions ready.
- [ ] On a separate existing npm installation, stop old services, remove the package
  with `npm uninstall -g --ignore-scripts swictation` before native setup, and prove
  user config, model, correction and metrics bytes survive.
- [ ] Run setup, verify existing valid TOML remains unchanged and models are verified.
- [ ] Prove service definitions use native stable paths and no Node/npm dependencies.
- [ ] Inspect doctor, deep content verification, JSON output and repair failures.

## Permissions and speech

- [ ] Grant Microphone and Accessibility permission to the actual installed components.
- [ ] Verify denied permissions yield a truthful error and recover after granting them.
- [ ] `Ctrl+Shift+D` starts/stops; `Ctrl+Space` push-to-talk matches configured behavior.
- [ ] Dictate into TextEdit, Notes, a terminal and a browser text field.
- [ ] Verify VAD pause handling, final stop-drain, punctuation and absence of duplicate text.
- [ ] Verify native CoreML model loading and actual inference; retain concrete errors.
- [ ] Exercise long dictation, rapid start/stop and sustained operation.
- [ ] Measure latency/memory with the exact model and settings; avoid guessed thresholds.
- [ ] Verify user-defined hotkeys and preserved preferences after restart.
- [ ] Verify the Tauri UI, tray, metrics, correction learning and settings persistence.

## Service lifecycle

- [ ] `start --ui`, `stop`, `status` and a subsequent start match real service state.
- [ ] Inspect daemon/UI logs and LaunchAgents for correct paths and library environment.
- [ ] Logout/login and reboot preserve the documented service behavior.
- [ ] No duplicate old npm daemon/UI remains after migration.
- [ ] Default logs preserve transcript privacy under ADR-034.

## Update, rollback and removal

- [ ] Between two real native versions, check then update and verify all components.
- [ ] Verify service restoration and accurately reported restoration failures.
- [ ] Repeat a dictation after update, including any renewed TCC permission prompts.
- [ ] Roll back once; verify the previous release and unchanged user data.
- [ ] Corrupt/truncated archive, wrong hash/target, unsafe entries, foreign launcher,
  interrupted transfer, full disk and concurrent mutation cannot silently replace
  the working release. Run destructive fixtures only in an isolated test profile.
- [ ] Preview then confirm uninstall; prove owned services and releases are removed.
- [ ] Verify config, models, GPU libraries, corrections and metrics survive default removal.
- [ ] Test each explicit purge independently against its printed owned paths.
- [ ] Confirm macOS's shared config/data parent never causes unrelated data removal.

## Result

Record pass/fail/blocked per item, exact supporting evidence, unresolved issues and
what was not tested. Release authorization is a separate decision; a checked box
without artifact and host evidence is insufficient.
