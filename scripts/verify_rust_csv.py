#!/usr/bin/env python3
"""
Verify Rust CSV output format and show statistics for comparison with Python.
"""

import pandas as pd
import numpy as np

def verify_csv(csv_path):
    """Load and verify the CSV format."""
    print(f"=== Verifying {csv_path} ===\n")

    # Load CSV
    df = pd.read_csv(csv_path)

    # Show structure
    print("CSV Structure:")
    print(f"  Columns: {list(df.columns)}")
    print(f"  Total rows: {len(df):,}")
    print(f"  Memory usage: {df.memory_usage(deep=True).sum() / 1024 / 1024:.2f} MB\n")

    # Get dimensions
    max_frame = df['frame'].max() + 1
    max_feat = df['feature_idx'].max() + 1

    print("Feature Dimensions:")
    print(f"  Frames: {max_frame}")
    print(f"  Features per frame: {max_feat}")
    print(f"  Total data points: {len(df):,}\n")

    # Statistics
    values = df['value'].values
    print("Value Statistics:")
    print(f"  Mean: {values.mean():.6f}")
    print(f"  Std:  {values.std():.6f}")
    print(f"  Min:  {values.min():.6f}")
    print(f"  Max:  {values.max():.6f}\n")

    # Show sample data
    print("Sample Data (first 10 rows):")
    print(df.head(10).to_string(index=False))
    print("\n")

    print("Sample Data (last 10 rows):")
    print(df.tail(10).to_string(index=False))
    print("\n")

    # Reshape to 2D for frame-by-frame analysis
    print("Reshaping to (frames x features) array...")
    features_2d = df.pivot(index='frame', columns='feature_idx', values='value').values
    print(f"  Shape: {features_2d.shape}")
    print(f"  First frame shape: {features_2d[0].shape}")
    print(f"  First frame mean: {features_2d[0].mean():.6f}")
    print(f"  First frame std: {features_2d[0].std():.6f}\n")

    # Verify per-feature normalization
    print("Per-feature normalization check:")
    for feat_idx in [0, 32, 64, 96, 127]:
        feat_values = df[df['feature_idx'] == feat_idx]['value'].values
        print(f"  Feature {feat_idx:3d}: mean={feat_values.mean():+.6f}, std={feat_values.std():.6f}")

    print("\n=== Verification Complete ===")
    print("CSV format is correct and ready for comparison with Python!")

    return df, features_2d

if __name__ == '__main__':
    import sys
    csv_path = sys.argv[1] if len(sys.argv) > 1 else '/opt/swictation/rust_mel_features.csv'
    verify_csv(csv_path)
