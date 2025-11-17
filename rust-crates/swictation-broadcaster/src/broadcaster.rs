use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::net::UnixListener;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use swictation_metrics::{RealtimeMetrics, DaemonState};
use chrono::Local;

use crate::client::{Client, ClientManager};
use crate::error::{BroadcasterError, Result};
use crate::events::{BroadcastEvent, TranscriptionSegment};

/// Real-time metrics broadcaster for UI clients
pub struct MetricsBroadcaster {
    socket_path: PathBuf,
    client_manager: ClientManager,
    transcription_buffer: Arc<RwLock<Vec<TranscriptionSegment>>>,
    last_state: Arc<RwLock<String>>,
    current_session_id: Arc<RwLock<Option<i64>>>,
    accept_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    running: Arc<RwLock<bool>>,
}

impl MetricsBroadcaster {
    /// Create new broadcaster
    pub async fn new(socket_path: impl AsRef<Path>) -> Result<Self> {
        let socket_path = socket_path.as_ref().to_path_buf();

        Ok(Self {
            socket_path,
            client_manager: ClientManager::new(),
            transcription_buffer: Arc::new(RwLock::new(Vec::new())),
            last_state: Arc::new(RwLock::new("idle".to_string())),
            current_session_id: Arc::new(RwLock::new(None)),
            accept_task: Arc::new(Mutex::new(None)),
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// Start the broadcaster (listen for clients)
    pub async fn start(&self) -> Result<()> {
        let is_running = *self.running.read().await;
        if is_running {
            return Err(BroadcasterError::AlreadyRunning);
        }

        // Remove existing socket file
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)?;
        }

        // Create Unix socket listener
        let listener = UnixListener::bind(&self.socket_path)?;

        // Set secure permissions (0600 = owner-only access)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if self.socket_path.exists() {
                let permissions = std::fs::Permissions::from_mode(0o600);
                std::fs::set_permissions(&self.socket_path, permissions)?;
            }
        }

        tracing::info!("Metrics broadcaster started on {:?} (permissions: 0600)", self.socket_path);

        // Mark as running
        *self.running.write().await = true;

        // Spawn client acceptance task
        let client_manager = self.client_manager.clone_arc();
        let buffer = Arc::clone(&self.transcription_buffer);
        let state = Arc::clone(&self.last_state);
        let session_id = Arc::clone(&self.current_session_id);
        let running = Arc::clone(&self.running);

        let task = tokio::spawn(async move {
            loop {
                // Check if still running
                if !*running.read().await {
                    break;
                }

                match listener.accept().await {
                    Ok((stream, _addr)) => {
                        tracing::info!("New client connection accepted");
                        let mut client = Client::new(stream);

                        // Send catch-up data
                        let current_state = state.read().await.clone();
                        let current_session = *session_id.read().await;
                        let buffer_snapshot = buffer.read().await.clone();

                        if let Err(e) = client.send_catch_up(
                            &current_state,
                            current_session,
                            &buffer_snapshot,
                        ).await {
                            tracing::warn!("Failed to send catch-up data: {}", e);
                            continue;
                        }

                        // Add to client list
                        let mut clients = client_manager.lock().await;
                        clients.push(client);
                        tracing::info!("Client added. Total: {}", clients.len());
                    }
                    Err(e) => {
                        tracing::error!("Failed to accept client: {}", e);
                    }
                }
            }
            tracing::info!("Client acceptance task stopped");
        });

        *self.accept_task.lock().await = Some(task);

