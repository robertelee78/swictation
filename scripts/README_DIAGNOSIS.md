# MEL Feature Extraction Diagnosis Suite

## Overview

This test suite diagnoses the root cause of the speech recognition accuracy discrepancy between the Rust and Python implementations by comparing mel-spectrogram feature extraction at the lowest level.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                  diagnose_feature_mismatch.sh               │
│                   (Master Orchestrator)                     │
└───────────────────────┬─────────────────────────────────────┘
                        │
        ┌───────────────┼───────────────┐
        │               │               │
        ▼               ▼               ▼
    ┌───────┐   ┌──────────┐   ┌──────────┐
    │ Rust  │   │ Python   │   │ Compare  │
    │Extract│   │ Extract  │   │ Features │
    └───┬───┘   └────┬─────┘   └────┬─────┘
        │            │               │
        ▼            ▼               ▼
    rust_mel    python_mel      comparison
    features    features         report
    .csv        .csv             .txt
```

## Components

### 1. Master Script: `diagnose_feature_mismatch.sh`

**Purpose**: Orchestrates the complete diagnostic workflow

**Features**:
- Color-coded output for readability
- Comprehensive error handling
- Progress tracking
- Automatic cleanup
- Detailed reporting

**Usage**:
```bash
# Use default audio file
./scripts/diagnose_feature_mismatch.sh

# Specify custom audio file
./scripts/diagnose_feature_mismatch.sh /path/to/audio.wav
```

### 2. Rust Feature Extractor: `export_mel_features`

**Location**: `rust-crates/target/release/examples/export_mel_features`

**Purpose**: Extract mel-spectrogram features using Rust implementation

**Output**: CSV file with frame-by-frame mel features (80 dimensions)

**Built from**: `rust-crates/swictation-stt/examples/export_mel_features.rs`

### 3. Python Feature Extractor: `extract_python_mel_features.py`

**Location**: `scripts/extract_python_mel_features.py`

**Purpose**: Extract mel-spectrogram features using sherpa-onnx Python implementation

**Output**: CSV file with frame-by-frame mel features (80 dimensions)

**Dependencies**: sherpa-onnx Python package

### 4. Feature Comparator: `compare_mel_features.py`

**Location**: `scripts/compare_mel_features.py`

**Purpose**: Perform element-by-element comparison of mel features

**Analysis**:
- Statistical comparison (mean, std, min, max)
- Element-wise difference metrics
- Correlation analysis
- Distribution visualization (via text)

## Workflow

### Step 1: Rust Feature Extraction
```
Audio File → Rust STT Pipeline → Mel Spectrogram → CSV Export
```

### Step 2: Python Feature Extraction
```
Audio File → sherpa-onnx → Mel Spectrogram → CSV Export
```

### Step 3: Comparison
```
Rust CSV + Python CSV → Statistical Analysis → Diagnosis Report
```

## Output Files

All outputs are written to `/tmp/`:

- **rust_mel_features.csv**: Mel features from Rust implementation
- **python_mel_features.csv**: Mel features from Python implementation
- **mel_comparison_report.txt**: Detailed comparison analysis

## Interpretation Guide

### ✅ Features MATCH

**Meaning**: Mel feature extraction is identical between implementations

**Root Cause**: Bug is in the encoder/decoder stage

**Investigation Areas**:
1. ONNX Runtime tensor format
2. Encoder input/output shapes
3. CTC decoder beam search parameters
4. Token probability distributions
5. Language model integration

### ❌ Features DIFFER

**Meaning**: Mel feature extraction differs between implementations

**Root Cause**: Bug is in the audio preprocessing stage

**Investigation Areas**:
1. STFT window/hop length parameters
2. Mel filterbank configuration (num filters, frequency range)
3. Normalization methods (mean/variance normalization)
4. Audio preprocessing (resampling, padding, windowing)
5. Floating-point precision issues

## Quick Reference

### Running Full Diagnosis
```bash
cd /opt/swictation
./scripts/diagnose_feature_mismatch.sh examples/en-short.mp3
```

### Manual Component Testing

**Test Rust extraction only**:
```bash
./rust-crates/target/release/examples/export_mel_features \
    examples/en-short.mp3 /tmp/rust_test.csv
```

**Test Python extraction only**:
```bash
python3.12 scripts/extract_python_mel_features.py \
    examples/en-short.mp3 /tmp/python_test.csv
```

**Compare existing CSVs**:
```bash
python3.12 scripts/compare_mel_features.py \
    /tmp/rust_mel_features.csv \
    /tmp/python_mel_features.csv
```

### Inspecting Results

**View CSV files**:
```bash
less /tmp/rust_mel_features.csv
less /tmp/python_mel_features.csv
```

**Side-by-side comparison**:
```bash
diff -y /tmp/rust_mel_features.csv /tmp/python_mel_features.csv | less
```

**Statistical summary**:
```bash
cat /tmp/mel_comparison_report.txt
```

## Error Handling

The master script includes comprehensive error handling:

- **Missing audio file**: Exits with clear error message
- **Build failures**: Automatically attempts to build Rust project
- **Missing Python scripts**: Validates all dependencies before running
- **Empty outputs**: Validates CSV files have data
- **Extraction failures**: Captures and reports errors from subprocesses

## Dependencies

### System Requirements
- Rust toolchain (for building examples)
- Python 3.12
- bash shell with color support

### Rust Dependencies
- sherpa-rs crate
- Parakeet-TDT 1.1B model files

### Python Dependencies
- sherpa-onnx
- numpy

### Audio Files
- Test audio: `/opt/swictation/examples/en-short.mp3`
- Custom audio: Any format supported by FFmpeg

## Troubleshooting

### "Rust binary not found"
```bash
cd rust-crates
cargo build --release --example export_mel_features
```

### "Python script not found"
Ensure you're running from the project root:
```bash
cd /opt/swictation
./scripts/diagnose_feature_mismatch.sh
```

### "sherpa-onnx not installed"
```bash
pip install sherpa-onnx
```

### Colors not showing
Set terminal environment:
```bash
export TERM=xterm-256color
```

## Integration with Hive Mind

This diagnostic suite is part of the Hive Mind debugging workflow:

1. **Pre-task hook**: Registers task with coordination system
2. **Post-edit hook**: Stores results in shared memory
3. **Notify hook**: Broadcasts completion to other agents

**Coordination**:
```bash
npx claude-flow@alpha hooks pre-task --description "mel-diagnosis"
npx claude-flow@alpha hooks post-edit --memory-key "hive/diagnosis/mel-features"
npx claude-flow@alpha hooks notify --message "diagnosis-complete"
```

## Next Steps

After running the diagnosis:

1. Review the comparison report
2. If features match → Focus on encoder/decoder (see AHA #20)
3. If features differ → Focus on audio preprocessing
4. Share findings via memory coordination
5. Update task status in Archon

## References

- AHA #20: Comprehensive debugging investigation
- Parakeet-TDT 1.1B documentation
- sherpa-onnx feature extraction API
- Rust audio processing implementation
