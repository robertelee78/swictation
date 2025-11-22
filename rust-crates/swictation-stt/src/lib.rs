//! Swictation Speech-to-Text with ONNX Runtime
//!
//! Pure Rust STT using direct ONNX Runtime integration with Parakeet-TDT models.
//!
//! ## Features
//!
//! - Direct ONNX Runtime integration (ort 2.0)
//! - Parakeet-TDT 0.6B/1.1B model support
//! - RNN-T Transducer architecture
//! - GPU acceleration via CUDA
//! - CPU fallback support
//! - Pure Rust API
//!
//! ## Quick Start
//!
//! ```no_run
//! use swictation_stt::OrtRecognizer;
//!
//! let mut recognizer = OrtRecognizer::new(
//!     "/opt/swictation/models/parakeet-tdt-0.6b-v3-onnx",
//!     true // use GPU
//! )?;
//!
//! let result = recognizer.recognize_file("audio.wav")?;
//! println!("Transcription: {}", result);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod audio; // Audio processing (mel-spectrogram)
pub mod engine; // Unified STT engine interface
pub mod error;
pub mod recognizer_ort; // Direct ONNX Runtime implementation

pub use audio::AudioProcessor;
pub use engine::{RecognitionResult, SttEngine}; // Unified STT engine enum
pub use error::{Result, SttError};
pub use recognizer_ort::OrtRecognizer;

/// Default model path
pub const DEFAULT_MODEL_PATH: &str = "/opt/swictation/models/parakeet-tdt-0.6b-v3-onnx";
