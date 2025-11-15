# X11 Validation Guide

## Overview

This guide walks through manual validation of Swictation's X11 support. Complete all steps to verify that X11 detection, tool selection, and text injection work correctly.

**Estimated Time:** 2-3 hours (includes one reboot to X11 session)

---

## Prerequisites

Before starting, ensure:
- ✅ You have access to an X11 session (not just XWayland)
- ✅ `xdotool` is installed: `sudo apt install xdotool`
- ✅ Swictation is built and installed
- ✅ You're currently on Wayland (to compare before/after)

---

## Phase 1: Pre-Reboot Baseline (Wayland)

**Goal:** Establish baseline behavior on Wayland before switching to X11.

### 1.1 Environment Check (Wayland)

Run the validation script:
```bash
./scripts/validate-x11.sh
```

**Expected output:**
```
Display Server: wayland
Desktop Environment: GNOME
Expected Tool: ydotool (or wtype for KDE/Sway)
⚠ You're on Wayland, not X11
```

### 1.2 Baseline Dictation Test (Wayland)

1. Start daemon: `swictation-daemon`
2. Check logs: Should show "Display Server: Wayland"
3. Press hotkey to start recording
4. Dictate: "Hello world period This is a test period"
5. Stop recording
6. **Record latency:** Time from speech end to text appearance: _____ ms

### 1.3 Note Wayland Performance

Record baseline metrics:
- Text injection latency: _____ ms
- CPU usage (idle): _____ %
- Memory usage: _____ MB

**Now proceed to reboot into X11.**

---

## Phase 2: X11 Session Setup

### 2.1 Reboot into X11

1. Log out of current session
2. At login screen, look for gear icon (⚙️) or "Session" selector
3. Select **"GNOME on Xorg"** or **"X11"** session (NOT "GNOME" which is Wayland)
4. Log in

**Common session names:**
- Ubuntu/GNOME: "GNOME on Xorg" or "Ubuntu on Xorg"
- KDE: "Plasma (X11)"
- Other DEs: Look for "X11" or "Xorg" suffix

### 2.2 Verify X11 Environment

Run validation script:
```bash
./scripts/validate-x11.sh
```

**Expected output:**
```
✓ DISPLAY: :0
✓ WAYLAND_DISPLAY: Not set (pure X11)
✓ XDG_SESSION_TYPE: x11
✓ Session Type: Pure X11
✓ xdotool: /usr/bin/xdotool
✓ Expected tool (xdotool) is installed
✓ Ready for X11 testing!
```

If you see any ✗ errors, fix them before proceeding.

---

## Phase 3: X11 Functional Testing

### Test 1: Basic Dictation (5 minutes)

**Goal:** Verify basic text injection works on X11.

1. Open text editor (gedit, kate, or any text field)
2. Start daemon: `swictation-daemon`
3. Check daemon logs: `journalctl --user -u swictation-daemon -f`
4. **Verify logs show:** `Display Server: X11`
5. **Verify logs show:** `Selected tool: xdotool`
6. Press hotkey to start recording
7. Dictate: "Hello world period This is a test period"
8. Stop recording
9. **Verify text appears:** "Hello world. This is a test."

**✅ Pass criteria:**
- Logs correctly identify X11
- Text injection works
- Punctuation transformation works ("period" → ".")

---

### Test 2: ASCII Character Support (5 minutes)

**Goal:** Verify STT outputs ASCII only (no Unicode).

Dictate the following in separate recordings:

**Symbols:**
```
"dollar sign at sign hashtag percent"
```
**Expected:** `$ @ # %`

**Punctuation:**
```
"period comma question mark exclamation"
```
**Expected:** `. , ? !`

**Numbers:**
```
"one two three four five"
```
**Expected:** `1 2 3 4 5` (if secretary mode enabled) or `one two three four five` (literal)

**Unicode test (should fail):**
```
"café résumé naïve"
```
**Expected:** `cafe resume naive` (NO accents - STT limitation)

**✅ Pass criteria:**
- ASCII characters work correctly
- No Unicode output (confirms STT limitation documented correctly)

---

### Test 3: Secretary Mode Commands (5 minutes)

**Goal:** Verify text transformations work identically to Wayland.

Dictate:
```
"caps hello world caps off"
```
**Expected:** `HELLO WORLD`

Dictate:
```
"new line new line new line"
```
**Expected:** Three line breaks

Dictate:
```
"period comma question mark exclamation mark"
```
**Expected:** `. , ? !`

**✅ Pass criteria:**
- All transformations match Wayland behavior exactly
- No regressions

---

### Test 4: Hotkey Registration (2 minutes)

**Goal:** Verify hotkey detection works on X11.

1. Restart daemon: `systemctl --user restart swictation-daemon`
2. Check logs: `journalctl --user -u swictation-daemon -n 50`
3. **Verify:** Hotkey registration logged
4. Press hotkey multiple times
5. **Verify:** Recording toggles correctly
6. **Verify:** No crashes or errors

**✅ Pass criteria:**
- Hotkey registers successfully
- Multiple presses work
- No errors in logs

---

