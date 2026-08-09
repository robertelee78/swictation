# macOS Setup Guide

Complete installation and setup guide for Swictation on macOS.

---

## Prerequisites

### Hardware Requirements

- **Apple Silicon Mac** (M1 or later)
  - Intel Macs are NOT supported
  - Check: `uname -m` should show `arm64`
- **16GB+ unified memory** — a hard requirement, not a recommendation.
  Postinstall aborts on Macs reporting less.
- **4GB free disk space** for models (1.9GB CoreML bundle) and libraries (~1.5GB)

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
   - Detects unified memory (35% reported as the GPU share)
   - Example: 16GB Mac → ~5.6GB GPU share
   - Aborts the install if unified memory is under ~16GB
   - Selects the native CoreML 1.1B model (every supported Mac gets the same model)

3. **Library Download** (~1.5GB)
   - ONNX Runtime with CoreML support
   - Model files from HuggingFace, pinned to immutable upstream revisions and
     verified against per-file SHA-256 hashes before they are kept

4. **Service Installation**
   - Creates LaunchAgent plists in `~/Library/LaunchAgents/`
   - `com.swictation.daemon.plist` - Main daemon
   - `com.swictation.ui.plist` - System tray UI

5. **Configuration**
   - Creates `~/Library/Application Support/swictation/config.toml`
   - Sets up logging directory

---

## Initial Setup

### Step 2: Grant Accessibility Permissions

**CRITICAL:** Swictation needs Accessibility permissions to inject text.

1. Open **System Settings**
2. Navigate to **Privacy & Security** → **Accessibility**
3. Click the **🔒 lock** to make changes (enter password)
4. Click **+** button
5. Navigate to the platform package's bin directory. Print it with
   `swictation --version` (shown as "Location"); it is normally
   `<npm-prefix>/lib/node_modules/@agidreams/darwin-arm64/bin/`
6. Add `swictation-daemon`
7. **Enable the checkbox** next to swictation-daemon
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

**Ctrl+Shift+D** - Toggle recording on/off (configurable)

**How it works:**
1. Press `Ctrl+Shift+D` to start recording
2. Speak naturally
3. Pause for 0.8 seconds → text appears automatically
4. Press `Ctrl+Shift+D` again to stop

### Testing

1. Open **TextEdit** (or any text editor)
2. Press `Ctrl+Shift+D`
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

Every supported Mac runs the same native CoreML 1.1B model. Machines below the 16GB
unified-memory floor are refused at install time rather than downgraded:

| Mac Configuration | Total RAM | GPU Share (35%) | Model Selected | Expected Latency |
|------------------|-----------|-----------------|----------------|------------------|
| M1 (8GB) | 8GB | ~2.8GB | Install refused | — |
| M1 (16GB) | 16GB | ~5.6GB | 1.1B CoreML | 150-300ms |
| M1 Pro (32GB) | 32GB | ~11.2GB | 1.1B CoreML | 150-250ms |
| M1 Max (64GB) | 64GB | ~22.4GB | 1.1B CoreML | 150-200ms |

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

Edit `~/Library/Application Support/swictation/config.toml`:

```toml
# Force specific model (overrides auto-detection)
stt_model_override = "1.1b-coreml"  # macOS native CoreML (alias "coreml-native")
                                    # "auto" picks this automatically
```

---

## Troubleshooting

### Start here: `swictation doctor`

Before working through the symptom-specific sections below, run `doctor`. It checks every
install step against what is on disk — models, GPU libraries, binaries, config, LaunchAgents,
Accessibility integration — and prints a repair command under each failure. It runs no
install work and writes nothing, so it is safe on a broken install and works even when the
binaries are missing.

```bash
swictation doctor          # health table for every install step
swictation doctor --deep   # ...and verify every model and library by SHA-256, not just size
swictation doctor --json   # machine-readable report (schemaVersion 1)
```

Exit codes: `0` nothing unhealthy, `1` something is unhealthy or blocked, `2` doctor itself
failed to run. Then fix only what it flagged:

```bash
swictation setup --repair     # re-run only the steps that are not healthy
swictation setup --list       # list the install steps and their ids
swictation setup --services   # or run a single step by id
```

