#!/usr/bin/env python3
"""
Convert NVIDIA Parakeet-TDT models from NeMo format to ONNX for sherpa-onnx.

This script downloads and converts Parakeet-TDT models (0.6B, 1.1B) to float32 ONNX format
for GPU-accelerated inference with sherpa-rs.

Requirements:
    pip install nemo_toolkit[asr] onnx onnxruntime

Usage:
    python3 convert-parakeet-to-onnx.py 0.6b  # Convert 0.6B model
    python3 convert-parakeet-to-onnx.py 1.1b  # Convert 1.1B model
    python3 convert-parakeet-to-onnx.py all   # Convert all models
"""

import sys
import os
from pathlib import Path

def convert_model(model_name: str, output_dir: str):
    """Convert a Parakeet TDT model to ONNX format."""
    try:
        import nemo.collections.asr as nemo_asr
    except ImportError:
        print("❌ Error: nemo_toolkit not installed")
        print("Install with: pip install nemo_toolkit[asr] onnx onnxruntime")
        sys.exit(1)

    print(f"\n{'='*60}")
    print(f"Converting {model_name} to ONNX")
    print(f"{'='*60}\n")

    # Map model size to HuggingFace identifier
    model_map = {
        "0.6b-v2": "nvidia/parakeet-tdt-0.6b-v2",
        "0.6b-v3": "nvidia/parakeet-tdt-0.6b-v3",
        "1.1b": "nvidia/parakeet-tdt_ctc-1.1b",
    }

    if model_name not in model_map:
        print(f"❌ Unknown model: {model_name}")
        print(f"Available models: {', '.join(model_map.keys())}")
        sys.exit(1)

    hf_model_id = model_map[model_name]
    output_path = Path(output_dir) / f"sherpa-onnx-nemo-parakeet-tdt-{model_name}"
    output_path.mkdir(parents=True, exist_ok=True)

    # Load model
    print(f"📥 Loading model from HuggingFace: {hf_model_id}")
    try:
        asr_model = nemo_asr.models.ASRModel.from_pretrained(hf_model_id)
    except Exception as e:
        print(f"❌ Error loading model: {e}")
        print("Note: Model download requires ~2-4GB and may take several minutes")
        sys.exit(1)

    # Prepare for export
    print(f"⚙️  Preparing model for ONNX export...")
    asr_model.eval()
    asr_model.to('cpu')

    # Set export config for TDT models
    if "ctc" in model_name.lower():
        print("⚙️  Configuring CTC export...")
        asr_model.set_export_config({'decoder_type': 'ctc'})
        onnx_file = str(output_path / "model.onnx")
    else:
        # For TDT transducer models, don't use set_export_config
        # Just export with base filename - NeMo will create encoder/decoder/joiner automatically
        print("⚠️  Note: TDT transducer export generates encoder.onnx, decoder.onnx, joiner.onnx")
        onnx_file = str(output_path / "model.onnx")

    # Export to ONNX
    print(f"📤 Exporting to ONNX: {output_path}")
    try:
        asr_model.export(onnx_file)
    except Exception as e:
        print(f"❌ Export failed: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)

    # Extract vocabulary tokens
    print(f"📝 Extracting vocabulary to tokens.txt")
    tokens_file = output_path / "tokens.txt"
    with open(tokens_file, 'w') as f:
        if hasattr(asr_model, 'decoder') and hasattr(asr_model.decoder, 'vocabulary'):
            for i, token in enumerate(asr_model.decoder.vocabulary):
                f.write(f"{token} {i}\n")
            f.write(f"<blk> {i+1}\n")
        elif hasattr(asr_model, 'tokenizer'):
            vocab = asr_model.tokenizer.vocab
            # Handle both dict and list vocab formats
            if isinstance(vocab, dict):
                for token, idx in sorted(vocab.items(), key=lambda x: x[1]):
                    f.write(f"{token} {idx}\n")
            elif isinstance(vocab, list):
                for idx, token in enumerate(vocab):
                    f.write(f"{token} {idx}\n")

    # Verify files
    print(f"\n✅ Conversion complete!")
    print(f"📁 Output directory: {output_path}")
    print(f"\nGenerated files:")
    for file in sorted(output_path.glob("*.onnx")):
        size_mb = file.stat().st_size / (1024 * 1024)
        print(f"  - {file.name} ({size_mb:.1f} MB)")
    if tokens_file.exists():
        print(f"  - {tokens_file.name}")

    print(f"\n💡 Usage in Rust:")
    print(f'   let recognizer = Recognizer::new("{output_path}", true)?;')
    print(f"\n🚀 Expected GPU speedup: 5-10x on long audio (>5 seconds)")
    print(f"⚠️  Note: GPU is slower on short audio (<3 seconds) due to overhead")

def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    model_arg = sys.argv[1].lower()
    output_dir = sys.argv[2] if len(sys.argv) > 2 else "/opt/swictation/models"

    if model_arg == "all":
        models = ["0.6b-v3", "1.1b"]
        print(f"Converting all models: {', '.join(models)}")
        for model in models:
            try:
                convert_model(model, output_dir)
            except Exception as e:
                print(f"❌ Failed to convert {model}: {e}")
                continue
    else:
        convert_model(model_arg, output_dir)

    print(f"\n{'='*60}")
    print(f"✅ All conversions complete!")
    print(f"{'='*60}")

if __name__ == "__main__":
    main()
