# Swictation

**Real-time voice-to-text dictation daemon for Sway/Wayland with GPU acceleration**

> Hands-free coding on Wayland with VAD-triggered auto-transcription, sub-second latency, 95%+ accuracy, and complete privacy.

[![Status](https://img.shields.io/badge/status-Production%20Ready-green)](https://github.com/robertelee78/swictation)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue)](LICENSE)

---

## **Quick Start** 🚀

### Prerequisites

**Before you begin, ensure you have:**
- ✅ NVIDIA GPU with 4GB+ VRAM (RTX A1000/3050/4060 or better)
  - **FP16 Optimization**: Uses only ~2.2GB VRAM typical, ~3.5GB peak
  - Works on budget GPUs: GTX 1050 Ti (4GB), RTX 3050 Mobile (4GB)
- ✅ Linux with Sway/Wayland compositor
- ✅ Python 3.10+, wtype, wl-clipboard, ffmpeg installed
- ✅ Project cloned to `/opt/swictation`

```bash
# Install system dependencies

# Arch/Manjaro:
sudo pacman -S python python-pip wtype wl-clipboard ffmpeg

# Ubuntu/Debian/Pop!_OS:
sudo apt install python3 python3-pip wtype wl-clipboard ffmpeg

# Fedora:
sudo dnf install python3 python3-pip wtype wl-clipboard ffmpeg

# Install Python packages (use the Python version that matches your installed packages)
pip3 install --break-system-packages -r requirements.txt
```

### Complete Installation

**IMPORTANT:** The Quick Start above is simplified. For a fresh install, you need additional steps:

1. **Clone with submodules** (or run `git submodule update --init --recursive`)
2. **Install Rust toolchain** (required to build text transformer)
3. **Build and install midstream transformer** (provides voice command → symbol conversion)
4. **Install Python dependencies**
5. **Setup systemd service and Sway keybinding**

**See [docs/install.md](docs/install.md) for complete step-by-step instructions.**

### Quick Setup (After Following Complete Installation)

```bash
cd /opt/swictation

# 1. Install systemd service (auto-starts with Sway)
./scripts/install-systemd-service.sh

# 2. Add Sway keybinding ($mod+Shift+d)
./scripts/setup-sway.sh

# 3. Reload Sway to apply changes
swaymsg reload
```

**What this does:**
- ✅ Daemon auto-starts when you log into Sway
- ✅ Adds `$mod+Shift+d` hotkey for toggle (respects your modifier key)
- ✅ Backs up your Sway config before changes
- ✅ Enables VAD-triggered auto-transcription

### Your First Recording

**1. Open any text editor** (kate, gedit, VSCode, vim, etc.)

**2. Press `$mod+Shift+d`** to start recording (you'll see daemon activity in logs)

**3. Speak naturally:**
```
YOU SAY:  "Hello world." [brief pause]
RESULT:   Hello world.  ← Text appears!

YOU SAY:  "This is a test." [brief pause]
RESULT:   This is a test.  ← More text appears!
```

**4. Press `$mod+Shift+d`** again to stop recording

**Expected behavior:**
- 🎤 Recording starts immediately (no visible indicator yet)
- ⏸️ After configured silence duration, VAD detects pause → transcription happens
- ⌨️ Text types into your focused window automatically
- 🛑 Second hotkey press stops recording

### Manual Testing (Without Sway)

```bash
# Terminal 1: Start daemon (watch output)
python3 /opt/swictation/src/swictationd.py

# Terminal 2: Toggle recording
python3 /opt/swictation/src/swictation_cli.py toggle

# Speak a sentence, pause briefly, watch terminal for transcription
# Example: "The quick brown fox." [brief pause] → See output in daemon logs

# Stop recording
python3 /opt/swictation/src/swictation_cli.py toggle
```

---

## **Voice Commands & Text Transformation** 🎤⚡

**NEW:** Swictation now includes **MidStream PyO3 text transformation** - blazingly fast voice command to symbol conversion!

### How It Works
```
┌─────────────────────────────────────────────────┐
│  YOU SPEAK → STT → Transform → Text Injection  │
│  "comma"  →  "comma"  →  ","  →  types ","      │
└─────────────────────────────────────────────────┘
```

**Performance:** ~1µs per transformation (296,677x faster than subprocess!)

### Punctuation
```
YOU SAY:              "Hello comma world period"
SWICTATION TYPES:     Hello, world.
```

### Symbols
```
YOU SAY:              "x equals open bracket one comma two comma three close bracket"
SWICTATION TYPES:     x = [1, 2, 3]
```

### Code Example
```
YOU SAY:              "def hello underscore world open parenthesis close parenthesis colon"
SWICTATION TYPES:     def hello_world():
```

⚡ **Performance:** 266 transformation rules loaded, ~0.3μs latency via Rust/PyO3
📖 **Technical Details:** See [docs/architecture.md](docs/architecture.md) for system design

---

## **Common Use Cases** 💡

### 1. Writing Documentation
```
Press $mod+Shift+d
"This function calculates the factorial period" [brief pause]
"It takes an integer as input period" [brief pause]
"Returns the factorial result period" [brief pause]
Press $mod+Shift+d

Result:
This function calculates the factorial. It takes an integer as input. Returns the factorial result.
```

### 2. Code Comments
```
"Hash comment TODO colon implement error handling" [brief pause]

Result:
# TODO: implement error handling
```

### 3. Quick Notes
```
"Meeting notes colon" [brief pause]
"Discussed authentication refactor period" [brief pause]
"Action items colon migrate to JWT tokens period" [brief pause]

Result:
Meeting notes: Discussed authentication refactor. Action items: migrate to JWT tokens.
```

### 4. Git Commits
```
"git commit hyphen m quote fix authentication bug quote" [brief pause]

Result:
git commit -m "fix authentication bug"
```

---

## **How It Works** ⚙️

### VAD-Triggered Workflow

```
┌─────────────────────────────────────────────────┐
│  1. Press $mod+Shift+d → Recording starts       │
│  2. Speak: "Hello world."                       │
│  3. Brief pause (silence)                       │
│  4. VAD detects pause → STT transcribes         │
│  5. Text injected: "Hello world. "              │
│  6. Speak: "Testing one two three."             │
│  7. Brief pause (silence)                       │
│  8. VAD detects pause → STT transcribes         │
│  9. Text injected: "Testing one two three. "    │
│ 10. Press $mod+Shift+d → Recording stops        │
└─────────────────────────────────────────────────┘
```

**Key Insight:** You don't toggle between each sentence! Just speak naturally with pauses, and text appears automatically after configured silence duration.

📖 **Full Documentation:** See [docs/](docs/) for architecture, troubleshooting, and advanced usage

---

## **Features** ✨

### Core Capabilities
- 🎙️ **VAD-Triggered Segmentation** - Auto-transcribe on natural pauses (configurable)
- 🎯 **Sub-Second Latency** - Real-time text injection with full segment accuracy
- 🔒 **100% Privacy** - All processing on local GPU, no cloud
- ⚡ **GPU Optimized** - FP16 mixed precision: 1.8GB model + 400MB buffer = ~2.2GB total
- 🌊 **Wayland Native** - wtype text injection, no X11 dependencies
- ⌨️ **Hotkey Control** - `$mod+Shift+d` toggle (user configurable)
- 🔄 **systemd Integration** - Auto-start with Sway
- 📋 **Full Unicode Support** - Emojis, Greek, Chinese, all languages
- 📊 **Performance Metrics** - Track WPM, latency, trends (`swictation stats`)

### Technical Highlights
- **STT Model:** NVIDIA Canary-1B-Flash (5.77% WER)
- **VAD Model:** Silero VAD (2.2 MB GPU overhead)
- **Text Transform:** MidStream PyO3 (~1µs latency, 296,677x faster than subprocess)
- **Audio Capture:** PipeWire/sounddevice hybrid backend
- **Text Injection:** wtype (Wayland) with wl-clipboard fallback
- **Daemon Architecture:** Unix socket IPC, state machine
- **Model Loading:** Automatic download on first run
- **Streaming Mode:** VAD-triggered with automatic segmentation

---

## **VAD-Triggered Segmentation** 🎙️

Swictation uses **Voice Activity Detection (VAD)** to automatically segment and transcribe your speech at natural pauses.

### How It Works

```
[Toggle ON] → Continuous recording starts
    ↓
[You speak] → Audio accumulates in buffer
    ↓
[Brief silence] → VAD detects pause → Transcribe segment → Inject text
    ↓
[You speak] → New segment starts
    ↓
[Toggle OFF] → Stop recording
```

### Benefits

- ✅ **Perfect Accuracy** - Full segment context (no chunk fragmentation)
- ✅ **Natural Workflow** - Speak in complete thoughts
- ✅ **Auto-Segmentation** - No manual toggle per sentence
- ✅ **Real-time Feel** - Text appears after natural pauses
- ✅ **Low Memory** - Only 2.2 MB VAD overhead

### Technical Details

- **VAD Model:** Silero VAD (2.2 MB GPU memory)
- **VAD Window:** 512ms for speech/silence detection
- **Silence Threshold:** Configurable (default: 0.8s) - See `~/.config/swictation/config.toml`
- **Min Segment:** 1 second (filters very short utterances)
- **STT Model:** NVIDIA Canary-1B-Flash (3.6 GB)

**Example:**
```
User: "Hello world." [brief pause] "Testing one two three."

Timeline:
0-1s:     Speak "Hello world." → buffer accumulating
1-1.8s:   Silence detected → transcribe → inject "Hello world. "
2-4s:     Speak "Testing one two three." → buffer accumulating
4-4.8s:   Silence detected → transcribe → inject "Testing one two three. "
```

### Performance Characteristics

**Latency breakdown (RTX A1000, default 0.8s silence threshold):**
```
Speech detection (VAD)  → 50ms
Silence threshold       → 800ms (configurable)
STT transcription       → 150-250ms
Text transformation     → 1µs (negligible!)
Text injection (wtype)  → 10-50ms
────────────────────────────────
Total after pause       → ~1.0s
```

**Memory usage:**
- GPU: ~2.2 GB typical, ~3.5GB peak (FP16 optimized)
  - STT model: ~1.8GB
  - Context buffer (20s): ~400MB
  - VAD model: 2.2 MB
- System RAM: ~200-250 MB
- Stable over 10+ minute sessions

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

### Complete Installation Guide

#### Step 1: Verify System Requirements

```bash
# Check GPU
nvidia-smi  # Should show your NVIDIA GPU with 4GB+ VRAM

# Check Python version
python3 --version  # Should be 3.10 or higher

# Check Wayland session
echo $XDG_SESSION_TYPE  # Should be "wayland"
```

#### Step 2: Install System Dependencies

**Arch/Manjaro:**
```bash
sudo pacman -S python python-pip wtype wl-clipboard ffmpeg cuda
```

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install python3 python3-pip wtype wl-clipboard ffmpeg nvidia-cuda-toolkit
```

#### Step 3: Clone Repository

```bash
# Clone to /opt (recommended location)
sudo mkdir -p /opt
sudo chown $USER:$USER /opt
git clone https://github.com/robertelee78/swictation.git /opt/swictation
cd /opt/swictation
```

#### Step 4: Install Python Dependencies

```bash
# Install packages (this may take 5-10 minutes)
pip install -r requirements.txt

# Download NVIDIA Canary-1B-Flash model (~3.5GB download)
# This happens automatically on first run, or manually:
python3 -c "from nemo.collections.asr.models import EncDecMultiTaskModel; EncDecMultiTaskModel.from_pretrained('nvidia/canary-1b-flash')"
```

**Expected output:**
```
Downloading model checkpoint...
Model nvidia/canary-1b-flash downloaded successfully
```

#### Step 5: Setup Sway Integration

```bash
cd /opt/swictation

# Install systemd service (auto-start with Sway)
./scripts/install-systemd-service.sh

# Add keybinding to Sway config
./scripts/setup-sway.sh

# Apply changes
swaymsg reload
```

**Expected output:**
```
✓ Service installed: ~/.config/systemd/user/swictation.service
✓ Keybinding added to ~/.config/sway/config
✓ Backup created: ~/.config/sway/config.backup
```

#### Step 6: Verify Installation

```bash
# Check daemon is running
systemctl --user status swictation.service

# Should see: Active: active (running)
```

**If not running:**
```bash
# Start manually
systemctl --user start swictation.service

# Enable auto-start
systemctl --user enable swictation.service

# Check logs
journalctl --user -u swictation.service -n 50
```

#### Step 7: Test Recording

1. Open a text editor (kate, gedit, etc.)
2. Press `$mod+Shift+d`
3. Say "hello world" and pause briefly
4. Text should appear!
5. Press `$mod+Shift+d` to stop

**Troubleshooting:** See [Troubleshooting](#troubleshooting-) section below if issues occur.

---

## **Understanding the System** 🧠

### What Happens When You Press $mod+Shift+d?

```
1. Sway keybinding → CLI → Unix socket (/tmp/swictation.sock)

2. Daemon state: IDLE ↔ RECORDING

3. While RECORDING:
   - Audio capture: PipeWire → 16kHz mono stream
   - VAD monitor: Silero VAD watches for configured silence duration
   - On silence detection:
     → Send buffer to Canary-1B-Flash STT
     → Transform text (MidStream PyO3, ~1µs)
     → wtype injects text to focused window
   - Buffer cleared, continue recording

4. When IDLE: Models stay loaded in GPU for instant restart
```

### Memory Layout

```
GPU VRAM (~2.2 GB typical with FP16 mixed precision):
├── Canary-1B-Flash STT model: ~1.8 GB (FP16, 50% reduction from 3.6GB FP32)
├── Context buffer (20s): ~400 MB (left context window for accuracy)
├── Silero VAD model: 2.2 MB (speech/silence detection)
└── Peak usage: ~3.5 GB (well under 4GB limit, safe on RTX A1000)

System RAM (~250 MB):
├── Audio buffer: ~10-30 MB (dynamic)
├── Python process: ~200 MB
└── Daemon overhead: ~20 MB
```

**FP16 Optimization Benefits:**
- 50% VRAM reduction with <0.5% WER accuracy impact
- Enables 20-30s context buffers (vs 10s in FP32)
- Prevents OOM crashes on 4GB GPUs
- Same or better performance (FP16 ops are faster)

### File Structure

```
/opt/swictation/
├── src/
│   ├── swictationd.py           # Main daemon (state machine + IPC + transform)
│   ├── audio_capture.py         # PipeWire audio streaming
│   ├── text_injection.py        # wtype Wayland text injection
│   ├── swictation_cli.py        # CLI tool for daemon control
│   ├── memory_manager.py        # GPU memory protection
│   └── performance_monitor.py   # Performance tracking
├── external/
│   └── midstream/               # MidStream text transformer (Git submodule)
│       └── crates/text-transform/  # PyO3 Rust bindings
├── scripts/
│   ├── install.sh               # Complete installation script
│   ├── install-systemd-service.sh
│   └── setup-sway.sh
├── config/
│   └── swictation.service       # systemd unit file
├── tests/
│   └── test_swictationd_transform.py  # Integration tests
└── docs/
    ├── architecture.md           # System design
    ├── pyo3-integration.md       # Text transformation details
    ├── voice-commands.md         # Voice coding reference
    └── troubleshooting.md        # Detailed troubleshooting
```

---

## **Usage** 📝

### Daily Workflow

**The daemon runs in the background automatically after installation.**

1. **Open any text editor, terminal, or code editor**
2. **Press `$mod+Shift+d`** → Recording starts
3. **Speak naturally with pauses**
4. **Text types automatically** after 2-second pauses
5. **Press `$mod+Shift+d`** → Recording stops

### Example Session: Writing Code

```python
# Open VSCode, focus on editor

[Press $mod+Shift+d to start]

YOU SAY: "def calculate underscore sum open parenthesis numbers close parenthesis colon"
[brief pause] → def calculate_sum(numbers):

YOU SAY: "return sum open parenthesis numbers close parenthesis"
[brief pause] → return sum(numbers)

[Press $mod+Shift+d to stop]

# Result:
# def calculate_sum(numbers):
#     return sum(numbers)
```

### Example Session: Documentation

```markdown
# Open kate or gedit, start typing

[Press $mod+Shift+d]

YOU SAY: "This function processes user input period"
[brief pause] → This function processes user input.

YOU SAY: "It validates the data and returns a cleaned version period"
[brief pause] → It validates the data and returns a cleaned version.

[Press $mod+Shift+d]
```

### Example Session: Terminal Commands

```bash
# Focus on terminal

[Press $mod+Shift+d]

YOU SAY: "git add period"
[brief pause] → git add.

YOU SAY: "git commit hyphen m quote update readme quote"
[brief pause] → git commit -m "update readme"

[Press $mod+Shift+d]

# Then press Enter to execute!
```

### CLI Commands (Advanced)

```bash
# Toggle recording (alternative to hotkey)
python3 /opt/swictation/src/swictation_cli.py toggle

# Check daemon status
python3 /opt/swictation/src/swictation_cli.py status
# Output: Recording: active / idle

# Stop daemon completely
python3 /opt/swictation/src/swictation_cli.py stop

# View performance metrics
python3 /opt/swictation/src/swictation_cli.py stats       # Latest session details
python3 /opt/swictation/src/swictation_cli.py history     # Recent sessions table
python3 /opt/swictation/src/swictation_cli.py summary     # Lifetime statistics
```

### Managing the Daemon

```bash
# Check daemon status
systemctl --user status swictation.service

# Start daemon
systemctl --user start swictation.service

# Stop daemon (save battery)
systemctl --user stop swictation.service

# Restart daemon (after config changes)
systemctl --user restart swictation.service

# View real-time logs
journalctl --user -u swictation.service -f

# View last 50 log lines
journalctl --user -u swictation.service -n 50
```

### Tips for Best Results

**DO:**
- ✅ Speak clearly at normal pace
- ✅ Pause 2+ seconds between thoughts
- ✅ Focus your text editor before speaking
- ✅ Use consistent punctuation ("period", "comma")
- ✅ Test in simple editor first (kate, gedit)

**DON'T:**
- ❌ Speak continuously for 30+ seconds
- ❌ Speak too fast without pauses
- ❌ Forget to say punctuation marks
- ❌ Expect automatic capitalization (not implemented)
- ❌ Switch windows during transcription

---

## **Performance** 📈

| Metric | Value | Status |
|--------|-------|--------|
| **VAD Latency** | <50ms | ✅ Excellent |
| **Segment Transcription** | <1s | ✅ Good |
| **STT Accuracy (WER)** | 5.77% | ✅ Excellent |
| **GPU Memory** | 3.6 GB (VAD: 2.2 MB, STT: 3.6 GB) | ✅ Perfect |
| **Processing Speed** | 0.106x RTF (9.4x realtime) | ✅ Excellent |

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
- [x] NVIDIA Canary-1B-Flash model integration
- [x] Silero VAD integration (automatic speech segmentation)
- [x] VAD-triggered transcription (auto-transcribe on configurable silence)
- [x] PipeWire audio capture with streaming callbacks
- [x] wtype text injection with full Unicode
- [x] Sway keybinding ($mod+Shift+d, user configurable)
- [x] systemd user service integration
- [x] Daemon process with Unix socket IPC
- [x] CLI tools (toggle, status, stop)
- [x] Setup automation scripts

### Phase 2: Text Transformation (COMPLETED)
- [x] MidStream PyO3 integration (~1µs latency)
- [x] Voice command to symbol conversion
- [x] Comprehensive test suite (13 tests passing)
- [x] Performance monitoring and statistics
- [x] Documentation & user guide
- [x] systemd integration guide

### Phase 3: Polish (IN PROGRESS)
- [ ] Performance optimization (adaptive silence threshold)
- [ ] Configuration system (YAML for VAD/streaming params)
- [ ] Extended voice command library (400+ commands)
- [ ] GUI status indicator

---

## **Limitations & Known Issues** ⚠️

**Current Limitations:**
- ✅ **Configurable silence threshold** - Set via `~/.config/swictation/config.toml`
- ⚠️ **Limited voice command library** - Common symbols only (see docs/voice-commands.md)
- ⚠️ **Manual systemd setup** - Setup scripts available but require manual run
- ⚠️ **No visual indicator** - No on-screen feedback when recording (daemon logs only)

**Completed (No Longer Limitations):**
- ✅ Text transformations working! ("comma" → ",", ~1µs latency)
- ✅ VAD integrated into main daemon (was test-only)
- ✅ Streaming with segment detection (was batch-only)
- ✅ Auto-transcription on pauses (was manual toggle per sentence)
- ✅ Graceful degradation (transformer failure doesn't crash daemon)

---

## **Documentation** 📚

- **[Installation Guide](docs/sway-integration.md)** - Complete setup instructions
- **[Architecture](docs/architecture.md)** - System design and components
- **[Troubleshooting](docs/troubleshooting.md)** - Common issues and solutions
- **[Voice Commands](docs/voice-commands.md)** - Coding command reference

---

## **Troubleshooting** 🔧

### Quick Diagnostics

**Is the daemon running?**
```bash
systemctl --user status swictation.service
# or
python3 /opt/swictation/src/swictation_cli.py status
```

**✅ Expected:** `Active: active (running)`
**❌ If not running:**
```bash
# Check logs for errors
journalctl --user -u swictation.service -n 50

# Restart daemon
systemctl --user restart swictation.service
```

### Common Issues

#### 1. Hotkey Not Working ($mod+Shift+d does nothing)

**Cause:** Sway keybinding not configured or daemon not running

**Fix:**
```bash
# Verify daemon is running
systemctl --user status swictation.service

# Re-run setup script
cd /opt/swictation
./scripts/setup-sway.sh
swaymsg reload

# Test manually
python3 /opt/swictation/src/swictation_cli.py toggle
```

#### 2. No Text Appears After Speaking

**Cause:** wtype not working or wrong window focus

**Fix:**
```bash
# Test wtype manually (open text editor first!)
echo "test text" | wtype -

# If nothing appears, check Wayland
echo $WAYLAND_DISPLAY  # Should show "wayland-0" or similar

# Verify you're in Wayland, not Xorg
echo $XDG_SESSION_TYPE  # Should be "wayland"
```

**Also check:**
- Is your text editor focused when transcription happens?
- Try a simple editor first (kate, gedit) before VSCode/vim

#### 3. Audio Not Being Captured

**Cause:** Wrong audio device or PipeWire issue

**Fix:**
```bash
# List audio devices
python3 -c "import sounddevice as sd; print(sd.query_devices())"

# Test recording (speaks back what you say)
python3 /opt/swictation/src/audio_capture.py 5

# Check PipeWire is running
systemctl --user status pipewire
```

#### 4. GPU Out of Memory

**Cause:** VRAM < 4GB or other GPU processes running

**Fix:**
```bash
# Check GPU memory usage
nvidia-smi

# With FP16 optimization:
# - Model: ~1.8GB
# - Buffer (20s): ~400MB
# - Total typical: ~2.2GB
# - Peak usage: ~3.5GB (safe on 4GB GPUs)
#
# Legacy FP32 mode required ~3.6GB (not recommended)

# Kill other GPU processes if needed
# Free up at least 2.5GB for safe operation
```

#### 5. Daemon Crashes on Startup

**Cause:** Missing dependencies or model download failure

**Fix:**
```bash
# Reinstall dependencies
pip install --force-reinstall -r requirements.txt

# Manually download model
python3 -c "from nemo.collections.asr.models import EncDecMultiTaskModel; EncDecMultiTaskModel.from_pretrained('nvidia/canary-1b-flash')"

# Check logs
journalctl --user -u swictation.service -n 100
```

### Performance Issues

**High latency (>3s after pause)?**
- Check GPU load: `nvidia-smi`
- Verify no CPU throttling
- Ensure daemon isn't using CPU fallback

**Text appears in wrong order?**
- This is a known limitation when speaking too fast
- Solution: Pause 2+ seconds between thoughts

**Battery draining fast?**
- VAD is very efficient (2.2 MB GPU)
- Main drain is continuous STT model in VRAM
- Consider stopping daemon when not in use:
  ```bash
  systemctl --user stop swictation.service  # Stop
  systemctl --user start swictation.service  # Start later
  ```

### Getting More Help

**Check detailed logs:**
```bash
# Real-time logs
journalctl --user -u swictation.service -f

# Last 200 lines
journalctl --user -u swictation.service -n 200
```

**Debug mode:**
```bash
# Stop systemd service
systemctl --user stop swictation.service

# Run manually to see all output
python3 /opt/swictation/src/swictationd.py
```

📖 **Full Troubleshooting Guide:** [docs/troubleshooting.md](docs/troubleshooting.md)

---

## **Contributing** 🤝

Contributions welcome! Priority areas:

1. **Configuration System** - YAML config for VAD thresholds and parameters
2. **Performance Optimization** - Adaptive silence threshold with smarter detection
3. **Extended Voice Commands** - Expand MidStream transformer library (see docs/voice-commands.md)
4. **GUI Status Indicator** - Visual feedback for recording state
5. **Testing** - Additional edge case coverage and integration tests

---

## **License** 📄

Apache License 2.0 - See [LICENSE](LICENSE) for details.

---

## **Acknowledgments** 🙏

- **NVIDIA** - Canary-1B-Flash STT model
- **Silero Team** - Lightweight VAD model
- **NeMo Contributors** - ASR framework
- **Sway/Wayland Community** - Compositor and tools
- **Rust/PyO3 Communities** - Text transformation infrastructure

---

**Status:** Production Ready - VAD-Triggered Streaming + Text Transformation Active

**Hardware Tested:** NVIDIA RTX A1000 Laptop GPU (4GB VRAM) with FP16 mixed precision (~2.2GB typical usage)

**Latest Feature:** MidStream PyO3 text transformation (~1µs latency, 296,677x faster than subprocess!)

**Next Milestone:** Configuration system and extended voice command library
