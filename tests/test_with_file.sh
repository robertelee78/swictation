#!/bin/bash
# Test the daemon by playing audio and recording simultaneously

echo "Starting daemon..."
python3 src/swictationd.py > /tmp/daemon_test.log 2>&1 &
DAEMON_PID=$!

sleep 20
echo "Daemon started (PID: $DAEMON_PID)"

echo "Starting recording..."
python3 src/swictation_cli.py toggle

echo "Playing test audio..."
aplay /tmp/test_recording.wav &
PLAY_PID=$!

sleep 4

echo "Stopping recording..."
python3 src/swictation_cli.py toggle

# Wait for transcription
sleep 5

echo "=== Daemon logs ==="
tail -50 /tmp/daemon_test.log | grep -E "(Text:|Transcribed|Injecting|injected)"

echo ""
echo "=== Daemon status ==="
python3 src/swictation_cli.py status

# Cleanup
kill $DAEMON_PID 2>/dev/null
