//! Error types for audio capture

use thiserror::Error;

pub type Result<T> = std::result::Result<T, AudioError>;

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Audio device error: {0}")]
    DeviceError(String),

    #[error("Audio stream error: {0}")]
    StreamError(String),

    #[error("Buffer overflow: tried to write {0} samples but only {1} available")]
    BufferOverflow(usize, usize),

    #[error("Buffer underflow: tried to read {0} samples but only {1} available")]
    BufferUnderflow(usize, usize),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Resampling error: {0}")]
    ResampleError(String),

    #[error("Not recording")]
    NotRecording,

    #[error("Already recording")]
    AlreadyRecording,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl AudioError {
    pub fn device<S: Into<String>>(msg: S) -> Self {
        Self::DeviceError(msg.into())
    }

    pub fn stream<S: Into<String>>(msg: S) -> Self {
        Self::StreamError(msg.into())
    }

    pub fn invalid_config<S: Into<String>>(msg: S) -> Self {
        Self::InvalidConfig(msg.into())
    }
}
