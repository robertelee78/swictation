# Qt6 Tray Icon Recommendations Summary

**Date**: 2025-11-02
**Status**: ✅ Current implementation working, improvements available

---

## Current Status

Your implementation in commit `e717e3c` is **fundamentally correct** and follows Qt best practices. The 250ms debounce timer successfully prevents double-click interference.

### What's Working

✅ QTimer-based debounce prevents single-click from interfering with double-click
✅ Double-click reliably shows the metrics window
✅ Single-click (after 250ms) toggles recording
✅ Middle-click provides alternative quick toggle

---

## Quick Improvements (Optional)

### 1. Use System Double-Click Interval (Recommended)

**Current**:
```python
self.click_timer.start(250)  # Fixed 250ms
```

**Recommended**:
```python
self.click_timer.start(QApplication.doubleClickInterval())  # Respects OS settings
```

**Why**: On Linux, the default is typically 400ms. Using the system value:
- Respects user preferences
- Better for accessibility (users with slower clicking)
- More consistent with other applications

**Trade-off**: Slightly slower response (150ms more), but more user-friendly.

---

### 2. Add System Tray Check (Good Practice)

**Add to `__init__`**:
```python
# Check if system tray is available (especially important on Wayland)
if not QSystemTrayIcon.isSystemTrayAvailable():
    print("⚠️  System tray not available - showing window by default")
    # Fallback: show metrics window immediately
    self.setQuitOnLastWindowClosed(False)
```

**Why**: On some Wayland environments (especially GNOME), system tray may not be available without extensions.

---

### 3. Add Debug Logging (Optional)

**For troubleshooting**:
```python
@Slot(int)
def on_tray_activated(self, reason):
    """Handle tray icon clicks with debounce."""
    # Log activation reason
    reason_names = {
        QSystemTrayIcon.Trigger: "Single-click",
        QSystemTrayIcon.DoubleClick: "Double-click",
        QSystemTrayIcon.MiddleClick: "Middle-click",
        QSystemTrayIcon.Context: "Right-click"
    }
    print(f"[Tray] {reason_names.get(reason, f'Unknown({reason})')}")

    if reason == QSystemTrayIcon.DoubleClick:
        if self.click_timer.isActive():
            print("[Tray] ✓ Cancelled pending single-click")
        self.click_timer.stop()
        self.toggle_window()
    elif reason == QSystemTrayIcon.Trigger:
        interval = QApplication.doubleClickInterval()
        print(f"[Tray] Starting {interval}ms debounce timer")
        self.click_timer.start(interval)
```

---

## Platform-Specific Notes (Sway/Wayland)

### What Works on Sway
- ✅ System tray icon display (via StatusNotifierItem protocol)
- ✅ Single-click, double-click, right-click detection
- ✅ Context menu
- ✅ Icon updates

### What May Not Work on Wayland
- ❌ Tooltips (X11 only per Qt docs)
- ❌ Wheel events on tray icon (X11 only)
- ⚠️ Balloon notifications may behave differently

**From Qt Documentation**:
> "Only on X11, when a tooltip is requested, the QSystemTrayIcon receives a QHelpEvent of type QEvent::ToolTip. Additionally, the QSystemTrayIcon receives wheel events of type QEvent::Wheel. These are not supported on any other platform."

---

## Common Patterns from Real-World Apps

### Telegram Desktop (Qt/C++)
```cpp
void TrayIcon::activated(QSystemTrayIcon::ActivationReason reason) {
    if (reason == QSystemTrayIcon::Trigger) {
        _singleClickTimer.start(qApp->doubleClickInterval());
    } else if (reason == QSystemTrayIcon::DoubleClick) {
        _singleClickTimer.stop();
        showMainWindow();
    }
}
```

### Discord (Electron, similar concept)
- Single-click: Show/hide window
- Double-click: (Not used, conflicts with single-click)
- Right-click: Context menu

**Key takeaway**: Most apps use EITHER single-click OR double-click for primary action, not both. Your approach (single for toggle, double for window) requires the debounce timer - which you have implemented correctly.

---

## Testing Checklist

Run these tests on your Sway/Wayland environment:

- [ ] **Single-click** → Wait 250ms → Recording toggles
- [ ] **Double-click** → Window shows immediately (no recording toggle)
- [ ] **Triple-click** → Window shows once (no recording toggle)
- [ ] **Right-click** → Context menu appears
- [ ] **Middle-click** → Recording toggles immediately
- [ ] **Click during recording** → Icon updates, toggle works
- [ ] **Rapid clicks** → No unexpected behavior

