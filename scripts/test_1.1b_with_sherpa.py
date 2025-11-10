#!/usr/bin/env python3
"""
Test our exported 1.1B Parakeet-TDT model with sherpa-onnx Python API.

This will definitively tell us if:
- sherpa-onnx gets correct transcriptions → problem is in our Rust impl
- sherpa-onnx ALSO gets wrong transcriptions → model export is broken
"""

import sys
import os

# Add sherpa-onnx Python path
sys.path.insert(0, '/var/tmp/sherpa-onnx/python-api-examples')

import sherpa_onnx
import soundfile as sf

def test_model(model_dir, test_files):
    """Test model with sherpa-onnx Python bindings."""

    print("=" * 70)
    print("🔍 Testing 1.1B Model with Sherpa-ONNX Python Bindings")
    print("=" * 70)
    print(f"\n📦 Model directory: {model_dir}\n")

    # Model paths
    encoder = f"{model_dir}/encoder.onnx"
    decoder = f"{model_dir}/decoder.onnx"
    joiner = f"{model_dir}/joiner.onnx"
    tokens = f"{model_dir}/tokens.txt"

    # Check files exist
    for f in [encoder, decoder, joiner, tokens]:
        if not os.path.exists(f):
            print(f"❌ File not found: {f}")
            return False

    print("✓ All model files found\n")

    # Create recognizer
    print("📦 Loading model...")
    recognizer = sherpa_onnx.OfflineRecognizer.from_transducer(
        encoder,
        decoder,
        joiner,
        tokens,
        num_threads=1,
        provider="cpu",
        debug=False,
        decoding_method="greedy_search",
        model_type="nemo_transducer"
    )
    print("✓ Model loaded\n")

    # Test each file
    all_passed = True
    for wav_path, expected in test_files:
        if not os.path.exists(wav_path):
            print(f"⚠ {wav_path} not found, skipping...\n")
            continue

        print(f"📝 Testing: {wav_path}")
        print(f"   Expected: '{expected}'")

        # Load audio and decode
        audio, sample_rate = sf.read(wav_path, dtype="float32", always_2d=True)
        audio = audio[:, 0]  # Use first channel

        stream = recognizer.create_stream()
        stream.accept_waveform(sample_rate, audio)
        recognizer.decode_stream(stream)
        result = stream.result.text.strip()

        print(f"   Got:      '{result}'")

        # Check result
        if expected.lower() in result.lower():
            print(f"   ✅ PASS\n")
        else:
            print(f"   ❌ FAIL\n")
            all_passed = False

    return all_passed

if __name__ == "__main__":
    # Model directory
    model_dir = "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-1.1b-converted"

    # Test files
    test_files = [
        ("/tmp/en-short.wav", "Hello world"),
        ("/tmp/en-long.wav", "open source AI community"),
    ]

    # Run test
    all_passed = test_model(model_dir, test_files)

    # Summary
    print("=" * 70)
    if all_passed:
        print("✅ Model works correctly with sherpa-onnx!")
        print("   → Problem is in our Rust implementation")
    else:
        print("❌ Model ALSO fails with sherpa-onnx!")
        print("   → Model export is broken OR test audio doesn't match expected text")
    print("=" * 70)
