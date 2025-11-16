# Ubuntu 25.10 (Questing Quetzal) Quick Start Guide

Swictation is fully optimized for Ubuntu 25.10 with GNOME Wayland. This guide provides a streamlined installation and setup process.

## System Overview

Ubuntu 25.10 ships with:
- ✅ GNOME 47+ (Wayland by default)
- ✅ GLIBC 2.40 (meets swictation requirements)
- ✅ PipeWire audio system
- ✅ Modern kernel with uinput support

## One-Command Installation

```bash
npm install -g swictation --foreground-scripts
```

**That's it!** The installation automatically:
1. ✅ Downloads GPU libraries (~2.3GB if NVIDIA GPU detected)
2. ✅ Installs ydotool for text injection
3. ✅ Configures uinput kernel module
4. ✅ Sets up keyboard shortcut: **Super+Shift+D**
5. ✅ Enables and starts systemd service
6. ✅ Tests GPU acceleration

## Verification

```bash
# Quick status check
swictation status

# Comprehensive verification
./node_modules/swictation/scripts/verify-installation.sh
```

**Expected output:**
```
✓ Wayland detected
✓ GNOME Wayland detected
✓ NVIDIA GPU detected (if applicable)
✓ GPU libraries installed
✓ ydotool installed
✓ uinput module loaded
✓ /dev/uinput permissions correct
✓ User in 'input' group
✓ GNOME keyboard shortcut configured
✓ Service enabled
✓ Service running
✓ IPC socket responding
```

## First Use

1. **Open any text editor** (gedit, VS Code, Terminal, etc.)
2. **Press Super+Shift+D** (Windows key + Shift + D)
3. **Speak** - You'll see a notification: "Recording started"
4. **Press Super+Shift+D again** - Notification: "Recording stopped"
5. **Your text appears** in the active window!

## Keyboard Shortcut

The hotkey is automatically configured in GNOME Settings:
- **Shortcut**: Super+Shift+D
- **Location**: Settings → Keyboard → View and Customize Shortcuts → Custom Shortcuts
- **Name**: Swictation Toggle

## GPU Acceleration

If you have an NVIDIA GPU:
- **Expected performance**: < 1 second transcription
- **Libraries**: Auto-downloaded CUDA + cuDNN (2.3GB)
- **Verification**: Check logs for "Successfully registered `CUDAExecutionProvider`"

```bash
journalctl --user -u swictation-daemon.service | grep CUDA
```

**Without GPU (CPU mode):**
- **Expected performance**: 20-30 seconds transcription
- **Still functional**: Yes, just slower

## Troubleshooting

### Text not typing
Most likely a permissions issue. Requires logout/login after installation:

```bash
# Check group membership
groups | grep input

# If not in 'input' group, you need to logout/login
# Or force group change:
newgrp input

# Test ydotool
sg input -c 'ydotool type "test"'
```

### Hotkey types "D" instead of toggling
The shortcut may not be configured. Manually configure it:

```bash
./node_modules/swictation/scripts/setup-gnome-shortcuts.sh
```

Then verify in Settings → Keyboard → View and Customize Shortcuts.

### Slow transcription (CPU instead of GPU)
**Always start via systemctl**, not manually!

```bash
# Stop any manual instances
pkill -f swictation-daemon

# Start via systemctl (has correct LD_LIBRARY_PATH)
systemctl --user restart swictation-daemon.service

# Verify GPU is being used
journalctl --user -u swictation-daemon.service | grep CUDA
# Should show: "Successfully registered `CUDAExecutionProvider`"
```

### Service won't start
Check the logs for errors:

```bash
journalctl --user -u swictation-daemon.service -n 50
```

Common issues:
- Missing models: Download with `swictation download-model 1.1b-gpu`
- Permission issues: Run verification script
- Port conflicts: Check if another instance is running

## Performance Expectations

### With NVIDIA GPU (RTX 20-series or newer)
- **Transcription**: < 1 second (62.8x realtime)
- **Text injection**: Instant
- **Hotkey response**: < 100ms
- **Total latency**: Speech → text appearing: ~1 second

### CPU Mode (no GPU)
- **Transcription**: 20-30 seconds
- **Text injection**: Instant
- **Hotkey response**: < 100ms
- **Total latency**: Speech → text appearing: ~25 seconds

## Secretary Mode

Swictation includes 60+ natural language commands:
- Say **"comma"** → Get `,`
- Say **"new line"** → Get line break
- Say **"quote hello quote"** → Get `"hello"`
- Say **"cap this is a test"** → Get `This is a test`
- Say **"mr smith said quote hello quote"** → Get `Mr. Smith said "hello"`

**[→ Full Secretary Mode Guide](secretary-mode.md)**

## Advanced Configuration

Configuration file: `~/.config/swictation/config.toml`

```toml
# Voice Activity Detection
vad_threshold = 0.25      # 0.0-1.0, higher = less sensitive
vad_min_silence = 0.8     # Seconds of silence before flush

# Recording mode
recording_mode = "toggle" # "toggle" or "ptt" (push-to-talk)

# Model selection
stt_model_override = "1.1b-gpu"  # Force specific model
```

Reload configuration:
```bash
systemctl --user restart swictation-daemon.service
```

## System Integration

### Autostart
Service is already enabled to start on login via systemd user service.

### Multiple Audio Devices
Swictation auto-detects all PulseAudio/PipeWire devices. Select your microphone in:
- GNOME Settings → Sound → Input
- Or configure in `~/.config/swictation/config.toml`

### Privacy
- Audio is processed **locally** on your machine
- No data sent to cloud services
- No internet connection required for transcription
- Full privacy and control

## System Requirements Met by Ubuntu 25.10

| Requirement | Ubuntu 25.10 Status |
|-------------|-------------------|
| GLIBC 2.39+ | ✅ 2.40 included |
| Wayland Support | ✅ Default display server |
| GNOME 45+ | ✅ GNOME 47 included |
| systemd | ✅ Pre-installed |
| PipeWire | ✅ Default audio system |
| Modern Kernel | ✅ Linux 6.11+ |
| uinput Support | ✅ Built-in |

## Additional Resources

- [Complete Wayland Support Guide](WAYLAND_SUPPORT.md)
- [Secretary Mode Commands](secretary-mode.md)
- [GitHub Repository](https://github.com/robertelee78/swictation)
- [Issue Tracker](https://github.com/robertelee78/swictation/issues)

## Support

If you encounter issues:
1. Run verification script: `./node_modules/swictation/scripts/verify-installation.sh`
2. Check logs: `journalctl --user -u swictation-daemon.service -n 50`
3. Report issues: https://github.com/robertelee78/swictation/issues

---

**Enjoy hands-free dictation on Ubuntu 25.10!** 🎤✨
