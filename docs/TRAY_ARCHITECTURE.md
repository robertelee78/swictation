# Swictation Tray Architecture

## Problem Statement

System tray icons are surprisingly complex in the Linux ecosystem due to fragmentation between display protocols (X11 vs Wayland) and desktop environments.

**The Core Issue:**
- **Tauri** uses `libayatana-appindicator` for Linux tray icons
- **Sway/Wayland** uses StatusNotifierItem (SNI) protocol via D-Bus
- These two don't communicate properly, resulting in fallback "red dot" icons instead of actual icons

**What Works:**
- Qt applications (Telegram, Zoom, etc.) use `QSystemTrayIcon` which implements SNI directly via Qt5DBus
- This works perfectly across all environments: macOS, Windows, Linux (X11/Wayland/Sway)

## Solution: Hybrid Tray Architecture

We implement **conditional tray based on detected environment**:

```
┌─────────────────────────────────────────┐
│   launch-swictation-ui.sh (detector)     │
│                                          │
│   if Sway/Wayland detected:              │
│     → Qt Tray (Python + PySide6)         │
│   else:                                   │
│     → Tauri Tray (Rust + web frontend)  │
└─────────────────────────────────────────┘
```

### Detection Logic

```bash
# Sway/Wayland detection
[[ "${XDG_SESSION_TYPE}" == "wayland" ]] && [[ -n "${SWAYSOCK}" ]]

# Future: add other problematic environments as discovered
```

### Files

- **Launcher**: `/opt/swictation/scripts/launch-swictation-ui.sh`
- **Qt Tray**: `/opt/swictation/src/ui/swictation_tray.py` (existing, proven)
- **Tauri UI**: `/opt/swictation/tauri-ui/src-tauri/target/release/swictation-ui`
- **Service**: `/opt/swictation/config/swictation-ui.service` (unified)

## Platform Support Matrix

| Platform | Environment | Tray Implementation | Status |
|----------|-------------|-------------------|--------|
| Linux | Sway/Wayland | Qt (PySide6) | ✅ Works |
| Linux | Gnome/X11 | Tauri | ✅ Works |
| Linux | KDE/X11 | Tauri | ✅ Works |
| Linux | Other X11 | Tauri | ✅ Should work |
| macOS | All | Tauri | ✅ Works |
| Windows | All | Tauri | ✅ Works |

## Technical Details

### Qt Tray (Sway/Wayland)
- Uses `QSystemTrayIcon` from PySide6
- Implements StatusNotifierItem via Qt5DBus
- Direct D-Bus communication with swaybar
- Includes Wayland-specific event filter for reliable right-click menus
- **Icon works perfectly** (same as Telegram, Zoom, etc.)

### Tauri Tray (Other Platforms)
- Uses Tauri v2 `tray-icon` crate
- Cross-platform: macOS (native), Windows (native), Linux (libayatana-appindicator)
- Full npm/React frontend
- **Known issue**: Red dot on Sway/Wayland (upstream bug)

## Why This Approach?

1. **Best of Both Worlds**: Qt for Sway, Tauri for everything else
2. **Minimal Changes**: Reuses existing working Qt code
3. **Cross-Platform**: Single codebase supports all environments
4. **Future-Proof**: Can add more env-specific implementations as needed

## Future Improvements

- [ ] Add Electron/NW.js detection (they have similar Sway issues)
- [ ] Create pure Rust SNI implementation (eliminate Python dependency)
- [ ] Contribute SNI support to Tauri upstream
- [ ] Add environment detection telemetry

## References

- [Sway StatusNotifierItem Implementation](https://github.com/swaywm/sway/blob/master/swaybar/tray/tray.c)
- [Qt QSystemTrayIcon Docs](https://doc.qt.io/qt-6/qsystemtrayicon.html)
- [Tauri Tray Icon Issues](https://github.com/tauri-apps/tauri/issues?q=tray+icon+linux)
- [StatusNotifierItem Spec](https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/)
