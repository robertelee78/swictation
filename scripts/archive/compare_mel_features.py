#!/usr/bin/env python3.12
"""
Compare mel features from Rust and Python implementations element-by-element.
Identifies divergence points and provides diagnostic guidance.
"""

import sys
import numpy as np
import pandas as pd

def load_features(csv_path):
    """Load features from CSV (frame, feature_idx, value format)"""
    df = pd.read_csv(csv_path)

    # Determine dimensions
    num_frames = df['frame'].max() + 1
    num_features = df['feature_idx'].max() + 1

    # Reshape to 2D array
    features = np.zeros((num_frames, num_features))
    for _, row in df.iterrows():
        features[int(row['frame']), int(row['feature_idx'])] = row['value']

    return features

def compute_feature_statistics(features, name):
    """Compute comprehensive statistics for features"""
    return {
        'name': name,
        'mean': np.mean(features),
        'std': np.std(features),
        'min': np.min(features),
        'max': np.max(features),
        'median': np.median(features),
        'q25': np.percentile(features, 25),
        'q75': np.percentile(features, 75)
    }

def find_divergence_point(rust_features, python_features):
    """Find the first frame where features significantly diverge"""
    diff = np.abs(rust_features - python_features)
    frame_diffs = np.mean(diff, axis=1)

    # Find first frame with mean difference > threshold
    THRESHOLD = 1e-3
    divergent_frames = np.where(frame_diffs > THRESHOLD)[0]

    if len(divergent_frames) > 0:
        return divergent_frames[0]
    return None

def analyze_per_feature_differences(rust_features, python_features):
    """Analyze differences for each mel feature dimension"""
    diff = np.abs(rust_features - python_features)
    num_features = rust_features.shape[1]

    feature_diffs = []
    for feat_idx in range(num_features):
        mean_diff = np.mean(diff[:, feat_idx])
        max_diff = np.max(diff[:, feat_idx])
        feature_diffs.append((feat_idx, mean_diff, max_diff))

    # Sort by mean difference
    feature_diffs.sort(key=lambda x: x[1], reverse=True)
    return feature_diffs

