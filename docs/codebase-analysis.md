# Swictation Rust Codebase Analysis

**Analysis Date:** 2025-11-11
**Researcher:** Hive Mind Research Agent
**Project:** Swictation Voice-to-Text Pipeline

---

## Executive Summary

Swictation is a Pure Rust voice-to-text dictation system with adaptive model selection, real-time audio processing, and GPU acceleration. The codebase consists of 6 workspace crates implementing a complete Audio → VAD → STT → Transform → Inject pipeline with comprehensive metrics tracking and real-time UI updates.

**Key Architectural Highlights:**
- **Adaptive STT Model Selection**: Automatically selects optimal model (0.6B CPU/GPU or 1.1B GPU INT8) based on available VRAM
- **Zero-Copy Audio Pipeline**: Lock-free circular buffers with sub-100μs latency
- **Unified STT Engine**: Abstract interface supporting multiple model implementations (sherpa-rs and direct ONNX Runtime)
- **Modern ONNX Runtime Integration**: Direct `ort` crate usage for GPU acceleration with CUDA 12.x support
- **Silero VAD v6**: ONNX Runtime-based VAD with LSTM state management and proper tensor handling
- **Real-Time Metrics Broadcasting**: Unix socket-based metrics streaming to UI clients

---

## 1. Workspace Structure

### 1.1 Cargo Workspace Layout

```toml
[workspace]
members = [
    "swictation-audio",     # Audio capture with cpal + ringbuf
    "swictation-stt",       # Unified STT engine (sherpa-rs + ort)
    "swictation-vad",       # Silero VAD v6 with ONNX Runtime
    "swictation-daemon",    # Main orchestration daemon
    "swictation-metrics",   # SQLite-based metrics collection + WASM
    "swictation-broadcaster", # Unix socket metrics broadcasting
]
resolver = "2"
```

**Shared Dependencies (workspace.dependencies):**
- `serde` v1.0 with `derive` feature
- `serde_json` v1.0
- `thiserror` v2.0 for error handling
- `anyhow` v1.0 for context-aware errors

---

## 2. Component Deep Dive

### 2.1 swictation-daemon (Orchestration)

**Path:** `rust-crates/swictation-daemon/`

**Purpose:** Main daemon process that coordinates all pipeline components

**Key Files:**
- `src/main.rs` - Entry point, event loop, hotkey handling, IPC server
- `src/pipeline.rs` - **CRITICAL**: Audio → VAD → STT → Transform pipeline
- `src/gpu.rs` - GPU detection and VRAM measurement
- `src/config.rs` - TOML configuration loading
- `src/hotkey.rs` - Cross-platform hotkey registration (X11/Wayland/macOS)
- `src/text_injection.rs` - Display server-agnostic text injection (xdotool/wtype)
- `src/ipc.rs` - Unix socket IPC for CLI control

**Dependencies:**
- **Internal**: All 5 other workspace crates
- **External**:
  - `midstreamer-text-transform` - Voice command → symbol transformation
  - `tokio` v1.0 with `full` features
  - `global-hotkey` v0.6 - Cross-platform hotkey capture
  - `swayipc` v3.0 - Wayland/Sway compositor integration
  - `sysinfo` v0.32 - System monitoring

**Architecture Patterns:**
- **Arc + Mutex** for shared state across threads
- **mpsc channels** for async audio chunk delivery (cpal callback → VAD/STT processing)
- **tokio::spawn** for background tasks (metrics updater, memory monitor, transcription injection)
- **select!** macro for event multiplexing (hotkeys, IPC, shutdown signals)

---

### 2.2 swictation-stt (Speech-to-Text Engine)

**Path:** `rust-crates/swictation-stt/`

**Purpose:** Unified STT interface supporting multiple Parakeet-TDT model implementations

**Key Files:**
- `src/engine.rs` - **CRITICAL**: `SttEngine` enum (unified interface)
- `src/recognizer.rs` - 0.6B model via sherpa-rs (CPU/GPU)
- `src/recognizer_ort.rs` - 1.1B INT8 model via direct ONNX Runtime
- `src/audio.rs` - Mel-spectrogram feature extraction (FFT, log-mel filterbanks)
- `src/lib.rs` - Public API exports
- `src/error.rs` - Error types

**Dependencies:**
- **sherpa-rs** (git version) - Sherpa-ONNX bindings with CUDA support
  - Features: `cuda` enabled
  - Backend: CUDA execution provider for 0.6B model
- **ort** v2.0.0-rc.8 - Direct ONNX Runtime for 1.1B model
  - Features: `ndarray`, `half`, `load-dynamic`
  - Reason: Bypasses sherpa-rs external weights bug
- **rustfft** v6.2 - FFT computation for mel-spectrograms
- **ndarray** v0.16 - Multi-dimensional array operations

**Model Support Matrix:**

| Model | Implementation | Quantization | VRAM Requirement | Latency | WER |
|-------|---------------|--------------|-----------------|---------|-----|
| Parakeet-TDT 0.6B | sherpa-rs | FP16/FP32/INT8 | 1.5GB (GPU) / 0 (CPU) | 100-400ms | 7-8% |
| Parakeet-TDT 1.1B | ort (direct) | INT8 | 4GB (GPU only) | 150-250ms | 5.77% |

**SttEngine Enum (Unified Interface):**

```rust
pub enum SttEngine {
    Parakeet0_6B(Recognizer),      // sherpa-rs backend
    Parakeet1_1B(OrtRecognizer),   // Direct ONNX Runtime
}

impl SttEngine {
    pub fn recognize(&mut self, audio: &[f32]) -> Result<RecognitionResult>;
    pub fn model_name(&self) -> &str;
    pub fn model_size(&self) -> &str;
    pub fn backend(&self) -> &str;      // "CPU" or "GPU"
    pub fn vram_required_mb(&self) -> u64;  // 0, 1536, or 4096
}
```

**Adaptive Model Selection Logic (pipeline.rs:78-218):**

