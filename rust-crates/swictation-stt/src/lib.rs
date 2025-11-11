//! Swictation Speech-to-Text with Sherpa-RS
//!
//! Pure Rust STT using sherpa-rs with Parakeet-TDT models.
//!
//! ## Features
//!
//! - Sherpa-ONNX based inference (proven working)
//! - Parakeet-TDT 0.6B/1.1B model support
//! - Separate decoder/joiner architecture (native support)
//! - GPU acceleration via CUDA
//! - Pure Rust API
//!
//! ## Quick Start
//!
//! ```no_run
//! use swictation_stt::Recognizer;
//!
//! let mut recognizer = Recognizer::new(
//!     "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8",
//!     true // use GPU
//! )?;
//!
//! let result = recognizer.recognize_file("audio.wav")?;
//! println!("Transcription: {}", result.text);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod audio; // Audio processing for 1.1B model
pub mod engine; // Unified STT engine interface
pub mod error;
pub mod recognizer;
pub mod recognizer_ort; // Direct ONNX Runtime for 1.1B model

pub use audio::AudioProcessor;
pub use engine::SttEngine; // Unified STT engine enum
pub use error::{Result, SttError};
pub use recognizer::{RecognitionResult, Recognizer};
pub use recognizer_ort::OrtRecognizer;

/// Default model path
pub const DEFAULT_MODEL_PATH: &str =
    "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8";
