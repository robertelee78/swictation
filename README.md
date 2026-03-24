# Swictation

**Voice-to-text dictation for Linux and macOS with GPU acceleration**

[![Linux](https://img.shields.io/badge/Linux-X11%2FWayland%20%7C%20CUDA-blue?logo=linux)](docs/window-manager-configs.md) [![macOS](https://img.shields.io/badge/macOS-Apple%20Silicon%20%7C%20CoreML-black?logo=apple)](docs/architecture.md)

Pure Rust daemon with VAD-triggered auto-transcription, sub-second latency, and complete privacy.

- **Linux:** X11/Wayland with NVIDIA CUDA acceleration
- **macOS:** Apple Silicon (M1+) with CoreML/Metal acceleration

[![Status](https://img.shields.io/badge/status-Production%20Ready-green)](https://github.com/robertelee78/swictation)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue)](LICENSE)
[![Language](https://img.shields.io/badge/language-Rust-orange)](https://www.rust-lang.org/)

---

## Quick Start

### Prerequisites

#### Linux
- **NVIDIA GPU** with 4GB+ VRAM for 0.6B model, 5GB+ for 1.1B model (or CPU fallback)
- **Ubuntu 24.04+** (GLIBC 2.39+ required)
- **Node.js 18+**
- **Text injection tool:**
  - X11: `sudo apt install xdotool`
  - Wayland (GNOME): `sudo apt install ydotool && sudo usermod -aG input $USER` (then logout/login)
  - Wayland (KDE/Sway): `sudo apt install wtype`

#### macOS
- **Apple Silicon** (M1/M2/M3/M4) - Intel Macs not supported
- **macOS 14 Sonoma** or **macOS 15 Sequoia** (required for CoreML)
- **16GB+ RAM minimum**
- **Node.js 18+**
- **Accessibility permissions** (granted during setup)

### Install

```bash
# Linux (x64)
npm install -g swictation --foreground-scripts

# macOS (Apple Silicon) — uses platform-specific optional package
npm install -g swictation --foreground-scripts

# Postinstall automatically (with retry and progress reporting):
# - Detects platform and GPU, downloads optimized libraries (~1.5GB)
# - Recommends and test-loads AI model (30-60s)
# - Installs services (systemd on Linux, launchd on macOS)
# - Shows platform-specific setup instructions

# Start
swictation start
```

### First Use

1. Open any text editor
2. **Press hotkey to start recording:**
   - **Linux:** `$mod+Shift+d` (Super+Shift+d)
   - **macOS:** `Ctrl+Shift+D`
3. Speak: "Hello world." [pause]
4. Text appears automatically after 0.8s silence
5. **Press hotkey again to stop**

---

## How It Works

```
[You speak] → VAD detects pause (0.8s) → STT transcribes → Text injected
```

**Components:**
- **VAD:** Silero VAD v6 (ONNX) - detects speech vs silence
- **STT:** Parakeet-TDT-1.1B (5.77% WER) or 0.6B (auto-selected by GPU memory)
  - **Linux:** ONNX Runtime with CUDA execution provider
  - **macOS:** Native CoreML via [coreml-native](https://github.com/robertelee78/coreml-native) crate (ANE acceleration)
- **Transform:** MidStream text-transform (Secretary Mode commands)
- **Inject:**
  - **Linux:** xdotool (X11) / wtype / ydotool (Wayland)
  - **macOS:** Accessibility API (native text injection)

**Performance:**
- **Linux (RTX A1000):** VAD 50ms, STT 150-250ms, Total ~1s
- **macOS (M1):** VAD 50ms, STT 150-300ms (CoreML GPU), Total ~1s
- **Audio length:** Unlimited — no 15-second cap. The pipeline processes arbitrary-length dictation via windowed chunking, so long-form dictation works without interruption.

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
vad_threshold = 0.25           # 0.0-1.0 (lower = more sensitive)
vad_min_silence = 0.8          # Seconds before transcription
vad_min_speech = 0.25          # Minimum speech length
stt_model_override = "auto"    # auto, 0.6b-cpu, 0.6b-gpu, or 1.1b-gpu

[hotkeys]
toggle = "Super+Shift+D"       # macOS: Ctrl+Shift+D
push_to_talk = "Super+Space"   # macOS: Ctrl+Space
```

---

## Troubleshooting

**Installation issues:**
Check the install log: `~/.local/share/swictation/install.log`

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
- Lower `vad_threshold` in config (try 0.15)
- Check logs for VAD probabilities

### macOS-Specific

**No text appears (macOS):**
- Grant Accessibility permissions: System Settings > Privacy & Security > Accessibility > Enable Swictation
- Restart the daemon after granting permissions

**Daemon won't start (macOS):**
```bash
# Check launchd logs
log show --predicate 'processImagePath CONTAINS "swictation"' --last 5m
```

**CoreML model not loading:**
- Ensure model files exist in `~/Library/Application Support/swictation/models/`
- Check that `.mlmodelc` directory is present for CoreML inference
- Verify Apple Silicon: `uname -m` should show `arm64`

**Wrong microphone selected:**
- The daemon auto-selects the default input device
- Change default input in System Settings > Sound > Input

📖 **More help:** [docs/window-manager-configs.md](docs/window-manager-configs.md)

---

## Architecture

**Pure Rust** - no Python runtime required

```
Audio (cpal) → VAD (Silero v6) → STT (Parakeet-TDT) →
Transform (MidStream) → Platform-specific text injection
```

**Platform-specific components:**
- **Linux:** PipeWire/ALSA audio, CUDA execution, xdotool/wtype/ydotool injection
- **macOS:** CoreAudio, CoreML execution, Accessibility API injection

**Crates:**
- `swictation-daemon` - Main binary (tokio async)
- `swictation-audio` - Audio capture
- `swictation-vad` - Voice activity detection
- `swictation-stt` - Speech-to-text (ONNX Runtime + native CoreML backends)
- `swictation-metrics` - Performance tracking
- `swictation-broadcaster` - Real-time metrics
- `swictation-paths` - Platform-aware path resolution
- `swictation-context-learning` - Context-aware meta-learning
- `swictation-wasm-utils` - WASM utility bindings
- `external/midstream/text-transform` - Secretary Mode (submodule)
- `coreml-native` - Native CoreML inference for macOS ([external repo](https://github.com/robertelee78/coreml-native))

**Audio Configuration:**
- Sample rate: 16kHz mono
- Capture chunks: 1024 samples (~64ms)
- VAD windows: 512 samples (~32ms)
- Processing: Lock-free circular buffer

📖 **[Architecture Details](docs/architecture.md)**

---

## Advanced

### GPU Support

#### Linux (NVIDIA CUDA)
Auto-detects NVIDIA architecture (Maxwell through Blackwell):

| Generation | GPUs | Package |
|------------|------|---------|
| sm_50-70 | GTX 750-1080, Quadro M/P | ~1.5GB |
| sm_75-86 | RTX 20/30 series, A-series | ~1.5GB |
| sm_89-120 | RTX 40/50 series, H100 | ~1.5GB |

CPU fallback for older/unsupported GPUs.

#### macOS (CoreML)
Auto-detects Apple Silicon and unified memory:

| Mac | Total RAM | GPU Share | Model |
|-----|-----------|-----------|-------|
| M1 (8GB) | 8GB | ~2.8GB | CPU fallback |
| M1 (16GB) | 16GB | ~5.6GB | 0.6B GPU |
| M1 Pro/Max | 32GB+ | ~11GB+ | 1.1B GPU |

Uses CoreML execution provider with FP16 models for optimal performance.

📖 **[Architecture Details](docs/architecture.md)** (includes GPU model selection)

### Metrics API

Real-time monitoring via Unix socket (platform-specific path, see `swictation-paths` crate):
- macOS: `~/Library/Application Support/swictation/swictation_metrics.sock`
- Linux: `$XDG_RUNTIME_DIR/swictation_metrics.sock` (fallback: `~/.local/share/swictation/swictation_metrics.sock`)
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
- **[Window Manager Configs](docs/window-manager-configs.md)** - X11/Wayland setup
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
