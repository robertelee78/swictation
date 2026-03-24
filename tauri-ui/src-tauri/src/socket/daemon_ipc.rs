//! Daemon IPC client for command socket communication.
//!
//! Connects to the daemon's Unix socket (`swictation.sock`) to send
//! toggle/status commands — matching the Linux Python tray approach.

use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tracing::debug;

/// Get the IPC socket path for daemon commands.
pub fn get_ipc_socket_path() -> String {
    swictation_paths::ipc_socket_path()
        .to_string_lossy()
        .to_string()
}

/// Daemon state as reported by the IPC socket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonState {
    Idle,
    Recording,
    Disconnected,
}

impl DaemonState {
    pub fn as_str(&self) -> &'static str {
        match self {
            DaemonState::Idle => "idle",
            DaemonState::Recording => "recording",
            DaemonState::Disconnected => "disconnected",
        }
    }
}

/// Send a JSON command to the daemon IPC socket and return the response.
async fn send_command(action: &str) -> Result<serde_json::Value, String> {
    let socket_path = get_ipc_socket_path();

    let mut stream = tokio::time::timeout(
        Duration::from_millis(500),
        UnixStream::connect(&socket_path),
    )
    .await
    .map_err(|_| "Connection timeout".to_string())?
    .map_err(|e| format!("Connection failed: {}", e))?;

    let cmd = serde_json::json!({"action": action});
    let cmd_bytes = serde_json::to_vec(&cmd).map_err(|e| e.to_string())?;

    stream
        .write_all(&cmd_bytes)
        .await
        .map_err(|e| format!("Write failed: {}", e))?;

    stream
        .flush()
        .await
        .map_err(|e| format!("Flush failed: {}", e))?;

    let mut buffer = vec![0u8; 1024];
    let n = tokio::time::timeout(Duration::from_millis(500), stream.read(&mut buffer))
        .await
        .map_err(|_| "Read timeout".to_string())?
        .map_err(|e| format!("Read failed: {}", e))?;

    if n == 0 {
        return Err("Empty response".to_string());
    }

    serde_json::from_slice(&buffer[..n]).map_err(|e| format!("Parse failed: {}", e))
}

/// Query the daemon's current state via IPC socket.
///
/// Returns `DaemonState::Disconnected` if the socket is unreachable.
pub async fn query_daemon_state() -> DaemonState {
    match send_command("status").await {
        Ok(resp) => {
            let state_str = resp
                .get("state")
                .and_then(|v| v.as_str())
                .unwrap_or("idle");
            debug!("Daemon state: {}", state_str);
            match state_str {
                "recording" => DaemonState::Recording,
                _ => DaemonState::Idle,
            }
        }
        Err(e) => {
            debug!("Daemon unreachable: {}", e);
            DaemonState::Disconnected
        }
    }
}

/// Send toggle command to the daemon via IPC socket.
///
/// Returns the new state after toggling, or an error.
pub async fn toggle_recording() -> Result<DaemonState, String> {
    let resp = send_command("toggle").await?;

    // The daemon toggle response includes state in the message
    // e.g. {"status": "success", "message": "Recording started"}
    let message = resp
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if message.to_lowercase().contains("started") || message.to_lowercase().contains("recording") {
        Ok(DaemonState::Recording)
    } else {
        Ok(DaemonState::Idle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_state_as_str() {
        assert_eq!(DaemonState::Idle.as_str(), "idle");
        assert_eq!(DaemonState::Recording.as_str(), "recording");
        assert_eq!(DaemonState::Disconnected.as_str(), "disconnected");
    }

    #[test]
    fn test_ipc_socket_path() {
        let path = get_ipc_socket_path();
        assert!(path.contains("swictation.sock"));
    }
}
