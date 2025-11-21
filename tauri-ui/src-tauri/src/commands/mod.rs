pub mod corrections;

use crate::database::Database;
use crate::models::{ConnectionStatus, LifetimeStats, SessionSummary, TranscriptionRecord};
use std::sync::Mutex;
use tauri::State;

// Re-export corrections types
pub use corrections::CorrectionsState;

/// Application state shared across commands
pub struct AppState {
    pub db: Mutex<Database>,
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

/// Toggle recording (triggers via hotkey or tray menu)
/// This command is deprecated - use the tray menu or hotkey instead.
/// The daemon handles toggle recording internally via global hotkey.
#[tauri::command]
pub async fn toggle_recording() -> Result<String, String> {
    // Recording toggle is handled by the daemon via hotkey (Ctrl+Shift+D)
    // and tray menu event emission. This command is kept for API compatibility.
    Ok("Toggle recording via hotkey (Ctrl+Shift+D) or tray menu".to_string())
}

/// Get socket connection status
/// This command is deprecated - connection status is sent via "metrics-connected" events.
#[tauri::command]
pub async fn get_connection_status() -> Result<ConnectionStatus, String> {
    // Connection status is now automatically sent via MetricsSocket events.
    // Listen to "metrics-connected" events in the frontend instead.
    Ok(ConnectionStatus {
        connected: true, // Placeholder - actual status via events
        socket_path: "/run/user/1000/swictation_metrics.sock".to_string(),
    })
}

/// Reset all database data (sessions, segments, lifetime stats)
#[tauri::command]
pub async fn reset_database(state: State<'_, AppState>) -> Result<(), String> {
    state
        .db
        .lock()
        .unwrap()
        .reset_database()
        .map_err(|e| format!("Failed to reset database: {}", e))
}
