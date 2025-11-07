use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// Path to the metrics broadcast socket
const METRICS_SOCKET_PATH: &str = "/tmp/swictation_metrics.sock";

/// Path to the command socket for daemon control
const COMMAND_SOCKET_PATH: &str = "/tmp/swictation.sock";

/// Reconnection delay after socket disconnect
const RECONNECT_DELAY_SECS: u64 = 5;

/// Socket read timeout (to detect stuck connections)
const SOCKET_TIMEOUT_SECS: u64 = 30;

/// Metrics socket event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MetricsEvent {
    /// Session started
    SessionStart {
        session_id: String,
        timestamp: u64,
    },

    /// Session ended
    SessionEnd {
        session_id: String,
        timestamp: u64,
    },

    /// Daemon state changed
    StateChange {
        #[serde(rename = "daemon_state")]
        state: String,
        timestamp: u64,
    },

    /// New transcription received
    Transcription {
        session_id: String,
        text: String,
        timestamp: u64,
        wpm: f64,
        latency_ms: u64,
    },

    /// Periodic metrics update
    MetricsUpdate {
        state: String,
        wpm: f64,
        words: u64,
        latency_ms: u64,
        segments: u64,
        duration_s: f64,
        gpu_memory_mb: f64,
        cpu_percent: f64,
    },
}

/// Unix socket connection manager for real-time metrics
pub struct MetricsSocket {
    socket_path: String,
    connected: bool,
}

impl MetricsSocket {
    /// Create a new MetricsSocket instance
    pub fn new() -> Self {
        Self {
            socket_path: METRICS_SOCKET_PATH.to_string(),
            connected: false,
        }
    }

    /// Connect to the metrics socket
    pub async fn connect() -> Result<Self> {
        let socket_path = METRICS_SOCKET_PATH;

        // Check if socket exists
        if !Path::new(socket_path).exists() {
            anyhow::bail!("Metrics socket does not exist: {}", socket_path);
        }

        info!("Connecting to metrics socket: {}", socket_path);

        Ok(Self {
            socket_path: socket_path.to_string(),
            connected: false,
        })
    }

    /// Listen for events and emit them to the Tauri frontend
    /// This function runs indefinitely with automatic reconnection
    pub async fn listen(&mut self, app_handle: AppHandle) -> Result<()> {
        loop {
            match self.connect_and_process(&app_handle).await {
                Ok(_) => {
                    info!("Socket connection closed normally");
                }
                Err(e) => {
                    error!("Socket connection error: {}", e);
                    self.connected = false;
                }
            }

            // Reconnect after delay
            warn!("Reconnecting to metrics socket in {} seconds...", RECONNECT_DELAY_SECS);
            sleep(Duration::from_secs(RECONNECT_DELAY_SECS)).await;
        }
    }

    /// Connect to socket and process events
    async fn connect_and_process(&mut self, app_handle: &AppHandle) -> Result<()> {
        // Connect to Unix socket
        let stream = UnixStream::connect(&self.socket_path)
            .await
            .context("Failed to connect to metrics socket")?;

        info!("✓ Connected to metrics socket");
        self.connected = true;

        // Emit connection status
        app_handle
            .emit("metrics-connected", true)
            .context("Failed to emit connection status")?;

        // Set up buffered reader for line-by-line processing
        let reader = BufReader::new(stream);
        let mut lines = reader.lines();

        // Process events line by line
        while let Some(line) = lines
            .next_line()
            .await
            .context("Failed to read from socket")?
        {
            if line.trim().is_empty() {
                continue;
            }

            debug!("Received raw event: {}", line);

            // Parse and handle event
            match serde_json::from_str::<MetricsEvent>(&line) {
                Ok(event) => {
                    if let Err(e) = self.handle_event(app_handle, event).await {
                        error!("Failed to handle event: {}", e);
                    }
                }
                Err(e) => {
                    warn!("Failed to parse event: {} (line: {})", e, line);
                }
            }
        }

        // Connection closed
        self.connected = false;
        app_handle
            .emit("metrics-connected", false)
            .context("Failed to emit disconnection status")?;

        Ok(())
    }

    /// Handle a parsed metrics event
    async fn handle_event(&self, app_handle: &AppHandle, event: MetricsEvent) -> Result<()> {
        debug!("Handling event: {:?}", event);

        match &event {
            MetricsEvent::SessionStart { session_id, .. } => {
                info!("Session started: {}", session_id);
                app_handle
                    .emit("session-start", event)
                    .context("Failed to emit session-start")?;
            }

            MetricsEvent::SessionEnd { session_id, .. } => {
                info!("Session ended: {}", session_id);
                app_handle
                    .emit("session-end", event)
                    .context("Failed to emit session-end")?;
            }

            MetricsEvent::StateChange { state, .. } => {
                info!("Daemon state changed: {}", state);
                app_handle
                    .emit("state-change", event)
                    .context("Failed to emit state-change")?;
            }

            MetricsEvent::Transcription {
                text, wpm, latency_ms, ..
            } => {
                debug!("Transcription: '{}' (WPM: {}, latency: {}ms)", text, wpm, latency_ms);
                app_handle
                    .emit("transcription", event)
                    .context("Failed to emit transcription")?;
            }

            MetricsEvent::MetricsUpdate {
                state,
                wpm,
                words,
                latency_ms,
                segments,
                duration_s,
                gpu_memory_mb,
                cpu_percent,
            } => {
                debug!(
                    "Metrics update: state={}, wpm={}, words={}, latency={}ms, segments={}, duration={}s, gpu={}MB, cpu={}%",
                    state, wpm, words, latency_ms, segments, duration_s, gpu_memory_mb, cpu_percent
                );
                app_handle
                    .emit("metrics-update", event)
                    .context("Failed to emit metrics-update")?;
            }
        }

        Ok(())
    }

