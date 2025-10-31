#!/bin/bash
# Live FP16 Transcription Test
# This script tests the FP16 model with real microphone input

echo "=============================================================================="
echo "FP16 LIVE TRANSCRIPTION TEST"
echo "=============================================================================="

# Check daemon status
if ! systemctl --user is-active --quiet swictation.service; then
    echo "❌ Daemon is not running"
    echo "   Start it: systemctl --user start swictation.service"
    exit 1
fi

# Verify FP16 is active
echo ""
echo "Checking FP16 status..."
if journalctl --user -u swictation.service -n 200 | grep -q "float16\|FP16"; then
    echo "✓ FP16 precision confirmed"
    journalctl --user -u swictation.service -n 200 | grep "Model Precision" | tail -1
else
    echo "⚠️  Cannot confirm FP16 from logs"
fi

# Show VRAM usage
echo ""
echo "Current VRAM usage:"
nvidia-smi --query-gpu=memory.used,memory.total,utilization.gpu --format=csv,noheader,nounits | \
    awk '{printf "  Used: %.0f MB / %.0f MB (%.0f%% GPU utilization)\n", $1, $2, $3}'

echo ""
echo "=============================================================================="
echo "INSTRUCTIONS FOR MANUAL TESTING:"
echo "=============================================================================="
echo ""
echo "1. Open a new terminal and run:"
echo "   journalctl --user -u swictation.service -f | grep -E '(Text:|Recording|Transcrib)'"
echo ""
echo "2. Toggle dictation on (usually Caps Lock or configured hotkey)"
echo ""
echo "3. Speak clearly: 'This is a test of the FP16 model accuracy'"
echo ""
echo "4. Toggle dictation off"
echo ""
echo "5. Check the journalctl output for:"
echo "   - 'Recording started' message"
echo "   - 'Text: <your transcription>' output"
echo ""
echo "6. Evaluate accuracy:"
echo "   ✓ Perfect: All words correct with proper punctuation"
echo "   ⚠️  Good: Minor errors (<5% word error rate)"
echo "   ❌ Poor: Multiple errors or gibberish"
echo ""
echo "=============================================================================="
echo "AUTOMATIC TEST (speaking through test audio):"
echo "=============================================================================="

# Try to find test audio files
if [ -f "tests/data/en-short.mp3" ]; then
    echo ""
    echo "Found test audio files. To test automatically:"
    echo ""
    echo "Option 1: Use ffmpeg to pipe audio to mic:"
    echo "  ffmpeg -re -i tests/data/en-short.mp3 -f pulse default"
    echo ""
    echo "Option 2: Play the file and capture it:"
    echo "  pactl load-module module-loopback"
    echo "  paplay tests/data/en-short.mp3"
    echo "  pactl unload-module module-loopback"
fi

echo ""
echo "=============================================================================="
echo "Expected Results with FP16:"
echo "=============================================================================="
echo "  • VRAM usage: ~1800 MB model + ~400 MB buffer = ~2200 MB total"
echo "  • GPU utilization: 85-90%"
echo "  • Transcription accuracy: <0.5% WER (Word Error Rate) degradation"
echo "  • Response time: Similar to FP32 (typically <1s for short audio)"
echo "=============================================================================="
