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
- **NVIDIA GPU** with 4GB+ VRAM for 0.6B model, 6GB+ for 1.1B model (or CPU fallback)
- **Ubuntu 24.04+** (GLIBC 2.39+ required)
- **Node.js 18+**
- **Text injection tool:**
  - X11: `sudo apt install xdotool`
  - Wayland (GNOME): `sudo apt install ydotool && sudo usermod -aG input $USER` (then logout/login)
  - Wayland (KDE/Sway): `sudo apt install wtype`
- **Sway / Hyprland / River only:** the tray falls back to a Python Qt app on these
  wlroots compositors (they render Qt/PySide6 tray icons more reliably than the Tauri
  tray). Install it with `pip3 install -r requirements-qt-tray.txt` (PySide6 6.8+), or
  from your distro: `apt install python3-pyside6` / `dnf install python3-pyside6` /
  `pacman -S pyside6`. Everything else — daemon, VAD, STT, injection — stays pure Rust,
  and GNOME/KDE/X11 users need no Python at all.

#### macOS
- **Apple Silicon** (M1 or later) - Intel Macs not supported
- **macOS 14 Sonoma** or **macOS 15 Sequoia** (required for CoreML)
- **16GB+ unified memory** — this is a hard requirement, not a recommendation.
  Postinstall aborts on Macs reporting less (the CoreML models need the headroom).
- **Node.js 18+**
- **Accessibility permissions** (granted during setup)

📖 **[Full macOS Setup Guide](docs/macos-setup.md)** - Step-by-step install, permissions, and verification

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

If postinstall was interrupted or you skipped it, the same work is reachable manually:

```bash
swictation doctor            # what, if anything, is missing or broken
swictation setup --repair    # run only the steps that are not healthy
swictation start
```

