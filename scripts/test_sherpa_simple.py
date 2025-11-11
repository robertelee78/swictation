#!/usr/bin/env python3.12
"""
Simple test to isolate sherpa-onnx issue with 1.1B model.
"""

import sherpa_onnx
import sys

def main():
    model_dir = "/opt/swictation/models/parakeet-tdt-1.1b"

    print("Testing sherpa-onnx with 1.1B model...")
    print(f"sherpa-onnx version: {sherpa_onnx.__version__}")

    try:
        print("\n1. Creating recognizer...")
        recognizer = sherpa_onnx.OfflineRecognizer.from_transducer(
            encoder=f"{model_dir}/encoder.int8.onnx",
            decoder=f"{model_dir}/decoder.int8.onnx",
            joiner=f"{model_dir}/joiner.int8.onnx",
            tokens=f"{model_dir}/tokens.txt",
            num_threads=2,
            sample_rate=16000,
            feature_dim=80,
            decoding_method="greedy_search",
            max_active_paths=4,
            provider="cpu",
            model_type="nemo_transducer",
            debug=True,  # Enable debug output
        )
        print("✅ Recognizer created successfully!")

        print(f"\n2. Sample rate: {recognizer.sample_rate} Hz")

        return 0

    except Exception as e:
        print(f"\n❌ Error: {e}")
        import traceback
        traceback.print_exc()
        return 1

if __name__ == "__main__":
    sys.exit(main())
