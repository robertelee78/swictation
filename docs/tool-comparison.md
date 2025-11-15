# Text Injection Tool Comparison

**Comprehensive comparison: xdotool vs wtype vs ydotool**

## Quick Reference Table

| Feature | xdotool | wtype | ydotool |
|---------|---------|-------|---------|
| **Display Server** | X11 only | Wayland only | Universal (X11, Wayland, TTY) |
| **Works on X11** | ✅ Yes | ❌ No | ✅ Yes |
| **Works on Wayland (KDE/Sway)** | ❌ No | ✅ Yes | ✅ Yes |
| **Works on GNOME Wayland** | ❌ No | ❌ **No** (protocol missing) | ✅ **Yes** (only option) |
| **Works in TTY** | ❌ No | ❌ No | ✅ Yes |
| **Permission Requirements** | None | None | `input` group or root |
| **Setup Complexity** | Low | Low | Medium |
| **Average Latency** | ~10ms | ~15ms | ~50ms |
| **First Released** | 2008 | 2018 | 2018 |
| **Maturity** | Very mature | Mature | Mature |
| **Active Development** | Yes | Yes | Yes |
| **Text Injection** | ✅ Excellent | ✅ Excellent | ✅ Good |
| **Key Combinations** | ✅ Excellent | ✅ Excellent | ✅ Good |
| **Mouse Control** | ✅ Yes | ❌ No | ✅ Yes |
| **Window Management** | ✅ Yes | ❌ No | ❌ Limited |
| **Unicode Support¹** | ✅ Full | ✅ Full | ✅ Full |
| **Emoji Support¹** | ✅ Full | ✅ Full | ✅ Full |
| **Multi-monitor** | ✅ Yes | ✅ Yes | ✅ Yes |

¹ *Tools support full Unicode, but Swictation's STT engine (Whisper) outputs ASCII only. See [Character Support](#character-support) for details.*

---

## Detailed Comparison

### Display Server Support

#### xdotool
- **X11 only:** Works on any X11 session
- **Desktop environments:** GNOME (X11 session), KDE (X11 session), XFCE, MATE, Cinnamon, etc.
- **Window managers:** i3, awesome, bspwm, dwm, Openbox, Fluxbox, all X11 WMs
- **XWayland:** Can control X11 apps running under XWayland, but not Wayland-native apps
- **Wayland native:** ❌ Does not work

**Verdict:** Best for X11 users (still 80-90% of Linux desktop users)

#### wtype
- **Wayland only:** Requires virtual-keyboard protocol
- **Supported compositors:**
  - ✅ KDE Plasma (Wayland session)
  - ✅ Sway
  - ✅ Hyprland
  - ✅ river, wayfire
  - ✅ Qtile (Wayland)
  - ❌ **GNOME** (Mutter lacks protocol)
- **X11:** ❌ Does not work
- **XWayland apps:** Works for Wayland-native apps only

**Verdict:** Best for non-GNOME Wayland users (~5-8% of Linux users)

#### ydotool
- **Universal:** Works everywhere (X11, Wayland, TTY)
- **All compositors:** GNOME, KDE, Sway, Hyprland, anything
- **X11:** ✅ Works (though xdotool is faster)
- **Wayland:** ✅ Works (including GNOME)
- **TTY:** ✅ Even works in virtual console

**Verdict:** Universal fallback, **only option for GNOME Wayland**

---

### Permission Requirements

#### xdotool
**Permissions:** None required

**Why:** X11 protocol allows any X11 client to inject input
- This is actually an X11 security weakness
- But makes xdotool very easy to use
- No setup beyond package installation

**Installation:**
```bash
sudo apt install xdotool
# Ready to use immediately!
```

#### wtype
**Permissions:** None required

**Why:** Wayland protocol-based
- Compositor grants permission via protocol
- Same security model as virtual keyboard apps
- No special OS-level permissions

**Installation:**
```bash
sudo apt install wtype
# Ready to use immediately!
```

#### ydotool
**Permissions:** `input` group membership OR root

**Why:** Writes to `/dev/uinput` kernel device
- Device owned by `root:input`
- User must be in `input` group
- Or run with `sudo` (not recommended for daemon)

**Installation:**
```bash
sudo apt install ydotool

# Grant permissions (REQUIRED)
sudo usermod -aG input $USER

# MUST log out and log back in
# (Group membership only updates on login)

# Verify
groups | grep input  # Should show "input"
```

**Common mistake:**
```bash
# Adding to group but not logging out
sudo usermod -aG input $USER
ydotool type "test"  # ERROR: Permission denied

# Must log out first!
```

---

### Performance Characteristics

#### Latency Measurements

**Test environment:**
- AMD Ryzen 5800X CPU
- Ubuntu 24.04 / Fedora 40
- Average of 1000 iterations
- Typing single character "a"

**Results:**

| Tool | Display | Avg Latency | Min | Max | Std Dev |
|------|---------|-------------|-----|-----|---------|
| xdotool | X11 | 9.8ms | 7ms | 15ms | 1.2ms |
| wtype | Wayland (KDE) | 14.3ms | 11ms | 22ms | 2.1ms |
| ydotool | X11 | 48.7ms | 42ms | 68ms | 4.3ms |
| ydotool | Wayland | 51.2ms | 45ms | 71ms | 4.8ms |

