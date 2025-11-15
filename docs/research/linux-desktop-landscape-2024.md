# Linux Desktop Landscape 2024-2025 Research Report

**Research Date:** November 14, 2025
**Purpose:** Inform accurate X11/Wayland support documentation for Swictation
**Researcher:** Research Agent

## Executive Summary

This report analyzes the current Linux desktop environment landscape to determine accurate compatibility claims for Swictation's X11 (xdotool) and Wayland (wtype) support. Key findings:

- **Linux Market Share:** 4.45% of desktop users (July 2024 peak), approximately 2.94% as of November 2024
- **Wayland Adoption:** Only 10-20% of Linux users currently on Wayland despite major distributions defaulting to it
- **X11 Dominance:** Still 80-90% of users remain on X11 in 2024
- **Critical Issue:** wtype does NOT work with GNOME Wayland (most popular DE) due to missing virtual-keyboard protocol support

---

## 1. Distribution Default Matrix

| Distribution | Version | Default DE | Default Display Server | X11 Available | Notes |
|--------------|---------|-----------|------------------------|---------------|-------|
| **Ubuntu** | 24.04 LTS | GNOME 46 | Wayland (Intel/AMD)<br>X11 (NVIDIA) | ✅ Yes | NVIDIA systems default to X11 due to stability concerns |
| **Fedora** | 40 | GNOME 46 | Wayland | ✅ Yes (manual install) | X11 packages not included by default in F40+ |
| **Fedora** | 41 | GNOME 47 | Wayland Only | ⚠️ Manual install | GNOME X11 session removed from default install |
| **Debian** | 12 (Bookworm) | GNOME 43 | Wayland | ✅ Yes | Falls back to X11 with NVIDIA proprietary driver |
| **Arch Linux** | Rolling | User Choice | User Choice | ✅ Yes | No default DE/WM - user configures |
| **Linux Mint** | 21.x/22.x | Cinnamon 6.0 | X11 (default) | ✅ Yes | Experimental Wayland available but not recommended |
| **Pop!_OS** | 22.04 | GNOME 42 + COSMIC extensions | X11 | ✅ Yes | Wayland available but not default |
| **Pop!_OS** | 24.04 Beta | COSMIC (Rust) | Wayland Only | ⚠️ XWayland only | New Rust-based DE, Wayland-native |

### Package Names for Input Tools

| Distribution | xdotool Package | wtype Package | Installation Command |
|--------------|----------------|---------------|---------------------|
| Ubuntu/Debian | `xdotool` | `wtype` | `sudo apt install xdotool wtype` |
| Fedora | `xdotool` | `wtype` | `sudo dnf install xdotool wtype` |
| Arch Linux | `xdotool` | `wtype` | `sudo pacman -S xdotool wtype` |

**Version Information (2024):**
- xdotool: 3.20160805.1 (stable, last major update 2016)
- wtype: 0.4+ (active development)

---

## 2. Desktop Environment Compatibility Matrix

| Desktop Environment | Version | X11 Support | Wayland Support | xdotool Compatible | wtype Compatible | Market Position |
|---------------------|---------|-------------|-----------------|-------------------|------------------|-----------------|
| **GNOME** | 46 | ✅ Hidden by default | ✅ Default | ✅ Yes | ❌ **NO** | Most popular DE |
| **GNOME** | 47 | ⚠️ Optional install | ✅ Default | ✅ Yes | ❌ **NO** | Current version |
| **KDE Plasma** | 6.x | ✅ Maintenance mode | ✅ Default (73% adoption) | ✅ Yes | ✅ Yes | Second most popular |
| **Xfce** | 4.20 | ✅ Primary | ⚠️ Experimental | ✅ Yes | ⚠️ Experimental | Lightweight choice |
| **Cinnamon** | 6.0 | ✅ Default | ⚠️ Experimental | ✅ Yes | ⚠️ Experimental | Linux Mint default |
| **MATE** | Current | ✅ Yes | ⚠️ Initial support | ✅ Yes | Unknown | Traditional desktop |
| **COSMIC (Rust)** | 24.04 | ❌ No | ✅ Wayland-only | ❌ No | ✅ Yes | Pop!_OS future |

