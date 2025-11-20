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
    state: DaemonState,  // Idle | Recording
    audio_capture: AudioCapture,
    vad: VadDetector,
    stt: SttEngine,  // Unified engine (OrtRecognizer for both models)
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
- VAD-triggered automatic segmentation (0.8s silence threshold default, configurable via config)
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
- `rust-crates/swictation-audio/src/capture.rs` (627 lines)
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
- **Model size:** ~630 KB (0.63 MB)
- **VRAM usage:** ~630 KB
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
    Parakeet0_6B(OrtRecognizer),   // Direct ONNX Runtime (GPU or CPU)
    Parakeet1_1B(OrtRecognizer),   // Direct ONNX Runtime (GPU only)
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

**Unified OrtRecognizer (Both Models):**
```rust
// Location: rust-crates/swictation-stt/src/recognizer_ort.rs
// Used for both 0.6B and 1.1B models via Direct ONNX Runtime integration
pub struct OrtRecognizer {
    encoder: Session,              // ONNX Runtime session
    decoder: Session,              // LSTM decoder with stateful RNN
    joiner: Session,               // Token predictor
    tokens: Vec<String>,           // 1024-1025 tokens (BPE vocabulary)
    blank_id: i64,                 // Blank token
    audio_processor: AudioProcessor,  // 80 or 128 mel bins (model-dependent)
    decoder_state1: Option<Array3<f32>>,  // LSTM state (2, batch, 640)
    decoder_state2: Option<Array3<f32>>,  // LSTM state (2, 1, 640)
    use_gpu: bool,                 // GPU or CPU mode
}

impl OrtRecognizer {
    /// Create from model directory (auto-detects FP32 vs INT8 variants)
    /// Prefers FP32 for GPU (better performance), INT8 for CPU (smaller memory)
    pub fn new<P: AsRef<Path>>(model_dir: P, use_gpu: bool) -> Result<Self>;

    /// Recognize from audio samples (used by pipeline)
    pub fn recognize_samples(&mut self, samples: &[f32]) -> Result<String>;

    /// Recognize from audio file (WAV/MP3/FLAC)
    pub fn recognize_file<P: AsRef<Path>>(&mut self, path: P) -> Result<String>;

    /// Check if GPU acceleration is enabled
    pub fn is_gpu(&self) -> bool;
}
```

**Note on sherpa-rs:**
The previous `Recognizer` implementation using sherpa-rs has been deprecated. Both 0.6B and 1.1B models now use direct ONNX Runtime integration (`OrtRecognizer`) for:
- Unified codebase (easier maintenance)
- Better Maxwell GPU support (sm_50-70 via CUDA 11.8)
- Consistent performance characteristics
- Simplified dependency management

**Model Characteristics:**

| Feature | 0.6B | 1.1B |
|---------|------|-----------|
| Type | RNN-T Transducer | RNN-T Transducer |
| Vocabulary | 1024 tokens | 1025 tokens |
| Mel Features | 128 bins | 80 bins |
| Quantization | FP32 (GPU), INT8 (CPU) | FP32 (GPU), INT8 (CPU) |
| Library | Direct ort 2.0.0-rc.8 | Direct ort 2.0.0-rc.8 |
| WER | ~7-8% | 5.77% (best quality) |
| Peak VRAM | ~800MB-1.2GB | ~3.5GB |
| **Min VRAM Threshold** | **3500MB (3.5GB)** | **6000MB (6GB)** |
| Headroom | Safe for 4GB GPUs | Safe for 8GB+ GPUs |
| Latency (GPU) | 100-150ms | 150-250ms |
| Latency (CPU) | 200-400ms | 300-500ms |

**VRAM Headroom Rationale:**
- **1.1B:** 6000MB threshold for ~3.5GB peak = 2.5GB headroom (42%) for safety margin and other GPU processes
- **0.6B GPU:** 3500MB threshold for ~1.2GB peak = 2.3GB headroom (66%) - fits comfortably in 4GB GPUs
- **0.6B CPU:** No VRAM required, uses ~960MB system RAM