```rust
// Decision tree for model selection:
if config.stt_model_override != "auto" {
    // MANUAL OVERRIDE: "0.6b-cpu", "0.6b-gpu", "1.1b-gpu"
    match config.stt_model_override.as_str() {
        "1.1b-gpu" => SttEngine::Parakeet1_1B(OrtRecognizer::new(...)?),
        "0.6b-gpu" => SttEngine::Parakeet0_6B(Recognizer::new(..., true)?),
        "0.6b-cpu" => SttEngine::Parakeet0_6B(Recognizer::new(..., false)?),
    }
} else {
    // AUTO MODE: VRAM-based selection
    let vram_mb = get_gpu_memory_mb().map(|(total, _)| total);

    if vram >= 4096 {
        // High VRAM: 1.1B INT8 (best quality)
        SttEngine::Parakeet1_1B(OrtRecognizer::new(..., true)?)
    } else if vram >= 1536 {
        // Moderate VRAM: 0.6B GPU
        SttEngine::Parakeet0_6B(Recognizer::new(..., true)?)
    } else {
        // Low/No VRAM: 0.6B CPU fallback
        SttEngine::Parakeet0_6B(Recognizer::new(..., false)?)
    }
}
```

**Key Design Decisions:**
1. **Why two implementations?** sherpa-rs has external weights loading bug for 1.1B model, so direct `ort` is used as workaround
2. **Why unified enum?** Allows hot-swapping models at runtime without changing pipeline code
3. **Why INT8 quantization?** Reduces VRAM from 6GB to 3.5GB while maintaining 5.77% WER

---

### 2.3 swictation-vad (Voice Activity Detection)

**Path:** `rust-crates/swictation-vad/`

**Purpose:** Silero VAD v6 implementation with ONNX Runtime and LSTM state management

**Key Files:**
- `src/lib.rs` - Public API (`VadDetector`, `VadConfig`, `VadResult`)
- `src/silero_ort.rs` - **CRITICAL**: Direct ONNX Runtime VAD with LSTM states
- `src/error.rs` - Error types

**Dependencies:**
- **ort** v2.0.0-rc.10 with `cuda` and `half` features
- **ndarray** v0.16 - Multi-dimensional arrays for LSTM states
- **rubato** v0.15 - Audio resampling (for non-16kHz input)

**Silero VAD v6 Architecture:**

```rust
pub struct SileroVadOrt {
    session: Arc<Mutex<Session>>,   // ONNX Runtime session

    // LSTM states (Silero VAD v6 uses 2-layer LSTM)
    h_state: Array3<f32>,  // Hidden state [2, 1, 64]
    c_state: Array3<f32>,  // Cell state [2, 1, 64]

    // Detection state machine
    triggered: bool,       // Currently in speech segment
    temp_end: usize,       // Potential end of speech segment
    current_sample: usize, // Sample counter

    // Speech buffering
    speech_buffer: Vec<f32>,  // Accumulates detected speech

    // Configuration
    threshold: f32,        // 0.003 for ONNX (NOT 0.5 like PyTorch!)
    min_speech_samples: usize,
    min_silence_samples: usize,
}
```

**CRITICAL VAD Findings:**

1. **ONNX Threshold Calibration (lib.rs:14-22):**
   - **PyTorch JIT**: probabilities ~0.02-0.2, use threshold ~0.5
   - **ONNX model**: probabilities ~0.0005-0.002, **use threshold ~0.001-0.005**
   - Default: **0.003** (optimal for ONNX)
   - This is NOT a bug - verified identical with Python onnxruntime

2. **Tensor Input Requirements (silero_ort.rs:139-140):**
   ```rust
   // CRITICAL: Must use C-contiguous layout (standard layout)
   let input_array = Array2::from_shape_vec((1, audio_chunk.len()), audio_chunk.to_vec())
       .map_err(...)?;
   // NOT Fortran layout! This breaks ONNX Runtime CUDA EP
   ```

3. **LSTM State Persistence (silero_ort.rs:23-26):**
   - Maintains 2 separate LSTM states (h and c)
   - Shape: `[2 layers, 1 batch, 64 hidden units]`
   - Updated after each inference for streaming continuity

**VAD State Machine (lib.rs:56-65):**

```rust
pub enum VadResult {
    Speech {
        start_sample: i32,
        samples: Vec<f32>,  // Complete speech segment
    },
    Silence,  // No speech detected
}
```

**Flush Integration (pipeline.rs:461-559):**
- VAD buffers audio until silence detected
- **Critical**: `vad.flush()` called on recording stop to process any buffered audio
- Ensures no speech is lost at segment boundaries

---

### 2.4 swictation-audio (Audio Capture)

**Path:** `rust-crates/swictation-audio/`

**Purpose:** High-performance zero-copy audio capture with real-time resampling

**Key Files:**
- `src/lib.rs` - Public API and `AudioConfig`
- `src/capture.rs` - **CRITICAL**: cpal-based audio capture with streaming callbacks
- `src/buffer.rs` - Lock-free circular buffer (ringbuf)
- `src/resampler.rs` - Real-time resampling with rubato

**Dependencies:**
- **cpal** v0.15 - Cross-platform audio I/O (PipeWire/ALSA/WASAPI)
- **ringbuf** v0.4 - Lock-free circular buffer
- **rubato** v0.15 - High-quality audio resampling
- **parking_lot** v0.12 - Fast mutex implementation

**Audio Pipeline Architecture:**

```text
Audio Device (Microphone/Loopback)
    ↓ cpal callback (audio thread)
CircularBuffer (lock-free ringbuf)
    ↓
Resampler (rubato) → 16kHz mono
    ↓
Chunk Callback (streaming mode)
    ↓ mpsc channel
VAD/STT Processing (tokio thread)
```

**Key Features:**

1. **Zero-Copy Design:**
   - Audio samples written directly to ringbuf by cpal callback
   - No heap allocations in audio thread
   - Sub-100μs callback latency

