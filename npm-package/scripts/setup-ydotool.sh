#!/bin/bash
# Automated ydotool setup for GNOME Wayland
# This script handles uinput module loading, udev rules, and user group membership

set -e

SCRIPT_NAME="Swictation ydotool Setup"
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() {
    echo -e "${CYAN}ℹ${NC} $1"
}

log_success() {
    echo -e "${GREEN}✓${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

log_error() {
    echo -e "${RED}✗${NC} $1"
}

echo -e "${CYAN}═══════════════════════════════════════${NC}"
echo -e "${CYAN}  $SCRIPT_NAME${NC}"
echo -e "${CYAN}═══════════════════════════════════════${NC}"
echo ""

# Check if running on Wayland
if [ -z "$WAYLAND_DISPLAY" ] && [ -z "$XDG_SESSION_TYPE" ] || [ "$XDG_SESSION_TYPE" != "wayland" ]; then
    log_warning "Not running on Wayland - ydotool may not be needed"
    log_info "This script is primarily for Wayland systems (GNOME, Sway)"
    read -p "Continue anyway? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 0
    fi
fi

# 1. Check if ydotool is installed
log_info "Checking for ydotool..."
if ! command -v ydotool &> /dev/null; then
    log_warning "ydotool not installed"
    log_info "Installing ydotool and netcat..."

    if command -v apt &> /dev/null; then
        sudo apt update && sudo apt install -y ydotool netcat-openbsd
    elif command -v dnf &> /dev/null; then
        sudo dnf install -y ydotool nmap-ncat
    elif command -v pacman &> /dev/null; then
        sudo pacman -S --noconfirm ydotool openbsd-netcat
    else
        log_error "Unsupported package manager"
        log_info "Please install ydotool and netcat manually"
        exit 1
    fi

    log_success "ydotool installed"
else
    log_success "ydotool already installed"
fi

# 2. Load uinput kernel module
log_info "Loading uinput kernel module..."
if lsmod | grep -q uinput; then
    log_success "uinput module already loaded"
else
    sudo modprobe uinput
    log_success "uinput module loaded"
fi

# 3. Make uinput load on boot
log_info "Configuring uinput to load on boot..."
if [ -f /etc/modules-load.d/uinput.conf ]; then
    log_success "uinput boot configuration already exists"
else
    echo "uinput" | sudo tee /etc/modules-load.d/uinput.conf > /dev/null
    log_success "Created /etc/modules-load.d/uinput.conf"
fi

# 4. Create udev rule for /dev/uinput permissions
log_info "Configuring /dev/uinput permissions..."
UDEV_RULE='/etc/udev/rules.d/80-uinput.rules'
UDEV_CONTENT='KERNEL=="uinput", GROUP="input", MODE="0660"'

if [ -f "$UDEV_RULE" ]; then
    if grep -q "$UDEV_CONTENT" "$UDEV_RULE"; then
        log_success "udev rule already configured"
    else
        log_warning "udev rule exists but differs - updating"
        echo "$UDEV_CONTENT" | sudo tee "$UDEV_RULE" > /dev/null
        log_success "Updated udev rule"
    fi
else
    echo "$UDEV_CONTENT" | sudo tee "$UDEV_RULE" > /dev/null
    log_success "Created $UDEV_RULE"
fi

# 5. Reload udev rules
log_info "Reloading udev rules..."
sudo udevadm control --reload-rules
sudo udevadm trigger
log_success "udev rules reloaded"

# 6. Add user to input group
log_info "Adding user to 'input' group..."
if groups | grep -q '\binput\b'; then
    log_success "User already in input group"
    GROUP_ADDED=false
else
    sudo usermod -aG input "$USER"
    log_success "User added to input group"
    GROUP_ADDED=true
fi

# 7. Verify /dev/uinput permissions
log_info "Verifying /dev/uinput permissions..."
sleep 1  # Give udev time to apply
UINPUT_PERMS=$(ls -l /dev/uinput 2>/dev/null || echo "")
if echo "$UINPUT_PERMS" | grep -q "root input"; then
    log_success "/dev/uinput permissions correct: root:input 0660"
else
    log_warning "/dev/uinput permissions may need manual adjustment"
    log_info "Expected: root:input 0660"
    log_info "Current: $UINPUT_PERMS"
fi

# 8. Test ydotool
echo ""
log_info "Testing ydotool access..."
if $GROUP_ADDED; then
    log_warning "Group membership requires logout/login or newgrp"
    log_info "Testing with 'sg input' wrapper..."
    if sg input -c 'ydotool type test' 2>&1 | grep -q "failed to open"; then
        log_error "ydotool test failed - permissions issue"
        log_info "Try: newgrp input"
    else
        log_success "ydotool works with 'sg input' wrapper"
    fi
else
    if ydotool type test 2>&1 | grep -q "failed to open"; then
        log_error "ydotool test failed - permissions issue"
        log_info "Try: newgrp input"
    else
        log_success "ydotool works directly"
    fi
fi

# Summary
echo ""
echo -e "${GREEN}═══════════════════════════════════════${NC}"
echo -e "${GREEN}  Setup Complete!${NC}"
echo -e "${GREEN}═══════════════════════════════════════${NC}"
echo ""

if $GROUP_ADDED; then
    log_warning "IMPORTANT: Group membership requires logout/login"
    log_info "Or run: newgrp input (in current shell)"
    echo ""
    log_info "To test ydotool:"
    echo "  sg input -c 'ydotool type \"Hello World\"'"
else
    log_info "ydotool is ready to use!"
    echo "  ydotool type \"Hello World\""
fi

echo ""
log_info "Swictation daemon will automatically use ydotool on Wayland"
