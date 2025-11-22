//! WebAssembly bindings for swictation-metrics
//!
//! Exposes database query functions to JavaScript/Node.js via wasm-bindgen.
//!
//! ## Usage from Node.js
//!
//! ```javascript
//! const { MetricsDatabaseWasm } = require('swictation-metrics');
//!
//! const db = new MetricsDatabaseWasm('~/.local/share/swictation/metrics.db');
//! const sessions = db.get_recent_sessions(10);
//! console.log(JSON.parse(sessions));
//! ```

use crate::database::MetricsDatabase;
use crate::models::{LifetimeMetrics, SegmentMetrics, SessionMetrics};
use wasm_bindgen::prelude::*;

/// WebAssembly wrapper for MetricsDatabase
///
/// All methods return JSON strings for easy JavaScript interop.
#[wasm_bindgen]
pub struct MetricsDatabaseWasm {
    db: MetricsDatabase,
}

#[wasm_bindgen]
impl MetricsDatabaseWasm {
    /// Create a new database connection
    ///
    /// # Arguments
    /// * `db_path` - Path to SQLite database (e.g., "~/.local/share/swictation/metrics.db")
    ///
    /// # Returns
    /// Database instance
    #[wasm_bindgen(constructor)]
    pub fn new(db_path: &str) -> Result<MetricsDatabaseWasm, JsValue> {
        let db = MetricsDatabase::new(db_path)
            .map_err(|e| JsValue::from_str(&format!("Failed to open database: {}", e)))?;

        Ok(MetricsDatabaseWasm { db })
    }

    /// Get recent sessions
    ///
    /// # Arguments
    /// * `limit` - Maximum number of sessions to return (default: 10)
    ///
    /// # Returns
    /// JSON string containing array of SessionMetrics
    ///
    /// # Example
    /// ```javascript
    /// const sessions = JSON.parse(db.get_recent_sessions(10));
    /// sessions.forEach(s => console.log(s.start_time, s.words_dictated));
    /// ```
    #[wasm_bindgen]
    pub fn get_recent_sessions(&self, limit: usize) -> Result<String, JsValue> {
        let sessions = self
            .db
            .get_recent_sessions(limit)
            .map_err(|e| JsValue::from_str(&format!("Query failed: {}", e)))?;

        serde_json::to_string(&sessions)
            .map_err(|e| JsValue::from_str(&format!("JSON serialization failed: {}", e)))
    }

    /// Get all transcription segments for a specific session
    ///
    /// # Arguments
    /// * `session_id` - Session ID from sessions table
    ///
    /// # Returns
    /// JSON string containing array of SegmentMetrics with transcription text
    ///
    /// # Example
    /// ```javascript
    /// const segments = JSON.parse(db.get_session_segments(123));
    /// segments.forEach(s => console.log(s.timestamp, s.text));
    /// ```
    #[wasm_bindgen]
    pub fn get_session_segments(&self, session_id: i64) -> Result<String, JsValue> {
        let segments = self
            .db
            .get_session_segments(session_id)
            .map_err(|e| JsValue::from_str(&format!("Query failed: {}", e)))?;

        serde_json::to_string(&segments)
            .map_err(|e| JsValue::from_str(&format!("JSON serialization failed: {}", e)))
    }

    /// Search transcriptions by text content
    ///
    /// # Arguments
    /// * `query` - Search term (SQLite LIKE query, case-insensitive)
    /// * `limit` - Maximum number of results (default: 20)
    ///
    /// # Returns
    /// JSON string containing array of SegmentMetrics matching the query
    ///
    /// # Example
    /// ```javascript
    /// const results = JSON.parse(db.search_transcriptions("hello world", 20));
    /// results.forEach(r => console.log(r.text));
    /// ```
    #[wasm_bindgen]
    pub fn search_transcriptions(&self, query: &str, limit: usize) -> Result<String, JsValue> {
        let results = self
            .db
            .search_transcriptions(query, limit)
            .map_err(|e| JsValue::from_str(&format!("Search failed: {}", e)))?;

        serde_json::to_string(&results)
            .map_err(|e| JsValue::from_str(&format!("JSON serialization failed: {}", e)))
    }

    /// Get lifetime statistics (aggregate metrics across all sessions)
    ///
    /// # Returns
    /// JSON string containing LifetimeMetrics with totals, averages, and records
    ///
    /// # Example
    /// ```javascript
    /// const stats = JSON.parse(db.get_lifetime_stats());
    /// console.log(`Total words: ${stats.total_words}`);
    /// console.log(`Best WPM: ${stats.best_wpm_value}`);
    /// ```
    #[wasm_bindgen]
    pub fn get_lifetime_stats(&self) -> Result<String, JsValue> {
        let stats = self
            .db
            .get_lifetime_stats()
            .map_err(|e| JsValue::from_str(&format!("Query failed: {}", e)))?;

        serde_json::to_string(&stats)
            .map_err(|e| JsValue::from_str(&format!("JSON serialization failed: {}", e)))
    }

    /// Get sessions from the last N days (for trend analysis)
    ///
    /// # Arguments
    /// * `days` - Number of days to look back (e.g., 7 for last week)
    ///
    /// # Returns
    /// JSON string containing array of SessionMetrics
    ///
    /// # Example
    /// ```javascript
    /// const last_week = JSON.parse(db.get_sessions_last_n_days(7));
    /// const wpm_trend = last_week.map(s => s.wpm);
    /// ```
    #[wasm_bindgen]
    pub fn get_sessions_last_n_days(&self, days: u32) -> Result<String, JsValue> {
        let sessions = self
            .db
            .get_sessions_last_n_days(days)
            .map_err(|e| JsValue::from_str(&format!("Query failed: {}", e)))?;

        serde_json::to_string(&sessions)
            .map_err(|e| JsValue::from_str(&format!("JSON serialization failed: {}", e)))
    }

    /// Get database file size in megabytes
    ///
    /// # Returns
    /// Size in MB as f64
    #[wasm_bindgen]
    pub fn get_database_size_mb(&self) -> Result<f64, JsValue> {
        self.db
            .get_database_size_mb()
            .map_err(|e| JsValue::from_str(&format!("Failed to get size: {}", e)))
    }
}

/// Initialize panic hook for better error messages in WASM
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
