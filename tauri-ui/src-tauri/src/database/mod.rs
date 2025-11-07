use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection, Row};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::models::{LifetimeStats, SessionSummary, TranscriptionRecord};

/// Thread-safe database wrapper for UI queries
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Open existing metrics database
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let db_path = Self::expand_path(db_path)?;

        // Ensure the database exists
        if !db_path.exists() {
            anyhow::bail!("Metrics database not found at {:?}", db_path);
        }

        let conn = Connection::open(&db_path)
            .context("Failed to open metrics database")?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
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

    /// Get recent sessions with transcription counts
    pub fn get_recent_sessions(&self, limit: usize) -> Result<Vec<SessionSummary>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT
                s.id,
                s.start_time,
                s.end_time,
                s.duration_s,
                s.words_dictated,
                s.segments_processed,
                s.wpm
             FROM sessions s
             ORDER BY s.start_time DESC
             LIMIT ?1"
        )?;

        let sessions = stmt.query_map([limit], |row| {
            let start_time: f64 = row.get(1)?;
            let end_time: Option<f64> = row.get(2)?;
            let duration_s: Option<f64> = row.get(3)?;

            Ok(SessionSummary {
                id: row.get(0)?,
                start_time: start_time as i64,
                end_time: end_time.map(|t| t as i64),
                duration_ms: duration_s.map(|d| (d * 1000.0) as i64),
                words_dictated: row.get(4)?,
                segments_count: row.get(5)?,
                wpm: row.get(6)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(sessions)
    }

    /// Get all transcriptions for a session (from segments table)
    pub fn get_session_transcriptions(&self, session_id: i64) -> Result<Vec<TranscriptionRecord>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT
                id,
                session_id,
                text,
                timestamp,
                total_latency_ms,
                words
             FROM segments
             WHERE session_id = ?1 AND text IS NOT NULL
             ORDER BY timestamp ASC"
        )?;

        let transcriptions = stmt.query_map([session_id], |row| {
            let timestamp: f64 = row.get(3)?;

            Ok(TranscriptionRecord {
                id: row.get(0)?,
                session_id: row.get(1)?,
                text: row.get(2)?,
                timestamp: timestamp as i64,
                latency_ms: row.get(4)?,
                words: row.get(5)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(transcriptions)
    }

    /// Search transcriptions by text content
    pub fn search_transcriptions(&self, query: &str, limit: usize) -> Result<Vec<TranscriptionRecord>> {
        let conn = self.conn.lock().unwrap();

        let search_pattern = format!("%{}%", query);

        let mut stmt = conn.prepare(
            "SELECT
                id,
                session_id,
                text,
                timestamp,
                total_latency_ms,
                words
             FROM segments
             WHERE text IS NOT NULL AND text LIKE ?1
             ORDER BY timestamp DESC
             LIMIT ?2"
        )?;

        let transcriptions = stmt.query_map(params![search_pattern, limit], |row| {
            let timestamp: f64 = row.get(3)?;

            Ok(TranscriptionRecord {
                id: row.get(0)?,
                session_id: row.get(1)?,
                text: row.get(2)?,
                timestamp: timestamp as i64,
                latency_ms: row.get(4)?,
                words: row.get(5)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(transcriptions)
    }

    /// Get lifetime statistics
    pub fn get_lifetime_stats(&self) -> Result<LifetimeStats> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT
                total_words,
                total_characters,
                total_sessions,
                total_time_minutes,
                avg_wpm,
                avg_latency_ms,
                best_wpm_value,
                best_wpm_session
             FROM lifetime_stats
             WHERE id = 1"
        )?;

        let mut rows = stmt.query([])?;

        if let Some(row) = rows.next()? {
            Ok(LifetimeStats {
                total_words: row.get(0)?,
                total_characters: row.get(1)?,
                total_sessions: row.get(2)?,
                total_time_minutes: row.get(3)?,
                average_wpm: row.get(4)?,
                average_latency_ms: row.get(5)?,
                best_wpm_value: row.get(6)?,
                best_wpm_session: row.get(7)?,
            })
        } else {
            // Return empty stats if no data exists yet
            Ok(LifetimeStats {
                total_words: 0,
                total_characters: 0,
                total_sessions: 0,
                total_time_minutes: 0.0,
                average_wpm: 0.0,
                average_latency_ms: 0.0,
                best_wpm_value: 0.0,
                best_wpm_session: None,
            })
        }
    }
}