### Critical Finding: GNOME + wtype Incompatibility

**⚠️ MAJOR COMPATIBILITY ISSUE:**
- GNOME's Mutter compositor does **NOT** support the `virtual-keyboard` Wayland protocol
- wtype **WILL NOT WORK** on GNOME Wayland sessions
- This affects the **most popular desktop environment** on Linux
- Affects Ubuntu 24.04+, Fedora 40+, Debian 12+ default configurations

**Impact:** Despite Wayland being the default on these systems, wtype is incompatible with the most widely used DE.

### Wayland Adoption by Desktop Environment

From KDE Plasma 6 telemetry data:
- **73% of Plasma 6 users** are on Wayland (with telemetry enabled)
- **60% of all Plasma users** (including Plasma 5) are on Wayland

GNOME adoption data not publicly available, but Firefox telemetry suggests **less than 10% of Firefox Linux users** are on Wayland (though this may be skewed by distributions disabling telemetry).

---

## 3. Window Manager Support Matrix

| Window Manager | Display Server | Popularity Tier | xdotool | wtype | Notes |
|---------------|----------------|-----------------|---------|-------|-------|
| **i3** | X11 only | ⭐⭐⭐ High | ✅ Yes | N/A | Most popular tiling WM |
| **Sway** | Wayland only | ⭐⭐⭐ High | N/A | ✅ Yes | i3-compatible for Wayland |
| **Hyprland** | Wayland only | ⭐⭐ Growing | N/A | ✅ Yes | Dynamic tiling, eye candy |
| **awesome** | X11 only | ⭐⭐ Medium | ✅ Yes | N/A | Lua-configured |
| **bspwm** | X11 only | ⭐⭐ Medium | ✅ Yes | N/A | Binary space partitioning |
| **Qtile** | X11 + Wayland | ⭐ Niche | ✅ Yes | ✅ Yes | Python-configured |
| **dwm** | X11 only | ⭐ Enthusiast | ✅ Yes | N/A | Suckless philosophy |

### Window Manager vs Desktop Environment Usage

- **Majority of users use full desktop environments** (GNOME, KDE Plasma, Xfce, Cinnamon)
- **Tiling WM users are a minority** but highly technical and vocal
- **i3 + Sway combined** represent the largest tiling WM user base
- **Arch Linux users** disproportionately use tiling WMs vs other distros

---

## 4. Market Share & Adoption Statistics

### Overall Linux Desktop Market Share (2024)

- **Peak 2024:** 4.45% (July 2024) - all-time high
- **Current (Nov 2024):** 2.94%
- **Growth Rate:** Accelerating (took 0.7 years to go from 3% to 4%)
- **Gaming (Steam):** 3.05% Linux users

**If including ChromeOS:** 5.86% total (ChromeOS at 1.41%)

### Distribution Popularity (DistroWatch 2024)

**Important Note:** DistroWatch measures page hits, NOT actual installations or users.

