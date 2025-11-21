use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer, Serialize};
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use super::socket_utils::{get_metrics_socket_path, get_ipc_socket_path};

/// Reconnection delay after socket disconnect
const RECONNECT_DELAY_SECS: u64 = 5;

/// Socket read timeout (to detect stuck connections)
const SOCKET_TIMEOUT_SECS: u64 = 30;

/// Custom deserializer for flexible number types (accepts f64 or u64)
fn deserialize_flexible_number<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Number(n) => {
            if let Some(v) = n.as_u64() {
                Ok(v)
            } else if let Some(v) = n.as_f64() {
                Ok(v.round() as u64)
            } else {
                Err(Error::custom("Invalid number"))
            }
        }
        _ => Err(Error::custom("Expected number")),
    }
}

/// Custom deserializer for flexible timestamp (accepts u64, f64, or string time)
fn deserialize_flexible_timestamp<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Number(n) => {
            if let Some(v) = n.as_u64() {
                Ok(v)
            } else if let Some(v) = n.as_f64() {
                Ok(v.round() as u64)
            } else {
                Err(Error::custom("Invalid timestamp"))
            }
        }
        serde_json::Value::String(_) => {
            // For string timestamps like "17:13:04", use current time
            Ok(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs())
        }
        _ => Err(Error::custom("Expected timestamp as number or string")),
    }
}

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
        state: String,
        #[serde(deserialize_with = "deserialize_flexible_timestamp")]
        timestamp: u64,
    },

    /// New transcription received
    Transcription {
        text: String,
        #[serde(deserialize_with = "deserialize_flexible_timestamp")]
        timestamp: u64,
        wpm: f64,
        #[serde(deserialize_with = "deserialize_flexible_number")]
        latency_ms: u64,
        words: i64,
    },

    /// Periodic metrics update
    MetricsUpdate {
        state: String,
        wpm: f64,
        words: i64,
        #[serde(deserialize_with = "deserialize_flexible_number")]
        latency_ms: u64,
        segments: i64,
        duration_s: f64,
        gpu_memory_mb: f64,
        gpu_memory_percent: f64,
        cpu_percent: f64,
        session_id: i64,
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
        let socket_path = get_metrics_socket_path();
        Self {
            socket_path: socket_path.to_string_lossy().to_string(),
            connected: false,
        }
    }

    /// Connect to the metrics socket
    pub async fn connect() -> Result<Self> {
        let socket_path = get_metrics_socket_path();
        let socket_path_str = socket_path.to_str()
            .context("Invalid socket path")?;

        // Check if socket exists
        if !socket_path.exists() {
            anyhow::bail!("Metrics socket does not exist: {}", socket_path_str);
        }

        info!("Connecting to metrics socket: {}", socket_path_str);

        Ok(Self {
            socket_path: socket_path_str.to_string(),
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
                gpu_memory_percent,
                cpu_percent,
                session_id,
            } => {
                debug!(
                    "Metrics update: state={}, wpm={}, words={}, latency={}ms, segments={}, duration={}s, gpu={}MB ({}%), cpu={}%, session={}",
                    state, wpm, words, latency_ms, segments, duration_s, gpu_memory_mb, gpu_memory_percent, cpu_percent, session_id
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
        let command_socket = get_ipc_socket_path();
        let command_socket_str = command_socket.to_str()
            .context("Invalid socket path")?;

        // Check if socket exists
        if !command_socket.exists() {
            anyhow::bail!("Command socket does not exist: {}", command_socket_str);
        }

        info!("Sending toggle command to daemon");

        // Connect to command socket
        let mut stream = UnixStream::connect(command_socket_str)
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
        assert!(socket.socket_path().ends_with("swictation_metrics.sock"));
        assert!(!socket.is_connected());
    }

    #[test]
    fn test_socket_default() {
        let socket = MetricsSocket::default();
        assert!(socket.socket_path().ends_with("swictation_metrics.sock"));
        assert!(!socket.is_connected());
    }
}
