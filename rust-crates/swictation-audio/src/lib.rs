//! Swictation Audio Capture
//!
//! High-performance audio capture with zero-copy circular buffers for real-time dictation.
//!
//! ## Features
//!
//! - Zero-copy lock-free circular buffer
//! - Native PipeWire/ALSA integration via cpal
//! - Real-time resampling to 16kHz mono
//! - PyO3 bindings for Python integration
//! - Predictable sub-100μs callback latency
//!
//! ## Architecture
//!
//! ```text
//! Audio Device (cpal)
//!   │
//!   ├─> CircularBuffer (lock-free ringbuf)
//!   │     │
//!   │     ├─> Resampler (rubato) -> 16kHz mono
//!   │     │
//!   │     └─> Chunk callbacks (optional streaming mode)
//!   │
//!   └─> AudioCapture (Python API via PyO3)
//! ```

pub mod buffer;
pub mod capture;
pub mod error;
pub mod resampler;

#[cfg(feature = "pyo3-bindings")]
pub mod python;

pub use buffer::CircularBuffer;
pub use capture::AudioCapture;
pub use error::{AudioError, Result};
pub use resampler::Resampler;

#[cfg(feature = "pyo3-bindings")]
use pyo3::prelude::*;

/// Audio sample rate constant (16kHz for STT models)
pub const TARGET_SAMPLE_RATE: u32 = 16000;

/// Default audio blocksize (samples per callback)
pub const DEFAULT_BLOCKSIZE: usize = 1024;

/// Audio configuration
#[derive(Debug, Clone)]
pub struct AudioConfig {
    /// Target sample rate (default: 16000 Hz)
    pub sample_rate: u32,
    /// Number of channels (default: 1 = mono)
    pub channels: u16,
    /// Samples per callback (default: 1024)
    pub blocksize: usize,
    /// Buffer duration in seconds (default: 10.0)
    pub buffer_duration: f32,
    /// Device index (None = default device)
    pub device_index: Option<usize>,
    /// Enable streaming mode with chunk callbacks
    pub streaming_mode: bool,
    /// Chunk duration for streaming mode (seconds)
    pub chunk_duration: f32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: TARGET_SAMPLE_RATE,
            channels: 1,
            blocksize: DEFAULT_BLOCKSIZE,
            buffer_duration: 10.0,
            device_index: None,
            streaming_mode: false,
            chunk_duration: 1.0,
        }
    }
}

#[cfg(feature = "pyo3-bindings")]
#[pymodule]
fn swictation_audio(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<python::PyAudioCapture>()?;
    m.add_class::<python::PyAudioConfig>()?;
    Ok(())
}
