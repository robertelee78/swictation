use tokio::net::UnixStream;
use tokio::io::AsyncWriteExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::error::Result;
use crate::events::{BroadcastEvent, TranscriptionSegment};

/// Client connection wrapper
pub struct Client {
    stream: UnixStream,
}

impl Client {
    pub fn new(stream: UnixStream) -> Self {
        Self { stream }
    }

    /// Send event to client
    pub async fn send_event(&mut self, event: &BroadcastEvent) -> Result<()> {
        let json_line = event.to_json_line()?;
        self.stream.write_all(json_line.as_bytes()).await?;
        Ok(())
    }

    /// Send current state to new client (catch-up)
    pub async fn send_catch_up(
        &mut self,
        current_state: &str,
        session_id: Option<i64>,
        buffer: &[TranscriptionSegment],
    ) -> Result<()> {
        // Send current state
        let state_event = BroadcastEvent::StateChange {
            state: current_state.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
        };
        self.send_event(&state_event).await?;

        // Send session start if active
        if let Some(sid) = session_id {
            let session_event = BroadcastEvent::SessionStart {
                session_id: sid,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs_f64(),
            };
            self.send_event(&session_event).await?;
        }

        // Send buffered transcriptions
        for segment in buffer {
            let trans_event = BroadcastEvent::Transcription {
                text: segment.text.clone(),
                timestamp: segment.timestamp.clone(),
                wpm: segment.wpm,
                latency_ms: segment.latency_ms,
                words: segment.words,
            };
            self.send_event(&trans_event).await?;
        }

        Ok(())
    }
}

/// Thread-safe client list manager
pub struct ClientManager {
    clients: Arc<Mutex<Vec<Client>>>,
}

impl ClientManager {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add new client
    pub async fn add_client(&self, client: Client) {
        let mut clients = self.clients.lock().await;
        clients.push(client);
        tracing::info!("New client connected. Total clients: {}", clients.len());
    }

    /// Broadcast event to all clients, removing dead ones
    pub async fn broadcast(&self, event: &BroadcastEvent) -> Result<()> {
        let mut clients = self.clients.lock().await;
        let mut dead_indices = Vec::new();

        for (idx, client) in clients.iter_mut().enumerate() {
            if let Err(e) = client.send_event(event).await {
                tracing::warn!("Failed to send to client {}: {}", idx, e);
                dead_indices.push(idx);
            }
        }

        // Remove dead clients in reverse order
        for idx in dead_indices.iter().rev() {
            clients.remove(*idx);
            tracing::info!("Removed dead client. Remaining: {}", clients.len());
        }

        Ok(())
    }

    /// Get current client count
    pub async fn client_count(&self) -> usize {
        self.clients.lock().await.len()
    }

    /// Get cloned Arc for sharing
    pub fn clone_arc(&self) -> Arc<Mutex<Vec<Client>>> {
        Arc::clone(&self.clients)
    }
}

impl Default for ClientManager {
    fn default() -> Self {
        Self::new()
    }
}
