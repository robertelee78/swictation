# macOS Setup Guide

Complete installation and setup guide for Swictation on macOS.

---

## Prerequisites

### Hardware Requirements

- **Apple Silicon Mac** (M1, M2, M3, or M4)
  - Intel Macs are NOT supported
  - Check: `uname -m` should show `arm64`
- **8GB RAM minimum** (16GB+ recommended for 1.1B model)
- **3GB free disk space** for models and libraries

### Software Requirements

- **macOS 14 Sonoma** or **macOS 15 Sequoia**
  - Earlier versions not supported (CoreML requirements)
  - Check: System Settings → General → About
- **Node.js 18+**
  - Install: `brew install node` or download from https://nodejs.org
  - Check: `node --version`

---

## Installation

### Step 1: Install via npm

```bash
# One-time npm setup (avoids sudo)
echo "prefix=$HOME/.npm-global" > ~/.npmrc
export PATH="$HOME/.npm-global/bin:$PATH"
echo 'export PATH="$HOME/.npm-global/bin:$PATH"' >> ~/.zprofile

# Install Swictation
npm install -g swictation --foreground-scripts
```

### What postinstall does automatically:

1. **Platform Detection**
   - Verifies macOS 14+ and Apple Silicon
   - Checks Node.js version

2. **GPU Detection**
   - Detects unified memory (35% allocated to GPU)
   - Example: 16GB Mac → ~5.6GB GPU share
   - Recommends appropriate model (0.6B or 1.1B)

3. **Library Download** (~1.5GB)
   - ONNX Runtime with CoreML support
   - Model files from HuggingFace

4. **Service Installation**
   - Creates LaunchAgent plists in `~/Library/LaunchAgents/`
   - `com.swictation.daemon.plist` - Main daemon
   - `com.swictation.ui.plist` - System tray UI

5. **Configuration**
   - Creates `~/.config/swictation/config.toml`
   - Sets up logging directory

---

## Initial Setup

### Step 2: Grant Accessibility Permissions

**CRITICAL:** Swictation needs Accessibility permissions to inject text.

1. Open **System Settings**
2. Navigate to **Privacy & Security** → **Accessibility**
3. Click the **🔒 lock** to make changes (enter password)
4. Click **+** button
5. Navigate to: `/Users/[your-username]/.npm-global/lib/node_modules/swictation/bin/`
6. Add `swictation-daemon-macos`
7. **Enable the checkbox** next to swictation-daemon-macos
8. Close System Settings

**Verification:**
```bash
# Check if permission granted
ls ~/Library/LaunchAgents/com.swictation.daemon.plist
# Should exist without errors
```

### Step 3: Start Swictation

```bash
# Start the daemon
swictation start

# Check status
swictation status

# Should show:
# Daemon: ● Active
# Socket: ● Connected
```

---

## Using Swictation

### Default Hotkey

**Cmd+Shift+D** - Toggle recording on/off

**How it works:**
1. Press `Cmd+Shift+D` to start recording
2. Speak naturally
3. Pause for 0.8 seconds → text appears automatically
4. Press `Cmd+Shift+D` again to stop

### Testing

1. Open **TextEdit** (or any text editor)
2. Press `Cmd+Shift+D`
3. Say: "Hello world period"
4. Wait 1 second
5. Text should appear: "Hello world."

### Secretary Mode

Speak punctuation and formatting commands:

```
YOU SAY:          "hello comma world period"
SWICTATION TYPES: Hello, world.

YOU SAY:          "number forty two items"
SWICTATION TYPES: 42 items

YOU SAY:          "open quote hello close quote"
SWICTATION TYPES: "hello"
```

📖 **[Full Secretary Mode Guide](secretary-mode.md)** - 60+ commands

---

## Service Management

### Manual Service Control

```bash
# Start services
swictation start

# Stop services
swictation stop

# Check status
swictation status

# View logs
tail -f ~/Library/Logs/swictation/daemon.log
tail -f ~/Library/Logs/swictation/daemon-error.log
```

