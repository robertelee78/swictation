# Swictation CLI

Command-line interface for managing the Swictation voice dictation daemon.

## Features

- 🎤 **Real-time voice transcription** using Parakeet-TDT-1.1B (NVIDIA)
- 📝 **Secretary Mode** - 60+ natural language commands for dictation
  - Say "comma" → Get ","
  - Say "mr smith said quote hello quote" → Get "Mr. Smith said 'Hello'"
  - Automatic capitalization, punctuation, numbers, quotes, formatting
  - **[→ Full Secretary Mode Guide](https://github.com/robertelee78/swictation/blob/main/docs/secretary-mode.md)**
- 🔄 **Smart text transformation** - MidStream Rust library (~1µs latency)
- ⚡ **Low latency** - Pure Rust implementation with CUDA acceleration
- 🖥️ **Wayland native** - wtype text injection for Sway/Wayland
- 🎯 **Hotkey support** - toggle recording with $mod+Shift+D
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

**Note**: GPU acceleration libraries (~209MB) are downloaded during installation from a **separate versioned release** (`gpu-libs-v1.0.0`). These are versioned independently from the npm package to avoid duplication across releases.

If GPU is not available, Swictation will run in CPU-only mode.

## Installation

### Recommended: User-Local Installation (No sudo required)

Configure npm to install packages to your home directory:

```bash
# One-time setup: Configure npm to use user-local directory
mkdir -p ~/.npm-global
npm config set prefix ~/.npm-global

# Add to PATH (add this line to ~/.bashrc or ~/.zshrc)
export PATH="$HOME/.npm-global/bin:$PATH"

# Source your shell config to apply changes
source ~/.bashrc  # or source ~/.zshrc

# Now install without sudo (recommended)
npm install -g swictation --foreground-scripts
```

### Alternative: System-Wide Installation (Requires sudo)

```bash
# Install with visible output
sudo npm install -g swictation --foreground-scripts

# Or standard install (output hidden by npm)
sudo npm install -g swictation
```

**Note**: User-local installation is recommended because:
- ✅ No sudo required
- ✅ No permission issues
- ✅ Each user has their own installation
- ✅ Cleaner and more secure

**Note**: The `--foreground-scripts` flag makes the installation progress visible. The postinstall script will:
1. Download GPU acceleration libraries (~209MB, if GPU detected)
2. Generate systemd service files
3. Set up configuration directories
4. Test GPU model loading (if GPU detected)

### Installation Options

You can customize the installation with environment variables:

```bash
# Default: Fast install, no model test-loading
npm install -g swictation

# Enable optional model test-loading (adds ~30s to verify GPU model loading)
TEST_MODEL_LOADING=1 npm install -g swictation

# Skip for CI/headless environments (minimal setup)
SKIP_MODEL_TEST=1 npm install -g swictation
```

**When to use:**
- `TEST_MODEL_LOADING=1` - Verify your GPU can load models during installation (useful for troubleshooting)
- `SKIP_MODEL_TEST=1` - For automated/CI environments where you want fastest install
- Default (no flags) - Recommended for most users (fast, no test-loading)

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

### GPU Requirements (1.1B Model - Recommended)
For optimal performance with the 1.1B model (62.8x realtime speed):
- **GPU**: NVIDIA GPU with 4GB+ VRAM
- **CUDA**: 11.8+ or 12.x
- **Compute Capability**: 7.0+ (Turing architecture or newer)
- **ONNX Runtime**: onnxruntime-gpu 1.16.0+

### Python Environment
The 1.1B GPU-accelerated model requires ONNX Runtime GPU:

```bash
pip3 install onnxruntime-gpu
```

**CRITICAL**: The postinstall script automatically detects ONNX Runtime and configures the systemd service. If detection fails, you'll need to manually set `ORT_DYLIB_PATH` (see Troubleshooting below).

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

### Installation Issues

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
- **Wayland**: Ensure `wtype` is installed and compositor supports input injection
- Check logs: `journalctl --user -u swictation-daemon -f`

### "100% blank predictions" or empty transcriptions

**Cause**: Missing ONNX Runtime library path in systemd service.

**Solution**: The postinstall script should automatically detect this. If it fails, manually add to `~/.config/systemd/user/swictation-daemon.service`:

```ini
[Service]
Environment="ORT_DYLIB_PATH=/path/to/onnxruntime/capi/libonnxruntime.so.X.Y.Z"
```

To find your ONNX Runtime path:
```bash
python3 -c "import onnxruntime; import os; print(os.path.join(os.path.dirname(onnxruntime.__file__), 'capi'))"
ls -la /path/from/above/libonnxruntime.so*
```

Then reload and restart:
```bash
systemctl --user daemon-reload
systemctl --user restart swictation-daemon
```

### Extremely slow performance (15+ seconds per transcription)

**Cause**: Missing CUDA library paths or using INT8 quantized models on GPU.

**Current behavior**: Should be 62.8x realtime (0.16s for 10s audio) with 1.1B FP32 model on GPU.

**Solution 1 - Check CUDA paths**: Verify in `~/.config/systemd/user/swictation-daemon.service`:
```ini
Environment="LD_LIBRARY_PATH=/usr/local/cuda/lib64:/usr/local/lib/node_modules/swictation/lib/native"
```

**Solution 2 - Verify using FP32 models**: Check logs for "Using FP32 model for GPU":
```bash
journalctl --user -u swictation-daemon -f
```

If you see "Using INT8 quantized model", the system is incorrectly using quantized models on GPU (no CUDA kernels). This should be auto-detected but can be forced in `~/.config/swictation/config.toml`:
```toml
stt_model_override = "1.1b-gpu"  # Forces FP32 models
```

After changes:
```bash
systemctl --user daemon-reload
systemctl --user restart swictation-daemon
```

### Only 1 audio device detected (should see 4+)

**Cause**: Missing PulseAudio/PipeWire session variables.

**Solution**: Verify in `~/.config/systemd/user/swictation-daemon.service`:
```ini
# Import full user environment for PulseAudio
ImportEnvironment=
```

This imports all user session variables including `PULSE_SERVER`, `DBUS_SESSION_BUS_ADDRESS`, etc.

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