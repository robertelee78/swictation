# Native release checklist

Updated: 2026-09-21. [ADR-038](adr/ADR-038-native-distribution-lifecycle.md) governs
distribution. This is a checklist of required evidence, not a record of completed
validation. The first native release is published; its actual results and remaining
hardware/desktop checks are recorded in [0.8.0 release evidence](validation/native-release-0.8.0.md).

## Source and build inputs

- [ ] Choose an authorized release commit and increasing stable version; bind the
  tag, Cargo versions, bundle manifests, and generated installer to that exact source.
- [ ] Record the toolchain, dependency locks, target and build inputs. Do not ship
  artifacts from a dirty or unidentified source tree.
- [ ] Build the Rust CLI, daemon and Tauri UI for both supported targets:
  `x86_64-unknown-linux-gnu` and `aarch64-apple-darwin`.
- [ ] Build Linux against the Ubuntu 24.04 compatibility floor and macOS against
  the macOS 14 deployment floor. Test each archive on its actual supported host.
- [ ] Run focused lifecycle/setup tests and applicable daemon/UI regressions.
- [ ] Use npm only for development/build dependencies in `tauri-ui/`. Product npm
  packages, registry publication, lifecycle hooks and JavaScript launchers are retired.

## Exact release artifacts

- [ ] Each native archive contains CLI, daemon, UI, required ONNX libraries,
  `release.json`, pinned model metadata and nonsecret setup assets. Models and
  downloaded GPU libraries remain outside the release bundle.
- [ ] Inspect archive paths and types: reject absolute paths, traversal, links and
  unexpected entries. Verify every required component and target identity.
- [ ] Record each final archive's byte length and SHA-256. Generate `install.sh`
  from those exact immutable values, with an exact versioned download URL.
- [ ] Verify macOS Developer ID signatures, hardened runtime, secure timestamp,
  Team ID and bundle identities. Require accepted notarization for the exact artifact.
- [ ] Sign nested components before their containing app; assess the distributed
  app using the appropriate Apple trust checks. Raw CLI and app assessment differ.
- [ ] Keep signing credentials in the protected signing environment. Do not execute
  untrusted candidate code while credentials are present.
- [ ] Use the native release workflow to require both supported builds and their
  validation. Do not bypass a failed target with skip-build or package-manager fallback.

## Packaged lifecycle proof on both hosts

Use an isolated user profile and the final packaged artifact, not source-tree binaries.
Record commands, exit status, artifact digests, host versions and sanitized logs.

- [ ] Fetch the generated installer, install, and run the installed `--version`.
- [ ] Verify PATH guidance, `~/.local/bin/swictation`, receipt ownership and the
  `<data>/install/current` selection.
- [ ] Prove install alone does not configure models or claim speech readiness.
- [ ] Run setup, inspect doctor output, start services, and test actual dictation.
- [ ] Verify service definitions use stable native paths and correct library
  environment; no Node interpreter, npm prefix or source checkout is referenced.
- [ ] Migrate an existing npm installation: stop old services, remove it with
  `npm uninstall -g --ignore-scripts swictation` before native setup, and preserve
  config/model/GPU/correction/metrics data. Explicitly start the new services.
- [ ] Exercise `update --check`, update between two real versions, and rollback.
  Verify the CLI, daemon, UI and libraries switch together.
- [ ] Compare configuration and user-data digests across update and rollback.
- [ ] Verify service restoration and accurately reported restoration failures.
- [ ] Exercise uninstall preview and confirmed removal. Default removal preserves
  user data; each explicit purge affects only its named owned class.
- [ ] Prove wrong hash/size/version/target, unsafe archives, foreign launchers,
  interrupted download, insufficient disk space and concurrent writers preserve
  the working release or report a precisely reconciled committed transition.
- [ ] Test failed receipt/publication operations and stale partial recovery.

## Product proof

- [ ] Linux: prove microphone capture, VAD, STT, stop-drain, Secretary Mode and
  text insertion in the actual X11/Wayland session. Verify CUDA where advertised.
- [ ] macOS: complete the [host checklist](macos-testing-checklist.md), including
  microphone and Accessibility permissions, CoreML inference and text insertion.
- [ ] Verify the Tauri UI and tray on advertised desktops. Prove the native
  wlroots selection uses the bundled Python/Qt tray and reports missing PySide6
  accurately; dictation itself remains independent of the optional tray.
- [ ] Reboot/login and verify the documented service behavior and retained settings.
- [ ] Record what was not tested. Filesystem fixture tests do not prove GPU,
  microphone, TCC, public-download operation, or host compatibility.

## Publication and verification

- [ ] Obtain the independently authorized release decision after reviewing the
  exact source and artifact evidence. A local passing suite does not authorize publish.
- [ ] Upload final assets without overwriting existing published bytes. If retrying,
  compare already-present assets and fail on disagreement.
- [ ] Fresh-download all published assets and compare bytes/digests with the tested
  candidates. Verify the published installer still binds its own exact tag.
- [ ] Exercise the documented public bootstrap on both hosts from the published
  assets and repeat the installed lifecycle checks.
- [ ] Replace pending-publication notices with release availability and the actual
  host-proof scope; retain explicit gaps for untested desktop/hardware behavior.
- [ ] Update release notes, supported-platform limitations and governing ADR status
  to reflect actual evidence, including any remaining gaps.

## Recovery

Operators use `swictation update --rollback` for the retained previous native release.
Publish a new immutable corrective version if necessary; do not rewrite release assets
or delete/repoint a published tag to conceal a failure. Preserve evidence and report
which source and artifact were affected.
