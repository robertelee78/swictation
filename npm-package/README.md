# Swictation

Voice-to-text dictation system with smart text transformation, powered by Whisper and written in Rust.

## Features

- 🎤 **Real-time voice transcription** using OpenAI Whisper
- 🔄 **Smart text transformation** - automatically converts spoken punctuation
- ⚡ **Low latency** - optimized Rust implementation
- 🖥️ **Cross-platform** - works on X11 and Wayland
- 🎯 **Hotkey support** - toggle recording with Super+Shift+D
- 📊 **Real-time metrics** - WPM, latency, and resource usage
- 🎨 **Modern UI** - Tauri-based interface with system tray integration

## Installation

```bash
npm install -g swictation
```

## Quick Start

1. **Initial setup** (configures systemd and hotkeys):
   ```bash
   swictation setup
   ```

2. **Start the service**:
   ```bash
   swictation start
   ```

3. **Launch the UI** (optional):
   ```bash
   swictation start --ui
   ```

4. **Toggle recording**:
   - Use hotkey: `Super+Shift+D`
   - Or command: `swictation toggle`

## Commands

- `swictation start [--ui]` - Start the daemon (and optionally the UI)
- `swictation stop` - Stop the daemon
- `swictation status` - Show service status
- `swictation toggle` - Toggle recording on/off
- `swictation setup` - Configure systemd and hotkeys
- `swictation help` - Show help message

## System Requirements

### Currently Supported
- **OS**: Linux x64
- **Display Server**: X11 or Wayland
- **Window Managers**: Sway, i3, GNOME, KDE

### Dependencies
- **Required**: systemd (for service management)
- **Optional**:
  - `wtype` - For text injection on Wayland
  - `xdotool` - For text injection on X11
  - `netcat (nc)` - For command-line control

### Install Dependencies

**Ubuntu/Debian**:
```bash
sudo apt install wtype xdotool netcat-openbsd
```

**Arch Linux**:
```bash
sudo pacman -S wtype xdotool gnu-netcat
```

**Fedora**:
```bash
sudo dnf install wtype xdotool nmap-ncat
```

## Configuration

Configuration file is located at `~/.config/swictation/config.toml`

### Hotkey Configuration

**Sway** (`~/.config/sway/config`):
```
bindsym Super+Shift+d exec echo '{"action":"toggle"}' | nc -U /tmp/swictation.sock
```

**i3** (`~/.config/i3/config`):
```
bindsym Mod4+Shift+d exec echo '{"action":"toggle"}' | nc -U /tmp/swictation.sock
```

## Architecture

Swictation consists of three main components:

1. **Daemon** (`swictation-daemon`) - Rust service handling audio capture and transcription
2. **UI** (`swictation-ui`) - Tauri application for metrics and control
3. **CLI** (`swictation`) - Node.js command-line interface

## Troubleshooting

### Service won't start
```bash
# Check status
swictation status

# View logs
journalctl --user -u swictation-daemon -f
```

### No audio input
```bash
# List audio devices
arecord -l

# Test microphone
arecord -d 5 test.wav && aplay test.wav
```

### Text not being typed
- **Wayland**: Install `wtype` package
- **X11**: Install `xdotool` package

## Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| Linux x64 | ✅ Supported | Full functionality |
| Linux ARM | 🚧 Coming soon | Raspberry Pi support planned |
| macOS | 🚧 Coming soon | M1/M2 and Intel |
| Windows | 🚧 Coming soon | Windows 10/11 |

## Development

Source code: https://github.com/yourusername/swictation

### Building from source
```bash
# Clone repository
git clone https://github.com/yourusername/swictation
cd swictation

# Build Rust components
cd rust-crates
cargo build --release

# Build Tauri UI
cd ../tauri-ui
npm install
npm run build
```

## License

MIT

## Contributing

Contributions are welcome! Please read our [Contributing Guide](https://github.com/yourusername/swictation/blob/main/CONTRIBUTING.md) for details.

## Support

- Issues: https://github.com/yourusername/swictation/issues
- Discussions: https://github.com/yourusername/swictation/discussions