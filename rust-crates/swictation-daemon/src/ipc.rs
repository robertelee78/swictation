//! Unix socket IPC server for toggle commands

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tracing::{debug, error, info};

use crate::Daemon;

/// IPC command
#[derive(Debug)]
enum IpcCommand {
    Toggle,
    Status,
    Quit,
}

impl IpcCommand {
    fn parse(s: &str) -> Result<Self> {
        match s.trim().to_lowercase().as_str() {
            "toggle" => Ok(Self::Toggle),
            "status" => Ok(Self::Status),
            "quit" | "exit" | "shutdown" => Ok(Self::Quit),
            _ => anyhow::bail!("Unknown command: {}", s),
        }
    }
}

/// Unix socket IPC server
pub struct IpcServer {
    listener: UnixListener,
    daemon: Arc<Daemon>,
}

impl IpcServer {
    /// Create new IPC server
    pub fn new(socket_path: &str, daemon: Arc<Daemon>) -> Result<Self> {
        // Remove existing socket if it exists
        let _ = std::fs::remove_file(socket_path);

        let listener = UnixListener::bind(socket_path)
            .context("Failed to bind Unix socket")?;

        info!("IPC server listening on {}", socket_path);

        Ok(Self { listener, daemon })
    }

    /// Accept next IPC connection
    pub async fn accept(&mut self) -> Result<(UnixStream, Arc<Daemon>)> {
        let (stream, _) = self.listener.accept().await
            .context("Failed to accept connection")?;
        Ok((stream, self.daemon.clone()))
    }
}

/// Handle a single IPC connection
pub async fn handle_connection(mut stream: UnixStream, daemon: Arc<Daemon>) -> Result<()> {
    let mut buffer = [0u8; 1024];
    let n = stream.read(&mut buffer).await?;

    if n == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..n]);
    debug!("Received IPC command: {}", request.trim());

    let response = match IpcCommand::parse(&request) {
        Ok(IpcCommand::Toggle) => {
            match daemon.toggle().await {
                Ok(msg) => msg,
                Err(e) => format!("Error: {}", e),
            }
        }
        Ok(IpcCommand::Status) => {
            daemon.status().await
        }
        Ok(IpcCommand::Quit) => {
            info!("Received quit command");
            std::process::exit(0);
        }
        Err(e) => {
            format!("Error: {}", e)
        }
    };

    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;

    Ok(())
}
