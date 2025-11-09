# Swictation Pipeline Test Plan
**Version:** 1.0
**Date:** 2025-11-08
**Author:** TESTER Agent

## Executive Summary

This document outlines a comprehensive testing strategy for the Swictation audio-to-text pipeline, covering all 6 components from audio capture through text injection. The test suite ensures GPU acceleration verification, latency targets (<250ms), and correct end-to-end functionality.

---

## 1. Pipeline Architecture Overview

```
┌─────────────────┐
│ Audio Capture   │ → cpal + ringbuf (16kHz mono)
└────────┬────────┘
         ↓
┌─────────────────┐
│ VAD Detection   │ → Silero VAD v6 (ONNX/CUDA)
└────────┬────────┘
         ↓
┌─────────────────┐
│ STT             │ → Parakeet-TDT 1.1B (CUDA)
└────────┬────────┘
         ↓
┌─────────────────┐
│ Transform       │ → Midstream text transform
└────────┬────────┘
         ↓
┌─────────────────┐
│ Text Injection  │ → wtype/xdotool
└────────┬────────┘
         ↓
┌─────────────────┐
│ Metrics         │ → SQLite metrics DB
└─────────────────┘
```

**Key Characteristics:**
- Sample Rate: 16kHz
- Channels: Mono (1 channel)
- Chunk Size: 512-8000 samples (0.032s - 0.5s)
- Target Latency: <250ms total
- GPU Provider: CUDA (NVIDIA) via ONNX Runtime

---

## 2. Test Fixtures

### 2.1 Audio Test Files

**Primary Test File:**
- **File:** `/opt/swictation/examples/en-short.mp3`
- **Duration:** 6.17 seconds
- **Content:** "Hello world.\nTesting, one, two, three"
- **Properties:** 48kHz mono MP3 (resampled to 16kHz by pipeline)
- **Expected Output:** "Hello world. Testing, one, two, three"

**Additional Test Files:**
- `/opt/swictation/examples/en-long.mp3` - Extended test case
- `/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/test_wavs/en.wav` - Reference WAV

### 2.2 Test Environment Requirements

**System:**
- Linux (X11 or Wayland)
- NVIDIA GPU with CUDA 12.x or 13.x
- `nvidia-smi` available for GPU verification

**Dependencies:**
- ONNX Runtime with CUDA support
- parakeet-rs with CUDA features
- wtype (Wayland) or xdotool (X11)

**Models:**
- Silero VAD: `/opt/swictation/models/silero-vad/silero_vad.onnx`
- Parakeet-TDT: `/opt/swictation/models/nvidia-parakeet-tdt-1.1b`

---

## 3. Unit Tests (Component-Level)

### 3.1 Audio Capture Tests

**Test Suite:** `swictation-audio/tests/capture_test.rs`

| Test ID | Test Name | Description | Pass Criteria |
|---------|-----------|-------------|---------------|
| AU-001 | `test_audio_config_defaults` | Verify default config (16kHz, mono) | Config matches expected values |
| AU-002 | `test_buffer_creation` | Create circular buffer | Buffer size = sample_rate * duration |
| AU-003 | `test_resampler_48k_to_16k` | Resample 48kHz → 16kHz | Output rate = 16000 Hz |
| AU-004 | `test_chunk_callback` | Verify chunk callback fires | Callback invoked with 8000 samples |
| AU-005 | `test_audio_device_list` | Enumerate audio devices | Returns at least 1 device |
| AU-006 | `test_streaming_mode` | Enable streaming with callbacks | Chunks delivered in real-time |

**Implementation:**
```rust
#[test]
fn test_resampler_48k_to_16k() {
    let input_rate = 48000;
    let output_rate = 16000;
    let resampler = Resampler::new(input_rate, output_rate, 1).unwrap();

    let input: Vec<f32> = (0..4800).map(|i| (i as f32).sin()).collect();
    let output = resampler.resample(&input).unwrap();

    // Output should be ~1/3 the size (48k → 16k)
    assert_eq!(output.len(), input.len() / 3);
}
```

### 3.2 VAD Detection Tests

**Test Suite:** `swictation-vad/tests/vad_test.rs`

