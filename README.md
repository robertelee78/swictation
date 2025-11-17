# Swictation

**High-performance voice-to-text dictation daemon for Linux (X11/Wayland) with GPU acceleration**

> Pure Rust implementation with VAD-triggered auto-transcription, sub-second latency, and complete privacy.

[![Status](https://img.shields.io/badge/status-Production%20Ready-green)](https://github.com/robertelee78/swictation)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue)](LICENSE)
[![Language](https://img.shields.io/badge/language-Rust-orange)](https://www.rust-lang.org/)

---

## **Quick Start** 🚀

### Prerequisites

- ✅ **NVIDIA GPU** with 4GB+ VRAM (RTX A1000/3050/4060 or better)
  - Uses ~2.2GB VRAM typical, ~3.5GB peak with FP16 optimization
- ✅ **Linux** with X11 or Wayland (Ubuntu 24.04+ recommended)
- ✅ **Node.js** 18+ (for npm installation)
- ✅ **Text injection tool:** xdotool (X11), wtype (Wayland), or ydotool (universal)
- ✅ **System tools:** wl-clipboard (optional), CUDA 11.8+

```bash
# Install system dependencies (Ubuntu/Debian 24.04+)
# For X11 (most users, fastest):
sudo apt install xdotool pipewire nvidia-cuda-toolkit

# For Wayland (GNOME - Ubuntu 24.04 default):
sudo apt install ydotool pipewire nvidia-cuda-toolkit
sudo usermod -aG input $USER  # Required for ydotool
# Then log out and log back in

# For Wayland (KDE/Sway/Hyprland):
sudo apt install wtype pipewire nvidia-cuda-toolkit

# Install system dependencies (Arch/Manjaro)
# Choose based on your environment (X11/Wayland/GNOME)
sudo pacman -S xdotool pipewire cuda  # X11
sudo pacman -S wtype pipewire cuda    # Wayland (non-GNOME)
sudo pacman -S ydotool pipewire cuda  # Wayland (GNOME) or universal

# See docs/installation-by-distro.md for complete guide
```

### GPU Support

Swictation automatically detects your GPU architecture and downloads optimized libraries:

| GPU Generation | Architectures | Example GPUs | Package Size |
|----------------|--------------|--------------|--------------|
| **Maxwell / Pascal / Volta** | sm_50-70 | GTX 750/900/1000 series<br>Quadro M/P series<br>Titan V, V100 | ~1.5GB |
| **Turing / Ampere** | sm_75-86 | GTX 16 series<br>RTX 20/30 series<br>A100, RTX A1000-A6000 | ~1.5GB |
| **Ada / Hopper / Blackwell** | sm_89-120 | RTX 4090<br>H100, B100/B200<br>RTX PRO 6000 Blackwell<br>RTX 50 series | ~1.5GB |

**Key Benefits:**
- **Automatic detection** - Zero configuration required
- **Optimized downloads** - 65-74% smaller than universal binaries
- **Wide compatibility** - Supports all NVIDIA GPUs from Maxwell (2014) through current Blackwell architecture
- **Native support** - No PTX hacks, native compilation for all architectures

**Unsupported GPUs** (< sm_50):
- GTX 600/700 series (Kepler)
- Quadro K series
- System automatically falls back to CPU mode

For more details, see [GPU Library Packages Documentation](docs/implementation/gpu-multi-package-guide.md).

### Installation (Recommended: npm)

**Easiest method - installs pre-built binaries:**

```bash
# One-time npm configuration (avoids sudo requirement)
echo "prefix=$HOME/.npm-global" > ~/.npmrc
export PATH="$HOME/.npm-global/bin:$PATH"
echo 'export PATH="$HOME/.npm-global/bin:$PATH"' >> ~/.profile

# Install globally (gets latest version)
npm install -g swictation --foreground-scripts

# The postinstall script will:
# - Detect your GPU architecture (sm_50-120) and download optimized libraries (~1.5GB)
# - Measure VRAM and recommend optimal AI model (1.1B for 6GB+, 0.6B for 3.5GB+)
# - Test-load the model to verify it works (~30-60 seconds)
# - Install systemd services automatically
# - Display hotkey setup instructions (manual configuration required)

# ⏱️  Note: Installation includes model test-loading (30-60s)
# This verifies GPU/VRAM compatibility before first use

# Alternative: Install to system-wide location (requires sudo):
# sudo npm install -g swictation --foreground-scripts --unsafe-perm

# Start the daemon
swictation start

# Check status
systemctl --user status swictation-daemon
```

### Installation (Manual Build)

**For developers or if you want to build from source:**

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Clone with submodules
git clone --recurse-submodules https://github.com/robertelee78/swictation.git /opt/swictation
cd /opt/swictation

# Build Rust daemon (release mode)
cd rust-crates
cargo build --release

# Install systemd service
mkdir -p ~/.config/systemd/user
cp config/swictation-daemon.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable swictation-daemon
systemctl --user start swictation-daemon

# Setup Sway hotkey (automatic for npm install)
./scripts/setup-sway.sh
swaymsg reload
```

### Your First Recording

1. **Open any text editor** (kate, gedit, VSCode, vim, etc.)
2. **Press `$mod+Shift+d`** to start recording
3. **Speak naturally:** "Hello world." [brief pause]
4. **Text appears automatically** after configured silence duration
5. **Press `$mod+Shift+d`** again to stop recording

---

## **Architecture** 🏗️

### Pure Rust Implementation

Swictation is a **native Rust application** with zero Python runtime dependencies. The entire processing pipeline runs in compiled native code for maximum performance.

```
┌────────────────────────────────────────────────────────┐
│         SWICTATION-DAEMON (Rust Binary)                │
│  State: [IDLE] ↔ [RECORDING] ↔ [PROCESSING]            │
└────────────────────────────────────────────────────────┘
           ↓              ↓              ↓
    ┌──────────┐   ┌──────────┐   ┌─────────────┐
    │  Audio   │   │   VAD    │   │     STT     │
    │ Capture  │→  │  Silero  │→  │  Parakeet   │
    │ (cpal)   │   │  v6 ONNX │   │  TDT-1.1B   │
    └──────────┘   └──────────┘   └─────────────┘
                                          ↓
                                   ┌─────────────┐
                                   │  Transform  │
                                   │  MidStream  │
                                   │    (Rust)   │
                                   └─────────────┘
                                          ↓
                                   ┌─────────────┐
                                   │   Inject    │
                                   │    wtype    │
                                   │  (Wayland)  │
                                   └─────────────┘
```

### Rust Workspace Crates

```
rust-crates/
├── swictation-daemon/      # Main daemon binary (tokio async)
├── swictation-audio/       # Audio capture (cpal/PipeWire)
├── swictation-vad/         # Voice Activity Detection (Silero v6 + ort)
├── swictation-stt/         # Speech-to-Text (Parakeet-TDT + ort)
├── swictation-metrics/     # Performance tracking
└── swictation-broadcaster/ # Real-time metrics broadcast

external/midstream/         # Text transformation (Git submodule)
└── crates/text-transform/  # Voice commands → symbols
```

### Key Technical Decisions
**ONNX Runtime (ort):**
- Uses latest ort crate (2.0.0-rc.10) with direct CUDA support
- ~150x faster than sherpa-rs for VAD operations
- Modern execution providers (CUDA, TensorRT, DirectML)
- Silero VAD v6 (August 2024, 16% less errors)
- Parakeet-TDT-1.1B via parakeet-rs (5.77% WER)

**Architecture Details:**
- **Core transcription daemon** is **100% Rust** (no Python runtime required)
- **Optional Python utilities** in `src/` directory:
  - System tray UI (`swictation_tray.py` - PySide6 for Wayland compatibility)
  - Configuration loader (`config_loader.py` - TOML parsing utilities)
  - CLI tools (`swictation_cli.py` - daemon control via Unix socket)
- systemd service executes pure Rust: `swictation-daemon` binary
- Python not required for transcription - only for optional UI/tools

---

## **Features** ✨

### Core Capabilities
- 🎙️ **VAD-Triggered Segmentation** - Auto-transcribe on natural pauses (0.25 confidence threshold, 0.8s silence duration)
- 🎯 **Sub-Second Latency** - Real-time text injection with full segment accuracy
- 🔒 **100% Privacy** - All processing on local GPU, no cloud
- ⚡ **GPU Optimized** - Silero VAD v6 (ONNX) + Parakeet-TDT-1.1B (CUDA)
- 🖥️ **Display Server Support** - Automatic detection: X11 (xdotool), Wayland (wtype/ydotool), GNOME Wayland (ydotool)
- ⌨️ **Hotkey Control** - `$mod+Shift+d` toggle via global-hotkey crate
- 🔄 **systemd Integration** - Auto-start with Sway
- 🦀 **Pure Rust** - Native performance, memory safety, zero Python dependency hell

### Technical Highlights
- **STT Model:** Parakeet-TDT-1.1B (5.77% WER, parakeet-rs)
- **VAD Model:** Silero VAD v6 (~630KB, ort 2.0.0-rc.10, threshold: 0.25 optimized for real-time)
- **Text Transform:** MidStream Rust crate (~1µs latency)
- **Audio Capture:** cpal with PipeWire backend
- **Text Injection:** Auto-selects xdotool (X11), wtype (Wayland), or ydotool (GNOME/universal)
- **Daemon Runtime:** Tokio async with state machine
- **ONNX Runtime:** Direct ort crate bindings (CUDA 11.8+)
- **Hotkeys:** global-hotkey crate (cross-platform)

---

## **VAD-Triggered Segmentation** 🎙️

### How It Works

```
[Toggle ON] → Continuous recording starts
    ↓
[You speak] → Audio accumulates in buffer
    ↓
[Brief silence (0.8s)] → VAD detects pause → Transcribe segment → Inject text
    ↓
[You speak again] → New segment starts
    ↓
[Toggle OFF] → Stop recording
```

### ONNX Threshold Configuration

**Daemon Default Configuration:**

```toml
# ~/.config/swictation/config.toml (runtime defaults)
[vad]
threshold = 0.25  # Empirically optimized for real-time transcription
min_silence_duration = 0.8  # Seconds of silence before transcription
min_speech_duration = 0.25  # Minimum speech length to process

# Note: Threshold range is 0.0-1.0 (higher = less sensitive to background noise)
# 0.25 provides optimal silence detection for real-time dictation
```

### Performance Characteristics

**Latency breakdown (RTX A1000, 0.8s silence threshold):**
```
VAD processing       → 50ms
Silence threshold    → 800ms (configurable)
STT transcription    → 150-250ms
Text transformation  → 1µs
Text injection       → 10-50ms
────────────────────────────────
Total after pause    → ~1.0s
```

**Memory usage:**
- GPU: ~2.2GB typical, ~3.5GB peak
  - Parakeet-TDT model: ~1.8GB
  - Context buffer: ~400MB
  - Silero VAD: ~630KB
- System RAM: ~150MB (Rust daemon)

---

## **Secretary Mode: Natural Dictation** 🎤⚡

### What is Secretary Mode?

**Secretary Mode** transforms spoken voice commands into written text, inspired by 1950s stenography. Speak naturally and dictate punctuation, formatting, and symbols exactly as you would to a secretary.

```
┌─────────────────────────────────────────────────┐
│  YOU SPEAK → STT → Transform → Text Injection   │
│  "comma"  →  "comma"  →  ","  →  types ","      │
└─────────────────────────────────────────────────┘
```

**Performance:** ~1µs per transformation (pure Rust, HashMap O(1) lookup)

### Quick Examples

**Basic Punctuation:**
```
YOU SAY:          "hello comma world period"
SWICTATION TYPES: Hello, world.
```

**Formal Letter:**
```
YOU SAY:          "dear mr smith comma new paragraph I need to schedule..."
SWICTATION TYPES: Dear Mr. Smith,

                  I need to schedule...
```

**Numbers:**
```
YOU SAY:          "number forty two items comma number nineteen fifty"
SWICTATION TYPES: 42 items, 1950
```

**Quotes:**
```
YOU SAY:          "she said quote hello world quote exclamation point"
SWICTATION TYPES: She said "Hello world"!
```

### Feature Highlights

✅ **60+ Transformation Rules** including:
- 🎯 Basic & extended punctuation (comma, period, colon, semicolon, dash, ellipsis)
- 📐 Brackets & parentheses (with plural forms: "open parentheses" works!)
- 💬 Smart quotes (stateful toggle, auto-capitalizes first word inside)
- 🔣 Special symbols ($, %, @, &, *, #, /, \, +, =, ×)
- 📝 Abbreviations (mister→Mr., doctor→Dr., etc.)
- 🔢 Number conversion ("number forty two"→42, year patterns "nineteen fifty"→1950)
- 📋 Formatting (new line, new paragraph, tab)
- 🔠 Capitalization modes (caps on/off, all caps [word], capital [letter] [word])
- ✨ Automatic capitalization (I pronoun, sentence starts, after quotes, after titles)

### 📖 Full Documentation

**[→ Complete Secretary Mode Guide](docs/secretary-mode.md)**

Includes:
- Complete command reference (60+ commands)
- Usage examples for letters, emails, notes
- Best practices and tips
- Technical details and architecture
- Troubleshooting guide

---

### Status: **Production Ready** ✅

- ✅ **60+ rules implemented** and tested
- ✅ **27/27 tests passing** with real-world voice samples
- ✅ **MidStream text-transform** integrated (pure Rust)
- ✅ **Automatic spacing** between VAD chunks
- ✅ **Smart capitalization** (titles, sentences, quotes, pronouns)

---

## **System Requirements** 📋

### Hardware
- **GPU:** NVIDIA GPU with 4GB+ VRAM (RTX A1000/3050/4060 or better)
- **RAM:** 8GB+ system RAM
- **CPU:** x86_64 processor

### Software
- **OS:** Linux with X11 or Wayland display server
- **NVIDIA Driver:** 535+ (CUDA 11.8+ compatible)
- **Audio:** PipeWire or PulseAudio
- **Rust:** Latest stable toolchain

### Build Dependencies
```bash
# System packages (Arch/Manjaro) - Choose based on environment
sudo pacman -S xdotool pipewire cuda        # X11
sudo pacman -S wtype pipewire cuda          # Wayland (KDE/Sway/Hyprland)
sudo pacman -S ydotool pipewire cuda        # Wayland (GNOME) or universal

# System packages (Ubuntu/Debian) - Choose based on environment
sudo apt install xdotool pipewire nvidia-cuda-toolkit      # X11 (majority of users)
sudo apt install wtype pipewire nvidia-cuda-toolkit        # Wayland (KDE/Sway/Hyprland)
sudo apt install ydotool pipewire nvidia-cuda-toolkit      # Wayland (GNOME - Ubuntu 24.04 default)

# For ydotool, also grant permissions:
sudo usermod -aG input $USER  # Then log out and log back in

# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

---

## **Display Server Support** 🖥️

Swictation **automatically detects** your display server (X11 or Wayland) and selects the best text injection tool.

### Supported Environments

| Environment | Tool | Installation |
|-------------|------|--------------|
| **X11** (80-90% of users) | xdotool | `sudo apt install xdotool` |
| **Wayland (GNOME)** Ubuntu/Fedora default | ydotool | `sudo apt install ydotool` + permissions |
| **Wayland (KDE/Sway/Hyprland)** | wtype | `sudo apt install wtype` |
| **Universal** (works everywhere) | ydotool | `sudo apt install ydotool` + permissions |

### Quick Setup by Distribution

**Ubuntu 24.04 (GNOME Wayland):**
```bash
sudo apt install ydotool
sudo usermod -aG input $USER
# Log out and log back in
```

**Ubuntu 22.04 / Linux Mint (X11):**
```bash
sudo apt install xdotool
# No permissions needed
```

**Fedora (GNOME Wayland):**
```bash
sudo dnf install ydotool
sudo usermod -aG input $USER
# Log out and log back in
```

**Arch + Sway/Hyprland:**
```bash
sudo pacman -S wtype
# No permissions needed
```

### How It Works

1. **Detects environment** via `XDG_SESSION_TYPE`, `WAYLAND_DISPLAY`, `DISPLAY`
2. **Detects desktop** via `XDG_CURRENT_DESKTOP` (checks for GNOME specifically)
3. **Checks available tools** using `which xdotool/wtype/ydotool`
4. **Selects best tool:**
   - X11 → xdotool (fastest ~10ms)
   - Wayland + GNOME → ydotool (only option, ~50ms)
   - Wayland + others → wtype (fast ~15ms)

**No configuration needed** - it just works!

### For More Information

- **Complete guide:** [docs/display-servers.md](docs/display-servers.md)
- **Tool comparison:** [docs/tool-comparison.md](docs/tool-comparison.md)
- **Installation by distro:** [docs/installation-by-distro.md](docs/installation-by-distro.md)
- **Troubleshooting:** [docs/troubleshooting-display-servers.md](docs/troubleshooting-display-servers.md)
- **Window manager configs:** [docs/window-manager-configs.md](docs/window-manager-configs.md)

---

## **Installation** 📦

### Recommended: npm Package

```bash
npm install -g swictation
```

**What it does:**
- ✅ Installs pre-compiled Rust binaries (no build required)
- ✅ Auto-detects GPU and recommends optimal model (1.1B or 0.6B)
- ✅ Downloads AI models (~1.5GB total)
- ✅ Sets up systemd services automatically
- ✅ Configures Sway/i3 hotkeys
- ✅ Creates config at `~/.config/swictation/config.toml`

### Alternative: Build from Source

```bash
# Clone repository
git clone --recurse-submodules https://github.com/robertelee78/swictation.git
cd swictation/rust-crates

# Build (requires Rust toolchain)
cargo build --release

# Binary location:
# ./target/release/swictation-daemon
```

---

### Why QT Tray for Sway?

The **QT system tray** (`tauri-ui/`) is **optional** and only needed for visual status indicators on Sway/Wayland. The core daemon works perfectly without any GUI.

**Sway Limitation:** Could not get the icon to show properly with pure Tauri app.

### Python Components

The `src/` directory contains **active Python utilities** (not legacy):
- **`ui/swictation_tray.py`** - PySide6/Qt system tray for Sway/Wayland (launches Tauri UI)
  - Works around Qt/Wayland context menu bugs using Telegram Desktop's proven solution
- **`config_loader.py`** - Configuration utilities for TOML parsing
- **`swictation_cli.py`** - CLI tool for daemon control via Unix socket

**Main UI:** Tauri (Rust + React) in `tauri-ui/`
**Sway Tray:** Python PySide6 (for reliable Wayland system tray)
**Core Daemon:** Pure Rust at `rust-crates/target/release/swictation-daemon`

---

## **Usage** 📝

### Daily Workflow

1. **Daemon runs automatically** (systemd service)
2. **Press `$mod+Shift+d`** → Recording starts
3. **Speak naturally with pauses**
4. **Text types automatically** after configured silence duration
5. **Press `$mod+Shift+d`** → Recording stops

### Managing the Daemon

```bash
# Check status
systemctl --user status swictation-daemon

# Start/stop/restart
systemctl --user start swictation-daemon
systemctl --user stop swictation-daemon
systemctl --user restart swictation-daemon

# View logs
journalctl --user -u swictation-daemon -f
```

### Configuration

Edit `~/.config/swictation/config.toml`:

```toml
[vad]
threshold = 0.25           # Empirically optimized for real-time (range: 0.0-1.0)
min_silence_duration = 0.8 # Seconds of silence before transcription
min_speech_duration = 0.25 # Minimum speech length to process

[stt]
model_override = "auto"    # auto, 0.6b-cpu, 0.6b-gpu, or 1.1b-gpu
# Paths configured automatically based on adaptive selection
```

---

## **Troubleshooting** 🔧

### Daemon Won't Start

```bash
# Check logs
journalctl --user -u swictation-daemon -n 50

# Verify binary exists
ls -lh /opt/swictation/rust-crates/target/release/swictation-daemon

# Rebuild if needed
cd /opt/swictation/rust-crates
cargo build --release
```

### No Text Appears

```bash
# Test wtype manually (open text editor first!)
echo "test text" | wtype -

# Verify Wayland
echo $XDG_SESSION_TYPE  # Should be "wayland"
```

### Low Probabilities / No Speech Detection

Check VAD threshold configuration:

```toml
# ~/.config/swictation/config.toml
[vad]
threshold = 0.25  # Default optimized value (lower = more sensitive, higher = better silence detection)
```

📖 **Check logs:** `journalctl --user -u swictation-daemon -f`

---

## **Documentation** 📚

- **[Swictation Architecture](docs/architecture.md)** - Swictation Architecture
- **[Display Server Guide](docs/display-servers.md)** - X11/Wayland technical deep dive
- **[Tool Comparison](docs/tool-comparison.md)** - xdotool vs wtype vs ydotool
- **[Installation by Distribution](docs/installation-by-distro.md)** - Distro-specific setup
- **[Troubleshooting Guide](docs/troubleshooting-display-servers.md)** - Common issues and solutions
- **[ONNX Threshold Guide](rust-crates/swictation-vad/ONNX_THRESHOLD_GUIDE.md)** - VAD tuning details
- **[Tauri UI Architecture](tauri-ui/docs/ARCHITECTURE.md)** - UI system design
- **[MidStream Transform](external/midstream/)** - Voice command library

---

## **Testing & Validation** 🧪

Swictation includes comprehensive testing tools for validating X11/Wayland support:

### Environment Validation

```bash
# Quick environment check
./scripts/validate-x11.sh
```

**Validates:**
- Display server detection (X11/Wayland)
- Tool availability (xdotool/wtype/ydotool)
- Expected tool selection for your environment
- Permission requirements (ydotool input group)

### Diagnostic Information

```bash
# Collect diagnostic data for troubleshooting
./scripts/diagnose.sh
```

**Gathers:**
- Environment variables
- System information
- Tool installations
- Daemon logs
- Process information
- Configuration files

Creates shareable diagnostic report for bug reports.

### Manual Testing Guide

For comprehensive validation of X11/Wayland support:

📖 **[X11 Validation Guide](docs/testing/x11-validation-guide.md)**

Step-by-step testing procedures including:
- Pre-reboot baseline (Wayland)
- X11 session setup
- 8 comprehensive test scenarios
- Performance benchmarking
- Bug reporting procedures

---

## **Advanced Features** 🚀

### Real-time Metrics Broadcasting

Swictation includes a metrics broadcasting system for monitoring and integration:

- **Unix Socket:** `/tmp/swictation_metrics.sock`
- **Protocol:** JSON messages with performance data
- **Metrics:** Audio levels, VAD probabilities, transcription latency
- **Integration:** Connect third-party monitoring tools

### Session Database

Persistent session tracking and analytics:

- **Storage:** SQLite database at `~/.local/share/swictation/metrics.db`
- **Data:** Recording sessions, performance metrics, usage patterns
- **Privacy:** All data stored locally, never transmitted

### Memory Pressure Monitoring

Automatic resource management:

- **VRAM Monitoring:** Real-time GPU memory tracking
- **RAM Monitoring:** System memory usage alerts
- **Thresholds:** Warnings at 80% usage
- **Adaptation:** Performance tuning based on available resources

### Keyboard Shortcut Injection

Beyond text, Swictation can inject keyboard shortcuts:

- **Syntax:** `<KEY:ctrl-c>`, `<KEY:alt-tab>`
- **Use Case:** Voice-controlled application switching
- **Configuration:** Custom shortcuts in config.toml

---

## **Contributing** 🤝

Contributions welcome! Priority areas:

1. **GPU Backend Support** - ROCm for AMD GPUs, DirectML for Windows
2. **Extended Voice Commands** - Expand MidStream transformer library
3. **GUI Status Indicator** - Improve Tauri UI for better visual feedback
4. **Performance Optimization** - Adaptive VAD threshold, streaming improvements
5. **Testing** - Integration tests, CI/CD pipeline

---

## **License** 📄

Apache License 2.0 - See [LICENSE](LICENSE) for details.

---

## **Acknowledgments** 🙏

- **NVIDIA** - Parakeet-TDT models
- **Silero Team** - Silero VAD v6
- **ort Contributors** - Rust ONNX Runtime bindings
- **parakeet-rs** - Parakeet model integration
- **Sway/Wayland Community** - Compositor and tools
- **Rust Community** - Language, ecosystem, and tooling

---

**Status:** Production Ready - Pure Rust Implementation

**Architecture:** Rust daemon + Silero VAD v6 (ONNX) + Parakeet-TDT-1.1B

**Latest Features:**
- ✅ Pure Rust implementation (zero Python runtime)
- ✅ ort 2.0.0-rc.10 with modern CUDA support
- ✅ Silero VAD v6 with ONNX threshold tuning
- ✅ MidStream text transformation (~1µs latency)
