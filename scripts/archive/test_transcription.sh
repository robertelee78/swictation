#!/usr/bin/env bash
# Test script to capture Parakeet-TDT transcriptions via audio loopback
#
# This plays MP3 files through speakers, records via microphone,
# and captures what the daemon actually transcribes

set -e

CLI="/opt/swictation/src/swictation_cli.py"
LOG_FILE="/tmp/swictation-test1.log"
AUDIO_FILE="${1:-/opt/swictation/examples/en-short.mp3}"
WAV_FILE="${AUDIO_FILE%.mp3}.wav"
EXPECTED_FILE="${AUDIO_FILE%.mp3}.txt"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "========================================"
echo "Swictation Transcription Test"
echo "========================================"
echo "Audio file: $AUDIO_FILE"

if [[ ! -f "$AUDIO_FILE" ]]; then
    echo -e "${RED}✗ Audio file not found: $AUDIO_FILE${NC}"
    exit 1
fi

if [[ ! -f "$EXPECTED_FILE" ]]; then
    echo -e "${YELLOW}⚠ Expected transcription file not found: $EXPECTED_FILE${NC}"
    EXPECTED_FILE=""
fi

echo ""
echo "Step 1: Check daemon status"
python3 "$CLI" status || {
    echo -e "${RED}✗ Daemon not running${NC}"
    exit 1
}

echo ""
echo "Step 2: Clear log and start recording"
# Mark the log with a timestamp
echo "=== TEST START $(date +%s) ===" >> "$LOG_FILE"
START_MARKER=$(date +%s)

python3 "$CLI" toggle
sleep 1

echo ""
echo "Step 3: Convert to WAV and play audio (this will take a few seconds)"
# Convert MP3 to WAV if needed
if [[ ! -f "$WAV_FILE" ]] || [[ "$AUDIO_FILE" -nt "$WAV_FILE" ]]; then
    echo "Converting MP3 to WAV..."
    ffmpeg -i "$AUDIO_FILE" -ar 16000 -ac 1 "$WAV_FILE" -y 2>&1 | tail -1
fi

# Play audio through speakers (acoustic test)
echo "Playing audio through speakers..."
mplayer "$AUDIO_FILE" 2>&1 > /dev/null || echo "mplayer completed"

echo ""
echo "Step 4: Wait for VAD to finish processing"
sleep 3

echo ""
echo "Step 5: Stop recording"
python3 "$CLI" toggle
sleep 2

echo ""
echo "Step 6: Extract transcription from logs"
echo "========================================"

# Find all "Injecting text:" lines after our marker
TRANSCRIPTION=$(awk "/=== TEST START $START_MARKER ===/,0" "$LOG_FILE" | \
    grep "Injecting text:" | \
    sed 's/.*Injecting text: //' || echo "")

if [[ -z "$TRANSCRIPTION" ]]; then
    echo -e "${RED}✗ No transcription found in logs${NC}"
    echo ""
    echo "Last 20 log lines after test start:"
    awk "/=== TEST START $START_MARKER ===/,0" "$LOG_FILE" | tail -20
    exit 1
fi

echo -e "${GREEN}Transcribed text:${NC}"
echo "$TRANSCRIPTION"

if [[ -n "$EXPECTED_FILE" ]]; then
    echo ""
    echo "========================================"
    echo -e "${YELLOW}Expected text:${NC}"
    cat "$EXPECTED_FILE"
    echo ""
    echo "========================================"
    echo -e "${YELLOW}Comparison:${NC}"

    # Simple diff (not perfect but gives us an idea)
    if echo "$TRANSCRIPTION" | diff -u "$EXPECTED_FILE" - > /dev/null 2>&1; then
        echo -e "${GREEN}✓ Perfect match!${NC}"
    else
        echo -e "${YELLOW}Differences detected (this is expected - we're documenting patterns):${NC}"
        echo "$TRANSCRIPTION" | diff -u "$EXPECTED_FILE" - || true
    fi
fi

echo ""
echo "Test complete!"
