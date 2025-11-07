//! Error types for VAD operations

use thiserror::Error;

/// Result type for VAD operations
pub type Result<T> = std::result::Result<T, VadError>;

/// VAD error types
#[derive(Error, Debug)]
pub enum VadError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Initialization error
    #[error("Initialization error: {0}")]
    Initialization(String),

    /// Processing error
    #[error("Processing error: {0}")]
    Processing(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl VadError {
    pub fn config<S: Into<String>>(msg: S) -> Self {
        Self::Config(msg.into())
    }

    pub fn initialization<S: Into<String>>(msg: S) -> Self {
        Self::Initialization(msg.into())
    }

    pub fn processing<S: Into<String>>(msg: S) -> Self {
        Self::Processing(msg.into())
    }
}
