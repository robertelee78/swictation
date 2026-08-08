# Swictation macOS Setup Guide

This guide covers setting up Swictation on macOS with Apple Silicon (M1 or later).

## Requirements

- **macOS 14.0** (Sonoma) or later — postinstall refuses older versions
- **Apple Silicon** (M1 or later)
- **16GB+ unified memory** — a hard requirement; postinstall aborts below it
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
4. Download speech recognition models (~1.9GB: Silero VAD + CoreML 1.1B bundle)

## Required: Accessibility Permission

**This step is mandatory for text injection to work.**

macOS requires explicit permission for applications to simulate keyboard input. Without this permission, speech recognition will work but text will NOT be typed into applications.

### Steps to Enable Accessibility

1. **Open System Settings** (or System Preferences on older macOS)

2. **Navigate to**: Privacy & Security → Accessibility

3. **Click the lock icon** (bottom left) and enter your password

4. **Click the + button** to add an application

5. **Navigate to the swictation-daemon binary**, which lives in the platform
   package, not the main package:
   - Run `swictation --version` and read the "Location" line
   - It is normally `<npm-prefix>/lib/node_modules/@agidreams/darwin-arm64/bin/swictation-daemon`

6. **Enable the checkbox** for swictation-daemon

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

The tray UI runs as its own LaunchAgent, installed alongside the daemon:

```bash
launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/com.swictation.ui.plist
launchctl start com.swictation.ui
```

`swictation start --ui` starts the daemon and the tray together.

## Hotkey Configuration

The default hotkey is **Ctrl+Shift+D** to toggle dictation.

To customize, edit `~/Library/Application Support/swictation/config.toml`:

```toml
[hotkeys]
toggle = "Ctrl+Shift+D"
```

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

1. Check the ONNX Runtime library exists in the platform package
   (`swictation --version` prints the location):
   ```bash
   ls -la <platform package dir>/lib/libonnxruntime.dylib
   ```

2. Verify models are downloaded:
   ```bash
   ls -la ~/Library/Application\ Support/swictation/models/
   ```

3. Check the launcher's own trace of what it resolved:
   ```bash
   tail -n 20 ~/Library/Logs/swictation/launcher.log
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

# Remove data files, config, and models (optional)
# On macOS config and data share one directory.
rm -rf ~/Library/Application\ Support/swictation
rm -rf ~/Library/Logs/swictation
rm -rf ~/Library/Caches/swictation
rm -f ~/Library/LaunchAgents/com.swictation.daemon.plist
rm -f ~/Library/LaunchAgents/com.swictation.ui.plist
```

## Support

- **GitHub Issues**: https://github.com/robertelee78/swictation/issues
- **Documentation**: https://github.com/robertelee78/swictation/tree/main/docs
