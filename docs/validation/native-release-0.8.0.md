# Native release 0.8.0 evidence

Published 2026-09-21: [Swictation v0.8.0](https://github.com/robertelee78/swictation/releases/tag/v0.8.0).
The immutable release source is `ee12044cadee53691e4140b735c51386aa195c1b`.
Later documentation changes on main do not replace that source or its artifacts.

## Automated evidence

- [Candidate preflight](https://github.com/robertelee78/swictation/actions/runs/35608061683):
  Linux/macOS native lifecycle contracts, Python packaging/signing contracts, full
  Rust workspace tests, native CLI/daemon and Tauri builds, Linux packaged lifecycle
  checks, and macOS Developer ID signing, accepted notarization and signed packaged
  lifecycle checks passed. The existing workspace has six ignored non-lifecycle
  tests; the native lifecycle contract suites execute without ignored tests.
- [Publication](https://github.com/robertelee78/swictation/actions/runs/35609406812):
  resolved the successful exact-source candidate, checked live job evidence, verified
  artifact and asset hashes, published without rebuilding, then downloaded and compared
  every public asset. The release has no npm runtime or product npm publication step.
- [Public installation and update](https://github.com/robertelee78/swictation/actions/runs/35609806567):
  Ubuntu 24.04 and macOS 14 fetched the anonymous public installer into isolated homes,
  installed 0.8.0, listed setup steps, checked updates, forced a same-version update,
  and uninstalled. Existing test configuration and model data survived unchanged;
  guarded service commands were never called. A separate local macOS run of the same
  public smoke script passed as well.
- The downloaded GitHub candidate ZIP was also independently verified locally against
  its GitHub SHA-256, `aea5ed9f3ea1ffc331f069eacbe772794baba9cecb762d3da5c6d975ac3f9142`.
  Every contained release asset matched the candidate receipt's size and SHA-256.
- The final Linux ELF dependency inventory was inspected: the CLI needs standard
  system runtime libraries, the daemon also needs ALSA, and the UI needs GTK/WebKitGTK
  and their transitive libraries. [Ubuntu runtime prerequisites](../installation.md#linux-system-libraries)
  are documented separately from application installation.

Download the [candidate receipt](https://github.com/robertelee78/swictation/releases/download/v0.8.0/candidate-receipt.json)
and [publication receipt](https://github.com/robertelee78/swictation/releases/download/v0.8.0/publication-receipt.json)
for source identities, job references, sizes and hashes.

| Archive | Bytes | SHA-256 |
| --- | ---: | --- |
| Linux x86_64 | 269229948 | `a30a5e83be45ddd568581adf2561a8f415baa7f79a068d7f4a0171ecb7d1994b` |
| macOS Apple Silicon | 36800375 | `9c82f3aef408bca9ef790e9668d573642f1cfb35445e1d9817a7c7094a880172` |

## Remaining host acceptance

Microphone capture, desktop permission prompts, GPU/CoreML inference, interactive
UI/tray behavior, text insertion and reboot/login behavior were not proved by these
CI installation checks. Use the [Linux checklist](../linux-native-install-test.md)
or [macOS checklist](../macos-testing-checklist.md) on the actual desktop.

A real upgrade and rollback between published native versions requires a subsequent
release; 0.8.0 cannot roll back to the retired npm package. Filesystem transition and
failure-path tests exercise update/rollback separately from real hardware readiness.

The operator's live npm installation was left untouched during release validation.
