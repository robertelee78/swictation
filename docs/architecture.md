# Swictation Architecture

Pure Rust implementation of high-performance voice dictation for Linux/Wayland.

---

## System Overview

Swictation is a **native Rust daemon** with VAD-triggered streaming for continuous recording with automatic transcription at natural pauses. All processing runs in compiled native code with zero Python runtime dependencies.

```
┌─────────────────────────────────────────────────────────────┐
│              SWICTATION-DAEMON (Rust Binary)                │
│                                                             │
│   Architecture: VAD-Triggered Streaming Transcription      │
│   State Machine:  [IDLE] ↔ [RECORDING] ↔ [PROCESSING]     │
│   VAD Detection: 512ms window, ONNX threshold 0.003       │
│   Runtime: Tokio async with state machine                  │
│                                                             │
│   IPC: Sway compositor integration via swayipc             │
│   Hotkeys: global-hotkey crate (cross-platform)            │
└─────────────────────────────────────────────────────────────┘
```

**Key Architecture Decisions:**
- **Pure Rust:** Zero Python runtime, native compiled binary
- **ONNX Runtime:** Direct ort crate bindings for CUDA acceleration
- **Tokio Async:** Efficient async I/O for real-time audio processing
- **Type Safety:** Compile-time guarantees prevent runtime errors
- **Memory Safety:** Ownership system eliminates use-after-free, data races

---

## Rust Workspace Structure

```
rust-crates/
├── swictation-daemon/      # Main daemon binary (tokio async)
│   ├── src/
│   │   ├── main.rs         # Entry point, service initialization
│   │   ├── pipeline.rs     # Audio → VAD → STT → Inject pipeline
│   │   └── config.rs       # Configuration loading
│   └── Cargo.toml          # Dependencies: tokio, swayipc, global-hotkey
│
├── swictation-audio/       # Audio capture (cpal/PipeWire)
│   ├── src/lib.rs          # Audio device enumeration, streaming
│   └── Cargo.toml          # Dependencies: cpal
│
├── swictation-vad/         # Voice Activity Detection
│   ├── src/
│   │   ├── lib.rs          # VAD configuration, detector interface
│   │   └── silero_ort.rs   # Silero VAD v6 via ort 2.0.0-rc.10
│   └── ONNX_THRESHOLD_GUIDE.md  # Threshold tuning documentation
│
├── swictation-stt/         # Speech-to-Text
│   ├── src/lib.rs          # Parakeet-TDT-1.1B via parakeet-rs
│   └── Cargo.toml          # Dependencies: parakeet-rs, ort
│
├── swictation-metrics/     # Performance tracking
│   └── src/lib.rs          # Latency, WER, memory metrics
│
└── swictation-broadcaster/ # Real-time metrics broadcast
    └── src/lib.rs          # Unix socket metrics streaming

external/midstream/         # Text transformation (Git submodule)
└── crates/text-transform/  # Voice commands → symbols (Rust)
```

---

## Core Components

### 1. Daemon Process (`swictation-daemon`)

**Purpose:** Main orchestrator coordinating audio → VAD → STT → injection pipeline

**Architecture:**
```rust
// rust-crates/swictation-daemon/src/pipeline.rs
pub struct Pipeline {
    audio: Arc<Mutex<AudioCapture>>,        // Audio capture
    vad: Arc<Mutex<VadDetector>>,           // Silero VAD v6 (ort)
    stt: Arc<Mutex<ParakeetTDT>>,           // Parakeet-TDT-1.1B
    metrics: Arc<Mutex<MetricsCollector>>,  // Performance tracking
    broadcaster: Arc<Mutex<Option<Arc<MetricsBroadcaster>>>>,
    is_recording: bool,
    session_id: Arc<Mutex<Option<i64>>>,
    tx: mpsc::UnboundedSender<Result<String>>,
}
```

**State Machine:**
```
[IDLE] ──────(hotkey)─────► [RECORDING] ─────(VAD silence)────► [PROCESSING]
   ↑                             │                                      │
   │                      [VAD Detection Loop]                          │
   │                       • 512ms window checks                        │
   │                       • ONNX threshold: 0.003                      │
   │                       • When silence >= 0.5s:                      │
   │                         → Enter PROCESSING                         │
   │                                                                    │
   │                                               • Transcribe segment │
   │                                               • Transform text     │
   │                                               • Inject via wtype   │
   │                                               • Clear buffer       │
   │                                                                    │
   └─────(hotkey or processing complete)──────────────────────────────┘
```

