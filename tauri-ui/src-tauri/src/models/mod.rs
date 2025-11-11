use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Session summary for history list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: i64,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub duration_s: f64,
    pub words_dictated: i32,
    pub wpm: f64,
    pub avg_latency_ms: f64,
}

/// Transcription record from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionRecord {
    pub id: i64,
    pub session_id: i64,
    pub text: String,
    pub timestamp: i64,
    pub latency_ms: Option<f64>,
    pub words: i32,
}

/// Lifetime statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifetimeStats {
    pub total_words: i64,
    pub total_characters: i64,
    pub total_sessions: i32,
    pub total_time_minutes: f64,
    pub average_wpm: f64,
    pub average_latency_ms: f64,
    pub best_wpm_value: f64,
    pub best_wpm_session: Option<i64>,
}

/// Connection status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub connected: bool,
    pub socket_path: String,
}

/// Daemon state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DaemonState {
    Idle,
    Recording,
    Processing,
    Error,
}
