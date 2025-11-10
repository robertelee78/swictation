#!/usr/bin/env python3.12
"""
CRITICAL VALIDATION: Test if Parakeet-TDT 1.1B export is correct.

Uses sherpa-onnx reference implementation (the library the models were designed for).

Results:
- Correct transcription → Export is GOOD, Rust code has bugs
- Nonsense/empty → Export is BROKEN, need to fix export script
"""

import sherpa_onnx
import sys
from pathlib import Path

def main():
    print("="*80)
    print("🔬 CRITICAL TEST: Validating 1.1B ONNX Export with sherpa-onnx")
    print("="*80)

    # Model configuration
    model_dir = "/opt/swictation/models/parakeet-tdt-1.1b"
    audio_file = "/opt/swictation/examples/en-short.mp3"

    if len(sys.argv) > 1:
        audio_file = sys.argv[1]

    print(f"\n📦 Model: {model_dir}")
    print(f"🎵 Audio: {audio_file}\n")

    # Use the correct API from research: from_transducer()
    print("⚙️  Initializing recognizer with from_transducer()...")
    try:
        recognizer = sherpa_onnx.OfflineRecognizer.from_transducer(
            encoder=f"{model_dir}/encoder.int8.onnx",
            decoder=f"{model_dir}/decoder.int8.onnx",
            joiner=f"{model_dir}/joiner.int8.onnx",
            tokens=f"{model_dir}/tokens.txt",
            num_threads=4,
            sample_rate=16000,
            feature_dim=80,  # Critical: 1.1B uses 80 mel features
            decoding_method="greedy_search",
            max_active_paths=4,
            provider="cpu",
            model_type="nemo_transducer",  # Tell sherpa-onnx this is NeMo format
        )
        print(f"✅ Recognizer created successfully")
    except Exception as e:
        print(f"❌ Failed to create recognizer: {e}")
        return 1

    # Load and process audio
    print(f"\n📂 Loading audio...")
    try:
        # Use read_wave function (different API in Python bindings)
        import wave
        import numpy as np
        with wave.open(audio_file, 'rb') as wf:
            sample_rate = wf.getframerate()
            frames = wf.readframes(wf.getnframes())
            samples = np.frombuffer(frames, dtype=np.int16).astype(np.float32) / 32768.0
        print(f"✅ Loaded {len(samples)} samples at {sample_rate} Hz")
    except Exception as e:
        print(f"❌ Failed to load audio: {e}")
        return 1

    # Create stream and decode
    print(f"\n🔄 Running recognition...")
    try:
        stream = recognizer.create_stream()
        stream.accept_waveform(sample_rate, samples)
        recognizer.decode_stream(stream)
        result = stream.result.text
    except Exception as e:
        print(f"❌ Recognition failed: {e}")
        return 1

    # Analyze result
    print("\n" + "="*80)
    print("📝 TRANSCRIPTION RESULT:")
    print("="*80)
    print(f"\n{result}\n")
    print("="*80)

    # Verdict
    if result.strip() and len(result.strip()) > 10:
        print(f"\n✅ SUCCESS: Export appears CORRECT")
        print(f"   Got {len(result.split())} words, {len(result)} chars")
        print(f"\n🔍 CONCLUSION:")
        print(f"   - ONNX export is valid ✅")
        print(f"   - Rust OrtRecognizer has a bug 🐛")
        print(f"   - Need to debug decoder/joiner logic in Rust")
        return 0
    elif result.strip() == "mmhmm" or len(result.strip()) < 5:
        print(f"\n❌ FAILURE: Export appears BROKEN")
        print(f"   Got nonsense: '{result.strip()}'")
        print(f"\n🔍 CONCLUSION:")
        print(f"   - ONNX export may be incorrect ⚠️")
        print(f"   - Export script needs fixing")
        print(f"   - OR sherpa-onnx also has issues with this export")
        return 1
    else:
        print(f"\n⚠️  UNCLEAR: Got partial result")
        print(f"   Length: {len(result)} chars")
        print(f"   May need manual review")
        return 2

if __name__ == "__main__":
    sys.exit(main())
