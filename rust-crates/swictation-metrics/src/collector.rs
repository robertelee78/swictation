//! Metrics collection orchestrator
//!
//! Matches Python implementation in src/metrics/collector.py

use anyhow::Result;
use chrono::Utc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::info;

use crate::database::MetricsDatabase;
use crate::models::{RealtimeMetrics, SegmentMetrics, SessionMetrics};

/// Orchestrates metrics collection for Swictation daemon
pub struct MetricsCollector {
    db: Arc<MetricsDatabase>,
    typing_baseline_wpm: f64,
    store_transcription_text: bool,

    // Warning configuration
    warnings_enabled: bool,
    high_latency_threshold_ms: f64,
    gpu_memory_threshold_percent: f64,

    // Current session tracking
    current_session: Arc<Mutex<Option<SessionMetrics>>>,
    session_segments: Arc<Mutex<Vec<SegmentMetrics>>>,
    session_start_time: Arc<Mutex<Option<Instant>>>,
    active_time_accumulator: Arc<Mutex<f64>>,

    // Real-time metrics
    pub realtime: Arc<Mutex<RealtimeMetrics>>,
}

impl MetricsCollector {
    /// Create new metrics collector
    pub fn new(
        db_path: &str,
        typing_baseline_wpm: f64,
        store_transcription_text: bool,
        warnings_enabled: bool,
        high_latency_threshold_ms: f64,
        gpu_memory_threshold_percent: f64,
    ) -> Result<Self> {
        let db = Arc::new(MetricsDatabase::new(db_path)?);

        Ok(Self {
            db,
            typing_baseline_wpm,
            store_transcription_text,
            warnings_enabled,
            high_latency_threshold_ms,
            gpu_memory_threshold_percent,
            current_session: Arc::new(Mutex::new(None)),
            session_segments: Arc::new(Mutex::new(Vec::new())),
            session_start_time: Arc::new(Mutex::new(None)),
            active_time_accumulator: Arc::new(Mutex::new(0.0)),
            realtime: Arc::new(Mutex::new(RealtimeMetrics::default())),
        })
    }

    /// Start a new metrics session
    pub fn start_session(&self) -> Result<i64> {
        let now = Utc::now();
        let mut session = SessionMetrics::default();
        session.session_start = Some(now);
        session.typing_speed_equivalent = self.typing_baseline_wpm;

        // Insert into database (get ID)
        let session_id = self.db.insert_session(&session)?;
        session.session_id = Some(session_id);

        // Update state
        *self.current_session.lock().unwrap() = Some(session);
        self.session_segments.lock().unwrap().clear();
        *self.session_start_time.lock().unwrap() = Some(Instant::now());
        *self.active_time_accumulator.lock().unwrap() = 0.0;

        // Update realtime metrics
        {
            let mut realtime = self.realtime.lock().unwrap();
            realtime.current_session_id = Some(session_id);
            realtime.segments_this_session = 0;
            realtime.words_this_session = 0;
            realtime.wpm_this_session = 0.0;
        }

        info!("🎤 Recording started (Session #{})", session_id);

        Ok(session_id)
    }

    /// End current session and finalize metrics
    pub fn end_session(&self) -> Result<SessionMetrics> {
        let session_id = {
            let current = self.current_session.lock().unwrap();
            current
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("No active session to end"))?
                .session_id
                .ok_or_else(|| anyhow::anyhow!("Session has no ID"))?
        };

        // Finalize timing
        let now = Utc::now();
        let total_duration = self
            .session_start_time
            .lock()
            .unwrap()
            .map(|start| start.elapsed().as_secs_f64())
            .unwrap_or(0.0);

        let active_time = *self.active_time_accumulator.lock().unwrap();
        let pause_time = total_duration - active_time;

        // Calculate aggregate metrics from segments
        let segments = self.session_segments.lock().unwrap();
        let (avg_latency, median_latency, p95_latency, avg_words, avg_duration) =
            if !segments.is_empty() {
                let mut latencies: Vec<f64> = segments.iter().map(|s| s.total_latency_ms).collect();
                latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());

                let avg_lat = latencies.iter().sum::<f64>() / latencies.len() as f64;
                let median_lat = latencies[latencies.len() / 2];
                let p95_idx = (latencies.len() as f64 * 0.95) as usize;
                let p95_lat = latencies[p95_idx.min(latencies.len() - 1)];

                let words: Vec<i32> = segments.iter().map(|s| s.words).collect();
                let avg_w = words.iter().sum::<i32>() as f64 / words.len() as f64;

                let durations: Vec<f64> = segments.iter().map(|s| s.duration_s).collect();
                let avg_d = durations.iter().sum::<f64>() / durations.len() as f64;