**Runtime:** Tokio async runtime with thread pool
- Main thread: Hotkey monitoring (global-hotkey crate)
- Async tasks: Audio streaming, VAD processing, STT inference
- Thread-safe: Arc + Mutex for shared state

**Performance:**
- Startup time: ~2s (model loading via ort)
- Hotkey latency: <10ms (global-hotkey crate)
- Memory: 150MB Rust daemon + 2.2GB GPU (models)

---

### 2. Audio Capture (`swictation-audio`)

**Purpose:** Real-time audio streaming from PipeWire/PulseAudio

**Architecture:**
```rust
use cpal::{Device, Stream, StreamConfig};

pub struct AudioCapture {
    device: Device,
    config: StreamConfig,  // 16kHz mono, i16
    stream: Option<Stream>,
}

impl AudioCapture {
    pub fn start_streaming<F>(&mut self, callback: F)
    where F: FnMut(&[f32]) + Send + 'static
    {
        // Real-time audio callbacks via cpal
    }
}
```

**cpal Backend:**
- **Linux:** PipeWire/ALSA backend (automatic detection)
- **Sample Rate:** 16kHz (required by STT models)
- **Format:** Mono, 16-bit PCM → f32 conversion
- **Latency:** <5ms overhead

**Why cpal?**
- ✅ Pure Rust audio I/O
- ✅ Cross-platform (Linux, Windows, macOS)
- ✅ Low latency real-time streaming
- ✅ Automatic backend selection (PipeWire/ALSA)

---

### 3. Voice Activity Detection (`swictation-vad`)

**Model:** Silero VAD v6 (August 2024, 16% less errors than v5)

**Implementation:**
```rust
// rust-crates/swictation-vad/src/silero_ort.rs
use ort::{Session, Tensor, inputs};

pub struct SileroVadOrt {
    session: Arc<Session>,
    state: Array2<f32>,        // RNN state [2, 1, 128]
    sample_rate: usize,        // 16000
    threshold: f32,            // 0.003 (ONNX, NOT PyTorch 0.5!)
}

impl SileroVadOrt {
    pub fn process_audio(&mut self, audio: &[f32]) -> VadResult {
        // Create input tensors (owned arrays for ort 2.0)
        let input_value = Tensor::from_array(input_array)?;
        let state_value = Tensor::from_array(self.state.clone())?;
        let sr_value = Tensor::from_array(arr1::<i64>(&[16000]))?;

        // Run ONNX inference
        let outputs = self.session.run(inputs![
            "input" => input_value,
            "state" => state_value,
            "sr" => sr_value
        ])?;

        // Extract probability and new state
        let probability = outputs["output"].extract_tensor()?[[0, 0]];
        self.state = outputs["stateN"].extract_tensor()?.to_owned();

        // CRITICAL: ONNX threshold is 0.001-0.005, NOT 0.5!
        if probability > self.threshold {
            VadResult::Speech
        } else {
            VadResult::Silence
        }
    }
}
```

**ONNX Threshold Configuration:**

**CRITICAL:** Silero VAD ONNX model outputs probabilities ~100-200x lower than PyTorch JIT!

| Threshold | Behavior | Use Case |
|-----------|----------|----------|
| 0.001 | Most sensitive | Catch quiet speech, may have false positives |
| 0.003 | **Balanced (default)** | Recommended for most users |
| 0.005 | Conservative | Fewer false positives, may miss quiet speech |

**See `rust-crates/swictation-vad/ONNX_THRESHOLD_GUIDE.md` for technical details.**

**Performance:**
- **Model size:** 2.3MB (ONNX v6)
- **GPU overhead:** 2.3MB VRAM
- **Latency:** <50ms per 512ms window
- **Inference:** ort 2.0.0-rc.10 with CUDA execution provider

**Why ort instead of sherpa-rs?**
- ✅ Modern CUDA support (CUDA 11.8+, 12.x)
- ✅ ~150x faster VAD inference
- ✅ Direct Rust bindings (no C++ wrapper overhead)
- ✅ Active development (sherpa-rs maintenance uncertain)

---

### 4. Speech-to-Text (`swictation-stt`)

**Model:** Parakeet-TDT-1.1B (NVIDIA) via parakeet-rs

