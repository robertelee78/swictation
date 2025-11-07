//! Data models for metrics tracking
//!
//! Matches Python implementation in src/metrics/models.py

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Daemon state enum (matches DaemonState in models.py)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DaemonState {
    Idle,
    Recording,
    Processing,
    Error,
}

impl std::fmt::Display for DaemonState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DaemonState::Idle => write!(f, "idle"),
            DaemonState::Recording => write!(f, "recording"),
            DaemonState::Processing => write!(f, "processing"),
            DaemonState::Error => write!(f, "error"),
        }
    }
}

/// Metrics for a single recording session (matches SessionMetrics dataclass)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetrics {
    // Identity
    pub session_id: Option<i64>,
    pub session_start: Option<DateTime<Utc>>,
    pub session_end: Option<DateTime<Utc>>,

    // Timing
    pub total_duration_s: f64,
    pub active_dictation_time_s: f64,
    pub pause_time_s: f64,

    // Content
    pub words_dictated: i32,
    pub characters_typed: i32,
    pub segments_processed: i32,

    // Performance
    pub words_per_minute: f64,
    pub typing_speed_equivalent: f64,
    pub average_latency_ms: f64,
    pub median_latency_ms: f64,
    pub p95_latency_ms: f64,

    // Quality indicators
    pub transformations_count: i32,
    pub keyboard_actions_count: i32,
    pub average_segment_words: f64,

    // Technical
    pub average_segment_duration_s: f64,
    pub gpu_memory_peak_mb: f64,
    pub gpu_memory_mean_mb: f64,
    pub cpu_usage_mean_percent: f64,
    pub cpu_usage_peak_percent: f64,
}

impl Default for SessionMetrics {
    fn default() -> Self {
        Self {
            session_id: None,
            session_start: None,
            session_end: None,
            total_duration_s: 0.0,
            active_dictation_time_s: 0.0,
            pause_time_s: 0.0,
            words_dictated: 0,
            characters_typed: 0,
            segments_processed: 0,
            words_per_minute: 0.0,
            typing_speed_equivalent: 40.0,
            average_latency_ms: 0.0,
            median_latency_ms: 0.0,
            p95_latency_ms: 0.0,
            transformations_count: 0,
            keyboard_actions_count: 0,
            average_segment_words: 0.0,
            average_segment_duration_s: 0.0,
            gpu_memory_peak_mb: 0.0,
            gpu_memory_mean_mb: 0.0,
            cpu_usage_mean_percent: 0.0,
            cpu_usage_peak_percent: 0.0,
        }
    }
}

impl SessionMetrics {
    /// Calculate words per minute from active time
    pub fn calculate_wpm(&mut self) {
        if self.active_dictation_time_s > 0.0 {
            self.words_per_minute = (self.words_dictated as f64 / self.active_dictation_time_s) * 60.0;
        } else {
            self.words_per_minute = 0.0;
        }
    }
}

/// Metrics for a single VAD-triggered segment (matches SegmentMetrics dataclass)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentMetrics {
    // Identity
    pub segment_id: Option<i64>,
    pub session_id: Option<i64>,
    pub timestamp: Option<DateTime<Utc>>,

    // Content
    pub duration_s: f64,
    pub words: i32,
    pub characters: i32,
    pub text: String,

    // Latency breakdown (matches database column names)
    pub vad_latency_ms: f64,
    pub audio_save_latency_ms: f64,
    pub stt_latency_ms: f64,
    pub transform_latency_us: f64,
    pub injection_latency_ms: f64,
    pub total_latency_ms: f64,

    // Quality indicators
    pub transformations_count: i32,
    pub keyboard_actions_count: i32,
}

impl Default for SegmentMetrics {
    fn default() -> Self {
        Self {
            segment_id: None,
            session_id: None,
            timestamp: None,
            duration_s: 0.0,
            words: 0,
            characters: 0,
            text: String::new(),
            vad_latency_ms: 0.0,
            audio_save_latency_ms: 0.0,
            stt_latency_ms: 0.0,
            transform_latency_us: 0.0,
            injection_latency_ms: 0.0,
            total_latency_ms: 0.0,
            transformations_count: 0,
            keyboard_actions_count: 0,
        }
    }
}

