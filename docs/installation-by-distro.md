# Installation by Distribution

**Complete installation instructions for top Linux distributions**

---

## Quick Reference

| Distribution | Default Environment | Recommended Tool | Installation Command |
|--------------|-------------------|-----------------|---------------------|
| Ubuntu 24.04 | GNOME + Wayland | ydotool | `sudo apt install ydotool` |
| Fedora 40/41 | GNOME + Wayland | ydotool | `sudo dnf install ydotool` |
| Debian 12 | GNOME + Wayland | ydotool | `sudo apt install ydotool` |
| Linux Mint 21.3 | Cinnamon + X11 | xdotool | `sudo apt install xdotool` |
| Arch Linux | User choice | (varies) | `sudo pacman -S xdotool/wtype/ydotool` |
| Pop!_OS 22.04 | GNOME + X11/Wayland | xdotool/ydotool | `sudo apt install xdotool ydotool` |
| openSUSE TW | KDE + Wayland | wtype | `sudo zypper install wtype` |
| Manjaro | KDE + Wayland | wtype | `sudo pacman -S wtype` |
| EndeavourOS | User choice | (varies) | `sudo pacman -S xdotool/wtype/ydotool` |

---

## Ubuntu

### Ubuntu 24.04 LTS (Noble Numbat)

**Default environment:**
- Desktop: GNOME 46
- Display server: **Wayland** (default)
- X11 session: Available (select at login)

#### Installation

**For default Wayland session (RECOMMENDED):**
```bash
# Install ydotool (only tool that works on GNOME Wayland)
sudo apt update
sudo apt install ydotool

# Grant permissions (REQUIRED)
sudo usermod -aG input $USER

# MUST log out and log back in
echo "Log out and log back in to activate group membership"
```

**Verification:**
```bash
# After logging back in
groups | grep input  # Should show "input"
ydotool type "test"  # Should type "test" into active window
```

**Alternative: Switch to X11 session**
```bash
# At login screen (GDM):
# 1. Click gear icon (bottom right)
# 2. Select "Ubuntu on Xorg"
# 3. Log in

# Then install xdotool (faster than ydotool)
sudo apt install xdotool

# No permissions needed!
xdotool type "test"
```

#### Troubleshooting

**"Permission denied" error with ydotool:**
```bash
# Verify group membership
groups | grep input

# If "input" not shown, you didn't log out
# Must log out (not just close terminal)
```

**Check which session you're on:**
```bash
echo $XDG_SESSION_TYPE  # "wayland" or "x11"
```

---

### Ubuntu 22.04 LTS (Jammy Jellyfish)

**Default environment:**
- Desktop: GNOME 42
- Display server: **X11** (default, Wayland available)

#### Installation

**For default X11 session:**
```bash
sudo apt update
sudo apt install xdotool

# No permissions needed
xdotool type "test"
```

**If using Wayland session (optional):**
```bash
sudo apt install ydotool
sudo usermod -aG input $USER
# Log out and log back in
```

---

## Fedora

### Fedora Workstation 40/41

**Default environment:**
- Desktop: GNOME 46/47
- Display server: **Wayland** (default)
- X11 session: Available but deprecated

#### Installation

**For default Wayland session:**
```bash
# Install ydotool (required for GNOME Wayland)
sudo dnf install ydotool

# Grant permissions
sudo usermod -aG input $USER

# Log out and log back in
```

**Verification:**
```bash
groups | grep input  # Should show "input"
ydotool type "test"
```

**Alternative: Switch to X11** (not recommended, deprecated)
```bash
# At login (GDM): Select "GNOME on Xorg"
sudo dnf install xdotool
xdotool type "test"
```

---

### Fedora KDE Spin 40/41

**Default environment:**
- Desktop: KDE Plasma 6
- Display server: **Wayland** (default)

#### Installation

```bash
# Install wtype (works on KDE Wayland)
sudo dnf install wtype

# No permissions needed
wtype "test"

# Optional: Install ydotool as universal fallback
sudo dnf install ydotool
sudo usermod -aG input $USER
# Log out and log back in
```

---

## Debian

