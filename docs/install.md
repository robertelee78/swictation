# Swictation Installation Guide

Complete installation instructions for the Swictation voice dictation system.

---

## System Requirements

### Hardware
- **GPU:** NVIDIA GPU with 4GB+ VRAM (RTX A1000, RTX 3050, RTX 4060 or better)
  - Works on budget GPUs: GTX 1050 Ti (4GB), RTX 3050 Mobile (4GB)
  - FP16 optimization uses ~2.2GB typical, ~3.5GB peak
- **RAM:** 8GB+ system RAM recommended
- **CPU:** x86_64 processor

### Software
- **OS:** Linux with Wayland compositor (Sway, Hyprland, or compatible)
- **Display Server:** Wayland (NOT X11)
- **Audio:** PipeWire or PulseAudio
- **Python:** 3.10 or higher
- **NVIDIA Driver:** 535+ (CUDA 11.8+ compatible)
- **Rust:** Latest stable (for building text transformer)

### Check Your System

```bash
# Check GPU
nvidia-smi  # Should show your NVIDIA GPU with 4GB+ VRAM

# Check Python version
python3 --version  # Should be 3.10 or higher

# Check Wayland session
echo $XDG_SESSION_TYPE  # Should output "wayland"

# Check display variable
echo $WAYLAND_DISPLAY  # Should output "wayland-0" or similar
```

---

## Installation Steps

### Step 1: Clone Repository with Submodules

**CRITICAL:** The midstream text transformer is a git submodule. You MUST initialize it.

```bash
# Option A: Clone with submodules (recommended for new installs)
git clone --recurse-submodules https://github.com/robertelee78/swictation.git /opt/swictation

# Option B: If you already cloned without --recurse-submodules
cd /opt/swictation
git submodule update --init --recursive
```

**Verify submodule is populated:**
```bash
ls -la /opt/swictation/external/midstream/
# Should show files, NOT an empty directory
```

---

### Step 2: Install System Dependencies

Choose your distribution:

**Arch Linux / Manjaro:**
```bash
sudo pacman -S python python-pip wtype wl-clipboard ffmpeg pipewire pipewire-pulse
```

**Ubuntu / Debian / Pop!_OS:**
```bash
sudo apt update
sudo apt install python3 python3-pip wtype wl-clipboard ffmpeg pipewire pipewire-pulse
```

**Fedora:**
```bash
sudo dnf install python3 python3-pip wtype wl-clipboard ffmpeg pipewire pipewire-pulseaudio
```

---

### Step 3: Install Rust Toolchain

The text transformer is written in Rust and requires the Rust toolchain to build.

```bash
# Install Rust via rustup (official installer)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow prompts, then activate Rust
source ~/.cargo/env

# Verify installation
rustc --version  # Should show rustc 1.70+ or higher
cargo --version  # Should show cargo 1.70+ or higher
```

**Note:** If Rust is already installed, skip this step.

---

### Step 4: Build MidStream Text Transformer

**This is the critical missing step that breaks fresh installs!**

The text transformer converts voice commands to symbols (e.g., "comma" → ",") with <1μs latency using Rust/PyO3.

```bash
# Install maturin (Rust-to-Python build tool)
pip3 install --user maturin

# Navigate to transformer crate
cd /opt/swictation/external/midstream/crates/text-transform

# Build the Python wheel (takes ~2-3 minutes first time)
maturin build --release --features pyo3

# Install the built wheel
pip3 install --break-system-packages ../../target/wheels/midstreamer_transform-*.whl
```

**Verify transformer installation:**
```bash
python3 -c "import midstreamer_transform; count, msg = midstreamer_transform.get_stats(); print(f'✅ {msg}')"
```

**Expected output:**
```
✅ 266 transformation rules loaded
```

**If verification fails:**
- Check wheel was built: `ls external/midstream/target/wheels/`
- Check maturin installed: `which maturin`
- Check Rust works: `rustc --version`
- See Troubleshooting section below

---

### Step 5: Install Python Dependencies

Now install the main Python packages (NeMo, PyTorch, etc.):

```bash
cd /opt/swictation

# Install all Python dependencies
pip3 install --break-system-packages -r requirements.txt
```

**This will take 5-10 minutes** as it downloads large ML packages (~3GB).

**Note:** The `--break-system-packages` flag is required on some distributions that use externally-managed Python environments.

---

### Step 6: Download STT Model

Download the NVIDIA Canary-1B-Flash model (~1.1GB):

```bash
python3 -c "from nemo.collections.asr.models import EncDecMultiTaskModel; EncDecMultiTaskModel.from_pretrained('nvidia/canary-1b-flash')"
```

**Expected output:**
```
Downloading model from HuggingFace...
GPU available: True
GPU: NVIDIA RTX A1000 Laptop GPU
✓ Model downloaded successfully
```