| Test ID | Test Name | Description | Pass Criteria |
|---------|-----------|-------------|---------------|
| VAD-001 | `test_vad_config_validation` | Validate config parameters | Rejects invalid threshold/sample rate |
| VAD-002 | `test_vad_silence_detection` | Process silent audio | Returns `VadResult::Silence` |
| VAD-003 | `test_vad_speech_detection` | Process speech audio from en-short.mp3 | Returns `VadResult::Speech` with samples |
| VAD-004 | `test_vad_threshold_sensitivity` | Test ONNX threshold (0.001-0.005) | Detects speech at 0.003 threshold |
| VAD-005 | `test_vad_gpu_provider` | Initialize with CUDA provider | No errors, uses GPU |
| VAD-006 | `test_vad_chunk_buffering` | Handle incomplete chunks | Buffers remainder for next call |
| VAD-007 | `test_vad_flush` | Flush remaining audio | Returns buffered speech segment |

**Critical Test - ONNX Threshold:**
```rust
#[test]
fn test_vad_onnx_threshold() {
    let config = VadConfig::with_model("/opt/swictation/models/silero-vad/silero_vad.onnx")
        .threshold(0.003)  // ONNX threshold (NOT 0.5!)
        .provider(Some("cuda".to_string()));

    let mut vad = VadDetector::new(config).unwrap();

    // Load en-short.mp3 samples (after resampling to 16kHz)
    let speech_samples = load_test_audio("en-short.mp3");

    let result = vad.process_audio(&speech_samples).unwrap();

    match result {
        VadResult::Speech { samples, .. } => {
            assert!(samples.len() > 0, "Should detect speech");
        }
        VadResult::Silence => {
            panic!("ONNX VAD failed to detect speech - check threshold!");
        }
    }
}
```

### 3.3 STT (Speech-to-Text) Tests

**Test Suite:** `swictation-stt/tests/parakeet_test.rs`

| Test ID | Test Name | Description | Pass Criteria |
|---------|-----------|-------------|---------------|
| STT-001 | `test_parakeet_model_load` | Load Parakeet-TDT model | Model loads without errors |
| STT-002 | `test_parakeet_cuda_provider` | Initialize with CUDA | Uses GPU execution |
| STT-003 | `test_parakeet_transcribe_en_short` | Transcribe en-short.mp3 | Output contains "hello world" and "testing" |
| STT-004 | `test_parakeet_latency` | Measure STT latency | <200ms for 6s audio |
| STT-005 | `test_parakeet_empty_audio` | Handle empty input | Returns empty string, no crash |
| STT-006 | `test_parakeet_16khz_requirement` | Verify sample rate | Rejects non-16kHz audio |

**Expected Output Test:**
```rust
#[test]
fn test_parakeet_transcribe_en_short() {
    let config = ExecutionConfig {
        execution_provider: ExecutionProvider::Cuda,
        intra_threads: 4,
        inter_threads: 1,
    };

    let stt = ParakeetTDT::from_pretrained(
        "/opt/swictation/models/nvidia-parakeet-tdt-1.1b",
        Some(config)
    ).unwrap();

    let samples = load_audio_16khz("en-short.mp3");
    let result = stt.transcribe_samples(samples, 16000, 1).unwrap();

    let text_lower = result.text.to_lowercase();
    assert!(text_lower.contains("hello world"), "Missing 'hello world'");
    assert!(text_lower.contains("testing"), "Missing 'testing'");
}
```

### 3.4 Text Transform Tests

**Test Suite:** `external/midstream/crates/text-transform/tests/transform_test.rs`

| Test ID | Test Name | Description | Pass Criteria |
|---------|-----------|-------------|---------------|
| TRF-001 | `test_basic_punctuation` | "comma" → "," | Correct symbol replacement |
| TRF-002 | `test_multi_word_patterns` | "question mark" → "?" | Multi-word patterns work |
| TRF-003 | `test_performance_target` | Transform 10-word input | <5ms average |
| TRF-004 | `test_voice_commands` | "Hello comma world period" | "Hello, world." |
| TRF-005 | `test_keyboard_shortcuts` | "backspace" → "<KEY:BackSpace>" | Key markers generated |
| TRF-006 | `test_quote_toggling` | Nested quotes | Opening/closing balanced |

### 3.5 Text Injection Tests

**Test Suite:** `swictation-daemon/tests/text_injection_test.rs`

