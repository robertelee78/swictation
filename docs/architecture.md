# Swictation Architecture

Detailed technical architecture documentation for the Swictation voice dictation system.

---

## System Overview

Swictation is a **pure Rust daemon** with VAD-triggered automatic transcription. The system uses **ONNX models** for both voice activity detection and speech recognition with CPU or GPU acceleration (auto-detected), with zero Python runtime dependencies.

```
┌─────────────────────────────────────────────────────────────┐
│           SWICTATION-DAEMON (Rust Binary)                   │
│                                                             │
│   Architecture: VAD-Triggered Streaming Transcription      │
│   State Machine:  [IDLE] ↔ [RECORDING] ↔ [PROCESSING]     │
│   Runtime: Tokio async with state machine                  │
│                                                             │
│   Control: Global hotkey ($mod+Shift+d)                    │
│   Output: wtype text injection (Wayland)                   │
└─────────────────────────────────────────────────────────────┘
```

---

## Core Components

### 1. Daemon Process (`swictation-daemon`)

**Binary:** `/opt/swictation/rust-crates/target/release/swictation-daemon`

**Purpose:** Main orchestrator coordinating audio → VAD → STT → transform → injection pipeline

**Architecture:**
```rust
struct SwictationDaemon {
    state: DaemonState,  // Idle | Recording | Processing
    audio_capture: AudioCapture,
    vad: VadDetector,
    stt: Recognizer,  // Sherpa-RS based recognizer
    text_transform: TextTransformer,
    text_injector: TextInjector,
}
```

**State Machine:**
```
[IDLE] ──────(hotkey press)─────► [RECORDING]
   ↑                                    │
   │                                    │ (continuous audio streaming)
   │                                    │ ↓
   │                             [VAD Detection Loop]
   │                                    │ • Process audio chunks
   │                                    │ • Detect speech vs silence
   │                                    │ • Track silence duration (0.8s threshold)
   │                                    │ • When silence >= 0.8s after speech:
   │                                    │   → Enter PROCESSING state
   │                                    │
   └─────(hotkey or segment complete)──────[PROCESSING]
                                                │ • Transcribe segment
                                                │ • Transform text (MidStream)
                                                │ • Inject via wtype
                                                │ • Clear buffer
                                                └─► Return to RECORDING or IDLE
```

**States:**
- `Idle`: Daemon running, not recording
- `Recording`: Continuously capturing audio, VAD monitoring for silence
- `Processing`: Transcribing and injecting text segment

**Key Features:**
- VAD-triggered automatic segmentation (0.8s silence threshold configurable)
- Continuous recording with real-time audio callbacks (tokio async)
- Lock-free ring buffer for audio streaming
- Global hotkey via global-hotkey crate (cross-platform)
- Pure Rust - zero Python runtime
- Graceful shutdown with signal handling

**Performance (Adaptive Model Selection):**
- Startup time: 1-3s (model loading + GPU detection)
- Hotkey latency: <10ms
- Transcription latency:
  - 1.1B GPU (5GB+ VRAM): 150-250ms
  - 0.6B GPU (3-4GB VRAM): 100-150ms
  - 0.6B CPU (fallback): 200-400ms
- Memory:
  - 1.1B GPU: ~2.2GB VRAM + 150MB RAM
  - 0.6B GPU: ~800MB VRAM + 150MB RAM
  - 0.6B CPU: ~960MB RAM

---

### 2. Audio Capture Module (`swictation-audio`)

**Purpose:** Real-time audio streaming from PipeWire using cpal

**Architecture:**
```rust
pub struct AudioCapture {
    sample_rate: u32,      // 16000 Hz (required by models)
    channels: u16,         // 1 (mono)
    buffer: RingBuffer,    // Lock-free circular buffer
    device: Device,        // cpal device handle
}
```

**Implementation:**
- **Backend:** cpal with PipeWire support
- **Buffer:** Lock-free ring buffer (ringbuf crate)
- **Resampling:** rubato for non-16kHz sources
- **Integration:** Direct into VAD processing pipeline

