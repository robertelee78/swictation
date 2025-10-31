#!/usr/bin/env python3
"""
Performance monitoring utilities for Swictation.
Tracks GPU memory, CPU usage, latency, and detects memory leaks.
"""

import time
import psutil
import threading
from typing import Optional, Dict, List, Callable
from dataclasses import dataclass, field
from collections import deque
import numpy as np

try:
    import torch
    HAS_TORCH = True
except ImportError:
    HAS_TORCH = False


@dataclass
class PerformanceMetrics:
    """Container for performance metrics snapshot"""
    timestamp: float

    # GPU metrics (if available)
    gpu_memory_allocated: float = 0.0  # MB
    gpu_memory_reserved: float = 0.0   # MB
    gpu_memory_peak: float = 0.0       # MB

    # CPU metrics
    cpu_percent: float = 0.0
    cpu_percent_per_core: List[float] = field(default_factory=list)

    # Memory metrics
    ram_used: float = 0.0         # MB
    ram_percent: float = 0.0
    ram_available: float = 0.0    # MB

    # Process metrics
    process_memory: float = 0.0   # MB
    process_cpu: float = 0.0
    num_threads: int = 0

    # Custom metrics
    custom: Dict[str, float] = field(default_factory=dict)


@dataclass
class LatencyMeasurement:
    """Container for latency measurements"""
    name: str
    start_time: float
    end_time: Optional[float] = None
    phase_times: Dict[str, float] = field(default_factory=dict)

    @property
    def total_ms(self) -> float:
        """Total latency in milliseconds"""
        if self.end_time is None:
            return 0.0
        return (self.end_time - self.start_time) * 1000

    def add_phase(self, phase_name: str, duration_ms: float):
        """Add a phase measurement"""
        self.phase_times[phase_name] = duration_ms

    def complete(self):
        """Mark measurement as complete"""
        if self.end_time is None:
            self.end_time = time.time()


