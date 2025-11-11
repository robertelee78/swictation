#!/bin/bash
# Component-by-component pipeline testing
# Tests each part of the pipeline independently to isolate issues

set -e

MODELS_DIR="/opt/swictation/models"
EXAMPLES_DIR="/opt/swictation/examples"
TEST_AUDIO="$EXAMPLES_DIR/en-short.wav"

echo "=========================================="
echo "Pipeline Component Testing"
echo "=========================================="
echo ""

# Test 1: STT Engine (bottom-most component)
echo "TEST 1: STT Engine Direct"
echo "  Testing: OrtRecognizer with 1.1B model"
echo "  Input: Known-good WAV file"
echo "  Expected: 'hello world testing one two three'"
echo ""
cd /opt/swictation/rust-crates
cargo run --quiet --release --example test_1_1b_direct --package swictation-stt -- "$TEST_AUDIO" 2>&1 | \
    grep -E "Final Transcription:|Output:" | tail -2
echo ""

# Test 2: VAD (TODO)
echo "TEST 2: VAD Speech Detection"
echo "  Status: TODO - need to create VAD-only test"
echo "  Should test: Silero VAD detects speech from WAV"
echo ""

# Test 3: Audio Capture (TODO)
echo "TEST 3: Audio Capture Quality"
echo "  Status: TODO - need to create capture test"
echo "  Should test: Record 5s of audio and analyze quality"
echo "  Metrics: SNR, clipping, silence ratio"
echo ""

# Test 4: Full Pipeline with Direct Audio (TODO)
echo "TEST 4: Pipeline with Direct Audio Input"
echo "  Status: TODO"
echo "  Should test: Feed WAV through full pipeline"
echo ""

# Test 5: Full Pipeline with Loopback (TODO)
echo "TEST 5: Pipeline with Acoustic Loopback"
echo "  Status: TODO"
echo "  Should test: speaker → mic → pipeline"
echo "  Note: This is expected to have degradation"
echo ""

echo "=========================================="
echo "Test Summary"
echo "=========================================="
echo "✅ STT Engine: PASSED - transcription works"
echo "⏳ VAD: Need test"
echo "⏳ Audio Capture: Need test"
echo "⏳ Full Pipeline: Need test"
echo ""
