# Swictation

**Voice-to-text dictation for Linux with GPU acceleration**

Pure Rust daemon with VAD-triggered auto-transcription, sub-second latency, and complete privacy (X11/Wayland).

[![Status](https://img.shields.io/badge/status-Production%20Ready-green)](https://github.com/robertelee78/swictation)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue)](LICENSE)
[![Language](https://img.shields.io/badge/language-Rust-orange)](https://www.rust-lang.org/)

---

## Quick Start

### Prerequisites

- **NVIDIA GPU** with 4GB+ VRAM for 0.6B model, 5GB+ for 1.1B model (or CPU fallback)
- **Ubuntu 24.04+** (GLIBC 2.39+ required)
- **Node.js 18+**
- **Text injection tool:**
  - X11: `sudo apt install xdotool`
  - Wayland (GNOME): `sudo apt install ydotool && sudo usermod -aG input $USER` (then logout/login)
  - Wayland (KDE/Sway): `sudo apt install wtype`

### Install

```bash
# One-time npm setup (avoids sudo)
echo "prefix=$HOME/.npm-global" > ~/.npmrc
export PATH="$HOME/.npm-global/bin:$PATH"
echo 'export PATH="$HOME/.npm-global/bin:$PATH"' >> ~/.profile

# Install
npm install -g swictation --foreground-scripts

# Postinstall automatically:
# - Detects GPU and downloads optimized libraries (~1.5GB)
# - Recommends and test-loads AI model (30-60s)
# - Installs systemd services
# - Shows hotkey setup instructions

# Start
swictation start
```

### First Use

1. Open any text editor
2. Press `$mod+Shift+d` (Super+Shift+d)
3. Speak: "Hello world." [pause]
4. Text appears automatically after 0.8s silence
5. Press `$mod+Shift+d` to stop

---

## How It Works

```
[You speak] → VAD detects pause (0.8s) → STT transcribes → Text injected
```

**Components:**
- **VAD:** Silero VAD v6 (ONNX) - detects speech vs silence
- **STT:** Parakeet-TDT-1.1B (5.77% WER) or 0.6B (auto-selected by VRAM)
- **Transform:** MidStream text-transform (Secretary Mode commands)
- **Inject:** xdotool (X11) / wtype / ydotool (Wayland)

**Performance (RTX A1000):**
- VAD: 50ms
- STT: 150-250ms
- Transform: 5µs
- Total: ~1s after you pause speaking

---

## Secretary Mode

Say punctuation and formatting commands naturally:

```
YOU SAY:          "hello comma world period"
SWICTATION TYPES: Hello, world.

YOU SAY:          "number forty two items"
SWICTATION TYPES: 42 items
```

**60+ commands:** punctuation, quotes, brackets, symbols, numbers, formatting, capitalization

📖 **[Full Secretary Mode Guide](docs/secretary-mode.md)** - Complete command reference and examples

---

## Intelligent Corrections

Learn personalized corrections from your editing:

```
YOU SAY:          "arkon"
SWICTATION TYPES: arkon           [you edit to "Archon"]
                  ↓
LEARNED:          "arkon" → "Archon" (fuzzy match, force uppercase)
FOREVER:          All future "arkon" → "Archon" automatically
```

**Features:**
- **Zero-friction learning**: Edit transcription → Click "Learn" → Saved forever
- **Phonetic fuzzy matching**: "arkon" matches "archon", "arkohn", "arckon" (configurable threshold 0.0-1.0, default 0.3)
- **Case intelligence**: Force "API" uppercase, "iPhone" title case, or preserve input case
- **Hot-reload**: No daemon restart needed (file-watched `~/.config/swictation/corrections.toml`)
- **Usage tracking**: See which patterns save you the most time

**Perfect for:**
- Technical jargon (Kubernetes, PostgreSQL, TypeScript)
- Personal names (Archon, Seraphina)
- Domain vocabulary (medical terms, legal phrases)
- Brand names (iPhone, GitHub, OpenAI)

Configure phonetic sensitivity in Settings UI (0.0 = exact only, 1.0 = very fuzzy, default: 0.3).

---

## Usage

### Daemon Control

```bash
# Check status
systemctl --user status swictation-daemon

# Start/stop/restart
systemctl --user {start|stop|restart} swictation-daemon

# View logs
journalctl --user -u swictation-daemon -f
```

### Configuration

Edit `~/.config/swictation/config.toml`:

```toml
[vad]
threshold = 0.25           # 0.0-1.0 (lower = more sensitive)
min_silence_duration = 0.8 # Seconds before transcription
min_speech_duration = 0.25 # Minimum speech length

[stt]
model_override = "auto"    # auto, 0.6b-cpu, 0.6b-gpu, or 1.1b-gpu
```

---

## Troubleshooting

**Daemon won't start:**
```bash
journalctl --user -u swictation-daemon -n 50
```

**No text appears:**
```bash
# Test injection tool
echo "test" | wtype -  # or xdotool type -

# Check display server
echo $XDG_SESSION_TYPE
```

**Low accuracy / no detection:**
- Lower `threshold` in config (try 0.15)
- Check logs for VAD probabilities

📖 **More help:** [docs/troubleshooting-display-servers.md](docs/troubleshooting-display-servers.md)

---

## Architecture

**Pure Rust** - no Python runtime required (optional Python tray for Sway/Wayland, Tauri UI for monitoring)

```
Audio (cpal/PipeWire) → VAD (Silero v6) → STT (Parakeet-TDT) →
Transform (MidStream) → Inject (xdotool/wtype/ydotool)
```

**Crates:**
- `swictation-daemon` - Main binary (tokio async)
- `swictation-audio` - Audio capture
- `swictation-vad` - Voice activity detection
- `swictation-stt` - Speech-to-text
- `swictation-metrics` - Performance tracking
- `swictation-broadcaster` - Real-time metrics
- `external/midstream/text-transform` - Secretary Mode (submodule)

**Audio Configuration:**
- Sample rate: 16kHz mono
- Capture chunks: 1024 samples (~64ms)
- VAD windows: 8000 samples (0.5s)
- Processing: Lock-free circular buffer

📖 **[Architecture Details](docs/architecture.md)**

---

## Advanced

### GPU Support

Auto-detects NVIDIA architecture (Maxwell through Blackwell):

| Generation | GPUs | Package |
|------------|------|---------|
| sm_50-70 | GTX 750-1080, Quadro M/P | ~1.5GB |
| sm_75-86 | RTX 20/30 series, A-series | ~1.5GB |
| sm_89-120 | RTX 40/50 series, H100 | ~1.5GB |

CPU fallback for older/unsupported GPUs.

📖 **[GPU Packages Guide](docs/implementation/gpu-multi-package-guide.md)**

### Metrics API

Real-time monitoring via Unix socket (`/tmp/swictation_metrics.sock`):
- Audio levels, VAD probabilities, transcription latency
- Session database: `~/.local/share/swictation/metrics.db`

### Keyboard Shortcuts

Voice-control keyboard shortcuts:
```
YOU SAY:          "press control c"
SWICTATION TYPES: [sends Ctrl+C keypress]
```

Configure in `config.toml`.

---

## Documentation

- **[Secretary Mode Guide](docs/secretary-mode.md)** - 60+ command reference
- **[Display Server Support](docs/display-servers.md)** - X11/Wayland deep dive
- **[Installation by Distro](docs/installation-by-distro.md)** - Distro-specific setup
- **[Architecture](docs/architecture.md)** - Technical implementation
- **[MidStream Transform](external/midstream/)** - Text transformation library

---

## Contributing

Priority areas:
1. AMD GPU support (ROCm)
2. Extended voice commands (MidStream)
3. Tauri UI improvements
4. Adaptive VAD threshold
5. Testing / CI/CD

---

## License

Apache 2.0 - See [LICENSE](LICENSE)

---

## Acknowledgments

**NVIDIA** (Parakeet-TDT) • **Silero Team** (VAD v6) • **ort Contributors** (Rust ONNX) • **parakeet-rs** • **Rust Community**
