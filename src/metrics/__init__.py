"""
Metrics collection and analysis module for Swictation.

Provides comprehensive performance tracking including:
- Session metrics (per-recording statistics)
- Segment metrics (granular per-VAD-segment data)
- Lifetime statistics (historical trends)
- Real-time monitoring during dictation
"""

from .models import SessionMetrics, SegmentMetrics, LifetimeMetrics, RealtimeMetrics
from .database import MetricsDatabase
from .collector import MetricsCollector

__all__ = [
    'SessionMetrics',
    'SegmentMetrics',
    'LifetimeMetrics',
    'RealtimeMetrics',
    'MetricsDatabase',
    'MetricsCollector',
]
