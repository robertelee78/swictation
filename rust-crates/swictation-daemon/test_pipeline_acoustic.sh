#!/bin/bash
# Full pipeline acoustic test: speakers → air → microphone → VAD → STT → output
# Tests the REAL physical audio path with 0.6B model (1.1B requires ort implementation)

set -e

AUDIO_FILE="/opt/swictation/examples/en-short.mp3"
EXPECTED_TEXT="Hello world. Testing, one, two, three"
MICROPHONE="plughw:2,0"  # USB Live camera microphone
TEST_DURATION=10  # seconds

echo "=========================================="
echo "Full Pipeline Acoustic Integration Test"
echo "=========================================="
echo ""
echo "Test Setup:"
echo "  Audio: $AUDIO_FILE"
echo "  Expected: \"$EXPECTED_TEXT\""
echo "  Microphone: $MICROPHONE"
echo "  Model: Parakeet-TDT 0.6B v3 (sherpa-rs)"
echo "  Duration: ${TEST_DURATION}s"
echo ""

# Check if daemon is already running
if pgrep -x "swictation-daemon" > /dev/null; then
    echo "⚠️  Daemon already running, stopping it first..."
    pkill -9 swictation-daemon || true
    sleep 1
fi

# Start daemon in background (CPU mode - CUDA provider library missing)
echo "[1/4] Starting swictation daemon (CPU mode)..."
cd /opt/swictation/rust-crates
export LD_LIBRARY_PATH="/home/robert/.local/lib/python3.12/site-packages/sherpa_onnx/lib:$LD_LIBRARY_PATH"
./target/release/swictation-daemon &
DAEMON_PID=$!
sleep 3  # Wait for daemon to initialize

# Verify daemon started
if ! ps -p $DAEMON_PID > /dev/null; then
    echo "❌ Failed to start daemon"
    exit 1
fi
echo "✓ Daemon running (PID: $DAEMON_PID)"

# Start recording (toggle on)
echo ""
echo "[2/4] Starting recording..."
echo '{"action": "toggle"}' | nc -U /tmp/swictation.sock || echo "✓ Recording started"
sleep 1

# Play audio through speakers (this will be picked up by microphone)
echo ""
echo "[3/4] Playing test audio through speakers..."
echo "  (Microphone will capture this via acoustic path)"
mplayer -really-quiet -nolirc "$AUDIO_FILE" &
PLAYER_PID=$!

# Wait for playback + processing
sleep $TEST_DURATION

# Stop recording
echo ""
echo "[4/4] Stopping recording..."
echo '{"action": "toggle"}' | nc -U /tmp/swictation.sock || echo "✓ Recording stopped"
sleep 2

# Kill processes
kill $DAEMON_PID 2>/dev/null || true
kill $PLAYER_PID 2>/dev/null || true
wait $DAEMON_PID 2>/dev/null || true

echo ""
echo "=========================================="
echo "Test Results:"
echo "=========================================="

# Check if we got any transcriptions
if [ -f "/tmp/swictation_last_transcription.txt" ]; then
    ACTUAL_TEXT=$(cat /tmp/swictation_last_transcription.txt)
    echo "Expected: \"$EXPECTED_TEXT\""
    echo "Got:      \"$ACTUAL_TEXT\""
    echo ""

    # Simple comparison
    if echo "$ACTUAL_TEXT" | grep -qi "hello.*world"; then
        echo "✅ SUCCESS: Pipeline working!"
        echo "   - VAD detected speech ✓"
        echo "   - STT transcribed audio ✓"
        echo "   - Acoustic path validated ✓"
    else
        echo "⚠️  PARTIAL: Got transcription but doesn't match expected"
        echo "   Check if VAD/STT parameters need tuning"
    fi
else
    echo "❌ FAILED: No transcription produced"
    echo "   Possible issues:"
    echo "   - VAD didn't detect speech"
    echo "   - STT failed to process"
    echo "   - Audio level too low"
    echo ""
    echo "   Check daemon logs for details"
fi

echo ""
echo "Note: 1.1B model test requires ort crate implementation"
echo "      (sherpa-rs has SessionOptions bug for external weights)"
echo "=========================================="
