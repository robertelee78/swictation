//! Sherpa-RS based speech recognition for Parakeet-TDT models
//!
//! This module provides a clean wrapper around sherpa-rs for Parakeet-TDT inference.

use crate::error::{Result, SttError};
use sherpa_rs::transducer::{TransducerConfig, TransducerRecognizer};
use std::path::Path;

/// Recognition result
#[derive(Debug, Clone)]
pub struct RecognitionResult {
    /// Transcribed text
    pub text: String,
    /// Confidence score (0.0 to 1.0) - sherpa-rs doesn't provide this yet
    pub confidence: f32,
    /// Processing time in milliseconds
    pub processing_time_ms: f64,
}

/// Speech recognizer using sherpa-rs
pub struct Recognizer {
    /// sherpa-rs transducer recognizer
    recognizer: TransducerRecognizer,
    /// Sample rate (always 16000 for Parakeet-TDT)
    sample_rate: u32,
}

impl Recognizer {
    /// Create new recognizer from model directory
    ///
    /// # Arguments
    ///
    /// * `model_path` - Path to directory containing encoder.int8.onnx, decoder.int8.onnx, joiner.int8.onnx, tokens.txt
    /// * `use_gpu` - Enable CUDA GPU acceleration
    ///
    /// # Example
    ///
    /// ```no_run
    /// use swictation_stt::Recognizer;
    ///
    /// let recognizer = Recognizer::new(
    ///     "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8",
    ///     true  // use GPU
    /// ).unwrap();
    /// ```
    pub fn new<P: AsRef<Path>>(model_path: P, use_gpu: bool) -> Result<Self> {
        let model_path = model_path.as_ref();

        // Auto-detect model quantization format (fp32, fp16, int8)
        let find_model_file = |base_name: &str| -> Result<std::path::PathBuf> {
            // Try fp16 first (best for GPU), then fp32, then int8
            for suffix in ["fp16.onnx", "onnx", "fp32.onnx", "int8.onnx"] {
                let path = model_path.join(format!("{}.{}", base_name, suffix));
                if path.exists() {
                    return Ok(path);
                }
            }
            Err(SttError::model_load(format!(
                "No {} model file found (tried fp16, fp32, int8 in {})",
                base_name,
                model_path.display()
            )))
        };

        let encoder_path = find_model_file("encoder")?;
        let decoder_path = find_model_file("decoder")?;
        let joiner_path = find_model_file("joiner")?;
        let tokens_path = model_path.join("tokens.txt");

        if !tokens_path.exists() {
            return Err(SttError::model_load(format!(
                "Missing tokens.txt file: {}",
                tokens_path.display()
            )));
        }

        // Configure sherpa-rs transducer
        let config = TransducerConfig {
            encoder: encoder_path.to_str().unwrap().to_string(),
            decoder: decoder_path.to_str().unwrap().to_string(),
            joiner: joiner_path.to_str().unwrap().to_string(),
            tokens: tokens_path.to_str().unwrap().to_string(),

            // Performance settings
            num_threads: if use_gpu { 1 } else { 4 },
            sample_rate: 16_000,
            feature_dim: 80,

            // Model type for Parakeet-TDT (NeMo transducer)
            model_type: "nemo_transducer".to_string(),

            // GPU provider: CUDA for offline recognition (TensorRT only supports online/streaming)
            // Note: CUDA EP may not fully utilize int8 quantization on Tensor Cores
            // but it's the only option for offline (file-based) recognition with GPU
            provider: Some(if use_gpu { "cuda" } else { "cpu" }.to_string()),

            debug: false,
            ..Default::default()
        };

        // Create recognizer
        let recognizer = TransducerRecognizer::new(config).map_err(|e| {
            SttError::model_load(format!("Failed to create sherpa-rs recognizer: {}", e))
        })?;

        tracing::info!(
            "✅ Loaded Parakeet-TDT model from {} (GPU: {})",
            model_path.display(),
            use_gpu
        );

        Ok(Self {
            recognizer,
            sample_rate: 16_000u32,
        })
    }

    /// Recognize speech from audio samples
    ///
    /// # Arguments
    ///
    /// * `audio` - Audio samples (16kHz, mono, f32)
    ///
    /// # Returns
    ///
    /// Recognition result with transcribed text
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use swictation_stt::Recognizer;
    /// # let mut recognizer = Recognizer::new("model_path", false).unwrap();
    /// let audio: Vec<f32> = vec![0.0; 16000]; // 1 second of silence
    /// let result = recognizer.recognize(&audio).unwrap();
    /// println!("Transcription: {}", result.text);
    /// ```
    pub fn recognize(&mut self, audio: &[f32]) -> Result<RecognitionResult> {
        let start = std::time::Instant::now();

        // sherpa-rs handles all preprocessing internally
        let text = self.recognizer.transcribe(self.sample_rate, audio);

        let processing_time_ms = start.elapsed().as_secs_f64() * 1000.0;

        Ok(RecognitionResult {
            text: text.trim().to_string(),
            confidence: 1.0, // sherpa-rs doesn't provide confidence scores yet
            processing_time_ms,
        })
    }

    /// Recognize speech from audio file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to WAV file (must be 16kHz mono)
    ///
    /// # Returns
    ///
    /// Recognition result with transcribed text
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use swictation_stt::Recognizer;
    /// # let mut recognizer = Recognizer::new("model_path", false).unwrap();
    /// let result = recognizer.recognize_file("audio.wav").unwrap();
    /// println!("Transcription: {}", result.text);
    /// ```
    pub fn recognize_file<P: AsRef<Path>>(&mut self, path: P) -> Result<RecognitionResult> {
        use sherpa_rs::read_audio_file;

        let start = std::time::Instant::now();

        // Read audio file (sherpa-rs handles format conversion)
        let (samples, sample_rate) =
            read_audio_file(path.as_ref().to_str().unwrap()).map_err(|e| {
                SttError::audio_processing(format!("Failed to read audio file: {}", e))
            })?;

        // Verify sample rate
        if sample_rate != 16_000 {
            return Err(SttError::audio_processing(format!(
                "Audio must be 16kHz, got {}Hz. Please resample your audio.",
                sample_rate
            )));
        }

        // Transcribe
        let text = self.recognizer.transcribe(sample_rate, &samples);

        let processing_time_ms = start.elapsed().as_secs_f64() * 1000.0;

        tracing::debug!(
            "Transcribed {} samples in {:.2}ms",
            samples.len(),
            processing_time_ms
        );

        Ok(RecognitionResult {
            text: text.trim().to_string(),
            confidence: 1.0,
            processing_time_ms,
        })
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires model files
    fn test_recognizer_creation() {
        let model_path = "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8";
        let recognizer = Recognizer::new(model_path, false);
        assert!(recognizer.is_ok());
    }

    #[test]
    fn test_recognition_result() {
        let result = RecognitionResult {
            text: "hello world".to_string(),
            confidence: 1.0,
            processing_time_ms: 100.0,
        };
        assert_eq!(result.text, "hello world");
        assert_eq!(result.confidence, 1.0);
    }
}