### Auto-start on Login

Services are configured to auto-start by default via LaunchAgents.

**Disable auto-start:**
```bash
launchctl unload ~/Library/LaunchAgents/com.swictation.daemon.plist
launchctl unload ~/Library/LaunchAgents/com.swictation.ui.plist
```

**Re-enable auto-start:**
```bash
launchctl load ~/Library/LaunchAgents/com.swictation.daemon.plist
launchctl load ~/Library/LaunchAgents/com.swictation.ui.plist
```

---

## Performance Optimization

### Model Selection by RAM

Swictation automatically selects the best model based on your Mac's unified memory:

| Mac Configuration | Total RAM | GPU Share (35%) | Model Selected | Expected Latency |
|------------------|-----------|-----------------|----------------|------------------|
| M1 (8GB) | 8GB | ~2.8GB | CPU fallback | 300-400ms |
| M1 (16GB) | 16GB | ~5.6GB | 0.6B GPU | 150-200ms |
| M1 Pro (32GB) | 32GB | ~11.2GB | 1.1B GPU | 200-250ms |
| M1 Max (64GB) | 64GB | ~22.4GB | 1.1B GPU | 150-200ms |

### GPU Acceleration

CoreML automatically uses:
- **GPU (Metal)** - For neural network inference
- **Neural Engine (ANE)** - For certain operations
- **CPU** - For unsupported operations

**Verify GPU usage:**
1. Open **Activity Monitor**
2. Select **GPU** tab
3. Start recording and speak
4. You should see GPU usage spike during transcription

### Manual Model Override

Edit `~/.config/swictation/config.toml`:

```toml
# Force specific model (overrides auto-detection)
stt_model_override = "1.1b-gpu"  # or "0.6b-gpu" or "0.6b-cpu"
```

---

## Troubleshooting

### "Permission denied" errors

**Cause:** Accessibility permissions not granted

**Fix:**
1. System Settings → Privacy & Security → Accessibility
2. Add `swictation-daemon-macos` binary
3. Enable the checkbox
4. Restart: `swictation stop && swictation start`

### Text not appearing

**Check 1 - Service running:**
```bash
swictation status
# Should show: Daemon: ● Active
```

**Check 2 - Logs:**
```bash
tail -f ~/Library/Logs/swictation/daemon-error.log
# Look for errors about Accessibility permissions
```

**Check 3 - Hotkey conflict:**
- Another app might be using Cmd+Shift+D
- Check System Settings → Keyboard → Keyboard Shortcuts

### CoreML/GPU not working

**Check 1 - Verify CoreML library:**
```bash
ls -lh ~/.npm-global/lib/node_modules/swictation/lib/native/libonnxruntime.dylib
# Should show ~30-50MB file
```

**Check 2 - Check logs for GPU usage:**
```bash
grep -i "coreml\|gpu\|metal" ~/Library/Logs/swictation/daemon.log
# Should see: "Enabling CoreML execution provider"
```

**Check 3 - Verify model format:**
- macOS uses FP16 models (best for CoreML)
- Check `~/.local/share/swictation/models/`
- Should have `.fp16.onnx` files

### Daemon crashes on startup

**Check 1 - Model files corrupted:**
```bash
# Re-download models
swictation download-models --force
```

**Check 2 - Library compatibility:**
```bash
# Check dylib dependencies
otool -L ~/.npm-global/lib/node_modules/swictation/lib/native/libonnxruntime.dylib
# All paths should exist
```

**Check 3 - Reinstall:**
```bash
npm uninstall -g swictation
npm install -g swictation --foreground-scripts
```

### High memory usage

**Normal behavior:**
- 1.1B model: ~3.5GB unified memory during inference
- 0.6B model: ~1.2GB unified memory
- Memory released after transcription completes

