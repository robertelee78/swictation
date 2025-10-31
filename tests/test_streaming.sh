#!/bin/bash
# Test streaming transcription with real audio playback

set -e

echo "=== Swictation Streaming Test ==="
echo

# Check daemon status
echo "Checking daemon status..."
python3 src/swictation_cli.py status

echo
echo "Starting recording (streaming mode)..."
python3 src/swictation_cli.py toggle

echo "Playing audio file through speakers..."
mpg123 /home/robert/Documents/python/translate-stream/examples/en-short.mp3 2>/dev/null &
PLAY_PID=$!

# Wait for playback to finish
wait $PLAY_PID
sleep 1

echo "Stopping recording..."
python3 src/swictation_cli.py toggle

echo "Waiting for final transcription (3 seconds)..."
sleep 3

echo
echo "=== RESULTS ==="
echo "Daemon log (streaming output):"
tail -100 /tmp/streaming_test.log | grep -E "(🎤→|Text:|Transcribed|Injecting|streaming)" || echo "No streaming output found"

echo
echo "Daemon status:"
python3 src/swictation_cli.py status

echo
echo "=== Test complete ==="
echo "Expected: Real-time text injection while recording"
echo "Look for '🎤→' markers showing streaming output"
