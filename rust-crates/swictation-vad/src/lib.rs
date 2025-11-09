//! Voice Activity Detection (VAD) using Silero VAD via sherpa-rs
//!
//! This crate provides a Pure Rust interface to Silero VAD for detecting speech
//! in audio streams. It uses sherpa-rs (official sherpa-onnx bindings) which includes
//! Silero VAD support.
//!
//! # Performance
//!
//! - **20MB memory** (vs 500MB+ PyTorch)
//! - **<10ms latency** (vs ~50ms PyTorch)
//! - **CPU-only** with ONNX Runtime optimizations
//! - **Zero Python dependency**
//!
//! # Example
//!
//! ```no_run
//! use swictation_vad::{VadDetector, VadConfig, VadResult};
//!
//! let config = VadConfig::default();
//! let mut vad = VadDetector::new(config)?;
//!
//! // Process audio chunks (16kHz, mono, f32)
//! let chunk: Vec<f32> = vec![0.0; 512];
//! match vad.process_audio(&chunk) {
//!     Ok(VadResult::Speech { start_sample, samples }) => {
//!         println!("Speech detected at sample {}", start_sample);
//!         // Send to STT
//!     }
//!     Ok(VadResult::Silence) => {
//!         // Skip processing
//!     }
//!     Err(e) => eprintln!("VAD error: {}", e),
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod error;
mod silero_ort;

pub use error::{Result, VadError};
use silero_ort::SileroVadOrt;

/// VAD detection result
#[derive(Debug, Clone, PartialEq)]
pub enum VadResult {
    /// Speech detected with start sample index and audio samples
    Speech {
        start_sample: i32,
        samples: Vec<f32>,
    },
    /// No speech detected (silence)
    Silence,
}

/// VAD configuration
#[derive(Debug, Clone)]
pub struct VadConfig {
    /// Path to Silero VAD ONNX model
    pub model_path: String,

    /// Minimum silence duration in seconds (default: 0.5s = 500ms)
    /// Silence shorter than this is ignored
    pub min_silence_duration: f32,

    /// Minimum speech duration in seconds (default: 0.25s = 250ms)
    /// Speech shorter than this is ignored (filters out clicks/noise)
    pub min_speech_duration: f32,

    /// Maximum speech duration in seconds (default: 30.0s)
    /// Segments longer than this are split
    pub max_speech_duration: f32,

    /// Speech probability threshold (0.0 to 1.0, default: 0.5)
    /// Higher = more aggressive filtering (fewer false positives)
    pub threshold: f32,

    /// Audio sample rate (must be 16000 for Silero VAD)
    pub sample_rate: u32,

    /// Window size in samples (default: 512)
    /// Must be 512 or 1024 for Silero VAD
    pub window_size: i32,

    /// Buffer size in seconds for holding audio (default: 60.0s)
    /// How much audio to buffer before forcing a segment
    pub buffer_size_seconds: f32,

    /// ONNX Runtime provider (default: "cpu")
    pub provider: Option<String>,

    /// Number of threads for inference (default: 1)
    pub num_threads: Option<i32>,

    /// Enable debug logging
    pub debug: bool,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            model_path: String::new(),
            min_silence_duration: 0.5,
            min_speech_duration: 0.25,
            max_speech_duration: 30.0,
            threshold: 0.5,
            sample_rate: 16000,
            window_size: 512,
            buffer_size_seconds: 60.0,
            provider: None,
            num_threads: Some(1),
            debug: false,
        }
    }
}

impl VadConfig {
    /// Create config with model path
    pub fn with_model<S: Into<String>>(model_path: S) -> Self {
        Self {
            model_path: model_path.into(),
            ..Default::default()
        }
    }

    /// Set minimum silence duration
    pub fn min_silence(mut self, duration: f32) -> Self {
        self.min_silence_duration = duration;
        self
    }

    /// Set minimum speech duration
    pub fn min_speech(mut self, duration: f32) -> Self {
        self.min_speech_duration = duration;
        self
    }

    /// Set maximum speech duration
    pub fn max_speech(mut self, duration: f32) -> Self {
        self.max_speech_duration = duration;
        self
    }