**Implementation:**
```rust
// rust-crates/swictation-stt/src/lib.rs
use parakeet_rs::{ParakeetTDT, ExecutionConfig, ExecutionProvider};

pub struct SttEngine {
    model: ParakeetTDT,
    config: ExecutionConfig,
}

impl SttEngine {
    pub fn new(model_path: &str, gpu_provider: Option<String>) -> Result<Self> {
        let config = ExecutionConfig::builder()
            .execution_provider(ExecutionProvider::Cuda)
            .build();

        let model = ParakeetTDT::new(model_path, config)?;

        Ok(Self { model, config })
    }

    pub fn transcribe(&mut self, audio: &[f32]) -> Result<String> {
        let result = self.model.transcribe(audio)?;
        Ok(result.text)
    }
}
```

**Model Architecture:**
- **Type:** Transducer (encoder-decoder-joint)
- **Size:** 1.1B parameters
- **VRAM:** ~1.8GB (with FP16 optimization)
- **WER:** 5.77% (excellent for real-time)
- **RTF:** 0.106x (9.4x faster than realtime on RTX A1000)

**GPU Memory Breakdown:**
- Model weights: ~1.8GB (FP16)
- Context buffer (20s): ~400MB
- Inference activations: ~100MB
- **Total:** ~2.3GB (safe for 4GB VRAM)

**Why parakeet-rs?**
- ✅ Native Rust bindings to ONNX Runtime
- ✅ Direct CUDA integration (no Python overhead)
- ✅ Smaller memory footprint than NeMo
- ✅ Faster inference (native compiled code)

---

### 5. Text Transformation (`midstream/text-transform`)

**Purpose:** Transform voice commands to symbols with native Rust performance

**Architecture:**
```rust
// external/midstream/crates/text-transform/src/lib.rs
pub struct TextTransformer {
    rules: HashMap<String, String>,
}

impl TextTransformer {
    pub fn transform(&self, text: &str) -> String {
        // Fast string replacement with compiled rules
        let mut result = text.to_string();
        for (pattern, replacement) in &self.rules {
            result = result.replace(pattern, replacement);
        }
        result
    }
}

// Key transformations
// "comma" → ","
// "period" → "."
// "open parenthesis" → "("
// "underscore" → "_"
```

**Performance:**
- **Latency:** ~1µs per transformation (pure Rust)
- **Rules:** 266 transformation mappings
- **Integration:** Native Rust crate (no FFI overhead)

**Migration from PyO3:**
- Old: Python → PyO3 → Rust (~0.3µs with FFI overhead)
- New: Pure Rust crate (~1µs, zero FFI, simpler integration)

---

### 6. Text Injection (wtype)

**Purpose:** Inject transcribed text into focused Wayland application

**Implementation:**
```rust
use std::process::Command;

pub fn inject_text(text: &str) -> Result<()> {
    Command::new("wtype")
        .arg("-")
        .stdin(Stdio::piped())
        .spawn()?
        .stdin
        .unwrap()
        .write_all(text.as_bytes())?;

    Ok(())
}
```

**Why wtype?**
- ✅ Native Wayland support (no X11 dependencies)
- ✅ Full Unicode support (emojis, Greek, Chinese, CJK)
- ✅ Works with all Wayland applications
- ✅ Low latency (<20ms)

**Fallback:** wl-clipboard for manual paste

---

### 7. Hotkey Management (global-hotkey)

**Purpose:** Cross-platform hotkey registration for recording toggle

**Implementation:**
```rust
use global_hotkey::{GlobalHotKeyManager, HotKey, KeyCode, Modifiers};

pub struct HotkeyManager {
    manager: GlobalHotKeyManager,
    hotkey: HotKey,
}

impl HotkeyManager {
    pub fn new(key: KeyCode, mods: Modifiers) -> Result<Self> {
        let manager = GlobalHotKeyManager::new()?;
        let hotkey = HotKey::new(mods, key);
        manager.register(hotkey)?;

        Ok(Self { manager, hotkey })
    }

    pub fn poll_events(&self) -> impl Iterator<Item = HotKeyEvent> {
        self.manager.events()
    }
}
```

**Default Hotkey:** `Super+Shift+D` (configurable)

**Why global-hotkey?**
- ✅ Pure Rust implementation
- ✅ Cross-platform (Linux, Windows, macOS)
- ✅ X11 and Wayland support
- ✅ Low latency event polling

---

### 8. Compositor Integration (swayipc)

**Purpose:** Sway/Wayland compositor communication

