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
│   State Machine:  [IDLE] ↔ [RECORDING]                     │
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
   │                                    │ • Process audio chunks in tokio task
   │                                    │ • Detect speech vs silence
   │                                    │ • Track silence duration (0.5s default)
   │                                    │ • When silence >= 0.5s after speech:
   │                                    │   → Transcribe segment (async)
   │                                    │   → Transform text (MidStream)
   │                                    │   → Inject via wtype
   │                                    │   → Clear buffer, continue recording
   │                                    │
   └─────────────(hotkey press again)──────┘
```

**States:**
- `Idle`: Daemon running, not recording (waiting for hotkey)
- `Recording`: Continuously capturing audio, VAD monitoring for silence, transcribing and injecting segments automatically when silence detected (all within this state)

**Key Features:**
- VAD-triggered automatic segmentation (0.5s silence threshold configurable via config)
- Continuous recording with real-time audio callbacks (tokio async)
- Lock-free ring buffer for audio streaming
- Global hotkey via global-hotkey crate (cross-platform)
- Pure Rust - zero Python runtime
- Graceful shutdown with signal handling
- Real-time metrics broadcasting via Unix socket (`/tmp/swictation_metrics.sock`)

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

**Adaptive Model Selection:** Intelligent runtime selection based on GPU VRAM at daemon startup

**Purpose:** Maximize transcription quality within hardware constraints

**Unified Interface (SttEngine):**
```rust
// Enum dispatch pattern - consistent API across model implementations
// Location: rust-crates/swictation-stt/src/engine.rs
pub enum SttEngine {
    Parakeet0_6B(Recognizer),      // sherpa-rs (GPU or CPU)
    Parakeet1_1B(OrtRecognizer),   // Direct ONNX Runtime (GPU only, INT8)
}

impl SttEngine {
    /// Recognize speech from audio samples (16kHz, mono, f32)
    pub fn recognize(&mut self, audio: &[f32]) -> Result<RecognitionResult>;

    /// Get model name for logging/metrics
    /// Returns: "Parakeet-TDT-0.6B" or "Parakeet-TDT-1.1B-INT8"
    pub fn model_name(&self) -> &str;

    /// Get model size identifier
    /// Returns: "0.6B" or "1.1B-INT8"
    pub fn model_size(&self) -> &str;

    /// Get backend type
    /// Returns: "GPU" or "CPU"
    pub fn backend(&self) -> &str;

    /// Get minimum VRAM required in MB
    /// Returns: 4096 (1.1B), 1536 (0.6B GPU), or 0 (0.6B CPU)
    pub fn vram_required_mb(&self) -> u64;
}
```

**Two Model Implementations:**

**1.1B INT8 Model (High-end GPUs):**
```rust
// Location: rust-crates/swictation-stt/src/recognizer_ort.rs
pub struct OrtRecognizer {
    encoder: Session,              // ONNX Runtime session
    decoder: Session,              // LSTM decoder with stateful RNN
    joiner: Session,               // Token predictor
    tokens: Vec<String>,           // 1025 tokens (BPE vocabulary)
    blank_id: i64,                 // Blank token (1024)
    audio_processor: AudioProcessor,  // 80 mel bins
    decoder_state1: Option<Array3<f32>>,  // LSTM state (2, batch, 640)
    decoder_state2: Option<Array3<f32>>,  // LSTM state (2, 1, 640)
}

impl OrtRecognizer {
    /// Create from model directory with encoder.int8.onnx, decoder.int8.onnx, joiner.int8.onnx
    pub fn new<P: AsRef<Path>>(model_dir: P, use_gpu: bool) -> Result<Self>;

    /// Recognize from audio samples (used by pipeline)
    pub fn recognize_samples(&mut self, samples: &[f32]) -> Result<String>;