**Source of Truth:** These thresholds are defined in `npm-package/postinstall.js` lines 1136-1156 and verified through real-world testing on production hardware (RTX A1000 4GB, RTX PRO 6000 Blackwell 97GB).

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
          │         │         │    │ VRAM ≥ 6GB?  │
          │         │         │    └──────────────┘
          │         │         │      YES│    │NO
          │         │         │         │    │
          │         │         │         │    ▼
          │         │         │         │  ┌───────────────┐
          │         │         │         │  │ VRAM ≥ 3.5GB? │
          │         │         │         │  └───────────────┘
          │         │         │         │    YES│    │NO
          │         │         │         │       │    │
          ▼         ▼         ▼         ▼       ▼    ▼
    ┌─────────┬─────────┬─────────┬─────────┬──────┬──────┐
    │ 1.1B    │ 0.6B    │ 0.6B    │ 1.1B    │ 0.6B │ 0.6B │
    │ FP32    │ FP32    │ FP32    │ FP32    │ FP32 │ FP32 │
    │ GPU     │ GPU     │ CPU     │ GPU     │ GPU  │ CPU  │
    │ FORCED  │ FORCED  │ FORCED  │ AUTO    │ AUTO │ AUTO │
    └─────────┴─────────┴─────────┴─────────┴──────┴──────┘
```

**Actual Implementation (pipeline.rs lines 77-227):**

```rust
// Location: rust-crates/swictation-daemon/src/pipeline.rs
// ADAPTIVE MODEL SELECTION based on GPU VRAM availability
//
// Decision tree (SOURCE OF TRUTH: npm-package/postinstall.js lines 1136-1156):
//   ≥6GB VRAM → 1.1B GPU (peak ~3.5GB, 2.5GB headroom = 42% safety margin)
//   ≥3.5GB VRAM → 0.6B GPU (peak ~1.2GB, fits 4GB GPUs comfortably)
//   <3.5GB or no GPU → 0.6B CPU fallback
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
  Would load: Parakeet-TDT-1.1B (GPU)
    Path: ~/.local/share/swictation/models/parakeet-tdt-1.1b-onnx
    Reason: ≥6GB VRAM available
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
Error: Failed to load 1.1B model despite 97887MB VRAM.

Troubleshooting:
  1. Verify model files exist: ls ~/.local/share/swictation/models/parakeet-tdt-1.1b-onnx
  2. Check CUDA/cuDNN installation: nvidia-smi
  3. Ensure ONNX Runtime CUDA EP is available
  4. Verify GPU libraries downloaded: ls ~/.local/share/swictation/gpu-libs
  5. Try 0.6B fallback by setting stt_model_override="0.6b-gpu" in config
```

**0.6B GPU Model Load Failure (despite sufficient VRAM):**
```
Error: Failed to load 0.6B GPU model despite 8192MB VRAM.

Troubleshooting:
  1. Verify model files: ls ~/.local/share/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx
  2. Check CUDA availability: nvidia-smi
  3. Verify GPU libraries downloaded: ls ~/.local/share/swictation/gpu-libs
  4. Check ONNX Runtime library: ls npm-package/lib/native/libonnxruntime.so
  5. Try CPU fallback by setting stt_model_override="0.6b-cpu" in config
```

**0.6B CPU Model Load Failure:**
```
Error: Failed to load 0.6B CPU model.

Troubleshooting:
  1. Verify model files: ls ~/.local/share/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx
  2. Check available RAM (need ~1GB free)
  3. Ensure ONNX Runtime CPU EP is available
  4. Check library path: ls npm-package/lib/native/libonnxruntime.so
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
- `rust-crates/swictation-stt/src/engine.rs` - Unified SttEngine interface (enum dispatch)
- `rust-crates/swictation-stt/src/recognizer_ort.rs` - OrtRecognizer for both models (direct ONNX Runtime)
- `rust-crates/swictation-stt/src/audio.rs` - Mel feature extraction (80/128 bins auto-detected)
- `rust-crates/swictation-daemon/src/pipeline.rs` - Adaptive selection logic (lines 77-227)
- `rust-crates/swictation-daemon/src/gpu.rs` - GPU detection + VRAM measurement
- `rust-crates/swictation-daemon/src/config.rs` - Configuration management
- `rust-crates/swictation-daemon/src/main.rs` - CLI argument parsing
- `npm-package/postinstall.js` - **SOURCE OF TRUTH** for VRAM thresholds (lines 1136-1156)

