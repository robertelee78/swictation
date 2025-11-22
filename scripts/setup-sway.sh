#!/bin/bash
# Sway integration setup script for Swictation
# Adds keybinding and autostart configuration

set -e

SWAY_CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/sway/config"
SWICTATION_KEYBINDING="/opt/swictation/config/sway-keybinding.conf"
BACKUP_SUFFIX=".swictation-backup-$(date +%Y%m%d-%H%M%S)"

echo "======================================================================"
echo "Swictation Sway Integration Setup"
echo "======================================================================"

# Check if Sway config exists
if [ ! -f "$SWAY_CONFIG" ]; then
    echo "✗ Sway config not found: $SWAY_CONFIG"
    echo "  Please create a Sway config first"
    exit 1
fi

echo "✓ Found Sway config: $SWAY_CONFIG"

# Create backup
echo ""
echo "Creating backup..."
cp "$SWAY_CONFIG" "$SWAY_CONFIG$BACKUP_SUFFIX"
echo "✓ Backup created: $SWAY_CONFIG$BACKUP_SUFFIX"

# Check if keybinding already exists
if grep -q "swictation" "$SWAY_CONFIG"; then
    echo ""
    echo "⚠ Swictation keybinding already exists in Sway config"
    echo "  No changes made"
    exit 0
fi

# Add keybinding include
echo ""
echo "Adding Swictation keybinding to Sway config..."

cat >> "$SWAY_CONFIG" << 'EOF'
# ======================================================================
# Swictation voice dictation
# ======================================================================
include /opt/swictation/config/sway-keybinding.conf
EOF

echo "✓ Keybinding added to $SWAY_CONFIG"

# Daemon autostart handled by systemd
echo ""
echo "ℹ️  Daemon autostart:"
echo "  Swictation uses systemd user service for automatic startup"
echo "  Run: ./scripts/install-systemd-service.sh"
echo "  (Manual Sway exec not recommended - would create duplicate instances)"

# Show instructions
echo ""
echo "======================================================================"
echo "Setup Complete!"
echo "======================================================================"
echo ""
echo "Keybinding added:"
echo "  \$mod+Shift+d → Toggle recording"
echo "  (Uses your configured \$mod key - typically Mod4=Super/Windows or Mod1=Alt)"
echo ""
echo "Next steps:"
echo "  1. Install systemd service: ./scripts/install-systemd-service.sh"
echo "  2. Reload Sway config: swaymsg reload"
echo "  3. Test: Press \$mod+Shift+d and speak"
echo ""
echo "To revert changes:"
echo "  cp $SWAY_CONFIG$BACKUP_SUFFIX $SWAY_CONFIG"
echo "  swaymsg reload"
echo ""
echo "======================================================================"