    /// Recognize from audio file (WAV/MP3/FLAC)
    pub fn recognize_file<P: AsRef<Path>>(&mut self, path: P) -> Result<String>;
}
```

**0.6B Model (Moderate GPUs / CPU):**
```rust
// Location: rust-crates/swictation-stt/src/recognizer.rs
pub struct Recognizer {
    recognizer: TransducerRecognizer,  // sherpa-rs wrapper
    sample_rate: u32,                  // 16000 Hz
    use_gpu: bool,                     // GPU or CPU mode
}

impl Recognizer {
    /// Create from model directory with encoder.onnx, decoder.onnx, joiner.onnx, tokens.txt
    pub fn new<P: AsRef<Path>>(model_path: P, use_gpu: bool) -> Result<Self>;

    /// Recognize speech from audio samples
    pub fn recognize(&mut self, audio: &[f32]) -> Result<RecognitionResult>;

    /// Recognize from audio file
    pub fn recognize_file<P: AsRef<Path>>(&mut self, path: P) -> Result<RecognitionResult>;

    /// Check if GPU acceleration is enabled
    pub fn is_gpu(&self) -> bool;
}
```

**Model Characteristics:**

| Feature | 0.6B | 1.1B INT8 |
|---------|------|-----------|
| Type | RNN-T Transducer | RNN-T Transducer |
| Vocabulary | 1024 tokens | 1025 tokens |
| Mel Features | 128 bins | 80 bins |
| Quantization | FP32/FP16/INT8 | INT8 only |
| Library | sherpa-rs | Direct ort 2.0.0-rc.10 |
| WER | ~7-8% | 5.77% (best quality) |
| Peak VRAM | ~1.2GB | ~3.5GB |
| **Min VRAM Threshold** | **1536MB (1.5GB)** | **4096MB (4GB)** |
| Headroom | 336MB (28%) | 596MB (17%) |
| Latency (GPU) | 100-150ms | 150-250ms |
| Latency (CPU) | 200-400ms | N/A (GPU only) |

**VRAM Headroom Rationale:**
- **1.1B:** 4096MB threshold for 3.5GB peak = 596MB headroom (17%) for other GPU processes
- **0.6B GPU:** 1536MB threshold for 1.2GB peak = 336MB headroom (28%) for other GPU processes
- **0.6B CPU:** No VRAM required, uses ~960MB system RAM

**Adaptive Model Selection Decision Tree:**

```
                    START: Daemon initialization
                              │
                              ▼
                    ┌──────────────────────┐
                    │ config.stt_model_    │
                    │ override != "auto"?  │
                    └──────────────────────┘
                         │           │
                      YES│           │NO
                         ▼           ▼
               ┌────────────┐   ┌────────────────┐
               │ CLI/Config │   │ detect_gpu()   │
               │  Override  │   │ get_gpu_vram() │
               └────────────┘   └────────────────┘
                    │                    │
          ┌─────────┼─────────┐         │
          │         │         │         │
      "1.1b-gpu" "0.6b-gpu" "0.6b-cpu" │
          │         │         │         │
          │         │         │         ▼
          │         │         │    ┌──────────────┐
          │         │         │    │ VRAM ≥ 4GB?  │
          │         │         │    └──────────────┘
          │         │         │      YES│    │NO
          │         │         │         │    │
          │         │         │         │    ▼
          │         │         │         │  ┌───────────────┐
          │         │         │         │  │ VRAM ≥ 1.5GB? │
          │         │         │         │  └───────────────┘
          │         │         │         │    YES│    │NO
          │         │         │         │       │    │
          ▼         ▼         ▼         ▼       ▼    ▼
    ┌─────────┬─────────┬─────────┬─────────┬──────┬──────┐
    │ 1.1B    │ 0.6B    │ 0.6B    │ 1.1B    │ 0.6B │ 0.6B │
    │ INT8    │ sherpa  │ sherpa  │ INT8    │sherpa│sherpa│
    │ GPU     │ GPU     │ CPU     │ GPU     │ GPU  │ CPU  │
    │ FORCED  │ FORCED  │ FORCED  │ AUTO    │ AUTO │ AUTO │
    └─────────┴─────────┴─────────┴─────────┴──────┴──────┘