`swictation setup` with no flags runs the whole install (config, GPU libraries, models,
services) and is still fine to use; `--repair` is the faster path once something is
merely broken rather than absent. See [Troubleshooting](#troubleshooting).

**Installs are tamper-evident.** Every model file is pinned to an immutable upstream
revision and verified against a SHA-256 recorded in `models.manifest.json` before it
takes its final name, so a truncated transfer, a proxy-injected error page, or an
upstream force-push fails the download instead of surfacing later as an opaque daemon
crash. A file that fails verification is never left in place: it is re-fetched on the
next run, and `swictation doctor --deep` re-checks the whole tree on demand (ADR-036).

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
- **STT:** Parakeet-TDT-1.1B (1.39% WER LibriSpeech test-clean) or 0.6B (1.93%) — auto-selected by GPU memory
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
- **Short utterances (ONNX path):** every chunk used to be padded out to 10,000 mel
  frames (100 s) before encoding, so a 3-second phrase ran the encoder over ~97%
  synthetic frames. Encoding now uses the true frame count, which removes that wasted
  work and the filler words the padding sometimes produced (ADR-035).
- **Stopping:** the utterance in flight when you press the hotkey is drained and
  transcribed exactly once, rather than discarded (ADR-035).

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
- **Hot-reload**: No daemon restart needed (file-watched `corrections.toml` in the config directory — `~/.config/swictation/` on Linux, `~/Library/Application Support/swictation/` on macOS)
- **Usage tracking**: See which patterns save you the most time

**Perfect for:**
- Technical jargon (Kubernetes, PostgreSQL, TypeScript)
- Personal names (Archon, Seraphina)
- Domain vocabulary (medical terms, legal phrases)
- Brand names (iPhone, GitHub, OpenAI)

Configure phonetic sensitivity in Settings UI (0.0 = exact only, 1.0 = very fuzzy, default: 0.3).

---

## Usage

### CLI

```bash
swictation doctor            # Check every install step, print a repair command per failure
swictation download-models   # Download AI models (alias: download-model)
swictation setup             # Run the install steps (config, models, services)
swictation start [--ui]      # Start the daemon (and optionally the tray UI)
swictation stop              # Stop the daemon
swictation status            # Service, socket, and platform status
swictation toggle            # Toggle recording on/off (what the hotkey calls)
swictation help              # Full usage
swictation --version         # Version info for every component
```

`doctor` and `setup` are the install-repair pair and are covered in
[Troubleshooting](#troubleshooting). `status` answers a different question — whether the
daemon is running right now — and needs a working install to be meaningful.

**`download-models`** fetches the Silero VAD model plus one or more STT models into the
platform data directory. With no arguments it downloads the platform default set. Pass
`--model=` (or a bare positional, e.g. `swictation download-model 1.1b-gpu`) to pick one,
and `--force` to re-download over existing files.

| Platform | `--model=` values | Size |
|----------|-------------------|------|
| Linux | `0.6b` (aliases `0.6b-gpu`, `0.6b-cpu`, `cpu-only`) | 2.55 GB |
| Linux | `1.1b` (alias `1.1b-gpu`) | 6.96 GB |
| Linux | `both` — default: VAD + 0.6B + 1.1B | ~9.5 GB |
| macOS | `1.1b-coreml` (alias `coreml-native`) | 1.9 GB |
| macOS | `0.6b-coreml` | 2.67 GB |
| macOS | `both` — default: VAD + CoreML 1.1B | 1.9 GB |

CoreML bundles are macOS-only and are rejected on Linux. Silero VAD (629 KB) is always
included.

### Daemon Control

**Linux (systemd):**
```bash
systemctl --user status swictation-daemon
systemctl --user {start|stop|restart} swictation-daemon
journalctl --user -u swictation-daemon -f
```

**macOS (launchd):**
```bash
launchctl list | grep com.swictation.daemon                          # status
launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/com.swictation.daemon.plist
launchctl start com.swictation.daemon
launchctl stop  com.swictation.daemon
launchctl bootout gui/$(id -u)/com.swictation.daemon                 # unload
tail -f ~/Library/Logs/swictation/daemon.log                         # logs
```

`swictation start` / `stop` / `status` wrap the right one for your platform, so prefer
those unless you are debugging the service itself.

### Configuration

Edit the config file — **Linux:** `~/.config/swictation/config.toml`,
**macOS:** `~/Library/Application Support/swictation/config.toml`:

```toml
vad_threshold = 0.25           # 0.0-1.0 (lower = more sensitive)
vad_min_silence = 0.8          # Seconds before transcription
vad_min_speech = 0.25          # Minimum speech length
vad_max_speech = 30.0          # Force transcription after this many seconds
stt_model_override = "auto"    # auto, 0.6b-cpu, 0.6b-gpu, 1.1b-gpu
                               # macOS also: 1.1b-coreml (alias coreml-native)
num_threads = 4                # ONNX Runtime thread count
phonetic_threshold = 0.3       # Learned-correction fuzziness (0.0-1.0)

[hotkeys]
toggle = "Super+Shift+D"       # macOS: Ctrl+Shift+D
push_to_talk = "Super+Space"   # macOS: Ctrl+Space
```

Every key is optional — each has a compiled-in default, so a partial or empty config
file is valid and unset keys fall back to the values above (ADR-034).

---

## Troubleshooting

### Start here: `swictation doctor`

Whatever the symptom, run this first. `doctor` evaluates every install step against what
is actually on disk and prints one line per step, with a repair command under each
failure. It executes no install work and writes nothing, so it is safe to run on an
install that is mid-failure — and it works even when the binaries themselves are missing.

```bash
swictation doctor          # health table for every install step
swictation doctor --deep   # ...and verify file contents by hash, not just size
swictation doctor --json   # machine-readable report (schemaVersion 1)
```

Exit codes: **0** nothing unhealthy, **1** at least one step unhealthy or blocked,
**2** doctor itself failed to run. The last one is deliberately distinct so a crashed
diagnostic is never mistaken for a broken install.

A normal run compares sizes. `--deep` streams a SHA-256 of every model file, GPU library
and platform binary and compares it against what was recorded when they were installed —
minutes of I/O on a multi-gigabyte model tree, which is why it is opt-in rather than the
default. Use it when a file is the right size but the daemon still misbehaves.

### Then fix it: `swictation setup --repair`

`--repair` re-runs only the steps whose check is currently failing, so a mangled service
unit no longer costs you a 9 GB model re-download — and reinstalling the whole package
stops being the first thing to reach for. Every step derives its state from disk and is
idempotent, so running one twice is harmless.

```bash
swictation setup --repair     # run only the steps that are not healthy
swictation setup --list       # list the install steps, their ids and what gates them
swictation setup --<id>       # run exactly one step, e.g. --services
```

Step ids: `platform`, `binaries`, `config-reset`, `gpu-libs`, `models`, `config-heal`,
`services`, `integration`, `verify`. (`cleanup` exists but runs only during npm install;
naming it here is rejected rather than silently ignored, as is any unrecognized flag.)
`--repair`, `--list` and `--<id>` are non-interactive — they never stop to ask about
auto-start — so they are safe to script.

### Where things live

| | Linux | macOS |
|---|---|---|
| Config | `~/.config/swictation/` | `~/Library/Application Support/swictation/` |
| Data + models | `~/.local/share/swictation/` | `~/Library/Application Support/swictation/` |
| Install log | `~/.local/share/swictation/install.log` | `~/Library/Logs/swictation/install.log` |
| Daemon logs | `journalctl --user -u swictation-daemon` | `~/Library/Logs/swictation/daemon.log`, `daemon-error.log` |
| Sockets | `$XDG_RUNTIME_DIR` (fallback: data dir) | `~/Library/Application Support/swictation/` |

`swictation status` prints the resolved socket paths for your machine.

**Installation issues:** run `swictation doctor` — its header prints the resolved install
log path for your machine, along with the platform, target user, and selected model it
resolved.

**Daemon won't start:**
```bash
# Linux
journalctl --user -u swictation-daemon -n 50

# macOS
tail -n 50 ~/Library/Logs/swictation/daemon-error.log
tail -n 20 ~/Library/Logs/swictation/launcher.log   # library/binary resolution
log show --predicate 'processImagePath CONTAINS "swictation"' --last 5m
```

**No text appears:**
```bash
# Linux — test the injection tool and check the display server
echo "test" | wtype -  # or xdotool type -
echo $XDG_SESSION_TYPE # x11 or wayland; picks which tool is used
```

On macOS text injection uses the Accessibility API, so there is no injection tool or
display-server variable to check. Instead grant permissions: System Settings > Privacy &
Security > Accessibility > enable Swictation, then restart the daemon.

**Low accuracy / no detection:**
- Lower `vad_threshold` in config (try 0.15)
- Check logs for VAD probabilities

### macOS-Specific

**CoreML model not loading:**
- Ensure model files exist in `~/Library/Application Support/swictation/models/`
- Check that `.mlmodelc` directory is present for CoreML inference
- Verify Apple Silicon: `uname -m` should show `arm64`
- Re-fetch with `swictation download-models --model=1.1b-coreml --force`

**Wrong microphone selected:**
- The daemon auto-selects the default input device
- Change default input in System Settings > Sound > Input

📖 **More help:** [macOS Setup Guide](docs/macos-setup.md) · [Window Manager Configs](docs/window-manager-configs.md)

---

## Architecture

**Pure Rust core** - no Python in the transcription pipeline. The sole exception is the
optional tray icon on Linux wlroots compositors (Sway/Hyprland/River), which uses a
Python/Qt fallback; see Prerequisites. Dictation works without it.

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
Auto-detects Apple Silicon and unified memory, then reports the 35% GPU share:

| Mac | Unified memory | GPU share | Model |
|-----|----------------|-----------|-------|
| Any Apple Silicon | under ~16GB | — | Install refused |
| M1 (16GB) | 16GB | ~5.6GB | 1.1B CoreML |
| M1 Pro/Max and later | 32GB+ | ~11GB+ | 1.1B CoreML |

Every supported Mac runs the same native CoreML 1.1B bundle (1.9GB) with full Apple
Neural Engine acceleration — there is no per-machine model downgrade, because machines
too small for it are rejected at install time.

📖 **[Architecture Details](docs/architecture.md)** (includes GPU model selection)

### Metrics API

Real-time monitoring via Unix socket (platform-specific path, see `swictation-paths` crate):
- macOS: `~/Library/Application Support/swictation/swictation_metrics.sock`
- Linux: `$XDG_RUNTIME_DIR/swictation_metrics.sock` (fallback: `~/.local/share/swictation/swictation_metrics.sock`)
- Audio levels, VAD probabilities, transcription latency
- Session database: `metrics.db` in the data directory — macOS: `~/Library/Application Support/swictation/metrics.db`, Linux: `~/.local/share/swictation/metrics.db`

### Keyboard Shortcuts

Voice-control keyboard shortcuts:
```
YOU SAY:          "press control c"
SWICTATION TYPES: [sends Ctrl+C keypress]
```

Configure in `config.toml`.

---

## Uninstall

```bash
# 1. Stop the services FIRST (see note below)
swictation stop

# Linux
systemctl --user disable swictation-daemon

# macOS
launchctl bootout gui/$(id -u)/com.swictation.daemon

# 2. Remove the package
npm uninstall -g swictation
```

**Stop the services first — npm will not do it for you.** The package ships a
`preuninstall.js` cleanup script, but npm 7 and later do not execute uninstall lifecycle
scripts at all, so on any modern npm it never runs and you are left with a service unit
pointing at deleted binaries (ADR-034). You can still invoke it by hand from the package
directory before removing it: `node preuninstall.js --force`.

**User data and models are deliberately preserved.** Uninstalling never deletes your
config, learned corrections, metrics database, or the downloaded models (up to ~9.5GB on
Linux). Remove them explicitly when you actually want them gone:

```bash
# Linux
rm -rf ~/.config/swictation ~/.local/share/swictation

# macOS
rm -rf ~/Library/Application\ Support/swictation ~/Library/Logs/swictation
```

---

## Documentation

- **[macOS Setup Guide](docs/macos-setup.md)** - Install, permissions, verification
- **[Secretary Mode Guide](docs/secretary-mode.md)** - 60+ command reference
- **[Window Manager Configs](docs/window-manager-configs.md)** - X11/Wayland setup
- **[Architecture](docs/architecture.md)** - Technical implementation
- **[Architecture Decision Records](docs/adr/)** - Why the system works the way it does
- **[MidStream Transform](external/midstream/)** - Text transformation library (submodule)

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