Reinstalling the package is no longer the first thing to try — `--repair` re-runs the broken
steps only, so a bad LaunchAgent does not cost you a model re-download.

### "Permission denied" errors

**Cause:** Accessibility permissions not granted

**Fix:**
1. System Settings → Privacy & Security → Accessibility
2. Add the `swictation-daemon` binary (see Step 2 for its location)
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
- Another app might be using Ctrl+Shift+D
- Check System Settings → Keyboard → Keyboard Shortcuts

### CoreML/GPU not working

**Check 1 - Verify CoreML library:**
```bash
# Find the platform package location, then check its lib/ directory
swictation --version          # prints "Location: <platform package dir>"
ls -lh <platform package dir>/lib/libonnxruntime.dylib
# Should show ~30-50MB file
```

**Check 2 - Check logs for GPU usage:**
```bash
grep -i "coreml\|gpu\|metal" ~/Library/Logs/swictation/daemon.log
# Should see: "Enabling CoreML execution provider"
```

**Check 3 - Verify model format:**
- macOS uses the native CoreML bundle (`parakeet-tdt-1.1b-coreml`)
- Check `~/Library/Application Support/swictation/models/`
- Should contain `.mlmodelc` directories (with `model.mil`, `weights/weight.bin`,
  `coremldata.bin`), not `.onnx` files
- Re-fetch with `swictation download-models --model=1.1b-coreml --force`

### Daemon crashes on startup

**Check 1 - Model files corrupted:**
```bash
# Verify every model file against its recorded SHA-256 first
swictation doctor --deep

# Re-fetch only if doctor reports corruption (repairs the models step alone)
swictation setup --models
```

`--deep` names the specific files that fail verification, so you can tell a corrupt model
from a permissions or library problem before re-downloading gigabytes.

**Check 2 - Library compatibility:**
```bash
# Check dylib dependencies
otool -L <platform package dir>/lib/libonnxruntime.dylib
# All paths should exist
```

**Check 3 - Repair the install:**
```bash
swictation doctor          # what is actually broken
swictation setup --repair  # re-run only those steps
```

Reinstall only if `doctor` still reports failures after a repair:
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

Hotkeys are read from `config.toml` at daemon startup. The macOS defaults are
`Ctrl+Shift+D` (toggle) and `Ctrl+Space` (push-to-talk):

```toml
[hotkeys]
toggle = "Ctrl+Shift+D"
push_to_talk = "Ctrl+Space"
```

Restart the daemon after changing them (`swictation stop && swictation start`).

### Silence Detection Tuning

Edit `~/Library/Application Support/swictation/config.toml`:

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

# 5. Remove configuration, data, and models (optional — up to ~2GB of models)
#    On macOS config and data share one directory.
rm -rf ~/Library/Application\ Support/swictation
rm -rf ~/Library/Logs/swictation
rm -rf ~/Library/Caches/swictation

# 6. Remove Accessibility permission
# System Settings → Privacy & Security → Accessibility
# Remove swictation-daemon
```

---

## Known Limitations

### Current Limitations (v0.7.36)

- **Intel Macs not supported** - Apple Silicon (ARM64) required
- **macOS 13 and earlier not supported** - CoreML requirements
- **Tray icon** - provided by the Tauri UI (`com.swictation.ui`)
- **Auto-selection is single-model** - on all supported Macs auto-select runs the
  CoreML 1.1B bundle; the 0.6B CoreML bundle can still be fetched manually with
  `swictation setup --models` / `download-model 0.6b-coreml` if you want it

### Compared to Linux

| Feature | Linux | macOS |
|---------|-------|-------|
| GPU | NVIDIA CUDA | CoreML/Metal |
| Model Format | ONNX FP32/INT8 | Native CoreML bundle |
| Hotkey | Configurable (`Super+Shift+D`) | Configurable (`Ctrl+Shift+D`) |
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
~/Library/Application Support/swictation/config.toml

# Models and data (same directory as config on macOS)
~/Library/Application Support/swictation/models/
~/Library/Application Support/swictation/metrics.db
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
