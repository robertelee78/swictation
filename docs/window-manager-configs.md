# Window Manager Configuration Examples

**Real-world configuration examples for popular window managers**

**Two things to know before copying these:**

- **Only one command is installed: `swictation`.** There are no `swictation-toggle` or
  `swictation-ui` binaries — use `swictation toggle` and `swictation start --ui`.
- **Status-bar indicators show whether the daemon is running, not whether it is
  recording.** The examples below poll
  `systemctl --user is-active swictation-daemon`, which reports `active` /
  `inactive`. Live recording state is published on the metrics Unix socket
  (`swictation_metrics.sock`), which needs a real client to read; a bar module cannot
  poll it with `cat`.

**Tray icon on Sway, Hyprland, and River:** these wlroots compositors do not host the
Tauri tray, so postinstall installs a small Python/Qt tray instead
(`swictation-ui.service` running `/usr/bin/python3 …/swictation_tray.py`). It needs
`python3` plus PySide6 6.8+ — `pip3 install -r requirements-qt-tray.txt`, or
`python3-pyside6` from your distro. Skip it if you do not want the tray: dictation,
hotkeys, and the CLI work without it, and everything except that tray is Rust.

---

## Table of Contents

1. [Tiling Window Managers](#tiling-window-managers)
   - [i3](#i3)
   - [Sway](#sway)
   - [bspwm](#bspwm)
   - [Hyprland](#hyprland)
   - [awesome](#awesome)
   - [dwm](#dwm)
   - [Qtile](#qtile)
2. [Desktop Environments](#desktop-environments)
   - [GNOME](#gnome)
   - [KDE Plasma](#kde-plasma)
   - [XFCE](#xfce)
   - [Cinnamon](#cinnamon)
3. [Stacking Window Managers](#stacking-window-managers)
   - [Openbox](#openbox)
   - [Fluxbox](#fluxbox)

---

## Tiling Window Managers

### i3

**Display server:** X11
**Tool:** xdotool

#### Basic Setup

**File:** `~/.config/i3/config`

```bash
# Start swictation daemon on i3 startup
exec --no-startup-id swictation-daemon

# Optional: Bind dictation toggle to hotkey
# Note: Swictation registers its own global hotkey via global-hotkeys crate
# This is just an alternative manual toggle
bindsym $mod+Shift+d exec --no-startup-id swictation toggle
```

#### Advanced Setup with Status Bar

**File:** `~/.config/i3/config`

```bash
# Auto-start swictation daemon
exec --no-startup-id swictation-daemon

# Add swictation status to i3status
# File: ~/.config/i3status/config
```

**File:** `~/.config/i3status/config`

i3status has no module that runs a command, so use i3blocks (or i3status-rust)
for a Swictation indicator:

**File:** `~/.config/i3blocks/config`

```ini
[swictation]
command=systemctl --user is-active swictation-daemon 2>/dev/null || echo inactive
interval=2
label=🎤
```

#### Workspace Rules

```bash
# Optional: Assign swictation UI to specific workspace
assign [class="swictation-ui"] $ws9

# Float swictation settings window
for_window [class="swictation-ui"] floating enable
```

---

### Sway

**Display server:** Wayland
**Tool:** wtype

#### Basic Setup

**File:** `~/.config/sway/config`

```bash
# Start swictation daemon
exec swictation-daemon

# Optional: Manual toggle binding
bindsym $mod+Shift+d exec swictation toggle
```

#### With Waybar Integration

**File:** `~/.config/sway/config`

```bash
# Auto-start swictation
exec swictation-daemon

# Waybar integration
bar {
    swaybar_command waybar
}
```

**File:** `~/.config/waybar/config`

```json
{
    "modules-right": ["pulseaudio", "custom/swictation", "clock"],

    "custom/swictation": {
        "exec": "systemctl --user is-active swictation-daemon 2>/dev/null || echo 'inactive'",
        "interval": 1,
        "format": "🎤 {}",
        "on-click": "swictation toggle"
    }
}
```

#### Startup with systemd

**File:** `~/.config/systemd/user/swictation-daemon.service`

```ini
[Unit]
Description=Swictation voice dictation daemon for Sway
PartOf=graphical-session.target

[Service]
Type=simple
ExecStart=%h/.local/share/swictation/bin/swictation-daemon
Restart=on-failure

[Install]
WantedBy=sway-session.target
```

**Enable:**
```bash
systemctl --user enable swictation-daemon.service
```

---

### bspwm

**Display server:** X11
**Tool:** xdotool

#### Basic Setup

**File:** `~/.config/bspwm/bspwmrc`

```bash
#!/bin/sh

# Start swictation daemon
swictation-daemon &

# Rest of your bspwm config...
```

#### With sxhkd Keybindings

**File:** `~/.config/sxhkd/sxhkdrc`

```bash
# Swictation manual toggle (optional)
super + shift + d
    swictation toggle

# Swictation settings UI
super + shift + s
    swictation start --ui
```

#### Polybar Integration

**File:** `~/.config/polybar/config.ini`

```ini
[module/swictation]
type = custom/script
exec = systemctl --user is-active swictation-daemon 2>/dev/null || echo "inactive"
interval = 1
format-prefix = "🎤 "
click-left = swictation toggle
```

**File:** `~/.config/bspwm/bspwmrc`

```bash
# Start polybar with swictation module
polybar mybar &
```

---

### Hyprland

**Display server:** Wayland
**Tool:** wtype

#### Basic Setup

**File:** `~/.config/hypr/hyprland.conf`

```bash
# Startup apps
exec-once = swictation-daemon

# Optional: Manual toggle binding
bind = SUPER SHIFT, D, exec, swictation toggle

# Optional: Open settings UI
bind = SUPER SHIFT, S, exec, swictation start --ui
```

#### Advanced with Waybar

**File:** `~/.config/hypr/hyprland.conf`

```bash
# Auto-start applications
exec-once = swictation-daemon
exec-once = waybar

# Window rules for swictation UI
windowrulev2 = float, class:(swictation start --ui)
windowrulev2 = size 800 600, class:(swictation start --ui)
windowrulev2 = center, class:(swictation start --ui)
```

**File:** `~/.config/waybar/config` (same as Sway example above)

---

### awesome

**Display server:** X11 or Wayland
**Tool:** xdotool (X11) or wtype (Wayland)

#### Basic Setup

**File:** `~/.config/awesome/rc.lua`

```lua
-- Auto-start swictation daemon
awful.spawn.with_shell("swictation-daemon")

-- Optional: Add keybinding for manual toggle
awful.key({ modkey, "Shift" }, "d",
    function()
        awful.spawn("swictation toggle")
    end,
    {description = "toggle swictation dictation", group = "swictation"}
)
```

#### With Widget

**File:** `~/.config/awesome/rc.lua`

```lua
-- Swictation status widget
local swictation_widget = wibox.widget.textbox()
swictation_widget.text = "🎤 OFF"

-- Update widget every second
gears.timer {
    timeout = 1,
    autostart = true,
    callback = function()
        awful.spawn.easy_async_with_shell(
            "systemctl --user is-active swictation-daemon 2>/dev/null || echo 'inactive'",
            function(stdout)
                swictation_widget.text = "🎤 " .. stdout:gsub("\n", "")
            end
        )
    end
}

-- Add to wibar
s.mywibox:setup {
    -- ... other widgets ...
    swictation_widget,
    -- ...
}

-- Make widget clickable
swictation_widget:buttons(gears.table.join(
    awful.button({}, 1, function()
        awful.spawn("swictation toggle")
    end)
))
```

---

### dwm

**Display server:** X11
**Tool:** xdotool

#### Setup

**Note:** dwm has no config files - configuration is done in `config.h` and requires recompilation.

#### Autostart Method 1: .xinitrc

**File:** `~/.xinitrc`

```bash
#!/bin/sh

# Start swictation daemon before dwm
swictation-daemon &

# Start dwm
exec dwm
```

#### Autostart Method 2: dwm autostart patch

If you've applied the autostart patch to dwm:

**File:** `~/.dwm/autostart.sh`

```bash
#!/bin/sh

swictation-daemon &
```

Make executable:
```bash
chmod +x ~/.dwm/autostart.sh
```

#### Keybinding (requires recompile)

**File:** `config.h`

```c
static const char *swictation_toggle[] = {
    "swictation toggle", NULL
};

static Key keys[] = {
    // ... other keys ...
    { MODKEY|ShiftMask, XK_d, spawn, {.v = swictation_toggle} },
    // ...
};
```

Then recompile and install:
```bash
cd ~/dwm
sudo make clean install
```

---

### Qtile

**Display server:** X11 or Wayland
**Tool:** xdotool (X11) or wtype (Wayland)

#### Basic Setup

**File:** `~/.config/qtile/config.py`

```python
import subprocess
from libqtile import hook

# Auto-start swictation on Qtile startup
@hook.subscribe.startup_once
def autostart():
    subprocess.Popen(["swictation-daemon"])

# Optional: Add keybinding
from libqtile.config import Key
from libqtile.lazy import lazy

keys = [
    # ... other keys ...
    Key([mod, "shift"], "d", lazy.spawn("swictation toggle"),
        desc="Toggle swictation dictation"),
    # ...
]
```

#### With Widget

```python
from libqtile import widget

screens = [
    Screen(
        top=bar.Bar([
            # ... other widgets ...
            widget.GenPollText(
                func=lambda: "🎤 " + subprocess.check_output(
                    "systemctl --user is-active swictation-daemon",
                    shell=True, stderr=subprocess.DEVNULL, text=True
                ).strip(),
                update_interval=1,
                mouse_callbacks={
                    'Button1': lazy.spawn("swictation toggle")
                }
            ),
            # ...
        ], 24),
    ),
]
```

---

## Desktop Environments

### GNOME

**Display server:** Wayland (default) or X11 (legacy)
**Tool:** ydotool (Wayland) or xdotool (X11)

#### Autostart (GUI Method)

1. Open "Startup Applications" (gnome-session-properties)
2. Click "Add"
3. Fill in:
   - Name: `Swictation`
   - Command: `swictation-daemon`
   - Comment: `Voice dictation service`
4. Click "Add"

#### Autostart (Manual Method)

**File:** `~/.config/autostart/swictation.desktop`

```ini
[Desktop Entry]
Type=Application
Name=Swictation Voice Dictation
Exec=swictation-daemon
Icon=microphone
Comment=Voice-to-text dictation daemon
X-GNOME-Autostart-enabled=true
Hidden=false
NoDisplay=false
```

#### GNOME Wayland Specific Setup

**IMPORTANT:** GNOME Wayland requires ydotool with permissions!

```bash
# 1. Install ydotool
sudo apt install ydotool  # Ubuntu/Debian
# OR
sudo dnf install ydotool  # Fedora

# 2. Grant permissions (REQUIRED)
sudo usermod -aG input $USER

# 3. Log out and log back in (CRITICAL!)

# 4. Verify
groups | grep input
ydotool type "test"
```

#### GNOME Extension (Optional)

Create a simple GNOME extension for status indicator:

**File:** `~/.local/share/gnome-shell/extensions/swictation@example.com/extension.js`

```javascript
const St = imports.gi.St;
const Main = imports.ui.main;
const Mainloop = imports.mainloop;
const GLib = imports.gi.GLib;

let panelButton;
let timeout;

function update_status() {
    try {
        let [ok, out] = GLib.spawn_command_line_sync(
            'systemctl --user is-active swictation-daemon');
        panelButton.set_label('🎤 ' + out.toString().trim());
    } catch (e) {
        panelButton.set_label('🎤 OFF');
    }
    return true;
}

function init() {
}

function enable() {
    panelButton = new St.Label({ text: '🎤 OFF' });
    Main.panel._rightBox.insert_child_at_index(panelButton, 0);

    timeout = Mainloop.timeout_add_seconds(1, update_status);
}

function disable() {
    Mainloop.source_remove(timeout);
    Main.panel._rightBox.remove_child(panelButton);
}
```

**File:** `~/.local/share/gnome-shell/extensions/swictation@example.com/metadata.json`

```json
{
  "uuid": "swictation@example.com",
  "name": "Swictation Status",
  "description": "Shows swictation dictation status",
  "shell-version": ["42", "43", "44", "45", "46"]
}
```

Enable:
```bash
gnome-extensions enable swictation@example.com
```

---

### KDE Plasma

**Display server:** Wayland (default in Plasma 6) or X11
**Tool:** wtype (Wayland) or xdotool (X11)

#### Autostart (GUI Method)

1. System Settings → Startup and Shutdown → Autostart
2. Click "Add..." → "Add Application..."
3. Find or type: `swictation-daemon`
4. Click "OK"

#### Autostart (Manual Method)

**File:** `~/.config/autostart/swictation.desktop`

```ini
[Desktop Entry]
Type=Application
Name=Swictation Voice Dictation
Exec=swictation-daemon
Icon=audio-input-microphone
Comment=Voice-to-text dictation daemon
X-KDE-autostart-after=panel
```

#### KDE Wayland Setup

**Tool:** wtype (KDE supports virtual-keyboard protocol)

```bash
# Install wtype
sudo apt install wtype      # Ubuntu/Debian
# OR
sudo dnf install wtype      # Fedora
# OR
sudo pacman -S wtype        # Arch

# Test
wtype "test"  # Should work on KDE Wayland
```

#### Plasma Widget (Optional)

Use "Command Output" widget:

1. Right-click panel → Add Widgets
2. Find "Command Output"
3. Configure:
   - Command: `systemctl --user is-active swictation-daemon 2>/dev/null || echo "inactive"`
   - Update interval: 1000ms
   - Prefix: `🎤 `

---

### XFCE

**Display server:** X11 (Wayland support experimental)
**Tool:** xdotool

#### Autostart (GUI Method)

1. Settings → Session and Startup → Application Autostart
2. Click "+" (Add)
3. Fill in:
   - Name: `Swictation`
   - Command: `swictation-daemon`
   - Description: `Voice dictation`
4. Click "OK"

#### Panel Plugin

Use "Generic Monitor" plugin:

1. Right-click panel → Panel → Add New Items
2. Find "Generic Monitor"
3. Right-click new monitor → Properties
4. Command: `systemctl --user is-active swictation-daemon 2>/dev/null || echo "inactive"`
5. Period: 1s
6. Label: `🎤`

---

### Cinnamon

**Display server:** X11 (primary), some Wayland support
**Tool:** xdotool

#### Autostart

1. System Settings → Startup Applications
2. Click "+" (Add)
3. Fill in:
   - Name: `Swictation`
   - Command: `swictation-daemon`
   - Comment: `Voice dictation daemon`
4. Click "Add"

#### Manual Method

**File:** `~/.config/autostart/swictation.desktop`

```ini
[Desktop Entry]
Type=Application
Name=Swictation
Exec=swictation-daemon
Icon=microphone
Comment=Voice dictation daemon
X-GNOME-Autostart-enabled=true
```

---

## Stacking Window Managers

### Openbox

**Display server:** X11
**Tool:** xdotool

#### Autostart

**File:** `~/.config/openbox/autostart`

```bash
#!/bin/bash

# Start swictation daemon
swictation-daemon &
```

Make executable:
```bash
chmod +x ~/.config/openbox/autostart
```

#### Keybinding

**File:** `~/.config/openbox/rc.xml`

```xml
<keyboard>
  <!-- ... other keybindings ... -->

  <!-- Swictation toggle -->
  <keybind key="W-S-d">
    <action name="Execute">
      <command>swictation toggle</command>
    </action>
  </keybind>
</keyboard>
```

Apply changes:
```bash
openbox --reconfigure
```

---

### Fluxbox

**Display server:** X11
**Tool:** xdotool

#### Autostart

**File:** `~/.fluxbox/startup`

```bash
#!/bin/sh

# Start swictation daemon
swictation-daemon &

# Start fluxbox (must be last)
exec fluxbox
```

#### Keybinding

**File:** `~/.fluxbox/keys`

```
# Swictation toggle
Mod4 Shift D :Exec swictation toggle
```

---

## Systemd User Service (Universal)

**Works with any window manager/DE**

### Create Service File

**File:** `~/.config/systemd/user/swictation-daemon.service`

```ini
[Unit]
Description=Swictation voice dictation daemon
After=graphical-session.target
PartOf=graphical-session.target

[Service]
Type=simple
ExecStart=%h/.local/share/swictation/bin/swictation-daemon
Restart=on-failure
RestartSec=5

# Environment variables (if needed)
Environment="DISPLAY=:0"
Environment="WAYLAND_DISPLAY=wayland-0"

[Install]
WantedBy=graphical-session.target
```

### Enable and Start

```bash
# Reload systemd user daemon
systemctl --user daemon-reload

# Enable service (auto-start on login)
systemctl --user enable swictation-daemon.service

# Start service now
systemctl --user start swictation-daemon.service

# Check status
systemctl --user status swictation-daemon.service

# View logs
journalctl --user -u swictation-daemon.service -f
```

---

## Verification

### Check if Daemon is Running

```bash
# Check process
ps aux | grep swictation-daemon

# Check with systemd (if using service)
systemctl --user status swictation-daemon

# Check logs
journalctl --user -u swictation-daemon -n 50
```

### Test Text Injection

```bash
# Open a text editor (gedit, kate, vim, etc.)
# Click in text field

# Test tool directly
xdotool type "test"    # X11
wtype "test"           # Wayland (non-GNOME)
ydotool type "test"    # Universal

# Should type "test" in active window
```

---

## See Also

- [Display Server Guide](display-servers.md) - Technical background
- [Installation by Distribution](installation-by-distro.md) - Distro-specific setup
- [Troubleshooting](troubleshooting-display-servers.md) - Common issues

---

**Last updated:** 2024-11-15
**Tested on:** i3 4.23, Sway 1.9, Hyprland 0.40, GNOME 46, KDE Plasma 6
