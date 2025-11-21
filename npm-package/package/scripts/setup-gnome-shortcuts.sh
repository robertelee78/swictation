#!/bin/bash
# Automated GNOME keyboard shortcut configuration for Swictation
# Sets up Super+Shift+D hotkey to toggle recording via IPC socket

set -e

SCRIPT_NAME="Swictation GNOME Shortcuts Setup"
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

# Check if running GNOME
if [ "$XDG_CURRENT_DESKTOP" != "ubuntu:GNOME" ] && [ "$XDG_CURRENT_DESKTOP" != "GNOME" ]; then
    log_warning "Not running GNOME desktop"
    log_info "This script is for GNOME Wayland systems"
    log_info "Current desktop: ${XDG_CURRENT_DESKTOP:-unknown}"
    echo ""
    log_info "For other desktops, configure hotkeys manually to call:"
    echo "  sh -c 'echo \"{\\\"action\\\": \\\"toggle\\\"}\" | nc -U /tmp/swictation.sock'"
    exit 1
fi

# Check if gsettings is available
if ! command -v gsettings &> /dev/null; then
    log_error "gsettings not found - cannot configure GNOME shortcuts"
    exit 1
fi

# Check if netcat is available
if ! command -v nc &> /dev/null; then
    log_warning "netcat not installed - installing..."
    sudo apt install -y netcat-openbsd || sudo dnf install -y nmap-ncat || sudo pacman -S openbsd-netcat
    log_success "netcat installed"
fi

log_info "Configuring GNOME keyboard shortcut for Swictation..."
echo ""

# Configure custom keybinding
KEYBINDING_PATH="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/swictation-toggle/"

# Get current custom keybindings
CURRENT_BINDINGS=$(gsettings get org.gnome.settings-daemon.plugins.media-keys custom-keybindings)

# Check if our binding already exists
if echo "$CURRENT_BINDINGS" | grep -q "swictation-toggle"; then
    log_success "Swictation keyboard shortcut already configured"
else
    # Add our binding to the list
    gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings "['$KEYBINDING_PATH']"
    log_success "Added Swictation to custom keybindings list"
fi

# Configure the toggle keybinding
gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KEYBINDING_PATH name "Swictation Toggle"
gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KEYBINDING_PATH command "sh -c 'echo \"{\\\"action\\\": \\\"toggle\\\"}\" | nc -U /tmp/swictation.sock'"
gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KEYBINDING_PATH binding "<Super><Shift>d"

log_success "Configured keyboard shortcut: Super+Shift+D"
echo ""

# Verify configuration
BINDING_NAME=$(gsettings get org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KEYBINDING_PATH name)
BINDING_CMD=$(gsettings get org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KEYBINDING_PATH command)
BINDING_KEY=$(gsettings get org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KEYBINDING_PATH binding)

log_info "Verification:"
echo "  Name: $BINDING_NAME"
echo "  Key:  $BINDING_KEY"
echo "  Cmd:  $BINDING_CMD"

echo ""
echo -e "${GREEN}═══════════════════════════════════════${NC}"
echo -e "${GREEN}  Setup Complete!${NC}"
echo -e "${GREEN}═══════════════════════════════════════${NC}"
echo ""

log_info "You can now toggle recording with: Super+Shift+D"
log_info "The shortcut is also visible in:"
echo "  Settings → Keyboard → View and Customize Shortcuts → Custom Shortcuts"
echo ""
log_warning "Note: The swictation daemon must be running for hotkeys to work"
log_info "Start daemon with: swictation start"
