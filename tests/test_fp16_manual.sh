#!/bin/bash
# Manual FP16 Transcription Test
# This script helps you manually test FP16 accuracy with the daemon

echo "=============================================================================="
echo "FP16 MANUAL TRANSCRIPTION TEST"
echo "=============================================================================="
echo ""

# Check daemon
if ! systemctl --user is-active --quiet swictation.service; then
    echo "❌ Daemon is not running"
    echo "   Start it: systemctl --user start swictation.service"
    exit 1
fi

echo "✓ Daemon is running"

# Check FP16
if journalctl --user -u swictation.service -n 200 | grep -q "float16\|FP16"; then
    echo "✓ FP16 precision confirmed"
    journalctl --user -u swictation.service -n 200 | grep "Model Precision" | tail -1 | sed 's/.*python3\[[0-9]*\]: /  /'
else
    echo "⚠️  Cannot confirm FP16 from logs"
fi

# Show VRAM
echo ""
echo "Current VRAM usage:"
nvidia-smi --query-gpu=memory.used,memory.total --format=csv,noheader,nounits | \
    awk '{printf "  %d MB / %d MB (%.1f%%)\n", $1, $2, ($1/$2)*100}'

echo ""
echo "=============================================================================="
echo "TEST PROCEDURE"
echo "=============================================================================="
echo ""
echo "We will test transcription accuracy using your microphone."
echo ""
echo "Test 1: Baseline Accuracy"
echo "  Expected text: 'Hello world. Testing, one, two, three.'"
echo "  Target: 95%+ accuracy (all words correct)"
echo ""
echo "Test 2: Natural Speech"
echo "  Speak naturally about any topic for 10-15 seconds"
echo "  Target: High accuracy with proper punctuation"
echo ""
echo "=============================================================================="
echo ""

# Monitor logs in background
echo "Starting log monitor (Ctrl+C to stop)..."
echo "You'll see transcriptions appear below as you speak."
echo ""
echo "Ready? Press Ctrl+C when done testing."
echo ""
echo "--- Transcription Output ---"
journalctl --user -u swictation.service -f -n 0 | grep --line-buffered -E "(Recording|Text:|Transcrib)"

