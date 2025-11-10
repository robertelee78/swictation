#!/usr/bin/env python3
"""
Export NVIDIA Parakeet-TDT 1.1B model to ONNX format
Based on sherpa-onnx official export script for parakeet-tdt-0.6b-v3
"""

from pathlib import Path
from typing import Dict
import os

import nemo.collections.asr as nemo_asr
import onnx
import torch
from onnxruntime.quantization import QuantType, quantize_dynamic


def add_meta_data(filename: str, meta_data: Dict[str, str]):
    """Add meta data to an ONNX model. It is changed in-place.

    Args:
      filename:
        Filename of the ONNX model to be changed.
      meta_data:
        Key-value pairs.
    """
    model = onnx.load(filename)
    while len(model.metadata_props):
        model.metadata_props.pop()

    for key, value in meta_data.items():
        meta = model.metadata_props.add()
        meta.key = key
        meta.value = str(value)

    if filename == "encoder.onnx":
        external_filename = "encoder"
        onnx.save(
            model,
            filename,
            save_as_external_data=True,
            all_tensors_to_one_file=True,
            location=external_filename + ".weights",
        )
    else:
        onnx.save(model, filename)


@torch.no_grad()
def main():
    print("="*80)
    print("Parakeet-TDT 1.1B ONNX Export (sherpa-onnx method)")
    print("="*80)

    # Check for local .nemo file first, otherwise download from HuggingFace
    nemo_file = Path("./parakeet-tdt-1.1b.nemo")
    if nemo_file.is_file():
        print(f"Loading from local file: {nemo_file}")
        asr_model = nemo_asr.models.ASRModel.restore_from(
            restore_path=str(nemo_file)
        )
    else:
        print("Downloading nvidia/parakeet-tdt-1.1b from HuggingFace...")
        asr_model = nemo_asr.models.ASRModel.from_pretrained(
            model_name="nvidia/parakeet-tdt-1.1b"
        )

    asr_model.eval()
    print(f"Model loaded successfully")
    print(f"  Encoder: {type(asr_model.encoder).__name__}")
    print(f"  Decoder: {type(asr_model.decoder).__name__}")
    print(f"  Joint: {type(asr_model.joint).__name__}")

    # Export vocabulary (tokens.txt)
    print("\nExporting vocabulary...")
    with open("./tokens.txt", "w", encoding="utf-8") as f:
        for i, s in enumerate(asr_model.joint.vocabulary):
            f.write(f"{s} {i}\n")
        # Add blank token at the end
        f.write(f"<blk> {i+1}\n")
    vocab_size = len(asr_model.joint.vocabulary) + 1
    print(f"  Saved to tokens.txt ({vocab_size} tokens including <blk>)")

    # Export ONNX models using NeMo's built-in export
    print("\nExporting ONNX models...")
    print("  Exporting encoder.onnx...")
    asr_model.encoder.export("encoder.onnx")

    print("  Exporting decoder.onnx...")
    asr_model.decoder.export("decoder.onnx")

    print("  Exporting joiner.onnx...")
    asr_model.joint.export("joiner.onnx")

    print("\n Exported models:")
    os.system("ls -lh *.onnx")

    # Get normalization type
    normalize_type = asr_model.cfg.preprocessor.normalize
    if normalize_type == "NA":
        normalize_type = ""

    # Metadata for sherpa-onnx compatibility
    meta_data = {
        "vocab_size": vocab_size,
        "normalize_type": normalize_type,
        "pred_rnn_layers": asr_model.decoder.pred_rnn_layers,
        "pred_hidden": asr_model.decoder.pred_hidden,
        "subsampling_factor": 8,  # Standard for FastConformer
        "model_type": "EncDecRNNTBPEModel",
        "version": "2",
        "model_author": "NeMo",
        "url": "https://huggingface.co/nvidia/parakeet-tdt-1.1b",
        "comment": "TDT (Token-and-Duration Transducer) 1.1B model",
        "feat_dim": 128,  # Mel filterbank features
    }

    print("\n Model metadata:")
    for key, value in meta_data.items():
        print(f"   {key}: {value}")

    # Quantize models to int8 for smaller size
    print("\nQuantizing models to INT8...")
    for m in ["encoder", "decoder", "joiner"]:
        print(f"  Quantizing {m}.onnx...")
        quantize_dynamic(
            model_input=f"./{m}.onnx",
            model_output=f"./{m}.int8.onnx",
            weight_type=QuantType.QUInt8 if m == "encoder" else QuantType.QInt8,
        )

    print("\nQuantized models:")
    os.system("ls -lh *.int8.onnx")

    # Add metadata to ONNX models
    print("\nAdding metadata to models...")
    add_meta_data("encoder.int8.onnx", meta_data)
    add_meta_data("encoder.onnx", meta_data)

    print("\n"+"="*80)
    print("✓ Export complete!")
    print("="*80)
    print("\nFiles created:")
    print("  - encoder.onnx + encoder.weights (fp32)")
    print("  - encoder.int8.onnx (quantized)")
    print("  - decoder.onnx (fp32)")
    print("  - decoder.int8.onnx (quantized)")
    print("  - joiner.onnx (fp32)")
    print("  - joiner.int8.onnx (quantized)")
    print("  - tokens.txt")
    print("\nNext steps:")
    print("  1. Test with sherpa-onnx or parakeet-rs")
    print("  2. Verify encoder input shape with onnxruntime")
    print("  3. Check decoder_joint vs decoder/joiner split")


if __name__ == "__main__":
    main()
