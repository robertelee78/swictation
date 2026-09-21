# Native distribution: local validation, 2026-09-21

This records the local qualification snapshot before GitHub publication, not release
approval or complete host validation. At that point the implementation was uncommitted
in `/opt/swictation`; no GitHub release, npm registry change, or migration of the
operator's live installation had been performed. Subsequent GitHub qualification is
recorded by the candidate and publication workflow receipts.

## Source and artifact identity

- Base commit: `120c276d19bdc1ed75a90c275a9528b695b1b9c6`.
- Changed-file manifest SHA-256:
  `8f2b640b99b132a1ce6ea6f5fdefffd76af91a5f75080a672872305399bb9708`.
- Exact paths, contents, modes, deletions and submodule revision are recorded in
  `/opt/swictation-native-distribution-build/source-receipt.json`. This report is
  excluded from that manifest to avoid self-reference. Base commit plus manifest
  describes the local changes; the base commit alone does not identify this build.
- Host: macOS 27, Apple Silicon; Rust 1.89.0.
- Product CLI candidate: `0.8.0`; existing daemon/UI components: `0.7.35`.
- Final local archive:
  `/opt/swictation-native-distribution-build/archive-signed-v2/swictation-0.8.0-aarch64-apple-darwin.tar.gz`.
- Archive size: `36942206` bytes; SHA-256:
  `c1b49458ba386c4650dfdef0faca5946be616d039c5607b90c423bdc8baaec48`.
- Apple notarization accepted submission:
  `d46f5936-e456-41c3-a1bd-0e956d9cf6f5`.

The archive's four-field release manifest records the base commit. This is a local
development candidate from the changed tree, **not an artifact built from that clean
commit**. Publication must rebuild from an authorized clean commit and repeat checks.

## Checks performed

| Check | Result |
| --- | --- |
| Rust workspace: `cargo test --locked --workspace` | 199 passed, 6 pre-existing ignored, 0 failed before the final trust syntax fix |
| Final native CLI: `cargo test --locked -p swictation-cli` in integrated checkout | 25 passed, 0 ignored, 0 failed; includes the added real Apple requirement-parser test |
| Native CLI clippy with `-D warnings` and rustfmt check | Passed in integrated checkout |
| Distribution Python tests in integrated checkout | 20 passed |
| Shellcheck, actionlint, whitespace validation | Passed |
| CLI and CoreML daemon release builds | Passed |
| Tauri frontend and macOS app release build | Passed |
| ONNX Runtime download | Official 1.22.0 arm64 archive matched pinned size and SHA-256 before extraction |
| Developer ID signing and notarization | Passed using a temporary keychain; both apps stapled and validated |
| Final signed archive install → installed version → uninstall | Passed in isolated HOME with full Apple trust checks, no unsigned bypass |

The installed lifecycle smoke test guards `launchctl` and `systemctl` and fails if
either is invoked. It proves binary-only installation and removal do not configure
or alter services. Native fixture tests exercise update/rollback, lock exclusion,
wrong hashes, downgrades, foreign paths, damaged-version repair and data preservation.

Logs are retained under `/opt/swictation-native-distribution-build/`, including
`workspace-tests.log`, `integrated-cli-tests.log`, `signing-v2.log`, and
`signed-install-smoke.log`. Private key material is not stored in the repository or
release archive. No GitHub signing environment was changed during this local check;
the subsequent authorized publication work provisions Swictation's own environment.

## Failures found and corrected

- The existing UI release profile stripped proc-macro libraries into Mach-O data
  rejected by macOS 27. An isolated build reproduced it; disabling stripping only
  for build helpers fixed the real UI build without changing locked dependencies.
- The first signed install rejected an inline Apple requirement because its `=`
  prefix was missing. The fixed command passes Apple's parser, and the rebuilt,
  re-signed, re-notarized archive passes the complete native installation test.
- The existing local keychain identity returned `errSecInternalComponent`.
  Importing the matching publisher certificate backup into an ephemeral keychain
  allowed signing without changing the existing keychain's access settings.

## Not yet verified

- Linux compilation/runtime, GPU variants and desktop integration on a Linux host.
- macOS 14 compatibility on an actual macOS 14 host.
- Real microphone capture, TCC consent, inference, hotkeys and text insertion.
- Service migration/update/rollback under a real desktop session; those paths have
  code review and fixture coverage, not complete host acceptance evidence.
- Public GitHub bootstrap downloads and updates between two published native versions.
- Execution of the new GitHub workflows and provisioning of Swictation's own protected
  signing environment. The workflow uses the publisher certificate and independently
  configurable notarization credentials; it does not retrieve hf2q credentials.

The product npm source and publishing machinery were removed. Ignored dependencies
and agent caches formerly inside `npm-package/` were moved intact to
`/opt/swictation-native-distribution-build/retired-npm-local-artifacts` for recovery.
Node/npm remains a build dependency for the Tauri frontend only. Installation and
setup are separate; the new native installer prints `swictation setup` as the next step.