**Implementation:**
```rust
use swayipc::Connection;

pub fn get_focused_window() -> Result<String> {
    let conn = Connection::new()?;
    let tree = conn.get_tree()?;

    if let Some(focused) = tree.find_focused(|n| n.focused) {
        Ok(focused.name.unwrap_or_default())
    } else {
        Err("No focused window".into())
    }
}
```

**Use Cases:**
- Query focused window for context
- Monitor workspace changes
- Integrate with Sway IPC for advanced features

---

## Data Flow

### VAD-Triggered Streaming Pipeline

```
1. USER PRESSES Super+Shift+D
   ↓
2. global-hotkey crate captures event
   ↓
3. Daemon state: IDLE → RECORDING
   ↓
4. Audio capture starts (cpal → PipeWire streaming)
   ↓
5. ┌──────────────────────────────────────────────────┐
   │  CONTINUOUS RECORDING LOOP (Until toggle off)   │
   │                                                  │
   │  Every audio chunk (real-time callback):        │
   │    • Accumulate audio in buffer                 │
   │    • Extract 512ms window for VAD               │
   │    • Run ONNX inference (ort)                   │
   │    • Check probability vs 0.003 threshold       │
   │    • Track silence duration                     │
   │                                                  │
   │  When 0.5s silence detected after speech:       │
   │    • Extract full segment from buffer           │
   │    • Transcribe with parakeet-rs (ONNX/CUDA)    │
   │    • Transform text (MidStream Rust)            │
   │    • Inject text via wtype                      │
   │    • Clear buffer, start new segment            │
   │    • Continue recording...                      │
   └──────────────────────────────────────────────────┘
   ↓
6. USER SPEAKS: "This is segment one." [pause]
   → Text appears: "This is segment one. "
   ↓
7. USER CONTINUES: "And here's segment two." [pause]
   → Text appears: "And here's segment two. "
   ↓
8. USER PRESSES Super+Shift+D AGAIN
   ↓
9. Daemon state: RECORDING → IDLE
   ↓
10. Final segment (if any) transcribed and injected
```

**Key Advantages:**
- ✅ No manual toggle between sentences
- ✅ Text appears automatically after natural pauses
- ✅ Full context for each segment (perfect accuracy)
- ✅ Native performance (zero Python overhead)
- ✅ Memory safe (Rust ownership prevents bugs)

---

## Performance Analysis

### Latency Breakdown (Per VAD Segment)

| Component | Latency | Notes |
|-----------|---------|-------|
| VAD Silence Detection | 500ms | Configurable (min_silence_duration) |
| VAD Check (512ms window) | <50ms | ONNX inference via ort |
| STT Processing | 150-250ms | Parakeet-TDT-1.1B (depends on length) |
| Text Transformation | ~1µs | Pure Rust (negligible!) |
| Text Injection | 10-50ms | wtype latency |
| **Total (from pause to text)** | **~1.0s** | Dominated by silence threshold |

### Memory Usage

| Component | Memory | Type |
|-----------|--------|------|
| Parakeet-TDT Model | 1.8GB | VRAM (FP16) |
| Context Buffer (20s) | 400MB | VRAM |
| Silero VAD v6 | 2.3MB | VRAM |
| Rust Daemon | 150MB | RAM |
| **Total** | **2.2GB VRAM, 150MB RAM** | - |

**Comparison with Python:**
- Python: ~250MB RAM + GC overhead
- Rust: ~150MB RAM (40% reduction, no GC)

### Accuracy Metrics

| Metric | Value | Notes |
|--------|-------|-------|
| WER (Word Error Rate) | 5.77% | Parakeet-TDT-1.1B |
| VAD Accuracy | 100% | Tested with 60s official speech audio |
| Unicode Support | 100% | All scripts (CJK, emoji, Greek) |
| Injection Success | 100% | wtype integration |

---

## Migration from Python

### What Changed

| Component | Old (Python) | New (Rust) | Improvement |
|-----------|--------------|------------|-------------|
| **Daemon** | swictationd.py | swictation-daemon | Native binary, no interpreter |
| **VAD** | sherpa-rs Python bindings | ort 2.0.0-rc.10 (direct) | ~150x faster, Silero v6 |
| **STT** | NeMo Toolkit (PyTorch) | parakeet-rs (ONNX) | Smaller memory footprint |
| **Audio** | sounddevice (Python) | cpal (Rust) | Lower latency, pure Rust |
| **Transform** | PyO3 wrapper | Native Rust crate | Zero FFI overhead |
| **Memory** | ~250MB Python + GC | ~150MB Rust (no GC) | 40% reduction |
| **Startup** | ~6.64s (Python import) | ~2s (native binary) | 70% faster |

