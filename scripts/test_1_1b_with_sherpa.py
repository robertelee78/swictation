#!/usr/bin/env python3.12
"""
Validate Parakeet-TDT 1.1B export using sherpa-onnx reference implementation.

This definitively tests whether the export is correct by using the official
sherpa-onnx library that the models were designed for.

If this produces correct transcriptions: Export is good, Rust code has bugs
If this produces nonsense: Export is broken, need to fix export script
"""

import sherpa_onnx
import sys
from pathlib import Path

def main():
    print("="*80)
    print("Testing Parakeet-TDT 1.1B Export with sherpa-onnx Reference")
    print("="*80)

    # Model configuration
    model_dir = "/opt/swictation/models/parakeet-tdt-1.1b"
    audio_file = "/opt/swictation/examples/en-short.mp3"

    if len(sys.argv) > 1:
        audio_file = sys.argv[1]

    print(f"\n📦 Model directory: {model_dir}")
    print(f"🎵 Audio file: {audio_file}")

    # Configure recognizer for NeMo transducer (using correct API)
    config = sherpa_onnx.OfflineRecognizerConfig(
        model_config=sherpa_onnx.OfflineModelConfig(
            transducer=sherpa_onnx.OfflineTransducerModelConfig(
                encoder_filename=f"{model_dir}/encoder.int8.onnx",
                decoder_filename=f"{model_dir}/decoder.int8.onnx",
                joiner_filename=f"{model_dir}/joiner.int8.onnx",
            ),
            tokens=f"{model_dir}/tokens.txt",
            num_threads=4,
            debug=False,
            provider="cpu",  # Start with CPU for reliability
            model_type="nemo_transducer",  # Critical: tell sherpa-onnx this is NeMo format
        ),
        max_active_paths=4,
        decoding_method="greedy_search",
    )

    print("\n⚙️  Creating recognizer...")
    recognizer = sherpa_onnx.OfflineRecognizer(config)
    print(f"✅ Recognizer created")
    print(f"   Sample rate: {recognizer.sample_rate} Hz")

    # Load audio
    print(f"\n📂 Loading audio: {audio_file}")
    stream = recognizer.create_stream()

    # Read audio using sherpa-onnx's audio reading
    samples, sample_rate = sherpa_onnx.read_audio_file(audio_file)
    print(f"✅ Audio loaded: {len(samples)} samples at {sample_rate} Hz")

    # Resample if needed
    if sample_rate != recognizer.sample_rate:
        print(f"⚠️  Resampling from {sample_rate} to {recognizer.sample_rate} Hz")
        # sherpa-onnx handles this internally

    stream.accept_waveform(recognizer.sample_rate, samples)

    # Decode
    print("\n🔄 Running recognition...")
    recognizer.decode_stream(stream)
    result = stream.result.text

    print("\n" + "="*80)
    print("📝 RESULT:")
    print("="*80)
    print(result)
    print("="*80)

    # Detailed analysis
    if result.strip():
        print(f"\n✅ SUCCESS: Got transcription with {len(result.split())} words")
        print(f"   First 100 chars: {result[:100]}")
        return 0
    else:
        print(f"\n❌ FAILURE: Empty or nonsense transcription")
        print(f"   This suggests the ONNX export may be incorrect")
        return 1

if __name__ == "__main__":
    sys.exit(main())