```

**Actual Implementation (pipeline.rs lines 77-227):**

```rust
// Location: rust-crates/swictation-daemon/src/pipeline.rs
// ADAPTIVE MODEL SELECTION based on GPU VRAM availability
//
// Decision tree:
//   ≥4GB VRAM → 1.1B INT8 GPU (peak 3.5GB + 596MB headroom = 17%)
//   ≥1.5GB VRAM → 0.6B GPU (peak 1.2GB + 336MB headroom = 28%)
//   <1.5GB or no GPU → 0.6B CPU fallback
//
// Config override: stt_model_override can force a specific model:
//   "auto" = VRAM-based selection (default)
//   "0.6b-cpu" = Force 0.6B CPU
//   "0.6b-gpu" = Force 0.6B GPU
//   "1.1b-gpu" = Force 1.1B GPU

let stt = if config.stt_model_override != "auto" {
    // MANUAL OVERRIDE: User specified exact model
    info!("STT model override active: {}", config.stt_model_override);

    match config.stt_model_override.as_str() {
        "1.1b-gpu" => {
            info!("  Loading Parakeet-TDT-1.1B-INT8 via ONNX Runtime (forced)...");
            let ort_recognizer = OrtRecognizer::new(&config.stt_1_1b_model_path, true)?;
            info!("✓ Parakeet-TDT-1.1B-INT8 loaded successfully (GPU, forced)");
            SttEngine::Parakeet1_1B(ort_recognizer)
        }
        "0.6b-gpu" => {
            info!("  Loading Parakeet-TDT-0.6B via sherpa-rs (GPU, forced)...");
            let recognizer = Recognizer::new(&config.stt_0_6b_model_path, true)?;
            info!("✓ Parakeet-TDT-0.6B loaded successfully (GPU, forced)");
            SttEngine::Parakeet0_6B(recognizer)
        }
        "0.6b-cpu" => {
            info!("  Loading Parakeet-TDT-0.6B via sherpa-rs (CPU, forced)...");
            let recognizer = Recognizer::new(&config.stt_0_6b_model_path, false)?;
            info!("✓ Parakeet-TDT-0.6B loaded successfully (CPU, forced)");
            SttEngine::Parakeet0_6B(recognizer)
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid stt_model_override: '{}'. \
                Valid options: 'auto', '0.6b-cpu', '0.6b-gpu', '1.1b-gpu'",
                config.stt_model_override
            ));
        }
    }
} else {
    // AUTO MODE: VRAM-based adaptive selection
    info!("STT model selection: auto (VRAM-based)");
    let vram_mb = get_gpu_memory_mb().map(|(total, _free)| total);

    if let Some(vram) = vram_mb {
        info!("Detected GPU with {}MB VRAM", vram);

        if vram >= 4096 {
            // High VRAM: Use 1.1B INT8 model for best quality (5.77% WER)
            info!("✓ Sufficient VRAM for 1.1B INT8 model (requires ≥4GB)");
            let ort_recognizer = OrtRecognizer::new(&config.stt_1_1b_model_path, true)?;
            info!("✓ Parakeet-TDT-1.1B-INT8 loaded successfully (GPU)");
            SttEngine::Parakeet1_1B(ort_recognizer)

        } else if vram >= 1536 {
            // Moderate VRAM: Use 0.6B GPU for good quality (7-8% WER)
            info!("✓ Sufficient VRAM for 0.6B GPU model (requires ≥1.5GB)");
            let recognizer = Recognizer::new(&config.stt_0_6b_model_path, true)?;
            info!("✓ Parakeet-TDT-0.6B loaded successfully (GPU)");
            SttEngine::Parakeet0_6B(recognizer)

        } else {
            // Low VRAM: Fall back to CPU
            warn!("⚠️  Only {}MB VRAM available (need ≥1.5GB for GPU)", vram);
            warn!("  Falling back to CPU mode (slower but functional)");
            let recognizer = Recognizer::new(&config.stt_0_6b_model_path, false)?;
            info!("✓ Parakeet-TDT-0.6B loaded successfully (CPU)");
            SttEngine::Parakeet0_6B(recognizer)
        }
    } else {
        // No GPU detected: Fall back to CPU
        warn!("⚠️  No GPU detected (nvidia-smi failed or no NVIDIA GPU)");
        let recognizer = Recognizer::new(&config.stt_0_6b_model_path, false)?;
        info!("✓ Parakeet-TDT-0.6B loaded successfully (CPU)");
        SttEngine::Parakeet0_6B(recognizer)
    }
};

