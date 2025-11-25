# Swictation macOS Setup Guide

This guide covers setting up Swictation on macOS with Apple Silicon (M1/M2/M3/M4).

## Requirements

- **macOS 13.0** (Ventura) or later
- **Apple Silicon** (M1, M2, M3, M4 series)
- **Node.js 18+** for installation

> **Note**: Intel Macs are not supported. Swictation requires Apple's Neural Engine for efficient speech recognition.

## Installation

```bash
npm install -g swictation
```

The postinstall script will automatically:
1. Download the macOS ARM64 daemon binary
2. Download the ONNX Runtime library with CoreML support
3. Download the Swictation UI application
4. Download speech recognition models (~1.1GB)

## Required: Accessibility Permission

**This step is mandatory for text injection to work.**

macOS requires explicit permission for applications to simulate keyboard input. Without this permission, speech recognition will work but text will NOT be typed into applications.

### Steps to Enable Accessibility

1. **Open System Settings** (or System Preferences on older macOS)

2. **Navigate to**: Privacy & Security → Accessibility

3. **Click the lock icon** (bottom left) and enter your password

4. **Click the + button** to add an application

5. **Navigate to the swictation-daemon binary**:
   - If installed globally with npm: `~/.npm-global/lib/node_modules/swictation/bin/swictation-daemon-macos`
   - Or check: `which swictation` to find the npm install location

6. **Enable the checkbox** for swictation-daemon-macos

### Verifying Permission

After granting permission, restart the daemon:

```bash
swictation stop
swictation start
```

Check the logs for any permission errors:
```bash
tail -f ~/Library/Logs/swictation/daemon.log
```

If you see "Failed to initialize text injector", the permission was not granted correctly.

## Optional: Microphone Permission

If you're using Swictation through Terminal or another app that needs microphone access:

1. **Open System Settings** → Privacy & Security → Microphone
2. Enable microphone access for Terminal (or your preferred terminal app)

## Starting Swictation

### Manual Start
```bash
swictation start
```

### Auto-start on Login
```bash
# Load the launch agent
launchctl load ~/Library/LaunchAgents/com.swictation.daemon.plist

# Verify it's loaded
launchctl list | grep swictation
```

### Using the UI Tray App

After installation, the Swictation UI app is available at:
- `~/Applications/Swictation.app`

You can add this to your Login Items for auto-start:
1. Open System Settings → General → Login Items
2. Click + and add Swictation.app

## Hotkey Configuration

The default hotkey is **Cmd+Shift+D** (⌘⇧D) to toggle dictation.

To customize, edit `~/.config/swictation/config.toml`:

```toml
[hotkey]
enabled = true
modifiers = ["super", "shift"]  # Cmd+Shift
key = "d"
```

Available modifiers: `super` (Cmd), `shift`, `alt` (Option), `ctrl`

## GPU Acceleration

Swictation automatically uses Apple's CoreML with Neural Engine acceleration. No configuration needed.

The 1.1B Parakeet-TDT model runs efficiently on Apple Silicon using:
- **Neural Engine** (ANE) for transformer operations
- **Metal GPU** for matrix operations
- **CPU** as fallback

You can verify GPU usage in Activity Monitor under "GPU History".

## Troubleshooting

### Text Not Being Typed

1. **Check Accessibility permission** (most common issue)
2. Verify daemon is running: `ps aux | grep swictation-daemon`
3. Check logs: `tail -f ~/Library/Logs/swictation/daemon.log`

### Daemon Not Starting

1. Check ONNX Runtime library exists:
   ```bash
   ls -la ~/.local/share/swictation/native/libonnxruntime.dylib
   ```

2. Verify models are downloaded:
   ```bash
   ls -la ~/.local/share/swictation/models/
   ```

3. Check for library issues:
   ```bash
   ORT_DYLIB_PATH=~/.local/share/swictation/native/libonnxruntime.dylib swictation-daemon-macos
   ```

### Hotkey Not Working

1. Check if another app is using the same hotkey
2. Verify hotkey service is enabled in config.toml
3. Try a different key combination

### Poor Recognition Quality

1. Check microphone input level in System Settings → Sound → Input
2. Ensure you're in a quiet environment
3. Speak clearly at a normal pace

## Uninstalling

```bash
# Stop the daemon
swictation stop

# Unload launch agent
launchctl unload ~/Library/LaunchAgents/com.swictation.daemon.plist

# Remove npm package
npm uninstall -g swictation

# Remove data files (optional)
rm -rf ~/.local/share/swictation
rm -rf ~/.config/swictation
rm ~/Library/LaunchAgents/com.swictation.daemon.plist
rm ~/Library/Logs/swictation
rm -rf ~/Applications/Swictation.app
```

## Support

- **GitHub Issues**: https://github.com/robertelee78/swictation/issues
- **Documentation**: https://github.com/robertelee78/swictation/tree/main/docs
