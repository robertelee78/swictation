#!/bin/bash
# Install Swictation systemd user service
# This script sets up auto-start for the Swictation daemon

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SERVICE_FILE="$PROJECT_DIR/config/swictation.service"
USER_SERVICE_DIR="$HOME/.config/systemd/user"
INSTALLED_SERVICE="$USER_SERVICE_DIR/swictation.service"

echo "=========================================="
echo "Swictation systemd Service Installation"
echo "=========================================="
echo ""

# Check if service file exists
if [ ! -f "$SERVICE_FILE" ]; then
    echo "❌ Error: Service file not found: $SERVICE_FILE"
    exit 1
fi

# Create user service directory if it doesn't exist
if [ ! -d "$USER_SERVICE_DIR" ]; then
    echo "📁 Creating user service directory: $USER_SERVICE_DIR"
    mkdir -p "$USER_SERVICE_DIR"
fi

# Stop existing service if running
if systemctl --user is-active --quiet swictation.service 2>/dev/null; then
    echo "⏹️  Stopping existing swictation.service..."
    systemctl --user stop swictation.service
fi

# Backup existing service file if present
if [ -f "$INSTALLED_SERVICE" ]; then
    BACKUP_FILE="$INSTALLED_SERVICE.backup.$(date +%Y%m%d-%H%M%S)"
    echo "💾 Backing up existing service to: $BACKUP_FILE"
    cp "$INSTALLED_SERVICE" "$BACKUP_FILE"
fi

# Copy service file
echo "📋 Installing service file..."
cp "$SERVICE_FILE" "$INSTALLED_SERVICE"

# Reload systemd daemon
echo "🔄 Reloading systemd daemon..."
systemctl --user daemon-reload

# Enable service for auto-start
echo "✅ Enabling swictation.service for auto-start..."
systemctl --user enable swictation.service

# Create default configuration
echo ""
echo "⚙️  Creating default configuration..."
mkdir -p ~/.config/swictation
if [ ! -f ~/.config/swictation/config.toml ]; then
    cp "$PROJECT_DIR/config/config.example.toml" ~/.config/swictation/config.toml
    echo "✓ Created ~/.config/swictation/config.toml"
else
    echo "✓ Config already exists: ~/.config/swictation/config.toml"
fi

# Check if we're in a Sway session
if [ -n "$SWAYSOCK" ]; then
    echo ""
    echo "🎯 Sway session detected!"
    echo "   Service will start automatically with Sway"

    # Offer to start now
    read -p "   Start service now? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "▶️  Starting swictation.service..."
        systemctl --user start swictation.service
        sleep 2

        # Show status
        if systemctl --user is-active --quiet swictation.service; then
            echo "✅ Service started successfully!"
            echo ""
            echo "📊 Service status:"
            systemctl --user status swictation.service --no-pager | head -15
        else
            echo "❌ Service failed to start. Check logs with:"
            echo "   journalctl --user -u swictation.service -f"
        fi
    fi
else
    echo ""
    echo "⚠️  Not in Sway session - service will start on next Sway login"
    echo "   To start manually: systemctl --user start swictation.service"
fi

echo ""
echo "=========================================="
echo "Installation Complete!"
echo "=========================================="
echo ""
echo "📝 Useful commands:"
echo "   Start:   systemctl --user start swictation.service"
echo "   Stop:    systemctl --user stop swictation.service"
echo "   Status:  systemctl --user status swictation.service"
echo "   Logs:    journalctl --user -u swictation.service -f"
echo "   Disable: systemctl --user disable swictation.service"
echo ""
echo "⚙️  Configuration:"
echo "   Edit:    ~/.config/swictation/config.toml"
echo "   Example: $PROJECT_DIR/config/config.example.toml"
echo "   After editing, restart: systemctl --user restart swictation.service"
echo ""
echo "🎯 To test the keybinding:"
echo "   Mod1+Shift+d (Alt+Shift+d) to toggle recording"
echo ""