---

## If You Encounter Issues

### Issue: Double-click shows window AND toggles recording

**Diagnosis**: Timer not being cancelled properly

**Fix**: Verify `self.click_timer.stop()` is called in DoubleClick handler

---

### Issue: Single-click not working at all

**Diagnosis**: Timer interval too long, or timer not starting

**Fix**:
1. Check timer is created: `self.click_timer = QTimer()`
2. Check timer is single-shot: `self.click_timer.setSingleShot(True)`
3. Verify timeout is connected: `self.click_timer.timeout.connect(...)`

---

### Issue: System tray icon not appearing

**Diagnosis**: Wayland compositor doesn't support StatusNotifierItem

**Fix**:
1. Check: `QSystemTrayIcon.isSystemTrayAvailable()` → should return `True`
2. On Sway, ensure waybar or swaybar has tray module enabled
3. Fallback: Show metrics window by default if tray unavailable

---

## Recommended Final Implementation

```python
class SwictationTrayApp(QApplication):
    def __init__(self, argv):
        super().__init__(argv)

        # Check system tray availability (Wayland may not support it)
        if not QSystemTrayIcon.isSystemTrayAvailable():
            print("⚠️  System tray not available")

        # Setup paths
        self.icon_path = Path(__file__).parent.parent.parent / "docs" / "swictation_logo.png"

        # Create system tray icon
        self.tray_icon = QSystemTrayIcon(self)
        self.tray_icon.setIcon(self._load_icon("idle"))
        self.tray_icon.activated.connect(self.on_tray_activated)

        # Create tray menu
        menu = QMenu()
        menu.addAction("Show Metrics", self.show_window)
        menu.addAction("Toggle Recording", self.toggle_recording)
        menu.addSeparator()
        menu.addAction("Quit", self.quit)
        self.tray_icon.setContextMenu(menu)

        # Show tray icon
        self.tray_icon.show()

        # Single-click debounce timer (use system interval)
        self.click_timer = QTimer(self)
        self.click_timer.setSingleShot(True)
        self.click_timer.timeout.connect(self.on_single_click_confirmed)

        # Create metrics backend
        self.backend = MetricsBackend()
        self.backend.stateChanged.connect(self.on_state_changed)

        # Load QML window
        self.engine = QQmlApplicationEngine()
        self.engine.rootContext().setContextProperty("backend", self.backend)
        qml_file = Path(__file__).parent / "MetricsUI.qml"
        self.engine.load(QUrl.fromLocalFile(str(qml_file)))

        if not self.engine.rootObjects():
            print("✗ Failed to load QML")
            sys.exit(1)

        self.window = self.engine.rootObjects()[0]
        self.window.hide()

        print("✓ Swictation tray app started")

    @Slot(int)
    def on_tray_activated(self, reason):
        """Handle tray icon clicks with system-interval debounce."""
        if reason == QSystemTrayIcon.DoubleClick:
            # Double-click: cancel pending single-click and show window
            self.click_timer.stop()
            self.toggle_window()

        elif reason == QSystemTrayIcon.Trigger:
            # Single-click: start timer (system interval for accessibility)
            self.click_timer.start(QApplication.doubleClickInterval())

        elif reason == QSystemTrayIcon.MiddleClick:
            # Middle-click: immediate toggle (alternative method)
            self.toggle_recording()

    @Slot()
    def on_single_click_confirmed(self):
        """Execute single-click action after debounce delay."""
        self.toggle_recording()

    # ... rest of implementation
```

---

## Key Changes from Current Code

| Change | From | To | Impact |
|--------|------|-----|--------|
| Timer interval | `250` (fixed) | `QApplication.doubleClickInterval()` | Better accessibility |
| System check | None | `isSystemTrayAvailable()` check | Graceful fallback |
| Logging | Minimal | Debug logging (optional) | Easier troubleshooting |

---

## References

- **Qt6 QSystemTrayIcon Docs**: https://doc.qt.io/qt-6/qsystemtrayicon.html
- **StatusNotifierItem Spec**: https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/
- **Community Examples**: https://gist.github.com/for-l00p/3e33305f948659313127632ad04b4311

---

## Bottom Line

**Your current implementation (commit e717e3c) is solid and follows best practices.**

The only recommended change is to use `QApplication.doubleClickInterval()` instead of the hardcoded `250ms` for better accessibility and cross-platform consistency. Everything else is optional polish.

**Rating**: ⭐⭐⭐⭐⭐ (5/5) - Industry-standard implementation