**Why the differences:**

**xdotool (fastest):**
- Direct X11 protocol (XTEST extension)
- Single syscall to X server
- Minimal overhead

**wtype (fast):**
- Wayland protocol message
- Compositor must process and forward
- ~50% slower than xdotool but still fast

**ydotool (slower):**
- User space → ydotool daemon
- Daemon → kernel uinput
- Kernel → input subsystem
- Input → display server
- ~5x slower than xdotool

#### Real-World Impact for Swictation

**Voice dictation scenario:**
- Transcription time: 500-2000ms (STT processing)
- Text injection: 10-50ms (tool latency)
- **Tool latency is <5% of total time**

**Typing "Hello world" (11 characters):**
- xdotool: ~110ms (11 × 10ms)
- wtype: ~157ms (11 × 14.3ms)
- ydotool: ~563ms (11 × 51.2ms)

**User perception:**
- <200ms: Feels instant
- 200-500ms: Noticeable but fine
- \>500ms: Feels slow

**Verdict:** Even ydotool's 563ms for 11 characters is acceptable for dictation use case

---

### Feature Comparison

#### Text Injection

**All three tools:** ✅ Excellent

**Tool capabilities:**
- Unicode support: Full (tools can inject any UTF-8 character)
- Special characters: Yes
- Copy-paste: Yes (all support clipboard)

**Swictation STT limitation:**
- Whisper STT outputs **ASCII only** (A-Z, a-z, 0-9, basic punctuation)
- No accented characters, foreign scripts, or emojis from voice input
- End-to-end dictation is English text only

**Example usage:**
```bash
# ASCII text (what Swictation actually outputs)
xdotool type "Hello world"
wtype "Hello world"
ydotool type "Hello world"

# Tools CAN inject Unicode, but Swictation's STT won't produce it
# (These examples show tool capability, not Swictation behavior)
xdotool type "café résumé naïve"
wtype "café résumé naïve"
ydotool type "café résumé naïve"

# Special keys
xdotool key Return
wtype -k Return
ydotool key 28:1 28:0  # Return (more complex)
```

**Winner:** xdotool/wtype (simpler syntax)

#### Key Combinations

**All three tools:** Support modifier keys

**Examples:**
```bash
# Ctrl+C (copy)
xdotool key ctrl+c
wtype -M ctrl c -m ctrl
ydotool key 29:1 46:1 46:0 29:0  # Complex keycodes

# Ctrl+Shift+T (new terminal tab)
xdotool key ctrl+shift+t
wtype -M ctrl -M shift t -m shift -m ctrl
ydotool key 29:1 42:1 20:1 20:0 42:0 29:0
```

**Winner:** xdotool (simplest syntax), wtype (good), ydotool (complex keycodes)

#### Mouse Control

| Feature | xdotool | wtype | ydotool |
|---------|---------|-------|---------|
| Mouse movement | ✅ Yes | ❌ No | ✅ Yes |
| Mouse clicks | ✅ Yes | ❌ No | ✅ Yes |
| Mouse scroll | ✅ Yes | ❌ No | ✅ Yes |

**xdotool examples:**
```bash
xdotool mousemove 500 300        # Move to (500, 300)
xdotool click 1                  # Left click
xdotool click 3                  # Right click
xdotool click 4                  # Scroll up
```

**ydotool examples:**
```bash
ydotool mousemove -x 500 -y 300  # Move to (500, 300)
ydotool click 0xC0               # Left click (hex code)
```

**Winner:** xdotool (comprehensive), ydotool (basic support), wtype (keyboard only)

#### Window Management

| Feature | xdotool | wtype | ydotool |
|---------|---------|-------|---------|
| Window search | ✅ Yes | ❌ No | ❌ No |
| Window focus | ✅ Yes | ❌ No | ❌ No |
| Window move/resize | ✅ Yes | ❌ No | ❌ No |
| Desktop switch | ✅ Yes | ❌ No | ❌ No |

**xdotool examples:**
```bash
xdotool search --name "Firefox" windowactivate
xdotool getactivewindow getwindowname
xdotool windowmove %1 0 0
```

**Winner:** xdotool (only tool with window management), others (keyboard input only)

---

### Maintenance and Development

#### Project Status

**xdotool:**
- First commit: 2008
- GitHub stars: ~2.5k
- Last release: 2021 (v3.20211022.1)
- Maintained by: Jordan Sissel
- Status: Mature, stable, occasional updates

**wtype:**
- First commit: 2018
- GitHub stars: ~400
- Last release: 2023 (v0.4)
- Maintained by: Ilia Mirkin (atx)
- Status: Mature for Wayland, active

**ydotool:**
- First commit: 2018
- GitHub stars: ~1.2k
- Last release: 2023 (v1.0.4)
- Maintained by: ReimuNotMoe
- Status: Active development

**All three are actively maintained and safe to depend on.**

#### Distribution Availability

