# Swictation System Tray Application

## Overview

The Swictation tray app provides a persistent system tray icon (like Telegram) with a metrics dashboard.

## Features

- **Persistent System Tray Icon**: Always visible in your system tray
- **Visual State Indicator**: Icon changes color when recording (red overlay)
- **Quick Actions**:
  - Single click: Toggle recording ON/OFF
  - Double click: Show/hide metrics window
  - Right-click: Context menu
- **Real-Time Metrics Dashboard**: Live WPM, latency, GPU/CPU usage
- **Dual Data Source**:
  - Socket: Real-time metrics from daemon
  - Database: Historical session data

## Installation

### Prerequisites

```bash
# Install PySide6 (Qt6 Python bindings)
pip install PySide6
```

### Running the Tray App

```bash
# Manual start
python3 /opt/swictation/src/ui/swictation_tray.py

# Or with systemd (Phase 4)
systemctl --user start swictation-tray.service
```

## Usage

### System Tray Icon

The tray icon shows the current recording state:

- **Default** (white/color): Idle, not recording
- **Red overlay**: Recording active

### Controls

1. **Single Click**: Toggle recording
   - Click once to start recording
   - Click again to stop recording
   - Same as `swictation toggle` command

2. **Double Click**: Show/hide metrics window
   - Opens the metrics dashboard
   - Close window to hide (icon stays in tray)

3. **Right-Click**: Context menu
   - Show Metrics
   - Toggle Recording
   - Quit

### Metrics Window

The metrics window displays (Phase 3 will expand this):

- **Live Session Tab**:
  - Current state (idle/recording)
  - Words Per Minute (WPM)
  - Total words spoken
  - Average latency (ms)
  - Segment count
  - Session duration
  - GPU memory usage
  - CPU usage

- **History Tab** (coming in Phase 3):
  - Last 10 sessions
  - Lifetime statistics

- **Transcriptions Tab** (coming in Phase 3):
  - Ephemeral transcriptions (RAM only)
  - Clears on new session

## Architecture

### Components

```
swictationd.py (daemon)
  ├─ MetricsBroadcaster
  │  └─ /tmp/swictation_metrics.sock (real-time events)
  └─ MetricsDatabase
     └─ ~/.local/share/swictation/metrics.db (historical data)

swictation_tray.py (tray app)
  ├─ QSystemTrayIcon (persistent icon)
  ├─ MetricsBackend
  │  ├─ Socket client (real-time updates)
  │  └─ Database client (historical queries)
  └─ MetricsUI.qml (dashboard window)
```

### Event Types

The daemon broadcasts these events via socket:

1. **session_start**: New recording session started
   ```json
   {"type": "session_start", "timestamp": "2025-11-03T02:00:00"}
   ```

2. **session_end**: Recording session ended
   ```json
   {"type": "session_end", "timestamp": "2025-11-03T02:05:30"}
   ```

3. **transcription**: New transcription available
   ```json
   {
     "type": "transcription",
     "text": "hello world",
     "timestamp": "2025-11-03T02:01:15",
     "wpm": 125.5,
     "latency_ms": 142.3
   }
   ```

4. **metrics_update**: Real-time metrics (every second)
   ```json
   {
     "type": "metrics_update",
     "state": "recording",
     "wpm": 150.5,
     "words": 100,
     "latency_ms": 125.3,
     "segments": 5,
     "duration_s": 65,
     "gpu_memory_mb": 512.0,
     "cpu_percent": 45.2
   }
   ```

5. **state_change**: Daemon state changed
   ```json
   {"type": "state_change", "state": "idle"}
   ```

## Testing

```bash
# Run backend logic tests (no display required)
python3 /opt/swictation/tests/test_tray_backend.py

# Expected output:
# ============================================================
# Tray Backend Unit Tests (No Display Required)
# ============================================================
#
# test_event_parsing: ✓ PASSED
# test_socket_protocol: ✓ PASSED
# test_icon_state_logic: ✓ PASSED
# test_database_compatibility: ✓ PASSED
#
# Results: 4 passed, 0 failed
```

## Troubleshooting

### Icon not appearing

1. Ensure you're running a desktop environment with system tray support
2. Check if daemon is running: `systemctl --user status swictation.service`
3. Check tray app logs

### Metrics not updating

1. Check socket connection status in UI (indicator shows red if disconnected)
2. Verify daemon is broadcasting: `nc -U /tmp/swictation_metrics.sock`
3. Check daemon logs: `journalctl --user -u swictation.service -f`

### Window not opening

1. Check QML file exists: `/opt/swictation/src/ui/MetricsUI.qml`
2. Check for Qt/QML errors in console output
3. Ensure PySide6 is installed correctly: `python3 -c "import PySide6.QtQml"`

## Files

- `/opt/swictation/src/ui/swictation_tray.py` - Main tray application
- `/opt/swictation/src/ui/MetricsUI.qml` - Metrics dashboard UI
- `/opt/swictation/docs/swictation_logo.png` - Tray icon
- `/opt/swictation/tests/test_tray_backend.py` - Unit tests

## Next Steps (Phase 3)

- Full QML UI with 3 tabs
- History tab with last 10 sessions
- Transcriptions tab with ephemeral text
- Tokyo Night dark theme
- Charts and visualizations