| Test ID | Test Name | Description | Pass Criteria |
|---------|-----------|-------------|---------------|
| INJ-001 | `test_display_server_detection` | Detect X11/Wayland | Correct server identified |
| INJ-002 | `test_plain_text_injection` | Inject "Hello, world!" | wtype/xdotool succeeds |
| INJ-003 | `test_key_marker_parsing` | Parse "<KEY:ctrl-c>" | Key extracted correctly |
| INJ-004 | `test_key_combination_send` | Send super+Right | Key sent without errors |
| INJ-005 | `test_mixed_text_and_keys` | "Text <KEY:Enter> more text" | Both parts injected |

**Note:** These tests may require X11/Wayland session or should be marked `#[ignore]` for CI.

### 3.6 Metrics Tests

**Test Suite:** `swictation-metrics/tests/metrics_test.rs`

| Test ID | Test Name | Description | Pass Criteria |
|---------|-----------|-------------|---------------|
| MET-001 | `test_session_lifecycle` | Create → End session | Session ID assigned |
| MET-002 | `test_segment_metrics_recording` | Record segment with latencies | Data saved to DB |
| MET-003 | `test_gpu_monitoring` | Enable GPU monitoring | GPU stats collected |
| MET-004 | `test_latency_thresholds` | Record high latency | Warning triggered |
| MET-005 | `test_wpm_calculation` | Calculate WPM from segment | 40-300 WPM range |

---

## 4. Integration Tests (Component Pairs)

### 4.1 Audio → VAD Integration

**Test Suite:** `swictation-daemon/tests/audio_vad_integration_test.rs`

| Test ID | Test Name | Description | Pass Criteria |
|---------|-----------|-------------|---------------|
| INT-001 | `test_audio_vad_pipeline` | Audio capture → VAD | Speech segments detected |
| INT-002 | `test_vad_chunk_alignment` | VAD processes 512-sample chunks | No buffer underrun |
| INT-003 | `test_silence_filtering` | Silent audio ignored | Only speech passed through |

```rust
#[tokio::test]
async fn test_audio_vad_pipeline() {
    // Create audio capture with streaming
    let audio_config = AudioConfig {
        sample_rate: 16000,
        streaming_mode: true,
        chunk_duration: 0.5, // 8000 samples
        ..Default::default()
    };
    let audio = AudioCapture::new(audio_config).unwrap();

    // Create VAD with CUDA
    let vad_config = VadConfig::with_model("...")
        .provider(Some("cuda".to_string()));
    let mut vad = VadDetector::new(vad_config).unwrap();

    // Set up channel for audio → VAD
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Audio callback sends chunks to VAD
    audio.set_chunk_callback(move |chunk| {
        let _ = tx.send(chunk);
    });

    audio.start().unwrap();

    // Process chunks through VAD
    let mut speech_detected = false;
    while let Some(chunk) = rx.recv().await {
        if let VadResult::Speech { .. } = vad.process_audio(&chunk).unwrap() {
            speech_detected = true;
            break;
        }
    }

    assert!(speech_detected, "VAD should detect speech from audio");
}
```

### 4.2 VAD → STT Integration

**Test Suite:** `swictation-daemon/tests/vad_stt_integration_test.rs`

| Test ID | Test Name | Description | Pass Criteria |
|---------|-----------|-------------|---------------|
| INT-004 | `test_vad_stt_pipeline` | VAD segments → STT | Transcription produced |
| INT-005 | `test_speech_segment_buffering` | VAD buffers complete utterances | STT receives full context |
| INT-006 | `test_concurrent_vad_stt` | Process multiple segments | No deadlocks |

### 4.3 STT → Transform Integration

**Test Suite:** `swictation-daemon/tests/stt_transform_integration_test.rs`

| Test ID | Test Name | Description | Pass Criteria |
|---------|-----------|-------------|---------------|
| INT-007 | `test_stt_transform_pipeline` | "hello comma world" → "hello, world" | Punctuation added |
| INT-008 | `test_transform_latency` | Transform time tracked | <5ms overhead |

### 4.4 Transform → Injection Integration

**Test Suite:** `swictation-daemon/tests/transform_injection_integration_test.rs`

| Test ID | Test Name | Description | Pass Criteria |
|---------|-----------|-------------|---------------|
| INT-009 | `test_transform_injection_pipeline` | Transformed text → keyboard | Text injected |
| INT-010 | `test_keyboard_actions` | "<KEY:BackSpace>" parsed and sent | Key executed |