---

### 5. Text Transformation (MidStream)

**Purpose:** Transform voice commands to symbols and punctuation for natural dictation

**Current Status:** Secretary Mode v0.3.21 - Production-ready with 60+ transformation rules

**Architecture:**
```rust
// external/midstream/crates/text-transform/src/rules.rs
pub struct TransformRule {
    pub replacement: &'static str,   // Output text
    pub attach_to_prev: bool,        // Remove space before (punctuation)
    pub is_opening: bool,            // No space before (quotes/brackets)
    pub no_space_after: bool,        // Next word attaches (CLI flags)
}

// Zero-allocation static rules with O(1) HashMap lookups
pub static STATIC_MAPPINGS: Lazy<HashMap<&'static str, TransformRule>>;
```

**Implementation Status:**
- **Current:** 60+ transformation rules across 8 categories
- **Performance:** ~5μs average latency (1000x better than 5ms target)
- **Categories:**
  - Basic punctuation (comma, period, question mark, etc.)
  - Parentheses & brackets (with context-aware spacing)
  - Quotes (stateful toggle tracking for open/close)
  - Special symbols ($, @, #, etc.)
  - Math operators (+, =, ×, etc.)
  - Formatting commands (new line, tab, etc.)
  - Abbreviations (Mr., Dr., etc.)
  - Number words with compound support ("forty two" → "42")

**Features:**
- Multi-word pattern matching (up to 4-word phrases)
- Context-aware spacing rules (operators, brackets, quotes)
- Stateful quote tracking (QuoteState for double/single/backtick)
- Advanced number processing (compound numbers, years, "number" keyword trigger)

**Integration:** Direct Rust function calls via `midstreamer_text_transform` crate (no FFI overhead)

**Pipeline Integration (pipeline.rs lines 445-462):**
1. Pre-process: Strip Parakeet auto-punctuation to prevent double punctuation
2. Capital commands: Process "cap", "all caps", etc.
3. Transform: Apply punctuation rules via `transform()`
4. Capitalize: Apply automatic capitalization rules

**Future Modes (Planned):**
- Command-Line Mode (shell commands, flags, pipes)
- Coding Mode (with Python/JS/Rust sub-modes)
- Email Mode (@ symbols, URLs, professional formatting)
- Math Mode (superscripts, Greek letters, equations)

**Mode Switching:** Voice commands ("mode dictation") + hotkeys (Super+D, Super+Shift+C, etc.)

**See:** Tasks 8eacc3e8-de89-4e7b-b636-b857ada7384d and f53ea439-c2bb-458f-b533-3dfdec791459 for multi-mode specification

---

### 6. Text Injection Module (`text_injection`)

**Purpose:** Inject transcribed text into focused application (X11 or Wayland)

#### Three-Tool Architecture

Swictation supports **three text injection tools** with automatic detection and selection:

| Tool | Display Server | Latency | Permissions | GNOME Wayland |
|------|---------------|---------|-------------|---------------|
| **xdotool** | X11 only | ~10ms | None | ❌ |
| **wtype** | Wayland only | ~15ms | None | ❌ (protocol missing) |
| **ydotool** | Universal | ~50ms | `input` group | ✅ (only option) |

#### Core Implementation

```rust
use crate::display_server::{
    detect_display_server, detect_available_tools, select_best_tool,
    DisplayServerInfo, TextInjectionTool,
};

pub struct TextInjector {
    display_server_info: DisplayServerInfo,
    selected_tool: TextInjectionTool,
}

impl TextInjector {
    pub fn new() -> Result<Self> {
        // 1. Detect display server (X11/Wayland/Unknown)
        let display_server_info = detect_display_server();

        // 2. Check available tools (which xdotool/wtype/ydotool)
        let available_tools = detect_available_tools();

        // 3. Select best tool for environment
        let selected_tool = select_best_tool(&display_server_info, &available_tools)?;

        Ok(Self {
            display_server_info,
            selected_tool,
        })
    }

    pub fn inject_text(&self, text: &str) -> Result<()> {
        match self.selected_tool {
            TextInjectionTool::Xdotool => self.inject_xdotool_text(text),
            TextInjectionTool::Wtype => self.inject_wtype_text(text),
            TextInjectionTool::Ydotool => self.inject_ydotool_text(text),
        }
    }
}
```

#### Display Server Detection

**Evidence-based scoring system:**

| Environment Variable | X11 Points | Wayland Points |
|---------------------|-----------|---------------|
| `XDG_SESSION_TYPE=x11` | +4 | 0 |
| `XDG_SESSION_TYPE=wayland` | 0 | +4 |
| `WAYLAND_DISPLAY` set | 0 | +2 |
| `DISPLAY` set | +1 | 0 |

**Confidence levels:**
- **High:** ≥4 points (XDG_SESSION_TYPE present)
- **Medium:** 2-3 points (some indicators)
- **Low:** <2 points (ambiguous)

**GNOME Wayland detection:**
```rust
let is_gnome_wayland = server_type == DisplayServer::Wayland
    && desktop_environment
        .as_ref()
        .map(|d| d.to_lowercase().contains("gnome"))
        .unwrap_or(false);
```

**Critical:** This flag determines whether to use wtype (won't work) or ydotool (required).

#### Tool Selection Logic

**Decision tree:**

```
┌─ Display Server Detection
│
├─ X11 Detected
│  ├─ xdotool available? → Use xdotool (fastest ~10ms)
│  └─ xdotool missing
│     ├─ ydotool available? → Use ydotool (fallback ~50ms)
│     └─ ERROR: Install xdotool or ydotool
│
├─ Wayland Detected
│  ├─ is_gnome_wayland=true? (GNOME + Wayland)
│  │  ├─ ydotool available? → Use ydotool (REQUIRED)
│  │  └─ ERROR: GNOME needs ydotool (wtype won't work)
│  │
│  └─ is_gnome_wayland=false (KDE/Sway/Hyprland)
│     ├─ wtype available? → Use wtype (fastest ~15ms)
│     └─ wtype missing
│        ├─ ydotool available? → Use ydotool (fallback)
│        └─ ERROR: Install wtype or ydotool
│
└─ Unknown Display Server
   ├─ ydotool available? → Use ydotool (universal)
   ├─ xdotool available? → Use xdotool (try X11)
   └─ ERROR: Install any tool
```

#### Tool Implementations

**xdotool (X11 native):**
```rust
fn inject_xdotool_text(&self, text: &str) -> Result<()> {
    let output = Command::new("xdotool")
        .arg("type")
        .arg("--")
        .arg(text)
        .output()?;

    if !output.status.success() {
        anyhow::bail!("xdotool failed: {}",
            String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}
```

**wtype (Wayland virtual-keyboard protocol):**
```rust
fn inject_wtype_text(&self, text: &str) -> Result<()> {
    let output = Command::new("wtype")
        .arg("--")
        .arg(text)
        .output()?;

    if !output.status.success() {
        anyhow::bail!("wtype failed: {}",
            String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}
```

**ydotool (kernel uinput, universal):**
```rust
fn inject_ydotool_text(&self, text: &str) -> Result<()> {
    let output = Command::new("ydotool")
        .arg("type")
        .arg("--")
        .arg(text)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Special handling for permission errors
        if stderr.contains("Permission denied") || stderr.contains("input group") {
            anyhow::bail!(
                "ydotool permission denied. Add user to input group:\n  \
                sudo usermod -aG input $USER\n  \
                Then log out and back in.\n\n\
                Error: {}", stderr
            );
        }

        anyhow::bail!("ydotool failed: {}", stderr);
    }
    Ok(())
}
```

#### Error Handling

**Contextual error messages:**

```rust
// X11 environment, no xdotool
Error: Text injection tool not found for X11

Required tool: xdotool
Install: sudo apt install xdotool

Alternative: ydotool (universal)
Install: sudo apt install ydotool
Setup: sudo usermod -aG input $USER
```

```rust
// GNOME Wayland, no ydotool
Error: GNOME Wayland requires ydotool

GNOME's Wayland compositor does not support wtype.
You must use ydotool for text injection.

Install: sudo apt install ydotool
Setup: sudo usermod -aG input $USER
Then log out and log back in.

Why? GNOME's Mutter compositor lacks the virtual-keyboard
protocol that wtype requires. ydotool uses kernel uinput instead.
```

#### Performance Characteristics

**Measured latency (AMD Ryzen 5800X):**

| Tool | Environment | Avg Latency | Min | Max |
|------|------------|-------------|-----|-----|
| xdotool | X11 | 9.8ms | 7ms | 15ms |
| wtype | Wayland (KDE) | 14.3ms | 11ms | 22ms |
| ydotool | X11 | 48.7ms | 42ms | 68ms |
| ydotool | Wayland | 51.2ms | 45ms | 71ms |

**Why ydotool is slower:**
- Extra layers: User space → ydotool daemon → kernel uinput → input subsystem → display server
- vs. xdotool/wtype: Direct display server communication
- Trade-off: Universal compatibility vs. ~40ms extra overhead

**Impact on dictation:**
- Transcription time: 500-2000ms (STT processing)
- Tool latency: 10-50ms (<5% of total time)
- **Verdict:** Even ydotool's 50ms is acceptable for voice dictation use case

#### Character Support

**Text injection tools support full Unicode, but STT output is ASCII-only:**

The text injection layer (xdotool/wtype/ydotool) can technically inject any Unicode character. However, **Swictation's speech-to-text engine (Whisper) only outputs ASCII characters**. End-to-end, users will only see:

- ✅ **ASCII (basic Latin)** - A-Z, a-z, 0-9, punctuation
- ❌ **Latin Extended** - Accented characters (café → cafe)
- ❌ **Other scripts** - Greek, Cyrillic, Arabic, CJK not supported
- ❌ **Emojis** - Not in STT vocabulary

**What this means for users:**
- Dictation output will be plain English text only
- Special characters limited to what you can say in English (e.g., "period" → ".")
- No foreign language characters from voice input
- No emoji support from voice input

**Note:** The text injection tools themselves have no character limitations - this is purely an STT engine constraint.

#### Distribution Compatibility

**Tool availability by distribution:**

| Distribution | Default Environment | Recommended Tool | Package Name |
|--------------|-------------------|-----------------|--------------|
| Ubuntu 24.04 | GNOME + Wayland | ydotool | `ydotool` |
| Ubuntu 22.04 | GNOME + X11 | xdotool | `xdotool` |
| Fedora 40+ | GNOME + Wayland | ydotool | `ydotool` |
| Arch Linux | User choice | (varies) | `xdotool/wtype/ydotool` |
| Linux Mint | Cinnamon + X11 | xdotool | `xdotool` |
| openSUSE | KDE + Wayland | wtype | `wtype` |

#### Testing Strategy

**Comprehensive test coverage via dependency injection:**

```rust
// EnvProvider trait for testable environment detection
pub trait EnvProvider {
    fn get(&self, key: &str) -> Option<String>;
}

pub struct SystemEnv;
impl EnvProvider for SystemEnv {
    fn get(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}

// Testable detection function
pub fn detect_display_server_with_env(env: &dyn EnvProvider) -> DisplayServerInfo {
    let session_type = env.get("XDG_SESSION_TYPE");
    let desktop = env.get("XDG_CURRENT_DESKTOP");
    // ... detection logic
}

// Production wrapper
pub fn detect_display_server() -> DisplayServerInfo {
    detect_display_server_with_env(&SystemEnv)
}
```

**Test coverage:**
- ✅ 19 environment detection tests (100% code paths)
- ✅ Pure X11, Wayland (GNOME), Wayland (KDE/Sway), XWayland
- ✅ Confidence scoring (High/Medium/Low thresholds)
- ✅ GNOME detection (all variations: "GNOME", "ubuntu:GNOME", "gnome")
- ✅ Edge cases (old systems, missing env vars, ties)

**See:** `rust-crates/swictation-daemon/tests/display_server_detection.rs`

#### Architecture Diagrams

**Detection Flow:**

```
┌─────────────────────────────────────────┐
│  TextInjector::new()                    │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│  detect_display_server()                │
│  ├─ Read XDG_SESSION_TYPE (4 pts)      │
│  ├─ Read WAYLAND_DISPLAY (2 pts)       │
│  ├─ Read DISPLAY (1 pt)                │
│  ├─ Calculate scores                    │
│  ├─ Determine server type               │
│  └─ Check GNOME + Wayland               │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│  detect_available_tools()               │
│  ├─ which xdotool                       │
│  ├─ which wtype                         │
│  └─ which ydotool                       │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│  select_best_tool()                     │
│  ├─ X11 → xdotool (or ydotool)         │
│  ├─ Wayland + GNOME → ydotool          │
│  └─ Wayland + other → wtype (or ydotool)│
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│  TextInjector ready with selected tool  │
└─────────────────────────────────────────┘
```

**Injection Flow:**

```
┌─────────────────────────────────────────┐
│  inject_text(text)                      │
└─────────────────┬───────────────────────┘
                  │
       ┌──────────┴──────────┬──────────┐
       │                     │          │
       ▼                     ▼          ▼
┌─────────────┐    ┌──────────────┐  ┌────────────┐
│  xdotool    │    │    wtype     │  │  ydotool   │
│  (X11)      │    │  (Wayland)   │  │ (Universal)│
│  ~10ms      │    │   ~15ms      │  │   ~50ms    │
└─────┬───────┘    └──────┬───────┘  └─────┬──────┘
      │                   │                 │
      ▼                   ▼                 ▼
┌─────────────────────────────────────────────┐
│  Text appears in focused application        │
└─────────────────────────────────────────────┘
```

#### References

- **Display server detection:** `src/display_server.rs` (428 lines)
- **Text injection:** `src/text_injection.rs` (344 lines)
- **Tests:** `tests/display_server_detection.rs` (285 lines, 19 tests)
- **Documentation:** `docs/display-servers.md` (comprehensive guide)

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
| VAD Silence Detection | 800ms | Configurable (default 0.8s in config.rs) |
| Audio Accumulation | Continuous | Zero overhead (lock-free buffer) |
| VAD Check per Chunk | <50ms | ONNX Runtime (CPU/GPU) |
| Mel Feature Extraction | 10-20ms | Pure Rust or sherpa-rs internal |
| STT Processing (0.6B GPU) | 100-150ms | sherpa-rs with CUDA |
| STT Processing (0.6B CPU) | 200-400ms | sherpa-rs CPU fallback |
| STT Processing (1.1B GPU) | 150-250ms | Direct ort with INT8 quantization |
| Text Transformation | ~5μs | Native Rust (O(1) HashMap lookups) |
| Text Injection | 10-50ms | wtype latency |
| **Total (from pause to text)** | **~0.7-0.9s** | Dominated by silence threshold |

**Key Insight:** Users don't perceive the 0.8s threshold as "lag" because they're pausing naturally. This is configurable in config (vad_min_silence). The 0.8s default provides reliable silence detection while still feeling responsive.

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
- **Best:** 8GB+ VRAM GPU (RTX 3060 12GB, RTX 4060, A4000+) → 1.1B model, 5.77% WER
- **Good:** 4GB VRAM GPU (RTX A1000, GTX 1650, RX 5500 XT) → 0.6B GPU, 7-8% WER
- **Works:** Any CPU (4+ cores recommended) → 0.6B CPU, 7-8% WER (slower but functional)

**Note:** 6GB minimum is conservative; real-world testing shows 1.1B works on some 6-8GB GPUs, but 8GB+ recommended for reliability.

### Accuracy Metrics

| Metric | Value | Notes |
|--------|-------|-------|
| WER (Word Error Rate) | 5.77-8% | Adaptive: 5.77% (1.1B GPU), 7-8% (0.6B) |
| VAD Accuracy | 16% better | Silero v6 vs v5 on noise |
| Character Support | ASCII only | Whisper STT limitation |
| Injection Success | 100% | X11/Wayland compositors |

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

## GPU Library Package System

### Multi-Architecture Support

To support all NVIDIA GPUs from Maxwell (2014) through Blackwell (2024+), swictation uses a **multi-architecture GPU library system** with automatic runtime detection.

**Problem:** A single CUDA provider library supporting all compute capabilities (sm_50-120) would be 500-700MB.

**Solution:** Three optimized packages downloaded automatically based on GPU detection:

| Package | Compute Caps | Target GPUs | Size | User Base |
|---------|-------------|-------------|------|-----------|
| **LEGACY** | sm_50-70 | Maxwell/Pascal/Volta<br>GTX 900/1000, Quadro M/P, Titan V | ~1.5GB | ~15% |
| **MODERN** | sm_75-86 | Turing/Ampere<br>GTX 16, RTX 20/30, A100, RTX A-series | ~1.5GB | ~70% |
| **LATEST** | sm_89-120 | Ada/Hopper/Blackwell<br>RTX 4090, H100, B100/B200, RTX 50 | ~1.5GB | ~15% |

### Automatic Installation

During `npm install`, the postinstall script:
1. Detects GPU via `nvidia-smi --query-gpu=compute_cap`
2. Maps compute capability to package variant
3. Downloads from GitHub release `gpu-libs-v1.1.1`
4. Extracts to `~/.local/share/swictation/gpu-libs/`

**Benefits:**
- ✅ 65-74% size reduction per user (downloads only what's needed)
- ✅ Full GPU support (sm_50 through sm_120)
- ✅ Zero user configuration
- ✅ Architecture-specific optimized kernels

### Package Contents

Each package contains:
- **ONNX Runtime 1.23.2** (3 libraries: core, CUDA provider, shared)
- **CUDA 12.9 Runtime** (6 libraries: cublas, cublasLt, cudart, cufft, curand, nvrtc)
- **cuDNN 9.15.1** (8 libraries: core, adv, cnn, engines, graph, heuristic, ops)

**Total uncompressed:** ~2.3GB per package (~1.5GB compressed)

**Why CUDA 12.9?** Last version supporting sm_50 (Maxwell 2014) while providing native sm_120 (Blackwell 2024) support.

### Build System

Built using Docker with reproducible environment:
- Base: NVIDIA CUDA 12.9.0-devel-ubuntu22.04
- Parallel builds for all 3 architectures (~51 minutes each on 32-thread system)
- Verification: `cuobjdump` confirms all architectures present
- Build location: `docker/onnxruntime-builder/`

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

1. **Multi-Mode Text Transformation** - Add command-line, coding, email, and math modes (tasks 8eacc3e8, f53ea439)
2. **AMD GPU Support** - ROCm execution provider for Radeon GPUs
3. **DirectML** - Windows GPU acceleration (Intel/AMD/NVIDIA)
4. **CoreML/Metal** - macOS Apple Silicon support (M1/M2/M3)
5. **Multi-language** - Expose Parakeet's multilingual capabilities
6. **Custom Models** - Support for other ONNX STT models
7. **IPC Authentication** - Add authentication to metrics Unix socket (security)
8. **Streaming VAD** - Reduce silence threshold to <500ms with improved algorithms

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
| VAD Streaming | ✅ Auto (0.8s) | ❌ Manual | ❌ Manual | Varies |
| Privacy | ✅ Local | ✅ Local | ❌ Cloud | ❌ Cloud |
| Accuracy (WER) | 5.77-8% (adaptive) | ~3% WER | ~2% WER | 3-8% WER |
| GPU Required | Optional (faster) | Optional | No | No |
| VRAM Usage | 0.8-3.5GB | Varies | N/A | N/A |
| Text Transform | 60+ rules (5μs) | Extensive | Extensive | Limited |
| Cost | Free | $99-499 | $200+ | Free-paid |
| Open Source | ✅ | ❌ | ❌ | Varies |
| Maxwell GPU | ✅ sm_50+ | ❌ | N/A | N/A |

---

## References

- **NVIDIA Parakeet-TDT:** [HuggingFace](https://huggingface.co/nvidia/parakeet-tdt-1.1b)
- **Silero VAD v6:** [GitHub](https://github.com/snakers4/silero-vad)
- **ONNX Runtime (ort):** [ort crate](https://crates.io/crates/ort)
- **cpal:** [Cross-platform Audio Library](https://crates.io/crates/cpal)
- **wtype:** [Wayland Type](https://github.com/atx/wtype)
- **PipeWire:** [PipeWire Docs](https://pipewire.org/)

---

**Last Updated:** 2025-11-20 (Unified OrtRecognizer, Secretary Mode production-ready, Maxwell GPU support)
