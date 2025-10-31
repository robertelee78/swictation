#!/bin/bash
# Automated FP16 Test with Live Daemon
# Routes test audio through PulseAudio to the daemon's input

set -e

echo "=============================================================================="
echo "FP16 AUTOMATED TRANSCRIPTION TEST"
echo "=============================================================================="

# Check daemon
if ! systemctl --user is-active --quiet swictation.service; then
    echo "❌ Daemon not running. Start it: systemctl --user start swictation.service"
    exit 1
fi

echo "✓ Daemon running"

# Verify FP16
if journalctl --user -u swictation.service -n 200 | grep -q "float16\|FP16"; then
    echo "✓ FP16 confirmed"
else
    echo "⚠️  Cannot confirm FP16"
fi

# Get current state
STATE=$(./src/swictation_cli.py status 2>&1 | grep -i "state:" | awk '{print $NF}')
echo "  Current state: $STATE"

if [ "$STATE" != "idle" ]; then
    echo "❌ Daemon not idle (state: $STATE)"
    exit 1
fi

echo ""
echo "=============================================================================="
echo "Test 1: Short Audio (en-short.mp3)"
echo "  Expected: 'Hello world. Testing, one, two, three.'"
echo "=============================================================================="

# Get default input device
DEFAULT_SOURCE=$(pactl get-default-source)
echo "  Default input: $DEFAULT_SOURCE"

# Start recording
echo "  [1/5] Starting recording..."
./src/swictation_cli.py toggle >/dev/null 2>&1
sleep 0.5

# Play audio to the default source using pacat
echo "  [2/5] Playing test audio (6s)..."
pacat --playback --file-format=mp3 tests/data/en-short.mp3 &
PLAY_PID=$!

# Wait for audio to finish
sleep 7

# Stop recording
echo "  [3/5] Stopping recording..."
./src/swictation_cli.py toggle >/dev/null 2>&1

# Wait for processing
echo "  [4/5] Waiting for transcription..."
sleep 3

# Get transcription from recent logs
echo "  [5/5] Checking transcription..."
TRANSCRIPTION=$(journalctl --user -u swictation.service --since "20 seconds ago" | grep "  Text:" | tail -1 | sed 's/.*Text: //')

if [ -n "$TRANSCRIPTION" ]; then
    echo ""
    echo "  Transcription: '$TRANSCRIPTION'"
    echo ""

    # Check for expected keywords
    KEYWORDS=("hello" "world" "testing" "one" "two" "three")
    MATCHED=0
    TOTAL=${#KEYWORDS[@]}

    for keyword in "${KEYWORDS[@]}"; do
        if echo "$TRANSCRIPTION" | grep -iq "$keyword"; then
            ((MATCHED++))
            echo "    ✓ Found: $keyword"
        else
            echo "    ✗ Missing: $keyword"
        fi
    done

    ACCURACY=$((MATCHED * 100 / TOTAL))
    echo ""
    echo "  Accuracy: $ACCURACY% ($MATCHED/$TOTAL keywords)"

    if [ $ACCURACY -ge 95 ]; then
        echo "  ✓ PASS (≥95% target)"
    elif [ $ACCURACY -ge 80 ]; then
        echo "  ⚠️  MARGINAL (80-94%)"
    else
        echo "  ✗ FAIL (<80%)"
    fi
else
    echo "  ✗ NO TRANSCRIPTION FOUND"
    echo ""
    echo "  Debug: Recent daemon logs:"
    journalctl --user -u swictation.service --since "20 seconds ago" | grep -E "(Recording|Captured|Streaming)" | tail -10
fi

echo ""
echo "=============================================================================="
echo "Test 2: Silent Audio (silent-10s.mp3)"
echo "  Expected: Empty or ≤2 words (no hallucinations)"
echo "=============================================================================="

# Start recording
echo "  [1/5] Starting recording..."
./src/swictation_cli.py toggle >/dev/null 2>&1
sleep 0.5

# Play silent audio
echo "  [2/5] Playing silent audio (10s)..."
pacat --playback --file-format=mp3 tests/data/silent-10s.mp3 &
PLAY_PID=$!

sleep 11

# Stop recording
echo "  [3/5] Stopping recording..."
./src/swictation_cli.py toggle >/dev/null 2>&1

# Wait for processing
echo "  [4/5] Waiting for transcription..."
sleep 3

# Get transcription
echo "  [5/5] Checking for hallucinations..."
TRANSCRIPTION=$(journalctl --user -u swictation.service --since "20 seconds ago" | grep "  Text:" | tail -1 | sed 's/.*Text: //')

if [ -z "$TRANSCRIPTION" ]; then
    echo ""
    echo "  Transcription: (empty)"
    echo "  ✓ PASS (no hallucinations)"
else
    echo ""
    echo "  Transcription: '$TRANSCRIPTION'"

    WORD_COUNT=$(echo "$TRANSCRIPTION" | wc -w)
    echo "  Word count: $WORD_COUNT"

    if [ $WORD_COUNT -le 2 ]; then
        echo "  ✓ PASS (≤2 words allowed)"
    else
        echo "  ✗ FAIL (hallucinated $WORD_COUNT words)"
    fi
fi

echo ""
echo "=============================================================================="
echo "FP16 VALIDATION COMPLETE"
echo "=============================================================================="
echo ""
echo "VRAM Usage:"
nvidia-smi --query-gpu=memory.used,memory.total --format=csv,noheader,nounits | \
    awk '{printf "  %d MB / %d MB (%.1f%%)\n", $1, $2, ($1/$2)*100}'

echo ""
echo "For more detailed testing, use:"
echo "  ./tests/test_fp16_manual.sh"
echo ""