class PerformanceMonitor:
    """
    Real-time performance monitoring for Swictation daemon.

    Features:
    - GPU memory tracking
    - CPU usage monitoring
    - Latency measurement
    - Memory leak detection
    - Performance warnings
    """

    def __init__(
        self,
        history_size: int = 1000,
        warning_callbacks: Optional[Dict[str, Callable]] = None
    ):
        """
        Initialize performance monitor.

        Args:
            history_size: Number of metrics snapshots to keep
            warning_callbacks: Dict of warning type -> callback function
        """
        self.history_size = history_size
        self.warning_callbacks = warning_callbacks or {}

        # Metrics history (using deque for efficient rolling window)
        self.metrics_history = deque(maxlen=history_size)

        # Latency measurements
        self.latency_measurements: Dict[str, List[LatencyMeasurement]] = {}
        self.active_measurements: Dict[str, LatencyMeasurement] = {}

        # Process monitor
        self.process = psutil.Process()

        # GPU availability
        self.has_gpu = HAS_TORCH and torch.cuda.is_available()

        # Warning thresholds
        self.thresholds = {
            'gpu_memory_mb': 4000,      # 4GB
            'gpu_memory_percent': 80.0, # 80% of total GPU memory
            'cpu_percent': 50.0,        # 50%
            'latency_ms': 2000,         # 2 seconds
            'memory_growth_mb_s': 1.0,  # 1 MB/s sustained growth
        }

        # Track GPU total memory for percentage calculations
        # Defer CUDA initialization until first use to avoid context issues
        self.gpu_total_memory = None

        # Background monitoring
        self.monitoring_active = False
        self.monitoring_thread: Optional[threading.Thread] = None
        self.monitoring_interval = 1.0  # seconds

    def capture_metrics(self, custom_metrics: Optional[Dict[str, float]] = None) -> PerformanceMetrics:
        """
        Capture current performance metrics snapshot.

        Args:
            custom_metrics: Optional custom metrics to include

        Returns:
            PerformanceMetrics snapshot
        """
        metrics = PerformanceMetrics(timestamp=time.time())

        # GPU metrics
        if self.has_gpu:
            metrics.gpu_memory_allocated = torch.cuda.memory_allocated() / 1e6  # MB
            metrics.gpu_memory_reserved = torch.cuda.memory_reserved() / 1e6    # MB
            metrics.gpu_memory_peak = torch.cuda.max_memory_allocated() / 1e6  # MB

        # CPU metrics
        metrics.cpu_percent = psutil.cpu_percent(interval=0.1)
        metrics.cpu_percent_per_core = psutil.cpu_percent(interval=0.1, percpu=True)

        # System memory
        mem = psutil.virtual_memory()
        metrics.ram_used = mem.used / 1e6  # MB
        metrics.ram_percent = mem.percent
        metrics.ram_available = mem.available / 1e6  # MB

        # Process metrics
        metrics.process_memory = self.process.memory_info().rss / 1e6  # MB
        metrics.process_cpu = self.process.cpu_percent(interval=0.1)
        metrics.num_threads = self.process.num_threads()

        # Custom metrics
        if custom_metrics:
            metrics.custom = custom_metrics

        # Add to history
        self.metrics_history.append(metrics)

        # Check for warnings
        self._check_thresholds(metrics)

        return metrics

    def start_latency_measurement(self, name: str) -> LatencyMeasurement:
        """
        Start a latency measurement.

        Args:
            name: Name of the operation being measured

        Returns:
            LatencyMeasurement object
        """
        measurement = LatencyMeasurement(name=name, start_time=time.time())
        self.active_measurements[name] = measurement
        return measurement

    def end_latency_measurement(self, name: str) -> Optional[LatencyMeasurement]:
        """
        End a latency measurement.

        Args:
            name: Name of the operation

        Returns:
            Completed LatencyMeasurement or None if not found
        """
        measurement = self.active_measurements.pop(name, None)
        if measurement:
            measurement.complete()

            # Store in history
            if name not in self.latency_measurements:
                self.latency_measurements[name] = []
            self.latency_measurements[name].append(measurement)

            # Keep only recent measurements
            if len(self.latency_measurements[name]) > self.history_size:
                self.latency_measurements[name] = self.latency_measurements[name][-self.history_size:]

            # Check latency threshold
            if measurement.total_ms > self.thresholds['latency_ms']:
                self._trigger_warning(
                    'high_latency',
                    f"{name} latency: {measurement.total_ms:.0f}ms (threshold: {self.thresholds['latency_ms']:.0f}ms)"
                )

            return measurement
        return None

    def measure_phase(self, measurement: LatencyMeasurement, phase_name: str) -> float:
        """
        Measure a phase within an active measurement.

        Args:
            measurement: Active measurement
            phase_name: Name of the phase

        Returns:
            Phase duration in milliseconds
        """
        current_time = time.time()

        # Calculate phase duration
        if measurement.phase_times:
            # Duration since last phase
            last_phase_time = sum(measurement.phase_times.values()) / 1000  # Convert to seconds
            phase_duration = (current_time - measurement.start_time - last_phase_time) * 1000
        else:
            # First phase - duration from start
            phase_duration = (current_time - measurement.start_time) * 1000

        measurement.add_phase(phase_name, phase_duration)
        return phase_duration

    def get_latency_stats(self, name: str) -> Optional[Dict[str, float]]:
        """
        Get latency statistics for a named operation.

        Args:
            name: Operation name

        Returns:
            Dict with mean, p50, p95, p99, max latencies in ms
        """
        if name not in self.latency_measurements:
            return None

        latencies = [m.total_ms for m in self.latency_measurements[name]]
        if not latencies:
            return None

        return {
            'count': len(latencies),
            'mean': np.mean(latencies),
            'median': np.percentile(latencies, 50),
            'p95': np.percentile(latencies, 95),
            'p99': np.percentile(latencies, 99),
            'max': np.max(latencies),
            'min': np.min(latencies),
        }

    def detect_memory_leak(self, window_seconds: float = 60.0) -> Dict[str, any]:
        """
        Detect memory leaks by analyzing memory growth over time.

        Args:
            window_seconds: Time window to analyze

        Returns:
            Dict with leak detection results
        """
        if len(self.metrics_history) < 2:
            return {
                'leak_detected': False,
                'reason': 'Insufficient data'
            }

        # Filter metrics within window
        now = time.time()
        window_metrics = [
            m for m in self.metrics_history
            if (now - m.timestamp) <= window_seconds
        ]

        if len(window_metrics) < 2:
            return {
                'leak_detected': False,
                'reason': 'Insufficient data in window'
            }

        # Analyze process memory growth
        timestamps = [m.timestamp for m in window_metrics]
        memory_values = [m.process_memory for m in window_metrics]

        # Linear regression to detect trend
        slope, intercept = np.polyfit(timestamps, memory_values, 1)

        # Slope is MB/second
        growth_rate_mb_s = slope

        # Detect leak if sustained growth exceeds threshold
        leak_detected = growth_rate_mb_s > self.thresholds['memory_growth_mb_s']

        result = {
            'leak_detected': leak_detected,
            'growth_rate_mb_s': growth_rate_mb_s,
            'window_seconds': window_seconds,
            'samples': len(window_metrics),
            'initial_memory_mb': memory_values[0],
            'final_memory_mb': memory_values[-1],
            'total_growth_mb': memory_values[-1] - memory_values[0],
        }

        if leak_detected:
            self._trigger_warning(
                'memory_leak',
                f"Memory leak detected: {growth_rate_mb_s:.2f} MB/s growth"
            )

        return result

    def get_gpu_memory_stats(self) -> Dict[str, float]:
        """
        Get GPU memory statistics.

        Returns:
            Dict with current, peak, reserved GPU memory in MB, plus percentages
        """
        if not self.has_gpu:
            return {
                'current_mb': 0.0,
                'peak_mb': 0.0,
                'reserved_mb': 0.0,
                'total_mb': 0.0,
                'free_mb': 0.0,
                'current_percent': 0.0,
                'peak_percent': 0.0,
                'available': False
            }

        # Lazy initialization of GPU total memory on first use
        if self.gpu_total_memory is None and self.has_gpu:
            try:
                self.gpu_total_memory = torch.cuda.get_device_properties(0).total_memory / 1e6
            except RuntimeError:
                # CUDA not initialized yet, return zeros
                return {
                    'current_mb': 0, 'peak_mb': 0, 'reserved_mb': 0,
                    'total_mb': 0, 'free_mb': 0, 'current_percent': 0,
                    'peak_percent': 0, 'reserved_percent': 0, 'free_percent': 0
                }

        current = torch.cuda.memory_allocated() / 1e6
        peak = torch.cuda.max_memory_allocated() / 1e6
        reserved = torch.cuda.memory_reserved() / 1e6
        total = self.gpu_total_memory or 0
        free = total - current if total > 0 else 0

        return {
            'current_mb': current,
            'peak_mb': peak,
            'reserved_mb': reserved,
            'total_mb': total,
            'free_mb': free,
            'current_percent': (current / total * 100) if total > 0 else 0,
            'peak_percent': (peak / total * 100) if total > 0 else 0,
            'available': True,
            'device_name': torch.cuda.get_device_name(0) if torch.cuda.is_available() else 'N/A'
        }

    def get_cpu_stats(self, window_seconds: float = 10.0) -> Dict[str, float]:
        """
        Get CPU usage statistics over a time window.

        Args:
            window_seconds: Time window to analyze

        Returns:
            Dict with mean, max CPU usage
        """
        if len(self.metrics_history) < 2:
            return {'mean': 0.0, 'max': 0.0, 'samples': 0}

        # Filter metrics within window
        now = time.time()
        window_metrics = [
            m for m in self.metrics_history
            if (now - m.timestamp) <= window_seconds
        ]

        if not window_metrics:
            return {'mean': 0.0, 'max': 0.0, 'samples': 0}

        cpu_values = [m.cpu_percent for m in window_metrics]

        return {
            'mean': np.mean(cpu_values),
            'max': np.max(cpu_values),
            'min': np.min(cpu_values),
            'samples': len(cpu_values)
        }

    def start_background_monitoring(self, interval: float = 1.0):
        """
        Start background monitoring thread.

        Args:
            interval: Monitoring interval in seconds
        """
        if self.monitoring_active:
            return

        self.monitoring_interval = interval
        self.monitoring_active = True

        self.monitoring_thread = threading.Thread(
            target=self._monitoring_loop,
            daemon=True
        )
        self.monitoring_thread.start()

    def stop_background_monitoring(self):
        """Stop background monitoring thread."""
        self.monitoring_active = False
        if self.monitoring_thread:
            self.monitoring_thread.join(timeout=5.0)

    def _monitoring_loop(self):
        """Background monitoring loop."""
        while self.monitoring_active:
            self.capture_metrics()
            time.sleep(self.monitoring_interval)

    def _check_thresholds(self, metrics: PerformanceMetrics):
        """Check metrics against thresholds and trigger warnings."""

        # GPU memory check (absolute and percentage)
        if self.has_gpu:
            # Absolute threshold
            if metrics.gpu_memory_allocated > self.thresholds['gpu_memory_mb']:
                self._trigger_warning(
                    'high_gpu_memory',
                    f"GPU memory: {metrics.gpu_memory_allocated:.0f}MB (threshold: {self.thresholds['gpu_memory_mb']:.0f}MB)"
                )

            # Percentage threshold (only if GPU total memory is known)
            if self.gpu_total_memory and self.gpu_total_memory > 0:
                gpu_percent = (metrics.gpu_memory_allocated / self.gpu_total_memory) * 100
                if gpu_percent > self.thresholds['gpu_memory_percent']:
                    self._trigger_warning(
                        'high_gpu_memory',
                        f"GPU memory: {gpu_percent:.1f}% (threshold: {self.thresholds['gpu_memory_percent']:.1f}%)"
                    )

        # CPU check
        if metrics.cpu_percent > self.thresholds['cpu_percent']:
            self._trigger_warning(
                'high_cpu',
                f"CPU usage: {metrics.cpu_percent:.1f}% (threshold: {self.thresholds['cpu_percent']:.1f}%)"
            )

    def _trigger_warning(self, warning_type: str, message: str):
        """Trigger a performance warning."""
        if warning_type in self.warning_callbacks:
            self.warning_callbacks[warning_type](message)
        else:
            print(f"⚠️  Performance Warning [{warning_type}]: {message}")

    def print_summary(self):
        """Print performance summary."""
        print("\n" + "=" * 80)
        print("📊 PERFORMANCE SUMMARY")
        print("=" * 80)

        # GPU stats
        gpu_stats = self.get_gpu_memory_stats()
        print(f"\n🎮 GPU Memory:")
        if gpu_stats['available']:
            print(f"  Current: {gpu_stats['current_mb']:.1f} MB ({gpu_stats['current_percent']:.1f}%)")
            print(f"  Peak: {gpu_stats['peak_mb']:.1f} MB ({gpu_stats['peak_percent']:.1f}%)")
            print(f"  Reserved: {gpu_stats['reserved_mb']:.1f} MB")
            print(f"  Free: {gpu_stats['free_mb']:.1f} MB")
            print(f"  Total: {gpu_stats['total_mb']:.1f} MB")
            print(f"  Device: {gpu_stats['device_name']}")
        else:
            print(f"  No GPU available")

        # CPU stats
        cpu_stats = self.get_cpu_stats()
        print(f"\n🖥️  CPU Usage (last 10s):")
        print(f"  Mean: {cpu_stats['mean']:.1f}%")
        print(f"  Max: {cpu_stats['max']:.1f}%")
        print(f"  Samples: {cpu_stats['samples']}")

        # Latency stats
        print(f"\n⏱️  Latency Measurements:")
        for name, measurements in self.latency_measurements.items():
            stats = self.get_latency_stats(name)
            if stats:
                print(f"  {name}:")
                print(f"    Mean: {stats['mean']:.1f}ms")
                print(f"    P95: {stats['p95']:.1f}ms")
                print(f"    P99: {stats['p99']:.1f}ms")
                print(f"    Count: {stats['count']}")

        # Memory leak check
        leak_result = self.detect_memory_leak(window_seconds=60.0)
        print(f"\n💾 Memory Leak Detection (60s window):")
        print(f"  Leak detected: {leak_result['leak_detected']}")
        if leak_result.get('growth_rate_mb_s') is not None:
            print(f"  Growth rate: {leak_result['growth_rate_mb_s']:.3f} MB/s")
            print(f"  Total growth: {leak_result.get('total_growth_mb', 0):.1f} MB")

        print("=" * 80)
