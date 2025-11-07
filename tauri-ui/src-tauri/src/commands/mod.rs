use crate::database::Database;
use crate::models::{ConnectionStatus, LifetimeStats, SessionSummary, TranscriptionRecord};
use crate::socket::SocketConnection;
use std::sync::{Arc, Mutex};
use tauri::State;

/// Application state shared across commands
pub struct AppState {
    pub db: Mutex<Database>,
    pub socket: Arc<SocketConnection>,
}

/// Get recent sessions from database
#[tauri::command]
pub async fn get_recent_sessions(
    state: State<'_, AppState>,
    limit: usize,
) -> Result<Vec<SessionSummary>, String> {
    state
        .db
        .lock()
        .unwrap()
        .get_recent_sessions(limit)
        .map_err(|e| format!("Failed to get recent sessions: {}", e))
}

/// Get session details (all transcriptions)
#[tauri::command]
pub async fn get_session_details(
    state: State<'_, AppState>,
    session_id: i64,
) -> Result<Vec<TranscriptionRecord>, String> {
    state
        .db
        .lock()
        .unwrap()
        .get_session_transcriptions(session_id)
        .map_err(|e| format!("Failed to get session transcriptions: {}", e))
}

/// Search transcriptions by text
#[tauri::command]
pub async fn search_transcriptions(
    state: State<'_, AppState>,
    query: String,
    limit: usize,
) -> Result<Vec<TranscriptionRecord>, String> {
    state
        .db
        .lock()
        .unwrap()
        .search_transcriptions(&query, limit)
        .map_err(|e| format!("Failed to search transcriptions: {}", e))
}

/// Get lifetime statistics
#[tauri::command]
pub async fn get_lifetime_stats(
    state: State<'_, AppState>,
) -> Result<LifetimeStats, String> {
    state
        .db
        .lock()
        .unwrap()
        .get_lifetime_stats()
        .map_err(|e| format!("Failed to get lifetime stats: {}", e))
}

/// Toggle recording (placeholder for now)
#[tauri::command]
pub async fn toggle_recording(
    state: State<'_, AppState>,
) -> Result<String, String> {
    state
        .socket
        .toggle_recording()
        .map_err(|e| format!("Failed to toggle recording: {}", e))
}

/// Get socket connection status
#[tauri::command]
pub async fn get_connection_status(
    state: State<'_, AppState>,
) -> Result<ConnectionStatus, String> {
    Ok(ConnectionStatus {
        connected: state.socket.is_connected().await,
        socket_path: "/tmp/swictation_metrics.sock".to_string(),
    })
}
