# Audio & VAD Quick Reference

**Last Updated:** 2025-11-08

## Critical Configuration

### Audio Capture (swictation-audio)
```rust
AudioConfig {
    sample_rate: 16000,        // Required for STT
    channels: 1,               // Mono
    blocksize: 1024,           // 64ms @ 16kHz
    buffer_duration: 10.0,     // 10 seconds = 640KB
    device_index: None,        // Auto-detect
    streaming_mode: false,     // Set true for real-time
    chunk_duration: 1.0,       // 1s chunks if streaming
}
```

### VAD (swictation-vad)
```rust
VadConfig {
    model_path: "models/silero_vad.onnx",
    threshold: 0.003,          // ⚠️ ONNX: 0.001-0.005 NOT 0.5!
    min_silence_duration: 0.5, // 500ms
    min_speech_duration: 0.25, // 250ms
    max_speech_duration: 30.0, // 30s
    sample_rate: 16000,        // Required
    window_size: 512,          // 32ms
    provider: None,            // None=auto, Some("cuda")
}
```

## Performance Specs

| Component | Metric | Value |
|-----------|--------|-------|
| **Audio Capture** | Callback Latency | <100μs |
| | Memory | ~800KB |
| | CPU Usage | 0.1% |
| **VAD (CUDA)** | Inference Latency | <10ms |
| | Memory | 20MB |
| | CPU Usage | 31% |
| **VAD (CPU)** | Inference Latency | ~15ms |
| | Memory | 20MB |
| | CPU Usage | 47% |
| **Combined** | RTF | <0.5 |

## ONNX Threshold Guide

🚨 **CRITICAL:** ONNX outputs 100-200x lower probabilities than PyTorch!

| Threshold | Use Case |
|-----------|----------|
| 0.005 | Conservative (fewer false positives) |
| **0.003** | **Balanced (DEFAULT)** |
| 0.001 | Sensitive (catch quiet speech) |

❌ **NEVER use 0.5 with ONNX** - it will never detect speech!

## Sample Rates & Conversions

**Supported Input:**
- Any sample rate (auto-resampled to 16kHz)
- Common: 48kHz (USB), 44.1kHz (CD), 16kHz (native)

**Output:**
- Always 16kHz mono f32
- Normalized to [-1.0, 1.0]

**Resampling Quality:**
- Algorithm: Sinc interpolation
- Window: BlackmanHarris2
- Chunk size: 100ms

## Buffer Sizes

**Audio:**
- Callback: 1024 samples (64ms)
- Circular: 160,000 samples (10s) = 640KB
- Resample: 4,800 samples (100ms @ 48kHz)

**VAD:**
- Window: 512 samples (32ms)
- Speech buffer: Up to 30s = 1.92MB
- RNN state: [2, 1, 128] = 2KB

## Data Flow

```
Microphone
  ↓ cpal (PipeWire/ALSA)
  ↓ i16 → f32 conversion (~10μs)
  ↓ Resample to 16kHz (~50μs)
  ↓ Multi-channel → mono (~5μs)
  ↓
CircularBuffer (lock-free, <1μs write)
  or
ChunkCallback (streaming mode)
  ↓
VadDetector::process_audio()
  ↓ Align to 512-sample windows
  ↓
SileroVadOrt::process()
  ↓ ONNX inference (<10ms CUDA, ~15ms CPU)
  ↓ State machine (triggered/silence)
  ↓ Duration filters (250ms speech, 500ms silence)
  ↓
VadResult::Speech { samples } or Silence
```

## Integration Pattern

```rust
// 1. Create VAD
let vad_config = VadConfig::with_model("models/silero_vad.onnx")
    .threshold(0.003)  // ONNX threshold!
    .min_silence(0.5)
    .min_speech(0.25);
let vad = Arc::new(Mutex::new(VadDetector::new(vad_config)?));

// 2. Create audio in streaming mode
let audio_config = AudioConfig {
    streaming_mode: true,
    chunk_duration: 0.5,
    ..Default::default()
};
let mut capture = AudioCapture::new(audio_config)?;

// 3. Connect callback
let vad_clone = Arc::clone(&vad);
capture.set_chunk_callback(move |chunk| {
    let mut vad = vad_clone.lock().unwrap();
    match vad.process_audio(&chunk) {
        Ok(VadResult::Speech { samples, .. }) => {
            // Send to STT
        }
        _ => {}
    }
});

// 4. Start
capture.start()?;
```

## Troubleshooting

| Problem | Likely Cause | Solution |
|---------|-------------|----------|
| No speech detected | Threshold too high | Use 0.001-0.005 |
| Too many false positives | Threshold too low | Increase to 0.005 |
| Speech cut off | min_silence too short | Increase to 0.8-1.0s |
| Buffer overflow | Processing too slow | Increase buffer_duration |
| Distorted audio | Sample format issue | Check i16 vs f32 |

## File Locations

**Audio:**
- `/opt/swictation/rust-crates/swictation-audio/src/lib.rs`
- `/opt/swictation/rust-crates/swictation-audio/src/capture.rs`
- `/opt/swictation/rust-crates/swictation-audio/src/buffer.rs`
- `/opt/swictation/rust-crates/swictation-audio/src/resampler.rs`

**VAD:**
- `/opt/swictation/rust-crates/swictation-vad/src/lib.rs`
- `/opt/swictation/rust-crates/swictation-vad/src/silero_ort.rs`
- `/opt/swictation/rust-crates/swictation-vad/ONNX_THRESHOLD_GUIDE.md` ⭐

## Key Dependencies

```toml
# Audio
cpal = "0.15"           # Cross-platform audio I/O
ringbuf = "0.4"         # Lock-free SPSC buffer
rubato = "0.15"         # High-quality resampling
parking_lot = "0.12"    # Fast mutex

# VAD
ort = "2.0.0-rc.10"     # ONNX Runtime with CUDA
ndarray = "0.16"        # N-dimensional arrays
```

## Test Commands

```bash
# List audio devices
cargo run --example list_devices

# Test live audio capture
cargo run --example test_live_audio [device_index]

# Verify VAD threshold
cargo run --example verify_threshold

# Run integration tests
cargo test --test integration_test
```

---

**For full details, see:** `/opt/swictation/docs/research/audio-vad-pipeline-analysis.md`
