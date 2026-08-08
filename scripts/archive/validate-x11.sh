#!/bin/bash
# X11/Wayland Environment Validation Script
# Usage: ./scripts/validate-x11.sh

set -e

echo "=================================================="
echo "Swictation X11/Wayland Environment Validation"
echo "=================================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check functions
check_pass() {
    echo -e "${GREEN}✓${NC} $1"
}

check_fail() {
    echo -e "${RED}✗${NC} $1"
}

check_warn() {
    echo -e "${YELLOW}⚠${NC} $1"
}

# 1. Environment Detection
echo "1. Environment Variables"
echo "------------------------"
if [ -n "$DISPLAY" ]; then
    check_pass "DISPLAY: $DISPLAY"
else
    check_fail "DISPLAY: Not set"
fi

if [ -n "$WAYLAND_DISPLAY" ]; then
    check_warn "WAYLAND_DISPLAY: $WAYLAND_DISPLAY (XWayland or Wayland session)"
else
    check_pass "WAYLAND_DISPLAY: Not set (pure X11)"
fi

if [ -n "$XDG_SESSION_TYPE" ]; then
    if [ "$XDG_SESSION_TYPE" = "x11" ]; then
        check_pass "XDG_SESSION_TYPE: x11"
    elif [ "$XDG_SESSION_TYPE" = "wayland" ]; then
        check_warn "XDG_SESSION_TYPE: wayland (you're on Wayland, not X11)"
    else
        check_warn "XDG_SESSION_TYPE: $XDG_SESSION_TYPE"
    fi
else
    check_warn "XDG_SESSION_TYPE: Not set"
fi

if [ -n "$XDG_CURRENT_DESKTOP" ]; then
    check_pass "XDG_CURRENT_DESKTOP: $XDG_CURRENT_DESKTOP"
else
    check_warn "XDG_CURRENT_DESKTOP: Not set"
fi

echo ""

# 2. Display Server Detection
echo "2. Display Server Detection"
echo "---------------------------"
if [ "$XDG_SESSION_TYPE" = "x11" ]; then
    check_pass "Session Type: Pure X11"
    EXPECTED_TOOL="xdotool"
elif [ "$XDG_SESSION_TYPE" = "wayland" ]; then
    if echo "$XDG_CURRENT_DESKTOP" | grep -iq "gnome"; then
        check_pass "Session Type: GNOME Wayland"
        EXPECTED_TOOL="ydotool"
    else
        check_pass "Session Type: Wayland (non-GNOME)"
        EXPECTED_TOOL="wtype"
    fi
else
    check_warn "Session Type: Unknown (fallback detection)"
    if [ -n "$DISPLAY" ] && [ -z "$WAYLAND_DISPLAY" ]; then
        check_pass "Detected: X11 (based on DISPLAY)"
        EXPECTED_TOOL="xdotool"
    elif [ -n "$WAYLAND_DISPLAY" ]; then
        check_pass "Detected: Wayland (based on WAYLAND_DISPLAY)"
        if echo "$XDG_CURRENT_DESKTOP" | grep -iq "gnome"; then
            EXPECTED_TOOL="ydotool"
        else
            EXPECTED_TOOL="wtype"
        fi
    else
        check_fail "Cannot detect display server"
        EXPECTED_TOOL="unknown"
    fi
fi

echo ""

# 3. Tool Availability
echo "3. Text Injection Tools"
echo "-----------------------"
XDOTOOL=$(which xdotool 2>/dev/null || echo "")
WTYPE=$(which wtype 2>/dev/null || echo "")
YDOTOOL=$(which ydotool 2>/dev/null || echo "")

if [ -n "$XDOTOOL" ]; then
    check_pass "xdotool: $XDOTOOL"
else
    check_fail "xdotool: Not found (install: sudo apt install xdotool)"
fi

if [ -n "$WTYPE" ]; then
    check_pass "wtype: $WTYPE"
else
    check_warn "wtype: Not found (optional for X11)"
fi

if [ -n "$YDOTOOL" ]; then
    check_pass "ydotool: $YDOTOOL"
    # Check if user is in input group
    if groups | grep -q input; then
        check_pass "User in 'input' group (ydotool will work)"
    else
        check_warn "User NOT in 'input' group (ydotool needs: sudo usermod -aG input $USER)"
    fi
else
    check_warn "ydotool: Not found (optional for X11)"
fi

echo ""

# 4. Expected Tool Check
echo "4. Tool Selection Validation"
echo "----------------------------"
if [ "$EXPECTED_TOOL" = "xdotool" ]; then
    if [ -n "$XDOTOOL" ]; then
        check_pass "Expected tool (xdotool) is installed"
    else
        check_fail "Expected tool (xdotool) is NOT installed"
        echo "   Install: sudo apt install xdotool"
    fi
elif [ "$EXPECTED_TOOL" = "wtype" ]; then
    if [ -n "$WTYPE" ]; then
        check_pass "Expected tool (wtype) is installed"
    else
        check_fail "Expected tool (wtype) is NOT installed"
        echo "   Install: sudo apt install wtype"
    fi
elif [ "$EXPECTED_TOOL" = "ydotool" ]; then
    if [ -n "$YDOTOOL" ]; then
        check_pass "Expected tool (ydotool) is installed"
        if groups | grep -q input; then
            check_pass "User has required permissions"
        else
            check_fail "User needs 'input' group membership"
            echo "   Fix: sudo usermod -aG input $USER && newgrp input"
        fi
    else
        check_fail "Expected tool (ydotool) is NOT installed"
        echo "   Install: sudo apt install ydotool"
    fi
else
    check_warn "Cannot determine expected tool"
fi

echo ""

# 5. Swictation Installation
echo "5. Swictation Installation"
echo "-------------------------"
DAEMON=$(which swictation-daemon 2>/dev/null || echo "")
if [ -n "$DAEMON" ]; then
    check_pass "swictation-daemon: $DAEMON"
else
    check_fail "swictation-daemon: Not found in PATH"
fi

echo ""

# 6. Summary
echo "=================================================="
echo "SUMMARY"
echo "=================================================="
echo ""
echo "Display Server: $XDG_SESSION_TYPE"
echo "Desktop Environment: $XDG_CURRENT_DESKTOP"
echo "Expected Tool: $EXPECTED_TOOL"
echo ""

if [ "$XDG_SESSION_TYPE" = "x11" ] && [ -n "$XDOTOOL" ]; then
    echo -e "${GREEN}✓ Ready for X11 testing!${NC}"
    echo ""
    echo "Next steps:"
    echo "1. Start daemon: swictation-daemon"
    echo "2. Check logs for 'Display Server: X11'"
    echo "3. Test dictation with hotkey"
    exit 0
elif [ "$XDG_SESSION_TYPE" = "wayland" ]; then
    echo -e "${YELLOW}⚠ You're on Wayland, not X11${NC}"
    echo ""
    echo "To test X11 support:"
    echo "1. Log out"
    echo "2. Select 'X11' session at login screen"
    echo "3. Log back in"
    echo "4. Run this script again"
    exit 1
else
    echo -e "${RED}✗ Environment validation failed${NC}"
    echo ""
    echo "Please review errors above and fix before testing."
    exit 1
fi
