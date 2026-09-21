# Swictation

Local voice dictation for Linux and macOS. Speak, pause, and Swictation types into
your focused application. Includes spoken punctuation and learned corrections.

## Install

```sh
curl -fsSL https://github.com/robertelee78/swictation/releases/latest/download/install.sh | sh
```

Follow any PATH instructions printed by the installer, then set up and start:

```sh
swictation setup
swictation start --ui
```

Setup downloads the speech models and configures the background services.

- **Linux:** x86_64, Ubuntu 24.04+ or equivalent with glibc 2.39+.
  Install the [audio/UI libraries](docs/installation.md#linux-system-libraries)
  and configure your [desktop's text-injection tool and hotkey](docs/window-manager-configs.md).
  NVIDIA CUDA acceleration and CPU fallback are supported.
- **macOS:** Apple Silicon, macOS 14+, 16 GB+ memory. Follow the
  [Microphone and Accessibility permission steps](docs/macos-setup.md#privacy-permissions).

## Dictate

Open a text editor and use your recording shortcut: **Ctrl+Shift+D** on macOS,
or your configured shortcut on Linux. Speak, pause for transcription, then use the
shortcut again to stop recording.

Say “hello comma world period” to type “Hello, world.” See the
[voice command guide](docs/secretary-mode.md) for punctuation and formatting.

```sh
swictation status       # Check whether services are running
swictation stop         # Stop services
swictation start --ui   # Start the daemon and tray UI
```

## Update

```sh
swictation update
```

Updates preserve your settings, models and learned data. Use
`swictation update --check` to check first, or `swictation update --rollback`
to restore a retained previous native release.

## Help

```sh
swictation doctor          # Diagnose setup problems
swictation setup --repair  # Repair setup
swictation logs            # Read service logs
swictation help            # Show commands
```

- [Installation, repair and uninstall](docs/installation.md)
- [macOS setup](docs/macos-setup.md)
- [Linux desktop configuration](docs/window-manager-configs.md)
- [Linux test checklist](docs/linux-native-install-test.md)
- [Architecture](docs/architecture.md) · [Changelog](CHANGELOG.md)

## Development

The CLI and daemon are in `rust-crates/`; the desktop UI is in `tauri-ui/`.
See the [architecture guide](docs/architecture.md) and
[release checklist](docs/RELEASE_CHECKLIST.md).

Licensed under [Apache 2.0](LICENSE).