### Test 5: Error Handling (3 minutes)

**Goal:** Verify error messages show correct tool for X11.

1. Stop daemon: `systemctl --user stop swictation-daemon`
2. Rename xdotool: `sudo mv /usr/bin/xdotool /usr/bin/xdotool.bak`
3. Start daemon: `swictation-daemon`
4. Check error message in logs

**Expected error:**
```
Error: Text injection tool not found

For X11, install xdotool:
  sudo apt install xdotool

Environment detected:
  XDG_SESSION_TYPE: x11
  DISPLAY: :0
  WAYLAND_DISPLAY: (not set)
```

**Verify:**
- ✅ Error mentions "xdotool" (not wtype or ydotool)
- ✅ Shows installation instructions
- ✅ Shows detected environment

5. Restore xdotool: `sudo mv /usr/bin/xdotool.bak /usr/bin/xdotool`
6. Restart daemon: `systemctl --user restart swictation-daemon`

**✅ Pass criteria:**
- Error message accurate for X11
- Helpful installation instructions
- Environment variables shown for debugging

---

### Test 6: Performance Testing (5 minutes)

**Goal:** Verify X11 performance is within ±20% of Wayland.

1. Open text editor
2. Start recording
3. Dictate continuously for 30 seconds (any content)
4. Stop recording
5. **Measure latency:** Time from speech end to text appearance: _____ ms
6. Compare to Wayland baseline from Phase 1

**Record metrics:**

| Metric | Wayland | X11 | Delta | Acceptable? |
|--------|---------|-----|-------|-------------|
| Text injection latency | _____ ms | _____ ms | _____ ms | ≤±20% |
| CPU usage (idle) | _____ % | _____ % | _____ % | Similar |
| Memory usage | _____ MB | _____ MB | _____ MB | Similar |

**✅ Pass criteria:**
- Latency within ±20% of Wayland
- CPU/memory usage similar
- No performance regressions

---

## Phase 4: Comparison Testing

### Reboot Back to Wayland

1. Log out
2. Select "GNOME" or "Wayland" session at login
3. Log back in

### Side-by-Side Validation (10 minutes)

Run same dictation tests on Wayland and verify:
- ✅ Identical behavior
- ✅ Similar performance
- ✅ No regressions

---

## Phase 5: Documentation Validation

### Verify Documentation Accuracy (5 minutes)

1. **README.md:** Follow installation instructions
2. **docs/display-servers.md:** Verify technical accuracy
3. **docs/tool-comparison.md:** Check tool recommendations
4. **docs/troubleshooting-display-servers.md:** Try diagnostic scripts

**✅ Pass criteria:**
- All links work
- Instructions accurate
- Examples match actual behavior
- STT ASCII limitation documented clearly

---

## Acceptance Checklist

Mark all as complete before signing off:

**Must-Have (All Required):**
- [ ] Automatic display server detection works (no manual config)
- [ ] Text injection works correctly on X11
- [ ] Hotkeys work on X11
- [ ] Error messages show correct tool for X11
- [ ] No regression in Wayland functionality
- [ ] Secretary mode commands work identically
- [ ] ASCII text injection works correctly
- [ ] Performance within ±20% of Wayland
- [ ] Documentation accurate (STT limitations documented)
- [ ] All test scenarios pass

**Should-Have (Nice to Have):**
- [ ] Environment switching after reboot seamless
- [ ] Error messages helpful and actionable
- [ ] Logs provide useful debugging info

---

## Bug Reporting

If you find issues during testing:

### Capture Diagnostic Information

```bash
# 1. Save error logs
journalctl --user -u swictation-daemon -n 100 > x11-error.log

# 2. Save environment
env | grep -E "(DISPLAY|WAYLAND|XDG)" > x11-env.txt

# 3. Run validation script
./scripts/validate-x11.sh > x11-validation.txt 2>&1
```

### Create Archon Issue

```bash
# Example Archon issue
Task Title: "X11 text injection fails on [describe issue]"
Description:
- Steps to reproduce
- Expected behavior
- Actual behavior
- Attached: x11-error.log, x11-env.txt, x11-validation.txt
```

---

## Success Criteria

**Phase 6 complete when:**
1. ✅ All 8 test scenarios pass
2. ✅ Performance benchmarks acceptable
3. ✅ No critical bugs found
4. ✅ Documentation validated
5. ✅ User confirms X11 support works as expected

**THEN:** X11/Wayland dual support is officially production-ready! 🚀

---

## Quick Reference Commands

```bash
# Environment validation
./scripts/validate-x11.sh

# Start daemon
swictation-daemon

# View logs (live)
journalctl --user -u swictation-daemon -f

# View logs (last 50 lines)
journalctl --user -u swictation-daemon -n 50

# Restart daemon
systemctl --user restart swictation-daemon

# Stop daemon
systemctl --user stop swictation-daemon

# Check tool availability
which xdotool wtype ydotool

# Check environment
echo $XDG_SESSION_TYPE
echo $DISPLAY
echo $WAYLAND_DISPLAY
echo $XDG_CURRENT_DESKTOP
```

---

**Last updated:** 2024-11-15