    /// Send toggle command to daemon control socket
    pub async fn send_toggle_command() -> Result<()> {
        let command_socket = COMMAND_SOCKET_PATH;

        // Check if socket exists
        if !Path::new(command_socket).exists() {
            anyhow::bail!("Command socket does not exist: {}", command_socket);
        }

        info!("Sending toggle command to daemon");

        // Connect to command socket
        let mut stream = UnixStream::connect(command_socket)
            .await
            .context("Failed to connect to command socket")?;

        // Send toggle command (newline-delimited)
        stream
            .write_all(b"toggle\n")
            .await
            .context("Failed to write toggle command")?;

        stream
            .flush()
            .await
            .context("Failed to flush toggle command")?;

        info!("✓ Toggle command sent successfully");

        Ok(())
    }

    /// Check if socket is currently connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Get the socket path
    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }
}

impl Default for MetricsSocket {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_event_deserialization() {
        // Test session_start
        let json = r#"{"type":"session_start","session_id":"test-123","timestamp":1234567890}"#;
        let event: MetricsEvent = serde_json::from_str(json).unwrap();
        match event {
            MetricsEvent::SessionStart { session_id, timestamp } => {
                assert_eq!(session_id, "test-123");
                assert_eq!(timestamp, 1234567890);
            }
            _ => panic!("Wrong event type"),
        }

        // Test metrics_update
        let json = r#"{"type":"metrics_update","state":"recording","wpm":120.5,"words":100,"latency_ms":150,"segments":10,"duration_s":60.5,"gpu_memory_mb":2048.0,"cpu_percent":45.2}"#;
        let event: MetricsEvent = serde_json::from_str(json).unwrap();
        match event {
            MetricsEvent::MetricsUpdate {
                state,
                wpm,
                words,
                latency_ms,
                segments,
                duration_s,
                gpu_memory_mb,
                cpu_percent,
            } => {
                assert_eq!(state, "recording");
                assert_eq!(wpm, 120.5);
                assert_eq!(words, 100);
                assert_eq!(latency_ms, 150);
                assert_eq!(segments, 10);
                assert_eq!(duration_s, 60.5);
                assert_eq!(gpu_memory_mb, 2048.0);
                assert_eq!(cpu_percent, 45.2);
            }
            _ => panic!("Wrong event type"),
        }

        // Test transcription
        let json = r#"{"type":"transcription","session_id":"test-123","text":"Hello world","timestamp":1234567890,"wpm":120.0,"latency_ms":100}"#;
        let event: MetricsEvent = serde_json::from_str(json).unwrap();
        match event {
            MetricsEvent::Transcription {
                session_id,
                text,
                timestamp,
                wpm,
                latency_ms,
            } => {
                assert_eq!(session_id, "test-123");
                assert_eq!(text, "Hello world");
                assert_eq!(timestamp, 1234567890);
                assert_eq!(wpm, 120.0);
                assert_eq!(latency_ms, 100);
            }
            _ => panic!("Wrong event type"),
        }

        // Test state_change
        let json = r#"{"type":"state_change","daemon_state":"recording","timestamp":1234567890}"#;
        let event: MetricsEvent = serde_json::from_str(json).unwrap();
        match event {
            MetricsEvent::StateChange { state, timestamp } => {
                assert_eq!(state, "recording");
                assert_eq!(timestamp, 1234567890);
            }
            _ => panic!("Wrong event type"),
        }

        // Test session_end
        let json = r#"{"type":"session_end","session_id":"test-123","timestamp":1234567890}"#;
        let event: MetricsEvent = serde_json::from_str(json).unwrap();
        match event {
            MetricsEvent::SessionEnd { session_id, timestamp } => {
                assert_eq!(session_id, "test-123");
                assert_eq!(timestamp, 1234567890);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_socket_creation() {
        let socket = MetricsSocket::new();
        assert_eq!(socket.socket_path(), METRICS_SOCKET_PATH);
        assert!(!socket.is_connected());
    }

    #[test]
    fn test_socket_default() {
        let socket = MetricsSocket::default();
        assert_eq!(socket.socket_path(), METRICS_SOCKET_PATH);
        assert!(!socket.is_connected());
    }
}