// Log final configuration
info!("📊 STT Engine: {} ({}, {})",
      stt.model_name(),
      stt.model_size(),
      stt.backend());

if stt.vram_required_mb() > 0 {
    info!("   Minimum VRAM: {}MB", stt.vram_required_mb());
}
```

**Configuration Override System:**

**Config File (config.toml):**
```toml
# Location: ~/.config/swictation/config.toml

# STT model selection override
# Options: "auto" (VRAM-based), "0.6b-cpu", "0.6b-gpu", "1.1b-gpu"
stt_model_override = "auto"

# Path to 0.6B model directory (sherpa-rs)
stt_0_6b_model_path = "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx"

# Path to 1.1B INT8 model directory (ONNX Runtime)
stt_1_1b_model_path = "/opt/swictation/models/parakeet-tdt-1.1b-onnx"
```

**CLI Flags (Testing):**
```bash
# Dry-run mode: Show model selection without loading
$ swictation-daemon --dry-run
🧪 DRY-RUN MODE: Showing model selection without loading
  Mode: auto (VRAM-based)
  Detected: 97887MB VRAM
  Would load: Parakeet-TDT-1.1B-INT8 (GPU)
    Path: /opt/swictation/models/parakeet-tdt-1.1b-onnx
    Reason: ≥4GB VRAM available
✅ Dry-run complete (no models loaded)

# Force specific model for testing
$ swictation-daemon --test-model 0.6b-cpu
🧪 CLI override: forcing model '0.6b-cpu'
✓ Parakeet-TDT-0.6B loaded successfully (CPU, forced)

$ swictation-daemon --test-model 0.6b-gpu
🧪 CLI override: forcing model '0.6b-gpu'
✓ Parakeet-TDT-0.6B loaded successfully (GPU, forced)

$ swictation-daemon --test-model 1.1b-gpu
🧪 CLI override: forcing model '1.1b-gpu'
✓ Parakeet-TDT-1.1B-INT8 loaded successfully (GPU, forced)
```

**CLI Implementation:**
```rust
// Location: rust-crates/swictation-daemon/src/main.rs lines 22-35
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "swictation-daemon")]
#[command(about = "Voice-to-text dictation daemon with adaptive model selection")]
struct CliArgs {
    /// Override STT model selection (bypasses auto-detection)
    #[arg(long, value_name = "MODEL")]
    #[arg(value_parser = ["0.6b-cpu", "0.6b-gpu", "1.1b-gpu"])]
    test_model: Option<String>,

    /// Dry-run: show model selection without loading models
    #[arg(long)]
    dry_run: bool,
}
```

**Troubleshooting Guide:**

**1.1B Model Load Failure (despite sufficient VRAM):**
```
Error: Failed to load 1.1B INT8 model despite 97887MB VRAM.

Troubleshooting:
  1. Verify model files exist: ls /opt/swictation/models/parakeet-tdt-1.1b-onnx
  2. Check CUDA/cuDNN installation: nvidia-smi
  3. Ensure ONNX Runtime CUDA EP is available
  4. Try 0.6B fallback by setting stt_model_override="0.6b-gpu" in config
