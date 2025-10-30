# Sway Integration Guide

Complete guide for integrating Swictation with Sway window manager.

## Quick Setup

### Automated Setup (Recommended)

```bash
# Run the setup script
sudo /opt/swictation/scripts/setup-sway.sh
```

This will:
1. ✅ Backup your Sway config
2. ✅ Add keybinding (`Mod1+Shift+d`)
3. ✅ (Optional) Add daemon autostart
4. ✅ Provide reload instructions

### Manual Setup

If you prefer manual configuration:

#### 1. Add Keybinding to Sway Config

Edit `~/.config/sway/config`:

```bash
# Swictation voice dictation
bindsym Mod1+Shift+d exec python3 /opt/swictation/src/swictation_cli.py toggle
```

Or include the config file:

```bash
include /opt/swictation/config/sway-keybinding.conf
```

#### 2. (Optional) Autostart Daemon

Add to `~/.config/sway/config`:

```bash
exec python3 /opt/swictation/src/swictationd.py
```

Or use systemd (recommended for better management):

```bash
# Copy service file
cp /opt/swictation/config/swictation.service ~/.config/systemd/user/

# Enable and start
systemctl --user enable swictation.service
systemctl --user start swictation.service
```

#### 3. Reload Sway

```bash
swaymsg reload
```

## Keybinding Details

### Default Keybinding

- **Mod1+Shift+d** (Alt+Shift+d)
  - First press: Start recording
  - Second press: Stop recording and transcribe
  - Transcribed text is injected at cursor

### Customization

You can change the keybinding in your Sway config:

```bash
# Use a different modifier
bindsym Mod4+Shift+d exec python3 /opt/swictation/src/swictation_cli.py toggle

# Use F-keys
bindsym F9 exec python3 /opt/swictation/src/swictation_cli.py toggle

# Add visual feedback with notify-send
bindsym Mod1+Shift+d exec python3 /opt/swictation/src/swictation_cli.py toggle && \
    notify-send "Swictation" "Recording toggled"
```

### Multiple Keybindings

You can add additional bindings for other functions:

```bash
# Status check
bindsym Mod1+Shift+s exec notify-send "Swictation" \
    "$(python3 /opt/swictation/src/swictation_cli.py status)"

# Stop daemon
bindsym Mod1+Shift+q exec python3 /opt/swictation/src/swictation_cli.py stop
```

## Testing

### Test Keybinding Without Sway

```bash
# Run the test script
/opt/swictation/scripts/test-keybinding.sh
```

### Test in Sway

1. Ensure daemon is running:
   ```bash
   python3 /opt/swictation/src/swictationd.py
   ```

2. Open any text editor (kate, gedit, vim, etc.)

3. Focus the editor window

4. Press `Alt+Shift+d` and speak

5. Press `Alt+Shift+d` again to stop

6. Text should appear at cursor

## Troubleshooting

### Keybinding Not Working

**Check if keybinding is registered:**
```bash
swaymsg -t get_binding_state
```

**Check Sway config syntax:**
```bash
sway --validate
```

**Reload Sway config:**
```bash
swaymsg reload
```

### Daemon Not Starting

**Check if daemon is running:**
```bash
python3 /opt/swictation/src/swictation_cli.py status
```

**Start daemon manually:**
```bash
python3 /opt/swictation/src/swictationd.py
```

**Check logs (if using systemd):**
```bash
journalctl --user -u swictation.service -f
```

### No Text Injection

**Verify wtype is working:**
```bash
echo "test" | wtype -
```

**Check focused window accepts text input:**
- Some windows (like terminals with tmux) may need special handling
- Try in a simple text editor first

**Check permissions:**
- Ensure Wayland socket is accessible
- Verify `$WAYLAND_DISPLAY` environment variable

### Audio Not Captured

**List audio devices:**
```bash
python3 -c "from src.audio_capture import AudioCapture; AudioCapture().list_devices()"
```

**Test audio capture:**
```bash
python3 /opt/swictation/src/audio_capture.py
```

**Check PipeWire/PulseAudio:**
```bash
pactl list sources short
```

## Advanced Configuration

### Visual Feedback with notify-send

```bash
bindsym Mod1+Shift+d exec bash -c '\
    python3 /opt/swictation/src/swictation_cli.py toggle && \
    STATE=$(python3 /opt/swictation/src/swictation_cli.py status | grep "state:" | cut -d: -f2) && \
    if [ "$STATE" = "recording" ]; then \
        notify-send -u low "Swictation" "🎤 Recording..." -t 2000; \
    else \
        notify-send -u low "Swictation" "✓ Processing" -t 2000; \
    fi'
```

### Audio Feedback with paplay

```bash
# Play sound on recording start/stop
bindsym Mod1+Shift+d exec bash -c '\
    python3 /opt/swictation/src/swictation_cli.py toggle && \
    paplay /usr/share/sounds/freedesktop/stereo/message.oga'
```

### Mode-based Recording

```bash
# Define a "dictation" mode
mode "dictation" {
    bindsym d exec python3 /opt/swictation/src/swictation_cli.py toggle
    bindsym s exec python3 /opt/swictation/src/swictation_cli.py status
    bindsym Escape mode "default"
}

# Enter dictation mode
bindsym Mod1+Shift+m mode "dictation"
```

## Integration with Other Tools

### i3status-rust

Add Swictation status to your status bar:

```toml
[[block]]
block = "custom"
command = "python3 /opt/swictation/src/swictation_cli.py status | grep 'state:' | cut -d: -f2"
interval = 2
```

### waybar

```json
{
    "custom/swictation": {
        "exec": "python3 /opt/swictation/src/swictation_cli.py status 2>/dev/null | grep -q 'recording' && echo '🎤' || echo '⏸️'",
        "interval": 1,
        "on-click": "python3 /opt/swictation/src/swictation_cli.py toggle"
    }
}
```

## See Also

- [Swictation README](../README.md)
- [Configuration Guide](configuration.md)
- [Troubleshooting Guide](troubleshooting.md)
- [Sway Documentation](https://github.com/swaywm/sway/wiki)