2. **Streaming Mode (capture.rs:78-84):**
   ```rust
   pub fn set_chunk_callback<F>(&mut self, callback: F)
   where F: Fn(Vec<f32>) + Send + Sync + 'static
   {
       self.chunk_callback = Some(Arc::new(callback));
   }
   ```
   - Callback invoked when `chunk_duration` seconds accumulated
   - Used by pipeline.rs to feed VAD in real-time

3. **Resampling Strategy:**
   - Resample to 16kHz mono immediately after capture
   - Accumulates input samples until resample ratio satisfied
   - Uses rubato's `FftFixedIn` resampler for quality

**Pipeline Integration (pipeline.rs:283-298):**

```rust
// Set up audio callback to push chunks via channel
let (audio_tx, mut audio_rx) = mpsc::unbounded_channel::<Vec<f32>>();

audio.set_chunk_callback(move |chunk| {
    // Runs in cpal's audio thread (non-blocking!)
    let _ = audio_tx.clone().send(chunk);
});

audio.start()?;  // Start cpal stream

// Spawn VAD/STT processing thread
tokio::spawn(async move {
    while let Some(chunk) = audio_rx.recv().await {
        // Process through VAD → STT in tokio thread
    }
});
```

---

### 2.5 swictation-metrics (Metrics Collection)

**Path:** `rust-crates/swictation-metrics/`

**Purpose:** SQLite-based metrics tracking with WASM support and GPU monitoring

**Key Files:**
- `src/lib.rs` - Public API (`MetricsCollector`)
- `src/database.rs` - SQLite schema and queries
- `src/models.rs` - Data models (`SessionMetrics`, `SegmentMetrics`)
- `src/collector.rs` - Metrics collection logic
- `src/gpu.rs` - GPU memory monitoring (NVML/Metal)
- `src/memory.rs` - RAM/VRAM monitoring with pressure detection
- `src/wasm.rs` - WASM bindings for browser/Node.js

**Dependencies:**
- **rusqlite** v0.32 with `bundled` feature (embedded SQLite)
- **serde** + **serde_json** for serialization
- **chrono** v0.4 with `serde` for timestamps
- **nvml-wrapper** v0.10 (Linux/Windows, optional)
- **metal** v0.29 (macOS, optional)
- **wasm-bindgen** (optional, for WASM target)

**Database Schema:**

```sql
CREATE TABLE sessions (
    session_id INTEGER PRIMARY KEY,
    start_time TEXT,
    end_time TEXT,
    duration_s REAL,
    words_dictated INTEGER,
    characters_dictated INTEGER,
    words_per_minute REAL,
    avg_latency_ms REAL,
    ...
);

CREATE TABLE segments (
    segment_id INTEGER PRIMARY KEY,
    session_id INTEGER REFERENCES sessions,
    timestamp TEXT,
    duration_s REAL,
    words INTEGER,
    characters INTEGER,
    text TEXT,  -- NULL if store_transcription_text=false (privacy)
    vad_latency_ms REAL,
    stt_latency_ms REAL,
    transform_latency_us REAL,
    injection_latency_ms REAL,
    total_latency_ms REAL,
    ...
);
```

**Metrics Collection Flow:**

```rust
// 1. Start session
let session_id = metrics.start_session()?;

// 2. Add segments during recording
metrics.add_segment(SegmentMetrics {
    session_id: Some(session_id),
    duration_s: 1.5,
    words: 12,
    text: "transcribed text".to_string(),  // Stored if store_text=true
    vad_latency_ms: 5.2,
    stt_latency_ms: 150.3,
    ...
})?;

// 3. End session (auto-calculates aggregates)
let session_metrics = metrics.end_session()?;
println!("WPM: {}", session_metrics.words_per_minute);
```

**GPU Monitoring (gpu.rs):**

```rust
// Detect GPU and get VRAM usage
let memory = metrics.get_gpu_memory()?;
println!("GPU: {} - {}/{} MB used",
         memory.device_name,
         memory.used_mb,
         memory.total_mb);

// Pressure detection (memory.rs)
let (ram_pressure, vram_pressure) = memory_monitor.check_pressure();
match vram_pressure {
    MemoryPressure::Warning => warn!("VRAM >80%"),
    MemoryPressure::Critical => error!("VRAM >90%"),
    MemoryPressure::Normal => {}
}
```

**WASM Integration:**
- Crate can be compiled to WASM with `--target wasm32-unknown-unknown`
- Feature: `wasm` enables `wasm-bindgen` and `js-sys`
- Allows metrics visualization in browser or Electron app

---

### 2.6 swictation-broadcaster (Real-Time Updates)

**Path:** `rust-crates/swictation-broadcaster/`

**Purpose:** Unix socket server for streaming metrics to UI clients

**Key Files:**
- `src/lib.rs` - Public API (`MetricsBroadcaster`)
- `src/broadcaster.rs` - Unix socket server implementation
- `src/events.rs` - Event types (`MetricsUpdate`, `TranscriptionEvent`)
- `src/client.rs` - Client connection management

**Dependencies:**
- **tokio** v1.0 with `net`, `sync`, `rt-multi-thread` features
- **serde** + **serde_json** for event serialization
- **swictation-metrics** for data models

**Event Types:**

```rust
pub enum BroadcastEvent {
    // Real-time metrics (1Hz)
    MetricsUpdate {
        cpu_percent: f32,
        ram_mb: u64,
        vram_mb: Option<u64>,
        current_state: DaemonState,  // Idle or Recording
        session_id: Option<i64>,
    },

    // Transcription results (as they occur)
    Transcription {
        session_id: i64,
        text: String,
        wpm: f64,
        latency_ms: f64,
        word_count: i32,
    },

    // Session lifecycle
    SessionStart { session_id: i64 },
    SessionEnd { session_id: i64 },

    // State changes
    StateChange { new_state: DaemonState },
}
```