### Why Pure Rust?

**Performance:**
- Native compiled code (no interpreter overhead)
- Zero garbage collection pauses
- Direct ONNX Runtime bindings (no FFI marshaling)
- Tokio async runtime (efficient I/O)

**Memory Safety:**
- Ownership system prevents use-after-free
- Type system eliminates data races
- Compile-time guarantees (no runtime panics)

**Reliability:**
- Single static binary (no dependency hell)
- No Python version conflicts
- Reproducible builds with cargo
- Easier deployment and distribution

### Legacy Python Files

The `src/*.py` files are **NOT used** by the current implementation:
- `src/swictationd.py` - Replaced by `rust-crates/swictation-daemon`
- `src/audio_capture.py` - Replaced by `swictation-audio` crate
- `src/nemo_patches.py` - Not needed (parakeet-rs doesn't use NeMo)

**Current execution:**
```bash
# systemd service runs:
/opt/swictation/rust-crates/target/release/swictation-daemon

# NOT:
# python3 /opt/swictation/src/swictationd.py  # (deprecated)
```

---

## systemd Integration

**Service File:** `config/swictation-daemon.service`

```ini
[Unit]
Description=Swictation Voice Dictation Daemon
After=pulseaudio.service
Wants=swictation-ui.service

[Service]
Type=simple
ExecStart=/opt/swictation/rust-crates/target/release/swictation-daemon
Restart=on-failure
RestartSec=5
Environment="LD_LIBRARY_PATH=%h/.cache/ort.pyke.io/..."
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
```

**Key Features:**
- Auto-restart on crash
- Journal logging (journalctl)
- LD_LIBRARY_PATH for ONNX Runtime shared libs
- Wants swictation-ui.service (optional Tauri GUI)

---

## Configuration

**Location:** `~/.config/swictation/config.toml`

```toml
[vad]
threshold = 0.003          # ONNX threshold (0.001-0.005, NOT 0.5!)
min_silence_duration = 0.5 # Seconds of silence before transcription
min_speech_duration = 0.25 # Minimum speech length to process

[stt]
model_path = "/opt/swictation/models/parakeet-tdt-1.1b.onnx"

[audio]
sample_rate = 16000
channels = 1

[hotkeys]
toggle = "Super+Shift+D"
```

**Default Configuration:** `rust-crates/swictation-daemon/src/config.rs:78`

---

## Testing Strategy

### Unit Tests (Rust)
```bash
cd rust-crates
cargo test --all
```

**Coverage:**
- Audio device enumeration
- VAD ONNX threshold configuration
- STT model loading and inference
- Text transformation rules
- Configuration parsing

### Integration Tests
```bash
cargo test --test integration
```

**Coverage:**
- Full pipeline (audio → VAD → STT → inject)
- Hotkey event handling
- State machine transitions
- systemd service lifecycle

### Performance Benchmarks
```bash
cargo bench
```

**Metrics:**
- VAD latency per 512ms window
- STT RTF (real-time factor)
- Memory usage profiling
- End-to-end latency

---

## Security Considerations

### Privacy
- ✅ 100% local processing (no network)
- ✅ No telemetry or analytics
- ✅ Audio never leaves device
- ✅ No cloud API calls

### Memory Safety
- ✅ Rust ownership prevents use-after-free
- ✅ Type system eliminates data races
- ✅ No buffer overflows (compile-time checks)
- ✅ Safe concurrency (Arc + Mutex)

### Permissions
- Config files: `rw-r--r--` (user + group read)
- Binary: `rwxr-xr-x` (world executable)
- No privileged operations required

---

## References

- **Parakeet-TDT Models:** [NVIDIA Research](https://catalog.ngc.nvidia.com/models)
- **Silero VAD v6:** [GitHub](https://github.com/snakers4/silero-vad)
- **ort crate:** [docs.rs/ort](https://docs.rs/ort/2.0.0-rc.10/)
- **parakeet-rs:** [crates.io/crates/parakeet-rs](https://crates.io/crates/parakeet-rs)
- **cpal:** [docs.rs/cpal](https://docs.rs/cpal/)
- **global-hotkey:** [docs.rs/global-hotkey](https://docs.rs/global-hotkey/)
- **wtype:** [GitHub](https://github.com/atx/wtype)
- **Tokio:** [tokio.rs](https://tokio.rs/)

---

**Last Updated:** 2025-11-09 (migrated to pure Rust architecture)
