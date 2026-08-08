#!/usr/bin/env bash
# Test long sample
set -e

MODEL_DIR="/opt/swictation/models/parakeet-tdt-1.1b"
EXAMPLES_DIR="/opt/swictation/examples"
TEMP_DIR="/tmp/acoustic_test"
mkdir -p "$TEMP_DIR"

WEBCAM_MIC_SOURCE="63"

echo "================================================================================"
echo "TEST 2: en-long.mp3 (Long Audio Sample)"
echo "================================================================================"
echo ""

# Play and capture (30 seconds)
echo "▶ Playing en-long.mp3 and capturing for 30 seconds..."
sleep 2

timeout 30 parecord \
    --device="$WEBCAM_MIC_SOURCE" \
    --rate=16000 \
    --channels=1 \
    --format=s16le \
    "$TEMP_DIR/captured_en-long.wav" &
CAPTURE_PID=$!

sleep 1
mplayer -really-quiet "$EXAMPLES_DIR/en-long.mp3" &

wait $CAPTURE_PID 2>/dev/null || true

echo ""
echo "✅ Capture complete"
ls -lh "$TEMP_DIR/captured_en-long.wav"

# Transcribe
echo ""
echo "▶ Transcribing with 1.1B model..."
python3.12 /opt/swictation/scripts/validate_1_1b_export.py "$TEMP_DIR/captured_en-long.wav" 2>&1 | tail -40

echo ""
echo "Expected (first sentence):"
echo "  \"The open-source AI community has scored a significant win...\""
