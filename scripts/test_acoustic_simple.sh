#!/usr/bin/env bash
#
# Simple physical acoustic test for 1.1B model
# Uses PipeWire/PulseAudio for audio capture
#

set -e

MODEL_DIR="/opt/swictation/models/parakeet-tdt-1.1b"
EXAMPLES_DIR="/opt/swictation/examples"
TEMP_DIR="/tmp/acoustic_test"
mkdir -p "$TEMP_DIR"

# PipeWire source for webcam microphone
WEBCAM_MIC_SOURCE="63"  # alsa_input.usb-Sonix_Technology_Co.__Ltd._USB_Live_camera

echo "================================================================================"
echo "🎯 PHYSICAL ACOUSTIC TEST - 1.1B with PipeWire"
echo "================================================================================"
echo ""
echo "Configuration:"
echo "  Speaker: HDMI output"
echo "  Microphone: Webcam (PipeWire source $WEBCAM_MIC_SOURCE)"
echo "  Model: Parakeet-TDT 1.1B"
echo "  Temp: $TEMP_DIR"
echo ""

# Test 1: Short sample
echo "================================================================================"
echo "TEST 1: en-short.mp3"
echo "================================================================================"
echo ""
echo "Expected: Hello world. Testing, one, two, three"
echo ""

# Play MP3 and capture simultaneously
echo "▶ Starting capture and playback..."
echo "  Capture will run for 8 seconds"
echo "  MP3 will play through HDMI speakers"
echo ""

sleep 1
echo "Starting in 3..."
sleep 1
echo "2..."
sleep 1
echo "1..."
sleep 1

# Start capture in background
timeout 8 parecord \
    --device="$WEBCAM_MIC_SOURCE" \
    --rate=16000 \
    --channels=1 \
    --format=s16le \
    "$TEMP_DIR/captured_en-short.wav" &
CAPTURE_PID=$!

# Wait for capture to stabilize
sleep 1

# Play MP3
mplayer -really-quiet "$EXAMPLES_DIR/en-short.mp3"

# Wait for capture to finish
wait $CAPTURE_PID 2>/dev/null || true

echo ""
echo "✅ Capture complete"
ls -lh "$TEMP_DIR/captured_en-short.wav"

# Transcribe with sherpa-onnx
echo ""
echo "▶ Transcribing with 1.1B model..."
echo ""

python3.12 - <<'PYTHON_SCRIPT'
import sherpa_onnx
import wave
import numpy as np
import sys

MODEL_DIR = "/opt/swictation/models/parakeet-tdt-1.1b"
WAV_FILE = "/tmp/acoustic_test/captured_en-short.wav"

# Initialize recognizer
try:
    recognizer = sherpa_onnx.OfflineRecognizer.from_transducer(
        encoder=f"{MODEL_DIR}/encoder.int8.onnx",
        decoder=f"{MODEL_DIR}/decoder.int8.onnx",
        joiner=f"{MODEL_DIR}/joiner.int8.onnx",
        tokens=f"{MODEL_DIR}/tokens.txt",
        num_threads=4,
        sample_rate=16000,
        feature_dim=80,
        decoding_method="greedy_search",
        max_active_paths=4,
        provider="cuda",  # Try CUDA first
        model_type="nemo_transducer",
    )
    print("✅ Recognizer initialized (CUDA)")
except Exception as e:
    # Fallback to CPU
    recognizer = sherpa_onnx.OfflineRecognizer.from_transducer(
        encoder=f"{MODEL_DIR}/encoder.int8.onnx",
        decoder=f"{MODEL_DIR}/decoder.int8.onnx",
        joiner=f"{MODEL_DIR}/joiner.int8.onnx",
        tokens=f"{MODEL_DIR}/tokens.txt",
        num_threads=4,
        sample_rate=16000,
        feature_dim=80,
        decoding_method="greedy_search",
        max_active_paths=4,
        provider="cpu",
        model_type="nemo_transducer",
    )
    print("✅ Recognizer initialized (CPU fallback)")

# Load audio
with wave.open(WAV_FILE, 'rb') as wf:
    sample_rate = wf.getframerate()
    frames = wf.readframes(wf.getnframes())
    samples = np.frombuffer(frames, dtype=np.int16).astype(np.float32) / 32768.0

print(f"✅ Loaded {len(samples):,} samples at {sample_rate} Hz")

# Transcribe
stream = recognizer.create_stream()
stream.accept_waveform(sample_rate, samples)
recognizer.decode_stream(stream)
result = stream.result.text.strip()

# Results
print("")
print("="*80)
print("📊 TRANSCRIPTION RESULTS:")
print("="*80)
print("")
print(f"Expected: \"Hello world. Testing, one, two, three\"")
print(f"Got:      \"{result}\"")
print("")
print(f"Words: {len(result.split()) if result else 0}")
print(f"Chars: {len(result)}")

if result and len(result) > 10:
    print("")
    print("✅ SUCCESS - Got meaningful transcription!")
    sys.exit(0)
else:
    print("")
    print("❌ FAILURE - Empty or nonsense transcription")
    sys.exit(1)
PYTHON_SCRIPT

echo ""
echo "================================================================================"
echo "Test complete"
echo "================================================================================"
