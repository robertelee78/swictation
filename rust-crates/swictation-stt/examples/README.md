# Swictation-STT Examples

Example programs demonstrating the usage of swictation-stt library.

## export_mel_features

Export mel-spectrogram features to CSV format for comparison and analysis.

### Usage

```bash
cargo run --release --example export_mel_features -- <audio_file> <output_csv>
```

### Example

```bash
# Export features from an MP3 file
cargo run --release --example export_mel_features -- examples/en-short.mp3 rust_mel_features.csv

# Export features from a WAV file
cargo run --release --example export_mel_features -- audio.wav features.csv
```

### Output Format

The CSV file contains three columns:
- `frame`: Frame index (0-based)
- `feature_idx`: Mel feature index (0-127 for 0.6B model, 0-79 for 1.1B model)
- `value`: Normalized mel feature value

Example CSV:
```csv
frame,feature_idx,value
0,0,-6.8570166
0,1,-0.00017929077
0,2,-6.804999
...
614,127,-0.18564619
```

### Feature Statistics

The tool outputs statistics about the extracted features:

```
Feature statistics:
  Mean: -0.000001  (normalized to ~0)
  Std:  0.996076   (normalized to ~1)
  Min:  -7.506382
  Max:  4.324483
```

### Comparison with Python

Use the verification script to analyze the CSV output:

```bash
python3 scripts/verify_rust_csv.py rust_mel_features.csv
```

This will show:
- CSV structure and dimensions
- Value statistics
- Per-feature normalization verification
- Sample data from beginning and end

### Technical Details

The mel features are extracted using:
- **Sample Rate**: 16 kHz
- **FFT Size**: 512
- **Hop Length**: 160 samples (10ms)
- **Window**: 400 samples (25ms) with Povey window
- **Mel Bins**: 128 (0.6B model) or 80 (1.1B model)
- **Frequency Range**: 20-7600 Hz
- **Normalization**: Per-feature (across time) mean=0, std=1

The features are compatible with NVIDIA NeMo Parakeet-TDT models and match
the sherpa-onnx feature extraction pipeline.

### Performance

For a 6.17-second audio file (en-short.mp3):
- **Processing Time**: ~10ms (feature extraction)
- **Output Frames**: 615 frames
- **Total Data Points**: 78,720 (615 frames × 128 features)
- **CSV File Size**: ~1.4 MB

### API Usage

You can also use the export function directly in your code:

```rust
use swictation_stt::audio::AudioProcessor;

let mut processor = AudioProcessor::new()?;
let samples = processor.load_audio("audio.mp3")?;
let features = processor.extract_mel_features(&samples)?;
processor.export_features_csv(&features, "output.csv")?;
```

## Future Examples

- `recognize_audio` - Full speech recognition pipeline
- `stream_recognition` - Real-time streaming recognition
- `benchmark_features` - Performance benchmarking