**Broadcasting Pattern (broadcaster.rs):**

```rust
// Daemon creates broadcaster
let broadcaster = Arc::new(
    MetricsBroadcaster::new("/tmp/swictation_metrics.sock").await?
);

// Start Unix socket server
broadcaster.start().await?;

// Broadcast events (non-blocking)
broadcaster.update_metrics(&realtime_metrics).await;
broadcaster.add_transcription(text, wpm, latency, word_count).await;
```

**Client Integration:**
- UI clients connect to `/tmp/swictation_metrics.sock`
- Receive JSON-encoded events over Unix socket
- Can be TypeScript/Python/any language with Unix socket support

---

## 3. Data Flow Analysis

### 3.1 Complete Pipeline Flow (pipeline.rs)

```text
┌─────────────────────────────────────────────────────────────────┐
│ 1. AUDIO CAPTURE (cpal callback, audio thread)                 │
│    Microphone/Loopback → cpal → Resampler → 16kHz mono         │
└─────────────────────────────┬───────────────────────────────────┘
                              │ mpsc::unbounded_channel
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ 2. VAD PROCESSING (tokio thread)                                │
│    • Buffer 0.5s chunks (8000 samples @ 16kHz)                 │
│    • Silero VAD v6 inference (LSTM states maintained)          │
│    • Accumulate speech segments                                │
│    • Detect silence → emit Speech{samples} or Silence          │
└─────────────────────────────┬───────────────────────────────────┘
                              │ VadResult::Speech
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ 3. STT PROCESSING (same tokio thread)                           │
│    • Select model based on VRAM (1.1B GPU / 0.6B GPU/CPU)     │
│    • Mel-spectrogram extraction (80 or 128 features)           │
│    • Encoder → Decoder → Joiner (transducer architecture)      │
│    • Beam search decoding → text transcription                 │
└─────────────────────────────┬───────────────────────────────────┘
                              │ RecognitionResult{text}
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ 4. TEXT TRANSFORMATION (midstreamer-text-transform)             │
│    • Voice commands → symbols/punctuation                       │
│    • "comma" → ","  "period" → "."  "new line" → "\n"         │
│    • Capitalization rules                                       │
└─────────────────────────────┬───────────────────────────────────┘
                              │ String (transformed)
                              ├───────────────────┐
                              ↓                   ↓
┌─────────────────────────────────────┐  ┌──────────────────────┐
│ 5. METRICS RECORDING                │  │ 6. TEXT INJECTION    │
│    • SegmentMetrics to SQLite       │  │    • xdotool (X11)   │
│    • Broadcast to UI clients        │  │    • wtype (Wayland) │
└─────────────────────────────────────┘  └──────────────────────┘
```

### 3.2 Timing Breakdown (pipeline.rs:344-430)

```rust
// Segment timing tracked for metrics:
let segment_start = Instant::now();

// 1. VAD detection (~5ms)
let vad_result = vad.process_audio(&chunk)?;
let vad_latency = segment_start.elapsed().as_millis() as f64;

// 2. STT transcription (100-250ms depending on model)
let stt_start = Instant::now();
let result = stt.recognize(&speech_samples)?;
let stt_latency = stt_start.elapsed().as_millis() as f64;

// 3. Text transformation (<1ms)
let transform_start = Instant::now();
let transformed = transform(&text);
let transform_latency = transform_start.elapsed().as_micros() as f64;

// 4. Total pipeline latency
let total_latency_ms = vad_latency + stt_latency + (transform_latency / 1000.0);
```

**Typical Latencies:**
- **VAD**: 5-10ms (ONNX Runtime, GPU accelerated)
- **STT 0.6B GPU**: 100-150ms (peak 1.2GB VRAM)
- **STT 1.1B INT8 GPU**: 150-250ms (peak 3.5GB VRAM)
- **STT 0.6B CPU**: 200-400ms (fallback)
- **Transform**: <1ms (string operations)
- **Total (GPU)**: **105-260ms** end-to-end

---

## 4. GPU Detection & Adaptive Selection

### 4.1 GPU Detection (daemon/src/gpu.rs)

```rust
pub fn detect_gpu_provider() -> Option<String> {
    // Try nvidia-smi first (CUDA/NVIDIA)
    if Command::new("nvidia-smi").output().is_ok() {
        return Some("cuda".to_string());
    }

    // TODO: Add Metal detection for macOS
    // TODO: Add ROCm detection for AMD

    None
}

pub fn get_gpu_memory_mb() -> Option<(u64, u64)> {
    // Parse nvidia-smi output for total/free VRAM
    let output = Command::new("nvidia-smi")
        .args(["--query-gpu=memory.total,memory.free", "--format=csv,noheader,nounits"])
        .output()
        .ok()?;

    // Returns (total_mb, free_mb)
}
```

### 4.2 Model Selection Decision Tree (pipeline.rs:78-218)

```rust
// DECISION TREE:
//
// CONFIG OVERRIDE?
//   YES → Use specified model (0.6b-cpu, 0.6b-gpu, 1.1b-gpu)
//   NO  → Auto-detect based on VRAM
//
// AUTO-DETECTION:
//   VRAM >= 4096 MB  → Parakeet-TDT 1.1B INT8 GPU (best quality)
//   VRAM >= 1536 MB  → Parakeet-TDT 0.6B GPU (good quality)
//   VRAM < 1536 MB   → Parakeet-TDT 0.6B CPU (fallback)
//   No GPU detected  → Parakeet-TDT 0.6B CPU (fallback)
//
// THRESHOLDS:
//   4096 MB = 1.1B peak (3.5GB) + 596MB headroom
//   1536 MB = 0.6B peak (1.2GB) + 336MB headroom
```

**Headroom Rationale:**
- VRAM requirements include model weights + activation memory
- Headroom accounts for other GPU processes (compositor, browser, etc.)
- Conservative thresholds prevent OOM crashes

### 4.3 VRAM Usage by Model

