"""
SQLite database interface for metrics storage.
"""

import sqlite3
import threading
from pathlib import Path
from typing import Optional, List, Dict, Any
from datetime import datetime
import time


class MetricsDatabase:
    """
    Thread-safe SQLite database for metrics storage.

    Features:
    - Automatic schema creation
    - Connection pooling per thread
    - Async write support
    - Transaction management
    """

    # Schema version for migrations
    SCHEMA_VERSION = 1

    def __init__(self, db_path: str = "~/.local/share/swictation/metrics.db"):
        """
        Initialize metrics database.

        Args:
            db_path: Path to SQLite database file
        """
        self.db_path = Path(db_path).expanduser()
        self.db_path.parent.mkdir(parents=True, exist_ok=True)

        # Thread-local storage for connections
        self._local = threading.local()

        # Initialize schema
        self._init_schema()

    def _get_connection(self) -> sqlite3.Connection:
        """Get thread-local database connection."""
        if not hasattr(self._local, 'conn'):
            self._local.conn = sqlite3.connect(
                str(self.db_path),
                check_same_thread=False,
                isolation_level=None  # Autocommit mode
            )
            self._local.conn.row_factory = sqlite3.Row
        return self._local.conn

    def _init_schema(self):
        """Initialize database schema if needed."""
        conn = self._get_connection()
        cursor = conn.cursor()

        # Sessions table
        cursor.execute("""
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
            )
        """)

        # Segments table
        cursor.execute("""
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
            )
        """)

        # Lifetime stats table (single row)
        cursor.execute("""
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
            )
        """)

        # Initialize lifetime_stats row if not exists
        cursor.execute("""
            INSERT OR IGNORE INTO lifetime_stats (id, last_updated)
            VALUES (1, ?)
        """, (time.time(),))

        # Create indexes
        cursor.execute("CREATE INDEX IF NOT EXISTS idx_sessions_start_time ON sessions(start_time)")
        cursor.execute("CREATE INDEX IF NOT EXISTS idx_segments_session_id ON segments(session_id)")
        cursor.execute("CREATE INDEX IF NOT EXISTS idx_segments_timestamp ON segments(timestamp)")

        conn.commit()

    def insert_session(self, session_data: Dict[str, Any]) -> int:
        """
        Insert new session record.

        Args:
            session_data: Session data dictionary

        Returns:
            Session ID
        """
        conn = self._get_connection()
        cursor = conn.cursor()

        # Remove session_id if present (auto-increment)
        session_data = {k: v for k, v in session_data.items() if k != 'session_id'}

        columns = ', '.join(session_data.keys())
        placeholders = ', '.join(['?' for _ in session_data])
        query = f"INSERT INTO sessions ({columns}) VALUES ({placeholders})"

        cursor.execute(query, list(session_data.values()))
        conn.commit()

        return cursor.lastrowid

    def update_session(self, session_id: int, session_data: Dict[str, Any]):
        """
        Update existing session record.

        Args:
            session_id: Session ID to update
            session_data: Updated session data
        """
        conn = self._get_connection()
        cursor = conn.cursor()

        # Build SET clause
        set_clause = ', '.join([f"{k} = ?" for k in session_data.keys()])
        query = f"UPDATE sessions SET {set_clause} WHERE id = ?"

        values = list(session_data.values()) + [session_id]
        cursor.execute(query, values)
        conn.commit()

    def insert_segment(self, segment_data: Dict[str, Any]) -> int:
        """
        Insert segment record.

        Args:
            segment_data: Segment data dictionary

        Returns:
            Segment ID
        """
        conn = self._get_connection()
        cursor = conn.cursor()

        # Remove segment_id if present
        segment_data = {k: v for k, v in segment_data.items() if k != 'segment_id'}

        columns = ', '.join(segment_data.keys())
        placeholders = ', '.join(['?' for _ in segment_data])
        query = f"INSERT INTO segments ({columns}) VALUES ({placeholders})"

        cursor.execute(query, list(segment_data.values()))
        conn.commit()

        return cursor.lastrowid

    def get_session(self, session_id: int) -> Optional[Dict[str, Any]]:
        """
        Get session by ID.

        Args:
            session_id: Session ID

        Returns:
            Session data dict or None
        """
        conn = self._get_connection()
        cursor = conn.cursor()

        cursor.execute("SELECT * FROM sessions WHERE id = ?", (session_id,))
        row = cursor.fetchone()

        return dict(row) if row else None

    def get_recent_sessions(self, limit: int = 10) -> List[Dict[str, Any]]:
        """
        Get recent sessions ordered by start time.

        Args:
            limit: Maximum number of sessions to return

        Returns:
            List of session dicts
        """
        conn = self._get_connection()
        cursor = conn.cursor()

        cursor.execute("""
            SELECT * FROM sessions
            ORDER BY start_time DESC
            LIMIT ?
        """, (limit,))

        return [dict(row) for row in cursor.fetchall()]

    def get_segments_for_session(self, session_id: int) -> List[Dict[str, Any]]:
        """
        Get all segments for a session.

        Args:
            session_id: Session ID

        Returns:
            List of segment dicts
        """
        conn = self._get_connection()
        cursor = conn.cursor()

        cursor.execute("""
            SELECT * FROM segments
            WHERE session_id = ?
            ORDER BY timestamp ASC
        """, (session_id,))

        return [dict(row) for row in cursor.fetchall()]

    def update_lifetime_stats(self, stats_data: Dict[str, Any]):
        """
        Update lifetime statistics (single row).

        Args:
            stats_data: Stats data dictionary
        """
        conn = self._get_connection()
        cursor = conn.cursor()

        # Always update timestamp
        stats_data['last_updated'] = time.time()

        # Build SET clause
        set_clause = ', '.join([f"{k} = ?" for k in stats_data.keys()])
        query = f"UPDATE lifetime_stats SET {set_clause} WHERE id = 1"

        cursor.execute(query, list(stats_data.values()))
        conn.commit()

    def get_lifetime_stats(self) -> Dict[str, Any]:
        """
        Get lifetime statistics.

        Returns:
            Lifetime stats dict
        """
        conn = self._get_connection()
        cursor = conn.cursor()

        cursor.execute("SELECT * FROM lifetime_stats WHERE id = 1")
        row = cursor.fetchone()

        return dict(row) if row else {}

    def get_sessions_last_n_days(self, days: int = 7) -> List[Dict[str, Any]]:
        """
        Get sessions from last N days for trend analysis.

        Args:
            days: Number of days to look back

        Returns:
            List of session dicts
        """
        conn = self._get_connection()
        cursor = conn.cursor()

        cutoff_time = time.time() - (days * 24 * 60 * 60)

        cursor.execute("""
            SELECT * FROM sessions
            WHERE start_time >= ?
            ORDER BY start_time ASC
        """, (cutoff_time,))

        return [dict(row) for row in cursor.fetchall()]

    def cleanup_old_data(self, days: int = 90):
        """
        Delete segments older than N days to manage database size.

        Args:
            days: Age threshold in days
        """
        conn = self._get_connection()
        cursor = conn.cursor()

        cutoff_time = time.time() - (days * 24 * 60 * 60)

        cursor.execute("DELETE FROM segments WHERE timestamp < ?", (cutoff_time,))
        conn.commit()

        # Note: Sessions are kept indefinitely for historical trends

    def get_database_size_mb(self) -> float:
        """
        Get database file size in MB.

        Returns:
            Size in megabytes
        """
        if self.db_path.exists():
            return self.db_path.stat().st_size / (1024 * 1024)
        return 0.0

    def close(self):
        """Close database connection."""
        if hasattr(self._local, 'conn'):
            self._local.conn.close()
            delattr(self._local, 'conn')