impl SegmentMetrics {
    /// Calculate WPM for this segment
    pub fn calculate_wpm(&self) -> f64 {
        if self.duration_s > 0.0 {
            (self.words as f64 / self.duration_s) * 60.0
        } else {
            0.0
        }
    }
}

/// Aggregate metrics across all sessions (matches LifetimeMetrics dataclass)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifetimeMetrics {
    // Totals
    pub total_words: i64,
    pub total_characters: i64,
    pub total_sessions: i32,
    pub total_dictation_time_minutes: f64,
    pub total_segments: i64,

    // Performance averages
    pub average_wpm: f64,
    pub average_latency_ms: f64,

    // Productivity
    pub typing_speed_equivalent: f64,
    pub speedup_factor: f64,
    pub estimated_time_saved_minutes: f64,

    // Trends (7-day rolling)
    pub wpm_trend_7day: f64,
    pub latency_trend_7day: f64,

    // System health
    pub cuda_errors_total: i32,
    pub cuda_errors_recovered: i32,
    pub memory_pressure_events: i32,
    pub high_latency_warnings: i32,

    // Personal bests
    pub best_wpm_session: Option<i64>,
    pub best_wpm_value: f64,
    pub longest_session_words: i32,
    pub longest_session_id: Option<i64>,
    pub lowest_latency_session: Option<i64>,
    pub lowest_latency_ms: f64,

    // Metadata
    pub last_updated: Option<DateTime<Utc>>,
}

impl Default for LifetimeMetrics {
    fn default() -> Self {
        Self {
            total_words: 0,
            total_characters: 0,
            total_sessions: 0,
            total_dictation_time_minutes: 0.0,
            total_segments: 0,
            average_wpm: 0.0,
            average_latency_ms: 0.0,
            typing_speed_equivalent: 40.0,
            speedup_factor: 1.0,
            estimated_time_saved_minutes: 0.0,
            wpm_trend_7day: 0.0,
            latency_trend_7day: 0.0,
            cuda_errors_total: 0,
            cuda_errors_recovered: 0,
            memory_pressure_events: 0,
            high_latency_warnings: 0,
            best_wpm_session: None,
            best_wpm_value: 0.0,
            longest_session_words: 0,
            longest_session_id: None,
            lowest_latency_session: None,
            lowest_latency_ms: 0.0,
            last_updated: None,
        }
    }
}

/// Real-time metrics during active recording (matches RealtimeMetrics dataclass)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeMetrics {
    // Current state
    pub current_state: DaemonState,
    pub recording_duration_s: f64,
    pub silence_duration_s: f64,
    pub speech_detected: bool,

    // Session progress
    pub current_session_id: Option<i64>,
    pub segments_this_session: i32,
    pub words_this_session: i32,
    pub wpm_this_session: f64,

    // Resource usage
    pub gpu_memory_current_mb: f64,
    pub gpu_memory_total_mb: f64,
    pub gpu_memory_percent: f64,
    pub cpu_percent_current: f64,

    // Last segment
    pub last_segment_words: i32,
    pub last_segment_latency_ms: f64,
    pub last_segment_wpm: f64,
    pub last_transcription: String,
}

impl Default for RealtimeMetrics {
    fn default() -> Self {
        Self {
            current_state: DaemonState::Idle,
            recording_duration_s: 0.0,
            silence_duration_s: 0.0,
            speech_detected: false,
            current_session_id: None,
            segments_this_session: 0,
            words_this_session: 0,
            wpm_this_session: 0.0,
            gpu_memory_current_mb: 0.0,
            gpu_memory_total_mb: 0.0,
            gpu_memory_percent: 0.0,
            cpu_percent_current: 0.0,
            last_segment_words: 0,
            last_segment_latency_ms: 0.0,
            last_segment_wpm: 0.0,
            last_transcription: String::new(),
        }
    }
}