def main():
    if len(sys.argv) < 3:
        print("Usage: compare_mel_features.py <rust_csv> <python_csv>")
        sys.exit(1)

    rust_csv = sys.argv[1]
    python_csv = sys.argv[2]

    print("="*80)
    print("📊 MEL FEATURE COMPARISON: Rust vs Python")
    print("="*80)
    print()

    # Load features
    print(f"📂 Loading Rust features: {rust_csv}")
    rust_features = load_features(rust_csv)
    print(f"   Shape: {rust_features.shape}")

    print(f"📂 Loading Python features: {python_csv}")
    python_features = load_features(python_csv)
    print(f"   Shape: {python_features.shape}")
    print()

    # Handle shape mismatch (compare overlapping region)
    if rust_features.shape != python_features.shape:
        print(f"⚠️  WARNING: Shape mismatch - comparing overlapping region only")
        print(f"   Rust: {rust_features.shape}")
        print(f"   Python: {python_features.shape}")

        # Use minimum dimensions for comparison
        min_frames = min(rust_features.shape[0], python_features.shape[0])
        min_features = min(rust_features.shape[1], python_features.shape[1])

        print(f"   Comparing: ({min_frames}, {min_features})")
        print()

        # Trim to overlapping region
        rust_features = rust_features[:min_frames, :min_features]
        python_features = python_features[:min_frames, :min_features]

    num_frames, num_features = rust_features.shape

    # Compute differences
    diff = np.abs(rust_features - python_features)
    max_diff = np.max(diff)
    mean_diff = np.mean(diff)
    median_diff = np.median(diff)

    # Find worst mismatches
    worst_frame, worst_feat = np.unravel_index(np.argmax(diff), diff.shape)

    # Statistics
    print("📊 STATISTICS:")
    rust_stats = compute_feature_statistics(rust_features, "Rust")
    python_stats = compute_feature_statistics(python_features, "Python")

    print(f"   Rust:")
    print(f"      Mean: {rust_stats['mean']:.6f}, Std: {rust_stats['std']:.6f}")
    print(f"      Min: {rust_stats['min']:.6f}, Max: {rust_stats['max']:.6f}")
    print(f"      Median: {rust_stats['median']:.6f}, Q25: {rust_stats['q25']:.6f}, Q75: {rust_stats['q75']:.6f}")

    print(f"   Python:")
    print(f"      Mean: {python_stats['mean']:.6f}, Std: {python_stats['std']:.6f}")
    print(f"      Min: {python_stats['min']:.6f}, Max: {python_stats['max']:.6f}")
    print(f"      Median: {python_stats['median']:.6f}, Q25: {python_stats['q25']:.6f}, Q75: {python_stats['q75']:.6f}")
    print()

    print("📏 DIFFERENCE METRICS:")
    print(f"   Max difference: {max_diff:.6f}")
    print(f"   Mean difference: {mean_diff:.6f}")
    print(f"   Median difference: {median_diff:.6f}")
    print(f"   Worst mismatch at: frame={worst_frame}, feature={worst_feat}")
    print(f"      Rust value: {rust_features[worst_frame, worst_feat]:.6f}")
    print(f"      Python value: {python_features[worst_frame, worst_feat]:.6f}")
    print(f"      Difference: {diff[worst_frame, worst_feat]:.6f}")
    print()

    # Find divergence point
    divergence_frame = find_divergence_point(rust_features, python_features)
    if divergence_frame is not None:
        print(f"⚠️  DIVERGENCE POINT: First significant difference at frame {divergence_frame}")
        print(f"   This suggests the issue may be cumulative or frame-dependent")
        print()

    # Per-frame analysis
    frame_diffs = np.mean(diff, axis=1)
    worst_frames = np.argsort(frame_diffs)[-5:]

    print("🎯 WORST 5 FRAMES (highest mean difference):")
    for i, frame_idx in enumerate(reversed(worst_frames)):
        mean_diff_frame = frame_diffs[frame_idx]
        max_diff_frame = np.max(diff[frame_idx, :])
        print(f"   #{i+1}: Frame {frame_idx} - mean_diff={mean_diff_frame:.6f}, max_diff={max_diff_frame:.6f}")
    print()

    # Per-feature analysis
    feature_diffs = analyze_per_feature_differences(rust_features, python_features)
    print("🔍 WORST 5 MEL FEATURES (highest mean difference):")
    for i, (feat_idx, mean_diff_feat, max_diff_feat) in enumerate(feature_diffs[:5]):
        print(f"   #{i+1}: Feature {feat_idx} - mean_diff={mean_diff_feat:.6f}, max_diff={max_diff_feat:.6f}")
    print()

    # Correlation
    correlation = np.corrcoef(rust_features.flatten(), python_features.flatten())[0, 1]
    print(f"🔗 CORRELATION: {correlation:.6f}")

    # Check if features are scaled differently
    rust_norm = rust_features / (np.std(rust_features) + 1e-10)
    python_norm = python_features / (np.std(python_features) + 1e-10)
    normalized_correlation = np.corrcoef(rust_norm.flatten(), python_norm.flatten())[0, 1]
    print(f"🔗 NORMALIZED CORRELATION: {normalized_correlation:.6f}")
    print()

    # Threshold-based verdict
    TOLERANCE = 1e-4  # Very strict tolerance
    LOOSE_TOLERANCE = 1e-3  # Looser tolerance for investigation

    print("="*80)
    if max_diff < TOLERANCE:
        print("✅ VERDICT: Features MATCH (within strict tolerance)")
        print("="*80)
        print()
        print("🎯 DIAGNOSIS:")
        print("   The mel feature extraction is CORRECT in Rust.")
        print("   The bug is in the ENCODER/DECODER pipeline:")
        print()
        print("   Next steps:")
        print("   1. Check encoder input tensor format (shape, axis order)")
        print("   2. Verify encoder output values match Python")
        print("   3. Check ONNX Runtime version consistency")
        print("   4. Inspect decoder/joiner tensor shapes")
        print("   5. Compare CTC blank token handling")
        print("   6. Verify temperature/log_softmax parameters")
        sys.exit(0)
    elif max_diff < LOOSE_TOLERANCE:
        print("⚠️  VERDICT: Features CLOSE but not identical (within loose tolerance)")
        print("="*80)
        print()
        print("🎯 DIAGNOSIS:")
        print("   Small numerical differences detected (likely acceptable).")
        print("   Could be due to:")
        print("   - Floating point precision differences")
        print("   - Different library implementations (subtle differences)")
        print("   - Normalization scale factors")
        print()
        print(f"   Max difference: {max_diff:.6f} (strict: {TOLERANCE}, loose: {LOOSE_TOLERANCE})")
        print()
        print("   Recommendations:")
        print("   1. If correlation > 0.999, proceed to encoder/decoder analysis")
        print("   2. If correlation < 0.999, investigate audio preprocessing")
        print("   3. Check if normalization constants differ")

        if correlation > 0.999:
            print()
            print("   ✅ High correlation detected - proceed to encoder/decoder")
            sys.exit(0)
        else:
            print()
            print("   ⚠️  Lower correlation - investigate preprocessing first")
            sys.exit(1)
    else:
        print("❌ VERDICT: Features DIFFER significantly (outside tolerance)")
        print("="*80)
        print()
        print("🎯 DIAGNOSIS:")
        print("   The mel feature extraction has BUGS in Rust.")
        print("   The difference is in the AUDIO PREPROCESSING:")
        print()
        print("   Investigate (in order of likelihood):")
        print("   1. STFT computation (FFT implementation differences)")
        print("   2. Window function (Povey vs Hamming/Hann)")
        print("   3. Mel filterbank triangular bins (boundary conditions)")
        print("   4. Power spectrum vs magnitude spectrum")
        print("   5. Per-feature normalization (mean/std calculation)")
        print("   6. Frame stride and padding")
        print("   7. Pre-emphasis filter")
        print("   8. Log-mel computation (log base, epsilon value)")
        print()
        print(f"   Max difference: {max_diff:.6f} (tolerance: {TOLERANCE})")
        print(f"   Correlation: {correlation:.6f}")

        if divergence_frame is not None:
            print()
            print(f"   ⚠️  Divergence starts at frame {divergence_frame}")
            print(f"      Check if there's cumulative error or frame-dependent logic")

        sys.exit(1)

if __name__ == "__main__":
    main()
