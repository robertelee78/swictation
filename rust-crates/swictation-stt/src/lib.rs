//! Swictation Speech-to-Text
//!
//! Pure Rust STT using ONNX Runtime with Parakeet-TDT-0.6B-V3 model.
//!
//! ## Features
//!
//! - ONNX-based inference (no PyTorch dependency)
//! - Parakeet-TDT-0.6B-V3 RNN-T model (6.05% WER)
//! - 640MB INT8 model (fits in 2-3GB VRAM)
//! - Real-time streaming support
//! - Pure Rust API (no PyO3)
//!
//! ## Architecture
//!
//! ```text
//! Audio (16kHz mono) → Encoder → Decoder → Joiner → Text
//!        ↓                ↓         ↓         ↓
//!   From audio crate  ONNX INT8  ONNX INT8  ONNX INT8
//! ```

pub mod error;
pub mod features;
pub mod model;
pub mod recognizer;
pub mod tokens;

pub use error::{SttError, Result};
pub use model::{ModelConfig, ParakeetModel};
pub use recognizer::{RecognitionResult, Recognizer};
pub use tokens::TokenDecoder;

/// Default model path
pub const DEFAULT_MODEL_PATH: &str = "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8";

/// Model configuration for Parakeet-TDT-0.6B-V3
#[derive(Debug, Clone)]
pub struct SttConfig {
    /// Path to model directory
    pub model_path: String,
    /// Number of threads for ONNX Runtime
    pub num_threads: usize,
    /// Use GPU if available
    pub use_gpu: bool,
    /// Provider (cpu, cuda, tensorrt)
    pub provider: String,
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            model_path: DEFAULT_MODEL_PATH.to_string(),
            num_threads: 4,
            use_gpu: true,
            provider: "cpu".to_string(),
        }
    }
}
