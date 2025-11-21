#!/bin/bash
# Swictation Installation Verification Script
# Checks all components and provides troubleshooting guidance

set -e

SCRIPT_NAME="Swictation Installation Verification"
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m'

CHECKS_PASSED=0
CHECKS_FAILED=0
WARNINGS=0

log_check() {
    echo -e "${CYAN}→${NC} $1"
}

log_pass() {
    echo -e "${GREEN}  ✓${NC} $1"
    ((CHECKS_PASSED++))
}

log_fail() {
    echo -e "${RED}  ✗${NC} $1"
    ((CHECKS_FAILED++))
}

log_warn() {
    echo -e "${YELLOW}  ⚠${NC} $1"
    ((WARNINGS++))
}

log_info() {
    echo -e "${CYAN}  ℹ${NC} $1"
}

echo -e "${CYAN}═══════════════════════════════════════${NC}"
echo -e "${CYAN}  $SCRIPT_NAME${NC}"
echo -e "${CYAN}═══════════════════════════════════════${NC}"
echo ""

# 1. Display Server Detection
log_check "Display Server Detection"
if [ -n "$WAYLAND_DISPLAY" ] || [ "$XDG_SESSION_TYPE" = "wayland" ]; then
    log_pass "Wayland detected"
    DISPLAY_SERVER="wayland"

    if [ "$XDG_CURRENT_DESKTOP" = "ubuntu:GNOME" ] || [ "$XDG_CURRENT_DESKTOP" = "GNOME" ]; then
        log_info "Desktop: GNOME on Wayland"
        DESKTOP="gnome"
    elif [ -n "$SWAYSOCK" ] && [ "$XDG_CURRENT_DESKTOP" = "sway" ]; then
        log_info "Desktop: Sway"
        DESKTOP="sway"
    else
        log_info "Desktop: Generic Wayland (${XDG_CURRENT_DESKTOP:-unknown})"
        DESKTOP="other"
    fi
elif [ -n "$DISPLAY" ]; then
    log_pass "X11 detected"
    DISPLAY_SERVER="x11"
    DESKTOP="x11"
else
    log_fail "No display server detected"
    DISPLAY_SERVER="none"
    DESKTOP="none"
fi

# 2. GPU Detection
log_check "GPU Detection"
if command -v nvidia-smi &> /dev/null; then
    GPU_INFO=$(nvidia-smi --query-gpu=name,memory.total --format=csv,noheader 2>/dev/null || echo "")
    if [ -n "$GPU_INFO" ]; then
        log_pass "NVIDIA GPU detected: $GPU_INFO"

        # Check GPU libraries
        if [ -d "$HOME/.local/share/swictation/gpu-libs" ]; then
            LIBS_COUNT=$(ls -1 "$HOME/.local/share/swictation/gpu-libs" | wc -l)
            log_pass "GPU libraries installed ($LIBS_COUNT files)"
        else
            log_warn "GPU libraries not found in ~/.local/share/swictation/gpu-libs"
            log_info "Run: npm install (should download automatically)"
        fi
    else
        log_warn "nvidia-smi present but no GPU detected"
    fi
else
    log_info "No NVIDIA GPU detected - CPU mode will be used"
fi

# 3. Text Injection Tool
log_check "Text Injection Tool"
if [ "$DISPLAY_SERVER" = "wayland" ]; then
    if command -v ydotool &> /dev/null; then
        log_pass "ydotool installed"

        # Check uinput module
        if lsmod | grep -q uinput; then
            log_pass "uinput module loaded"
        else
            log_fail "uinput module not loaded"
            log_info "Run: sudo modprobe uinput"
        fi

        # Check /dev/uinput permissions
        if [ -e /dev/uinput ]; then
            PERMS=$(ls -l /dev/uinput | awk '{print $1, $3":"$4}')
            if echo "$PERMS" | grep -q "input"; then
                log_pass "/dev/uinput permissions correct"
            else
                log_warn "/dev/uinput permissions may be incorrect: $PERMS"
                log_info "Expected: crw-rw---- root:input"
            fi
        else
            log_fail "/dev/uinput device not found"
        fi

        # Check user group membership
        if groups | grep -q '\binput\b'; then
            log_pass "User in 'input' group"
        else
            log_fail "User not in 'input' group"
            log_info "Run: sudo usermod -aG input $USER"
            log_info "Then logout/login or: newgrp input"
        fi
    else
        log_fail "ydotool not installed (required for Wayland)"
        log_info "Run: sudo apt install ydotool  (Ubuntu/Debian)"
        log_info "Or: ./scripts/setup-ydotool.sh"
    fi
elif [ "$DISPLAY_SERVER" = "x11" ]; then
    if command -v xdotool &> /dev/null; then
        log_pass "xdotool installed"
    else
        log_warn "xdotool not installed (recommended for X11)"
        log_info "Run: sudo apt install xdotool"
    fi
