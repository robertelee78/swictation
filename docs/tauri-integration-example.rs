// Example Tauri backend integration for Swictation metrics database
// Place this in your Tauri src-tauri/src/main.rs or create a separate module

use serde::{Deserialize, Serialize};
use swictation_metrics::{MetricsDatabase, SessionMetrics, SegmentMetrics, LifetimeMetrics};
use tauri::State;
use std::sync::Arc;

// Wrapper for thread-safe database state
pub struct DbState {
    pub db: Arc<MetricsDatabase>,
}

// Simplified DTOs for frontend (optional - you can use the full structs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: i64,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub duration_s: f64,
    pub words_dictated: i32,
    pub wpm: f64,
    pub avg_latency_ms: f64,
    pub gpu_peak_mb: f64,
    pub cpu_peak_percent: f64,
}

impl From<SessionMetrics> for SessionSummary {
    fn from(s: SessionMetrics) -> Self {
        Self {
            id: s.session_id.unwrap_or(0),
            start_time: s.session_start.map(|dt| dt.to_rfc3339()),
            end_time: s.session_end.map(|dt| dt.to_rfc3339()),
            duration_s: s.total_duration_s,
            words_dictated: s.words_dictated,
            wpm: s.words_per_minute,
            avg_latency_ms: s.average_latency_ms,
            gpu_peak_mb: s.gpu_memory_peak_mb,
            cpu_peak_percent: s.cpu_usage_peak_percent,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionRecord {
    pub id: i64,
    pub session_id: i64,
    pub timestamp: Option<String>,
    pub text: String,
    pub words: i32,
    pub duration_s: f64,
    pub total_latency_ms: f64,
    pub wpm: f64,
}

impl From<SegmentMetrics> for TranscriptionRecord {
    fn from(s: SegmentMetrics) -> Self {
        Self {
            id: s.segment_id.unwrap_or(0),
            session_id: s.session_id.unwrap_or(0),
            timestamp: s.timestamp.map(|dt| dt.to_rfc3339()),
            text: s.text,
            words: s.words,
            duration_s: s.duration_s,
            total_latency_ms: s.total_latency_ms,
            wpm: s.calculate_wpm(),
        }
    }
}

// Tauri Commands

/// Get recent dictation sessions
#[tauri::command]
async fn get_recent_sessions(
    db: State<'_, DbState>,
    limit: usize,
) -> Result<Vec<SessionSummary>, String> {
    db.db
        .get_recent_sessions(limit)
        .map(|sessions| sessions.into_iter().map(SessionSummary::from).collect())
        .map_err(|e| format!("Database error: {}", e))
}

/// Get all transcriptions for a specific session
#[tauri::command]
async fn get_session_transcriptions(
    db: State<'_, DbState>,
    session_id: i64,
) -> Result<Vec<TranscriptionRecord>, String> {
    db.db
        .get_session_segments(session_id)
        .map(|segments| segments.into_iter().map(TranscriptionRecord::from).collect())
        .map_err(|e| format!("Database error: {}", e))
}

/// Search transcriptions by text query
#[tauri::command]
async fn search_transcriptions(
    db: State<'_, DbState>,
    query: String,
    limit: usize,
) -> Result<Vec<TranscriptionRecord>, String> {
    db.db
        .search_transcriptions(&query, limit)
        .map(|segments| segments.into_iter().map(TranscriptionRecord::from).collect())
        .map_err(|e| format!("Database error: {}", e))
}

/// Get lifetime statistics
#[tauri::command]
async fn get_lifetime_stats(
    db: State<'_, DbState>,
) -> Result<LifetimeMetrics, String> {
    db.db
        .get_lifetime_stats()
        .map_err(|e| format!("Database error: {}", e))
}

/// Get sessions from last N days
#[tauri::command]
async fn get_sessions_last_n_days(
    db: State<'_, DbState>,
    days: u32,
) -> Result<Vec<SessionSummary>, String> {
    db.db
        .get_sessions_last_n_days(days)
        .map(|sessions| sessions.into_iter().map(SessionSummary::from).collect())
        .map_err(|e| format!("Database error: {}", e))
}

/// Get database statistics
#[tauri::command]
async fn get_database_info(
    db: State<'_, DbState>,
) -> Result<DatabaseInfo, String> {
    let size_mb = db.db
        .get_database_size_mb()
        .map_err(|e| format!("Failed to get database size: {}", e))?;

    let stats = db.db
        .get_lifetime_stats()
        .map_err(|e| format!("Failed to get lifetime stats: {}", e))?;

    Ok(DatabaseInfo {
        size_mb,
        total_sessions: stats.total_sessions,
        total_segments: stats.total_segments,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseInfo {
    pub size_mb: f64,
    pub total_sessions: i32,
    pub total_segments: i64,
}

// Main application setup
fn main() {
    // Initialize database with proper error handling
    let db_path = dirs::data_dir()
        .expect("Failed to get data directory")
        .join("swictation")
        .join("metrics.db");

    let db = MetricsDatabase::new(&db_path)
        .expect("Failed to initialize metrics database");

    println!("✓ Metrics database initialized at: {:?}", db_path);

    tauri::Builder::default()
        .manage(DbState { db: Arc::new(db) })
        .invoke_handler(tauri::generate_handler![
            get_recent_sessions,
            get_session_transcriptions,
            search_transcriptions,
            get_lifetime_stats,
            get_sessions_last_n_days,
            get_database_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Example frontend TypeScript usage:
/*
import { invoke } from '@tauri-apps/api/tauri';

// Get recent sessions
const sessions = await invoke<SessionSummary[]>('get_recent_sessions', {
  limit: 10
});

// Get session details
const transcriptions = await invoke<TranscriptionRecord[]>(
  'get_session_transcriptions',
  { sessionId: 123 }
);

// Search all transcriptions
const results = await invoke<TranscriptionRecord[]>(
  'search_transcriptions',
  { query: "machine learning", limit: 20 }
);

// Get lifetime statistics
const stats = await invoke<LifetimeStats>('get_lifetime_stats');

console.log(`Total words dictated: ${stats.total_words}`);
console.log(`Average WPM: ${stats.average_wpm}`);
console.log(`Time saved: ${stats.estimated_time_saved_minutes} minutes`);

// Get weekly performance
const weekSessions = await invoke<SessionSummary[]>(
  'get_sessions_last_n_days',
  { days: 7 }
);

// Database info
const dbInfo = await invoke<DatabaseInfo>('get_database_info');
console.log(`Database size: ${dbInfo.size_mb.toFixed(2)} MB`);
*/
