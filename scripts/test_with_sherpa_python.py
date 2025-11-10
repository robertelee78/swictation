#!/usr/bin/env python3
"""
Test our exported 1.1B model with sherpa-onnx Python bindings.

If sherpa-onnx gets correct transcriptions with our exported model,
then the problem is in our Rust implementation.

If sherpa-onnx ALSO gets wrong transcriptions, then the model export is broken.
"""

import sherpa_onnx
from pathlib import Path

def test_model():
    """Test with sherpa-onnx Python bindings."""

    model_dir = "/opt/swictation/models/parakeet-tdt-1.1b-exported"

    print("🔍 Testing 1.1B Model with Sherpa-ONNX Python Bindings\n")
    print("="*60)

    # Create recognizer
    print("\n📦 Loading model...")
    recognizer = sherpa_onnx.OfflineRecognizer.from_transducer(
        tokens=f"{model_dir}/tokens.txt",
        encoder=f"{model_dir}/encoder.int8.onnx",
        decoder=f"{model_dir}/decoder.onnx",
        joiner=f"{model_dir}/joiner.int8.onnx",
        num_threads=1,
        sample_rate=16000,
        feature_dim=128,
        debug=True,
    )
    print("✓ Model loaded\n")

    # Test files
    test_files = [
        ("/tmp/en-short.wav", "Hello world"),
        ("/tmp/en-long.wav", "open source AI community"),
    ]

    all_passed = True
    for wav_path, expected in test_files:
        if not Path(wav_path).exists():
            print(f"⚠ {wav_path} not found, skipping...")
            continue

        print(f"📝 Testing: {wav_path}")
        print(f"   Expected: {expected}")

        # Create stream and decode
        stream = recognizer.create_stream()
        stream.accept_waveform(16000, sherpa_onnx.read_wave(wav_path)[1])

        recognizer.decode_stream(stream)
        result = stream.result.text

        print(f"   Got:      {result}")

        # Check
        if expected.lower() in result.lower():
            print(f"   ✅ PASS\n")
        else:
            print(f"   ❌ FAIL\n")
            all_passed = False

    print("="*60)
    if all_passed:
        print("✅ Model works correctly with sherpa-onnx!")
        print("   → Problem is in our Rust implementation")
    else:
        print("❌ Model ALSO fails with sherpa-onnx!")
        print("   → Model export is broken")
    print("="*60)

if __name__ == "__main__":
    test_model()