```

**0.6B GPU Model Load Failure (despite sufficient VRAM):**
```
Error: Failed to load 0.6B GPU model despite 8192MB VRAM.

Troubleshooting:
  1. Verify model files: ls /opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx
  2. Check CUDA availability: nvidia-smi
  3. Verify sherpa-rs CUDA support
  4. Try CPU fallback by setting stt_model_override="0.6b-cpu" in config
```

**0.6B CPU Model Load Failure:**
```
Error: Failed to load 0.6B CPU model.

Troubleshooting:
  1. Verify model files: ls /opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx
  2. Check available RAM (need ~1GB free)
  3. Ensure ONNX Runtime CPU EP is available
```

**Usage Examples:**

**Example 1: Auto-detection (recommended):**
```bash
$ swictation-daemon
🎙️ Starting Swictation Daemon v0.1.0
📋 Configuration loaded from /home/user/.config/swictation/config.toml
🎮 GPU detected: CUDA
STT model selection: auto (VRAM-based)
Detected GPU with 97887MB VRAM
✓ Sufficient VRAM for 1.1B INT8 model (requires ≥4GB)
  Loading Parakeet-TDT-1.1B-INT8 via ONNX Runtime...
✓ Parakeet-TDT-1.1B-INT8 loaded successfully (GPU)
📊 STT Engine: Parakeet-TDT-1.1B-INT8 (1.1B-INT8, GPU)
   Minimum VRAM: 4096MB
🚀 Swictation daemon ready!
```

**Example 2: Force CPU mode (low-end hardware):**
```toml
# Edit ~/.config/swictation/config.toml
stt_model_override = "0.6b-cpu"
```
```bash
$ swictation-daemon
STT model override active: 0.6b-cpu
  Loading Parakeet-TDT-0.6B via sherpa-rs (CPU, forced)...
