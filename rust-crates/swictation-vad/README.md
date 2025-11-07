# swictation-vad

Voice Activity Detection (VAD) for Swictation using Silero VAD via sherpa-rs.

## Features

- ✅ **Pure Rust** - Zero Python dependency, uses sherpa-rs (official sherpa-onnx bindings)
- ✅ **20MB memory** - 96% reduction from 500MB+ PyTorch runtime
- ✅ **<10ms latency** - 5x faster than PyTorch implementation (~50ms)
- ✅ **475-512x real-time** - Extremely fast processing
- ✅ **Battle-tested** - Uses Silero VAD, production-proven model
- ✅ **CPU-only** - ONNX Runtime optimizations, no GPU required

## Performance Comparison

| Metric | PyTorch (Old) | ONNX Rust (New) | Improvement |
|--------|---------------|-----------------|-------------|
| Memory | 500MB+ | 20MB | **96% reduction** |
| Latency | ~50ms | <10ms | **5x faster** |
| Real-time factor | ~10x | 475-512x | **48-51x faster** |
| Dependencies | Python + PyTorch | Pure Rust | **Zero Python** |

## Test Results

### Short Sample (6.14s audio)
- **1 speech segment** detected (1.84s)
- **12ms processing time** (512x real-time)

### Long Sample (84.07s audio)
- **4 speech segments** detected (81.05s total speech)
- **177ms processing time** (475x real-time)
- Correctly filtered 3s of silence

## Usage

```rust
use swictation_vad::{VadConfig, VadDetector, VadResult};

// Configure VAD
let config = VadConfig::with_model("/path/to/silero_vad.onnx")
    .min_silence(0.5)
    .min_speech(0.25)
    .threshold(0.5);

// Create detector
let mut vad = VadDetector::new(config)?;

// Process audio chunks (16kHz, mono, f32)
let chunk: Vec<f32> = vec![0.0; 8000]; // 0.5 seconds
match vad.process_audio(&chunk)? {
    VadResult::Speech { start_sample, samples } => {
        println!("Speech: {} samples at {}", samples.len(), start_sample);
        // Send to STT
    }
    VadResult::Silence => {
        // Skip processing
    }
}
```

## Configuration

```rust
VadConfig::with_model("silero_vad.onnx")
    .min_silence(0.5)      // Minimum silence duration (seconds)
    .min_speech(0.25)      // Minimum speech duration (seconds)
    .max_speech(30.0)      // Maximum speech segment (seconds)
    .threshold(0.5)        // Speech probability threshold (0.0-1.0)
    .buffer_size(60.0)     // Audio buffer size (seconds)
    .debug()               // Enable debug logging
```

### Parameters

- **min_silence_duration**: Silence shorter than this is ignored (default: 0.5s)
- **min_speech_duration**: Speech shorter than this is filtered (default: 0.25s)
- **max_speech_duration**: Segments longer are split (default: 30.0s)
- **threshold**: Speech probability 0.0-1.0, higher = stricter (default: 0.5)

## Model

Download Silero VAD ONNX model (2.3MB):
```bash
wget https://github.com/snakers4/silero-vad/raw/master/src/silero_vad/data/silero_vad.onnx
```

## Examples

Run the examples:
```bash
# Basic test with synthetic audio
cargo run --release --example test_vad_basic

# Real audio test (requires test audio files)
cargo run --release --example test_vad_realfile
```

## Integration with STT Pipeline

```rust
use swictation_vad::{VadDetector, VadConfig, VadResult};
use swictation_stt::Recognizer;

let mut vad = VadDetector::new(VadConfig::default())?;
let mut stt = Recognizer::new("model_path")?;

// Audio stream callback
audio.on_chunk(|chunk: &[f32]| {
    match vad.process_audio(chunk) {
        Ok(VadResult::Speech { samples, .. }) => {
            // Only transcribe speech segments
            if let Ok(result) = stt.recognize(&samples) {
                println!("Transcription: {}", result.text);
            }
        }
        Ok(VadResult::Silence) => {
            // Skip silence, save 95%+ of STT processing
        }
        Err(e) => eprintln!("VAD error: {}", e),
    }
});
```

## Requirements

- Rust 1.70+
- Audio must be:
  - **16kHz sample rate** (required by Silero VAD)
  - **Mono** (single channel)
  - **f32 samples** normalized to [-1.0, 1.0]

## License

Apache-2.0
