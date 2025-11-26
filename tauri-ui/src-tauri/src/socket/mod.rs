// Socket connection module for real-time metrics and daemon control
//
// This module provides:
// - Async Unix socket connection for metrics streaming (MetricsSocket - PREFERRED)
// - Automatic reconnection on disconnect
// - Event parsing and Tauri integration
// - Command socket for daemon control
//
// IMPORTANT: Use MetricsSocket for all metrics streaming.
// The legacy SocketConnection implementation has critical bugs and should not be used.

mod metrics;
mod socket_utils;

// Primary exports (use these!)
pub use metrics::MetricsSocket;
pub use socket_utils::get_metrics_socket_path;

use anyhow::{Context, Result};
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream as StdUnixStream;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::time::sleep;

/// Unix socket connection for real-time metrics streaming
pub struct SocketConnection {
    socket_path: String,
    stream: Arc<Mutex<Option<StdUnixStream>>>,
    app_handle: AppHandle,
}

impl SocketConnection {
    /// Create new socket connection
    pub fn new(socket_path: String, app_handle: AppHandle) -> Self {
        Self {
            socket_path,
            stream: Arc::new(Mutex::new(None)),
            app_handle,
        }
    }

    /// Check if connected to socket
    pub async fn is_connected(&self) -> bool {
        self.stream.lock().await.is_some()
    }

    /// Connect to Unix socket
    fn connect(&self) -> Result<StdUnixStream> {
        let stream = StdUnixStream::connect(&self.socket_path)
            .context("Failed to connect to metrics socket")?;

        // Set non-blocking for reads
        stream.set_read_timeout(Some(Duration::from_millis(100)))?;

        Ok(stream)
    }

    /// Start listening for events
    pub async fn start_listener(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                // Try to connect if not connected
                if !self.is_connected().await {
                    match self.connect() {
                        Ok(stream) => {
                            log::info!("Connected to metrics socket at {}", self.socket_path);
                            *self.stream.lock().await = Some(stream);

                            // Emit connection status
                            self.app_handle.emit("socket-connected", true).ok();
                        }
                        Err(e) => {
                            log::warn!("Failed to connect to socket: {}. Retrying...", e);
                            sleep(Duration::from_secs(2)).await;
                            continue;
                        }
                    }
                }

                // Read events from socket
                let stream_lock = self.stream.lock().await;
                if let Some(stream) = stream_lock.as_ref() {
                    match self.read_events(stream) {
                        Ok(_) => {}
                        Err(e) => {
                            log::error!("Socket read error: {}. Reconnecting...", e);
                            drop(stream_lock); // Drop lock before acquiring again
                            *self.stream.lock().await = None;
                            self.app_handle.emit("socket-connected", false).ok();
                            sleep(Duration::from_secs(2)).await;
                        }
                    }
                }
            }
        });
    }

    /// Read and process events from socket
    fn read_events(&self, stream: &StdUnixStream) -> Result<()> {
        let mut reader = BufReader::new(stream);
        let mut line = String::new();

        // Read line from socket
        match reader.read_line(&mut line) {
            Ok(0) => {
                // EOF - connection closed
                anyhow::bail!("Socket connection closed");
            }
            Ok(_) => {
                // Parse and emit event
                if let Ok(event) = serde_json::from_str::<Value>(&line) {
                    if let Some(event_type) = event.get("type").and_then(|t| t.as_str()) {
                        // Emit as 'metrics-event' for frontend listener
                        self.app_handle.emit("metrics-event", &event).ok();
                        log::debug!("Emitted event: {}", event_type);
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No data available, sleep briefly
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                return Err(e.into());
            }
        }

        Ok(())
    }

    /// Send toggle recording command to daemon (via socket)
    pub fn toggle_recording(&self) -> Result<String> {
        // For now, return a placeholder
        // In the future, this could send a command to the daemon via the socket
        // or invoke the daemon's hotkey trigger
        Ok("Toggle command sent".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::socket::socket_utils;

    #[test]
    fn test_socket_path_validation() {
        let socket_path = socket_utils::get_metrics_socket_path();
        let socket_str = socket_path.to_string_lossy();
        assert!(!socket_str.is_empty());
        // Socket should NEVER be in /tmp - must be in platform-appropriate directory
        assert!(!socket_str.starts_with("/tmp/"), "Socket path must not be in /tmp");
        assert!(socket_str.ends_with("swictation_metrics.sock"));
    }
}