✓ Parakeet-TDT-0.6B loaded successfully (CPU, forced)
📊 STT Engine: Parakeet-TDT-0.6B (0.6B, CPU)
```

**Example 3: Quick testing without config edits:**
```bash
$ swictation-daemon --test-model 0.6b-gpu
🧪 CLI override: forcing model '0.6b-gpu'
✓ Parakeet-TDT-0.6B loaded successfully (GPU, forced)
```

**Key Files:**
- `rust-crates/swictation-stt/src/engine.rs` - Unified SttEngine interface
- `rust-crates/swictation-stt/src/recognizer.rs` - 0.6B with sherpa-rs (GPU/CPU)
- `rust-crates/swictation-stt/src/recognizer_ort.rs` - 1.1B with direct ort (GPU only)
- `rust-crates/swictation-stt/src/audio.rs` - Mel feature extraction (80 bins for 1.1B)
- `rust-crates/swictation-daemon/src/pipeline.rs` - Adaptive selection logic (lines 77-227)
- `rust-crates/swictation-daemon/src/gpu.rs` - GPU detection + VRAM measurement
- `rust-crates/swictation-daemon/src/config.rs` - Configuration management (lines 54-62)
- `rust-crates/swictation-daemon/src/main.rs` - CLI argument parsing (lines 22-35, 169-222)

---

### 5. Text Transformation (MidStream)

**Purpose:** Transform voice commands to symbols (future feature)

**Current Status:** Rules reset to 0 (intentionally awaiting Parakeet-TDT behavior analysis)

**Architecture:**
```rust
// external/midstream/crates/text-transform/src/rules.rs
pub struct TextTransformer {
    rules: Vec<TransformRule>,  // Currently 0 rules (intentional)
}
```

**Implementation Status:**
- **Current:** 0 transformation rules (passthrough mode)
- **Reason:** Waiting for Parakeet-TDT STT output analysis (task 4218691c)
- **Next Step:** Implement dictation mode (task 3393b914)
- **Target:** 30-50 secretary dictation rules for natural punctuation
- **Focus:** Natural language punctuation only (NOT programming symbols)
- **Examples:** "comma" → ",", "period" → ".", "new paragraph" → "\n\n"
- **Performance:** ~1μs latency (native Rust)

**Integration:** Direct Rust function calls via midstreamer_text_transform crate (no FFI overhead)

**Why Empty?** Smart engineering - analyze real STT output first, then design rules based on actual model behavior, not assumptions.

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
   │  CONTINUOUS RECORDING LOOP (within Recording state) │
   │                                                 │
   │  Every audio chunk (real-time):                │
   │    • Accumulate audio in lock-free ring buffer │
   │    • Feed to VAD (Silero v6 ONNX)             │
   │    • Check speech vs silence (0.003 threshold) │
   │    • Track silence duration                    │
   │                                                 │
   │  When 0.5s silence detected after speech:      │
   │    • Extract full segment from buffer          │
   │    • Spawn async task to process segment:      │
   │      - Extract mel features (80 or 128 bins)   │
   │      - Transcribe with Parakeet-TDT (CPU/GPU)  │
   │      - Transform text (MidStream - planned)    │
   │      - Inject via wtype immediately            │
   │    • Clear buffer, start new segment           │
   │    • Continue recording in parallel...         │
   └─────────────────────────────────────────────────┘
   ↓
5. USER SPEAKS: "This is segment one." [0.5s pause]
   → VAD detects silence → transcribe (async) → inject
   → Text appears: "This is segment one."
   ↓
6. USER CONTINUES: "And here's segment two." [0.5s pause]
   → VAD detects silence → transcribe (async) → inject
   → Text appears: "And here's segment two."
   ↓
7. USER PRESSES $mod+Shift+d AGAIN
   ↓
8. Final segment (if any) transcribed and injected
   ↓
9. Daemon state: RECORDING → IDLE
   ↓
10. Session metrics saved (words dictated, WPM, etc.)
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
| VAD Silence Detection | 500ms | Configurable (default 0.5s in config.rs:80) |
| Audio Accumulation | Continuous | Zero overhead (lock-free buffer) |
| VAD Check per Chunk | <50ms | ONNX Runtime (CPU/GPU) |
| Mel Feature Extraction | 10-20ms | Pure Rust or sherpa-rs internal |
| STT Processing (0.6B GPU) | 100-150ms | sherpa-rs with CUDA |
| STT Processing (0.6B CPU) | 200-400ms | sherpa-rs CPU fallback |
| STT Processing (1.1B GPU) | 150-250ms | Direct ort with INT8 quantization |
| Text Transformation | ~1μs | Native Rust (currently passthrough) |
| Text Injection | 10-50ms | wtype latency |
| **Total (from pause to text)** | **~0.7-0.9s** | Dominated by silence threshold |

**Key Insight:** Users don't perceive the 0.5s threshold as "lag" because they're pausing naturally. This is configurable in config (vad_min_silence).

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
3. **Wayland + X11** - wtype (Wayland) and xdotool (X11) supported, auto-detected
4. **GPU Optional** - Works on CPU, faster with NVIDIA CUDA (AMD ROCm/DirectML planned)
5. **English Only** - Model supports multilingual but not exposed
6. **Text Transformation** - Currently 0 rules (intentional, awaiting STT analysis)

### Future Improvements

1. **Text Transformation Rules** - Implement 30-50 secretary dictation patterns (task 3393b914)
2. **AMD GPU Support** - ROCm execution provider (sherpa-rs + ort backend)
3. **DirectML** - Windows GPU acceleration
4. **CoreML/Metal** - macOS Apple Silicon support
5. **Multi-language** - Expose Parakeet's multilingual capabilities
6. **Custom Models** - Support for other ONNX STT models
7. **IPC Authentication** - Add authentication to metrics Unix socket (security)

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