---

## 5. End-to-End Tests (Full Pipeline)

### 5.1 E2E Test: en-short.mp3 → Text Output

**Test Suite:** `swictation-daemon/tests/e2e_pipeline_test.rs`

**Test ID:** E2E-001
**Test Name:** `test_full_pipeline_en_short`

**Steps:**
1. Initialize all components with CUDA
2. Load en-short.mp3 → Audio buffer
3. Process through Audio Capture → VAD → STT → Transform
4. Verify output text matches expected
5. Measure total latency

**Expected Results:**
```
Input:  en-short.mp3 (6.17s audio)
Output: "Hello world. Testing, one, two, three" (or similar)
Latency: <250ms total (VAD + STT + Transform)
GPU: CUDA used for VAD and STT
```

**Implementation:**
```rust
#[tokio::test]
#[ignore] // Requires GPU and audio file
async fn test_full_pipeline_en_short() {
    // Setup
    let config = DaemonConfig::default();
    let gpu_provider = Some("cuda".to_string());

    let (mut pipeline, mut rx) = Pipeline::new(config, gpu_provider)
        .await
        .expect("Failed to create pipeline");

    // Load test audio file
    let audio_samples = load_audio_mp3_to_16khz("examples/en-short.mp3");

    // Simulate audio capture by injecting samples
    pipeline.start_recording().await.unwrap();

    // Feed audio in 0.5s chunks (8000 samples)
    let start_time = Instant::now();
    for chunk in audio_samples.chunks(8000) {
        // Process chunk through pipeline
        // (In real implementation, audio callback does this)
    }

    // Collect transcription results
    let mut transcriptions = Vec::new();
    while let Ok(text) = rx.try_recv() {
        if let Ok(t) = text {
            transcriptions.push(t);
        }
    }

    pipeline.stop_recording().await.unwrap();

    let total_latency = start_time.elapsed().as_millis();
    let full_text = transcriptions.join(" ").to_lowercase();

    // Assertions
    assert!(full_text.contains("hello world"), "Missing 'hello world'");
    assert!(full_text.contains("testing"), "Missing 'testing'");
    assert!(total_latency < 250, "Latency too high: {}ms", total_latency);
}
```

### 5.2 E2E Test: Latency Breakdown

**Test ID:** E2E-002
**Test Name:** `test_pipeline_latency_breakdown`

**Metrics Tracked:**
- VAD latency: <10ms
- STT latency: <200ms
- Transform latency: <5ms
- Total latency: <250ms

```rust
#[test]
fn test_pipeline_latency_breakdown() {
    // Process en-short.mp3 and track each stage
    let metrics = process_with_metrics("en-short.mp3");

    assert!(metrics.vad_latency_ms < 10.0, "VAD too slow");
    assert!(metrics.stt_latency_ms < 200.0, "STT too slow");
    assert!(metrics.transform_latency_us < 5000.0, "Transform too slow");
    assert!(metrics.total_latency_ms < 250.0, "Total latency exceeds target");
}
```

### 5.3 E2E Test: GPU Verification

**Test ID:** E2E-003
**Test Name:** `test_gpu_acceleration_active`

**Steps:**
1. Run pipeline with GPU provider
2. Check nvidia-smi during execution
3. Verify GPU memory usage > 0
4. Compare latency with/without GPU

```rust
#[test]
#[ignore] // Requires NVIDIA GPU
fn test_gpu_acceleration_active() {
    // Run with CUDA
    let gpu_latency = run_pipeline_with_gpu("cuda");

    // Run with CPU (fallback)
    let cpu_latency = run_pipeline_with_gpu(None);

    // GPU should be faster (at least 2x for STT)
    assert!(gpu_latency < cpu_latency / 2,
            "GPU not providing speedup: GPU={}ms, CPU={}ms",
            gpu_latency, cpu_latency);

    // Check nvidia-smi shows GPU usage
    let output = Command::new("nvidia-smi")
        .arg("--query-gpu=memory.used")
        .arg("--format=csv,noheader,nounits")
        .output()
        .unwrap();

    let gpu_mem: f32 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .unwrap();

    assert!(gpu_mem > 0.0, "GPU memory not used - CUDA not active");
}
```

---

## 6. GPU Acceleration Verification

### 6.1 CUDA Detection Tests

