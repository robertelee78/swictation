#!/usr/bin/env python3.12
"""
Extract mel-filterbank features using torchaudio (Kaldi-compatible).
This replicates the SAME feature extraction logic as sherpa-onnx C++ recognizer.

Uses torchaudio's Kaldi-compatible fbank with settings matching NeMo Parakeet-TDT.
"""

import sys
import numpy as np
import torch
import torchaudio
import torchaudio.compliance.kaldi as kaldi
import soundfile as sf
from pathlib import Path

def main():
    if len(sys.argv) < 3:
        print("Usage: extract_python_mel_features.py <audio_file> <output_csv>")
        sys.exit(1)

    audio_file = sys.argv[1]
    output_csv = sys.argv[2]

    print(f"📂 Loading audio: {audio_file}")

    # Load audio with soundfile
    samples, sample_rate = sf.read(audio_file, dtype='float32')

    # Ensure mono
    if len(samples.shape) > 1:
        samples = samples.mean(axis=1)

    print(f"✅ Loaded {len(samples)} samples at {sample_rate} Hz")

    # Resample to 16kHz if needed
    if sample_rate != 16000:
        print(f"⚠️  Resampling from {sample_rate} to 16000 Hz")
        resampler = torchaudio.transforms.Resample(orig_freq=sample_rate, new_freq=16000)
        samples_tensor = torch.from_numpy(samples).unsqueeze(0)
        samples_tensor = resampler(samples_tensor)
        samples = samples_tensor.squeeze(0).numpy()
        sample_rate = 16000
        print(f"✅ Resampled to {len(samples)} samples at {sample_rate} Hz")

    # Convert to torch tensor
    waveform = torch.from_numpy(samples).unsqueeze(0)  # Shape: (1, num_samples)

    print(f"⚙️  Extracting mel-filterbank features...")
    print(f"   Settings (matching NeMo/Kaldi):")
    print(f"   - num_mel_bins: 80")
    print(f"   - sample_frequency: 16000")
    print(f"   - dither: 0.0")
    print(f"   - snip_edges: False")
    print(f"   - low_freq: 20.0")
    print(f"   - high_freq: -400.0 (Nyquist - 400)")

    # Extract features using Kaldi-compatible fbank
    # These settings MUST match sherpa-onnx/kaldi settings
    features = kaldi.fbank(
        waveform,
        num_mel_bins=80,          # N_MEL_FEATURES in Rust
        sample_frequency=16000,    # Target sample rate
        dither=0.0,               # No dithering (matches Rust)
        snip_edges=False,         # Don't snip edges
        low_freq=20.0,            # Low frequency cutoff
        high_freq=7600.0,         # High frequency = Nyquist - 400 = 8000 - 400 = 7600
        preemphasis_coefficient=0.97,  # Standard pre-emphasis
        frame_length=25.0,        # 25ms frames
        frame_shift=10.0,         # 10ms shift (80% overlap)
        use_energy=False,         # Use log mel filterbank, not energy
        remove_dc_offset=False,   # CRITICAL: NeMo models use remove_dc_offset=False
    )

    features_np = features.numpy()

    print(f"✅ Extracted features: shape {features_np.shape}")
    print(f"   Frames: {features_np.shape[0]}")
    print(f"   Mel bins: {features_np.shape[1]}")

    # Statistics
    mean = np.mean(features_np)
    std = np.std(features_np)
    print(f"📊 Statistics:")
    print(f"   Mean: {mean:.6f}")
    print(f"   Std: {std:.6f}")
    print(f"   Min: {np.min(features_np):.6f}")
    print(f"   Max: {np.max(features_np):.6f}")

    # Export to CSV (matching Rust format)
    print(f"💾 Exporting to: {output_csv}")
    with open(output_csv, 'w') as f:
        f.write("frame,feature_idx,value\n")
        for frame_idx in range(features_np.shape[0]):
            for feat_idx in range(features_np.shape[1]):
                value = features_np[frame_idx, feat_idx]
                f.write(f"{frame_idx},{feat_idx},{value}\n")

    print(f"✅ Python features exported successfully")
    print(f"   Total data points: {features_np.shape[0] * features_np.shape[1]}")

if __name__ == "__main__":
    main()