**Performance:**
- Latency: <5ms overhead
- Chunk size: Configurable (default 512 samples)
- Buffer: Lock-free for real-time performance

**Key Files:**
- `rust-crates/swictation-audio/src/capture.rs` (619 lines)
- `rust-crates/swictation-audio/src/buffer.rs` (167 lines)
- `rust-crates/swictation-audio/src/resampler.rs` (199 lines)

---

### 3. Voice Activity Detection (Silero VAD v6)

**Model:** Silero VAD v6 ONNX (August 2024 release)

**Purpose:** Detect speech vs silence for automatic segmentation

**Implementation:**
```rust
pub struct VadDetector {
    model: Session,           // ort 2.0.0-rc.10
    threshold: f32,           // 0.003 (ONNX threshold, NOT 0.5!)
    min_silence: Duration,    // 0.5-0.8s typical
    min_speech: Duration,     // 0.25s minimum
    sample_rate: u32,         // 16000 Hz
}
```

**Performance:**
- **Model size:** 2.3 MB
- **VRAM usage:** 2.3 MB
- **Latency:** <50ms per window
- **Accuracy:** 16% better on noisy data vs v5
- **Threshold:** 0.001-0.005 (ONNX outputs ~100-200x lower than PyTorch!)

**CRITICAL: ONNX Threshold Configuration**

The ONNX model outputs probabilities ~100-200x lower than PyTorch JIT:
- **PyTorch JIT:** probabilities ~0.02-0.2, use threshold ~0.5
- **ONNX:** probabilities ~0.0005-0.002, use threshold 0.001-0.005

**Recommended thresholds:**
- `0.001` - Most sensitive (catches quiet speech, may have false positives)
- `0.003` - Balanced (recommended default)
- `0.005` - Conservative (fewer false positives, may miss quiet speech)

See `rust-crates/swictation-vad/ONNX_THRESHOLD_GUIDE.md` for technical details.

**Integration:**
```rust
impl VadDetector {
    pub fn process_audio(&mut self, chunk: &[f32]) -> Result<VadResult> {
        // Returns Speech { samples } or Silence
    }
}
```

**Why VAD?**
- Automatic segmentation at natural pauses
- Prevents transcription of silence → battery savings
- Reduces GPU cycles → thermal optimization
- Enables continuous recording workflow

---

### 4. Speech-to-Text Engine (Parakeet-TDT)

**Adaptive Model Selection:** Runtime selection based on available GPU VRAM at startup

**Purpose:** Best-quality STT within available hardware constraints

**Available Architectures:**

**1.1B Model (High-end GPUs):**
```rust
pub struct OrtRecognizer {
    encoder: Session,          // Direct ort 2.0.0-rc.8
    decoder: Session,
    joiner: Session,
    audio_processor: AudioProcessor,
    decoder_state1: Array3<f32>,  // LSTM states
    decoder_state2: Array3<f32>,
}
```

**0.6B Model (Low-end GPUs / CPU):**
```rust
pub struct Recognizer {
    recognizer: TransducerRecognizer,  // sherpa-rs wrapper
    sample_rate: u32,                  // 16000 Hz
}
```

**Model Characteristics:**

| Feature | 0.6B | 1.1B |
|---------|------|------|
| Type | RNN-T Transducer | RNN-T Transducer |
| Vocabulary | 1024 tokens | 1025 tokens |
| Mel Features | 128 bins | 80 bins |
| Quantization | INT8 | FP32/FP16/INT8 |
| Library | sherpa-rs | Direct ort |
| WER | ~7-8% | 5.77% |
| VRAM Required | ~800MB | ~2.2GB |