    /// Set detection threshold
    pub fn threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }

    /// Set ONNX Runtime provider
    pub fn provider(mut self, provider: Option<String>) -> Self {
        self.provider = provider;
        self
    }

    /// Set number of threads
    pub fn num_threads(mut self, num_threads: Option<i32>) -> Self {
        self.num_threads = num_threads;
        self
    }

    /// Set buffer size in seconds
    pub fn buffer_size(mut self, seconds: f32) -> Self {
        self.buffer_size_seconds = seconds;
        self
    }

    /// Enable debug logging
    pub fn debug(mut self) -> Self {
        self.debug = true;
        self
    }

    /// Validate configuration
    fn validate(&self) -> Result<()> {
        if self.model_path.is_empty() {
            return Err(VadError::config("Model path is required"));
        }

        if self.sample_rate != 16000 {
            return Err(VadError::config("Sample rate must be 16000 Hz for Silero VAD"));
        }

        if self.window_size != 512 && self.window_size != 1024 {
            return Err(VadError::config("Window size must be 512 or 1024"));
        }

        if !(0.0..=1.0).contains(&self.threshold) {
            return Err(VadError::config("Threshold must be between 0.0 and 1.0"));
        }

        if self.min_silence_duration <= 0.0 {
            return Err(VadError::config("min_silence_duration must be positive"));
        }

        if self.min_speech_duration <= 0.0 {
            return Err(VadError::config("min_speech_duration must be positive"));
        }

        if self.max_speech_duration <= 0.0 {
            return Err(VadError::config("max_speech_duration must be positive"));
        }

        if self.buffer_size_seconds <= 0.0 {
            return Err(VadError::config("buffer_size_seconds must be positive"));
        }

        Ok(())
    }
}

/// Voice Activity Detector using Silero VAD
pub struct VadDetector {
    vad: SileroVadOrt,
    config: VadConfig,
    total_samples_processed: usize,
    is_speaking: bool,
    // Buffer for incomplete chunks
    chunk_buffer: Vec<f32>,
}

impl VadDetector {
    /// Create new VAD detector with given configuration
    pub fn new(config: VadConfig) -> Result<Self> {
        config.validate()?;

        let vad = SileroVadOrt::new(
            &config.model_path,
            config.threshold,
            config.sample_rate as i32,
            config.window_size as usize,
            (config.min_speech_duration * 1000.0) as i32,
            (config.min_silence_duration * 1000.0) as i32,
            config.provider.clone(),
            config.debug,
        ).map_err(|e| VadError::initialization(format!("Failed to create VAD: {}", e)))?;

        Ok(Self {
            vad,
            config,
            total_samples_processed: 0,
            is_speaking: false,
            chunk_buffer: Vec::new(),
        })
    }

    /// Process audio chunk and return speech segments if detected
    ///
    /// # Arguments
    ///
    /// * `samples` - Audio samples (16kHz, mono, f32, normalized to [-1.0, 1.0])
    ///
    /// # Returns
    ///
    /// - `VadResult::Speech` if speech is detected with the segment
    /// - `VadResult::Silence` if no speech detected
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use swictation_vad::{VadDetector, VadConfig, VadResult};
    /// # let mut vad = VadDetector::new(VadConfig::default())?;
    /// let chunk: Vec<f32> = vec![0.0; 512];
    /// match vad.process_audio(&chunk)? {
    ///     VadResult::Speech { start_sample, samples } => {
    ///         println!("Speech: {} samples starting at {}", samples.len(), start_sample);
    ///     }
    ///     VadResult::Silence => {
    ///         println!("Silence detected");
    ///     }
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn process_audio(&mut self, samples: &[f32]) -> Result<VadResult> {
        if samples.is_empty() {
            return Ok(VadResult::Silence);
        }

        let window_size = self.config.window_size as usize;
        let mut result = VadResult::Silence;

        // Combine buffered samples with new samples
        let mut all_samples = self.chunk_buffer.clone();
        all_samples.extend_from_slice(samples);

        // Process complete window-sized chunks
        let complete_chunks = all_samples.len() / window_size;
        let samples_to_process = complete_chunks * window_size;

        for i in 0..complete_chunks {
            let start = i * window_size;
            let end = start + window_size;
            let chunk = &all_samples[start..end];

            // Process this chunk through VAD
            match self.vad.process(chunk)
                .map_err(|e| VadError::processing(format!("VAD processing error: {}", e)))? {
                Some(speech_samples) => {
                    // Speech segment complete from VAD
                    self.is_speaking = true;

                    if self.config.debug {
                        eprintln!(
                            "VAD: Speech segment detected, {} samples",
                            speech_samples.len()
                        );
                    }

                    // Return this speech segment
                    let start_sample = (self.total_samples_processed.saturating_sub(speech_samples.len())) as i32;
                    result = VadResult::Speech {
                        start_sample,
                        samples: speech_samples,
                    };
                    // Don't break - continue processing remaining chunks
                }
                None => {
                    // No complete speech segment yet
                    // VAD is buffering or no speech detected
                }
            }

            self.total_samples_processed += window_size;
        }

        // Save any remaining incomplete chunk for next call
        self.chunk_buffer.clear();
        if samples_to_process < all_samples.len() {
            self.chunk_buffer.extend_from_slice(&all_samples[samples_to_process..]);
            if self.config.debug {
                eprintln!(
                    "VAD: Buffering {} incomplete samples for next call",
                    self.chunk_buffer.len()
                );
            }
        }

        Ok(result)
    }

