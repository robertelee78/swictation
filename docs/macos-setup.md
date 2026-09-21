# macOS setup

## Requirements

- Apple Silicon; Intel Macs are outside the supported artifact matrix.
- macOS 14 or newer.
- At least 16 GiB unified memory for CoreML setup.
- Microphone access and Accessibility permission for recording and text insertion.

No Node.js, npm, Rust compiler, or checkout is required for the installed application.

## Install and configure

```sh
curl -fsSL https://github.com/robertelee78/swictation/releases/latest/download/install.sh | sh
swictation setup
swictation start
```

The installer verifies the exact release archive and creates the stable command at
`~/.local/bin/swictation`. Follow its PATH guidance when needed:

```sh
export PATH="$HOME/.local/bin:$PATH"
swictation --version
swictation setup
swictation doctor
```

The complete native bundle is selected by
`~/Library/Application Support/swictation/install/current`. The daemon is under its
`bin/` directory; bundled libraries and UI belong to that same release. Setup writes
user LaunchAgents with stable native paths and library environment. Existing valid
configuration is preserved. Model weights live outside the release under
`~/Library/Application Support/swictation/models`.

`swictation start` launches the daemon and menu-bar UI. Choose **Show Metrics**
from the Swictation icon to open the dashboard.

## Privacy permissions

Open **System Settings → Privacy & Security** and enable **Microphone** and
**Accessibility** for Swictation. If you need to select the app manually, use:

```text
~/Library/Application Support/swictation/install/SwictationDaemon.app
```

Setup creates this stable path to the signed daemon app. Text insertion retries
automatically after Accessibility access is granted. Speech received before recovery
is discarded; speak again after enabling permission.

Open TextEdit, press `Ctrl+Shift+D`, say “hello world period”, pause, and press
the shortcut again to stop. Confirm the text appears once.

The red Swictation microphone is the app's recording control. The orange microphone
indicator belongs to macOS and appears while an app uses the microphone.

## Configuration

Configuration is at `~/Library/Application Support/swictation/config.toml`.
Missing keys use compiled defaults. Do not replace an existing config to repair a
service or update a binary.

```toml
vad_threshold = 0.25
vad_min_silence = 0.8
stt_model_override = "1.1b-coreml"  # "auto" also selects the macOS model

[hotkeys]
toggle = "Ctrl+Shift+D"
push_to_talk = "Ctrl+Space"
```

Restart the daemon after changing startup settings. Model artifacts are the native
CoreML bundles, not the Linux CUDA/ONNX model layout.

## Diagnose and repair

```sh
swictation doctor
swictation doctor --deep
swictation doctor --json
swictation setup --list
swictation setup --repair
swictation setup --services
swictation setup --models
```

Doctor reads installed artifacts and setup state. Deep verification hashes model
content and can take time. On macOS, the integration check always reports `unknown`
and doctor exits 1 because it does not probe Microphone or Accessibility consent.
This remains true after you grant permissions; repeating repair cannot clear that
check. Resolve any separate unhealthy artifact or service checks, then verify
permissions and actual dictation manually. A healthy receipt or loaded LaunchAgent
is not a speech test.

Inspect service errors with:

```sh
tail -n 50 ~/Library/Logs/swictation/daemon-error.log
tail -n 50 ~/Library/Logs/swictation/daemon.log
launchctl list | grep com.swictation
```

For missing/corrupt model files, run `setup --models`. For incorrect service paths,
run `setup --services`. For missing release binaries or libraries, reinstall using
the verified native installer; a model repair does not replace the release bundle.

If no text appears, verify the service, microphone input, Accessibility permission
and shortcut conflicts. If CoreML fails, inspect the concrete daemon error and
`doctor --deep`; do not infer readiness from Activity Monitor alone.

## Update and rollback

```sh
swictation update --check
swictation update
swictation update --rollback
```

The entire release changes together. Configuration, downloaded models, GPU libraries,
corrections and metrics are preserved; update does not run setup. One previous release
is retained for explicit rollback. After a transition, inspect `status` and repeat a
short dictation, including permission checks if macOS requests them.

## Uninstall

```sh
swictation uninstall
swictation uninstall --yes
```

Review the first command's preview. Confirmed native uninstall removes owned release
files, launcher and service integration while preserving user state. Use the explicit
`--purge-config` or `--purge-cache` flags only for their printed paths. Do not delete
the shared Application Support parent to remove the executable. You may remove the
retired component's privacy permission manually in System Settings afterward.

For bug reports, include `swictation --version`, macOS version, hardware/memory,
`doctor` output, service errors and reproduction steps. Review logs for private data
before sharing them. See [Secretary Mode](secretary-mode.md),
[architecture](architecture.md), and the [macOS host checklist](macos-testing-checklist.md).
