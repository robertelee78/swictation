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
        let conn = Connection::open(&db_path)
            .context("Failed to open metrics database")?;

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
        let path_str = path.as_ref().to_str()
            .context("Invalid path encoding")?;

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

        let start_time = session.session_start
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

        let timestamp = segment.timestamp
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

        let mut stmt = conn.prepare(
            "SELECT * FROM sessions WHERE id = ?1"
        )?;

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

        let last_updated = metrics.last_updated
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

    /// Convert database row to SessionMetrics
    fn row_to_session(&self, row: &Row) -> Result<SessionMetrics> {
        let start_time: Option<f64> = row.get("start_time")?;
        let end_time: Option<f64> = row.get("end_time")?;

        Ok(SessionMetrics {
            session_id: row.get("id")?,
            session_start: start_time.map(|t| {
                DateTime::from_timestamp(t as i64, 0).unwrap_or_else(Utc::now)
            }),
            session_end: end_time.map(|t| {
                DateTime::from_timestamp(t as i64, 0).unwrap_or_else(Utc::now)
            }),
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
            last_updated: last_updated.map(|t| {
                DateTime::from_timestamp(t as i64, 0).unwrap_or_else(Utc::now)
            }),
        })
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
}
