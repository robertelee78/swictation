#!/bin/bash
# Test Swictation keybinding setup
# Simulates the keybinding action without requiring Sway

set -e

echo "======================================================================"
echo "Swictation Keybinding Test"
echo "======================================================================"
echo ""
echo "This script simulates the Mod1+Shift+d keybinding action."
echo "It will test the CLI → daemon communication pipeline."
echo ""

# Check if daemon is running
echo "1️⃣ Checking daemon status..."
if python3 /opt/swictation/src/swictation_cli.py status 2>/dev/null; then
    echo "✓ Daemon is running"
else
    echo "✗ Daemon not running"
    echo ""
    echo "Start the daemon first:"
    echo "  python3 /opt/swictation/src/swictationd.py"
    exit 1
fi

echo ""
echo "2️⃣ Testing toggle command (simulating Alt+Shift+d)..."
echo "   This will start recording..."
sleep 2

python3 /opt/swictation/src/swictation_cli.py toggle

echo ""
echo "✓ Recording started!"
echo ""
echo "Speak now... (5 seconds)"
sleep 5

echo ""
echo "3️⃣ Stopping recording (simulating Alt+Shift+d again)..."
python3 /opt/swictation/src/swictation_cli.py toggle

echo ""
echo "✓ Recording stopped. Processing audio..."
sleep 2

echo ""
echo "======================================================================"
echo "Test Complete!"
echo "======================================================================"
echo ""
echo "If you saw transcribed text appear, the keybinding will work!"
echo "If not, check:"
echo "  - Microphone permissions"
echo "  - Audio input device"
echo "  - Daemon logs"
echo ""
