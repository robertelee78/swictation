#!/usr/bin/env python3.12
"""
Visualize mel-spectrogram feature divergence between Rust and Python.
Creates heatmap showing where differences occur across time/frequency.
"""

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import seaborn as sns
import sys
from pathlib import Path

def load_features_csv(csv_path: str) -> np.ndarray:
    """Load mel features from CSV into numpy array"""
    df = pd.read_csv(csv_path)

    # Pivot to 2D array (frames x features)
    num_frames = df['frame'].max() + 1
    num_features = df['feature_idx'].max() + 1

    features = np.zeros((num_frames, num_features))
    for _, row in df.iterrows():
        features[int(row['frame']), int(row['feature_idx'])] = row['value']

    return features

def visualize_divergence(rust_csv: str, python_csv: str, output_path: str = "mel_divergence.png"):
    """Create visualization of feature divergence"""

    print(f"Loading features for visualization...")
    rust_features = load_features_csv(rust_csv)
    python_features = load_features_csv(python_csv)

    # Compute absolute difference
    diff = np.abs(rust_features - python_features)

    # Create figure with subplots
    fig, axes = plt.subplots(2, 2, figsize=(16, 12))

    # 1. Rust features heatmap
    sns.heatmap(rust_features.T, ax=axes[0, 0], cmap='viridis', cbar_kws={'label': 'Value'})
    axes[0, 0].set_title('Rust Mel Features', fontsize=14, fontweight='bold')
    axes[0, 0].set_xlabel('Frame (time)')
    axes[0, 0].set_ylabel('Mel Bin (frequency)')

    # 2. Python features heatmap
    sns.heatmap(python_features.T, ax=axes[0, 1], cmap='viridis', cbar_kws={'label': 'Value'})
    axes[0, 1].set_title('Python Mel Features', fontsize=14, fontweight='bold')
    axes[0, 1].set_xlabel('Frame (time)')
    axes[0, 1].set_ylabel('Mel Bin (frequency)')

    # 3. Absolute difference heatmap
    sns.heatmap(diff.T, ax=axes[1, 0], cmap='hot', cbar_kws={'label': 'Absolute Difference'})
    axes[1, 0].set_title(f'Absolute Difference (max: {diff.max():.6f})', fontsize=14, fontweight='bold')
    axes[1, 0].set_xlabel('Frame (time)')
    axes[1, 0].set_ylabel('Mel Bin (frequency)')

    # 4. Difference statistics by frame
    frame_max_diff = diff.max(axis=1)
    frame_mean_diff = diff.mean(axis=1)

    axes[1, 1].plot(frame_max_diff, label='Max diff per frame', alpha=0.7)
    axes[1, 1].plot(frame_mean_diff, label='Mean diff per frame', alpha=0.7)
    axes[1, 1].set_title('Divergence Over Time', fontsize=14, fontweight='bold')
    axes[1, 1].set_xlabel('Frame (time)')
    axes[1, 1].set_ylabel('Difference')
    axes[1, 1].legend()
    axes[1, 1].grid(True, alpha=0.3)

    plt.tight_layout()
    plt.savefig(output_path, dpi=150, bbox_inches='tight')
    print(f"✅ Visualization saved to: {output_path}")

    # Also create a focused view of the worst divergence region
    if diff.max() > 0.001:
        fig2, ax = plt.subplots(figsize=(12, 8))

        # Find region with worst divergence
        worst_frame = np.argmax(diff.max(axis=1))
        frame_start = max(0, worst_frame - 10)
        frame_end = min(diff.shape[0], worst_frame + 10)

        region_diff = diff[frame_start:frame_end, :]

        sns.heatmap(region_diff.T, ax=ax, cmap='hot', cbar_kws={'label': 'Absolute Difference'})
        ax.set_title(f'Divergence Hotspot (frames {frame_start}-{frame_end})', fontsize=14, fontweight='bold')
        ax.set_xlabel('Frame offset')
        ax.set_ylabel('Mel Bin (frequency)')

        hotspot_path = output_path.replace('.png', '_hotspot.png')
        plt.savefig(hotspot_path, dpi=150, bbox_inches='tight')
        print(f"✅ Hotspot visualization saved to: {hotspot_path}")

if __name__ == "__main__":
    rust_csv = sys.argv[1] if len(sys.argv) > 1 else "/opt/swictation/rust_mel_features.csv"
    python_csv = sys.argv[2] if len(sys.argv) > 2 else "/opt/swictation/python_mel_features.csv"
    output = sys.argv[3] if len(sys.argv) > 3 else "/opt/swictation/mel_divergence.png"

    visualize_divergence(rust_csv, python_csv, output)
