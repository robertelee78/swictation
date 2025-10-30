# Swictation Troubleshooting Guide

Common issues and solutions for Swictation voice dictation system.

---

## Quick Diagnostic Checklist ✓

Run these commands to diagnose most issues:

```bash
# 1. Check if daemon is running
python3 /opt/swictation/src/swictation_cli.py status

# 2. Test audio capture
python3 /opt/swictation/src/audio_capture.py list

# 3. Test text injection
echo "test" | wtype -

# 4. Check GPU availability
python3 -c "import torch; print(f'CUDA: {torch.cuda.is_available()}')"

# 5. Check Wayland
echo $WAYLAND_DISPLAY  # Should output something like "wayland-1"
```

---

## Daemon Issues

### Daemon Won't Start

**Symptoms:** `python3 swictationd.py` exits immediately or hangs

**Diagnostics:**
```bash
# Check for existing daemon
pgrep -f swictationd.py

# Check socket
ls -la /tmp/swictation.sock

# Try starting with verbose output
python3 /opt/swictation/src/swictationd.py 2>&1 | tee daemon.log
```

**Common Causes & Fixes:**

**1. Socket already exists (previous daemon crashed)**
```bash
# Remove stale socket
rm /tmp/swictation.sock

# Kill existing process
pkill -f swictationd.py

# Start fresh
python3 /opt/swictation/src/swictationd.py
```

**2. Model download failed**
```bash
# Manually download model
python3 -c "
from nemo.collections.asr.models import EncDecMultiTaskModel
model = EncDecMultiTaskModel.from_pretrained('nvidia/canary-1b-flash')
print('Model downloaded successfully')
"
```

**3. CUDA/GPU not available**
```bash
# Check CUDA
python3 -c "import torch; print(torch.cuda.is_available())"

# If False, check NVIDIA driver
nvidia-smi

# Reinstall CUDA-enabled PyTorch
pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu118
```

---

### Daemon Crashes During Operation

**Symptoms:** Daemon exits with error during recording/transcription

**Check Logs:**
```bash
# If using systemd
journalctl --user -u swictation.service -f

# If running manually
# (output should be visible in terminal)
```

**Common Errors:**

**1. CUDA Out of Memory (OOM)**
```
RuntimeError: CUDA out of memory. Tried to allocate X GB
```

**Fix:** Audio chunk size too large
```python
# In swictationd.py, reduce chunk size:
CHUNK_SIZE = 10  # seconds (default)
# Try reducing to 5 seconds:
CHUNK_SIZE = 5
```

**2. Audio Device Disconnected**
```
OSError: [Errno 19] Device or resource busy
```

**Fix:** Reconnect audio device or restart PipeWire
```bash
# Restart PipeWire
systemctl --user restart pipewire pipewire-pulse

# Or restart daemon
systemctl --user restart swictation.service
```

**3. Model Loading Timeout**
```
TimeoutError: Model loading took too long
```

**Fix:** Increase timeout or use faster storage
```bash
# Move model cache to faster disk (e.g., NVMe)
mv ~/.cache/huggingface /path/to/fast/disk/
ln -s /path/to/fast/disk/huggingface ~/.cache/huggingface
```

---

## Audio Capture Issues

### No Audio Devices Found

**Symptoms:** `python3 audio_capture.py list` shows no devices

**Diagnostics:**
```bash
# Check PipeWire/PulseAudio
pactl list sources short

# Check ALSA
arecord -l

# Check sounddevice
python3 -c "import sounddevice as sd; print(sd.query_devices())"
```

**Fixes:**

**1. PipeWire not running**
```bash
# Start PipeWire
systemctl --user start pipewire pipewire-pulse

# Enable auto-start
systemctl --user enable pipewire pipewire-pulse
```

**2. Permissions issue**
```bash
# Add user to audio group
sudo usermod -aG audio $USER

# Logout and login for changes to take effect
```

---

### Audio Captured But No Transcription

**Symptoms:** Recording works, but no text appears or transcription is empty

**Diagnostics:**
```bash
# Test with known audio file
python3 tests/test_canary.py

# Check VAD threshold
python3 tests/test_canary_vad.py
```

**Common Causes:**

**1. Audio too quiet**
```bash
# Increase microphone gain
pactl set-source-volume @DEFAULT_SOURCE@ +10%

# Or use alsamixer
alsamixer
```

