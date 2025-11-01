"""
Data models for metrics tracking.
"""

from dataclasses import dataclass, field, asdict
from typing import Dict, Optional
from datetime import datetime
from enum import Enum


class DaemonState(Enum):
    """Daemon state enum (mirrored from swictationd.py)"""
    IDLE = "idle"
    RECORDING = "recording"
    PROCESSING = "processing"
    ERROR = "error"


@dataclass
class SessionMetrics:
    """Metrics for a single recording session."""

    # Identity
    session_id: Optional[int] = None
    session_start: Optional[datetime] = None
    session_end: Optional[datetime] = None

    # Timing
    total_duration_s: float = 0.0  # Wall clock time
    active_dictation_time_s: float = 0.0  # Excludes silence
    pause_time_s: float = 0.0  # Time in silence

    # Content
    words_dictated: int = 0
    characters_typed: int = 0
    segments_processed: int = 0

    # Performance
    words_per_minute: float = 0.0
    typing_speed_equivalent: float = 40.0  # Baseline comparison
    average_latency_ms: float = 0.0
    median_latency_ms: float = 0.0
    p95_latency_ms: float = 0.0

    # Quality indicators
    transformations_count: int = 0
    keyboard_actions_count: int = 0
    average_segment_words: float = 0.0

    # Technical
    average_segment_duration_s: float = 0.0
    gpu_memory_peak_mb: float = 0.0
    gpu_memory_mean_mb: float = 0.0
    cpu_usage_mean_percent: float = 0.0
    cpu_usage_peak_percent: float = 0.0

    def to_dict(self) -> Dict:
        """Convert to dictionary for storage."""
        data = asdict(self)
        # Convert datetime to timestamps
        if self.session_start:
            data['session_start'] = self.session_start.timestamp()
        if self.session_end:
            data['session_end'] = self.session_end.timestamp()
        return data

    def calculate_wpm(self):
        """Calculate words per minute from active time."""
        if self.active_dictation_time_s > 0:
            self.words_per_minute = (self.words_dictated / self.active_dictation_time_s) * 60
        else:
            self.words_per_minute = 0.0


@dataclass
class SegmentMetrics:
    """Metrics for a single VAD-triggered segment."""

    # Identity
    segment_id: Optional[int] = None
    session_id: Optional[int] = None
    timestamp: Optional[datetime] = None

    # Content
    duration_s: float = 0.0
    words: int = 0
    characters: int = 0
    text: str = ""

    # Latency breakdown (match database column names)
    vad_latency_ms: float = 0.0
    audio_save_latency_ms: float = 0.0
    stt_latency_ms: float = 0.0
    transform_latency_us: float = 0.0
    injection_latency_ms: float = 0.0
    total_latency_ms: float = 0.0

    # Quality indicators
    transformations_count: int = 0
    keyboard_actions_count: int = 0

    def to_dict(self) -> Dict:
        """Convert to dictionary for storage."""
        data = asdict(self)
        if self.timestamp:
            data['timestamp'] = self.timestamp.timestamp()
        return data

    def calculate_wpm(self) -> float:
        """Calculate WPM for this segment."""
        if self.duration_s > 0:
            return (self.words / self.duration_s) * 60
        return 0.0


@dataclass
class LifetimeMetrics:
    """Aggregate metrics across all sessions."""

    # Totals
    total_words: int = 0
    total_characters: int = 0
    total_sessions: int = 0
    total_dictation_time_minutes: float = 0.0
    total_segments: int = 0

    # Performance averages
    average_wpm: float = 0.0
    average_latency_ms: float = 0.0

    # Productivity
    typing_speed_equivalent: float = 40.0
    speedup_factor: float = 1.0
    estimated_time_saved_minutes: float = 0.0

    # Trends (7-day rolling)
    wpm_trend_7day: float = 0.0
    latency_trend_7day: float = 0.0

    # System health
    cuda_errors_total: int = 0
    cuda_errors_recovered: int = 0
    memory_pressure_events: int = 0
    high_latency_warnings: int = 0

    # Personal bests
    best_wpm_session: Optional[int] = None
    best_wpm_value: float = 0.0
    longest_session_words: int = 0
    longest_session_id: Optional[int] = None
    lowest_latency_session: Optional[int] = None
    lowest_latency_ms: float = 0.0

    # Metadata
    last_updated: Optional[datetime] = None

    def to_dict(self) -> Dict:
        """Convert to dictionary for storage."""
        data = asdict(self)
        if self.last_updated:
            data['last_updated'] = self.last_updated.timestamp()
        return data


@dataclass
class RealtimeMetrics:
    """Real-time metrics during active recording."""

    # Current state
    current_state: DaemonState = DaemonState.IDLE
    recording_duration_s: float = 0.0
    silence_duration_s: float = 0.0
    speech_detected: bool = False

    # Session progress
    current_session_id: Optional[int] = None
    segments_this_session: int = 0
    words_this_session: int = 0
    wpm_this_session: float = 0.0

    # Resource usage
    gpu_memory_current_mb: float = 0.0
    gpu_memory_total_mb: float = 0.0
    gpu_memory_percent: float = 0.0
    cpu_percent_current: float = 0.0

    # Last segment
    last_segment_words: int = 0
    last_segment_latency_ms: float = 0.0
    last_segment_wpm: float = 0.0
    last_transcription: str = ""

    def to_dict(self) -> Dict:
        """Convert to dictionary."""
        data = asdict(self)
        data['current_state'] = self.current_state.value
        return data
