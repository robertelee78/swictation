# macOS Tray App Window Close Behavior - Implementation Summary

## Problem Statement
When users close the Swictation window on macOS, the app exits completely. With launchd KeepAlive configuration, this causes the window to immediately reopen, making it impossible for users to close the window.

## Expected Behavior
- Tray icon always visible in menu bar
- Closing window should **hide** it, not exit the app
- User can show window again via tray icon
- App stays running in background
- Only "Quit" menu option should actually exit

## Implementation

### 1. Tauri Window Event Handler
**File**: `/opt/swictation/tauri-ui/src-tauri/src/main.rs` (lines 156-163)

```rust
.on_window_event(|window, event| {
    if let WindowEvent::CloseRequested { api, .. } = event {
        // Hide window instead of closing to keep app running in tray
        // This is standard tray app behavior on all platforms
        window.hide().unwrap();
        api.prevent_close();
    }
})
```

**What this does**:
- Intercepts the window close event
- Calls `window.hide()` to hide the window
- Calls `api.prevent_close()` to prevent actual app termination
- Works on all platforms (macOS, Linux, Windows)

### 2. Tray Icon Menu
**File**: `/opt/swictation/tauri-ui/src-tauri/src/main.rs` (lines 36-75)

Tray menu includes:
- **Show Metrics** - Shows/focuses the window (lines 59-65)
- **Toggle Recording** - Toggles recording state (lines 67-70)
- **Quit** - Exits the application with `app.exit(0)` (lines 71-73)

### 3. Tray Icon Click Behaviors
**File**: `/opt/swictation/tauri-ui/src-tauri/src/main.rs` (lines 76-104)

- **Left Click**: Toggle recording (lines 77-85)
- **Middle Click**: Toggle window visibility (lines 86-102)

### 4. Window Configuration
**File**: `/opt/swictation/tauri-ui/src-tauri/tauri.conf.json` (lines 31-55)

Added explicit window properties:
```json
{
  "closable": true,
  "minimizable": true,
  "maximizable": true,
  "hiddenTitle": false
}
```

## Potential Issues & Root Cause Analysis

### launchd KeepAlive Configuration
**File**: `/opt/swictation/npm-package/templates/macos/com.swictation.ui.plist` (lines 24-38)

```xml
<key>KeepAlive</key>
<dict>
    <!-- Restart if process exits unexpectedly -->
    <key>SuccessfulExit</key>
    <false/>
    <!-- Restart if process crashes -->
    <key>Crashed</key>
    <true/>
</dict>
```

**Critical Setting**: `SuccessfulExit = false`

This means:
- If the app exits with code 0 (success), launchd **will restart it**
- The "Quit" menu calls `app.exit(0)` which triggers restart
- This is **by design** to keep the UI running if daemon is running

### Why This Might Still Cause Issues

1. **If window close somehow exits**: Despite `prevent_close()`, if the app exits, launchd restarts it
2. **Quit menu behavior**: Clicking "Quit" will cause restart due to `SuccessfulExit = false`
3. **Expected behavior**: Users should use window close (X button), not Quit menu

## Recommended Solutions

### Option 1: Keep Current Implementation (Recommended)
- Window close button **hides** the window ✅
- App stays running in background ✅
- Tray icon remains visible ✅
- Users can reopen via "Show Metrics" menu ✅

**Issue**: "Quit" menu will trigger restart due to launchd config

### Option 2: Modify Quit Behavior
Change quit to exit with non-zero code to prevent restart:

```rust
"quit" => {
    app.exit(1); // Non-zero exit prevents launchd restart
}
```

**Downside**: Might be logged as "error" in system logs

### Option 3: Modify launchd Configuration
Remove `SuccessfulExit` or set to `true`:

```xml
<key>KeepAlive</key>
<dict>
    <!-- Only restart if process crashes -->
    <key>Crashed</key>
    <true/>
</dict>
```

**Downside**: App won't auto-restart if manually quit

### Option 4: Remove Quit Menu on macOS
Hide the quit option on macOS, forcing users to use Activity Monitor:

```rust
#[cfg(not(target_os = "macos"))]
let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

#[cfg(not(target_os = "macos"))]
let menu = Menu::with_items(app, &[&show_metrics, &toggle_recording, &separator, &quit])?;

#[cfg(target_os = "macos")]
let menu = Menu::with_items(app, &[&show_metrics, &toggle_recording])?;
```

## Testing Instructions

### Test 1: Window Close Behavior
1. Launch Swictation UI
2. Click window close button (X)
3. **Expected**: Window disappears, tray icon remains
4. **Expected**: App still running (check Activity Monitor)

### Test 2: Show Window from Tray
1. After hiding window, click tray icon "Show Metrics"
2. **Expected**: Window reappears and gets focus

### Test 3: Quit Behavior
1. Click tray icon → "Quit"
2. **Expected**: App exits
3. **With current launchd**: App restarts immediately
4. **Expected fix**: App stays closed OR modify launchd config

### Test 4: Middle Click Tray
1. Middle-click tray icon
2. **Expected**: Window toggles visibility

## Build & Deploy

```bash
# Build release version
cd /opt/swictation/tauri-ui/src-tauri
cargo build --release

# Binary location
./target/release/swictation-ui

# Install updated launchd plist (if modified)
cp /opt/swictation/npm-package/templates/macos/com.swictation.ui.plist \
   ~/Library/LaunchAgents/com.swictation.ui.plist
launchctl unload ~/Library/LaunchAgents/com.swictation.ui.plist
launchctl load ~/Library/LaunchAgents/com.swictation.ui.plist
```

## Files Modified

1. `/opt/swictation/tauri-ui/src-tauri/src/main.rs` - Window close handler
2. `/opt/swictation/tauri-ui/src-tauri/tauri.conf.json` - Window properties

## Conclusion

The Tauri code is **correctly implemented** for standard tray app behavior:
- Window close → hide (not exit) ✅
- Tray menu with Show/Quit options ✅
- Multiple ways to show window ✅

The **real issue** is the launchd `KeepAlive` configuration conflicting with user expectations. Users clicking "Quit" expect the app to stay closed, but launchd restarts it.

**Recommended fix**: Modify launchd plist to remove `SuccessfulExit` setting, allowing normal quit behavior.
