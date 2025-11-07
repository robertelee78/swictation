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

    #[error("Token decoding error: {0}")]
    TokenDecodingError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("ONNX Runtime error: {0}")]
    OnnxRuntime(#[from] ort::Error),

    #[error("Array shape error: {0}")]
    ShapeError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<ndarray::ShapeError> for SttError {
    fn from(err: ndarray::ShapeError) -> Self {
        SttError::ShapeError(err.to_string())
    }
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

    pub fn config<S: Into<String>>(msg: S) -> Self {
        Self::ConfigError(msg.into())
    }
}
