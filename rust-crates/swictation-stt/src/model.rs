//! ONNX model loading and configuration for Parakeet-TDT-0.6B-V3

use ort::session::{builder::GraphOptimizationLevel, Session};
use std::path::{Path, PathBuf};

use crate::error::{Result, SttError};
use crate::tokens::TokenDecoder;

/// Model configuration
#[derive(Debug, Clone)]
pub struct ModelConfig {
    /// Path to encoder model
    pub encoder_path: PathBuf,
    /// Path to decoder model
    pub decoder_path: PathBuf,
    /// Path to joiner model
    pub joiner_path: PathBuf,
    /// Path to tokens file
    pub tokens_path: PathBuf,
    /// Number of threads
    pub num_threads: usize,
    /// Execution provider
    pub provider: String,
}

impl ModelConfig {
    /// Create config from model directory
    pub fn from_directory<P: AsRef<Path>>(path: P) -> Result<Self> {
        let base = path.as_ref();

        if !base.exists() {
            return Err(SttError::config(format!(
                "Model directory does not exist: {}",
                base.display()
            )));
        }

        let encoder_path = base.join("encoder.int8.onnx");
        let decoder_path = base.join("decoder.int8.onnx");
        let joiner_path = base.join("joiner.int8.onnx");
        let tokens_path = base.join("tokens.txt");

        // Verify all files exist
        for (name, path) in [
            ("encoder", &encoder_path),
            ("decoder", &decoder_path),
            ("joiner", &joiner_path),
            ("tokens", &tokens_path),
        ] {
            if !path.exists() {
                return Err(SttError::config(format!(
                    "{} model file not found: {}",
                    name,
                    path.display()
                )));
            }
        }

        Ok(Self {
            encoder_path,
            decoder_path,
            joiner_path,
            tokens_path,
            num_threads: 4,
            provider: "cpu".to_string(),
        })
    }
}

/// Parakeet-TDT-0.6B-V3 ONNX model ensemble
pub struct ParakeetModel {
    /// Encoder session
    encoder: Session,
    /// Decoder session
    decoder: Session,
    /// Joiner session
    joiner: Session,
    /// Token decoder
    pub tokens: TokenDecoder,
    /// Configuration
    config: ModelConfig,
}

impl ParakeetModel {
    /// Load Parakeet model from directory
    pub fn from_directory<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config = ModelConfig::from_directory(path)?;
        Self::from_config(config)
    }

    /// Load model from configuration
    pub fn from_config(config: ModelConfig) -> Result<Self> {
        println!("\n=== Loading Parakeet-TDT-0.6B-V3 ONNX Model ===");
        println!("Encoder: {}", config.encoder_path.display());
        println!("Decoder: {}", config.decoder_path.display());
        println!("Joiner: {}", config.joiner_path.display());
        println!("Tokens: {}", config.tokens_path.display());

        // Load encoder
        println!("\nLoading encoder...");
        let encoder = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(config.num_threads)?
            .commit_from_file(&config.encoder_path)
            .map_err(|e| SttError::model_load(format!("Failed to load encoder: {}", e)))?;

        println!("✓ Encoder loaded");

        // Load decoder
        println!("Loading decoder...");
        let decoder = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(config.num_threads)?
            .commit_from_file(&config.decoder_path)
            .map_err(|e| SttError::model_load(format!("Failed to load decoder: {}", e)))?;

        println!("✓ Decoder loaded");

        // Load joiner
        println!("Loading joiner...");
        let joiner = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(config.num_threads)?
            .commit_from_file(&config.joiner_path)
            .map_err(|e| SttError::model_load(format!("Failed to load joiner: {}", e)))?;

        println!("✓ Joiner loaded");

        // Load tokens
        println!("Loading tokens...");
        let tokens = TokenDecoder::from_file(&config.tokens_path)?;

        println!("✓ Tokens loaded ({} tokens)", tokens.vocab_size());
        println!("\n=== Model Ready ===");

        Ok(Self {
            encoder,
            decoder,
            joiner,
            tokens,
            config,
        })
    }

    /// Get model configuration
    pub fn config(&self) -> &ModelConfig {
        &self.config
    }

    /// Get encoder input shape info
    pub fn encoder_info(&self) -> String {
        format!("Encoder: {} inputs, {} outputs",
                self.encoder.inputs.len(),
                self.encoder.outputs.len())
    }

    /// Get decoder input shape info
    pub fn decoder_info(&self) -> String {
        format!("Decoder: {} inputs, {} outputs",
                self.decoder.inputs.len(),
                self.decoder.outputs.len())
    }

    /// Get joiner input shape info
    pub fn joiner_info(&self) -> String {
        format!("Joiner: {} inputs, {} outputs",
                self.joiner.inputs.len(),
                self.joiner.outputs.len())
    }

    /// Get access to encoder session for direct inference
    pub fn encoder(&mut self) -> &mut Session {
        &mut self.encoder
    }

    /// Get access to decoder session for direct inference
    pub fn decoder(&mut self) -> &mut Session {
        &mut self.decoder
    }

    /// Get access to joiner session for direct inference
    pub fn joiner(&mut self) -> &mut Session {
        &mut self.joiner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_config_from_directory() {
        let config = ModelConfig::from_directory("/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8");

        if config.is_ok() {
            let cfg = config.unwrap();
            assert!(cfg.encoder_path.exists());
            assert!(cfg.decoder_path.exists());
            assert!(cfg.joiner_path.exists());
            assert!(cfg.tokens_path.exists());
        }
    }

    #[test]
    fn test_model_loading() {
        // Only run if model exists
        let model_path = "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8";
        if !Path::new(model_path).exists() {
            println!("Skipping model loading test - model not found");
            return;
        }

        let model = ParakeetModel::from_directory(model_path);
        assert!(model.is_ok(), "Model should load successfully");

        let model = model.unwrap();
        assert_eq!(model.tokens.vocab_size(), 8193, "Should have 8193 tokens");
    }
}
