# Python Mel Feature Extractor

## Overview

`extract_python_mel_features.py` extracts mel-filterbank features using torchaudio's Kaldi-compatible fbank implementation. This provides a reference implementation for comparison with the Rust audio processing pipeline.

## Features

- **Kaldi-compatible**: Uses `torchaudio.compliance.kaldi.fbank()` which matches the C++ implementation used by sherpa-onnx
- **NeMo-aligned**: Settings configured to match NeMo Parakeet-TDT model expectations
- **CSV Export**: Outputs features in the same format as Rust implementation for easy comparison

## Usage

```bash
python3.12 scripts/extract_python_mel_features.py <audio_file> <output_csv>
```

### Example

```bash
python3.12 scripts/extract_python_mel_features.py examples/en-short.mp3 python_mel_features.csv
```

## Configuration

The script uses the following Kaldi-compatible settings:

- **num_mel_bins**: 80 (matches N_MEL_FEATURES in Rust)
- **sample_frequency**: 16000 Hz
- **dither**: 0.0 (no dithering)
- **snip_edges**: False
- **low_freq**: 20.0 Hz
- **high_freq**: 7600.0 Hz (Nyquist - 400 = 8000 - 400)
- **preemphasis_coefficient**: 0.97
- **frame_length**: 25.0 ms
- **frame_shift**: 10.0 ms (80% overlap)
- **use_energy**: False (use log mel filterbank)

## Output Format

CSV format with three columns:
```
frame,feature_idx,value
0,0,-15.942385
0,1,-15.942385
...
```

## Dependencies

- Python 3.12
- torch
- torchaudio
- numpy
- soundfile

## Testing Results

For `examples/en-short.mp3`:
- Input: 294912 samples @ 48000 Hz
- Resampled: 98304 samples @ 16000 Hz
- Output: 614 frames × 80 mel bins = 49120 data points
- Statistics:
  - Mean: -8.18
  - Std: 3.16
  - Min: -15.94
  - Max: 5.83

## Comparison with Rust

To compare Python vs Rust mel features:

1. Extract Rust features:
   ```bash
   cargo run --example export_mel_features -- examples/en-short.mp3 rust_mel_features.csv
   ```

2. Extract Python features:
   ```bash
   python3.12 scripts/extract_python_mel_features.py examples/en-short.mp3 python_mel_features.csv
   ```

3. Compare:
   ```bash
   python3.12 scripts/compare_mel_features.py rust_mel_features.csv python_mel_features.csv
   ```

## Notes

- The script automatically resamples audio to 16 kHz if needed
- Stereo audio is automatically converted to mono
- torchaudio's Kaldi implementation is used instead of the sherpa-onnx Python bindings because the latter doesn't expose feature extraction directly
