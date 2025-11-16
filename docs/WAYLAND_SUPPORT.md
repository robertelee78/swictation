# Wayland Support Guide

Swictation fully supports modern Wayland-based Linux desktops, including GNOME and Sway.

## Automated Setup

The npm package automatically detects and configures Wayland support during installation:

### ✅ What's Automated

1. **ydotool Installation & Setup**
   - Installs ydotool for text injection
   - Loads uinput kernel module
   - Configures udev rules for /dev/uinput permissions
   - Adds user to `input` group

2. **GNOME Keyboard Shortcuts** (GNOME only)
   - Auto-configures Super+Shift+D hotkey
   - Sets up IPC socket communication
   - Registers custom keyboard shortcut in GNOME Settings

3. **GPU Acceleration**
   - Downloads CUDA + cuDNN libraries (2.3GB)
   - Configures LD_LIBRARY_PATH for systemd service
   - Enables GPU-accelerated speech recognition

4. **Systemd Service**
   - Generates service file with correct environment variables
   - Auto-enables service (starts on login)
   - Attempts to start service immediately

## GNOME Wayland (Ubuntu 25.10+)

### Installation
```bash
npm install -g swictation
# Automated setup runs during installation:
# ✓ ydotool installed
# ✓ Kernel modules configured
# ✓ GNOME shortcuts configured
# ✓ Service enabled and started
```

### Hotkeys
- **Super+Shift+D**: Toggle recording (auto-configured)
- Visible in: Settings → Keyboard → View and Customize Shortcuts

### Manual Configuration (if needed)
```bash
# Run setup scripts individually:
./node_modules/swictation/scripts/setup-ydotool.sh
./node_modules/swictation/scripts/setup-gnome-shortcuts.sh

# Or verify installation:
./node_modules/swictation/scripts/verify-installation.sh
```

## Sway Wayland

### Installation
```bash
npm install -g swictation
# Automated setup runs during installation:
# ✓ ydotool installed and configured
# ✓ Service enabled
```

### Hotkey Configuration (Manual)
Add to `~/.config/sway/config`:

```bash
# Swictation voice-to-text hotkeys
bindsym $mod+Shift+d exec sh -c 'echo "{\"action\": \"toggle\"}" | nc -U /tmp/swictation.sock'

# Optional: Push-to-talk
bindsym $mod+Space exec sh -c 'echo "{\"action\": \"ptt_press\"}" | nc -U /tmp/swictation.sock'
bindsym --release $mod+Space exec sh -c 'echo "{\"action\": \"ptt_release\"}" | nc -U /tmp/swictation.sock'
```

Reload Sway:
```bash
swaymsg reload
```

## Requirements

### All Wayland Compositors
- **ydotool**: Text injection (auto-installed on Ubuntu/Debian)
- **netcat**: IPC communication (auto-installed)
- **uinput kernel module**: Device emulation (auto-loaded)
- **input group**: User permissions (auto-configured)

### GNOME-Specific
- **gsettings**: Keyboard shortcut configuration (pre-installed)
- **GNOME Shell 45+**: Custom keyboard shortcuts support

### Sway-Specific
- **Sway 1.8+**: IPC support (pre-installed)
- **SWAYSOCK**: Environment variable (auto-detected)

## Wayland Security Model

Wayland's security model prevents applications from capturing global hotkeys directly. Swictation handles this by:

1. **GNOME**: Uses GNOME's custom keyboard shortcuts system (auto-configured)
2. **Sway**: Uses Sway's config-based bindings (manual configuration)
3. **Other**: Requires compositor-specific configuration

## Verification

Check installation status:
```bash
./node_modules/swictation/scripts/verify-installation.sh
```

Expected output:
```
✓ Wayland detected
✓ GNOME Wayland detected
✓ NVIDIA GPU detected
✓ GPU libraries installed
✓ ydotool installed
✓ uinput module loaded
✓ /dev/uinput permissions correct
✓ User in 'input' group
✓ GNOME keyboard shortcut configured
✓ Service file exists
✓ Service enabled
✓ Service running
✓ IPC socket responding
```

## Troubleshooting

### Text not typing
**Symptom**: Hotkey toggles recording but text doesn't appear

**Solution**:
```bash
# 1. Check ydotool access
sg input -c 'ydotool type "test"'

# 2. Verify group membership (may require logout/login)
groups | grep input

# 3. Check uinput permissions
ls -l /dev/uinput
# Should show: crw-rw---- 1 root input

# 4. Reload udev if needed
sudo udevadm control --reload-rules
sudo udevadm trigger
```

### Hotkeys not working (GNOME)
**Symptom**: Super+Shift+D types "D" instead of toggling

**Solution**:
```bash
# 1. Check GNOME shortcut configuration
gsettings get org.gnome.settings-daemon.plugins.media-keys custom-keybindings

# 2. Reconfigure shortcuts
./scripts/setup-gnome-shortcuts.sh

# 3. Verify in Settings
# Settings → Keyboard → View and Customize Shortcuts
# Look for "Swictation Toggle"
```

### Slow transcription (CPU instead of GPU)
**Symptom**: Transcription takes 20-30 seconds instead of <1 second

**Solution**:
```bash
# 1. Check GPU libraries
ls -lh ~/.local/share/swictation/gpu-libs/libcudnn.so.9

# 2. Restart daemon via systemctl (not manually!)
systemctl --user restart swictation-daemon.service

# 3. Check logs for GPU detection
journalctl --user -u swictation-daemon.service | grep CUDA

# Expected:
# "Successfully registered `CUDAExecutionProvider`"
# "cuDNN version: 91501"
```

### Permission denied errors
**Symptom**: `failed to open uinput device`

**Solution**:
```bash
# 1. Add user to input group
sudo usermod -aG input $USER

# 2. Logout/login or force group change
newgrp input

# 3. Test ydotool
sg input -c 'ydotool type "test"'
```

## Architecture

```
Wayland Compositor (GNOME/Sway)
    ↓
GNOME: Custom Keyboard Shortcut
Sway: Config-based binding
    ↓
IPC Command: {"action": "toggle"}
    ↓
/tmp/swictation.sock
    ↓
Swictation Daemon
    ↓
ydotool (via /dev/uinput)
    ↓
Text appears in focused app
```

## Performance

With GPU acceleration (properly configured):
- **Transcription**: < 1 second
- **Text injection**: Instant
- **Hotkey response**: < 100ms

Without GPU (CPU fallback):
- **Transcription**: 20-30 seconds
- **Text injection**: Instant
- **Hotkey response**: < 100ms

**Always ensure systemd service is used** (not manual daemon start) for proper GPU configuration!

## Additional Resources

- [Setup Scripts](/scripts/): Automated configuration tools
- [Verification Script](/scripts/verify-installation.sh): Post-install checks
- [GitHub Issues](https://github.com/robertelee78/swictation/issues): Report problems
- [Ubuntu 25.10 Guide](./UBUNTU_25_10.md): Ubuntu-specific instructions