**If excessive:**
- Check Activity Monitor → Memory tab
- Look for multiple swictation processes
- Kill duplicates: `pkill -f swictation-daemon`
- Restart: `swictation start`

---

## Advanced Configuration

### Hotkey Customization

**Note:** Custom hotkeys not yet supported on macOS (planned for future release).

Current implementation uses `Cmd+Shift+D` (hardcoded in daemon).

### Silence Detection Tuning

Edit `~/.config/swictation/config.toml`:

```toml
# Silence duration before auto-transcription (seconds)
silence_duration = 0.8  # Default: 0.8 (800ms)

# Increase for slower speakers: 1.2
# Decrease for faster workflow: 0.5
```

### VAD Threshold

```toml
# Voice Activity Detection sensitivity (0.0 - 1.0)
vad_threshold = 0.25  # Default: 0.25

# Lower = more sensitive (may trigger on background noise)
# Higher = less sensitive (may miss soft speech)
```

---

## Uninstallation

### Complete Removal

```bash
# 1. Stop services
swictation stop

# 2. Unload LaunchAgents
launchctl unload ~/Library/LaunchAgents/com.swictation.daemon.plist
launchctl unload ~/Library/LaunchAgents/com.swictation.ui.plist

# 3. Remove LaunchAgent plists
rm ~/Library/LaunchAgents/com.swictation.daemon.plist
rm ~/Library/LaunchAgents/com.swictation.ui.plist

# 4. Uninstall npm package
npm uninstall -g swictation

# 5. Remove configuration and data (optional)
rm -rf ~/.config/swictation
rm -rf ~/.local/share/swictation
rm -rf ~/Library/Logs/swictation

# 6. Remove Accessibility permission
# System Settings → Privacy & Security → Accessibility
# Remove swictation-daemon-macos
```

---

## Known Limitations

### Current Limitations (v0.7.0)

- **Intel Macs not supported** - Apple Silicon (ARM64) required
- **macOS 13 and earlier not supported** - CoreML requirements
- **Custom hotkeys not yet supported** - Cmd+Shift+D only
- **No Python tray UI** - Tauri UI planned for future release
- **INT8 models not optimized** - CoreML prefers FP16 (auto-selected)

### Compared to Linux

| Feature | Linux | macOS |
|---------|-------|-------|
| GPU | NVIDIA CUDA | CoreML/Metal |
| Model Format | FP32/INT8 | FP16 |
| Hotkey | Configurable | Cmd+Shift+D (fixed) |
| Display Server | X11/Wayland | Quartz (native) |
| Text Injection | xdotool/wtype/ydotool | Accessibility API |
| Service Manager | systemd | launchd |

---

## Getting Help

### Log Locations

```bash
# Daemon logs
~/Library/Logs/swictation/daemon.log
~/Library/Logs/swictation/daemon-error.log

# UI logs (if running)
~/Library/Logs/swictation/ui.log
~/Library/Logs/swictation/ui-error.log

# Configuration
~/.config/swictation/config.toml

# Models and data
~/.local/share/swictation/models/
~/.local/share/swictation/metrics.db
```

### Reporting Issues

When reporting issues, include:

1. **System info:**
   ```bash
   system_profiler SPSoftwareDataType SPHardwareDataType | grep "System Version\|Model Name\|Chip\|Memory"
   ```

2. **Swictation version:**
   ```bash
   swictation --version
   ```

3. **Error logs:**
   ```bash
   tail -50 ~/Library/Logs/swictation/daemon-error.log
   ```

4. **Steps to reproduce**

**Submit at:** https://github.com/robertelee78/swictation/issues

---

## Next Steps

- **[Secretary Mode Guide](secretary-mode.md)** - Learn all 60+ voice commands
- **[Architecture Documentation](architecture.md)** - Technical deep dive
- **[Troubleshooting Guide](troubleshooting-display-servers.md)** - Common issues

---

**Enjoy hands-free dictation! 🎤**
