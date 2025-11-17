//! Unix socket IPC server for toggle commands

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tracing::{debug, info};

use crate::Daemon;

/// IPC command - JSON only
#[derive(Debug, serde::Deserialize)]
struct IpcCommand {
    action: String,
}

impl IpcCommand {
    fn parse(s: &str) -> Result<Self> {
        serde_json::from_str(s.trim())
            .context("Invalid JSON. Expected: {\"action\": \"toggle|status|quit\"}")
    }

    fn to_command_type(&self) -> Result<CommandType> {
        match self.action.to_lowercase().as_str() {
            "toggle" => Ok(CommandType::Toggle),
            "status" => Ok(CommandType::Status),
            "quit" | "exit" | "shutdown" => Ok(CommandType::Quit),
            _ => anyhow::bail!("Unknown action: {}", self.action),
        }
    }
}

#[derive(Debug)]
enum CommandType {
    Toggle,
    Status,
    Quit,
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

        // Set secure permissions (0600 = owner-only access)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let socket_path_buf = std::path::Path::new(socket_path);
            if socket_path_buf.exists() {
                let permissions = std::fs::Permissions::from_mode(0o600);
                std::fs::set_permissions(socket_path_buf, permissions)
                    .context("Failed to set socket permissions")?;
            }
        }

        info!("IPC server listening on {} (permissions: 0600)", socket_path);

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

    // Create JSON response
    let response = match IpcCommand::parse(&request) {
        Ok(cmd) => match cmd.to_command_type() {
            Ok(CommandType::Toggle) => {
                match daemon.toggle().await {
                    Ok(msg) => serde_json::json!({
                        "status": "success",
                        "message": msg
                    }),
                    Err(e) => serde_json::json!({
                        "status": "error",
                        "error": format!("{}", e)
                    }),
                }
            }
            Ok(CommandType::Status) => {
                let status = daemon.status().await;
                serde_json::json!({
                    "status": "success",
                    "state": status
                })
            }
            Ok(CommandType::Quit) => {
                info!("Received quit command");
                std::process::exit(0);
            }
            Err(e) => {
                serde_json::json!({
                    "status": "error",
                    "error": format!("{}", e)
                })
            }
        },
        Err(e) => {
            serde_json::json!({
                "status": "error",
                "error": format!("{}", e)
            })
        }
    };

    let response_str = serde_json::to_string(&response)?;
    stream.write_all(response_str.as_bytes()).await?;
    stream.flush().await?;

    Ok(())
}
