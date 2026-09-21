# Native installation and lifecycle

Updated: 2026-09-21. Governed by [ADR-038](adr/ADR-038-native-distribution-lifecycle.md).

The native channel starts with the published [0.8.0 release](https://github.com/robertelee78/swictation/releases/tag/v0.8.0).
Its Linux and macOS packages passed installation checks; the macOS package is signed
and notarized. See the [release evidence](validation/native-release-0.8.0.md) for
the tested source, assets and remaining desktop/hardware checks.

## Supported hosts

- Linux x86_64: Ubuntu 24.04 or newer; a user systemd session for managed services.
- macOS Apple Silicon: macOS 14 or newer, with at least 16 GiB for CoreML setup.

Installed use requires no Node.js, npm, Rust compiler, or source checkout. The native
release contains the CLI, daemon, UI, ONNX libraries and nonsecret setup assets.
Models and downloaded GPU libraries are separate user data. Platform audio, text
injection tools, and macOS privacy permissions are still required for dictation.
The optional wlroots tray requires host Python/PySide6; it is not part of speech inference.

### Linux system libraries

On Ubuntu 24.04, install the audio and desktop runtime libraries before setup:

```sh
sudo apt update
sudo apt install libasound2t64 libwebkit2gtk-4.1-0 libgtk-3-0t64 libayatana-appindicator3-1
```

These provide ALSA audio, the Tauri webview, GTK and tray support; apt installs their
transitive dependencies. Other distributions need the equivalent runtime packages.
Development headers and compilers are unnecessary. The release's Linux ELF dependency
inventory confirms the daemon's ALSA and UI's GTK/WebKitGTK requirements; see also
[Tauri's runtime dependency documentation](https://v2.tauri.app/distribute/debian/).

Setup and doctor check audio/text-injection tools. Setup's daemon verification can
expose missing daemon libraries, but doctor does not currently load-test the desktop
UI. Install the libraries above even if the CLI itself runs successfully.

## Fresh installation

Fetch the installer successfully before running it:

```sh
installer=$(mktemp /tmp/swictation-install.XXXXXX) &&
  curl -fsSL https://github.com/robertelee78/swictation/releases/latest/download/install.sh -o "$installer" &&
  sh "$installer"
```

You can inspect the downloaded file before running its final command. The generated
installer pins the exact release version, archive size, and checksum. It validates
its selected archive before activating the release. Unsupported hosts, damaged bytes,
unsafe paths, and conflicting ownership fail without replacing a working release.

The launcher is `~/.local/bin/swictation`. If the installer reports that this directory
is missing from PATH, run:

```sh
export PATH="$HOME/.local/bin:$PATH"
```

The installer prints guidance rather than editing your shell startup files.
The active release lives at `<data>/install/current`; `<data>` is
`~/.local/share/swictation` on Linux and `~/Library/Application Support/swictation`
on macOS. Do not edit or copy files into the managed release directories.

Installation only supplies release files. Configure the host explicitly:

```sh
swictation setup
swictation doctor
swictation start --ui
swictation status
```

On macOS, complete the [privacy permission steps](macos-setup.md) before testing
recording and text insertion. Successful installation, setup, and a running process
are separate checks; prove dictation in a text editor before relying on it.

## Migrate an existing npm installation

Keep your configuration, models, GPU libraries, corrections and metrics in place.
There is no need to download valid model files again merely to change distribution.

1. While the old command still exists, stop its services:

   ```sh
   swictation stop
   ```

2. Remove only the old global product package, with its scripts disabled:

   ```sh
   npm uninstall -g --ignore-scripts swictation
   ```

   Do this **before native setup**. An old cleanup hook must not remove newly generated
   native service files. Do not run `preuninstall.js` manually or delete a source
   checkout. npm is needed only to remove this old package; it is not part of the new
   runtime. If removal needs permissions for the old prefix, resolve that ownership
   separately and keep the data directories intact.

3. Install the native release using the fresh-install command above. Add the printed
   PATH entry if needed, then verify which command the shell will use:

   ```sh
   command -v swictation
   swictation --version
   ```

   The managed command should be `~/.local/bin/swictation`. A conflicting old launcher
   is not permission to overwrite an unrelated file; inspect and resolve it first.

4. Configure native paths and explicitly start the new services:

   ```sh
   swictation setup
   swictation doctor
   swictation start --ui
   swictation status
   ```

   Check existing preferences and test one dictation. Setup preserves valid TOML.
   Updates after this migration use `swictation update`.

## Setup and repair

```sh
swictation setup --list
swictation setup --repair
swictation setup --services
swictation setup --models
swictation setup --gpu-libs
swictation setup --config
swictation doctor --deep
swictation doctor --json
```

`setup --list` and `--help` describe the supported step names. `--repair` checks the
actual artifacts and repairs unhealthy setup components; it reports unresolved work
as failure. `--config` creates missing configuration without resetting existing valid
TOML. Invalid TOML syntax is backed up before defaults are created. Parseable
configuration with invalid field types is preserved and must be corrected explicitly.

Models retain the pinned source revisions, exact lengths and SHA-256 values from
[ADR-036](adr/ADR-036-model-download-integrity-manifest.md). `doctor --deep` checks
model content, which can take time on large models. `doctor` reports disk state;
it does not substitute for microphone, GPU, TCC, or text-injection testing.

Use `download-models --help` for model selection and explicit redownload options.
A release update does not perform setup, reset preferences, or silently replace
analysis of a failing host with a successful installation receipt.

## Update and rollback

```sh
swictation update --check
swictation update
swictation update --version 0.8.0
swictation update --force       # Re-download the current release for binary repair
swictation update --rollback
```

An explicit version must have a published matching native release. Normal updates
accept stable versions and refuse downgrades. Use `--rollback` to restore the retained
previous release. Check, force and rollback are different operations; do not combine them. `--force`
can repair damaged release files by re-downloading the current version; it does not
authorize a downgrade or changes to user data.

The candidate is downloaded, bounded and verified before activation. A lock
serializes mutations. A same-directory symlink replacement switches the entire
release; CLI, daemon, UI and bundled libraries stay together. One previous release
is retained. Existing service state is restored after a successful transition;
service restoration errors are reported and require attention.

Update and rollback preserve configuration, models, downloaded GPU libraries,
corrections and metrics. They do not run setup or roll back user data. If a service
fails after a transition, inspect `swictation status`, `swictation doctor`, and the
platform service logs before retrying. Retain the full error, including whether
activation already completed.

## Uninstall

```sh
swictation uninstall
swictation uninstall --yes
```

Without `--yes`, review the removal preview; it exits nonzero without mutation. Default removal stops/removes owned
service integration, the owned launcher, and installed releases. It preserves user
configuration, models, GPU libraries, corrections and metrics. Source checkouts and
unrelated launchers are outside its ownership.

Optional data removal is separately explicit:

```sh
swictation uninstall --purge-config
swictation uninstall --purge-cache
```

Review the exact affected paths in the preview before adding `--yes` to either command.
`--purge-config` removes only `config.toml`. `--purge-cache` removes the `models/`
and `gpu-libs/` trees. Learned databases and logs remain. A purge flag authorizes its
named class only; confirmation alone never implies purge.
On macOS, configuration and data share a parent directory, so do not replace scoped
purge with recursive deletion of `~/Library/Application Support/swictation`.
