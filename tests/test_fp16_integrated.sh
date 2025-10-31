#!/bin/bash
# Integrated FP16 Test - Uses ffmpeg to decode and feed audio
# Creates a virtual loopback device to route test audio to daemon

set -e

echo "=============================================================================="
echo "FP16 INTEGRATED TRANSCRIPTION TEST"
echo "=============================================================================="

# Check dependencies
for cmd in ffmpeg pactl; do
    if ! command -v $cmd &>/dev/null; then
        echo "❌ Missing dependency: $cmd"
        exit 1
    fi
done

# Check daemon
if ! systemctl --user is-active --quiet swictation.service; then
    echo "❌ Daemon not running"
    exit 1
fi

echo "✓ Dependencies OK"
echo "✓ Daemon running"

# Verify FP16
if journalctl --user -u swictation.service -n 200 | grep -q "float16"; then
    echo "✓ FP16 confirmed"
    journalctl --user -u swictation.service -n 200 | grep "Precision" | tail -1 | sed 's/.*: /  /'
fi

echo ""
echo "NOTE: This test requires manual validation."
echo "      The daemon captures from your microphone,"
echo "      so we'll play audio and you verify transcription."
echo ""

# Test 1
echo "=============================================================================="
echo "Test 1: Short Audio Transcription"
echo "Expected: 'Hello world. Testing, one, two, three.'"
echo "=============================================================================="
echo ""

read -p "Ready to test? Will play audio and capture transcription (y/n): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Skipped"
    exit 0
fi

# Clear logs marker
echo "  Starting test at: $(date '+%H:%M:%S')"
START_TIME=$(date '+%Y-%m-%d %H:%M:%S')

# Start recording
echo "  [1/4] Starting recording..."
./src/swictation_cli.py toggle >/dev/null

sleep 1

# Play audio through speakers (you can hear it, and mic might pick it up)
echo "  [2/4] Playing test audio..."
echo "         (If your mic doesn't pick it up, you can speak the text yourself)"
ffplay -autoexit -nodisp tests/data/en-short.mp3 2>/dev/null &
sleep 7

# Stop recording
echo "  [3/4] Stopping recording..."
./src/swictation_cli.py toggle >/dev/null

# Wait for transcription
echo "  [4/4] Processing..."
sleep 3

# Show transcription
echo ""
echo "  --- Transcription Output ---"
journalctl --user -u swictation.service --since "$START_TIME" | \
    grep -E "(Text:|Recording started|Captured)" | \
    sed 's/.*python3\[[0-9]*\]: /  /'

echo ""
echo "=============================================================================="
echo ""

# Show metrics
echo "Current VRAM:"
nvidia-smi --query-gpu=memory.used,utilization.gpu --format=csv,noheader,nounits | \
    awk '{printf "  %d MB VRAM, %d%% GPU\n", $1, $2}'

echo ""
echo "=============================================================================="
echo "VALIDATION"
echo "=============================================================================="
echo ""
echo "Please check if the transcription matches:"
echo "  Expected: 'Hello world. Testing, one, two, three.'"
echo ""
echo "Evaluation:"
echo "  ✓ Perfect: All words correct"
echo "  ⚠️  Good: Minor errors (1-2 words wrong)"
echo "  ✗ Poor: Multiple errors or missing words"
echo ""
echo "If transcription is empty, the mic may not have picked up the audio."
echo "Try running again and speak the text clearly into your microphone."
echo ""