**2. VAD threshold too strict**
```python
# In swictationd.py, lower VAD threshold:
VAD_THRESHOLD = 0.5  # default
# Try:
VAD_THRESHOLD = 0.3  # more sensitive
```

**3. Wrong audio format**
```python
# Ensure 16kHz mono audio
# In audio_capture.py:
self.sample_rate = 16000  # Required by Canary-1B
self.channels = 1         # Mono only
```

---

## Text Injection Issues

### Text Not Appearing at Cursor

**Symptoms:** Transcription works, but text doesn't appear in target application

**Diagnostics:**
```bash
# Test wtype manually
echo "Hello, world!" | wtype -

# Check Wayland
echo $WAYLAND_DISPLAY

# Test in simple editor first (kate, gedit)
```

**Common Fixes:**

**1. Wayland not available**
```bash
# Check if running X11 instead of Wayland
echo $XDG_SESSION_TYPE  # Should be "wayland"

# If X11, Swictation won't work (X11 not supported)
# Switch to Wayland session on login
```

**2. wtype not installed**
```bash
# Install wtype
# Arch/Manjaro:
sudo pacman -S wtype

# Ubuntu/Debian:
sudo apt install wtype

# Or build from source:
git clone https://github.com/atx/wtype
cd wtype
meson build && ninja -C build
sudo ninja -C build install
```

**3. Target application doesn't accept input**
```bash
# Some applications block programmatic input
# Try different application (kate, gedit, vim, firefox)

# Fallback to clipboard mode:
python3 -c "
from text_injection import TextInjector, InjectionMethod
injector = TextInjector(method=InjectionMethod.CLIPBOARD)
injector.inject('test')
"
# Then manually paste with Ctrl+V
```

---

### Unicode Characters Not Working

**Symptoms:** Emojis, Greek, or Chinese characters appear as boxes or are missing

**Diagnostics:**
```bash
# Test Unicode support
echo "Hello 世界 🌍 αβγ" | wtype -

# Check terminal encoding
locale  # Should show UTF-8
```

**Fix:** Ensure UTF-8 locale
```bash
# Set UTF-8 locale
export LANG=en_US.UTF-8
export LC_ALL=en_US.UTF-8

# Make permanent in ~/.bashrc or ~/.zshrc
echo "export LANG=en_US.UTF-8" >> ~/.bashrc
```

---

## Sway Integration Issues

### Keybinding Not Working

**Symptoms:** Pressing `Alt+Shift+d` does nothing

**Diagnostics:**
```bash
# Check Sway config syntax
sway --validate

# Check if keybinding is registered
swaymsg -t get_binding_state

# Test command manually
python3 /opt/swictation/src/swictation_cli.py toggle
```

**Fixes:**

**1. Keybinding not added to config**
```bash
# Check Sway config
grep -i swictation ~/.config/sway/config

# If missing, run setup script
sudo /opt/swictation/scripts/setup-sway.sh

# Or add manually
echo "bindsym Mod1+Shift+d exec python3 /opt/swictation/src/swictation_cli.py toggle" >> ~/.config/sway/config
```

**2. Config not reloaded**
```bash
# Reload Sway config
swaymsg reload
```

**3. Keybinding conflict**
```bash
# Check for existing Mod1+Shift+d binding
grep "Mod1+Shift+d" ~/.config/sway/config

# Change to different key if conflict exists
bindsym Mod1+Shift+v exec python3 /opt/swictation/src/swictation_cli.py toggle
```

---

## Performance Issues

### High Latency (>1 second)

**Symptoms:** Long delay between speaking and text appearance

**Diagnostics:**
```bash
# Check GPU utilization
nvidia-smi dmon -s u -c 10

# Run benchmark
python3 tests/test_canary_chunked.py
```

**Optimization Tips:**

**1. GPU thermal throttling**
```bash
# Check GPU temperature
nvidia-smi

# If >80°C, improve cooling or reduce GPU clock
nvidia-smi -pl 60  # Set power limit to 60W
```

**2. Reduce chunk size**
```python
# In swictationd.py:
CHUNK_SIZE = 10  # default
# Try 5 seconds for faster processing:
CHUNK_SIZE = 5
```

**3. Close background GPU applications**
```bash
# Check GPU processes
nvidia-smi

# Kill unnecessary processes
# (browsers with hardware acceleration, games, etc.)
```

---

### High Memory Usage

**Symptoms:** System freezing, OOM killer, swap usage

