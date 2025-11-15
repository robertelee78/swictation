# Linux Desktop Landscape Research - Executive Summary

**Date:** November 14, 2025
**Researcher:** Research Agent
**Full Report:** [linux-desktop-landscape-2024.md](./linux-desktop-landscape-2024.md)

---

## 🎯 Critical Findings

### The GNOME Problem
**⚠️ MAJOR ISSUE:** wtype (Wayland typing tool) does **NOT** work on GNOME Wayland sessions.

- **Why:** GNOME's Mutter compositor lacks `virtual-keyboard` Wayland protocol support
- **Impact:** GNOME is the **most popular desktop environment** (35-45% of Linux users)
- **Affected:** Ubuntu 24.04, Fedora 40/41, Debian 12 (all default to GNOME Wayland)

### Reality Check: X11 Still Dominates

- **80-90% of Linux users** remain on X11 (as of November 2024)
- **Only 10-20%** have adopted Wayland despite major distros defaulting to it
- **Why:** Compatibility issues, NVIDIA drivers, workflow tools, user inertia

---

## 📊 Quick Stats

| Metric | Value | Source |
|--------|-------|--------|
| Linux Desktop Market Share | 2.94% (Nov 2024) | StatCounter |
| Peak Market Share 2024 | 4.45% (July 2024) | StatCounter |
| Wayland Adoption | 10-20% | Firefox telemetry |
| X11 Users | 80-90% | Estimate from sources |
| KDE Plasma 6 Wayland Adoption | 73% | KDE telemetry |

---

## 🏆 Top Desktop Environments

1. **GNOME** (35-45%) - Wayland default, **wtype incompatible**
2. **KDE Plasma** (20-30%) - Wayland 73% adoption, **wtype compatible**
3. **Xfce** (10-15%) - X11 primary, Wayland experimental
4. **Cinnamon** (5-10%) - X11 default (Linux Mint)
5. **Tiling WMs** (5-10%) - i3 (X11), Sway (Wayland)

---

## 🛠️ Tool Compatibility Matrix

| Tool | Method | GNOME Wayland | KDE Wayland | Sway/Hyprland | Speed | Setup |
|------|--------|---------------|-------------|---------------|-------|-------|
| **xdotool** | X11 API | ✅ (X11 session) | ✅ (X11 session) | ❌ N/A | Fast | Easy |
| **wtype** | virtual-keyboard | ❌ **NO** | ✅ Yes | ✅ Yes | Fast | Easy |
| **ydotool** | Linux uinput | ✅ Yes | ✅ Yes | ✅ Yes | Slow | Complex |

---

## ✅ Recommended Strategy for Swictation

### Implementation Priority

1. **Primary:** xdotool for X11 (covers 80-90% of users)
2. **Secondary:** wtype for compatible Wayland compositors (KDE, Sway, Hyprland)
3. **Fallback:** ydotool for GNOME Wayland and universal compatibility

### Detection Logic

```bash
# Pseudo-code for tool selection
if X11_session:
    use xdotool
elif Wayland_session:
    if compositor_supports_virtual_keyboard:
        try wtype (fast)
    else:
        use ydotool (GNOME and others)
```

### Auto-Detection Strategy

1. Check `$XDG_SESSION_TYPE` (x11 vs wayland)
2. Check `$XDG_CURRENT_DESKTOP` (gnome, kde, sway, etc.)
3. Test tool availability (`command -v xdotool/wtype/ydotool`)
4. Test tool functionality (try non-critical operation)
5. Fall back gracefully with clear error messages

---

## 📋 Testing Priorities

### Tier 1: Must Work (70%+ users)
- ✅ Ubuntu 24.04 + GNOME + X11 (NVIDIA)
- ✅ Ubuntu 24.04 + GNOME + Wayland → **must use ydotool**
- ✅ Fedora 41 + GNOME + Wayland → **must use ydotool**
- ✅ Debian 12 + GNOME

### Tier 2: Should Work (20-25% users)
- KDE Plasma 6 + Wayland (wtype)
- KDE Plasma 6 + X11 (xdotool)
- Linux Mint + Cinnamon (xdotool)
- Arch + i3 (xdotool)

### Tier 3: Nice to Have (5-10% users)
- Sway (wtype)
- Hyprland (wtype)
- Pop!_OS 24.04 COSMIC (wtype)

---

## 📝 Documentation Guidelines

### ✅ DO Say:
- "Supports X11 via xdotool (works for 80-90% of Linux users)"
- "Wayland support on compatible compositors (KDE Plasma, Sway, Hyprland)"
- "GNOME Wayland users: requires ydotool (see installation guide)"
- "Tested on Ubuntu 24.04, Fedora 41, KDE Plasma 6, Sway"

### ❌ DON'T Say:
- "Full Wayland support"
- "Works on all Wayland desktops"
- "Wayland support via wtype" (without caveats)

### Include This Warning:
> **Note for GNOME Users:** If you're using GNOME on Wayland (default on Ubuntu 24.04+, Fedora 40+), you'll need to install ydotool instead of wtype, as GNOME's compositor doesn't support the virtual-keyboard protocol. See the [GNOME Wayland Setup Guide](#) for details.

---

## 🔮 Future Outlook

- **X11 Relevance:** Will remain important through 2026-2027
- **Wayland Growth:** Expect 30-40% adoption by end of 2025
- **GNOME Issue:** No indication GNOME will add virtual-keyboard protocol support
- **KDE Leadership:** Plasma 6 leading Wayland adoption (73% of users)
- **Recommendation:** Build for X11 first, add Wayland as progressive enhancement

---

## 📚 Key Sources

1. **Market Share:** StatCounter, IT's FOSS, Linuxiac
2. **Wayland Adoption:** Phoronix (Firefox telemetry), KDE telemetry
3. **Distribution Info:** Official Ubuntu, Fedora, Debian documentation
4. **Package Info:** Debian packages.debian.org, Arch repos, Fedora repos
5. **DE Support:** GNOME Blog, KDE documentation, Xfce roadmap
6. **Developer Survey:** Stack Overflow 2024 (65,000+ respondents)

---

## 🎬 Next Actions

1. **Development Team:** Implement multi-tool detection and fallback logic
2. **Documentation Team:** Create clear compatibility table and installation guides
3. **Testing Team:** Set up test environments for Tier 1 configurations
4. **QA Team:** Verify ydotool setup process on GNOME Wayland systems

**Full detailed report with all data, citations, and tables:** [linux-desktop-landscape-2024.md](./linux-desktop-landscape-2024.md)