fi

# 4. Hotkey Configuration
log_check "Hotkey Configuration"
if [ "$DESKTOP" = "gnome" ]; then
    BINDINGS=$(gsettings get org.gnome.settings-daemon.plugins.media-keys custom-keybindings 2>/dev/null || echo "")
    if echo "$BINDINGS" | grep -q "swictation-toggle"; then
        log_pass "GNOME keyboard shortcut configured"
        BINDING=$(gsettings get org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/swictation-toggle/ binding 2>/dev/null)
        log_info "Hotkey: $BINDING"
    else
        log_warn "GNOME keyboard shortcut not configured"
        log_info "Run: ./scripts/setup-gnome-shortcuts.sh"
    fi
elif [ "$DESKTOP" = "sway" ]; then
    SWAY_CONFIG="$HOME/.config/sway/config"
    if [ -f "$SWAY_CONFIG" ] && grep -q "swictation" "$SWAY_CONFIG"; then
        log_pass "Sway hotkeys configured"
    else
        log_warn "Sway hotkeys not found in config"
        log_info "Add to ~/.config/sway/config:"
        log_info "  bindsym \$mod+Shift+d exec sh -c 'echo \"{\\\"action\\\": \\\"toggle\\\"}\" | nc -U /tmp/swictation.sock'"
    fi
fi

# 5. Systemd Service
log_check "Systemd Service"
SERVICE_FILE="$HOME/.config/systemd/user/swictation-daemon.service"
if [ -f "$SERVICE_FILE" ]; then
    log_pass "Service file exists"

    if systemctl --user is-enabled swictation-daemon.service &> /dev/null; then
        log_pass "Service enabled"
    else
        log_warn "Service not enabled"
        log_info "Run: systemctl --user enable swictation-daemon.service"
    fi

    if systemctl --user is-active swictation-daemon.service &> /dev/null; then
        log_pass "Service running"
    else
        log_warn "Service not running"
        log_info "Run: systemctl --user start swictation-daemon.service"
    fi
else
    log_fail "Service file not found"
    log_info "Run: swictation setup"
fi

# 6. IPC Socket
log_check "IPC Socket"
if [ -S /tmp/swictation.sock ]; then
    log_pass "IPC socket exists"

    # Test socket connection
    if command -v nc &> /dev/null; then
        RESPONSE=$(echo '{"action": "status"}' | nc -U /tmp/swictation.sock 2>/dev/null || echo "")
        if echo "$RESPONSE" | grep -q "status"; then
            log_pass "IPC socket responding"
        else
            log_warn "IPC socket not responding correctly"
        fi
    else
        log_warn "netcat not installed - cannot test socket"
        log_info "Install: sudo apt install netcat-openbsd"
    fi
else
    log_warn "IPC socket not found (daemon may not be running)"
fi

# 7. AI Models
log_check "AI Models"
MODELS_DIR="$HOME/.local/share/swictation/models"
if [ -d "$MODELS_DIR" ]; then
    MODEL_COUNT=$(find "$MODELS_DIR" -name "*.onnx" | wc -l)
    if [ "$MODEL_COUNT" -gt 0 ]; then
        log_pass "$MODEL_COUNT model files found"
    else
        log_warn "Models directory exists but no .onnx files found"
        log_info "Download models: swictation download-model 1.1b-gpu"
    fi
else
    log_fail "Models directory not found"
    log_info "Download models: swictation download-model 1.1b-gpu"
fi

# Summary
echo ""
echo -e "${CYAN}═══════════════════════════════════════${NC}"
echo -e "${CYAN}  Summary${NC}"
echo -e "${CYAN}═══════════════════════════════════════${NC}"

if [ $CHECKS_FAILED -eq 0 ] && [ $WARNINGS -eq 0 ]; then
    echo -e "${GREEN}  ✓ All checks passed!${NC}"
    echo ""
    log_info "Swictation is fully configured and ready to use"
    log_info "Start with: swictation start"
    log_info "Toggle with: Super+Shift+D (GNOME) or configured hotkey"
elif [ $CHECKS_FAILED -eq 0 ]; then
    echo -e "${YELLOW}  ⚠ $CHECKS_PASSED checks passed, $WARNINGS warnings${NC}"
    echo ""
    log_info "Swictation should work but may need attention"
    log_info "Review warnings above for recommendations"
else
    echo -e "${RED}  ✗ $CHECKS_FAILED checks failed${NC}"
    echo -e "${GREEN}  ✓ $CHECKS_PASSED checks passed${NC}"
    echo -e "${YELLOW}  ⚠ $WARNINGS warnings${NC}"
    echo ""
    log_info "Please address failed checks before using Swictation"
    log_info "Run setup scripts as recommended above"
fi

echo ""
exit $([ $CHECKS_FAILED -eq 0 ] && echo 0 || echo 1)
