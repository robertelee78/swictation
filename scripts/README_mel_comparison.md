# Mel Feature Comparison Tools

Tools for analyzing divergence between Rust and Python mel-spectrogram implementations.

## Scripts

### 1. compare_mel_features.py
Statistical comparison of feature arrays.

**Usage:**
```bash
python3.12 scripts/compare_mel_features.py [rust_csv] [python_csv]

# With default paths:
python3.12 scripts/compare_mel_features.py

# With custom paths:
python3.12 scripts/compare_mel_features.py rust_output.csv python_output.csv
```

**Output:**
- Shape validation
- Statistical summaries (mean, std, range)
- Difference metrics (max, mean, median, std)
- Top 10 divergence points
- Overall verdict (match/minor/significant)

### 2. visualize_mel_divergence.py
Visual analysis of feature divergence.

**Usage:**
```bash
python3.12 scripts/visualize_mel_divergence.py [rust_csv] [python_csv] [output_png]

# With defaults:
python3.12 scripts/visualize_mel_divergence.py

# Custom output:
python3.12 scripts/visualize_mel_divergence.py rust.csv python.csv divergence.png
```

**Output:**
- `mel_divergence.png`: 4-panel comparison
  - Rust features heatmap
  - Python features heatmap
  - Absolute difference heatmap
  - Difference over time plot
- `mel_divergence_hotspot.png`: Zoomed view of worst divergence region

## Interpretation Guide

### Verdict Thresholds

**✅ FEATURES MATCH (max diff < 0.001)**
- Bug is NOT in mel feature extraction
- Investigate later stages (encoder, decoder)

**⚠️ MINOR DIFFERENCES (0.001 ≤ max diff < 0.1)**
- Likely floating-point precision differences
- May be acceptable depending on model sensitivity
- Consider if using different FFT libraries

**❌ SIGNIFICANT MISMATCH (max diff ≥ 0.1)**
- Bug IS in mel feature extraction pipeline
- Check:
  - FFT implementation differences
  - Window function application
  - Mel filterbank construction
  - Normalization steps

## Expected CSV Format

```csv
frame,feature_idx,value
0,0,0.123456
0,1,0.234567
1,0,0.345678
...
```

Where:
- `frame`: Time frame index
- `feature_idx`: Mel bin index
- `value`: Feature value

## Dependencies

```bash
pip install pandas numpy matplotlib seaborn
```

## Integration with Hive Mind

These tools are used by the Model-Output-Analyst to:
1. Identify exact divergence points
2. Quantify magnitude of differences
3. Visualize patterns in divergence
4. Guide debugging efforts to correct pipeline stage