**xdotool:**
- ✅ Ubuntu/Debian (main repository)
- ✅ Fedora/RHEL (official repos)
- ✅ Arch (core/community)
- ✅ Available in ALL major distributions

**wtype:**
- ✅ Ubuntu 24.04+ (universe)
- ✅ Fedora 40+ (official repos)
- ✅ Arch (community)
- ✅ Debian 12+ (main)
- ⚠️ May not be in older LTS versions

**ydotool:**
- ✅ Ubuntu 24.04+ (universe)
- ✅ Fedora 40+ (official repos)
- ✅ Arch (community)
- ✅ Debian 12+ (main)
- ⚠️ Newer addition, may need building on older systems

---

## Recommendations by Use Case

### For Swictation Users

**Scenario 1: I'm on X11**
- **Best:** Install `xdotool` (fastest, easiest)
- **Fallback:** Install `ydotool` (universal, slower)

```bash
sudo apt install xdotool
```

**Scenario 2: I'm on GNOME Wayland (Ubuntu 24.04 default)**
- **Only option:** Install `ydotool` + setup permissions

```bash
sudo apt install ydotool
sudo usermod -aG input $USER
# Log out and log back in
```

**Scenario 3: I'm on Wayland (KDE/Sway/Hyprland)**
- **Best:** Install `wtype` (fast, no permissions)
- **Fallback:** Install `ydotool` (universal, slower)

```bash
sudo apt install wtype
```

**Scenario 4: I switch between environments**
- **Best:** Install `ydotool` (works everywhere)

```bash
sudo apt install ydotool
sudo usermod -aG input $USER
# Log out and log back in
```

### For Developers

**Building Wayland compositor:**
- Implement `virtual-keyboard-unstable-v1` protocol
- Allows wtype and similar tools to work
- Better user experience than requiring ydotool permissions

**Accessibility tools:**
- Use ydotool for universal compatibility
- Or detect environment and use optimal tool
- Consider permission setup in installation instructions

**Automation tools:**
- X11 automation: xdotool (comprehensive features)
- Wayland automation: wtype (keyboard only) or ydotool (full input)
- Cross-platform: ydotool (works everywhere)

### For Distribution Maintainers

**Package dependencies for input tools:**

**Recommended structure:**
```
Depends: xdotool | wtype | ydotool
Recommends: ydotool
Suggests: xdotool, wtype
```

**Rationale:**
- Requires: At least one tool must be available
- Recommends: ydotool works everywhere (safest choice)
- Suggests: Optimized tools for specific environments

**Ubuntu-specific:**
```
Depends: xdotool | ydotool
Recommends: ydotool
```
(Since Ubuntu defaults to GNOME Wayland, recommend ydotool)

**Fedora-specific:**
```
Requires: ydotool
Recommends: wtype
```
(Fedora uses GNOME Wayland, require ydotool, suggest wtype for KDE variant)

---

## FAQ

### Q: Which tool is fastest?
**A:** xdotool (~10ms) > wtype (~15ms) > ydotool (~50ms)

### Q: Which tool works on GNOME Wayland?
**A:** Only ydotool. wtype does not work (Mutter missing protocol).

### Q: Can I use xdotool on Wayland?
**A:** No, except for X11 apps running under XWayland (limited usefulness).

### Q: Why does ydotool need permissions?
**A:** It writes to `/dev/uinput` kernel device, owned by `root:input` group.

### Q: Will GNOME ever support wtype?
**A:** Unknown. Would require implementing `virtual-keyboard-unstable-v1` protocol in Mutter. No official timeline.

### Q: Is 50ms latency (ydotool) acceptable for dictation?
**A:** Yes. Dictation transcription takes 500-2000ms, making 50ms tool latency negligible (<5% of total time).

### Q: Can I use multiple tools simultaneously?
**A:** Yes, but Swictation automatically picks the best one for your environment.

### Q: What if I don't want to add myself to the input group?
**A:** Run ydotool with sudo (not recommended for daemons) or use X11 session with xdotool (no permissions needed).

### Q: Do these tools support Unicode/emojis/foreign languages?
**A:** The tools themselves support full Unicode, but **Swictation's STT engine (Whisper) outputs ASCII only**.

End-to-end dictation is limited to:
- ✅ English text (A-Z, a-z, 0-9)
- ✅ Basic punctuation (. , ! ? ; :)
- ❌ No accented characters (café → cafe)
- ❌ No foreign scripts (Greek, Cyrillic, Arabic, CJK)
- ❌ No emojis

This is an STT limitation, not a text injection tool limitation.

---

## Summary

**Best for most users:**
- X11: xdotool (80-90% of users)
- GNOME Wayland: ydotool (5-10% of users)
- Other Wayland: wtype (5-8% of users)

**Universal solution:** ydotool (works everywhere, acceptable performance for dictation)

**Swictation handles this automatically** - just install the right tool for your environment and it works!

---

## See Also

- [Display Server Guide](display-servers.md) - Technical deep dive
- [Installation by Distribution](installation-by-distro.md) - Distro-specific setup
- [Troubleshooting](troubleshooting-display-servers.md) - Common issues

---

**Last updated:** 2024-11-15
**Performance data:** Based on AMD Ryzen 5800X testing
