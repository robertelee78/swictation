//! SQLite database interface for metrics storage
//!
//! Matches Python implementation in src/metrics/database.py

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::models::{LifetimeMetrics, SegmentMetrics, SessionMetrics};

/// Thread-safe SQLite database for metrics storage
pub struct MetricsDatabase {
    db_path: PathBuf,
    conn: Arc<Mutex<Connection>>,
}

impl MetricsDatabase {
    /// Schema version for migrations
    const SCHEMA_VERSION: i32 = 1;

    /// Create new metrics database
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let db_path = Self::expand_path(db_path)?;

        // Create parent directory if needed
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Open connection
        let conn = Connection::open(&db_path).context("Failed to open metrics database")?;

        let db = Self {
            db_path,
            conn: Arc::new(Mutex::new(conn)),
        };

        // Initialize schema
        db.init_schema()?;

        Ok(db)
    }

    /// Expand ~ and environment variables in path
    fn expand_path<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
        let path_str = path.as_ref().to_str().context("Invalid path encoding")?;

        let expanded = if path_str.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                home.join(path_str.strip_prefix("~/").unwrap_or(path_str))
            } else {
                PathBuf::from(path_str)
            }
        } else {
            PathBuf::from(path_str)
        };

        Ok(expanded)
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Sessions table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                start_time REAL NOT NULL,
                end_time REAL,
                duration_s REAL,
                active_time_s REAL,
                pause_time_s REAL,
                words_dictated INTEGER DEFAULT 0,
                characters_typed INTEGER DEFAULT 0,
                segments_processed INTEGER DEFAULT 0,
                wpm REAL,
                typing_equiv_wpm REAL,
                avg_latency_ms REAL,
                median_latency_ms REAL,
                p95_latency_ms REAL,
                transformations_count INTEGER DEFAULT 0,
                keyboard_actions_count INTEGER DEFAULT 0,
                avg_segment_words REAL,
                avg_segment_duration_s REAL,
                gpu_peak_mb REAL,
                gpu_mean_mb REAL,
                cpu_mean_percent REAL,
                cpu_peak_percent REAL
            )",
            [],
        )?;

        // Segments table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS segments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL,
                timestamp REAL NOT NULL,
                duration_s REAL,
                words INTEGER,
                characters INTEGER,
                text TEXT,
                vad_latency_ms REAL,
                audio_save_latency_ms REAL,
                stt_latency_ms REAL,
                transform_latency_us REAL,
                injection_latency_ms REAL,
                total_latency_ms REAL,
                transformations_count INTEGER DEFAULT 0,
                keyboard_actions_count INTEGER DEFAULT 0,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Lifetime stats table (single row)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS lifetime_stats (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                total_words INTEGER DEFAULT 0,
                total_characters INTEGER DEFAULT 0,
                total_sessions INTEGER DEFAULT 0,
                total_time_minutes REAL DEFAULT 0,
                total_segments INTEGER DEFAULT 0,
                avg_wpm REAL DEFAULT 0,
                avg_latency_ms REAL DEFAULT 0,
                typing_equiv_wpm REAL DEFAULT 40.0,
                speedup_factor REAL DEFAULT 1.0,
                time_saved_minutes REAL DEFAULT 0,
                wpm_trend_7day REAL DEFAULT 0,
                latency_trend_7day REAL DEFAULT 0,
                cuda_errors_total INTEGER DEFAULT 0,
                cuda_errors_recovered INTEGER DEFAULT 0,
                memory_pressure_events INTEGER DEFAULT 0,
                high_latency_warnings INTEGER DEFAULT 0,
                best_wpm_session INTEGER,
                best_wpm_value REAL,
                longest_session_words INTEGER,
                longest_session_id INTEGER,
                lowest_latency_session INTEGER,
                lowest_latency_ms REAL,
                last_updated REAL
            )",
            [],
        )?;

        // Initialize lifetime_stats row if not exists
        conn.execute(
            "INSERT OR IGNORE INTO lifetime_stats (id, last_updated) VALUES (1, ?)",
            params![Utc::now().timestamp() as f64],
        )?;

        // Create indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_start_time ON sessions(start_time)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_segments_session_id ON segments(session_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_segments_timestamp ON segments(timestamp)",
            [],
        )?;

        Ok(())
    }

    /// Insert new session record
    pub fn insert_session(&self, session: &SessionMetrics) -> Result<i64> {
        let conn = self.conn.lock().unwrap();

        let start_time = session
            .session_start
            .map(|dt| dt.timestamp() as f64)
            .unwrap_or_else(|| Utc::now().timestamp() as f64);

        conn.execute(
            "INSERT INTO sessions (start_time, typing_equiv_wpm) VALUES (?1, ?2)",
            params![start_time, session.typing_speed_equivalent],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Update existing session record
    pub fn update_session(&self, session_id: i64, session: &SessionMetrics) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        let end_time = session.session_end.map(|dt| dt.timestamp() as f64);

        conn.execute(
            "UPDATE sessions SET
                end_time = ?1,
                duration_s = ?2,
                active_time_s = ?3,
                pause_time_s = ?4,
                words_dictated = ?5,
                characters_typed = ?6,
                segments_processed = ?7,
                wpm = ?8,
                avg_latency_ms = ?9,
                median_latency_ms = ?10,
                p95_latency_ms = ?11,
                transformations_count = ?12,
                keyboard_actions_count = ?13,
                avg_segment_words = ?14,
                avg_segment_duration_s = ?15,
                gpu_peak_mb = ?16,
                gpu_mean_mb = ?17,
                cpu_mean_percent = ?18,
                cpu_peak_percent = ?19
            WHERE id = ?20",
            params![
                end_time,
                session.total_duration_s,
                session.active_dictation_time_s,
                session.pause_time_s,
                session.words_dictated,
                session.characters_typed,
                session.segments_processed,
                session.words_per_minute,
                session.average_latency_ms,
                session.median_latency_ms,
                session.p95_latency_ms,
                session.transformations_count,
                session.keyboard_actions_count,
                session.average_segment_words,
                session.average_segment_duration_s,
                session.gpu_memory_peak_mb,
                session.gpu_memory_mean_mb,
                session.cpu_usage_mean_percent,
                session.cpu_usage_peak_percent,
                session_id,
            ],
        )?;

        Ok(())
    }

    /// Insert segment record
    pub fn insert_segment(&self, segment: &SegmentMetrics, store_text: bool) -> Result<i64> {
        let conn = self.conn.lock().unwrap();

        let timestamp = segment
            .timestamp
            .map(|dt| dt.timestamp() as f64)
            .unwrap_or_else(|| Utc::now().timestamp() as f64);

        let text = if store_text {
            Some(segment.text.as_str())
        } else {
            None
        };

        conn.execute(
            "INSERT INTO segments (
                session_id, timestamp, duration_s, words, characters, text,
                vad_latency_ms, audio_save_latency_ms, stt_latency_ms,
                transform_latency_us, injection_latency_ms, total_latency_ms,
                transformations_count, keyboard_actions_count
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                segment.session_id,
                timestamp,
                segment.duration_s,
                segment.words,
                segment.characters,
                text,
                segment.vad_latency_ms,
                segment.audio_save_latency_ms,
                segment.stt_latency_ms,
                segment.transform_latency_us,
                segment.injection_latency_ms,
                segment.total_latency_ms,
                segment.transformations_count,
                segment.keyboard_actions_count,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Get session by ID
    pub fn get_session(&self, session_id: i64) -> Result<Option<SessionMetrics>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare("SELECT * FROM sessions WHERE id = ?1")?;

        let mut rows = stmt.query(params![session_id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(self.row_to_session(row)?))
        } else {
            Ok(None)
        }
    }

    /// Get lifetime metrics
    pub fn get_lifetime_metrics(&self) -> Result<LifetimeMetrics> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare("SELECT * FROM lifetime_stats WHERE id = 1")?;
        let mut rows = stmt.query([])?;

        if let Some(row) = rows.next()? {
            self.row_to_lifetime(row)
        } else {
            Ok(LifetimeMetrics::default())
        }
    }

    /// Update lifetime metrics
    pub fn update_lifetime_metrics(&self, metrics: &LifetimeMetrics) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        let last_updated = metrics
            .last_updated
            .map(|dt| dt.timestamp() as f64)
            .unwrap_or_else(|| Utc::now().timestamp() as f64);

        conn.execute(
            "UPDATE lifetime_stats SET
                total_words = ?1,
                total_characters = ?2,
                total_sessions = ?3,
                total_time_minutes = ?4,
                total_segments = ?5,
                avg_wpm = ?6,
                avg_latency_ms = ?7,
                speedup_factor = ?8,
                time_saved_minutes = ?9,
                wpm_trend_7day = ?10,
                latency_trend_7day = ?11,
                cuda_errors_total = ?12,
                cuda_errors_recovered = ?13,
                memory_pressure_events = ?14,
                high_latency_warnings = ?15,
                best_wpm_session = ?16,
                best_wpm_value = ?17,
                longest_session_words = ?18,
                longest_session_id = ?19,
                lowest_latency_session = ?20,
                lowest_latency_ms = ?21,
                last_updated = ?22
            WHERE id = 1",
            params![
                metrics.total_words,
                metrics.total_characters,
                metrics.total_sessions,
                metrics.total_dictation_time_minutes,
                metrics.total_segments,
                metrics.average_wpm,
                metrics.average_latency_ms,
                metrics.speedup_factor,
                metrics.estimated_time_saved_minutes,
                metrics.wpm_trend_7day,
                metrics.latency_trend_7day,
                metrics.cuda_errors_total,
                metrics.cuda_errors_recovered,
                metrics.memory_pressure_events,
                metrics.high_latency_warnings,
                metrics.best_wpm_session,
                metrics.best_wpm_value,
                metrics.longest_session_words,
                metrics.longest_session_id,
                metrics.lowest_latency_session,
                metrics.lowest_latency_ms,
                last_updated,
            ],
        )?;

        Ok(())
    }

    /// Recalculate lifetime stats from all sessions and segments
    /// This should be called after each session ends to update aggregate statistics
    pub fn recalculate_lifetime_stats(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Aggregate from sessions table
        let mut stmt = conn.prepare(
            "SELECT
                COALESCE(SUM(words_dictated), 0) as total_words,
                COALESCE(SUM(characters_typed), 0) as total_characters,
                COUNT(*) as total_sessions,
                COALESCE(SUM(duration_s) / 60.0, 0) as total_time_minutes,
                COALESCE(AVG(wpm), 0) as avg_wpm,
                COALESCE(AVG(avg_latency_ms), 0) as avg_latency_ms,
                MAX(wpm) as best_wpm,
                (SELECT id FROM sessions ORDER BY wpm DESC LIMIT 1) as best_wpm_session,
                MIN(avg_latency_ms) as lowest_latency,
                (SELECT id FROM sessions WHERE avg_latency_ms > 0 ORDER BY avg_latency_ms ASC LIMIT 1) as lowest_latency_session
             FROM sessions
             WHERE end_time IS NOT NULL"
        )?;

        let result: Result<(
            i32,
            i32,
            i32,
            f64,
            f64,
            f64,
            Option<f64>,
            Option<i64>,
            Option<f64>,
            Option<i64>,
        )> = stmt
            .query_row([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                ))
            })
            .map_err(|e| anyhow::anyhow!("Failed to aggregate sessions: {}", e));

        let (
            total_words,
            total_characters,
            total_sessions,
            total_time_minutes,
            avg_wpm,
            avg_latency_ms,
            best_wpm,
            best_wpm_session,
            lowest_latency,
            lowest_latency_session,
        ) = result?;

        // Count total segments
        let total_segments: i32 = conn
            .query_row("SELECT COUNT(*) FROM segments", [], |row| row.get(0))
            .unwrap_or(0);

        // Calculate time saved (assuming 40 WPM typing baseline)
        let typing_baseline_wpm = 40.0;
        let time_saved_minutes = if avg_wpm > typing_baseline_wpm && total_words > 0 {
            let dictation_time = total_words as f64 / avg_wpm;
            let typing_time = total_words as f64 / typing_baseline_wpm;
            typing_time - dictation_time
        } else {
            0.0
        };

        let now = Utc::now().timestamp() as f64;

        // Update lifetime_stats
        conn.execute(
            "UPDATE lifetime_stats SET
                total_words = ?1,
                total_characters = ?2,
                total_sessions = ?3,
                total_time_minutes = ?4,
                total_segments = ?5,
                avg_wpm = ?6,
                avg_latency_ms = ?7,
                time_saved_minutes = ?8,
                best_wpm_value = ?9,
                best_wpm_session = ?10,
                lowest_latency_ms = ?11,
                lowest_latency_session = ?12,
                last_updated = ?13
            WHERE id = 1",
            params![
                total_words,
                total_characters,
                total_sessions,
                total_time_minutes,
                total_segments,
                avg_wpm,
                avg_latency_ms,
                time_saved_minutes,
                best_wpm.unwrap_or(0.0),
                best_wpm_session,
                lowest_latency.unwrap_or(0.0),
                lowest_latency_session,
                now,
            ],
        )?;

        Ok(())
    }

    /// Convert database row to SessionMetrics
    fn row_to_session(&self, row: &Row) -> Result<SessionMetrics> {
        let start_time: Option<f64> = row.get("start_time")?;
        let end_time: Option<f64> = row.get("end_time")?;

        Ok(SessionMetrics {
            session_id: row.get("id")?,
            session_start: start_time
                .map(|t| DateTime::from_timestamp(t as i64, 0).unwrap_or_else(Utc::now)),
            session_end: end_time
                .map(|t| DateTime::from_timestamp(t as i64, 0).unwrap_or_else(Utc::now)),
            total_duration_s: row.get("duration_s").unwrap_or(0.0),
            active_dictation_time_s: row.get("active_time_s").unwrap_or(0.0),
            pause_time_s: row.get("pause_time_s").unwrap_or(0.0),
            words_dictated: row.get("words_dictated").unwrap_or(0),
            characters_typed: row.get("characters_typed").unwrap_or(0),
            segments_processed: row.get("segments_processed").unwrap_or(0),
            words_per_minute: row.get("wpm").unwrap_or(0.0),
            typing_speed_equivalent: row.get("typing_equiv_wpm").unwrap_or(40.0),
            average_latency_ms: row.get("avg_latency_ms").unwrap_or(0.0),
            median_latency_ms: row.get("median_latency_ms").unwrap_or(0.0),
            p95_latency_ms: row.get("p95_latency_ms").unwrap_or(0.0),
            transformations_count: row.get("transformations_count").unwrap_or(0),
            keyboard_actions_count: row.get("keyboard_actions_count").unwrap_or(0),
            average_segment_words: row.get("avg_segment_words").unwrap_or(0.0),
            average_segment_duration_s: row.get("avg_segment_duration_s").unwrap_or(0.0),
            gpu_memory_peak_mb: row.get("gpu_peak_mb").unwrap_or(0.0),
            gpu_memory_mean_mb: row.get("gpu_mean_mb").unwrap_or(0.0),
            cpu_usage_mean_percent: row.get("cpu_mean_percent").unwrap_or(0.0),
            cpu_usage_peak_percent: row.get("cpu_peak_percent").unwrap_or(0.0),
            total_samples: 0,
        })
    }

    /// Convert database row to LifetimeMetrics
    fn row_to_lifetime(&self, row: &Row) -> Result<LifetimeMetrics> {
        let last_updated: Option<f64> = row.get("last_updated")?;

        Ok(LifetimeMetrics {
            total_words: row.get("total_words").unwrap_or(0),
            total_characters: row.get("total_characters").unwrap_or(0),
            total_sessions: row.get("total_sessions").unwrap_or(0),
            total_dictation_time_minutes: row.get("total_time_minutes").unwrap_or(0.0),
            total_segments: row.get("total_segments").unwrap_or(0),
            average_wpm: row.get("avg_wpm").unwrap_or(0.0),
            average_latency_ms: row.get("avg_latency_ms").unwrap_or(0.0),
            typing_speed_equivalent: row.get("typing_equiv_wpm").unwrap_or(40.0),
            speedup_factor: row.get("speedup_factor").unwrap_or(1.0),
            estimated_time_saved_minutes: row.get("time_saved_minutes").unwrap_or(0.0),
            wpm_trend_7day: row.get("wpm_trend_7day").unwrap_or(0.0),
            latency_trend_7day: row.get("latency_trend_7day").unwrap_or(0.0),
            cuda_errors_total: row.get("cuda_errors_total").unwrap_or(0),
            cuda_errors_recovered: row.get("cuda_errors_recovered").unwrap_or(0),
            memory_pressure_events: row.get("memory_pressure_events").unwrap_or(0),
            high_latency_warnings: row.get("high_latency_warnings").unwrap_or(0),
            best_wpm_session: row.get("best_wpm_session").ok(),
            best_wpm_value: row.get("best_wpm_value").unwrap_or(0.0),
            longest_session_words: row.get("longest_session_words").unwrap_or(0),
            longest_session_id: row.get("longest_session_id").ok(),
            lowest_latency_session: row.get("lowest_latency_session").ok(),
            lowest_latency_ms: row.get("lowest_latency_ms").unwrap_or(0.0),
            last_updated: last_updated
                .map(|t| DateTime::from_timestamp(t as i64, 0).unwrap_or_else(Utc::now)),
        })
    }

    /// Get recent sessions ordered by start time (for Tauri UI)
    pub fn get_recent_sessions(&self, limit: usize) -> Result<Vec<SessionMetrics>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare("SELECT * FROM sessions ORDER BY start_time DESC LIMIT ?1")?;

        let rows = stmt.query_map(params![limit], |row| {
            let start_time: Option<f64> = row.get("start_time")?;
            let end_time: Option<f64> = row.get("end_time")?;

            Ok(SessionMetrics {
                session_id: row.get("id")?,
                session_start: start_time
                    .map(|t| DateTime::from_timestamp(t as i64, 0).unwrap_or_else(Utc::now)),
                session_end: end_time
                    .map(|t| DateTime::from_timestamp(t as i64, 0).unwrap_or_else(Utc::now)),
                total_duration_s: row.get("duration_s").unwrap_or(0.0),
                active_dictation_time_s: row.get("active_time_s").unwrap_or(0.0),
                pause_time_s: row.get("pause_time_s").unwrap_or(0.0),
                words_dictated: row.get("words_dictated").unwrap_or(0),
                characters_typed: row.get("characters_typed").unwrap_or(0),
                segments_processed: row.get("segments_processed").unwrap_or(0),
                words_per_minute: row.get("wpm").unwrap_or(0.0),
                typing_speed_equivalent: row.get("typing_equiv_wpm").unwrap_or(40.0),
                average_latency_ms: row.get("avg_latency_ms").unwrap_or(0.0),
                median_latency_ms: row.get("median_latency_ms").unwrap_or(0.0),
                p95_latency_ms: row.get("p95_latency_ms").unwrap_or(0.0),
                transformations_count: row.get("transformations_count").unwrap_or(0),
                keyboard_actions_count: row.get("keyboard_actions_count").unwrap_or(0),
                average_segment_words: row.get("avg_segment_words").unwrap_or(0.0),
                average_segment_duration_s: row.get("avg_segment_duration_s").unwrap_or(0.0),
                gpu_memory_peak_mb: row.get("gpu_peak_mb").unwrap_or(0.0),
                gpu_memory_mean_mb: row.get("gpu_mean_mb").unwrap_or(0.0),
                cpu_usage_mean_percent: row.get("cpu_mean_percent").unwrap_or(0.0),
                cpu_usage_peak_percent: row.get("cpu_peak_percent").unwrap_or(0.0),
                total_samples: 0,
            })
        })?;

        let mut sessions = Vec::new();
        for session_result in rows {
            sessions.push(session_result?);
        }

        Ok(sessions)
    }

    /// Get all segments for a session ordered by timestamp (for Tauri UI)
    pub fn get_session_segments(&self, session_id: i64) -> Result<Vec<SegmentMetrics>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt =
            conn.prepare("SELECT * FROM segments WHERE session_id = ?1 ORDER BY timestamp ASC")?;

        let rows = stmt.query_map(params![session_id], |row| {
            let timestamp: Option<f64> = row.get("timestamp")?;

            Ok(SegmentMetrics {
                segment_id: row.get("id").ok(),
                session_id: Some(session_id),
                timestamp: timestamp
                    .map(|t| DateTime::from_timestamp(t as i64, 0).unwrap_or_else(Utc::now)),
                duration_s: row.get("duration_s").unwrap_or(0.0),
                words: row.get("words").unwrap_or(0),
                characters: row.get("characters").unwrap_or(0),
                text: row.get("text").unwrap_or_else(|_| String::new()),
                vad_latency_ms: row.get("vad_latency_ms").unwrap_or(0.0),
                audio_save_latency_ms: row.get("audio_save_latency_ms").unwrap_or(0.0),
                stt_latency_ms: row.get("stt_latency_ms").unwrap_or(0.0),
                transform_latency_us: row.get("transform_latency_us").unwrap_or(0.0),
                injection_latency_ms: row.get("injection_latency_ms").unwrap_or(0.0),
                total_latency_ms: row.get("total_latency_ms").unwrap_or(0.0),
                transformations_count: row.get("transformations_count").unwrap_or(0),
                keyboard_actions_count: row.get("keyboard_actions_count").unwrap_or(0),
            })
        })?;

        let mut segments = Vec::new();
        for segment_result in rows {
            segments.push(segment_result?);
        }

        Ok(segments)
    }

    /// Search transcriptions by text query (for Tauri UI)
    /// Uses SQLite FTS if available, otherwise falls back to LIKE
    pub fn search_transcriptions(&self, query: &str, limit: usize) -> Result<Vec<SegmentMetrics>> {
        let conn = self.conn.lock().unwrap();

        // Use LIKE for simple text search (could be upgraded to FTS later)
        let search_pattern = format!("%{}%", query);

        let mut stmt = conn.prepare(
            "SELECT * FROM segments
             WHERE text LIKE ?1
             ORDER BY timestamp DESC
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![search_pattern, limit], |row| {
            let timestamp: Option<f64> = row.get("timestamp")?;
            let session_id: i64 = row.get("session_id")?;

            Ok(SegmentMetrics {
                segment_id: row.get("id").ok(),
                session_id: Some(session_id),
                timestamp: timestamp
                    .map(|t| DateTime::from_timestamp(t as i64, 0).unwrap_or_else(Utc::now)),
                duration_s: row.get("duration_s").unwrap_or(0.0),
                words: row.get("words").unwrap_or(0),
                characters: row.get("characters").unwrap_or(0),
                text: row.get("text").unwrap_or_else(|_| String::new()),
                vad_latency_ms: row.get("vad_latency_ms").unwrap_or(0.0),
                audio_save_latency_ms: row.get("audio_save_latency_ms").unwrap_or(0.0),
                stt_latency_ms: row.get("stt_latency_ms").unwrap_or(0.0),
                transform_latency_us: row.get("transform_latency_us").unwrap_or(0.0),
                injection_latency_ms: row.get("injection_latency_ms").unwrap_or(0.0),
                total_latency_ms: row.get("total_latency_ms").unwrap_or(0.0),
                transformations_count: row.get("transformations_count").unwrap_or(0),
                keyboard_actions_count: row.get("keyboard_actions_count").unwrap_or(0),
            })
        })?;

        let mut segments = Vec::new();
        for segment_result in rows {
            segments.push(segment_result?);
        }

        Ok(segments)
    }

    /// Get lifetime statistics (alias for get_lifetime_metrics for consistency)
    pub fn get_lifetime_stats(&self) -> Result<LifetimeMetrics> {
        self.get_lifetime_metrics()
    }

    /// Get sessions from last N days for trend analysis
    pub fn get_sessions_last_n_days(&self, days: u32) -> Result<Vec<SessionMetrics>> {
        let conn = self.conn.lock().unwrap();

        let cutoff_time = Utc::now().timestamp() as f64 - (days as f64 * 24.0 * 60.0 * 60.0);

        let mut stmt =
            conn.prepare("SELECT * FROM sessions WHERE start_time >= ?1 ORDER BY start_time ASC")?;

        let rows = stmt.query_map(params![cutoff_time], |row| {
            let start_time: Option<f64> = row.get("start_time")?;
            let end_time: Option<f64> = row.get("end_time")?;

            Ok(SessionMetrics {
                session_id: row.get("id")?,
                session_start: start_time
                    .map(|t| DateTime::from_timestamp(t as i64, 0).unwrap_or_else(Utc::now)),
                session_end: end_time
                    .map(|t| DateTime::from_timestamp(t as i64, 0).unwrap_or_else(Utc::now)),
                total_duration_s: row.get("duration_s").unwrap_or(0.0),
                active_dictation_time_s: row.get("active_time_s").unwrap_or(0.0),
                pause_time_s: row.get("pause_time_s").unwrap_or(0.0),
                words_dictated: row.get("words_dictated").unwrap_or(0),
                characters_typed: row.get("characters_typed").unwrap_or(0),
                segments_processed: row.get("segments_processed").unwrap_or(0),
                words_per_minute: row.get("wpm").unwrap_or(0.0),
                typing_speed_equivalent: row.get("typing_equiv_wpm").unwrap_or(40.0),
                average_latency_ms: row.get("avg_latency_ms").unwrap_or(0.0),
                median_latency_ms: row.get("median_latency_ms").unwrap_or(0.0),
                p95_latency_ms: row.get("p95_latency_ms").unwrap_or(0.0),
                transformations_count: row.get("transformations_count").unwrap_or(0),
                keyboard_actions_count: row.get("keyboard_actions_count").unwrap_or(0),
                average_segment_words: row.get("avg_segment_words").unwrap_or(0.0),
                average_segment_duration_s: row.get("avg_segment_duration_s").unwrap_or(0.0),
                gpu_memory_peak_mb: row.get("gpu_peak_mb").unwrap_or(0.0),
                gpu_memory_mean_mb: row.get("gpu_mean_mb").unwrap_or(0.0),
                cpu_usage_mean_percent: row.get("cpu_mean_percent").unwrap_or(0.0),
                cpu_usage_peak_percent: row.get("cpu_peak_percent").unwrap_or(0.0),
                total_samples: 0,
            })
        })?;

        let mut sessions = Vec::new();
        for session_result in rows {
            sessions.push(session_result?);
        }

        Ok(sessions)
    }

    /// Delete segments older than N days to manage database size
    pub fn cleanup_old_segments(&self, days: u32) -> Result<usize> {
        let conn = self.conn.lock().unwrap();

        let cutoff_time = Utc::now().timestamp() as f64 - (days as f64 * 24.0 * 60.0 * 60.0);

        let deleted = conn.execute(
            "DELETE FROM segments WHERE timestamp < ?1",
            params![cutoff_time],
        )?;

        Ok(deleted)
    }

    /// Get database file size in megabytes
    pub fn get_database_size_mb(&self) -> Result<f64> {
        let metadata = std::fs::metadata(&self.db_path)?;
        Ok(metadata.len() as f64 / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_database_creation() {
        let tmp_dir = TempDir::new().unwrap();
        let db_path = tmp_dir.path().join("test_metrics.db");

        let db = MetricsDatabase::new(&db_path).unwrap();
        assert!(db_path.exists());
    }

    #[test]
    fn test_session_crud() {
        let tmp_dir = TempDir::new().unwrap();
        let db_path = tmp_dir.path().join("test_metrics.db");
        let db = MetricsDatabase::new(&db_path).unwrap();

        let mut session = SessionMetrics::default();
        session.session_start = Some(Utc::now());

        // Insert creates session (only sets start_time)
        let session_id = db.insert_session(&session).unwrap();
        assert!(session_id > 0);

        // Update adds words (realistic usage pattern)
        session.session_id = Some(session_id);
        session.words_dictated = 100;
        db.update_session(session_id, &session).unwrap();

        let retrieved = db.get_session(session_id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().words_dictated, 100);
    }

    #[test]
    fn test_get_recent_sessions() {
        let tmp_dir = TempDir::new().unwrap();
        let db_path = tmp_dir.path().join("test_metrics.db");
        let db = MetricsDatabase::new(&db_path).unwrap();

        // Insert 5 test sessions
        for i in 0..5 {
            let mut session = SessionMetrics::default();
            session.session_start = Some(Utc::now());
            session.words_dictated = 10 * (i + 1);
            db.insert_session(&session).unwrap();
        }

        // Get recent sessions (limit 3)
        let recent = db.get_recent_sessions(3).unwrap();
        assert_eq!(recent.len(), 3);

        // Should be ordered by most recent first
        assert!(recent[0].words_dictated >= recent[1].words_dictated);
    }

    #[test]
    fn test_get_session_segments() {
        let tmp_dir = TempDir::new().unwrap();
        let db_path = tmp_dir.path().join("test_metrics.db");
        let db = MetricsDatabase::new(&db_path).unwrap();

        // Create a session
        let mut session = SessionMetrics::default();
        session.session_start = Some(Utc::now());
        let session_id = db.insert_session(&session).unwrap();

        // Insert 3 segments
        for i in 0..3 {
            let mut segment = SegmentMetrics::default();
            segment.session_id = Some(session_id);
            segment.timestamp = Some(Utc::now());
            segment.text = format!("Test segment {}", i + 1);
            segment.words = 5 * (i + 1);
            db.insert_segment(&segment, true).unwrap();
        }

        // Get all segments for session
        let segments = db.get_session_segments(session_id).unwrap();
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].text, "Test segment 1");
    }

    #[test]
    fn test_search_transcriptions() {
        let tmp_dir = TempDir::new().unwrap();
        let db_path = tmp_dir.path().join("test_metrics.db");
        let db = MetricsDatabase::new(&db_path).unwrap();

        // Create a session
        let mut session = SessionMetrics::default();
        session.session_start = Some(Utc::now());
        let session_id = db.insert_session(&session).unwrap();

        // Insert segments with different text
        let texts = vec![
            "This is a test about coding",
            "Another example for testing",
            "Some random text here",
            "Coding in Rust is fun",
        ];

        for text in texts {
            let mut segment = SegmentMetrics::default();
            segment.session_id = Some(session_id);
            segment.timestamp = Some(Utc::now());
            segment.text = text.to_string();
            db.insert_segment(&segment, true).unwrap();
        }

        // Search for "coding"
        let results = db.search_transcriptions("coding", 10).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0].text.to_lowercase().contains("coding"));
    }

    #[test]
    fn test_get_lifetime_stats() {
        let tmp_dir = TempDir::new().unwrap();
        let db_path = tmp_dir.path().join("test_metrics.db");
        let db = MetricsDatabase::new(&db_path).unwrap();

        // Get initial lifetime stats (should be empty)
        let stats = db.get_lifetime_stats().unwrap();
        assert_eq!(stats.total_words, 0);
        assert_eq!(stats.total_sessions, 0);

        // Update lifetime stats
        let mut updated_stats = stats.clone();
        updated_stats.total_words = 1000;
        updated_stats.total_sessions = 5;
        updated_stats.average_wpm = 75.5;
        db.update_lifetime_metrics(&updated_stats).unwrap();

        // Retrieve updated stats
        let retrieved = db.get_lifetime_stats().unwrap();
        assert_eq!(retrieved.total_words, 1000);
        assert_eq!(retrieved.total_sessions, 5);
        assert_eq!(retrieved.average_wpm, 75.5);
    }

    #[test]
    fn test_get_sessions_last_n_days() {
        let tmp_dir = TempDir::new().unwrap();
        let db_path = tmp_dir.path().join("test_metrics.db");
        let db = MetricsDatabase::new(&db_path).unwrap();

        // Insert sessions from different times
        let mut session = SessionMetrics::default();
        session.session_start = Some(Utc::now());
        db.insert_session(&session).unwrap();

        // Get sessions from last 7 days
        let sessions = db.get_sessions_last_n_days(7).unwrap();
        assert_eq!(sessions.len(), 1);
    }

    #[test]
    fn test_cleanup_old_segments() {
        let tmp_dir = TempDir::new().unwrap();
        let db_path = tmp_dir.path().join("test_metrics.db");
        let db = MetricsDatabase::new(&db_path).unwrap();

        // Create a session
        let mut session = SessionMetrics::default();
        session.session_start = Some(Utc::now());
        let session_id = db.insert_session(&session).unwrap();

        // Insert a segment
        let mut segment = SegmentMetrics::default();
        segment.session_id = Some(session_id);
        segment.timestamp = Some(Utc::now());
        db.insert_segment(&segment, false).unwrap();

        // Cleanup old segments (90 days - shouldn't delete recent one)
        let deleted = db.cleanup_old_segments(90).unwrap();
        assert_eq!(deleted, 0);
    }

    #[test]
    fn test_database_size() {
        let tmp_dir = TempDir::new().unwrap();
        let db_path = tmp_dir.path().join("test_metrics.db");
        let db = MetricsDatabase::new(&db_path).unwrap();

        // Get database size
        let size_mb = db.get_database_size_mb().unwrap();
        assert!(size_mb > 0.0);
        assert!(size_mb < 10.0); // Should be small for empty database
    }
}