**Adaptive Model Selection at Startup:**
```rust
// Daemon auto-detects GPU and selects best model (src/gpu.rs):
let gpu_provider = detect_gpu_provider();
let gpu_vram_mb = get_gpu_memory_mb();

let (model_path, recognizer) = match gpu_vram_mb {
    Some(vram) if vram >= 4096 => {
        // Strong GPU (5GB+ VRAM) → 1.1B model with OrtRecognizer
        info!("Strong GPU detected ({}MB VRAM) - using 1.1B model", vram);
        let recognizer = OrtRecognizer::new(
            "/opt/swictation/models/parakeet-tdt-1.1b",
            true  // GPU
        )?;
        ("1.1B", Box::new(recognizer))
    },
    Some(vram) if vram >= 1024 => {
        // Weak GPU (3-4GB VRAM) → Try 0.6B with GPU, fallback to CPU
        info!("Weak GPU detected ({}MB VRAM) - trying 0.6B with GPU", vram);
        let recognizer = Recognizer::new(
            "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8",
            true  // GPU
        ).unwrap_or_else(|e| {
            warn!("GPU failed: {}, falling back to CPU", e);
            Recognizer::new(model_path, false).unwrap()
        });
        ("0.6B", Box::new(recognizer))
    },
    _ => {
        // No GPU or insufficient VRAM → 0.6B with CPU
        info!("No GPU or insufficient VRAM - using 0.6B CPU");
        let recognizer = Recognizer::new(
            "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8",
            false  // CPU
        )?;
        ("0.6B CPU", Box::new(recognizer))
    }
};
```

**Performance by Configuration:**

| Configuration | Latency | Memory | WER | Use Case |
|---------------|---------|--------|-----|----------|
| **1.1B GPU** | 150-250ms | 2.2GB VRAM | 5.77% | Strong GPU (RTX 3060+, A1000+) |
| **0.6B GPU** | 100-150ms | 800MB VRAM | 7-8% | Weak GPU (RTX 2060, 1660) |
| **0.6B CPU** | 200-400ms | 960MB RAM | 7-8% | No GPU or fallback |

**Note:** 1.1B model is **GPU-only** (no CPU fallback due to high latency)

**Audio Processing Pipeline (0.6B - sherpa-rs):**
```rust
// Used for: Weak GPU or CPU fallback
let mut recognizer = Recognizer::new(
    "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8",
    use_gpu
)?;

let result = recognizer.recognize(&audio_samples)?;
// sherpa-rs handles all preprocessing internally:
// - Mel-spectrogram extraction (128 bins)
// - Feature normalization
// - Encoder/decoder/joiner inference
// - Greedy search decoding

println!("{}", result.text);
```

**Audio Processing Pipeline (1.1B - direct ort):**
```rust
// Used for: Strong GPU (5GB+ VRAM)
let mut recognizer = OrtRecognizer::new(
    "/opt/swictation/models/parakeet-tdt-1.1b",
    true  // GPU-only
)?;

let text = recognizer.recognize_file("audio.wav")?;
// OrtRecognizer handles:
// - Audio loading (WAV/MP3/FLAC via symphonia)
// - Mel-spectrogram extraction (80 bins)
// - Encoder inference with dynamic shapes
// - TDT greedy search with LSTM state persistence
// - Token-to-text conversion
```

**Implementation Status:**

✅ **Implemented - Adaptive Selection:**
- Runtime model selection based on GPU VRAM
- Strong GPU (5GB+) → 1.1B with OrtRecognizer
- Weak GPU (3-4GB) → 0.6B with sherpa-rs GPU
- No GPU → 0.6B with sherpa-rs CPU
- Automatic fallback on GPU allocation failure

**Key Files:**
- `rust-crates/swictation-stt/src/recognizer.rs` - 0.6B with sherpa-rs (weak GPU / CPU)
- `rust-crates/swictation-stt/src/recognizer_ort.rs` - 1.1B with direct ort (strong GPU only)
- `rust-crates/swictation-stt/src/audio.rs` - Mel feature extraction (for 1.1B)
- `rust-crates/swictation-daemon/src/pipeline.rs` - Adaptive model selection logic
- `rust-crates/swictation-daemon/src/gpu.rs` - GPU detection + VRAM checking

