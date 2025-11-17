# Swictation CLI

Command-line interface for managing the Swictation voice dictation daemon.

## ✨ Wayland Support - Fully Automated! ✨

**NEW in v0.4.0**: Swictation now fully supports Wayland-based Linux desktops with **automated installation**:

- ✅ **GNOME Wayland** (Ubuntu 25.10+) - Hotkeys auto-configured
- ✅ **Sway** - Text injection auto-setup
- ✅ **GPU Acceleration** - CUDA libraries auto-downloaded
- ✅ **Service Management** - Auto-enabled systemd service

**One command installation:**
```bash
npm install -g swictation --foreground-scripts
# Everything is configured automatically!
```

**[→ Complete Wayland Support Guide](https://github.com/robertelee78/swictation/blob/main/docs/WAYLAND_SUPPORT.md)**

## Features

- 🎤 **Real-time voice transcription** using Parakeet-TDT-1.1B (NVIDIA)
- 📝 **Secretary Mode** - 60+ natural language commands for dictation
  - Say "comma" → Get ","
  - Say "mr smith said quote hello quote" → Get "Mr. Smith said 'Hello'"
  - Automatic capitalization, punctuation, numbers, quotes, formatting
  - **[→ Full Secretary Mode Guide](https://github.com/robertelee78/swictation/blob/main/docs/secretary-mode.md)**
- 🔄 **Smart text transformation** - MidStream Rust library (~5µs latency)
- ⚡ **Low latency** - Pure Rust implementation with CUDA acceleration
- 🖥️ **Wayland & X11 Support** - Full dual display server support
- 🎯 **Hotkey support** - Auto-configured on GNOME, manual setup on Sway
- 📊 **Real-time metrics** - WPM, latency, GPU/CPU usage
- 🦀 **Pure Rust daemon** - Zero Python runtime dependencies

## System Requirements

### Required
- **Operating System**: Ubuntu 24.04 LTS or later (x64)
- **Node.js**: >= 18.0.0
- **GLIBC**: >= 2.39 (Ubuntu 24.04+)
- **libasound2**: Audio library for ALSA
  ```bash
  # Ubuntu 24.04+
  sudo apt-get install libasound2t64

  # Ubuntu 22.04 and older
  sudo apt-get install libasound2
  ```

### Optional (for GPU Acceleration)
- **NVIDIA GPU**: Any CUDA-capable GPU
- **CUDA**: 12.x or later
- **VRAM**: 4GB minimum (6GB+ recommended for 1.1B model)
- **Driver**: NVIDIA proprietary drivers (not nouveau)

**Note**: GPU acceleration libraries (~2.3GB CUDA + cuDNN) are automatically downloaded during installation. These include CUDA runtime and cuDNN libraries optimized for your GPU architecture.

If GPU is not available, Swictation will run in CPU-only mode (slower transcription).

## Installation

### One-Command Installation

**Recommended: User-Local Installation (No sudo required)**

#### If using nvm (Node Version Manager)

**nvm already handles user-local installations automatically!** No additional setup needed:

```bash
# Just install (nvm manages the prefix)
npm install -g swictation --foreground-scripts
```

**The installation will automatically:**
1. Download GPU acceleration libraries (~2.3GB, if NVIDIA GPU detected)
2. Install and configure ydotool (Wayland text injection)
3. Configure GNOME keyboard shortcuts (if on GNOME Wayland)
4. Set up and enable systemd service
5. Start the daemon service

**Important**: If you previously set a custom npm prefix, remove it:
```bash
npm config delete prefix
```

#### If NOT using nvm

Configure npm to install packages to your home directory:

```bash
# One-time setup: Configure npm to use user-local directory
mkdir -p ~/.npm-global
npm config set prefix ~/.npm-global

# Add to PATH (add this line to ~/.bashrc or ~/.zshrc)
export PATH="$HOME/.npm-global/bin:$PATH"

# Source your shell config to apply changes
source ~/.bashrc  # or source ~/.zshrc

# Now install without sudo
npm install -g swictation --foreground-scripts
```

### Alternative: System-Wide Installation (Requires sudo)

```bash
# Install with visible output
sudo npm install -g swictation --foreground-scripts

# Or standard install (output hidden by npm)
sudo npm install -g swictation
```

**Why user-local installation is recommended**:
- ✅ No sudo required
- ✅ No permission issues
- ✅ Each user has their own installation
- ✅ Cleaner and more secure
- ✅ Works with nvm seamlessly

**Note**: The `--foreground-scripts` flag makes the installation progress visible. The postinstall script will:
1. Download GPU acceleration libraries (~2.3GB CUDA + cuDNN, if NVIDIA GPU detected)
2. Install and configure ydotool (Wayland text injection)
3. Set up uinput kernel module and permissions
4. Configure GNOME keyboard shortcuts (if on GNOME Wayland)
5. Generate systemd service files with GPU library paths
6. Enable and start the daemon service
7. Test GPU model loading (if GPU detected)

### Installation Options

You can customize the installation with environment variables:

```bash
# Default: Standard install with GPU model testing (if GPU detected)
npm install -g swictation --foreground-scripts

# Skip model testing for CI/headless environments (faster install)
SKIP_MODEL_TEST=1 npm install -g swictation --foreground-scripts
```

**Installation behavior:**
- **Default**: Model test-loading runs automatically when GPU is detected (~30s to verify)
- **SKIP_MODEL_TEST=1**: Skips model testing entirely (useful for CI/automated environments)

**Note**: Model testing is recommended to ensure your GPU setup works correctly. It verifies that the daemon can load models with your CUDA installation.

## Quick Start

**GNOME Wayland (Ubuntu 25.10+)**: Everything is auto-configured! Just start using it:
```bash
# Installation already configured everything, just start using:
swictation status         # Check daemon is running
# Press Super+Shift+D to toggle recording (already configured!)
```

**Sway/Other Compositors**: Add hotkey binding to your config (see Hotkey Configuration section), then:
```bash
swictation status         # Check daemon is running
# Use your configured hotkey to toggle recording
```

**Manual Setup** (if needed):
```bash
swictation setup          # Configure systemd service
swictation start          # Start the daemon
swictation toggle         # Toggle recording via command
```

**Optional UI**:
```bash
swictation start --ui     # Launch with visual interface
```

## Commands

- `swictation start [--ui]` - Start the daemon (and optionally the UI)
- `swictation stop` - Stop the daemon
- `swictation status` - Show service status
- `swictation toggle` - Toggle recording on/off
- `swictation setup` - Configure systemd and hotkeys
- `swictation help` - Show help message

## Display Server Support

### Fully Supported (Automated Setup)
- ✅ **GNOME Wayland** (Ubuntu 25.10+) - Keyboard shortcuts auto-configured
- ✅ **Sway** - Text injection auto-setup, manual hotkey configuration
- ✅ **X11** - Traditional X11 support with xdotool

### System Requirements
- **OS**: Linux x64
- **Distribution**: Ubuntu 24.04 LTS or newer (GLIBC 2.39+)
  - ✅ Ubuntu 24.04 LTS (Noble Numbat)
  - ✅ Ubuntu 25.10+ (Questing Quetzal) - **Recommended for Wayland**
  - ✅ Debian 13+ (Trixie)
  - ✅ Fedora 39+
  - ❌ Ubuntu 22.04 LTS - NOT supported (GLIBC 2.35 too old)
- **Node.js**: 18.0.0 or higher
- **Storage**: ~12 GB total (AI models + GPU libraries)
- **GPU**: NVIDIA with 4GB+ VRAM (CUDA 12.x) or CPU-only mode

### GPU Requirements (1.1B Model - Recommended)
For optimal performance with the 1.1B model (62.8x realtime speed):
- **GPU**: NVIDIA GPU with 4GB+ VRAM
- **CUDA**: 11.8+ or 12.x
- **Compute Capability**: 7.0+ (Turing architecture or newer)
- **ONNX Runtime**: onnxruntime-gpu 1.16.0+

### GPU Acceleration
GPU acceleration libraries (CUDA + cuDNN) are **automatically downloaded** during npm installation. No manual CUDA installation required!

### Runtime Dependencies

**Automatically Installed by npm postinstall:**
- ydotool (Wayland text injection) - Auto-installed on Wayland systems
- netcat (IPC socket communication) - Auto-installed

**Pre-installed on most systems:**
- GLIBC 2.39+ (Ubuntu 24.04+)
- libasound2 (ALSA sound library)
- systemd (service management)
- PipeWire or PulseAudio (audio capture)

**For X11 systems (optional):**
```bash
sudo apt install xdotool  # Ubuntu/Debian
sudo pacman -S xdotool    # Arch Linux
sudo dnf install xdotool  # Fedora
```

## Configuration

Configuration file is located at `~/.config/swictation/config.toml`

### Hotkey Configuration

**GNOME Wayland**: ✅ **Automatically configured during installation!**
- Default hotkey: `Super+Shift+D`
- Visible in: Settings → Keyboard → View and Customize Shortcuts
- **No manual configuration needed**

**Sway** - Add to `~/.config/sway/config`:
```bash
# Swictation toggle
bindsym $mod+Shift+d exec sh -c 'echo "{\"action\": \"toggle\"}" | nc -U /tmp/swictation.sock'

# Optional: Push-to-talk
bindsym $mod+Space exec sh -c 'echo "{\"action\": \"ptt_press\"}" | nc -U /tmp/swictation.sock'
bindsym --release $mod+Space exec sh -c 'echo "{\"action\": \"ptt_release\"}" | nc -U /tmp/swictation.sock'
```
Then reload: `swaymsg reload`

**X11 (i3, etc.)** - Add to config:
```bash
bindsym Mod4+Shift+d exec echo '{"action":"toggle"}' | nc -U /tmp/swictation.sock
```

**[→ See Complete Wayland Support Guide](https://github.com/robertelee78/swictation/blob/main/docs/WAYLAND_SUPPORT.md)** for detailed setup and troubleshooting.

## Architecture

Swictation consists of three main components:

1. **Daemon** (`swictation-daemon`) - Rust service handling audio capture and transcription
2. **UI** (`swictation-ui`) - Tauri application for metrics and control
3. **CLI** (`swictation`) - Node.js command-line interface

## Verification

After installation, verify everything is working:

```bash
# Run comprehensive verification script
./node_modules/swictation/scripts/verify-installation.sh

# Or check manually
swictation status        # Check daemon status
systemctl --user status swictation-daemon.service
journalctl --user -u swictation-daemon -n 50  # View logs
```

**Expected on Wayland:**
- ✓ Wayland detected
- ✓ GNOME/Sway detected (as applicable)
- ✓ GPU libraries installed (if NVIDIA GPU present)
- ✓ ydotool installed
- ✓ uinput module loaded
- ✓ User in 'input' group
- ✓ GNOME keyboard shortcut configured (if GNOME)
- ✓ Service enabled and running
- ✓ IPC socket responding

## Troubleshooting

### Wayland-Specific Issues

**Text not typing after transcription**:
```bash
# 1. Verify ydotool permissions
sg input -c 'ydotool type "test"'

# 2. Check group membership (may need logout/login)
groups | grep input

# 3. Verify uinput permissions
ls -l /dev/uinput
# Should show: crw-rw---- 1 root input

# 4. Re-run setup if needed
./node_modules/swictation/scripts/setup-ydotool.sh
```

**GNOME: Hotkey not working (types "D" instead)**:
```bash
# Verify shortcut configuration
gsettings get org.gnome.settings-daemon.plugins.media-keys custom-keybindings

# Re-configure if needed
./node_modules/swictation/scripts/setup-gnome-shortcuts.sh

# Check in Settings → Keyboard → View and Customize Shortcuts
# Look for "Swictation Toggle"
```

**Slow transcription (20-30 seconds instead of <1 second)**:
This means CPU mode instead of GPU. **Always start daemon via systemctl**, not manually!
```bash
# Restart via systemctl (has correct LD_LIBRARY_PATH)
systemctl --user restart swictation-daemon.service

# Check logs for GPU detection
journalctl --user -u swictation-daemon.service | grep CUDA
# Expected: "Successfully registered `CUDAExecutionProvider`"
# Expected: "cuDNN version: 91501"
```

### General Installation Issues

#### "Cannot see postinstall output"
**Solution**: Use the `--foreground-scripts` flag:
```bash
sudo npm install -g swictation --foreground-scripts
```

#### "GPU libraries download failed (404)"
**Cause**: GitHub release for this version doesn't exist yet.
**Solution**: This only happens with unreleased versions. Published npm versions will work.

#### "libasound.so.2: cannot open shared object file"
**Cause**: Missing ALSA library.
**Solution**:
```bash
# Ubuntu 24.04+
sudo apt-get install libasound2t64

# Ubuntu 22.04 and older
sudo apt-get install libasound2
```

#### "Model test-loading failed during installation"
**Cause**: GPU not detected or CUDA libraries missing.
**Solution**: This is non-fatal. Swictation will fall back to CPU mode. To verify GPU setup after installation:
```bash
# Check if CUDA is available
nvidia-smi

# Check CUDA libraries
ls /usr/local/cuda/lib64/
```

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
- **Wayland**: See "Wayland-Specific Issues" section above
- **X11**: Ensure `xdotool` is installed
- Check logs: `journalctl --user -u swictation-daemon -f`

### Audio device issues

**Only 1 audio device detected (should see 4+)**:
The systemd service should automatically import PulseAudio/PipeWire environment variables. Verify the service is running:
```bash
systemctl --user status swictation-daemon.service
```

### Real-time transcription not working (only transcribes at end)

**Cause**: VAD threshold too low - background noise exceeds threshold.

**Solution**: Adjust in `~/.config/swictation/config.toml`:
```toml
vad_threshold = 0.25      # Default optimized for real-time
vad_min_silence = 0.8     # Seconds of silence before flush
```

Lower threshold (0.003) causes continuous speech detection. Optimal is 0.25 for balanced sensitivity.

### After fixing environment variables

Always reload systemd and restart the daemon:
```bash
systemctl --user daemon-reload
systemctl --user restart swictation-daemon

# Verify it started correctly
systemctl --user status swictation-daemon
journalctl --user -u swictation-daemon -n 50
```

## Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| **Linux Wayland (GNOME)** | ✅ **Fully Supported** | Auto-configured hotkeys, ydotool setup |
| **Linux Wayland (Sway)** | ✅ **Fully Supported** | Auto ydotool setup, manual hotkeys |
| **Linux X11** | ✅ Supported | Traditional X11 support |
| **NVIDIA GPU Acceleration** | ✅ Supported | Auto-downloaded CUDA + cuDNN |
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

## Additional Resources

- **[Wayland Support Guide](https://github.com/robertelee78/swictation/blob/main/docs/WAYLAND_SUPPORT.md)** - Complete Wayland setup and troubleshooting
- **[Secretary Mode Guide](https://github.com/robertelee78/swictation/blob/main/docs/secretary-mode.md)** - Natural language commands reference
- **[Testing & Validation Guide](https://github.com/robertelee78/swictation/blob/main/docs/testing/README.md)** - Display server testing procedures

## Support

- Issues: https://github.com/robertelee78/swictation/issues
- Discussions: https://github.com/robertelee78/swictation/discussions
- Source Code: https://github.com/robertelee78/swictation