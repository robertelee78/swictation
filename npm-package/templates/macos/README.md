# macOS LaunchAgent Templates

This directory contains launchd plist templates for auto-starting Swictation services on macOS.

## Files

- `com.swictation.daemon.plist` - Main daemon service (STT engine, audio capture, hotkeys)
- `com.swictation.ui.plist` - System tray UI (depends on daemon)

## Template Variables

These templates use placeholder variables that must be replaced during installation:

### Daemon Service (`com.swictation.daemon.plist`)

| Variable | Description | Example Value |
|----------|-------------|---------------|
| `{{DAEMON_PATH}}` | Full path to daemon binary | `/usr/local/lib/node_modules/swictation/bin/swictation-daemon-macos` |
| `{{LOG_DIR}}` | Log file directory | `$HOME/.local/share/swictation/logs` or `$HOME/Library/Logs/swictation` |
| `{{ORT_DYLIB_PATH}}` | ONNX Runtime library path (CoreML-enabled) | `/usr/local/lib/node_modules/swictation/lib/native/libonnxruntime.dylib` |
| `{{DYLD_LIBRARY_PATH}}` | Dynamic library search path | `/usr/local/lib/node_modules/swictation/lib/native` |
| `{{HOME}}` | User home directory | `/Users/username` |

### UI Service (`com.swictation.ui.plist`)

| Variable | Description | Example Value |
|----------|-------------|---------------|
| `{{UI_PATH}}` | Full path to UI binary | `/usr/local/lib/node_modules/swictation/bin/swictation-ui-macos` |
| `{{LOG_DIR}}` | Log file directory | `$HOME/.local/share/swictation/logs` or `$HOME/Library/Logs/swictation` |
| `{{HOME}}` | User home directory | `/Users/username` |

## Installation Process

1. **Replace template variables** in plist files with actual paths
2. **Copy to LaunchAgents directory**:
   ```bash
   cp com.swictation.daemon.plist ~/Library/LaunchAgents/
   cp com.swictation.ui.plist ~/Library/LaunchAgents/
   ```
3. **Set permissions** (user read/write only):
   ```bash
   chmod 644 ~/Library/LaunchAgents/com.swictation.daemon.plist
   chmod 644 ~/Library/LaunchAgents/com.swictation.ui.plist
   ```
4. **Load services**:
   ```bash
   launchctl load ~/Library/LaunchAgents/com.swictation.daemon.plist
   launchctl load ~/Library/LaunchAgents/com.swictation.ui.plist
   ```

## Service Management

### Start Services
```bash
launchctl start com.swictation.daemon
launchctl start com.swictation.ui
```

### Stop Services
```bash
launchctl stop com.swictation.ui
launchctl stop com.swictation.daemon
```

### Check Status
```bash
launchctl list | grep swictation
```

### View Logs
```bash
# Daemon logs
tail -f ~/Library/Logs/swictation/daemon.log
tail -f ~/Library/Logs/swictation/daemon-error.log

# UI logs
tail -f ~/Library/Logs/swictation/ui.log
tail -f ~/Library/Logs/swictation/ui-error.log
```

### Unload Services (Disable Auto-Start)
```bash
launchctl unload ~/Library/LaunchAgents/com.swictation.ui.plist
launchctl unload ~/Library/LaunchAgents/com.swictation.daemon.plist
```

### Remove Services Completely
```bash
launchctl unload ~/Library/LaunchAgents/com.swictation.ui.plist
launchctl unload ~/Library/LaunchAgents/com.swictation.daemon.plist
rm ~/Library/LaunchAgents/com.swictation.daemon.plist
rm ~/Library/LaunchAgents/com.swictation.ui.plist
```

## Key Features

### Daemon Service
- **Auto-start on login**: `RunAtLoad=true`
- **Restart on crash**: `KeepAlive` with crash detection
- **Throttled restarts**: 5-second delay between restart attempts (prevents crash loops)
- **GPU acceleration**: CoreML-enabled ONNX Runtime via `ORT_DYLIB_PATH` and `DYLD_LIBRARY_PATH`
- **Resource limits**: 4096 file descriptors (soft), 8192 (hard)
- **Session type**: Aqua (GUI session) for CoreML/Metal/audio access
- **Comprehensive logging**: Separate stdout and stderr logs

### UI Service
- **Depends on daemon**: Only runs when daemon is active (`OtherJobEnabled`)
- **Restart on crash**: Automatic restart with throttling
- **Resource limits**: 100MB memory (soft), 200MB (hard), 256/512 file descriptors
- **Interactive process**: GUI application with proper session access
- **Lightweight**: Lower priority than daemon (`Nice=5`)

## Troubleshooting

### Services Won't Start
1. Check logs for errors
2. Verify binary paths are correct
3. Ensure binaries are executable (`chmod +x`)
4. Check Accessibility permissions (System Settings → Privacy & Security → Accessibility)

### Services Crash Immediately
1. Check `daemon-error.log` and `ui-error.log`
2. Verify ONNX Runtime library exists at `{{ORT_DYLIB_PATH}}`
3. Ensure CoreML is available (Apple Silicon required)
4. Check environment variables are set correctly

### GPU Acceleration Not Working
1. Verify `ORT_DYLIB_PATH` points to CoreML-enabled library
2. Check `DYLD_LIBRARY_PATH` includes library directory
3. Ensure running on Apple Silicon (M1/M2/M3/M4)
4. Check Activity Monitor for GPU usage during inference

## Differences from Linux systemd

| Feature | Linux (systemd) | macOS (launchd) |
|---------|----------------|-----------------|
| **GPU provider** | CUDA (nvidia-smi) | CoreML (Metal) |
| **Library path** | `LD_LIBRARY_PATH` | `DYLD_LIBRARY_PATH` |
| **Service location** | `~/.config/systemd/user/` | `~/Library/LaunchAgents/` |
| **Start command** | `systemctl --user start` | `launchctl start` |
| **Session type** | `After=graphical-session.target` | `LimitLoadToSessionType=Aqua` |
| **Dependency** | `Requires=`, `After=` | `OtherJobEnabled` |
| **Resource limits** | `MemoryMax=`, `CPUQuota=` | `SoftResourceLimits`, `HardResourceLimits` |

## References

- [Apple launchd.plist man page](https://www.manpagez.com/man/5/launchd.plist/)
- [launchctl man page](https://www.manpagez.com/man/1/launchctl/)
- [Creating Launch Daemons and Agents](https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/CreatingLaunchdJobs.html)