### Debian 12 (Bookworm)

**Default environment:**
- Desktop: GNOME 43
- Display server: **Wayland** (default)

#### Installation

**For default Wayland session:**
```bash
sudo apt update
sudo apt install ydotool

# Grant permissions
sudo usermod -aG input $USER

# Log out and log back in
groups | grep input
ydotool type "test"
```

**For X11 session** (select at login):
```bash
sudo apt install xdotool
xdotool type "test"
```

---

## Linux Mint

### Linux Mint 21.3 (Virginia)

**Default environment:**
- Desktop: Cinnamon 5.8
- Display server: **X11** (Wayland not default)

#### Installation

```bash
sudo apt update
sudo apt install xdotool

# No permissions needed
xdotool type "test"
```

**Note:** Linux Mint focuses on X11 stability, so xdotool is the best choice.

---

## Arch Linux

**Default environment:** User chooses during installation

### For X11 Users (i3, awesome, bspwm, dwm, etc.)

```bash
sudo pacman -Syu
sudo pacman -S xdotool

# No permissions needed
xdotool type "test"
```

### For Wayland Users (Sway, Hyprland, river)

**Sway/Hyprland:**
```bash
sudo pacman -S wtype

# No permissions needed
wtype "test"

# Optional: ydotool as fallback
sudo pacman -S ydotool
sudo usermod -aG input $USER
# Log out and log back in
```

**Note:** Arch users typically know their environment, so choose the appropriate tool.

### For GNOME Wayland Users

```bash
sudo pacman -S ydotool
sudo usermod -aG input $USER
# Log out and log back in
groups | grep input
ydotool type "test"
```

### Universal Install (works everywhere)

```bash
sudo pacman -S ydotool
sudo usermod -aG input $USER
# Log out and log back in
```

---

## Pop!_OS

### Pop!_OS 22.04

**Default environment:**
- Desktop: GNOME (System76 customized)
- Display server: **X11** (NVIDIA users) or **Wayland** (AMD/Intel users)

#### For NVIDIA GPU (X11 default)

```bash
sudo apt update
sudo apt install xdotool

# No permissions needed
xdotool type "test"
```

#### For AMD/Intel GPU (Wayland default)

```bash
sudo apt install ydotool

# Grant permissions
sudo usermod -aG input $USER

# Log out and log back in
groups | grep input
ydotool type "test"
```

#### Check which you're using:

```bash
echo $XDG_SESSION_TYPE  # "wayland" or "x11"
```

---

## openSUSE

### openSUSE Tumbleweed (Rolling)

**Default environment:**
- Desktop: KDE Plasma (Wayland default) or GNOME (user choice)

#### For KDE Plasma (default)

```bash
sudo zypper refresh
sudo zypper install wtype

# No permissions needed
wtype "test"
```

#### For GNOME

```bash
sudo zypper install ydotool

# Grant permissions
sudo usermod -aG input $USER

# Log out and log back in
groups | grep input
ydotool type "test"
```

---

## Manjaro

### Manjaro KDE Edition

**Default environment:**
- Desktop: KDE Plasma 6
- Display server: **Wayland** (default)

#### Installation

```bash
sudo pacman -Syu
sudo pacman -S wtype

# No permissions needed
wtype "test"

# Optional: ydotool as fallback
sudo pacman -S ydotool
sudo usermod -aG input $USER
# Log out and log back in
```

### Manjaro GNOME Edition

```bash
sudo pacman -S ydotool
sudo usermod -aG input $USER
# Log out and log back in
groups | grep input
ydotool type "test"
```

---

## EndeavourOS

**Default environment:** User chooses during installation (Arch-based)

### Installation

**Same as Arch Linux** - choose based on your environment:

**X11 (any WM):**
```bash
sudo pacman -S xdotool
```

**Wayland (Sway/Hyprland/KDE):**
```bash
sudo pacman -S wtype
```

**Wayland (GNOME):**
```bash
sudo pacman -S ydotool
sudo usermod -aG input $USER
# Log out and log back in
```

---

## Universal Installation (All Distributions)

**For users who switch environments or want maximum compatibility:**