**If no GPU detected:**
```
GPU available: False
⚠ No GPU detected - STT will be slower on CPU
```

---

### Step 7: Install Systemd Service

Set up the daemon to auto-start with your Sway session:

```bash
cd /opt/swictation

# Install systemd user service
./scripts/install-systemd-service.sh

# Enable auto-start
systemctl --user enable swictation.service

# Start daemon now
systemctl --user start swictation.service
```

**Verify service is running:**
```bash
systemctl --user status swictation.service
```

**Expected output:**
```
● swictation.service - Swictation Voice Dictation Daemon
   Loaded: loaded
   Active: active (running) since ...
```

**Configuration file created:**

The install script creates `~/.config/swictation/config.toml` with default settings for Voice Activity Detection (VAD). You can edit this file to tune sensitivity and response speed for your speaking style and environment (see Configuration section below).

---

### Step 8: Configure Sway Keybinding

Add the toggle keybinding to your Sway config:

```bash
cd /opt/swictation

# Add keybinding (backs up existing config)
./scripts/setup-sway.sh

# Reload Sway to apply changes
swaymsg reload
```

**Default keybinding:** `$mod+Shift+d` (where `$mod` is your configured modifier key - usually Super/Windows or Alt)

**Verify keybinding was added:**
```bash
grep swictation ~/.config/sway/config
```

---

## Verification

### 1. Check Daemon Status

```bash
systemctl --user status swictation.service
```

Should show `Active: active (running)`.

### 2. Check Transformer Loaded

```bash
journalctl --user -u swictation.service -n 50 | grep "Text Transform"
```

**Expected output:**
```
✅ Text Transform: 266 transformation rules loaded
```

**If you see instead:**
```
⚠️  midstreamer_transform not installed - transformations disabled
```

This means Step 4 (building the transformer) failed. Go back to Step 4.

### 3. Test Recording Toggle

```bash
# Check current status
python3 /opt/swictation/src/swictation_cli.py status

# Toggle recording on
python3 /opt/swictation/src/swictation_cli.py toggle

# Check status again
python3 /opt/swictation/src/swictation_cli.py status
# Should show "Recording: active"

# Toggle off
python3 /opt/swictation/src/swictation_cli.py toggle
```

### 4. Test Voice Dictation

1. Open a text editor (kate, gedit, VSCode, etc.)
2. Press `$mod+Shift+d` to start recording
3. Say: "Hello comma world period" and pause 2 seconds
4. Text should appear: "Hello, world."
5. Press `$mod+Shift+d` again to stop recording

---

## Troubleshooting

### Submodule is Empty

**Problem:** `/opt/swictation/external/midstream/` contains no files

**Solution:**
```bash
cd /opt/swictation
git submodule update --init --recursive
ls external/midstream/  # Should now show files
```

### Transformer Not Loaded

**Problem:** Daemon logs show "midstreamer_transform not installed"

**Check if wheel was built:**
```bash
ls /opt/swictation/external/midstream/target/wheels/
```

If no files, rebuild:
```bash
cd /opt/swictation/external/midstream/crates/text-transform
maturin build --release --features pyo3
pip3 install --break-system-packages ../../target/wheels/midstreamer_transform-*.whl
```

**Check if wheel is installed:**
```bash
pip3 list | grep midstream
```

Should show: `midstreamer-transform  0.1.0`

**Test import directly:**
```bash
python3 -c "import midstreamer_transform; print('SUCCESS')"
```

### No GPU / CUDA Not Found

**Problem:** `nvidia-smi` not found or shows no GPU

**Check driver:**
```bash
nvidia-smi
```

**If command not found:**
```bash
# Arch/Manjaro
sudo pacman -S nvidia nvidia-utils

# Ubuntu/Debian
sudo apt install nvidia-driver-535

# Reboot after installation
sudo reboot
```

**Check CUDA availability in Python:**
```bash
python3 -c "import torch; print(f'CUDA available: {torch.cuda.is_available()}')"
```

### Audio Not Captured

**Problem:** Daemon runs but doesn't capture audio

**Check PipeWire:**
```bash
systemctl --user status pipewire
```

**List audio devices:**
```bash
python3 -c "import sounddevice as sd; print(sd.query_devices())"
```

**Test audio capture:**
The legacy Python audio_capture.py has been replaced by the Rust implementation. To test audio:
```bash
# Check logs for audio device detection
journalctl --user -u swictation-daemon -n 50 | grep -i audio
```

### Text Not Appearing (wtype fails)

**Problem:** Recording works but text doesn't appear in focused window

**Test wtype manually:**
```bash
# Open a text editor first, then run:
echo "test text" | wtype -
```

If nothing appears:

**Check you're on Wayland (not X11):**
```bash
echo $XDG_SESSION_TYPE  # Should be "wayland"
echo $WAYLAND_DISPLAY   # Should show "wayland-0" or similar
```

