#!/usr/bin/env python3
"""
Convert NVIDIA Parakeet-TDT 1.1B model to ONNX format
Based on istupakov's conversion script for 0.6B model
"""

import os
import json
import nemo.collections.asr as nemo_asr
import onnx
from onnx.external_data_helper import convert_model_to_external_data

def convert_parakeet_tdt_to_onnx(
    model_name="nvidia/parakeet-tdt-1.1b",
    output_dir="./parakeet-tdt-1.1b-onnx",
    use_local_attention=False,
    local_attn_context=[128, 128]
):
    """
    Convert Parakeet-TDT model to ONNX format

    Args:
        model_name: HuggingFace model ID or local path
        output_dir: Directory to save ONNX models
        use_local_attention: Enable local attention (for streaming)
        local_attn_context: Context window for local attention
    """

    print(f"Loading model: {model_name}")
    model = nemo_asr.models.ASRModel.from_pretrained(model_name)

    # Optional: Enable local attention for streaming
    if use_local_attention:
        print(f"Enabling local attention with context {local_attn_context}")
        model.change_attention_model('rel_pos_local_attn', local_attn_context)

    # Disable convolution chunking for long audio handling
    print("Disabling convolution chunking")
    model.encoder.set_default_att_context_size([-1, -1])

    # Create output directory
    os.makedirs(output_dir, exist_ok=True)

    # Export model to ONNX
    print(f"Exporting model to ONNX in {output_dir}")
    model.export(
        output=output_dir,
        check_trace=False,  # Skip trace checking for large models
        onnx_opset_version=17,  # Use recent ONNX opset
    )

    # Convert to external data format for better portability
    print("Converting encoder to external data format")
    encoder_path = os.path.join(output_dir, "encoder.onnx")
    if os.path.exists(encoder_path):
        onnx_model = onnx.load(encoder_path)
        convert_model_to_external_data(
            onnx_model,
            all_tensors_to_one_file=True,
            location="encoder.weights",
            size_threshold=0,  # Externalize all weights
            convert_attribute=False
        )
        onnx.save(onnx_model, encoder_path)
        print(f"Encoder saved with external weights")

    # Convert decoder-joiner to external data format
    print("Converting decoder_joint to external data format")
    decoder_joint_path = os.path.join(output_dir, "decoder_joint.onnx")
    if os.path.exists(decoder_joint_path):
        onnx_model = onnx.load(decoder_joint_path)
        convert_model_to_external_data(
            onnx_model,
            all_tensors_to_one_file=True,
            location="decoder_joint.weights",
            size_threshold=0,
            convert_attribute=False
        )
        onnx.save(onnx_model, decoder_joint_path)
        print(f"Decoder-joiner saved with external weights")

    # Create vocab.txt from tokenizer
    print("Creating vocab.txt")
    vocab_path = os.path.join(output_dir, "vocab.txt")
    with open(vocab_path, 'w', encoding='utf-8') as f:
        for token_id in range(len(model.tokenizer.vocab)):
            token = model.tokenizer.ids_to_tokens([token_id])[0]
            f.write(f"{token} {token_id}\n")
    print(f"Vocabulary saved: {len(model.tokenizer.vocab)} tokens")

    # Create config.json with model parameters
    print("Creating config.json")
    config = {
        "features_size": model.preprocessor.featurizer.nfilt,  # 128 for Parakeet
        "subsampling_factor": model.encoder.subsampling_factor,
        "vocab_size": len(model.tokenizer.vocab),
        "sample_rate": model.preprocessor._sample_rate,
        "model_name": model_name,
        "attention_type": "rel_pos_local_attn" if use_local_attention else "rel_pos",
    }
    config_path = os.path.join(output_dir, "config.json")
    with open(config_path, 'w') as f:
        json.dump(config, f, indent=2)

    print(f"\n✓ Conversion complete!")
    print(f"  Output directory: {output_dir}")
    print(f"  Files created:")
    print(f"    - encoder.onnx + encoder.weights")
    print(f"    - decoder_joint.onnx + decoder_joint.weights")
    print(f"    - vocab.txt ({len(model.tokenizer.vocab)} tokens)")
    print(f"    - config.json")
    print(f"\n  Model config:")
    print(f"    Features: {config['features_size']}")
    print(f"    Subsampling: {config['subsampling_factor']}")
    print(f"    Vocab size: {config['vocab_size']}")
    print(f"    Sample rate: {config['sample_rate']}")

if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Convert Parakeet-TDT model to ONNX")
    parser.add_argument(
        "--model",
        default="nvidia/parakeet-tdt-1.1b",
        help="Model name or path (default: nvidia/parakeet-tdt-1.1b)"
    )
    parser.add_argument(
        "--output",
        default="./parakeet-tdt-1.1b-onnx",
        help="Output directory (default: ./parakeet-tdt-1.1b-onnx)"
    )
    parser.add_argument(
        "--local-attention",
        action="store_true",
        help="Enable local attention for streaming"
    )

    args = parser.parse_args()

    convert_parakeet_tdt_to_onnx(
        model_name=args.model,
        output_dir=args.output,
        use_local_attention=args.local_attention
    )
