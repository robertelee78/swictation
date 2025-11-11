# Swictation CLI

Command-line interface for managing the Swictation voice dictation daemon.

## Features

- 🎤 **Real-time voice transcription** using Parakeet-TDT-1.1B (NVIDIA)
- 🔄 **Smart text transformation** - MidStream library for voice commands
- ⚡ **Low latency** - Pure Rust implementation with CUDA acceleration
- 🖥️ **Wayland native** - wtype text injection for Sway/Wayland
- 🎯 **Hotkey support** - toggle recording with $mod+Shift+D
- 📊 **Real-time metrics** - WPM, latency, GPU/CPU usage
- 🦀 **Pure Rust daemon** - Zero Python runtime dependencies

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
- **Distribution**: Ubuntu 24.04 LTS or newer (GLIBC 2.39+)
  - ✅ Ubuntu 24.04 LTS (Noble Numbat)
  - ✅ Ubuntu 25.10+ (Questing Quetzal)
  - ✅ Debian 13+ (Trixie)
  - ✅ Fedora 39+
  - ❌ Ubuntu 22.04 LTS - NOT supported (GLIBC 2.35 too old)
- **Node.js**: 18.0.0 or higher
- **Python**: 3.8+ (for model downloads via HuggingFace CLI)
- **Storage**: 9.43 GB for AI models
- **Display Server**: Wayland (Sway/i3-compatible compositors)
- **GPU**: NVIDIA with 4GB+ VRAM (CUDA 11.8+) or CPU-only mode
- **Window Managers**: Sway, i3, Hyprland

### Runtime Dependencies
- **Required**:
  - GLIBC 2.39+ (Ubuntu 24.04+)
  - libasound2 (ALSA sound library)
  - systemd (service management)
  - wtype (Wayland text injection)
  - wl-clipboard (Wayland clipboard)
  - PipeWire or PulseAudio (audio capture)
- **Optional**:
  - netcat (nc) - For socket-based control

### Install Dependencies

**Ubuntu/Debian**:
```bash
sudo apt install wtype wl-clipboard pipewire netcat-openbsd
```

**Arch Linux**:
```bash
sudo pacman -S wtype wl-clipboard pipewire gnu-netcat
```

**Fedora**:
```bash
sudo dnf install wtype wl-clipboard pipewire nmap-ncat
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
- **Wayland**: Ensure `wtype` is installed and compositor supports input injection
- Check logs: `journalctl --user -u swictation-daemon -f`

## Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| Linux x64 + NVIDIA GPU | ✅ Supported | Full functionality with CUDA |
| Linux AMD GPU | 🚧 Planned | ROCm support |
| macOS (Apple Silicon/Intel) | 🚧 Planned | CoreML/Metal execution providers |
| Windows + NVIDIA | 🚧 Planned | CUDA support |
| Windows + AMD/Intel | 🚧 Planned | DirectML execution provider |

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