**Verify wtype is installed:**
```bash
which wtype  # Should show path like /usr/bin/wtype
```

**Try simple editor first:**
- Test with `kate` or `gedit` before VSCode/vim
- Make sure text editor window is FOCUSED when text appears

### Out of Memory (GPU)

**Problem:** Daemon crashes with CUDA out of memory

**Check GPU memory usage:**
```bash
nvidia-smi
```

**Free up VRAM:**
- Close other GPU applications (browsers with GPU acceleration, games, etc.)
- You need ~2.5GB free for safe operation
- FP16 mode uses ~2.2GB typical, ~3.5GB peak

**If still failing on 4GB GPU:**
- Check no other processes using GPU: `nvidia-smi`
- Restart to clear GPU memory: `sudo systemctl restart display-manager`

### Daemon Won't Start

**Problem:** `systemctl --user status swictation.service` shows failed

**Check logs:**
```bash
journalctl --user -u swictation.service -n 100
```

**Common issues:**
- Missing dependencies: Reinstall requirements.txt
- Model not downloaded: Run Step 6 again
- Python version too old: Check `python3 --version` (need 3.10+)

**Try manual start to see full errors:**
```bash
# Stop systemd service
systemctl --user stop swictation-daemon

# Run Rust daemon manually to see output
cd /opt/swictation/rust-crates
cargo run --release --bin swictation-daemon
```

---

## Configuration

Swictation uses `~/.config/swictation/config.toml` for configuration (created automatically by the install script).

### Available Settings

Currently, only VAD (Voice Activity Detection) settings are configurable:

**Config location:** `~/.config/swictation/config.toml`

**Available settings:**
```toml
[vad]
# Speech detection threshold
# IMPORTANT: Silero VAD ONNX uses MUCH LOWER thresholds than PyTorch!
# Valid range for ONNX: 0.0005-0.01 (NOT 0.0-1.0)
#
# The ONNX model outputs probabilities ~100-200x lower than PyTorch.
# DO NOT use PyTorch thresholds (0.5) - speech will never be detected!
#
# - 0.001 = most sensitive (catches quiet speech, may have false positives)
# - 0.003 = balanced (recommended default for ONNX)
# - 0.005 = conservative (fewer false positives, may miss quiet speech)
threshold = 0.003

# Silence duration in seconds before processing text
# How long to wait after speech ends before transcribing
# - Lower = faster response, may cut off sentences
# - Higher = more complete sentences, slower response
# - Common range: 0.5-3.0 seconds
silence_duration = 2.0
```

**After changing config:**
```bash
systemctl --user restart swictation.service
```

### Tuning for Your Environment

**Problem: Missing quiet speech or soft words**
```toml
[vad]
threshold = 0.001  # ONNX: Lower threshold = more sensitive
silence_duration = 2.0
```

**Problem: Too many false triggers from background noise**
```toml
[vad]
threshold = 0.005  # ONNX: Higher threshold = less sensitive
silence_duration = 2.0
```

**Problem: Sentences getting cut off mid-thought**
```toml
[vad]
threshold = 0.003  # Keep balanced ONNX threshold
silence_duration = 3.0  # Wait longer for complete thoughts
```

**Problem: Text takes too long to appear (slow response)**
```toml
[vad]
threshold = 0.003  # Keep balanced ONNX threshold
silence_duration = 1.0  # Faster response, shorter pauses
```

**NOTE:** See `/opt/swictation/rust-crates/swictation-vad/ONNX_THRESHOLD_GUIDE.md` for detailed threshold information.

### What is NOT Configurable

The following are hardcoded for optimal operation:
- **Keybinding** - Managed via Sway config (`~/.config/sway/config`), not Swictation
- **Model selection** - nvidia/canary-1b-flash is the only supported model
- **Audio device** - Auto-detected by the system
- **Injection method** - Auto-selects wtype with clipboard fallback
- **Sample rate, chunk settings** - Model-specific, optimized for Canary
- **Min segment duration** - 1.0s minimum (hardcoded for quality)

---

## Uninstallation

To completely remove Swictation:

```bash
# Stop and disable service
systemctl --user stop swictation.service
systemctl --user disable swictation.service
rm ~/.config/systemd/user/swictation.service

# Remove Sway keybinding (manual edit)
# Edit ~/.config/sway/config and remove swictation lines

# Remove installation
rm -rf /opt/swictation

# Remove config
rm -rf ~/.config/swictation

# Uninstall Python packages (optional)
pip3 uninstall -y midstreamer-transform nemo_toolkit
```

---

## Next Steps

- **Documentation:** See `docs/architecture.md` for technical details
- **GitHub:** https://github.com/robertelee78/swictation
- **Issues:** Report bugs at https://github.com/robertelee78/swictation/issues

**Enjoy hands-free coding!** 🎤
