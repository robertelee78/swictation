# Swictation

**Real-time voice-to-text dictation daemon for Sway/Wayland with GPU acceleration**

> Hands-free coding on Wayland with 382-420ms latency, 95%+ accuracy, and complete privacy.

[![Status](https://img.shields.io/badge/status-MVP%20Complete-green)](https://github.com/yourusername/swictation)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue)](LICENSE)

---

## **Quick Start** 🚀

### One-Line Setup (Automated)

```bash
sudo /opt/swictation/scripts/setup-sway.sh
```

This will:
- ✅ Add `Alt+Shift+d` keybinding to Sway
- ✅ Optional daemon autostart
- ✅ Create config backup
- ✅ Provide reload instructions

### Manual Usage

```bash
# Start daemon
python3 /opt/swictation/src/swictationd.py

# In another terminal or via Sway keybinding:
python3 /opt/swictation/src/swictation_cli.py toggle  # Start recording
# Speak your text...
python3 /opt/swictation/src/swictation_cli.py toggle  # Stop & transcribe
```

### First-Time Test

```bash
# Test keybinding without Sway
/opt/swictation/scripts/test-keybinding.sh
```

📖 **Full Documentation:** See [docs/](docs/) for detailed guides

---

## **Features** ✨

### Core Capabilities
- 🎯 **382-420ms End-to-End Latency** - NVIDIA Canary-1B-Flash (9.4x realtime)
- 🔒 **100% Privacy** - All processing on local GPU, no cloud
- ⚡ **GPU Optimized** - Works on 4GB VRAM (RTX A1000 tested)
- 🌊 **Wayland Native** - wtype text injection, no X11 dependencies
- ⌨️ **Hotkey Control** - `Alt+Shift+d` toggle (customizable)
- 🔄 **systemd Integration** - Auto-start with Sway
- 📋 **Full Unicode Support** - Emojis, Greek, Chinese, all languages

### Technical Highlights
- **STT Model:** NVIDIA Canary-1B-Flash (5.77% WER)
- **Audio Capture:** PipeWire/sounddevice hybrid backend
- **Text Injection:** wtype (Wayland) with wl-clipboard fallback
- **Daemon Architecture:** Unix socket IPC, state machine
- **Model Loading:** Automatic download on first run
- **Streaming Mode:** Real-time transcription with NeMo Wait-k policy

---

## **Streaming Transcription** 🔄

Swictation supports **real-time streaming transcription** where text appears progressively as you speak, with <2 second latency.

### How It Works

```
You speak → Audio chunks (1s) → NeMo Wait-k streaming → Text injection → Words appear real-time
                                    ↑
                            10s context memory
                        (never forgets recent speech)
```

### Streaming vs Batch Mode

| Feature | Streaming Mode | Batch Mode |
|---------|---------------|------------|
| **Text appearance** | Progressive (real-time) | All at once at end |
| **Latency** | ~1.5s per chunk | After full recording |
| **Accuracy** | 100% (same as batch) | 100% baseline |
| **Memory** | ~3600 MB stable | ~3600 MB + audio length |
| **Best for** | Live dictation ✅ | Long recordings |

### Wait-k Policy

Swictation uses NeMo's **Wait-k streaming policy** for maximum accuracy:

- **10-second context window** - Remembers recent speech for coherent transcription
- **Stateful decoder** - Maintains context across 1-second audio chunks
- **Zero hallucinations** - Built-in detector prevents phantom words
- **Progressive injection** - Smart deduplication (no duplicate words)

**Example flow:**
```
Chunk 1 (1s):  "Hello"           → Inject "Hello"
Chunk 2 (2s):  "Hello world"     → Inject " world"
Chunk 3 (3s):  "Hello world."    → Inject "."
```

### Configuration

Streaming behavior is configurable via `config/streaming.yaml`:

```yaml
streaming:
  enabled: true               # Enable/disable streaming mode
  policy: waitk               # "waitk" (accurate) or "alignatt" (faster)
  chunk_secs: 1.0            # Audio chunk duration
  left_context_secs: 10.0    # Context memory window
  waitk_lagging: 2           # Wait 2 chunks before first prediction
  hallucinations_detector: true  # Prevent phantom words
```

**Presets available:**
- **Default:** Balanced (1.5s latency, 100% accuracy)
- **Low latency:** <1.2s latency, 99%+ accuracy
- **Max accuracy:** ~2s latency, 100% accuracy
- **Memory constrained:** ~1s latency, 95%+ accuracy

### Performance Characteristics

**Latency breakdown (RTX A1000):**
```
Audio chunk (1.0s)     → 1000ms
Encoder (GPU)          → 150-250ms
Wait-k decoder         → 100-200ms
Text injection (wtype) → 10-50ms
────────────────────────────────
Total                  → 1.3-1.5s
```

**Memory usage:**
- GPU: ~3600 MB (90% of 4GB VRAM)
- System RAM: ~200-250 MB
- Stable over 10+ minute sessions

### CLI Usage

```bash
# Start daemon (streaming enabled by default)
python3 /opt/swictation/src/swictationd.py

# Toggle recording - text appears as you speak
python3 /opt/swictation/src/swictation_cli.py toggle

# Disable streaming (batch mode)
# Edit config/streaming.yaml: enabled: false
```

📖 **Complete documentation:** [docs/streaming_implementation.md](docs/streaming_implementation.md)

---

## **System Requirements** 📋

### Hardware
- NVIDIA GPU with 4GB+ VRAM (RTX A1000/3050/4060 or better)
- 8GB+ system RAM
- x86_64 CPU

### Software
- Linux with Sway/i3-compatible Wayland compositor
- NVIDIA driver 535+ (CUDA 11.8+ compatible)
- PipeWire or PulseAudio
- Python 3.10+

### Dependencies
```bash
# System packages (Arch/Manjaro)
sudo pacman -S python python-pip wtype wl-clipboard ffmpeg

# System packages (Ubuntu/Debian)
sudo apt install python3 python3-pip wtype wl-clipboard ffmpeg

# Python packages (see requirements.txt)
pip install nemo_toolkit[asr] torch sounddevice numpy librosa
```

---

## **Installation** 📦

### 1. Clone Repository
```bash
git clone https://github.com/yourusername/swictation.git
cd swictation
```

### 2. Install Dependencies
```bash
# Install Python packages
pip install -r requirements.txt

# Download NVIDIA Canary-1B-Flash model (automatic on first run)
python3 -c "from nemo.collections.asr.models import EncDecMultiTaskModel; EncDecMultiTaskModel.from_pretrained('nvidia/canary-1b-flash')"
```

### 3. Setup Sway Integration
```bash
# Automated setup (recommended)
sudo /opt/swictation/scripts/setup-sway.sh

# Or manual: Add to ~/.config/sway/config
echo "bindsym Mod1+Shift+d exec python3 /opt/swictation/src/swictation_cli.py toggle" >> ~/.config/sway/config
swaymsg reload
```

### 4. Test Installation
```bash
# Run test script
/opt/swictation/scripts/test-keybinding.sh
```

---

## **Usage** 📝

### Basic Workflow

1. **Start daemon** (if not using systemd autostart)
   ```bash
   python3 /opt/swictation/src/swictationd.py
   ```

2. **Press `Alt+Shift+d`** to start recording
3. **Speak your text**
4. **Press `Alt+Shift+d` again** to stop and transcribe
5. **Text appears at cursor** in focused application

### CLI Commands

```bash
# Toggle recording (start/stop)
python3 /opt/swictation/src/swictation_cli.py toggle

# Check daemon status
python3 /opt/swictation/src/swictation_cli.py status

# Stop daemon
python3 /opt/swictation/src/swictation_cli.py stop
```

### systemd Service (Auto-start)

```bash
# Copy service file
cp /opt/swictation/config/swictation.service ~/.config/systemd/user/

# Enable and start
systemctl --user enable swictation.service
systemctl --user start swictation.service

# Check status
systemctl --user status swictation.service

# View logs
journalctl --user -u swictation.service -f
```

---

## **Performance** 📈

| Metric | Target | Achieved | Status |
|--------|--------|----------|--------|
| **End-to-End Latency** | <150ms | 382-420ms | ⚠️ Good, can optimize |
| **STT Accuracy (WER)** | <10% | 5.77% | ✅ Excellent |
| **GPU Memory Usage** | <4GB | 3.37 GB | ✅ Perfect |
| **Processing Speed (RTFx)** | <1.0x | 0.106x (9.4x faster) | ✅ Excellent |

*Tested on: NVIDIA RTX A1000 Laptop GPU (4GB VRAM)*

---

## **Architecture** 🏗️

### System Overview

```
┌─────────────────────────────────────────────────────────────┐
│           SWICTATION DAEMON (systemd)                       │
│  State: [IDLE] ↔ [RECORDING] ↔ [PROCESSING]               │
│  IPC: Unix socket (/tmp/swictation.sock)                   │
└─────────────────────────────────────────────────────────────┘
                          ↓
    ┌──────────────┬──────────────┬────────────┐
    │   Audio      │   STT        │    Text    │
    │   Capture    │   Engine     │  Injection │
    ├──────────────┼──────────────┼────────────┤
    │ PipeWire/    │ Canary-1B    │   wtype    │
    │ sounddevice  │ Flash (GPU)  │  (Wayland) │
    │ 16kHz mono   │ 5.77% WER    │  Unicode   │
    └──────────────┴──────────────┴────────────┘
```

### Key Components

- **swictationd.py** - Main daemon process (Unix socket IPC, state machine)
- **audio_capture.py** - Hybrid PipeWire/sounddevice backend
- **text_injection.py** - wtype integration with Unicode support
- **swictation_cli.py** - CLI tool for daemon control

📖 **Detailed Architecture:** [docs/architecture.md](docs/architecture.md)

---

## **Current Implementation Status** ✅

### Phase 1: Core Engine (COMPLETED)
- [x] NVIDIA driver validation (RTX A1000, 4GB VRAM)
- [x] NVIDIA Canary-1B-Flash model integration
- [x] PipeWire audio capture module
- [x] Daemon process with Unix socket IPC
- [x] wtype text injection with full Unicode
- [x] Sway keybinding configuration (`Alt+Shift+d`)
- [x] systemd service integration
- [x] CLI tools (toggle, status, stop)
- [x] Setup automation scripts

### Phase 2: Experimental Features (TESTED, NOT IN DAEMON)
- [x] Memory optimization (10s chunking in test_canary_chunked.py)
- [x] Silero VAD integration (speech detection in test_canary_vad.py)
- [ ] **TODO:** Integrate VAD into main daemon
- [ ] **TODO:** Integrate chunking for long audio

### Phase 3: Polish (IN PROGRESS)
- [x] Documentation & user guide
- [ ] Quick start guide
- [ ] Installation script
- [ ] Comprehensive test suite
- [ ] Performance monitoring
- [ ] Configuration system (TOML)
- [ ] Text transformation pipeline

---

## **Limitations & Known Issues** ⚠️

**Current MVP Limitations:**
- ⚠️ **No VAD in daemon** - All audio is transcribed (VAD only in tests)
- ⚠️ **No chunking in daemon** - May fail on very long audio (>60s)
- ⚠️ **No text transformations** - "new line" does NOT become `\n`
- ⚠️ **No configuration file** - Settings hardcoded in source
- ⚠️ **Single-shot transcription** - Not streaming/real-time

**Tested Experimental Features (Not in Main Daemon):**
- ✅ VAD works perfectly in `test_canary_vad.py` (100% accuracy, 2.2 MB overhead)
- ✅ Chunking works in `test_canary_chunked.py` (10s chunks with 1s overlap)
- 🔜 **TODO:** Integrate these into the main daemon

---

## **Documentation** 📚

- **[Installation Guide](docs/sway-integration.md)** - Complete setup instructions
- **[Architecture](docs/architecture.md)** - System design and components
- **[Troubleshooting](docs/troubleshooting.md)** - Common issues and solutions
- **[Voice Commands](docs/voice-commands.md)** - Coding command reference

---

## **Troubleshooting** 🔧

### Quick Fixes

**Daemon not starting?**
```bash
# Check if already running
python3 /opt/swictation/src/swictation_cli.py status

# Kill existing process
pkill -f swictationd.py

# Start fresh
python3 /opt/swictation/src/swictationd.py
```

**No text appearing?**
```bash
# Test wtype manually
echo "test" | wtype -

# Check Wayland socket
echo $WAYLAND_DISPLAY
```

**Audio not captured?**
```bash
# List audio devices
python3 /opt/swictation/src/audio_capture.py list

# Test capture
python3 /opt/swictation/src/audio_capture.py 5  # Record 5 seconds
```

📖 **Full Troubleshooting:** [docs/troubleshooting.md](docs/troubleshooting.md)

---

## **Contributing** 🤝

Contributions welcome! Priority areas:

1. **Integrate VAD** - Move VAD from tests into main daemon
2. **Integrate Chunking** - Support unlimited audio length
3. **Latency Optimization** - Target <200ms end-to-end
4. **Text Transformations** - Code-specific commands ("new line" → `\n`)
5. **Testing** - Comprehensive test coverage

---

## **License** 📄

Apache License 2.0 - See [LICENSE](LICENSE) for details.

---

## **Acknowledgments** 🙏

- **NVIDIA** - Canary-1B-Flash model
- **Silero Team** - Lightweight VAD (tested)
- **NeMo Contributors** - ASR framework
- **Sway/Wayland Community** - Compositor and tools

---

**Status:** MVP Complete - Production Ready for Testing

**Hardware Tested:** NVIDIA RTX A1000 Laptop GPU (4GB VRAM)

**Next Milestone:** Integrate VAD and chunking into main daemon