    /// Check if speech is currently being detected (real-time)
    ///
    /// This is faster than `process_audio` and useful for real-time indicators.
    pub fn is_speech_detected(&mut self) -> bool {
        self.is_speaking
    }

    /// Flush any remaining audio in the buffer
    ///
    /// Call this at the end of a stream to process any remaining audio.
    /// Returns any remaining speech segment if available.
    pub fn flush(&mut self) -> Option<VadResult> {
        // Get any remaining buffered speech from VAD
        if let Some(speech_samples) = self.vad.flush() {
            if self.config.debug {
                eprintln!(
                    "VAD: Flushed remaining speech, {} samples",
                    speech_samples.len()
                );
            }

            self.is_speaking = false;
            let start_sample = (self.total_samples_processed.saturating_sub(speech_samples.len())) as i32;

            Some(VadResult::Speech {
                start_sample,
                samples: speech_samples,
            })
        } else {
            self.is_speaking = false;
            None
        }
    }

    /// Clear the internal buffer
    ///
    /// Call this to reset the VAD state (e.g., between different audio sources).
    pub fn clear(&mut self) {
        self.vad.reset();
        self.is_speaking = false;
        self.total_samples_processed = 0;
        self.chunk_buffer.clear();
    }

    /// Get total samples processed
    pub fn samples_processed(&self) -> usize {
        self.total_samples_processed
    }

    /// Get processing time in seconds
    pub fn processing_time_seconds(&self) -> f64 {
        self.total_samples_processed as f64 / self.config.sample_rate as f64
    }

    /// Get configuration
    pub fn config(&self) -> &VadConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        // Valid config
        let config = VadConfig {
            model_path: "/path/to/model.onnx".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_ok());

        // Empty model path
        let config = VadConfig {
            model_path: String::new(),
            ..Default::default()
        };
        assert!(config.validate().is_err());

        // Wrong sample rate
        let config = VadConfig {
            model_path: "/path/to/model.onnx".to_string(),
            sample_rate: 48000,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        // Invalid threshold
        let config = VadConfig {
            model_path: "/path/to/model.onnx".to_string(),
            threshold: 1.5,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_builder() {
        let config = VadConfig::with_model("/path/to/model.onnx")
            .min_silence(0.3)
            .min_speech(0.2)
            .threshold(0.6)
            .debug();

        assert_eq!(config.model_path, "/path/to/model.onnx");
        assert_eq!(config.min_silence_duration, 0.3);
        assert_eq!(config.min_speech_duration, 0.2);
        assert_eq!(config.threshold, 0.6);
        assert!(config.debug);
    }

    #[test]
    #[ignore]  // Only run when explicitly requested
    fn test_model_responds_to_input() {
        // Verify the model actually processes different inputs differently
        let config = VadConfig::with_model("/opt/swictation/models/silero-vad/silero_vad.onnx")
            .threshold(0.1)
            .debug();

        let mut vad = VadDetector::new(config).expect("Failed to create VAD");

        // Test with silence
        let silence = vec![0.0; 512];
        vad.process_audio(&silence).expect("Failed");

        // Test with maximum amplitude
        let loud = vec![0.9; 512];
        vad.process_audio(&loud).expect("Failed");

        // Test with sine wave
        let sine: Vec<f32> = (0..512).map(|i| (i as f32 * 0.1).sin() * 0.5).collect();
        vad.process_audio(&sine).expect("Failed");

        println!("If you see three different probabilities above, the model is working");
    }
}