**Diagnostics:**
```bash
# Check memory usage
nvidia-smi  # GPU memory
free -h     # System memory

# Monitor daemon
ps aux | grep swictationd
```

**Fixes:**

**1. Reduce audio buffer size**
```python
# In audio_capture.py:
self.buffer_size = 16000 * 60  # 1 minute (default)
# Try 30 seconds:
self.buffer_size = 16000 * 30
```

**2. Enable model quantization** (future feature)
```python
# Coming soon: 8-bit quantization for 2GB VRAM GPUs
```

**3. Use smaller STT model** (future feature)
```python
# Coming soon: Support for Whisper tiny/small models
```

---

## systemd Issues

### Service Won't Start

**Symptoms:** `systemctl --user start swictation` fails

**Diagnostics:**
```bash
# Check service status
systemctl --user status swictation.service

# View logs
journalctl --user -u swictation.service -n 50

# Check service file syntax
systemd-analyze verify ~/.config/systemd/user/swictation.service
```

**Fixes:**

**1. Service file not installed**
```bash
# Copy service file
cp /opt/swictation/config/swictation.service ~/.config/systemd/user/

# Reload systemd
systemctl --user daemon-reload
```

**2. Python path wrong**
```bash
# Find correct Python path
which python3  # e.g., /usr/bin/python3

# Update service file ExecStart path
nano ~/.config/systemd/user/swictation.service
```

**3. DISPLAY/WAYLAND_DISPLAY not set**
```bash
# Service needs Wayland socket
# Edit service file to import environment:
[Service]
ImportEnvironment=WAYLAND_DISPLAY DISPLAY XDG_RUNTIME_DIR
```

---

## Model Issues

### Model Download Fails

**Symptoms:** "Failed to download model" error on first run

**Fixes:**

**1. Network issues**
```bash
# Download manually
huggingface-cli download nvidia/canary-1b-flash

# Or use wget
wget https://huggingface.co/nvidia/canary-1b-flash/resolve/main/canary-1b-flash.nemo
```

**2. Disk space**
```bash
# Model is ~1.1 GB
# Check free space
df -h ~/.cache/huggingface

# Clear old models if needed
rm -rf ~/.cache/huggingface/hub/models--*
```

**3. HuggingFace authentication**
```bash
# Some models require HF token
huggingface-cli login

# Enter token from https://huggingface.co/settings/tokens
```

---

### Model Crashes on Load

**Symptoms:** Segmentation fault or CUDA error during model loading

**Fixes:**

**1. CUDA version mismatch**
```bash
# Check CUDA version
nvcc --version
nvidia-smi  # Shows CUDA driver version

# Reinstall matching PyTorch
# For CUDA 11.8:
pip install torch==2.1.0+cu118 -f https://download.pytorch.org/whl/torch_stable.html
```

**2. Corrupted model cache**
```bash
# Delete and re-download
rm -rf ~/.cache/huggingface/hub/models--nvidia--canary-1b-flash
python3 -c "from nemo.collections.asr.models import EncDecMultiTaskModel; EncDecMultiTaskModel.from_pretrained('nvidia/canary-1b-flash')"
```

---

## General Tips

### Enable Debug Logging

```python
# In swictationd.py, add at top:
import logging
logging.basicConfig(level=logging.DEBUG)
```

### Test Components Individually

```bash
# Test audio only
python3 /opt/swictation/src/audio_capture.py 5

# Test STT only
python3 /opt/swictation/tests/test_canary.py

# Test text injection only
python3 /opt/swictation/tests/test_text_injection_automated.py

# Test daemon IPC only
python3 /opt/swictation/tests/test_daemon.py
```

### Check Dependencies

```bash
# Verify all dependencies
pip list | grep -E "nemo|torch|sound"

# Reinstall if needed
pip install -r /opt/swictation/requirements.txt --force-reinstall
```

---

## Getting Help

If none of these solutions work:

1. **Check logs:**
   ```bash
   journalctl --user -u swictation.service -f
   ```

2. **Enable debug mode:**
   ```bash
   SWICTATION_DEBUG=1 python3 /opt/swictation/src/swictationd.py
   ```

3. **File issue on GitHub:**
   - Include error logs
   - System info (GPU, OS, Sway version)
   - Steps to reproduce

4. **Community support:**
   - Join discussions on GitHub
   - Check existing issues for similar problems

---

**Last Updated:** 2025-10-30