---

### 5. Text Transformation (MidStream)

**Purpose:** Transform voice commands to symbols (future feature)

**Current Status:** Rules reset to 0 (rebuilding for dictation mode)

**Architecture:**
```rust
// external/midstream/crates/text-transform/
pub struct TextTransformer {
    rules: Vec<TransformRule>,  // Currently 0 rules
}
```

**Planned Implementation:**
- **Target:** 30-50 secretary dictation rules
- **Focus:** Natural language punctuation (NOT programming symbols)
- **Examples:** "comma" → ",", "period" → ".", "new paragraph" → "\n\n"
- **Performance:** ~1μs latency (native Rust)

**Integration:** Direct Rust function calls (no FFI overhead)

See task `4218691c-852d-4a67-ac89-b61bd6b18443` for pattern documentation plan.

---

### 6. Text Injection Module (`text_injection`)

**Purpose:** Inject transcribed text into focused Wayland application

**Implementation:**
```rust
pub struct TextInjector {
    method: InjectionMethod,  // Wtype | WlClipboard
}

pub fn inject(&self, text: &str) -> Result<()> {
    match self.method {
        InjectionMethod::Wtype => self.inject_wtype(text),
        InjectionMethod::WlClipboard => self.inject_clipboard(text),
    }
}
```

**Primary Method: wtype**
```bash
# Wayland-native keyboard simulation
echo "Hello, world!" | wtype -
```

**Advantages:**
- ✅ Native Wayland support (no X11 dependencies)
- ✅ Full Unicode support (emojis, Greek, Chinese, all scripts)
- ✅ Works with all Wayland applications
- ✅ Low latency (10-50ms)

**Fallback: wl-clipboard**
```bash
# Clipboard paste as fallback
echo "Hello, world!" | wl-copy
# User manually pastes with Ctrl+V
```

**Unicode Handling:**
- All Unicode ranges supported via UTF-8 encoding
- Tested: ASCII, Latin Extended, Greek, Cyrillic, CJK, Emojis

**Performance:**
- Latency: 10-50ms per batch
- Success rate: 100% on Sway/Wayland compositors

---

## Data Flow

### Complete Pipeline (One Transcription Cycle)

```
1. USER PRESSES $mod+Shift+d (Global Hotkey)
   ↓
2. Daemon state: IDLE → RECORDING
   ↓
3. Audio capture starts (cpal → PipeWire → streaming)
   ↓
4. ┌─────────────────────────────────────────────────┐
   │  CONTINUOUS RECORDING LOOP                      │
   │                                                 │
   │  Every audio chunk (real-time):                │
   │    • Accumulate audio in lock-free ring buffer │
   │    • Feed to VAD (Silero v6 ONNX)             │
   │    • Check speech vs silence (0.003 threshold) │
   │    • Track silence duration                    │
   │                                                 │
   │  When 0.8s silence detected after speech:      │
   │    • Extract full segment from buffer          │
   │    • State: RECORDING → PROCESSING             │
   │    • Extract mel features (80 bins)            │
   │    • Transcribe with Parakeet-TDT (CPU/GPU) │
   │    • Transform text (MidStream - future)       │
   │    • Inject via wtype immediately              │
   │    • Clear buffer, start new segment           │
   │    • State: PROCESSING → RECORDING             │
   │    • Continue recording...                     │
   └─────────────────────────────────────────────────┘
   ↓
5. USER SPEAKS: "This is segment one." [0.8s pause]
   → VAD detects silence → transcribe → inject
   → Text appears: "This is segment one."
   ↓
6. USER CONTINUES: "And here's segment two." [0.8s pause]
   → VAD detects silence → transcribe → inject
   → Text appears: "And here's segment two."
   ↓
7. USER PRESSES $mod+Shift+d AGAIN
   ↓
8. Final segment (if any) transcribed and injected
   ↓
9. Daemon state: RECORDING → IDLE
```

