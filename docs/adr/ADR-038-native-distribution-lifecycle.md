# ADR-038: Native installation, update, and removal

- **Status:** Implemented
- **Date:** 2026-09-21
- **Updated:** 2026-09-21
- **Implementation note:** Native lifecycle and packaging replace npm in the published
  0.8.0 release. Both packaged hosts passed CI installation checks; macOS signing and
  notarization passed. [Release evidence](../validation/native-release-0.8.0.md) tracks
  publication proof and the remaining Linux/macOS desktop acceptance checks.
- **2026-09-21 correction:** 0.8.1 aligns automatic daemon verification with CoreML
  startup and adds packaged-daemon selection and missing-model regression checks.
- **Supersedes:** ADR-027, ADR-028 and ADR-032's npm distribution mechanism;
  ADR-037's JavaScript registry and npm lifecycle entry point.
- **Preserves:** ADR-034/035 configuration and upgrade safety, ADR-036 model integrity,
  ADR-037 disk-derived health, explicit repair and honest failure reporting.

## Decision

The operator installs a prebuilt native release, without Node, npm, Rust, or a
source checkout. A Rust `swictation` CLI owns install/update/rollback/uninstall,
setup, models, diagnostics and service management. Node is only a development
dependency of the existing Tauri frontend. Product npm packages, lifecycle hooks,
JavaScript launcher and registry publication are removed.

hf2q ADR-045 provides implemented prior art: immutable release assets, verified
downloads, ownership checks, serialized atomic activation, retained rollback,
and preservation-first removal. Its ADR remains Proposed with portions implemented.
repo-to-cve ADR-059 is Accepted but its distribution implementation remains open;
we use its explicit separation of executable installation from setup as design input.

Supported artifacts are Linux x86_64 (Ubuntu 24.04+) and macOS Apple Silicon
(macOS 14+, 16 GiB+ for CoreML). One archive contains the native CLI, daemon, UI,
ONNX libraries and nonsecret setup assets. Model weights and downloaded GPU
libraries remain in their existing platform data directories, outside releases.

## Ownership and transitions

`~/.local/bin/swictation` points at `<data>/install/current/bin/swictation`.
The current symlink activates an entire immutable release directory. The bundled
`release.json` records schema, version, target and source commit; the locally
written receipt binds it to the verified archive digest. A previous symlink
retains one rollback selection. A filesystem lock serializes all mutations.
Unsafe paths, foreign launchers, malformed receipts, archive links/traversal,
checksum mismatches and incompatible targets fail closed.

Install and update stage and validate the entire archive before stopping existing
services. Activation is a same-directory symlink rename. Service restoration
errors are reported; prior binaries are retained for recovery. No receipt claims
that downloaded artifacts or configured services prove running speech readiness.
Stable service paths always traverse the current release. Update preserves
configuration, models, GPU libraries and learned data, and never runs setup.

`update --check` performs no installation. Normal update accepts stable SemVer
and refuses downgrade; `update --rollback` is explicit. Default uninstall removes
only owned launchers, service integration and release files; user state survives.
Purge flags are explicit and cannot be inferred from uninstall confirmation.
Existing npm-managed launchers are not deleted by guessed directory scans. The
native launcher is installed separately; operators remove the old npm package
with scripts disabled before native setup so its hook cannot tear down new units.

## Setup and evidence

Setup is explicit and rerunnable. Existing valid TOML is preserved. Models retain
the exact pinned revision, size and SHA-256 manifest from ADR-036. Doctor checks
artifacts rather than historical receipts. Repair reports unresolved components
as failure. Services use explicit native paths and platform library environment;
no source checkout, npm package or JavaScript interpreter is referenced.

CI builds each supported target and packages its exact inputs. Release publication
requires both targets and validation, matching version/tag and source SHA, and
macOS signing/notarization; there is no skip-build or npm fallback. Local fixture
tests prove filesystem transitions, daemon model selection and missing-file failures,
not real microphone, GPU, TCC
or published-download operation. Those require the respective host/release proof.

## Acceptance

- Native install -> check -> update -> rollback -> uninstall through CLI.
- Wrong hash, unsafe archive, foreign path and concurrent writer preserve active bytes.
- Config/model data survive install, update, rollback and default uninstall.
- Setup, doctor, repair, service commands and model downloads need no npm runtime.
- Build/release automation contains no product npm packaging or publishing.
- Publication claims bind to the successful candidate and public asset receipts.
