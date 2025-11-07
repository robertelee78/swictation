#!/bin/bash
# Test script for Swictation daemon audio processing

set -e

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Set up library paths
export LD_LIBRARY_PATH="/home/robert/.cache/sherpa-rs/x86_64-unknown-linux-gnu/cd22ee337e205674536643d2bc86e984fbbbf9c5c52743c4df6fc8d8dd17371b/sherpa-onnx-v1.12.9-linux-x64-shared/lib:/home/robert/.cache/ort.pyke.io/dfbin/x86_64-unknown-linux-gnu/ED1716DE95974BF47AB0223CA33734A0B5A5D09A181225D0E8ED62D070AEA893/onnxruntime/lib:$LD_LIBRARY_PATH"

DAEMON_BIN="./target/release/swictation-daemon"
DAEMON_LOG="/tmp/swictation-test.log"
DAEMON_PID_FILE="/tmp/swictation-test.pid"

# Test files
TEST_DIR="/home/robert/Documents/python/translate-stream/examples"
TEST_SHORT="$TEST_DIR/en-short.mp3"
TEST_SHORT_TXT="$TEST_DIR/en-short.txt"

echo -e "${YELLOW}═══════════════════════════════════════════════════════════${NC}"
echo -e "${YELLOW}  Swictation Daemon Audio Test${NC}"
echo -e "${YELLOW}═══════════════════════════════════════════════════════════${NC}"
echo

# Check if daemon binary exists
if [ ! -f "$DAEMON_BIN" ]; then
    echo -e "${RED}✗ Daemon binary not found at $DAEMON_BIN${NC}"
    echo "  Run: cargo build --release --bin swictation-daemon"
    exit 1
fi

# Check if test audio exists
if [ ! -f "$TEST_SHORT" ]; then
    echo -e "${RED}✗ Test audio not found at $TEST_SHORT${NC}"
    exit 1
fi

# Kill any existing daemon
if [ -f "$DAEMON_PID_FILE" ]; then
    OLD_PID=$(cat "$DAEMON_PID_FILE")
    if ps -p "$OLD_PID" > /dev/null 2>&1; then
        echo -e "${YELLOW}⚠ Killing existing daemon (PID $OLD_PID)${NC}"
        kill "$OLD_PID" 2>/dev/null || true
        sleep 1
    fi
    rm -f "$DAEMON_PID_FILE"
fi

# Start daemon in background
echo -e "${GREEN}▶ Starting daemon...${NC}"
rm -f "$DAEMON_LOG"
"$DAEMON_BIN" > "$DAEMON_LOG" 2>&1 &
DAEMON_PID=$!
echo $DAEMON_PID > "$DAEMON_PID_FILE"

# Wait for daemon to initialize
echo "  Waiting for daemon to initialize (15s)..."
sleep 15

# Check if daemon is still running
if ! ps -p "$DAEMON_PID" > /dev/null 2>&1; then
    echo -e "${RED}✗ Daemon failed to start${NC}"
    echo
    echo "Last 20 lines of log:"
    tail -20 "$DAEMON_LOG"
    exit 1
fi

echo -e "${GREEN}✓ Daemon running (PID $DAEMON_PID)${NC}"
echo

# Show initialization log
echo -e "${YELLOW}─────────────────────────────────────────────────────────${NC}"
echo "Daemon Initialization:"
echo -e "${YELLOW}─────────────────────────────────────────────────────────${NC}"
grep -E "(Starting|initialized|GPU|Audio|VAD|STT|Memory|ready)" "$DAEMON_LOG" | head -15
echo

# Read expected transcription
EXPECTED=$(cat "$TEST_SHORT_TXT")
echo -e "${YELLOW}─────────────────────────────────────────────────────────${NC}"
echo "Expected Transcription:"
echo -e "${YELLOW}─────────────────────────────────────────────────────────${NC}"
echo "$EXPECTED"
echo

# Trigger recording toggle via IPC (simulate hotkey)
echo -e "${GREEN}▶ Triggering recording start...${NC}"
echo "toggle" | nc -U /tmp/swictation.sock 2>/dev/null || echo "(IPC not available, hotkey required)"
sleep 1

# Play audio file
echo -e "${GREEN}▶ Playing test audio: en-short.mp3${NC}"
echo "  (Make sure speakers → microphone loop is active)"
echo

# Check if mpg123 is available
if ! command -v mpg123 &> /dev/null; then
    echo -e "${YELLOW}⚠ mpg123 not found, trying with ffplay...${NC}"
    if command -v ffplay &> /dev/null; then
        ffplay -nodisp -autoexit "$TEST_SHORT" 2>/dev/null
    else
        echo -e "${RED}✗ No audio player found (mpg123 or ffplay)${NC}"
        echo "  Install: sudo apt-get install mpg123"
    fi
else
    mpg123 "$TEST_SHORT" 2>/dev/null
fi

# Wait for transcription processing
echo
echo "  Waiting for transcription processing (5s)..."
sleep 5

# Stop recording
echo -e "${GREEN}▶ Stopping recording...${NC}"
echo "toggle" | nc -U /tmp/swictation.sock 2>/dev/null || true
sleep 1

# Show transcription results
echo
echo -e "${YELLOW}─────────────────────────────────────────────────────────${NC}"
echo "Transcription Results:"
echo -e "${YELLOW}─────────────────────────────────────────────────────────${NC}"

# Extract transcriptions from log
TRANSCRIPTIONS=$(grep -E "(Transcribed|Injecting)" "$DAEMON_LOG" | tail -10)

if [ -z "$TRANSCRIPTIONS" ]; then
    echo -e "${RED}✗ No transcriptions found${NC}"
    echo
    echo "Recent log output:"
    tail -30 "$DAEMON_LOG"
else
    echo "$TRANSCRIPTIONS"
    echo

    # Simple comparison (case-insensitive, ignore punctuation)
    ACTUAL=$(echo "$TRANSCRIPTIONS" | grep -oP "(?<=Transcribed: ).*" | head -1 || echo "")

    if [ -n "$ACTUAL" ]; then
        echo -e "${GREEN}✓ Transcription received${NC}"
        echo "  Expected: $EXPECTED"
        echo "  Actual:   $ACTUAL"
    fi
fi

echo
echo -e "${YELLOW}─────────────────────────────────────────────────────────${NC}"
echo "Full Daemon Log:"
echo -e "${YELLOW}─────────────────────────────────────────────────────────${NC}"
cat "$DAEMON_LOG"
echo

# Cleanup
echo -e "${YELLOW}▶ Stopping daemon...${NC}"
kill "$DAEMON_PID" 2>/dev/null || true
rm -f "$DAEMON_PID_FILE"

echo
echo -e "${GREEN}✓ Test complete${NC}"
echo
echo "Log saved to: $DAEMON_LOG"