**Key Advantages:**
- ✅ No manual toggle between sentences
- ✅ Text appears automatically after natural pauses
- ✅ Full context for each segment (accurate transcription)
- ✅ Continuous workflow (speak naturally)
- ✅ Pure Rust (no Python overhead)

---

## Performance Analysis

### Latency Breakdown (Per VAD Segment)

| Component | Latency | Notes |
|-----------|---------|-------|
| VAD Silence Detection | 800ms | Configurable threshold (0.5s-2s) |
| Audio Accumulation | Continuous | Zero overhead (lock-free buffer) |
| VAD Check per Chunk | <50ms | ONNX Runtime (CPU/GPU) |
| Mel Feature Extraction | 10-20ms | Pure Rust or sherpa-rs internal |
| STT Processing (0.6B) | 100-400ms | sherpa-rs (CPU/GPU auto-detect) |
| STT Processing (1.1B planned) | 150-800ms | Direct ort (CPU/GPU) |
| Text Transformation | ~1μs | Native Rust (negligible) |
| Text Injection | 10-50ms | wtype latency |
| **Total (from pause to text)** | **~1.0s** | Dominated by silence threshold |

**Key Insight:** Users don't perceive the 0.8s threshold as "lag" because they're pausing naturally.

### Memory Usage

**Adaptive Selection - Three Configurations:**

**1.1B GPU (Strong GPU: 5GB+ VRAM):**

| Component | Memory | Type |
|-----------|--------|------|
| Parakeet-TDT 1.1B | ~1.8 GB | VRAM |
| Context Buffer | ~400 MB | VRAM |
| Silero VAD | 2.3 MB | VRAM |
| Audio Buffer | ~10 MB | RAM |
| Rust Daemon | ~150 MB | RAM |
| **Total** | **~2.2 GB VRAM, ~160 MB RAM** | Typical |
| **Peak** | **~3.5 GB VRAM** | During inference |

**0.6B GPU (Weak GPU: 3-4GB VRAM):**

| Component | Memory | Type |
|-----------|--------|------|
| Parakeet-TDT 0.6B | ~800 MB | VRAM |
| Silero VAD | 2.3 MB | VRAM |
| Audio Buffer | ~10 MB | RAM |
| Rust Daemon | ~150 MB | RAM |
| **Total** | **~800 MB VRAM, ~160 MB RAM** | Typical |

**0.6B CPU (No GPU / Fallback):**

| Component | Memory | Type |
|-----------|--------|------|
| Parakeet-TDT 0.6B | ~800 MB | RAM |
| Silero VAD | 2.3 MB | RAM |
| Audio Buffer | ~10 MB | RAM |
| Rust Daemon | ~150 MB | RAM |
| **Total** | **~960 MB RAM** | CPU-only mode |

**Hardware Recommendations:**
- **Best:** 5GB+ VRAM GPU (RTX 3060, A1000, 4060) → 1.1B model, 5.77% WER
- **Good:** 3-4GB VRAM GPU (RTX 2060, 1660) → 0.6B GPU, 7-8% WER
- **Works:** Any CPU → 0.6B CPU, 7-8% WER (slower but functional)

### Accuracy Metrics

| Metric | Value | Notes |
|--------|-------|-------|
| WER (Word Error Rate) | 5.77-8% | Adaptive: 5.77% (1.1B GPU), 7-8% (0.6B) |
| VAD Accuracy | 16% better | Silero v6 vs v5 on noise |
| Unicode Support | 100% | All scripts tested |
| Injection Success | 100% | Wayland compositors |

---

## Workspace Structure