| Test ID | Test Name | Description | Pass Criteria |
|---------|-----------|-------------|---------------|
| GPU-001 | `test_cuda_available` | Verify CUDA installation | nvidia-smi succeeds |
| GPU-002 | `test_onnx_cuda_provider` | ONNX Runtime with CUDA | Provider initialized |
| GPU-003 | `test_parakeet_cuda_provider` | Parakeet with CUDA | ExecutionProvider::Cuda used |
| GPU-004 | `test_gpu_memory_tracking` | Monitor GPU memory | Memory increases during inference |

### 6.2 Performance Comparison Tests

```rust
#[test]
fn test_gpu_vs_cpu_performance() {
    let test_audio = load_audio("en-short.mp3");

    // Measure GPU performance
    let gpu_start = Instant::now();
    let gpu_result = run_stt_with_provider(&test_audio, ExecutionProvider::Cuda);
    let gpu_time = gpu_start.elapsed().as_millis();

    // Measure CPU performance
    let cpu_start = Instant::now();
    let cpu_result = run_stt_with_provider(&test_audio, ExecutionProvider::Cpu);
    let cpu_time = cpu_start.elapsed().as_millis();

    // Verify results are identical
    assert_eq!(gpu_result.text, cpu_result.text, "GPU and CPU results differ");

    // GPU should be significantly faster
    let speedup = cpu_time as f64 / gpu_time as f64;
    assert!(speedup > 2.0, "GPU speedup too low: {:.2}x", speedup);

    println!("GPU: {}ms, CPU: {}ms, Speedup: {:.2}x", gpu_time, cpu_time, speedup);
}
```

---

## 7. Test Execution Sequence

### 7.1 Development Testing (Fast Feedback)

**Run Order:**
1. Unit tests (component-level) - `cargo test --lib`
2. Integration tests (pairs) - `cargo test --test integration_*`
3. E2E tests (full pipeline) - `cargo test --test e2e_* --ignored`

**Estimated Time:**
- Unit tests: ~30 seconds
- Integration tests: ~2 minutes
- E2E tests: ~5 minutes
- **Total: ~7-8 minutes**

### 7.2 CI/CD Pipeline Testing

**GitHub Actions Workflow:**
```yaml
name: Pipeline Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest-gpu  # Requires GPU runner

    steps:
      - uses: actions/checkout@v3

      - name: Install CUDA
        run: |
          # Install CUDA 12.x or 13.x

      - name: Build
        run: cargo build --release --features cuda

      - name: Unit Tests
        run: cargo test --lib

      - name: Integration Tests
        run: cargo test --test '*_integration_*'

      - name: E2E Tests
        run: cargo test --test 'e2e_*' --ignored

      - name: GPU Verification
        run: nvidia-smi
```

### 7.3 Manual Testing Checklist

**Pre-Release Validation:**
- [ ] Run full test suite with GPU
- [ ] Verify en-short.mp3 transcribes correctly
- [ ] Check latency breakdown (<250ms total)
- [ ] Test on X11 and Wayland
- [ ] Verify text injection works
- [ ] Check metrics DB recording
- [ ] Test with long audio (en-long.mp3)
- [ ] Stress test (100+ consecutive recordings)

---

## 8. Test Data Requirements

### 8.1 Audio Fixtures

**Location:** `/opt/swictation/examples/`

| File | Duration | Content | Purpose |
|------|----------|---------|---------|
| en-short.mp3 | 6.17s | "Hello world. Testing, one, two, three" | Primary E2E test |
| en-long.mp3 | ~30s+ | Extended speech | Stress testing |
| silence.wav | 5s | Silence | VAD negative test |
| noise.wav | 5s | White noise | False positive test |

**Test Audio Generator:**
```bash
# Generate silence test file
ffmpeg -f lavfi -i anullsrc=r=16000:cl=mono -t 5 -acodec pcm_s16le silence.wav

# Generate noise test file
ffmpeg -f lavfi -i anoisesrc=r=16000:a=0.1:c=mono -t 5 -acodec pcm_s16le noise.wav
```

### 8.2 Expected Outputs

**File:** `tests/fixtures/expected_outputs.json`
```json
{
  "en-short.mp3": {
    "text": "Hello world. Testing, one, two, three",
    "variations": [
      "hello world testing one two three",
      "Hello world testing 1 2 3"
    ],
    "min_words": 7,
    "max_latency_ms": 250
  }
}
```