| Model | Weights | Activations (peak) | Total (peak) | Threshold |
|-------|---------|-------------------|--------------|-----------|
| 1.1B INT8 | 1.1GB | ~2.4GB | ~3.5GB | 4096MB |
| 0.6B GPU | 600MB | ~600MB | ~1.2GB | 1536MB |
| 0.6B CPU | 600MB | 0 (RAM) | ~960MB RAM | 0 |

---

## 5. Key Architectural Patterns

### 5.1 Unified STT Engine Pattern

**Problem:** Need to support multiple model implementations (sherpa-rs, direct ONNX Runtime) with different APIs

**Solution:** Enum-based unified interface

```rust
// stt/src/engine.rs
pub enum SttEngine {
    Parakeet0_6B(Recognizer),        // sherpa-rs backend
    Parakeet1_1B(OrtRecognizer),     // Direct ONNX Runtime
}

impl SttEngine {
    pub fn recognize(&mut self, audio: &[f32]) -> Result<RecognitionResult> {
        match self {
            SttEngine::Parakeet0_6B(r) => r.recognize(audio),
            SttEngine::Parakeet1_1B(r) => {
                // Adapter: OrtRecognizer::recognize_samples returns String
                let text = r.recognize_samples(audio)?;
                Ok(RecognitionResult { text, confidence: 1.0, ... })
            }
        }
    }
}
```

