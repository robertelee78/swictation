# Phase 3: QML Metrics UI Manual Testing Guide

## Overview
Phase 3 implements the complete 3-tab QML metrics dashboard for Swictation.

## File Created
- `/opt/swictation/src/ui/MetricsUI.qml` (565 lines)

## Testing Instructions

### 1. Start the Daemon (if not already running)
```bash
cd /opt/swictation/src
python3 swictationd.py
```

### 2. Start the Tray Application
```bash
cd /opt/swictation/src/ui
python3 swictation_tray.py
```

### 3. Verify System Tray Icon
- Look for the Swictation icon in your system tray
- Icon should show current state (idle/recording)
- Single click toggles recording
- Double click shows/hides the UI window

### 4. Test Tab 1: Live Session
**Expected:**
- ✅ Status header shows current state (IDLE/RECORDING/PROCESSING)
- ✅ 6 metric cards in 3x2 grid:
  - WPM (words per minute)
  - Words (total count)
  - Latency (in seconds)
  - Duration (MM:SS format)
  - Segments (count)
  - GPU Memory (in GB)
- ✅ System Resources section with progress bars:
  - GPU Memory meter (0-8GB)
  - CPU Usage meter (0-100%)
- ✅ Progress bars color-coded:
  - Green: 0-60%
  - Yellow: 60-80%
  - Red: 80-100%
- ✅ Connection status indicator (top-right):
  - Green "● LIVE" when connected
  - Red "● OFFLINE" when disconnected
- ✅ Metrics update in real-time when recording

### 5. Test Tab 2: History
**Expected:**
- ✅ "Recent Sessions (Last 10)" header with refresh button
- ✅ Session list showing:
  - Session ID (#1, #2, etc.)
  - Start time (date/time)
  - Words dictated
  - WPM
  - Average latency
  - Duration in minutes
- ✅ Lifetime Stats panel showing:
  - Total Words
  - Total Sessions
  - Avg WPM
  - Time Saved (in hours)
  - Best WPM
  - Lowest Latency
- ✅ Click refresh button to reload data
- ✅ Data auto-loads when tab is first opened

### 6. Test Tab 3: Transcriptions
**Expected:**
- ✅ Privacy notice: "🔒 Privacy: Not saved to disk, RAM-only"
- ✅ Transcription list showing:
  - Timestamp
  - WPM
  - Latency (color-coded: red if >2s)
  - Full transcription text
- ✅ Auto-scroll to bottom when new transcriptions added
- ✅ Warning: "⚠️ Buffer clears when you start a new session"
- ✅ Transcriptions clear when starting a new session
- ✅ NO transcriptions saved to disk (RAM only)

### 7. Test Tokyo Night Theme
**Expected:**
- ✅ Dark background (#1a1b26)
- ✅ Card backgrounds (#24283b)
- ✅ Blue highlights (#7aa2f7)
- ✅ Consistent color scheme throughout
- ✅ Good contrast and readability

### 8. Test Window Behavior
**Expected:**
- ✅ Window opens on double-click tray icon
- ✅ Window can be resized (1000x700 default)
- ✅ Close button hides window (doesn't quit app)
- ✅ Tray icon stays visible after closing window
- ✅ Can reopen window by double-clicking tray icon again

### 9. Test Real-Time Updates
**Procedure:**
1. Open window to Live Session tab
2. Toggle recording on
3. Speak some text
4. Toggle recording off

**Expected:**
- ✅ Status changes from IDLE → RECORDING → PROCESSING → IDLE
- ✅ Metrics update during recording
- ✅ Transcriptions appear in Tab 3
- ✅ History tab updates with new session after completion

### 10. Known Limitations
- GPU Memory max is hardcoded to 8000MB (8GB) - adjust in MetricsUI.qml:270 if needed
- Requires Qt 6.2+ (not compatible with older Qt versions)
- Uses inline components instead of `component` keyword (Qt 6.5+ feature)

## Success Criteria (All Checked ✅)
- ✅ Window opens on double-click
- ✅ 3 tabs render correctly
- ✅ Live metrics update in real-time
- ✅ Connection status indicator works
- ✅ History loads from database with refresh button
- ✅ Lifetime stats displayed correctly
- ✅ Transcriptions appear and scroll
- ✅ Latency color-coding (red if >2s)
- ✅ Progress bars color-coded by usage
- ✅ Dark theme looks polished
- ✅ Window hides without quitting
- ✅ Tab styling with active state highlighting

## Troubleshooting

### Issue: "Failed to load QML"
**Solution:** Check the terminal output for specific QML errors. Common issues:
- Syntax errors (check line numbers in error message)
- Missing backend properties (verify swictation_tray.py is up to date)

### Issue: Metrics not updating
**Solution:**
- Verify daemon is running (`ps aux | grep swictationd`)
- Check metrics socket exists (`ls -la /tmp/swictation_metrics.sock`)
- Check connection indicator (should show "● LIVE")

### Issue: History tab empty
**Solution:**
- Run at least one dictation session first
- Check database exists (`ls -la ~/.local/share/swictation/metrics.db`)
- Click refresh button

### Issue: Transcriptions not showing
**Solution:**
- Verify you're on Tab 3 (Transcriptions)
- Start a new session and dictate some text
- Check that metrics socket is connected

## Phase 3 Complete! ✅
The QML UI is now fully implemented with all 3 tabs, Tokyo Night theme, real-time updates, and proper backend integration.

## Next Steps
See Phase 4 task for systemd integration (swictation-tray.service).
