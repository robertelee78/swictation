//! Error types for STT operations

use thiserror::Error;

pub type Result<T> = std::result::Result<T, SttError>;

#[derive(Error, Debug)]
pub enum SttError {
    #[error("Model loading error: {0}")]
    ModelLoadError(String),

    #[error("Inference error: {0}")]
    InferenceError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Audio processing error: {0}")]
    AudioProcessing(String),

    #[error("Audio loading error: {0}")]
    AudioLoadError(String),

    #[error("Feature extraction error: {0}")]
    FeatureExtractionError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl SttError {
    pub fn model_load<S: Into<String>>(msg: S) -> Self {
        Self::ModelLoadError(msg.into())
    }

    pub fn inference<S: Into<String>>(msg: S) -> Self {
        Self::InferenceError(msg.into())
    }

    pub fn invalid_input<S: Into<String>>(msg: S) -> Self {
        Self::InvalidInput(msg.into())
    }

    pub fn audio_processing<S: Into<String>>(msg: S) -> Self {
        Self::AudioProcessing(msg.into())
    }

    pub fn config<S: Into<String>>(msg: S) -> Self {
        Self::ConfigError(msg.into())
    }
}
