use thiserror::Error;

#[derive(Error, Debug)]
pub enum BroadcasterError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Socket path error: {0}")]
    SocketPath(String),

    #[error("Broadcaster not started")]
    NotStarted,

    #[error("Broadcaster already running")]
    AlreadyRunning,
}

pub type Result<T> = std::result::Result<T, BroadcasterError>;