        Ok(())
    }

    /// Stop the broadcaster
    pub async fn stop(&self) -> Result<()> {
        let is_running = *self.running.read().await;
        if !is_running {
            return Err(BroadcasterError::NotStarted);
        }

        // Mark as not running
        *self.running.write().await = false;

        // Abort accept task
        if let Some(task) = self.accept_task.lock().await.take() {
            task.abort();
        }

        // Remove socket file
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)?;
        }

        tracing::info!("Metrics broadcaster stopped");
        Ok(())
    }

    /// Start a new session (clears transcription buffer)
    pub async fn start_session(&self, session_id: i64) {
        // Clear buffer
        self.transcription_buffer.write().await.clear();

        // Update session ID
        *self.current_session_id.write().await = Some(session_id);

        // Broadcast event
        let event = BroadcastEvent::SessionStart {
            session_id,
            timestamp: Self::current_timestamp(),
        };

        if let Err(e) = self.client_manager.broadcast(&event).await {
            tracing::error!("Failed to broadcast session_start: {}", e);
        }

        tracing::info!("Session started: {}", session_id);
    }

    /// End session (buffer stays visible)
    pub async fn end_session(&self, session_id: i64) {
        // Update session ID
        *self.current_session_id.write().await = None;

        // Broadcast event
        let event = BroadcastEvent::SessionEnd {
            session_id,
            timestamp: Self::current_timestamp(),
        };

        if let Err(e) = self.client_manager.broadcast(&event).await {
            tracing::error!("Failed to broadcast session_end: {}", e);
        }

        tracing::info!("Session ended: {}", session_id);
    }

    /// Add transcription segment to buffer and broadcast
    pub async fn add_transcription(
        &self,
        text: String,
        wpm: f64,
        latency_ms: f64,
        words: i32,
    ) {
        let timestamp = Self::current_time_string();

        // Create segment
        let segment = TranscriptionSegment {
            text: text.clone(),
            timestamp: timestamp.clone(),
            wpm,
            latency_ms,
            words,
        };

        // Add to buffer
        self.transcription_buffer.write().await.push(segment);

        // Broadcast event
        let event = BroadcastEvent::Transcription {
            text,
            timestamp,
            wpm,
            latency_ms,
            words,
        };

        if let Err(e) = self.client_manager.broadcast(&event).await {
            tracing::error!("Failed to broadcast transcription: {}", e);
        }
    }

    /// Update and broadcast real-time metrics
    pub async fn update_metrics(&self, realtime: &RealtimeMetrics) {
        let state = Self::daemon_state_to_string(&realtime.current_state);

        let event = BroadcastEvent::MetricsUpdate {
            state,
            session_id: realtime.current_session_id,
            segments: realtime.segments_this_session,
            words: realtime.words_this_session,
            wpm: realtime.wpm_this_session,
            duration_s: realtime.recording_duration_s,
            latency_ms: realtime.last_segment_latency_ms,
            gpu_memory_mb: realtime.gpu_memory_current_mb,
            gpu_memory_percent: realtime.gpu_memory_percent,
            cpu_percent: realtime.cpu_percent_current,
        };

        if let Err(e) = self.client_manager.broadcast(&event).await {
            tracing::error!("Failed to broadcast metrics_update: {}", e);
        }
    }

    /// Broadcast daemon state change
    pub async fn broadcast_state_change(&self, state: DaemonState) {
        let state_str = Self::daemon_state_to_string(&state);

        // Update last state
        *self.last_state.write().await = state_str.clone();

        let event = BroadcastEvent::StateChange {
            state: state_str,
            timestamp: Self::current_timestamp(),
        };

        if let Err(e) = self.client_manager.broadcast(&event).await {
            tracing::error!("Failed to broadcast state_change: {}", e);
        }
    }

    /// Get current client count
    pub async fn client_count(&self) -> usize {
        self.client_manager.client_count().await
    }

    /// Get buffer size
    pub async fn buffer_size(&self) -> usize {
        self.transcription_buffer.read().await.len()
    }

    // Helper functions

    fn current_timestamp() -> f64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
    }

    fn current_time_string() -> String {
        Local::now().format("%H:%M:%S").to_string()
    }

    fn daemon_state_to_string(state: &DaemonState) -> String {
        match state {
            DaemonState::Idle => "idle".to_string(),
            DaemonState::Recording => "recording".to_string(),
            DaemonState::Processing => "processing".to_string(),
            DaemonState::Error => "error".to_string(),
        }
    }
}

impl Drop for MetricsBroadcaster {
    fn drop(&mut self) {
        // Clean up socket file
        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_broadcaster_create() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path().to_path_buf();
        std::fs::remove_file(&path).ok();

        let broadcaster = MetricsBroadcaster::new(path).await.unwrap();
        assert_eq!(broadcaster.client_count().await, 0);
        assert_eq!(broadcaster.buffer_size().await, 0);
    }

    #[tokio::test]
    async fn test_session_lifecycle() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path().to_path_buf();
        std::fs::remove_file(&path).ok();

        let broadcaster = MetricsBroadcaster::new(path).await.unwrap();

        // Start session should clear buffer
        broadcaster.add_transcription("test".to_string(), 100.0, 200.0, 1).await;
        assert_eq!(broadcaster.buffer_size().await, 1);

        broadcaster.start_session(123).await;
        assert_eq!(broadcaster.buffer_size().await, 0);

        // Add new transcription
        broadcaster.add_transcription("new".to_string(), 150.0, 180.0, 1).await;
        assert_eq!(broadcaster.buffer_size().await, 1);

        // End session should keep buffer
        broadcaster.end_session(123).await;
        assert_eq!(broadcaster.buffer_size().await, 1);
    }
}
