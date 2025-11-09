//! Direct ONNX Runtime recognizer for Parakeet-TDT 1.1B model
//!
//! Bypasses sherpa-rs to work around the SessionOptions bug with external weights.
//! This implementation uses the `ort` crate directly for full control over ONNX Runtime.
//!
//! ## Environment Setup
//!
//! The `ort` crate requires ONNX Runtime 1.22+ for version 2.0.0-rc.10.
//! Set the `ORT_DYLIB_PATH` environment variable to point to a compatible library:
//!
//! ```bash
//! export ORT_DYLIB_PATH=/path/to/libonnxruntime.so.1.23.2
//! ```
//!
//! For example, with onnxruntime-gpu installed via pip:
//! ```bash
//! export ORT_DYLIB_PATH=$(python3 -c "import onnxruntime; import os; print(os.path.join(os.path.dirname(onnxruntime.__file__), 'capi/libonnxruntime.so.1.23.2'))")
//! ```

use crate::error::{Result, SttError};
use ort::{
    execution_providers as ep,
    session::{builder::GraphOptimizationLevel, Session},
};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Direct ONNX Runtime recognizer for 1.1B Parakeet-TDT model
pub struct OrtRecognizer {
    encoder: Session,
    decoder: Session,
    joiner: Session,
    tokens: Vec<String>,
    blank_id: i64,
    unk_id: i64,
    model_path: PathBuf,
}

impl OrtRecognizer {
    /// Create new recognizer from model directory
    ///
    /// # Arguments
    /// * `model_dir` - Path to directory containing encoder.onnx, decoder.onnx, joiner.onnx, tokens.txt
    /// * `use_gpu` - Enable CUDA execution provider
    ///
    /// # Example
    /// ```no_run
    /// use swictation_stt::recognizer_ort::OrtRecognizer;
    ///
    /// let recognizer = OrtRecognizer::new(
    ///     "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-1.1b-converted",
    ///     true
    /// )?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new<P: AsRef<Path>>(model_dir: P, use_gpu: bool) -> Result<Self> {
        let model_path = model_dir.as_ref().to_path_buf();

        info!("Loading 1.1B Parakeet-TDT model with direct ONNX Runtime");
        info!("Model directory: {}", model_path.display());

        // Load tokens
        let tokens = Self::load_tokens(&model_path)?;
        let blank_id = 0; // Parakeet-TDT uses blank_id=0
        let unk_id = 2;   // Parakeet-TDT uses unk_id=2

        debug!("Loaded {} tokens (blank={}, unk={})", tokens.len(), blank_id, unk_id);

        // Configure ONNX Runtime session options
        let mut session_builder = Session::builder()
            .map_err(|e| SttError::ModelLoadError(format!("Failed to create session builder: {}", e)))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| SttError::ModelLoadError(format!("Failed to set optimization level: {}", e)))?
            .with_intra_threads(4)
            .map_err(|e| SttError::ModelLoadError(format!("Failed to set intra threads: {}", e)))?;

        if use_gpu {
            info!("Enabling CUDA execution provider");
            session_builder = session_builder
                .with_execution_providers([
                    ep::CUDAExecutionProvider::default().build(),
                    ep::CPUExecutionProvider::default().build(),
                ])
                .map_err(|e| SttError::ModelLoadError(format!("Failed to set execution providers: {}", e)))?;
        } else {
            info!("Using CPU execution provider");
        }

        // Load the three ONNX models (external weights load automatically!)
        info!("Loading encoder.onnx...");
        let encoder_path = model_path.join("encoder.onnx");
        let encoder = session_builder
            .commit_from_file(&encoder_path)
            .map_err(|e| SttError::ModelLoadError(format!("Failed to load encoder: {}", e)))?;
        info!("✓ Encoder loaded (external weights automatically loaded)");

        info!("Loading decoder.onnx...");
        let decoder_path = model_path.join("decoder.onnx");
        let decoder = Session::builder()
            .map_err(|e| SttError::ModelLoadError(format!("Failed to create decoder session builder: {}", e)))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| SttError::ModelLoadError(format!("Failed to set decoder optimization: {}", e)))?
            .commit_from_file(&decoder_path)
            .map_err(|e| SttError::ModelLoadError(format!("Failed to load decoder: {}", e)))?;
        info!("✓ Decoder loaded");

        info!("Loading joiner.onnx...");
        let joiner_path = model_path.join("joiner.onnx");
        let joiner = Session::builder()
            .map_err(|e| SttError::ModelLoadError(format!("Failed to create joiner session builder: {}", e)))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| SttError::ModelLoadError(format!("Failed to set joiner optimization: {}", e)))?
            .commit_from_file(&joiner_path)
            .map_err(|e| SttError::ModelLoadError(format!("Failed to load joiner: {}", e)))?;
        info!("✓ Joiner loaded");

        Ok(Self {
            encoder,
            decoder,
            joiner,
            tokens,
            blank_id,
            unk_id,
            model_path,
        })
    }

    /// Load tokens from tokens.txt
    fn load_tokens(model_dir: &Path) -> Result<Vec<String>> {
        let tokens_path = model_dir.join("tokens.txt");
        let contents = fs::read_to_string(&tokens_path)
            .map_err(|e| SttError::ModelLoadError(
                format!("Failed to read tokens.txt: {}", e)
            ))?;

        Ok(contents.lines().map(|s| s.to_string()).collect())
    }

    /// Test encoder inference with dummy input
    ///
    /// This method is for validation purposes only - to prove the 1.1B model
    /// loads and runs successfully with the ort crate.
    ///
    /// # Returns
    /// Confirmation that inference completed
    pub fn test_encoder_inference(&mut self) -> Result<String> {
        info!("Running test inference with dummy input...");

        // Create dummy input using tuple form (shape, data)
        use ort::value::Tensor;

        // audio_signal: (batch=1, num_frames=80, num_features=128)
        let audio_signal_data: Vec<f32> = vec![0.0; 1 * 80 * 128];
        let audio_signal = Tensor::from_array((vec![1usize, 80, 128], audio_signal_data.into_boxed_slice()))
            .map_err(|e| SttError::InferenceError(format!("Failed to create audio_signal tensor: {}", e)))?;

        // length: (batch=1,)
        let length_data: Vec<i64> = vec![80];
        let length_tensor = Tensor::from_array((vec![1usize], length_data.into_boxed_slice()))
            .map_err(|e| SttError::InferenceError(format!("Failed to create length tensor: {}", e)))?;

        // Get output info before running
        let output_names: Vec<_> = self.encoder.outputs.iter().map(|o| o.name.clone()).collect();

        // Run encoder
        let _outputs = self.encoder.run(ort::inputs!["audio_signal" => audio_signal, "length" => length_tensor])
            .map_err(|e| SttError::InferenceError(format!("Encoder inference failed: {}", e)))?;

        Ok(format!("✅ Encoder inference successful! Outputs: {}", output_names.join(", ")))
    }

    /// Get model information
    pub fn model_info(&self) -> String {
        format!(
            "OrtRecognizer:\n  Model: {}\n  Tokens: {}\n  Blank ID: {}\n  UNK ID: {}",
            self.model_path.display(),
            self.tokens.len(),
            self.blank_id,
            self.unk_id
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires model files
    fn test_ort_recognizer_init() {
        let model_dir = "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-1.1b-converted";
        let recognizer = OrtRecognizer::new(model_dir, false);
        assert!(recognizer.is_ok());
    }
}
