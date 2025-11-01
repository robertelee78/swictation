"""
Metrics collection orchestrator.
"""

import time
import threading
from typing import Optional, List
from datetime import datetime
from contextlib import contextmanager
import numpy as np

from .models import SessionMetrics, SegmentMetrics, LifetimeMetrics, RealtimeMetrics
from .database import MetricsDatabase


class MetricsCollector:
    """
    Orchestrates metrics collection for Swictation daemon.

    Features:
    - Session context management
    - Segment tracking with timing
    - Async database writes
    - Lifetime stats updates
    - Real-time metrics
    """

    def __init__(
        self,
        db_path: str = "~/.local/share/swictation/metrics.db",
        typing_baseline_wpm: float = 40.0,
        store_transcription_text: bool = False
    ):
        """
        Initialize metrics collector.

        Args:
            db_path: Path to metrics database
            typing_baseline_wpm: Baseline typing speed for comparisons
            store_transcription_text: Whether to store full transcription text
        """
        self.db = MetricsDatabase(db_path)
        self.typing_baseline_wpm = typing_baseline_wpm
        self.store_transcription_text = store_transcription_text

        # Current session tracking
        self.current_session: Optional[SessionMetrics] = None
        self.session_segments: List[SegmentMetrics] = []
        self.session_start_time: Optional[float] = None
        self.active_time_accumulator: float = 0.0

        # Real-time metrics
        self.realtime = RealtimeMetrics()

        # Thread lock for session updates
        self._lock = threading.Lock()

    def start_session(self) -> SessionMetrics:
        """
        Start a new metrics session.

        Returns:
            SessionMetrics object for current session
        """
        with self._lock:
            now = datetime.now()
            self.current_session = SessionMetrics(
                session_start=now,
                typing_speed_equivalent=self.typing_baseline_wpm
            )
            self.session_segments = []
            self.session_start_time = time.time()
            self.active_time_accumulator = 0.0

            # Insert into database (get ID)
            session_data = {
                'start_time': now.timestamp(),
                'typing_equiv_wpm': self.typing_baseline_wpm
            }
            session_id = self.db.insert_session(session_data)
            self.current_session.session_id = session_id

            # Update realtime metrics
            self.realtime.current_session_id = session_id
            self.realtime.segments_this_session = 0
            self.realtime.words_this_session = 0
            self.realtime.wpm_this_session = 0.0

            print(f"🎤 Recording started (Session #{session_id})", flush=True)

            return self.current_session

    def end_session(self) -> SessionMetrics:
        """
        End current session and finalize metrics.

        Returns:
            Completed SessionMetrics
        """
        with self._lock:
            if not self.current_session:
                raise RuntimeError("No active session to end")

            # Finalize timing
            now = datetime.now()
            self.current_session.session_end = now
            self.current_session.total_duration_s = time.time() - self.session_start_time
            self.current_session.active_dictation_time_s = self.active_time_accumulator
            self.current_session.pause_time_s = (
                self.current_session.total_duration_s - self.active_time_accumulator
            )

            # Calculate aggregate metrics from segments
            if self.session_segments:
                latencies = [seg.total_latency_ms for seg in self.session_segments]
                segment_words = [seg.words for seg in self.session_segments]
                segment_durations = [seg.duration_s for seg in self.session_segments]

                self.current_session.average_latency_ms = np.mean(latencies)
                self.current_session.median_latency_ms = np.median(latencies)
                self.current_session.p95_latency_ms = np.percentile(latencies, 95)
                self.current_session.average_segment_words = np.mean(segment_words)
                self.current_session.average_segment_duration_s = np.mean(segment_durations)

            # Calculate WPM
            self.current_session.calculate_wpm()

            # Update database
            session_data = {
                'end_time': now.timestamp(),
                'duration_s': self.current_session.total_duration_s,
                'active_time_s': self.current_session.active_dictation_time_s,
                'pause_time_s': self.current_session.pause_time_s,
                'words_dictated': self.current_session.words_dictated,
                'characters_typed': self.current_session.characters_typed,
                'segments_processed': self.current_session.segments_processed,
                'wpm': self.current_session.words_per_minute,
                'avg_latency_ms': self.current_session.average_latency_ms,
                'median_latency_ms': self.current_session.median_latency_ms,
                'p95_latency_ms': self.current_session.p95_latency_ms,
                'transformations_count': self.current_session.transformations_count,
                'keyboard_actions_count': self.current_session.keyboard_actions_count,
                'avg_segment_words': self.current_session.average_segment_words,
                'avg_segment_duration_s': self.current_session.average_segment_duration_s,
                'gpu_peak_mb': self.current_session.gpu_memory_peak_mb,
                'gpu_mean_mb': self.current_session.gpu_memory_mean_mb,
                'cpu_mean_percent': self.current_session.cpu_usage_mean_percent,
                'cpu_peak_percent': self.current_session.cpu_usage_peak_percent,
            }
            self.db.update_session(self.current_session.session_id, session_data)

            # Update lifetime stats
            self._update_lifetime_stats()

            # Print summary
            print(f"\n🛑 Recording stopped\n", flush=True)
            print(f"📊 Session #{self.current_session.session_id} Summary:", flush=True)
            print(f"   ├─ Segments: {self.current_session.segments_processed}", flush=True)
            print(f"   ├─ Words: {self.current_session.words_dictated} ({self.current_session.characters_typed} characters)", flush=True)
            print(f"   ├─ Time: {self.current_session.active_dictation_time_s:.1f}s active / {self.current_session.total_duration_s:.1f}s total", flush=True)
            print(f"   ├─ Speed: {self.current_session.words_per_minute:.0f} wpm ({self.current_session.words_per_minute/self.typing_baseline_wpm:.1f}x faster than typing)", flush=True)
            print(f"   ├─ Latency: {self.current_session.average_latency_ms:.0f}ms avg ({self.current_session.median_latency_ms:.0f}-{self.current_session.p95_latency_ms:.0f}ms range)", flush=True)
            print(f"   └─ Status: ✓ Saved to database\n", flush=True)

            session = self.current_session
            self.current_session = None
            self.session_segments = []

            return session

    def record_segment(
        self,
        audio_duration_s: float,
        transcription: str,
        stt_latency_ms: float = 0.0,
        transform_latency_us: float = 0.0,
        injection_latency_ms: float = 0.0,
        gpu_memory_mb: float = 0.0,
        cpu_percent: float = 0.0
    ) -> SegmentMetrics:
        """
        Record metrics for a completed segment.

        Args:
            audio_duration_s: Duration of audio segment
            transcription: Transcribed text
            stt_latency_ms: STT inference latency
            transform_latency_us: Text transformation latency (microseconds)
            injection_latency_ms: Text injection latency
            gpu_memory_mb: GPU memory usage
            cpu_percent: CPU usage percentage

        Returns:
            SegmentMetrics object
        """
        with self._lock:
            if not self.current_session:
                raise RuntimeError("No active session")

            # Count words and characters
            words = self._count_words(transcription)
            characters = len(transcription)

            # Count transformations and keyboard actions
            transformations = transcription.count('<KEY:')
            keyboard_actions = transcription.count('<KEY:')

            # Calculate total latency
            total_latency_ms = (
                stt_latency_ms +
                (transform_latency_us / 1000.0) +
                injection_latency_ms
            )

            # Create segment metrics
            segment = SegmentMetrics(
                session_id=self.current_session.session_id,
                timestamp=datetime.now(),
                duration_s=audio_duration_s,
                words=words,
                characters=characters,
                text=transcription if self.store_transcription_text else "",
                vad_latency_ms=0.0,  # Set by caller if available
                audio_save_latency_ms=0.0,  # Set by caller if available
                stt_latency_ms=stt_latency_ms,
                transform_latency_us=transform_latency_us,
                injection_latency_ms=injection_latency_ms,
                total_latency_ms=total_latency_ms,
                transformations_count=transformations,
                keyboard_actions_count=keyboard_actions
            )

            # Store in database
            segment_data = segment.to_dict()
            segment_data.pop('segment_id', None)
            segment_id = self.db.insert_segment(segment_data)
            segment.segment_id = segment_id

            # Add to session tracking
            self.session_segments.append(segment)
            self.active_time_accumulator += audio_duration_s

            # Update session totals
            self.current_session.words_dictated += words
            self.current_session.characters_typed += characters
            self.current_session.segments_processed += 1
            self.current_session.transformations_count += transformations
            self.current_session.keyboard_actions_count += keyboard_actions

            # Update GPU/CPU tracking
            if gpu_memory_mb > self.current_session.gpu_memory_peak_mb:
                self.current_session.gpu_memory_peak_mb = gpu_memory_mb

            # Running average for GPU
            if self.current_session.segments_processed == 1:
                self.current_session.gpu_memory_mean_mb = gpu_memory_mb
            else:
                n = self.current_session.segments_processed
                self.current_session.gpu_memory_mean_mb = (
                    (self.current_session.gpu_memory_mean_mb * (n - 1) + gpu_memory_mb) / n
                )

            # Track CPU
            if cpu_percent > self.current_session.cpu_usage_peak_percent:
                self.current_session.cpu_usage_peak_percent = cpu_percent
            if self.current_session.segments_processed == 1:
                self.current_session.cpu_usage_mean_percent = cpu_percent
            else:
                n = self.current_session.segments_processed
                self.current_session.cpu_usage_mean_percent = (
                    (self.current_session.cpu_usage_mean_percent * (n - 1) + cpu_percent) / n
                )

            # Update realtime metrics
            self.realtime.segments_this_session = self.current_session.segments_processed
            self.realtime.words_this_session = self.current_session.words_dictated
            if self.active_time_accumulator > 0:
                self.realtime.wpm_this_session = (
                    self.current_session.words_dictated / self.active_time_accumulator
                ) * 60
            self.realtime.last_segment_words = words
            self.realtime.last_segment_latency_ms = total_latency_ms
            self.realtime.last_segment_wpm = segment.calculate_wpm()
            if self.store_transcription_text:
                self.realtime.last_transcription = transcription

            # Print real-time feedback
            self._print_segment_feedback(segment)

            return segment

    def _print_segment_feedback(self, segment: SegmentMetrics):
        """Print real-time segment feedback to stderr."""
        segment_num = self.current_session.segments_processed
        wpm = segment.calculate_wpm()

        # Color-code latency
        if segment.total_latency_ms < 500:
            latency_indicator = ""  # Green (implicit)
        elif segment.total_latency_ms < 1000:
            latency_indicator = ""  # Yellow
        else:
            latency_indicator = " ⚠️  (HIGH!)"  # Red

        print(f"✓ Segment {segment_num}: {segment.duration_s:.1f}s | "
              f"{segment.words} words | {wpm:.0f} wpm | "
              f"{segment.total_latency_ms:.0f}ms{latency_indicator}", flush=True)
        print(f"   └─ STT: {segment.stt_latency_ms:.0f}ms | "
              f"Transform: {segment.transform_latency_us:.1f}µs | "
              f"Inject: {segment.injection_latency_ms:.0f}ms\n", flush=True)

        # Check for performance warnings
        if self.current_session.segments_processed > 1:
            avg_latency = np.mean([s.total_latency_ms for s in self.session_segments[:-1]])
            if segment.total_latency_ms > 2.0 * avg_latency:
                print(f"   ⚠️  Performance warning: Latency {segment.total_latency_ms/avg_latency:.1f}x session average\n", flush=True)

    def _count_words(self, text: str) -> int:
        """
        Count words in text.

        Simple whitespace-based splitting (industry standard for WPM).

        Args:
            text: Text to count words in

        Returns:
            Word count
        """
        words = text.split()
        words = [w.strip() for w in words if w.strip()]
        return len(words)

    def _update_lifetime_stats(self):
        """Update lifetime statistics incrementally."""
        lifetime = self.db.get_lifetime_stats()

        # Update totals
        lifetime['total_words'] += self.current_session.words_dictated
        lifetime['total_characters'] += self.current_session.characters_typed
        lifetime['total_sessions'] += 1
        lifetime['total_time_minutes'] += self.current_session.active_dictation_time_s / 60.0
        lifetime['total_segments'] += self.current_session.segments_processed

        # Recalculate averages
        if lifetime['total_sessions'] > 0:
            # Get all sessions for accurate average
            all_sessions = self.db.get_recent_sessions(limit=1000000)  # Get all
            if all_sessions:
                wpms = [s['wpm'] for s in all_sessions if s['wpm'] and s['wpm'] > 0]
                latencies = [s['avg_latency_ms'] for s in all_sessions if s['avg_latency_ms']]

                if wpms:
                    lifetime['avg_wpm'] = np.mean(wpms)
                if latencies:
                    lifetime['avg_latency_ms'] = np.mean(latencies)

        # Calculate productivity metrics
        if lifetime['avg_wpm'] > 0:
            lifetime['speedup_factor'] = lifetime['avg_wpm'] / self.typing_baseline_wpm
            typing_time_minutes = lifetime['total_words'] / self.typing_baseline_wpm
            lifetime['time_saved_minutes'] = typing_time_minutes - lifetime['total_time_minutes']

        # Update personal bests
        if (not lifetime['best_wpm_value'] or
            self.current_session.words_per_minute > lifetime['best_wpm_value']):
            lifetime['best_wpm_session'] = self.current_session.session_id
            lifetime['best_wpm_value'] = self.current_session.words_per_minute

        if (not lifetime['longest_session_words'] or
            self.current_session.words_dictated > lifetime['longest_session_words']):
            lifetime['longest_session_id'] = self.current_session.session_id
            lifetime['longest_session_words'] = self.current_session.words_dictated

        if (not lifetime['lowest_latency_ms'] or
            self.current_session.average_latency_ms < lifetime['lowest_latency_ms']):
            lifetime['lowest_latency_session'] = self.current_session.session_id
            lifetime['lowest_latency_ms'] = self.current_session.average_latency_ms

        # Calculate 7-day trends
        recent_sessions = self.db.get_sessions_last_n_days(days=7)
        if len(recent_sessions) >= 2:
            timestamps = [s['start_time'] for s in recent_sessions]
            wpms = [s['wpm'] for s in recent_sessions if s['wpm']]
            latencies = [s['avg_latency_ms'] for s in recent_sessions if s['avg_latency_ms']]

            if len(wpms) >= 2:
                wpm_slope, _ = np.polyfit(timestamps, wpms, 1)
                lifetime['wpm_trend_7day'] = wpm_slope * (7 * 24 * 60 * 60)  # Convert to per-week

            if len(latencies) >= 2:
                latency_slope, _ = np.polyfit(timestamps, latencies, 1)
                lifetime['latency_trend_7day'] = latency_slope * (7 * 24 * 60 * 60)

        # Store updated stats
        self.db.update_lifetime_stats(lifetime)

    def get_realtime_metrics(self) -> RealtimeMetrics:
        """Get current real-time metrics."""
        with self._lock:
            return self.realtime

    def update_gpu_metrics(self, memory_mb: float, memory_total_mb: float):
        """Update real-time GPU metrics."""
        self.realtime.gpu_memory_current_mb = memory_mb
        self.realtime.gpu_memory_total_mb = memory_total_mb
        if memory_total_mb > 0:
            self.realtime.gpu_memory_percent = (memory_mb / memory_total_mb) * 100

    def update_cpu_metrics(self, cpu_percent: float):
        """Update real-time CPU metrics."""
        self.realtime.cpu_percent_current = cpu_percent

    def close(self):
        """Close metrics collector."""
        self.db.close()
