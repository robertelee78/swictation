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

# Add autostart for daemon (optional)
echo ""
read -p "Start Swictation daemon automatically with Sway? (Y/n): " -n 1 -r
echo

if [[ ! $REPLY =~ ^[Nn]$ ]]; then
    if grep -q "exec.*swictationd" "$SWAY_CONFIG"; then
        echo "  Daemon autostart already configured"
    else
        cat >> "$SWAY_CONFIG" << 'EOF'
exec python3 /opt/swictation/src/swictationd.py
EOF
        echo "✓ Daemon autostart added to Sway config"
    fi
fi

# Show instructions
echo ""
echo "======================================================================"
echo "Setup Complete!"
echo "======================================================================"
echo ""
echo "Keybinding added:"
echo "  Mod1+Shift+d (Alt+Shift+d) → Toggle recording"
echo ""
echo "Next steps:"
echo "  1. Reload Sway config: swaymsg reload"
echo "  2. OR restart Sway to apply changes"
echo "  3. Test: Press Alt+Shift+d and speak"
echo ""
echo "To revert changes:"
echo "  cp $SWAY_CONFIG$BACKUP_SUFFIX $SWAY_CONFIG"
echo "  swaymsg reload"
echo ""
echo "======================================================================"