---

## 9. Pass/Fail Criteria

### 9.1 Component-Level Criteria

| Component | Metric | Pass Threshold |
|-----------|--------|----------------|
| Audio Capture | Chunk delivery | >95% chunks delivered on time |
| VAD | Speech detection | >90% true positives, <5% false positives |
| STT | WER (Word Error Rate) | <10% for clean speech |
| Transform | Latency | <5ms for 10-word input |
| Injection | Success rate | 100% (no errors) |
| Metrics | Data integrity | 100% (all metrics saved) |

### 9.2 Integration-Level Criteria

| Integration | Metric | Pass Threshold |
|-------------|--------|----------------|
| Audio → VAD | Latency | <10ms |
| VAD → STT | Latency | <200ms |
| STT → Transform | Latency | <5ms |
| Transform → Injection | Latency | <50ms |

### 9.3 E2E Criteria

**Must Pass All:**
- ✅ en-short.mp3 transcribes with >80% accuracy
- ✅ Total latency <250ms
- ✅ GPU acceleration active (CUDA provider used)
- ✅ No crashes or hangs
- ✅ Metrics recorded correctly
- ✅ Text injection successful (if applicable)

---

## 10. Failure Analysis Procedures

### 10.1 Common Failure Scenarios

**Scenario 1: VAD Not Detecting Speech**
- **Symptom:** All chunks return `VadResult::Silence`
- **Cause:** ONNX threshold too high (using PyTorch 0.5 instead of ONNX 0.003)
- **Fix:** Set `threshold(0.003)` in VadConfig
- **Verification:** Check VAD probabilities in debug logs

**Scenario 2: STT Transcription Empty**
- **Symptom:** `result.text` is empty string
- **Cause:** Audio not reaching STT or model not loaded
- **Fix:** Verify VAD is passing speech segments to STT
- **Verification:** Add debug logging in pipeline.rs line 225

**Scenario 3: GPU Not Used**
- **Symptom:** Slow inference, nvidia-smi shows 0% GPU
- **Cause:** CUDA provider not initialized or ONNX Runtime built without CUDA
- **Fix:** Rebuild with `--features cuda` and verify CUDA installation
- **Verification:** Check `ExecutionProvider::Cuda` in logs

**Scenario 4: High Latency (>250ms)**
- **Symptom:** Total latency exceeds target
- **Cause:** CPU inference or blocking operations
- **Fix:** Enable GPU, check for mutex contention in pipeline
- **Verification:** Profile with `perf` or `cargo flamegraph`

### 10.2 Debugging Tools

```rust
// Enable VAD debug logging
let vad_config = VadConfig::with_model("...")
    .debug();  // Prints probabilities to stderr

// Enable pipeline debug logging
RUST_LOG=debug cargo test test_full_pipeline

// GPU profiling
nvidia-smi dmon -s u -d 1  # Monitor GPU utilization
```

---

## 11. Test Maintenance

### 11.1 Test Update Triggers

**Update tests when:**
- Model version changes (VAD or STT)
- Threshold values adjusted
- New features added (e.g., additional transformations)
- Performance targets changed
- New audio formats supported

### 11.2 Regression Test Suite

**Protected Tests (Must Always Pass):**
- E2E-001: Full pipeline with en-short.mp3
- GPU-003: CUDA provider initialization
- VAD-004: ONNX threshold sensitivity
- STT-003: Transcription accuracy

**Rationale:** These tests ensure core functionality remains intact across changes.

---

## 12. Test Coverage Targets

### 12.1 Code Coverage Goals

| Crate | Target Coverage | Critical Paths |
|-------|----------------|----------------|
| swictation-audio | 80% | Audio callback, resampler |
| swictation-vad | 85% | VAD processing, GPU init |
| swictation-stt | 75% | Model loading, transcription |
| swictation-daemon | 70% | Pipeline orchestration |
| text-transform | 90% | All transformation rules |
| text-injection | 60% | Key parsing (injection needs X11/Wayland) |

**Measurement:**
```bash
cargo tarpaulin --all-features --workspace --timeout 300 --out Html
```

### 12.2 Uncovered Areas (Acceptable Gaps)

- Hardware-specific code (GPU drivers)
- Platform-specific display server code (X11/Wayland)
- Error recovery paths that require hardware failures
- IPC socket communication (requires daemon running)

