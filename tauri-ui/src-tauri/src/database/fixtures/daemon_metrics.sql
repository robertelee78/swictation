-- Schema fixture from swictation-metrics/src/database.rs at 314d646.
-- Only the daemon creates this schema in production.
CREATE TABLE IF NOT EXISTS sessions (
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
            );

CREATE TABLE IF NOT EXISTS segments (
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
            );

CREATE TABLE IF NOT EXISTS lifetime_stats (
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
            );

INSERT INTO lifetime_stats (id, last_updated) VALUES (1, 1700000000);