**Benefits:**
- Single interface for pipeline code (pipeline.rs doesn't care about backend)
- Runtime model swapping without code changes
- Easy to add new model implementations
- Type-safe variant handling

### 5.2 Lock-Free Audio Pipeline

**Problem:** Audio callbacks run in real-time thread, must complete in <1ms

**Solution:** Lock-free circular buffer + mpsc channel

```rust
// Audio thread (cpal callback):
audio.set_chunk_callback(move |chunk| {
    // NO locks, NO heap allocations, NO blocking operations
    let _ = audio_tx.send(chunk);  // mpsc is lock-free
});

// Processing thread (tokio):
tokio::spawn(async move {
    while let Some(chunk) = audio_rx.recv().await {
        // Can use locks here - not in real-time thread
        vad.process_audio(&chunk)?;
        stt.recognize(&speech_samples)?;
    }
});
```

**Benefits:**
- Predictable sub-100μs callback latency
- No audio glitches or dropouts
- Backpressure handled by channel capacity
- CPU-efficient (no spinlocks)

### 5.3 LSTM State Management for Streaming VAD

**Problem:** Silero VAD v6 requires persistent LSTM states for streaming audio

**Solution:** Manual LSTM state tensor management

```rust
// vad/src/silero_ort.rs
pub struct SileroVadOrt {
    h_state: Array3<f32>,  // [2, 1, 64] - LSTM hidden state
    c_state: Array3<f32>,  // [2, 1, 64] - LSTM cell state
    ...
}

pub fn process(&mut self, audio_chunk: &[f32]) -> Result<Option<Vec<f32>>> {
    // 1. Prepare inputs (audio + previous LSTM states)
    let outputs = session.run(inputs![
        "input" => input_array,
        "h" => self.h_state.view(),
        "c" => self.c_state.view(),
    ])?;

    // 2. Extract outputs (probability + updated LSTM states)
    let prob = outputs["output"].extract_tensor::<f32>()?;
    let new_h = outputs["hn"].extract_tensor::<f32>()?;
    let new_c = outputs["cn"].extract_tensor::<f32>()?;

    // 3. Update states for next chunk
    self.h_state = new_h.into_dimensionality::<Ix3>()?.to_owned();
    self.c_state = new_c.into_dimensionality::<Ix3>()?.to_owned();

    // 4. Run state machine (triggered, temp_end, speech_buffer)
    ...
}
```

**Benefits:**
- Streaming audio continuity (no context loss between chunks)
- State machine correctly handles speech start/end detection
- Proper LSTM state dimensions prevent ONNX Runtime errors

### 5.4 Adaptive Configuration Pattern

**Problem:** Need different configurations for different hardware/environments

**Solution:** TOML config with overrides + auto-detection

```rust
// daemon/src/config.rs
pub struct DaemonConfig {
    // Model paths
    pub stt_0_6b_model_path: String,
    pub stt_1_1b_model_path: String,
    pub vad_model_path: String,

    // Model selection (auto, 0.6b-cpu, 0.6b-gpu, 1.1b-gpu)
    pub stt_model_override: String,

    // VAD tuning
    pub vad_threshold: f32,
    pub vad_min_silence: f32,
    pub vad_min_speech: f32,
    ...
}

// Load from ~/.config/swictation/config.toml
let config = DaemonConfig::load()?;

// CLI overrides
if let Some(model) = cli.test_model {
    config.stt_model_override = model;
}
```

**Benefits:**
- Single config file for all settings
- CLI overrides for testing
- Auto-detection as default (works out-of-box)
- Per-user configuration

---

## 6. Critical Implementation Details

### 6.1 ONNX Runtime Dynamic Loading (recognizer_ort.rs:9-17)

**Issue:** `ort` crate v2.0.0-rc.8 requires ONNX Runtime 1.21.x, but system has 1.21.0

**Solution:** Dynamic library loading with `ORT_DYLIB_PATH`

```bash
# Set environment variable to point to compatible ONNX Runtime library
export ORT_DYLIB_PATH=/usr/lib/x86_64-linux-gnu/libonnxruntime.so.1.21.0

# Or use Python's onnxruntime-gpu package:
export ORT_DYLIB_PATH=$(python3 -c "import onnxruntime; import os; print(os.path.join(os.path.dirname(onnxruntime.__file__), 'capi/libonnxruntime.so.1.21.0'))")
```

**Why needed:**
- Avoids version conflicts between system ONNX Runtime and `ort` crate expectations
- Allows using CUDA-enabled ONNX Runtime from pip package
- Critical for CUDA execution provider availability

### 6.2 Silero VAD ONNX Threshold Calibration (vad/src/lib.rs:117-119)

**Issue:** Silero VAD ONNX model outputs probabilities ~100-200x lower than PyTorch JIT

**Solution:** Use threshold 0.003 (NOT 0.5 like PyTorch examples)

```rust
impl Default for VadConfig {
    fn default() -> Self {
        Self {
            // NOTE: Silero VAD ONNX model has ~100-200x lower probabilities than PyTorch JIT
            // Optimal threshold for ONNX: 0.001-0.005 (NOT 0.5 as in PyTorch examples)
            threshold: 0.003,
            ...
        }
    }
}
```

**Verification:**
- Tested with Python `onnxruntime` - probabilities match Rust implementation
- NOT a bug - ONNX graph optimization changes numerical scale
- Using 0.5 threshold will NEVER detect speech with ONNX model

### 6.3 Tensor Layout Requirements (vad/src/silero_ort.rs:139-140)

**Issue:** ONNX Runtime CUDA EP requires C-contiguous (standard) layout

**Solution:** Always use `Array::from_shape_vec` (standard layout by default)

```rust
// ✅ CORRECT: C-contiguous (standard) layout
let input_array = Array2::from_shape_vec((1, audio_chunk.len()), audio_chunk.to_vec())?;

// ❌ WRONG: Fortran layout
let input_array = Array2::from_shape_vec((1, audio_chunk.len()).f(), audio_chunk.to_vec())?;
```

**Why critical:**
- CUDA execution provider expects row-major (C) layout
- Fortran layout causes silent inference errors or crashes
- Must verify with `input_array.is_standard_layout()`

### 6.4 External ONNX Weights Loading (recognizer_ort.rs:125-131)

**Issue:** sherpa-rs has bug loading external weights for 1.1B model

**Solution:** Use `ort` crate directly - `commit_from_file` loads external weights automatically

```rust
// Direct ONNX Runtime loads external weights automatically:
// encoder.onnx references encoder.onnx.data (external weights file)
let encoder = session_builder
    .commit_from_file(&encoder_path)  // Automatically loads .onnx.data
    .map_err(|e| SttError::ModelLoadError(format!("Failed to load encoder: {}", e)))?;
```

**Why direct `ort` needed:**
- sherpa-rs 0.4.x doesn't handle external weights correctly for some models
- 1.1B model has 1.1GB weights split into separate `.onnx.data` file
- `ort` crate handles this automatically via `commit_from_file`

### 6.5 Model Input Format Detection (recognizer_ort.rs:154-181)

**Issue:** 0.6B and 1.1B models have different encoder input formats

**Solution:** Auto-detect from model directory name

```rust
// 0.6B model: (batch, 128 features, time) - NEEDS transpose
// 1.1B model: (batch, time, 80 features) - NO transpose

let transpose_input = if model_path.to_string_lossy().contains("1.1b") {
    info!("Detected 1.1B model - using natural format (no transpose)");
    false
} else if model_path.to_string_lossy().contains("0.6b") {
    info!("Detected 0.6B model - using transposed format");
    true
} else {
    info!("Unknown model, defaulting to transpose");
    true  // Safe default for 0.6B
};

// Also auto-detect mel feature count (80 for 1.1B, 128 for 0.6B)
let n_mel_features = if model_path.contains("1.1b") { 80 } else { 128 };
```

**Why needed:**
- Different model architectures expect different tensor shapes
- Avoids hardcoding model-specific logic in pipeline
- Allows single recognizer implementation for both models

---

## 7. Testing & Validation

### 7.1 Component-by-Component Testing (tests/)

**Test Files:**
- `rust-crates/swictation-vad/tests/test_pipeline.rs` - Full VAD → STT integration test
- `rust-crates/swictation-stt/tests/` - STT model loading and inference tests
- `rust-crates/swictation-audio/tests/` - Audio capture buffer tests

**Testing Strategy:**
1. **Unit Tests:** Individual component functionality (buffer, resampler, mel-spectrogram)
2. **Integration Tests:** VAD → STT pipeline with real audio files
3. **End-to-End Tests:** Full daemon pipeline with mock audio input

### 7.2 Validation Results (from commit messages)

**Latest Validation (commit a5a89758):**
- ✅ Audio capture working (cpal + ringbuf)
- ✅ VAD detection working (Silero VAD v6 with LSTM states)
- ✅ STT transcription working (0.6B and 1.1B models)
- ✅ Flush integration working (no lost audio at segment boundaries)
- ✅ End-to-end pipeline validated

**Known Issues Fixed:**
1. ~~Silero VAD LSTM state shape mismatch~~ (fixed: [2,1,64] not [1,2,64])
2. ~~VAD tensor name mismatches~~ (fixed: 'h'/'c' not 'h0'/'c0')
3. ~~ONNX threshold too high~~ (fixed: 0.003 not 0.5)
4. ~~Flush not processing buffered audio~~ (fixed: added flush call on stop)

---

## 8. Dependencies & External Components

### 8.1 External Dependencies

**Core ML Libraries:**
- **sherpa-rs** (git) - Sherpa-ONNX Rust bindings
  - Version: git main (supports CUDA 12.x + cuDNN 9.x)
  - Features: `cuda` enabled
  - Used for: 0.6B model inference

- **ort** (ONNX Runtime) - Direct ONNX Runtime bindings
  - Version: 2.0.0-rc.8 (STT), 2.0.0-rc.10 (VAD)
  - Features: `ndarray`, `half`, `load-dynamic`, `cuda`
  - Used for: 1.1B model inference, VAD inference

**Audio Processing:**
- **cpal** v0.15 - Cross-platform audio I/O
- **ringbuf** v0.4 - Lock-free circular buffer
- **rubato** v0.15 - Audio resampling
- **rustfft** v6.2 - FFT for mel-spectrograms

**System Integration:**
- **tokio** v1.0 - Async runtime
- **global-hotkey** v0.6 - Hotkey registration
- **swayipc** v3.0 - Wayland compositor IPC
- **sysinfo** v0.32 - System monitoring

**Data Storage:**
- **rusqlite** v0.32 - SQLite database
- **serde** v1.0 + **serde_json** - Serialization
- **chrono** v0.4 - Date/time handling

### 8.2 External Crates (outside workspace)

**midstreamer-text-transform:**
- Path: `external/midstream/crates/text-transform`
- Purpose: Voice command → symbol/punctuation transformation
- Example: "hello comma world" → "hello, world"

### 8.3 Model Files (not in repo)

**Required Models:**
1. **Parakeet-TDT 0.6B** (sherpa-onnx format)
   - Path: `/opt/swictation/models/parakeet-tdt-0.6b-v3-onnx/`
   - Files: `encoder.int8.onnx`, `decoder.int8.onnx`, `joiner.int8.onnx`, `tokens.txt`
   - Size: ~600MB

2. **Parakeet-TDT 1.1B INT8** (ONNX format)
   - Path: `/opt/swictation/models/parakeet-tdt-1.1b-onnx/`
   - Files: `encoder.int8.onnx` + `encoder.int8.onnx.data` (external weights)
   - Size: ~1.1GB

3. **Silero VAD v6**
   - Path: `/opt/swictation/models/silero_vad.onnx`
   - Size: ~1.8MB

---

## 9. Configuration & Deployment

### 9.1 Configuration Files

**Location:** `~/.config/swictation/config.toml`

**Key Settings:**
```toml
[models]
stt_0_6b_model_path = "/opt/swictation/models/parakeet-tdt-0.6b-v3-onnx"
stt_1_1b_model_path = "/opt/swictation/models/parakeet-tdt-1.1b-onnx"
vad_model_path = "/opt/swictation/models/silero_vad.onnx"

# Model selection: "auto", "0.6b-cpu", "0.6b-gpu", "1.1b-gpu"
stt_model_override = "auto"

[audio]
device_index = null  # null = default device
sample_rate = 16000
channels = 1

[vad]
threshold = 0.003
min_silence = 0.5
min_speech = 0.25
max_speech = 30.0

[hotkeys]
toggle = "Super+Shift+D"
push_to_talk = "Super+Space"

[ipc]
socket_path = "/tmp/swictation.sock"
```

### 9.2 Environment Variables

```bash
# ONNX Runtime library path (critical for CUDA support)
export ORT_DYLIB_PATH=/usr/lib/x86_64-linux-gnu/libonnxruntime.so.1.21.0

# Audio device override (optional)
export SWICTATION_AUDIO_DEVICE="USB Microphone"

# Logging level (optional)
export RUST_LOG=info
```

### 9.3 Systemd Service (daemon deployment)

**Service File:** `/etc/systemd/user/swictation.service`

```ini
[Unit]
Description=Swictation Voice-to-Text Daemon
After=pipewire.service

[Service]
Type=simple
ExecStart=/usr/local/bin/swictation-daemon
Environment="ORT_DYLIB_PATH=/usr/lib/x86_64-linux-gnu/libonnxruntime.so.1.21.0"
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
```

**Commands:**
```bash
systemctl --user enable swictation
systemctl --user start swictation
systemctl --user status swictation
```

---

## 10. Performance Characteristics

### 10.1 Memory Usage

| Component | Idle | Recording (0.6B CPU) | Recording (0.6B GPU) | Recording (1.1B GPU) |
|-----------|------|---------------------|---------------------|---------------------|
| Audio buffer | 10MB | 10MB | 10MB | 10MB |
| VAD model | 20MB | 20MB | 20MB (RAM) + 20MB (VRAM) | 20MB (RAM) + 20MB (VRAM) |
| STT model | 600MB | 960MB (RAM) | 1200MB (VRAM) | 3500MB (VRAM) |
| Daemon overhead | 50MB | 50MB | 50MB | 50MB |
| **Total** | **680MB** | **1040MB RAM** | **80MB RAM + 1220MB VRAM** | **80MB RAM + 3520MB VRAM** |

### 10.2 Latency Breakdown (GPU)

| Stage | 0.6B GPU | 1.1B GPU |
|-------|----------|----------|
| Audio capture | <1ms | <1ms |
| VAD detection | 5-10ms | 5-10ms |
| STT transcription | 100-150ms | 150-250ms |
| Text transformation | <1ms | <1ms |
| Text injection | 10-50ms | 10-50ms |
| **Total (typical)** | **115-210ms** | **165-310ms** |

**Throughput:**
- 0.6B GPU: ~6-10 segments/second
- 1.1B GPU: ~3-6 segments/second
- 0.6B CPU: ~2-5 segments/second

### 10.3 Accuracy (WER - Word Error Rate)

| Model | WER | Quantization | Quality |
|-------|-----|--------------|---------|
| Parakeet-TDT 1.1B | 5.77% | INT8 | Excellent |
| Parakeet-TDT 0.6B | 7-8% | FP16/INT8 | Good |

---

## 11. Recent Commits & Evolution

### 11.1 Commit History Analysis (last 5 commits)

1. **a5a89758** - "fix: Complete end-to-end pipeline validation - ALL WORKING!"
   - Full pipeline validation
   - VAD → STT → Transform → Inject working

2. **6bc608b0** - "feat: Add component-by-component pipeline testing"
   - Integration tests for VAD → STT
   - Test audio files for validation

3. **0a5b0375** - "fix(vad): Fix Silero VAD ONNX tensor names, LSTM states, and flush integration"
   - Fixed LSTM state shape: [2,1,64] not [1,2,64]
   - Fixed tensor names: 'h'/'c' not 'h0'/'c0'
   - Added flush() call to process buffered audio

4. **6d00de8c** - "debug: Add ONNX model metadata logging to identify VAD tensor names"
   - Added debug output for ONNX model inputs/outputs
   - Helped diagnose tensor naming issues

5. **6e872f8c** - "docs: Update architecture.md Section 4 with accurate adaptive STT implementation"
   - Documentation of adaptive model selection logic
   - VRAM threshold documentation

### 11.2 Key Milestones

- ✅ **Audio capture working** (cpal + ringbuf, zero-copy)
- ✅ **VAD integration complete** (Silero VAD v6 with LSTM states)
- ✅ **Dual STT engines** (sherpa-rs for 0.6B, direct ort for 1.1B)
- ✅ **Adaptive model selection** (VRAM-based decision tree)
- ✅ **Flush integration** (no lost audio at segment boundaries)
- ✅ **End-to-end pipeline** (Audio → VAD → STT → Transform → Inject)

---

## 12. Future Enhancements & TODOs

### 12.1 Identified in Codebase

**GPU Detection (daemon/src/gpu.rs:10-12):**
```rust
// TODO: Add Metal detection for macOS
// TODO: Add ROCm detection for AMD GPUs
```

**Audio Capture (audio/src/capture.rs:141-150):**
```rust
// TODO: Add device selection by partial name match
// TODO: Add device priority list (prefer USB over built-in)
```

**Metrics (metrics/src/lib.rs):**
```rust
// TODO: Add WASM metrics API for browser/Electron integration
// TODO: Add metrics export (CSV, JSON)
```

### 12.2 Potential Improvements

1. **Model Caching:**
   - Keep both 0.6B and 1.1B models loaded in memory
   - Hot-swap based on real-time VRAM availability

2. **Streaming STT:**
   - Current: Batch processing after VAD detects speech
   - Future: True streaming with partial results

3. **Multi-Language Support:**
   - Parakeet-TDT supports multiple languages
   - Add language detection/switching

4. **Advanced VAD:**
   - Speaker diarization (who is speaking)
   - Noise robustness improvements

5. **Metrics Dashboard:**
   - Web-based real-time metrics visualization
   - Historical performance analysis

---

## 13. Conclusions & Recommendations

### 13.1 Strengths

1. **Modern Rust Architecture:**
   - Pure Rust implementation (no Python dependencies)
   - Type-safe, memory-safe, thread-safe
   - Zero-copy audio pipeline

2. **Adaptive Intelligence:**
   - Automatic model selection based on hardware
   - Graceful CPU fallback when GPU unavailable
   - Conservative VRAM thresholds prevent OOM

3. **Production-Ready:**
   - Comprehensive error handling
   - Metrics tracking with SQLite persistence
   - Real-time UI updates via Unix socket broadcasting
   - Systemd integration for daemon deployment

4. **Performance:**
   - Sub-100μs audio callback latency
   - 105-310ms end-to-end latency (GPU)
   - 5.77-8% WER (excellent accuracy)

### 13.2 Areas for Improvement

1. **Testing Coverage:**
   - Need more unit tests for edge cases
   - Stress testing for long recording sessions
   - Cross-platform testing (currently Linux-focused)

2. **Documentation:**
   - API documentation (rustdoc)
   - User guide for configuration
   - Troubleshooting guide

3. **GPU Support:**
   - Add AMD ROCm support
   - Add Apple Metal support (macOS)
   - Improve GPU detection reliability

4. **Model Management:**
   - Automatic model downloading
   - Model version checking
   - Support for custom models

### 13.3 Critical Findings for Development

**Must Know for New Developers:**

1. **ONNX Runtime Setup:** MUST set `ORT_DYLIB_PATH` environment variable
2. **VAD Threshold:** Use 0.003 for ONNX (NOT 0.5 from PyTorch examples)
3. **Tensor Layout:** Always use C-contiguous (standard) layout for CUDA
4. **LSTM States:** Must persist [2,1,64] shaped tensors between VAD chunks
5. **Model Selection:** 4GB VRAM threshold for 1.1B, 1.5GB for 0.6B GPU

**Common Pitfalls:**
- Using wrong VAD threshold → never detects speech
- Forgetting to call `vad.flush()` → loses last speech segment
- Using Fortran layout → CUDA inference fails silently
- Not setting `ORT_DYLIB_PATH` → CUDA execution provider unavailable

---

## 14. Appendix: File Index

### 14.1 Critical Files Reference

| File | Purpose | LOC | Complexity |
|------|---------|-----|------------|
| `daemon/src/pipeline.rs` | Pipeline orchestration | 632 | HIGH ⭐⭐⭐ |
| `daemon/src/main.rs` | Daemon entry point | 464 | MEDIUM ⭐⭐ |
| `stt/src/engine.rs` | Unified STT interface | 215 | MEDIUM ⭐⭐ |
| `stt/src/recognizer_ort.rs` | 1.1B ONNX Runtime recognizer | ~500 | HIGH ⭐⭐⭐ |
| `vad/src/silero_ort.rs` | Silero VAD implementation | ~300 | HIGH ⭐⭐⭐ |
| `audio/src/capture.rs` | Audio capture | ~400 | MEDIUM ⭐⭐ |
| `metrics/src/lib.rs` | Metrics collection | ~500 | MEDIUM ⭐⭐ |

### 14.2 Dependency Graph

```text
swictation-daemon
├── swictation-audio (capture)
├── swictation-vad (VAD)
├── swictation-stt (STT)
│   ├── sherpa-rs (0.6B)
│   └── ort (1.1B)
├── swictation-metrics (tracking)
│   ├── rusqlite
│   └── sysinfo
├── swictation-broadcaster (real-time updates)
└── midstreamer-text-transform (external)
```

---

**End of Codebase Analysis**

**Next Steps:**
1. Review this document with development team
2. Update CLAUDE.md with critical findings
3. Create developer onboarding guide
4. Document model installation procedure
5. Set up CI/CD for automated testing

---

**Researcher:** Hive Mind Research Agent
**Coordination Key:** `hive/research/codebase-structure`
**Status:** Complete ✅
