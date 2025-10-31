#!/bin/bash
# End-to-end test: Record while playing audio

set -e

echo "=== Swictation End-to-End Test ==="
echo

# Start daemon if not running
if ! pgrep -f swictationd.py > /dev/null; then
    echo "Starting daemon..."
    python3 src/swictationd.py > /tmp/e2e_test.log 2>&1 &
    DAEMON_PID=$!
    echo "Waiting for daemon to load (25 seconds)..."
    sleep 25
else
    echo "Daemon already running"
fi

# Check daemon status
echo "Checking daemon status..."
python3 src/swictation_cli.py status

echo
echo "Starting recording..."
python3 src/swictation_cli.py toggle

echo "Playing audio file through speakers..."
mpg123 /home/robert/Documents/python/translate-stream/examples/en-short.mp3 2>/dev/null &
PLAY_PID=$!

# Wait for playback to finish
wait $PLAY_PID
sleep 1

echo "Stopping recording..."
python3 src/swictation_cli.py toggle

echo "Waiting for transcription (10 seconds)..."
sleep 10

echo
echo "=== RESULTS ==="
echo "Daemon log:"
tail -50 /tmp/e2e_test.log | grep -E "(→|Text:|Transcribed|Injecting|injected|Empty)" || echo "No transcription output found"

echo
echo "Daemon status:"
python3 src/swictation_cli.py status

echo
echo "=== Test complete ==="
echo "Expected text: 'Hello world. Testing, one, two, three'"