### Ubuntu/Debian-based:
```bash
sudo apt update
sudo apt install ydotool
sudo usermod -aG input $USER
# Log out and log back in
```

### Fedora/RHEL-based:
```bash
sudo dnf install ydotool
sudo usermod -aG input $USER
# Log out and log back in
```

### Arch-based:
```bash
sudo pacman -S ydotool
sudo usermod -aG input $USER
# Log out and log back in
```

### openSUSE:
```bash
sudo zypper install ydotool
sudo usermod -aG input $USER
# Log out and log back in
```

**Advantages:**
- ✅ Works on X11, Wayland, TTY
- ✅ Works on all compositors (including GNOME)
- ✅ One tool, all environments

**Trade-off:**
- ~50ms latency vs 10-15ms for xdotool/wtype
- Requires `input` group permissions

---

## Verification Steps

### Check your environment

```bash
# Display server
echo $XDG_SESSION_TYPE  # "x11", "wayland", or empty

# Desktop environment
echo $XDG_CURRENT_DESKTOP  # "GNOME", "KDE", "sway", etc.

# Display variables
echo $DISPLAY  # Usually ":0" or ":1" for X11
echo $WAYLAND_DISPLAY  # Usually "wayland-0" for Wayland
```

### Test tools

**xdotool (X11 only):**
```bash
which xdotool  # Check if installed
xdotool type "test"  # Should type "test"
```

**wtype (Wayland, non-GNOME only):**
```bash
which wtype
wtype "test"  # Should type "test" on KDE/Sway/Hyprland
```

**ydotool (universal):**
```bash
which ydotool
groups | grep input  # Verify permissions
ydotool type "test"  # Should type "test" everywhere
```

---

## Troubleshooting by Distribution

### Ubuntu: "Permission denied" with ydotool

**Problem:** Installed ydotool but getting permission errors

**Solution:**
```bash
# 1. Add to group
sudo usermod -aG input $USER

# 2. MUST log out and log back in
# (Not just close terminal - full logout!)

# 3. Verify
groups | grep input  # Should show "input"

# 4. Test
ydotool type "test"  # Should work now
```

### Fedora: "ydotool not found"

**Problem:** Package not available

**Solution:**
```bash
# Verify Fedora version
cat /etc/fedora-release

# ydotool available in Fedora 40+
sudo dnf install ydotool

# For older Fedora, use Rust/Cargo:
sudo dnf install rust cargo
cargo install ydotool
```

### Arch: "wtype doesn't work on GNOME"

**Problem:** Using wtype on GNOME Wayland

**Solution:**
```bash
# Check environment
echo $XDG_CURRENT_DESKTOP  # If "GNOME"...

# Install ydotool instead
sudo pacman -S ydotool
sudo usermod -aG input $USER
# Log out and log back in
```

### Debian: "Package ydotool not found"

**Problem:** Older Debian version

**Solution:**
```bash
# Check Debian version
cat /etc/debian_version

# ydotool available in Debian 12+
# For Debian 11, switch to X11 and use xdotool:
sudo apt install xdotool
```

---

## Distribution Defaults Summary

**GNOME + Wayland (requires ydotool):**
- Ubuntu 24.04+
- Fedora Workstation 40+
- Debian 12+
- RHEL 9+

**X11 (use xdotool):**
- Ubuntu 22.04 (default)
- Linux Mint (all versions)
- MX Linux
- Older distributions

**KDE Wayland (use wtype):**
- Fedora KDE Spin 40+
- openSUSE Tumbleweed
- Manjaro KDE
- Arch + KDE

**Tiling Wayland (use wtype):**
- Sway
- Hyprland
- river
- (User-configured on Arch/etc.)

---

## See Also

- [Display Server Guide](display-servers.md) - Technical details
- [Tool Comparison](tool-comparison.md) - Feature comparison
- [Troubleshooting](troubleshooting-display-servers.md) - Common issues
- [Window Manager Configs](window-manager-configs.md) - WM-specific setup

---

**Last updated:** 2024-11-15
**Based on:** Distribution defaults as of November 2024