                (avg_lat, median_lat, p95_lat, avg_w, avg_d)
            } else {
                (0.0, 0.0, 0.0, 0.0, 0.0)
            };

        // Update session metrics
        let mut session = {
            let mut current = self.current_session.lock().unwrap();
            current.take().unwrap()
        };

        session.session_end = Some(now);
        session.total_duration_s = total_duration;
        session.active_dictation_time_s = active_time;
        session.pause_time_s = pause_time;
        session.average_latency_ms = avg_latency;
        session.median_latency_ms = median_latency;
        session.p95_latency_ms = p95_latency;
        session.average_segment_words = avg_words;
        session.average_segment_duration_s = avg_duration;

        // Calculate WPM
        session.calculate_wpm();

        // Update database
        self.db.update_session(session_id, &session)?;

        info!("📊 Session #{} complete: {} words in {:.1}s ({:.1} WPM)",
              session_id,
              session.words_dictated,
              session.total_duration_s,
              session.words_per_minute);

        Ok(session)
    }

    /// Record a segment
    pub fn add_segment(&self, segment: SegmentMetrics) -> Result<()> {
        let session_id = {
            let current = self.current_session.lock().unwrap();
            current
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("No active session"))?
                .session_id
                .ok_or_else(|| anyhow::anyhow!("Session has no ID"))?
        };

        let mut seg = segment;
        seg.session_id = Some(session_id);
        seg.timestamp = Some(Utc::now());

        // Insert into database
        self.db.insert_segment(&seg, self.store_transcription_text)?;

        // Update session aggregates
        {
            let mut current = self.current_session.lock().unwrap();
            if let Some(ref mut session) = *current {
                session.words_dictated += seg.words;
                session.characters_typed += seg.characters;
                session.segments_processed += 1;
                session.transformations_count += seg.transformations_count;
                session.keyboard_actions_count += seg.keyboard_actions_count;
            }
        }

        // Add to segment list
        self.session_segments.lock().unwrap().push(seg.clone());

        // Update active time accumulator
        *self.active_time_accumulator.lock().unwrap() += seg.duration_s;

        // Update realtime metrics
        {
            let mut realtime = self.realtime.lock().unwrap();
            realtime.segments_this_session += 1;
            realtime.words_this_session += seg.words;
            realtime.last_segment_words = seg.words;
            realtime.last_segment_latency_ms = seg.total_latency_ms;
            realtime.last_segment_wpm = seg.calculate_wpm();
            realtime.last_transcription = seg.text.clone();

            // Calculate session WPM
            let active_time = *self.active_time_accumulator.lock().unwrap();
            if active_time > 0.0 {
                realtime.wpm_this_session = (realtime.words_this_session as f64 / active_time) * 60.0;
            }
        }

        // Check for warnings
        if self.warnings_enabled {
            if seg.total_latency_ms > self.high_latency_threshold_ms {
                info!("⚠️  High latency detected: {:.1}ms", seg.total_latency_ms);
            }
        }

        Ok(())
    }

    /// Update GPU memory metrics
    pub fn update_gpu_memory(&self, current_mb: f64, total_mb: f64) {
        let mut realtime = self.realtime.lock().unwrap();
        realtime.gpu_memory_current_mb = current_mb;
        realtime.gpu_memory_total_mb = total_mb;
        realtime.gpu_memory_percent = if total_mb > 0.0 {
            (current_mb / total_mb) * 100.0
        } else {
            0.0
        };

        // Track peak in session
        if let Some(ref mut session) = *self.current_session.lock().unwrap() {
            session.gpu_memory_peak_mb = session.gpu_memory_peak_mb.max(current_mb);
        }

        // Check threshold
        if self.warnings_enabled && realtime.gpu_memory_percent > self.gpu_memory_threshold_percent {
            info!("⚠️  High GPU memory usage: {:.1}%", realtime.gpu_memory_percent);
        }
    }

    /// Update CPU usage
    pub fn update_cpu_usage(&self, cpu_percent: f64) {
        let mut realtime = self.realtime.lock().unwrap();
        realtime.cpu_percent_current = cpu_percent;

        // Track peak in session
        if let Some(ref mut session) = *self.current_session.lock().unwrap() {
            session.cpu_usage_peak_percent = session.cpu_usage_peak_percent.max(cpu_percent);
        }
    }

    /// Update recording duration
    pub fn update_recording_duration(&self) {
        if let Some(start_time) = *self.session_start_time.lock().unwrap() {
            let mut realtime = self.realtime.lock().unwrap();
            realtime.recording_duration_s = start_time.elapsed().as_secs_f64();
        }
    }

    /// Get current realtime metrics (clone)
    pub fn get_realtime_metrics(&self) -> RealtimeMetrics {
        self.realtime.lock().unwrap().clone()
    }

    /// Check if session is active
    pub fn has_active_session(&self) -> bool {
        self.current_session.lock().unwrap().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_session_lifecycle() {
        let tmp_dir = TempDir::new().unwrap();
        let db_path = tmp_dir.path().join("test_metrics.db");

        let collector = MetricsCollector::new(
            db_path.to_str().unwrap(),
            40.0,
            false,
            true,
            1000.0,
            80.0,
        ).unwrap();

        // Start session
        let session_id = collector.start_session().unwrap();
        assert!(session_id > 0);
        assert!(collector.has_active_session());

        // Add segment
        let mut segment = SegmentMetrics::default();
        segment.words = 10;
        segment.duration_s = 2.0;
        segment.total_latency_ms = 500.0;
        collector.add_segment(segment).unwrap();

        // End session
        let session = collector.end_session().unwrap();
        assert_eq!(session.words_dictated, 10);
        assert!(!collector.has_active_session());
    }
}
