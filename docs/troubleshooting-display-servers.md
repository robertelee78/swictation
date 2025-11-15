# Troubleshooting Display Server Support

**Common issues and solutions for X11/Wayland text injection**

---

## Table of Contents

1. [GNOME Wayland Issues](#gnome-wayland-issues)
2. [Permission Errors (ydotool)](#permission-errors-ydotool)
3. [Tool Not Found](#tool-not-found)
4. [Wrong Display Server Detected](#wrong-display-server-detected)
5. [Text Injection Not Working](#text-injection-not-working)
6. [Performance Issues](#performance-issues)
7. [Environment Detection](#environment-detection)

---

## GNOME Wayland Issues

### Error: "GNOME Wayland requires ydotool"

**Symptoms:**
```
Error: GNOME Wayland requires ydotool

GNOME's Wayland compositor does not support wtype.
You must use ydotool for text injection.
```

**Diagnosis:**
```bash
# Check if you're on GNOME Wayland
echo $XDG_SESSION_TYPE      # Should show "wayland"
echo $XDG_CURRENT_DESKTOP   # Should show "GNOME" or "ubuntu:GNOME"
```

**Root cause:**
- GNOME's Mutter compositor doesn't implement `virtual-keyboard-unstable-v1` protocol
- wtype requires this protocol
- ydotool uses kernel uinput instead (works on GNOME)

**Solution:**
```bash
# 1. Install ydotool
sudo apt install ydotool      # Ubuntu/Debian
# OR
sudo dnf install ydotool      # Fedora
# OR
sudo pacman -S ydotool        # Arch

# 2. Grant permissions (REQUIRED)
sudo usermod -aG input $USER

# 3. MUST log out and log back in
# (Not just close terminal - full desktop logout!)

# 4. Verify after logging back in
groups | grep input   # Should show "input"
ydotool type "test"   # Should type "test"

# 5. Restart swictation daemon
systemctl --user restart swictation-daemon
```

**Alternative: Switch to X11 session**
```bash
# At login screen (GDM):
# 1. Click gear icon (bottom right)
# 2. Select "Ubuntu on Xorg" or "GNOME on Xorg"
# 3. Log in

# Then install xdotool instead (faster, no permissions)
sudo apt install xdotool
xdotool type "test"
```

---

## Permission Errors (ydotool)

### Error: "Permission denied" when running ydotool

**Symptoms:**
```
$ ydotool type "test"
Failed to create ydotool client: Permission denied
```

**OR:**
```
Error: Permission denied accessing /dev/uinput
```

**Diagnosis:**
```bash
# Check /dev/uinput permissions
ls -l /dev/uinput
# Should show: crw-rw---- 1 root input 10, 223 Nov 15 10:30 /dev/uinput

# Check your group membership
groups | grep input
# Should show "input" in the list
```

**Root cause:**
- `/dev/uinput` owned by `root:input` group
- User must be member of `input` group
- Group membership only updates on login

**Solution:**
```bash
# 1. Add yourself to input group
sudo usermod -aG input $USER

# 2. CRITICAL: Log out and log back in
# Opening a new terminal is NOT enough!
# Must do full desktop logout/login

# 3. Verify group membership
groups | grep input
# Should now show "input"

# 4. Test ydotool
ydotool type "test"
# Should work without errors
```

**Common mistake:**
```bash
# ❌ WRONG (doesn't refresh groups)
sudo usermod -aG input $USER
ydotool type "test"  # Still fails!

# ✅ CORRECT
sudo usermod -aG input $USER
# [Log out and log back in]
ydotool type "test"  # Works!
```

**Immediate verification (without logout):**
```bash
# Start new session with updated groups (temporary)
newgrp input
# Now in subshell with input group active
ydotool type "test"  # Should work
exit  # Return to normal shell

# But still need to log out for permanent effect
```

---

### Error: "ydotool: command not found"

**Symptoms:**
```bash
$ ydotool type "test"
bash: ydotool: command not found
```

**Diagnosis:**
```bash
# Check if ydotool is installed
which ydotool
# If no output, it's not installed

# Check your distribution version
lsb_release -a    # Ubuntu/Debian
cat /etc/fedora-release  # Fedora
```

**Solution by distribution:**

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install ydotool

# If not found, you may be on older version
cat /etc/os-release  # Check VERSION_ID

# ydotool available in:
# - Ubuntu 24.04+
# - Debian 12+

# For older versions, switch to X11 + xdotool
```

**Fedora:**
```bash
sudo dnf install ydotool

# Available in Fedora 40+
```

**Arch:**
```bash
sudo pacman -S ydotool
```

---

## Tool Not Found

### Error: "Text injection tool not found for X11"

**Symptoms:**
```
Error: Text injection tool not found for X11

Required tool: xdotool
Install command:
  Ubuntu/Debian: sudo apt install xdotool
```

**Diagnosis:**
```bash
# Check environment
echo $XDG_SESSION_TYPE  # Should show "x11"
echo $DISPLAY           # Should show ":0" or ":1"

# Check if xdotool installed
which xdotool           # No output = not installed
```

**Solution:**
```bash
# Install xdotool
sudo apt install xdotool      # Ubuntu/Debian
# OR
sudo dnf install xdotool      # Fedora
# OR
sudo pacman -S xdotool        # Arch

# Verify
which xdotool
xdotool type "test"

# Restart daemon
systemctl --user restart swictation-daemon
```

---

### Error: "Text injection tool not found for Wayland"

**Symptoms:**
```
Error: Wayland text injection tool not found

Recommended tool: wtype
  Ubuntu/Debian: sudo apt install wtype
```

**Diagnosis:**
```bash
# Check environment
echo $XDG_SESSION_TYPE       # Should show "wayland"
echo $XDG_CURRENT_DESKTOP    # Shows your DE
echo $WAYLAND_DISPLAY        # Should show "wayland-0"

# Check if on GNOME
echo $XDG_CURRENT_DESKTOP    # If "GNOME", use ydotool instead!
```

**Solution (non-GNOME Wayland):**
```bash
# KDE/Sway/Hyprland: Install wtype
sudo apt install wtype        # Ubuntu/Debian
# OR
sudo dnf install wtype        # Fedora
# OR
sudo pacman -S wtype          # Arch

# Verify
which wtype
wtype "test"
```

**Solution (GNOME Wayland):**
```bash
# GNOME needs ydotool, not wtype
sudo apt install ydotool
sudo usermod -aG input $USER
# Log out and log back in
groups | grep input
ydotool type "test"
```

---

## Wrong Display Server Detected

### Swictation detects X11 but I'm on Wayland

**Symptoms:**
- You selected Wayland session at login
- Swictation says "Display Server: X11"

**Diagnosis:**
```bash
# Check actual session type
echo $XDG_SESSION_TYPE   # What does this show?

# Check all display variables
env | grep -E "(DISPLAY|WAYLAND|XDG_SESSION)"

# Expected for Wayland:
# XDG_SESSION_TYPE=wayland
# WAYLAND_DISPLAY=wayland-0
# DISPLAY=:0  # (XWayland compatibility)
```

**Possible causes:**

**1. Actually on X11 session**
```bash
# You may have selected X11 by accident at login
# Check login screen for session selector

# Solution: Log out, select Wayland session
# Usually "Ubuntu" or "GNOME" (not "on Xorg")
```

**2. XDG_SESSION_TYPE not set**
```bash
# Old system or custom setup may not set this
echo $XDG_SESSION_TYPE   # Empty?

# If Wayland running but XDG_SESSION_TYPE empty:
export XDG_SESSION_TYPE=wayland
# Add to ~/.profile or ~/.bashrc

# Or update login manager configuration
```

**3. Started from terminal/SSH**
```bash
# Display variables only set in graphical session
# If starting from TTY/SSH, they won't be available

# Solution: Start daemon from desktop environment autostart
```

---

### Swictation detects Wayland but I'm on X11

**Diagnosis:**
```bash
echo $XDG_SESSION_TYPE   # Should show "x11"
echo $DISPLAY            # Should show ":0"
echo $WAYLAND_DISPLAY    # Should be empty

# If WAYLAND_DISPLAY is set, you might be on XWayland
```

**Cause: XWayland confusion**
- You're on Wayland session
- Running X11 app creates DISPLAY variable
- Both DISPLAY and WAYLAND_DISPLAY set
- Swictation correctly detects Wayland

**Verification:**
```bash
# Check your actual session at login
# "Ubuntu" = Wayland
# "Ubuntu on Xorg" = X11

# Or check process:
ps aux | grep -i wayland  # If results, you're on Wayland
ps aux | grep Xorg        # If results, you're on X11
```

---

## Text Injection Not Working

### Text appears but in wrong application

**Symptoms:**
- Run swictation, text appears in different window
- Not the active application

**Cause:** Focus/timing issue

**Solution:**
```bash
# Ensure application is focused before dictating
# Click in text field first
# Wait for cursor to appear
# Then start dictation
```

---

### Text appears garbled or has extra characters

**Symptoms:**
- Dictated "Hello" but got "HHeelllloo"
- Or missing characters

**Diagnosis:**
```bash
# Check tool latency
time xdotool type "test"   # Should be <50ms
time ydotool type "test"   # May be slower

# Check system load
top
# High CPU/memory usage can cause delays
```

**Solutions:**

**1. Switch to faster tool (if possible)**
```bash
# X11: Use xdotool (fastest)
sudo apt install xdotool

# Wayland (non-GNOME): Use wtype
sudo apt install wtype
```

**2. Check for conflicting software**
```bash
# Other automation tools may interfere
# Disable: autokey, xbindkeys, etc.

systemctl --user status autokey
systemctl --user stop autokey  # If running
```

**3. Reduce system load**
```bash
# Close unnecessary applications
# Check background processes
```

---

### Text doesn't appear at all

**Symptoms:**
- Swictation says "typing" but nothing appears
- No errors shown

**Diagnosis steps:**

**1. Verify tool works independently**
```bash
# Test the tool directly
xdotool type "test"    # X11
wtype "test"           # Wayland (non-GNOME)
ydotool type "test"    # Universal

# Click in a text editor first!
# Cursor must be in text field
```

**2. Check application accepts input**
```bash
# Some applications block programmatic input
# Try in different app:
# - gedit (usually works)
# - Terminal (usually works)
# - Web browser (may block)
```

**3. Check permissions (ydotool only)**
```bash
groups | grep input   # Must show "input"
ls -l /dev/uinput     # Should be rw- for input group
```

**4. Check daemon running**
```bash
systemctl --user status swictation-daemon
# Should show "active (running)"

# Check logs
journalctl --user -u swictation-daemon -n 50
# Look for errors
```

---

## Performance Issues

### Text appears very slowly (>1 second delay)

**Symptoms:**
- Noticeable lag between dictation and text appearing
- Each character takes time

**Diagnosis:**
```bash
# Check which tool is being used
# ydotool is slowest (~50ms per character)

# Check logs
journalctl --user -u swictation-daemon | grep "selected_tool"

# Measure latency
time ydotool type "a"  # Check if >50ms
```

**Solutions:**

**1. Use faster tool if possible**
```bash
# X11: xdotool (~10ms) is much faster
# Switch to X11 session if ydotool too slow

# At login: Select "Ubuntu on Xorg"
sudo apt install xdotool
```

**2. Check ydotool daemon**
```bash
# ydotool has a daemon component
systemctl --user status ydotoold  # Should be running

# If not:
systemctl --user start ydotoold
systemctl --user enable ydotoold
```

**3. System performance**
```bash
# Check CPU usage
top
# If swictation-daemon using >50% CPU, may be other issue

# Check disk I/O
iotop  # Slow disk can affect everything
```

---

## Environment Detection

### How to verify your environment

**Complete diagnostic:**
```bash
#!/bin/bash

echo "=== Display Server Detection ==="
echo "XDG_SESSION_TYPE: $XDG_SESSION_TYPE"
echo "XDG_CURRENT_DESKTOP: $XDG_CURRENT_DESKTOP"
echo "WAYLAND_DISPLAY: $WAYLAND_DISPLAY"
echo "DISPLAY: $DISPLAY"
echo

echo "=== Available Tools ==="
which xdotool && echo "✅ xdotool: $(xdotool --version 2>&1 | head -1)"
which wtype && echo "✅ wtype: installed"
which ydotool && echo "✅ ydotool: $(ydotool --version 2>&1)"
echo

echo "=== Permissions (ydotool) ==="
groups | grep input && echo "✅ User in input group" || echo "❌ Not in input group"
ls -l /dev/uinput 2>/dev/null && echo "✅ /dev/uinput accessible"
echo

echo "=== Recommendation ==="
if [ "$XDG_SESSION_TYPE" = "x11" ]; then
    echo "You're on X11. Install: sudo apt install xdotool"
elif [ "$XDG_SESSION_TYPE" = "wayland" ]; then
    if [[ "$XDG_CURRENT_DESKTOP" == *"GNOME"* ]]; then
        echo "You're on GNOME Wayland. Install: sudo apt install ydotool"
        echo "Then: sudo usermod -aG input $USER (and log out/in)"
    else
        echo "You're on Wayland ($XDG_CURRENT_DESKTOP). Install: sudo apt install wtype"
    fi
else
    echo "Unknown environment. Install ydotool for universal support."
fi
```

**Save as `check-swictation-env.sh` and run:**
```bash
chmod +x check-swictation-env.sh
./check-swictation-env.sh
```

---

## Getting Help

### Check Swictation Logs

```bash
# Recent errors
journalctl --user -u swictation-daemon -n 100 | grep -i error

# All logs
journalctl --user -u swictation-daemon -f

# Specific time range
journalctl --user -u swictation-daemon --since "1 hour ago"
```

### Enable Debug Logging

```bash
# Set environment variable
export RUST_LOG=debug

# Restart daemon
systemctl --user restart swictation-daemon

# View debug logs
journalctl --user -u swictation-daemon -f
```

### Report Bug with Environment Info

When reporting issues, include:

```bash
# Generate diagnostic report
cat > swictation-debug.txt <<EOF
=== System Info ===
Distribution: $(lsb_release -ds 2>/dev/null || cat /etc/os-release | grep PRETTY_NAME)
Kernel: $(uname -r)

=== Display Server ===
XDG_SESSION_TYPE=$XDG_SESSION_TYPE
XDG_CURRENT_DESKTOP=$XDG_CURRENT_DESKTOP
WAYLAND_DISPLAY=$WAYLAND_DISPLAY
DISPLAY=$DISPLAY

=== Installed Tools ===
xdotool: $(which xdotool && xdotool --version 2>&1 | head -1 || echo "not installed")
wtype: $(which wtype && echo "installed" || echo "not installed")
ydotool: $(which ydotool && ydotool --version 2>&1 || echo "not installed")

=== Permissions ===
User groups: $(groups)
/dev/uinput: $(ls -l /dev/uinput 2>&1)

=== Daemon Status ===
$(systemctl --user status swictation-daemon 2>&1)

=== Recent Logs ===
$(journalctl --user -u swictation-daemon -n 50 2>&1)
EOF

echo "Debug info saved to swictation-debug.txt"
```

Then attach `swictation-debug.txt` to bug report.

---

## See Also

- [Display Server Guide](display-servers.md) - Technical background
- [Installation by Distribution](installation-by-distro.md) - Setup for your distro
- [Tool Comparison](tool-comparison.md) - Feature comparison

---

**Last updated:** 2024-11-15