---

## 13. Performance Benchmarking

### 13.1 Benchmark Suite

**Location:** `benches/pipeline_benchmarks.rs`

```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark_vad_processing(c: &mut Criterion) {
    c.bench_function("vad_process_chunk", |b| {
        let mut vad = create_vad_with_cuda();
        let chunk = vec![0.0; 512];

        b.iter(|| {
            vad.process_audio(&chunk).unwrap()
        });
    });
}

fn benchmark_stt_transcription(c: &mut Criterion) {
    c.bench_function("stt_transcribe_6s", |b| {
        let stt = create_stt_with_cuda();
        let samples = load_audio("en-short.mp3");

        b.iter(|| {
            stt.transcribe_samples(samples.clone(), 16000, 1).unwrap()
        });
    });
}

criterion_group!(benches, benchmark_vad_processing, benchmark_stt_transcription);
criterion_main!(benches);
```

**Run Benchmarks:**
```bash
cargo bench --features cuda
```

### 13.2 Performance Regression Detection

**Baseline Performance (With GPU):**
- VAD: <5ms per 512-sample chunk
- STT: <150ms for 6s audio
- Transform: <1ms for 10 words
- Total: <200ms for typical utterance

**Alert Thresholds:**
- >20% slowdown triggers review
- >50% slowdown blocks release

---

## 14. Appendix

### 14.1 Test File Structure

```
rust-crates/
├── swictation-daemon/
│   ├── tests/
│   │   ├── orchestrator_test.rs          # Existing unit tests
│   │   ├── audio_vad_integration_test.rs # NEW
│   │   ├── vad_stt_integration_test.rs   # NEW
│   │   ├── e2e_pipeline_test.rs          # NEW
│   │   └── gpu_verification_test.rs      # NEW
│   └── benches/
│       └── pipeline_benchmarks.rs        # NEW
├── swictation-audio/
│   └── tests/
│       └── capture_test.rs               # NEW
├── swictation-vad/
│   └── tests/
│       └── vad_test.rs                   # Expand existing
└── swictation-stt/
    └── tests/
        └── parakeet_test.rs              # NEW

examples/
├── en-short.mp3                          # Existing
├── en-long.mp3                           # Existing
├── silence.wav                           # NEW
└── noise.wav                             # NEW

tests/
└── fixtures/
    └── expected_outputs.json             # NEW
```

### 14.2 CI/CD Integration

**Required GitHub Secrets:**
- `CUDA_VERSION`: "12.4.0" or "13.0.0"
- `NVIDIA_DRIVER_VERSION`: "550.54.15" (or latest)

**Self-Hosted GPU Runner Setup:**
```yaml
# .github/workflows/gpu-tests.yml
runs-on: [self-hosted, linux, gpu, cuda]

steps:
  - name: Verify GPU
    run: |
      nvidia-smi
      nvcc --version
```

### 14.3 Test Execution Scripts

**File:** `scripts/run_all_tests.sh`
```bash
#!/bin/bash
set -e

echo "=== Unit Tests ==="
cargo test --lib --all-features

echo "=== Integration Tests ==="
cargo test --test '*_integration_*' --all-features

echo "=== E2E Tests (Requires GPU) ==="
cargo test --test 'e2e_*' --all-features --ignored

echo "=== Benchmarks ==="
cargo bench --features cuda

echo "=== Coverage Report ==="
cargo tarpaulin --all-features --out Html

echo "✅ All tests passed!"
```

**Make executable:**
```bash
chmod +x scripts/run_all_tests.sh
```

---

## 15. Success Metrics Summary

**Test Suite Completeness:**
- ✅ 6 component-level test suites
- ✅ 4 integration test suites
- ✅ 3 E2E test scenarios
- ✅ 4 GPU verification tests
- ✅ Performance benchmarks

**Coverage:**
- ✅ All 6 pipeline components tested
- ✅ GPU acceleration verified
- ✅ Latency targets validated
- ✅ en-short.mp3 fixture used

**Quality Gates:**
- ✅ <250ms total latency
- ✅ >80% transcription accuracy
- ✅ CUDA provider active
- ✅ No crashes or deadlocks

---

**Document Status:** ✅ COMPLETE
**Review Required:** Architect, Coder, Researcher
**Implementation Priority:** HIGH (Required for v1.0 release)
