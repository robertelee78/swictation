# Phase 2 Manual Testing Checklist

## Prerequisites
- [ ] Daemon running: `systemctl --user status swictation.service` OR `ps aux | grep swictationd`
- [ ] Metrics socket exists: `ls -la /tmp/swictation_metrics.sock`
- [ ] Icon file exists: `ls -la /opt/swictation/docs/swictation_logo.png`

## Launch Tray App
```bash
# Start tray app manually
python3 /opt/swictation/src/ui/swictation_tray.py

# Expected output:
# ✓ Connected to metrics socket
# ✓ Swictation tray app started
```

## Visual Tests

### 1. System Tray Icon
- [ ] Icon appears in system tray (check bar where Telegram icon would be)
- [ ] Icon is visible and not hidden
- [ ] Icon shows Swictation logo

### 2. Icon State Changes
- [ ] Icon is default color when idle
- [ ] Single-click icon toggles recording
- [ ] Icon turns red overlay when recording state
- [ ] Icon returns to normal when stopped

### 3. Window Show/Hide
- [ ] Double-click icon opens metrics window
- [ ] Window shows metrics data
- [ ] Close window button hides window (icon stays in tray)
- [ ] Double-click again re-opens window

### 4. Context Menu
- [ ] Right-click icon shows menu
- [ ] Menu has "Show Metrics" option
- [ ] Menu has "Toggle Recording" option
- [ ] Menu has "Quit" option
- [ ] "Quit" actually quits the app

### 5. Real-Time Metrics
- [ ] Connection status shows "Connected to daemon" (green dot)
- [ ] WPM updates during recording
- [ ] Words count increases
- [ ] Latency shows in milliseconds
- [ ] Duration timer increments
- [ ] GPU Memory shows MB value
- [ ] CPU shows percentage

### 6. Socket Connection
- [ ] If daemon stops, connection status shows "Disconnected" (red dot)
- [ ] If daemon restarts, reconnects automatically within 5 seconds
- [ ] Metrics resume updating after reconnect

## Current Status

App is running (PID: 852430) but needs visual confirmation from user.

**Action Required:** Check your system tray for the new Swictation icon!