```
rust-crates/
├── swictation-daemon/      # Main daemon binary (tokio async)
├── swictation-audio/       # Audio capture (cpal/PipeWire)
├── swictation-vad/         # Voice Activity Detection (Silero v6 + ort)
├── swictation-stt/         # Speech-to-Text (Parakeet-TDT + sherpa-rs)
├── swictation-metrics/     # Performance tracking
└── swictation-broadcaster/ # Real-time metrics broadcast

external/midstream/         # Text transformation (Git submodule)
└── crates/text-transform/  # Voice commands → symbols
```

---

## Scaling Considerations

### Current Limitations

1. **Single User** - One daemon per user session
2. **Single GPU** - No multi-GPU support (when using GPU)
3. **Wayland Only** - wtype limitation (no X11 support)
4. **GPU Optional** - Works on CPU, faster with NVIDIA CUDA (AMD ROCm/DirectML planned)
5. **English Only** - Model supports multilingual but not exposed

### Future Improvements

1. **AMD GPU Support** - ROCm execution provider (sherpa-rs + ort backend)
2. **DirectML** - Windows GPU acceleration
3. **CoreML/Metal** - macOS Apple Silicon support
4. **Configurable VAD Threshold** - UI for silence duration adjustment
5. **Multi-language** - Expose Parakeet's multilingual capabilities
6. **Custom Models** - Support for other ONNX STT models
7. **Voice Commands** - Rebuild text transformation rules (30-50 dictation patterns)

---

## Security Considerations

### Privacy
- ✅ 100% local processing (no network)
- ✅ No telemetry or analytics
- ✅ Audio never leaves device
- ✅ No cloud API calls
- ✅ All models run locally on CPU or GPU

### Permissions
- Binary: `rwxr-xr-x` (world executable)
- Config files: `rw-r--r--` (user + group read)
- Models: `rw-r--r--` (user + group read)

### Attack Surface
- Global hotkey only attack vector (minimal)
- No network exposure
- systemd sandboxing available
- No privileged operations required

---

## systemd Integration

**Service File:** `~/.config/systemd/user/swictation-daemon.service`

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
Environment="LD_LIBRARY_PATH=%h/.cache/ort.pyke.io/dfbin/x86_64-unknown-linux-gnu/ED1716DE95974BF47AB0223CA33734A0B5A5D09A181225D0E8ED62D070AEA893/onnxruntime/lib"
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
```

**Key Features:**
- Auto-restart on crash
- ONNX Runtime library path configuration
- Logs to journalctl
- Starts with user session

---

## Comparison with Alternatives

| Feature | Swictation | Talon | Dragon | Cloud STT |
|---------|-----------|-------|--------|-----------|
| Wayland Support | ✅ Native | ❌ X11 only | ❌ Windows | ✅ Browser |
| Runtime | Pure Rust | Python | Native | JavaScript |
| Latency | ~1s (VAD pause) | 100-150ms | 50-100ms | 500-1000ms |
| VAD Streaming | ✅ Auto | ❌ Manual | ❌ Manual | Varies |
| Privacy | ✅ Local | ✅ Local | ❌ Cloud | ❌ Cloud |
| Accuracy (WER) | 5.77-8% (adaptive) | ~3% WER | ~2% WER | 3-8% WER |
| GPU Required | Optional (faster) | Optional | No | No |
| VRAM Usage | 0.8-2.2GB | Varies | N/A | N/A |
| Cost | Free | $99-499 | $200+ | Free-paid |
| Open Source | ✅ | ❌ | ❌ | Varies |

---

## References

- **NVIDIA Parakeet-TDT:** [HuggingFace](https://huggingface.co/nvidia/parakeet-tdt-1.1b)
- **Silero VAD v6:** [GitHub](https://github.com/snakers4/silero-vad)
- **ONNX Runtime (ort):** [ort crate](https://crates.io/crates/ort)
- **cpal:** [Cross-platform Audio Library](https://crates.io/crates/cpal)
- **wtype:** [Wayland Type](https://github.com/atx/wtype)
- **PipeWire:** [PipeWire Docs](https://pipewire.org/)

---

**Last Updated:** 2025-11-10 (Pure Rust implementation complete)
