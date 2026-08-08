# Swictation

Voice-to-text dictation for Linux and macOS with GPU acceleration. Pure Rust daemon with VAD-triggered auto-transcription, sub-second latency, and complete privacy.

Supported platforms:

- Linux x64 (Ubuntu 24.04+, GLIBC 2.39+)
- macOS Apple Silicon (M1 or later), macOS 14 Sonoma+

## Install

```bash
npm install -g swictation --foreground-scripts
```

The `--foreground-scripts` flag shows installation progress. Postinstall automatically:

1. Detects your GPU and downloads optimized acceleration libraries
2. Downloads and test-loads AI models (~30-60s on first install)
3. Sets up the background service (systemd on Linux, launchd on macOS)
4. Shows platform-specific setup instructions

## Platform Requirements

**Linux x64**
- Ubuntu 24.04+ (GLIBC 2.39+), Node.js 18+
- Optional: NVIDIA GPU with 4GB+ VRAM for the 0.6B model, 6GB+ for the 1.1B model
  (CPU fallback available)

**macOS Apple Silicon (M1 or later)**
- macOS 14 Sonoma or later, Node.js 18+
- 16GB+ unified memory — a hard requirement. Postinstall refuses to install on Macs
  reporting less, because the CoreML models need the headroom.
- Intel Macs are not supported
- Accessibility permissions granted during setup

## Commands

```bash
swictation download-models   # Download AI models (alias: download-model)
swictation setup             # Install services, configure hotkeys/permissions
swictation start [--ui]      # Start the daemon (and optionally the tray UI)
swictation stop              # Stop the daemon
swictation status            # Show service, socket, and platform status
swictation toggle            # Toggle recording on/off
swictation help              # Full usage
swictation --version         # Show version
```

Postinstall runs `download-models` and `setup` for you; run them by hand if the install
was interrupted.

`download-models` always fetches Silero VAD (629 KB) plus one or more STT models. Choose
with `--model=` (a bare positional also works, e.g. `download-model 1.1b-gpu`) and add
`--force` to re-download:

- **Linux:** `0.6b` (2.55 GB; aliases `0.6b-gpu`, `0.6b-cpu`, `cpu-only`), `1.1b`
  (6.96 GB; alias `1.1b-gpu`), or `both` (default, ~9.5 GB)
- **macOS:** `1.1b-coreml` (1.9 GB; alias `coreml-native`), `0.6b-coreml` (2.67 GB), or
  `both` (default — VAD plus the CoreML 1.1B bundle)

## Where things live

| | Linux | macOS |
|---|---|---|
| Config | `~/.config/swictation/` | `~/Library/Application Support/swictation/` |
| Data + models | `~/.local/share/swictation/` | `~/Library/Application Support/swictation/` |
| Install log | `~/.local/share/swictation/install.log` | `~/Library/Logs/swictation/install.log` |
| Daemon logs | `journalctl --user -u swictation-daemon` | `~/Library/Logs/swictation/daemon.log` |

## Uninstall

```bash
# 1. Stop the services FIRST — npm will not do it for you
swictation stop
systemctl --user disable swictation-daemon              # Linux
launchctl bootout gui/$(id -u)/com.swictation.daemon    # macOS

# 2. Remove the package
npm uninstall -g swictation
```

npm 7 and later do not run uninstall lifecycle scripts, so the bundled `preuninstall.js`
cleanup never fires on modern npm — stop and disable the service yourself first, or the
unit is left pointing at deleted binaries.

User data and models are preserved on uninstall (up to ~9.5GB of models on Linux).
Delete them explicitly if you want them gone:

```bash
rm -rf ~/.config/swictation ~/.local/share/swictation                       # Linux
rm -rf ~/Library/Application\ Support/swictation ~/Library/Logs/swictation  # macOS
```

## Documentation

Full documentation, configuration reference, and troubleshooting guides:
https://github.com/robertelee78/swictation

- macOS setup: https://github.com/robertelee78/swictation/blob/main/docs/macos-setup.md
- Secretary Mode: https://github.com/robertelee78/swictation/blob/main/docs/secretary-mode.md
- Window manager configs: https://github.com/robertelee78/swictation/blob/main/docs/window-manager-configs.md

## Links

- Source: https://github.com/robertelee78/swictation
- Issues: https://github.com/robertelee78/swictation/issues

## License

Apache-2.0