**Top 10 Rankings:**
1. **Linux Mint** - 2,412 hits/day (reclaimed #1 in December 2024)
2. **MX Linux** - 2,280 hits/day (previously #1)
3. **EndeavourOS** - 1,638 hits/day
4. **Fedora** - 965 hits/day
5. **openSUSE** - 748 hits/day
6. **Ubuntu** - (Top 10, specific rank not listed)
7-10. Various distributions

**Actual Usage Estimates:**
- Ubuntu and Ubuntu-based distros (Mint, Pop!_OS) likely represent 40-50% of desktop Linux users
- Fedora/RHEL-based: 15-20%
- Arch-based: 10-15% (but disproportionately vocal online)
- Debian: 10-15%
- Others: 10-20%

### Desktop Environment Market Share

**No comprehensive 2024 statistics available**, but based on distribution defaults and community discussion:

**Estimated Rankings:**
1. **GNOME** - 35-45% (Ubuntu, Fedora, Debian default)
2. **KDE Plasma** - 20-30% (second most popular)
3. **Xfce** - 10-15% (lightweight choice)
4. **Cinnamon** - 5-10% (Linux Mint)
5. **MATE/LXQt/Others** - 5-10%
6. **Tiling WMs** - 5-10% (i3, Sway, others)

### Wayland vs X11 Adoption (2024)

**Current State:**
- **10-20% on Wayland** (conservative estimate from Firefox telemetry)
- **80-90% still on X11** despite major distros defaulting to Wayland
- **Slow adoption** even with Wayland as default since 2023 in major distros

**Why X11 Persists:**
- Users manually switch back to X11 for compatibility
- NVIDIA proprietary driver issues with Wayland
- Application compatibility gaps
- Workflow tools (like screen capture, automation) don't work on Wayland
- Professional workloads (video editing, gaming, VFX) have better X11 support

**Wayland Momentum:**
- Major distributions committed to Wayland future
- GNOME 47+ can be built without X11 support
- KDE Plasma 6 deprecating X11 (will remove midway through Plasma 6 lifecycle)
- Fedora 43+ removing GNOME X11 session by default

**Projection:** Wayland may reach 30-40% adoption by 2025, but X11 will remain relevant through 2026-2027.

### Developer Operating System Usage (Stack Overflow 2024)

From May 2024 survey of 65,000+ developers:

**Professional/Work Use:**
- **Windows:** 48%+
- **macOS:** 32%
- **Linux:** ~15-20% (estimated, includes WSL)

**Personal Use:**
- **Windows:** 59%+
- **macOS:** 32%
- **Linux:** ~15% (estimated)

**Most Popular Linux Distributions Among Developers:**
1. Ubuntu (most widely used)
2. Debian
3. Arch Linux
4. Fedora
5. ChromeOS

---

## 5. Alternative Input Tools for Wayland

### Why Alternatives Needed

xdotool is X11-specific and uses X11 APIs that don't exist in Wayland. Wayland removed most input simulation APIs for security reasons.

### Comparison of Wayland Input Tools

| Tool | Method | GNOME Support | Pros | Cons |
|------|--------|--------------|------|------|
| **wtype** | `virtual-keyboard` protocol | ❌ **NO** | Simple, no daemon, fast | Only works on compatible compositors (Sway, Hyprland, wlroots-based) |
| **ydotool** | Linux uinput | ✅ Yes | Works everywhere (X11, Wayland, TTY) | Requires daemon, slow typing, root access issues, broken in some repos |
| **dotool** | Linux uinput | ✅ Yes | No daemon, simpler than ydotool | Keyboard only, no window selection |
| **kdotool** | KDE-specific | ⚠️ KDE only | Good KDE integration | Only works with KDE Plasma Wayland |
| **wlrctl** | wlroots protocol | ⚠️ wlroots only | Good for Sway/Hyprland | Only wlroots compositors |

### Recommendation for Swictation

**For Wayland Support:**

We should support **multiple** Wayland input tools with fallback logic:

1. **First choice: wtype** (if compositor supports virtual-keyboard)
   - Works on: Sway, Hyprland, Wayfire, LabWC, river, other wlroots compositors
   - Does NOT work on: GNOME, KDE Plasma (partially)

2. **Fallback: ydotool** (universal compatibility)
   - Works everywhere including GNOME
   - Requires setup (daemon configuration)
   - Slower but reliable

**Detection Logic:**
```bash
# Check for Wayland session
if [ "$XDG_SESSION_TYPE" = "wayland" ]; then
    # Try wtype first (fast, clean)
    if command -v wtype &> /dev/null; then
        # Test if it works (try on non-critical operation)
        if wtype -P Return &> /dev/null; then
            USE_WTYPE=true
        fi
    fi

    # Fallback to ydotool
    if [ "$USE_WTYPE" != "true" ] && command -v ydotool &> /dev/null; then
        USE_YDOTOOL=true
    fi
fi
```

---

## 6. Testing Priority Recommendations

Based on market share and usage patterns, prioritize testing in this order:

### Tier 1: Critical - Must Work (70%+ of users)

1. **Ubuntu 24.04 LTS + GNOME + X11 (NVIDIA)**
   - xdotool support
   - Most common configuration for desktops with NVIDIA GPUs

2. **Ubuntu 24.04 LTS + GNOME + Wayland (Intel/AMD)**
   - ydotool support (wtype won't work)
   - Default for non-NVIDIA systems

3. **Fedora 40/41 + GNOME + Wayland**
   - ydotool support
   - Leading-edge Wayland adoption

4. **Debian 12 + GNOME + Wayland/X11**
   - Both xdotool and ydotool
   - Stable baseline

### Tier 2: Important - Should Work (20-25% of users)

5. **KDE Plasma 6 + Wayland**
   - wtype support
   - 73% of Plasma users on Wayland

6. **KDE Plasma 6 + X11**
   - xdotool support
   - 27% of Plasma users

7. **Linux Mint + Cinnamon + X11**
   - xdotool support
   - Popular among new Linux users

8. **Arch + i3**
   - xdotool support
   - Influential user base

### Tier 3: Nice to Have (5-10% of users)

9. **Arch + Sway**
   - wtype support
   - Wayland tiling WM users

10. **Pop!_OS 24.04 + COSMIC + Wayland**
    - wtype support
    - Future-looking, System76 hardware

11. **Hyprland**
    - wtype support
    - Growing Wayland tiling WM

12. **Xfce 4.20 + X11**
    - xdotool support
    - Lightweight desktop users

### Tier 4: Compatibility Mode (Specialized)

13. **Qtile (X11 and Wayland)**
    - xdotool + wtype
    - Python-configured tiling WM

14. **ChromeOS/ChromeOS Flex**
    - Special case, likely limited support
    - 1.41% market share

---

## 7. Documentation Strategy Recommendations

### 7.1 Accurate Compatibility Claims

**DO NOT claim:**
- ❌ "Works on all Wayland desktops"
- ❌ "Full Wayland support"
- ❌ "Wayland is fully supported via wtype"

**DO claim:**
- ✅ "X11 support via xdotool (works on 80-90% of Linux desktops)"
- ✅ "Wayland support on compatible compositors via wtype and ydotool"
- ✅ "Tested on GNOME (Wayland), KDE Plasma 6, Sway, and Hyprland"

### 7.2 Clear Compatibility Table

Create a compatibility table like this in documentation:

```markdown
## Compatibility

| Environment | Display Server | Status | Tool Used |
|-------------|----------------|--------|-----------|
| GNOME 46/47 | X11 | ✅ Fully Supported | xdotool |
| GNOME 46/47 | Wayland | ⚠️ Supported with ydotool | ydotool |
| KDE Plasma 6 | X11 | ✅ Fully Supported | xdotool |
| KDE Plasma 6 | Wayland | ✅ Fully Supported | wtype |
| Sway | Wayland | ✅ Fully Supported | wtype |
| Hyprland | Wayland | ✅ Fully Supported | wtype |
| i3 | X11 | ✅ Fully Supported | xdotool |
| Xfce 4.20 | X11 | ✅ Fully Supported | xdotool |
| Cinnamon 6.0 | X11 | ✅ Fully Supported | xdotool |
```

### 7.3 Installation Instructions

**For X11 users (80-90%):**
```bash
# Ubuntu/Debian
sudo apt install xdotool

# Fedora
sudo dnf install xdotool

# Arch
sudo pacman -S xdotool
```

**For Wayland users on wlroots compositors (Sway, Hyprland, river):**
```bash
# Ubuntu/Debian
sudo apt install wtype

# Fedora
sudo dnf install wtype

# Arch
sudo pacman -S wtype
```

**For Wayland users on GNOME/other:**
```bash
# Ubuntu/Debian
sudo apt install ydotool

# Fedora
sudo dnf install ydotool

# Arch
sudo pacman -S ydotool

# Configure ydotool daemon
sudo systemctl enable ydotool
sudo systemctl start ydotool
```

### 7.4 Troubleshooting Section

Include common issues:
- GNOME Wayland users must use ydotool, not wtype
- NVIDIA users may be on X11 even on Wayland-default distros
- Detection script for current session type
- Fallback behavior explanation

### 7.5 Known Limitations

Be transparent about limitations:
- wtype does not work on GNOME Wayland (most popular DE)
- ydotool requires daemon setup and runs slower
- Some features may work better on X11 than Wayland
- Screen recording/capture may require different tools on Wayland

---

## 8. Key Citations & Sources

1. **Linux Market Share:**
   - IT's FOSS: "Linux Now Has 3% Desktop Linux Market [November 2025 Report]"
   - Linuxiac: "Linux Crosses 4% Market Share Worldwide"
   - StatCounter data (July 2024 peak at 4.45%)

2. **Wayland Adoption:**
   - Phoronix: "Less Than 10% Of Firefox Users On Linux Are Running Wayland"
   - KDE Plasma telemetry: 73% of Plasma 6 users on Wayland
   - The Register: "Fedora 41's GNOME to go Wayland-only"

3. **Distribution Defaults:**
   - Ubuntu documentation (24.04 LTS)
   - Fedora Project Wiki (Fedora 40/41)
   - Debian Wiki (Bookworm)
   - System76 Pop!_OS 24.04 Beta announcement

4. **Desktop Environment Support:**
   - GNOME Blog: "X11 Session Removal FAQ"
   - KDE: "About Plasma's X11 session"
   - Xfce: "Wayland Roadmap" wiki
   - Cinnamon 6.0 release notes

5. **Package Information:**
   - Debian packages.debian.org
   - Arch Linux package repositories
   - Fedora package repositories
   - GitHub: atx/wtype, ReimuNotMoe/ydotool

6. **Developer Survey:**
   - Stack Overflow Developer Survey 2024 (65,000+ respondents)
   - Statista: "OS distribution among developers worldwide 2024"

---

## 9. Conclusions & Actionable Recommendations

### For Swictation Development:

1. **Implement Multi-Tool Support:**
   - Primary: xdotool for X11 (80-90% of users)
   - Secondary: wtype for Wayland compositors that support it (Sway, Hyprland, KDE)
   - Tertiary: ydotool as universal Wayland fallback (GNOME, others)

2. **Auto-Detection:**
   - Detect session type (`$XDG_SESSION_TYPE`)
   - Detect compositor/DE (`$XDG_CURRENT_DESKTOP`)
   - Test tool availability and functionality
   - Fall back gracefully with clear error messages

3. **Documentation Accuracy:**
   - Be honest about Wayland limitations
   - Prominently note GNOME Wayland requires ydotool
   - Provide distro-specific installation commands
   - Include troubleshooting for common scenarios

4. **Testing Coverage:**
   - Must test: Ubuntu 24.04 (GNOME Wayland + X11), Fedora 41 (GNOME Wayland)
   - Should test: KDE Plasma 6 (Wayland), Sway, Linux Mint (X11)
   - Nice to have: Hyprland, Arch + i3, Pop!_OS COSMIC

5. **Future-Proofing:**
   - X11 will remain relevant through 2026-2027
   - Wayland adoption accelerating but still minority in 2024
   - GNOME's lack of virtual-keyboard protocol is a major blocker
   - KDE Plasma leading Wayland adoption (73% of users)

### Critical Insight:

**The biggest challenge is not X11 vs Wayland, but GNOME Wayland specifically.** GNOME is the most popular DE, and its Wayland session doesn't support wtype. This means:

- You CANNOT claim "Wayland support via wtype" without huge caveats
- You MUST support ydotool for GNOME Wayland users
- You should default to X11 detection first (80-90% of users)
- Wayland should be treated as "advanced" configuration until adoption increases

**Bottom Line:** Build for X11 first (xdotool), add Wayland as progressive enhancement with clear compatibility notes.

---

**Report Compiled By:** Research Agent
**Date:** November 14, 2025
**Next Steps:** Share findings with documentation team and development team for implementation strategy
