#!/usr/bin/env python3.12
"""
Compare raw audio samples from Rust and Python to identify loading/resampling differences.
"""

import sys
import numpy as np
import pandas as pd
import torch
import torchaudio
import soundfile as sf

def load_rust_samples(csv_path):
    """Load samples exported from Rust"""
    df = pd.read_csv(csv_path)
    return df['value'].values

def main():
    # Load Rust samples
    print("📂 Loading Rust raw audio samples from CSV...")
    rust_samples = load_rust_samples('/tmp/rust_raw_audio.csv')
    print(f"   Rust samples: {len(rust_samples)}")

    # Load Python samples (same as extract_python_mel_features.py)
    print("📂 Loading Python audio...")
    audio_file = '/opt/swictation/examples/en-short.mp3'
    samples, sample_rate = sf.read(audio_file, dtype='float32')

    # Ensure mono
    if len(samples.shape) > 1:
        samples = samples.mean(axis=1)

    # Resample to 16kHz
    if sample_rate != 16000:
        print(f"⚙️  Resampling from {sample_rate} to 16000 Hz...")
        resampler = torchaudio.transforms.Resample(orig_freq=sample_rate, new_freq=16000)
        samples_tensor = torch.from_numpy(samples).unsqueeze(0)
        samples_tensor = resampler(samples_tensor)
        samples = samples_tensor.squeeze(0).numpy()

    print(f"   Python samples: {len(samples)}")

    # Statistics
    print("\n📊 STATISTICS:")
    print("   Rust:")
    print(f"      Length: {len(rust_samples)}")
    print(f"      Mean: {np.mean(rust_samples):.6f}")
    print(f"      RMS: {np.sqrt(np.mean(rust_samples**2)):.6f}")
    print(f"      Min: {np.min(rust_samples):.6f}")
    print(f"      Max: {np.max(rust_samples):.6f}")

    print("   Python:")
    print(f"      Length: {len(samples)}")
    print(f"      Mean: {np.mean(samples):.6f}")
    print(f"      RMS: {np.sqrt(np.mean(samples**2)):.6f}")
    print(f"      Min: {np.min(samples):.6f}")
    print(f"      Max: {np.max(samples):.6f}")

    # Compare overlapping region
    min_len = min(len(rust_samples), len(samples))
    rust_clip = rust_samples[:min_len]
    python_clip = samples[:min_len]

    # Differences
    diff = np.abs(rust_clip - python_clip)
    max_diff = np.max(diff)
    mean_diff = np.mean(diff)

    # Correlation
    correlation = np.corrcoef(rust_clip, python_clip)[0, 1]

    print("\n📏 COMPARISON (overlapping region):")
    print(f"   Comparing: {min_len} samples")
    print(f"   Max difference: {max_diff:.6f}")
    print(f"   Mean difference: {mean_diff:.6f}")
    print(f"   Correlation: {correlation:.6f}")

    # RMS ratio
    rust_rms = np.sqrt(np.mean(rust_clip**2))
    python_rms = np.sqrt(np.mean(python_clip**2))
    rms_ratio = rust_rms / python_rms if python_rms > 0 else 0
    rms_db = 20 * np.log10(rms_ratio) if rms_ratio > 0 else 0

    print(f"\n🔊 AMPLITUDE ANALYSIS:")
    print(f"   Rust RMS: {rust_rms:.6f}")
    print(f"   Python RMS: {python_rms:.6f}")
    print(f"   Ratio: {rms_ratio:.4f}")
    print(f"   In dB: {rms_db:.2f} dB")

    print("\n" + "="*80)
    if correlation > 0.999 and mean_diff < 1e-5:
        print("✅ VERDICT: Raw audio samples MATCH")
        print("   The offset is NOT from audio loading/resampling")
        print("   Bug must be in mel-spectrogram extraction")
    elif correlation > 0.99:
        print("⚠️  VERDICT: Raw audio samples CLOSE but not identical")
        print("   Small numerical differences (likely acceptable)")
        print(f"   Amplitude ratio: {rms_ratio:.4f} ({rms_db:.2f} dB)")
    else:
        print("❌ VERDICT: Raw audio samples DIFFER significantly")
        print("   The offset originates from audio loading/resampling")
        print(f"   RMS difference: {rms_db:.2f} dB")
    print("="*80)

if __name__ == "__main__":
    main()
