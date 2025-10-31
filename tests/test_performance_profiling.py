#!/usr/bin/env python3
"""
Performance profiling test for Swictation streaming.
Measures end-to-end latency and identifies bottlenecks.
"""

import sys
import time
import threading
from pathlib import Path
from typing import Dict, List

# Add src to path
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from performance_monitor import PerformanceMonitor, LatencyMeasurement
import numpy as np


class StreamingProfiler:
    """Profiler for streaming transcription pipeline"""

    def __init__(self):
        self.monitor = PerformanceMonitor(history_size=1000)
        self.test_results: List[Dict] = []

    def simulate_streaming_chunk_processing(self, chunk_size_ms: int = 400) -> Dict:
        """
        Simulate processing of a single streaming chunk.

        Args:
            chunk_size_ms: Chunk size in milliseconds

        Returns:
            Dict with timing measurements
        """
        # Start overall measurement
        measurement = self.monitor.start_latency_measurement('chunk_processing')

        # Phase 1: Audio capture → chunk ready
        time.sleep(chunk_size_ms / 1000.0)  # Simulate audio capture
        capture_latency = self.monitor.measure_phase(measurement, 'audio_capture')

        # Phase 2: VAD (Voice Activity Detection)
        time.sleep(0.002)  # 2ms for VAD
        vad_latency = self.monitor.measure_phase(measurement, 'vad')

        # Phase 3: Transcription (STT)
        time.sleep(0.100)  # 100ms for STT
        stt_latency = self.monitor.measure_phase(measurement, 'stt')

        # Phase 4: Text injection
        time.sleep(0.001)  # 1ms for injection
        injection_latency = self.monitor.measure_phase(measurement, 'injection')

        # Complete measurement
        self.monitor.end_latency_measurement('chunk_processing')

        return {
            'total_ms': measurement.total_ms,
            'phases': {
                'audio_capture': capture_latency,
                'vad': vad_latency,
                'stt': stt_latency,
                'injection': injection_latency,
            }
        }

    def run_profiling_test(self, num_chunks: int = 100, chunk_size_ms: int = 400):
        """
        Run profiling test with multiple chunks.

        Args:
            num_chunks: Number of chunks to process
            chunk_size_ms: Chunk size in milliseconds
        """
        print("=" * 80)
        print("⏱️  STREAMING PERFORMANCE PROFILING TEST")
        print("=" * 80)

        print(f"\nConfiguration:")
        print(f"  Number of chunks: {num_chunks}")
        print(f"  Chunk size: {chunk_size_ms}ms")
        print(f"  Expected duration: ~{num_chunks * chunk_size_ms / 1000:.1f}s")

        # Start background monitoring
        self.monitor.start_background_monitoring(interval=1.0)

        print(f"\n🚀 Processing {num_chunks} chunks...")

        start_time = time.time()

        for i in range(num_chunks):
            result = self.simulate_streaming_chunk_processing(chunk_size_ms)
            self.test_results.append(result)

            # Progress indicator
            if (i + 1) % 10 == 0:
                print(f"  Processed {i + 1}/{num_chunks} chunks...")

        total_duration = time.time() - start_time

        # Stop monitoring
        self.monitor.stop_background_monitoring()

        print(f"\n✓ Processing complete in {total_duration:.2f}s")

        # Print results
        self._print_results()

    def _print_results(self):
        """Print profiling results"""
        print("\n" + "=" * 80)
        print("📊 PROFILING RESULTS")
        print("=" * 80)

        # Overall latency stats
        stats = self.monitor.get_latency_stats('chunk_processing')

        if stats:
            print(f"\n⏱️  End-to-End Latency:")
            print(f"  Mean: {stats['mean']:.1f}ms")
            print(f"  Median (P50): {stats['median']:.1f}ms")
            print(f"  P95: {stats['p95']:.1f}ms")
            print(f"  P99: {stats['p99']:.1f}ms")
            print(f"  Max: {stats['max']:.1f}ms")
            print(f"  Min: {stats['min']:.1f}ms")

            # Check target
            target_latency = 2000  # 2 seconds
            if stats['p95'] < target_latency:
                print(f"  ✅ P95 latency within target (<{target_latency}ms)")
            else:
                print(f"  ⚠️  P95 latency exceeds target ({target_latency}ms)")

        # Phase breakdown
        if self.test_results:
            print(f"\n🔍 Phase Breakdown (mean latencies):")

            phase_names = ['audio_capture', 'vad', 'stt', 'injection']
            phase_totals = {name: [] for name in phase_names}

            for result in self.test_results:
                for name, latency in result['phases'].items():
                    phase_totals[name].append(latency)

            total_mean = sum(np.mean(latencies) for latencies in phase_totals.values())

            for name, latencies in phase_totals.items():
                mean_latency = np.mean(latencies)
                percentage = (mean_latency / total_mean * 100) if total_mean > 0 else 0
                print(f"  {name:20s}: {mean_latency:8.2f}ms ({percentage:5.1f}%)")

            # Identify bottleneck
            bottleneck = max(phase_totals.items(), key=lambda x: np.mean(x[1]))
            print(f"\n🎯 Bottleneck: {bottleneck[0]} ({np.mean(bottleneck[1]):.2f}ms)")

        # Resource usage
        print(f"\n💻 Resource Usage:")

        cpu_stats = self.monitor.get_cpu_stats(window_seconds=300)
        print(f"  CPU (mean): {cpu_stats['mean']:.1f}%")
        print(f"  CPU (max): {cpu_stats['max']:.1f}%")

        # Check CPU target
        cpu_target = 50.0
        if cpu_stats['mean'] < cpu_target:
            print(f"  ✅ CPU usage within target (<{cpu_target}%)")
        else:
            print(f"  ⚠️  CPU usage exceeds target ({cpu_target}%)")

        gpu_stats = self.monitor.get_gpu_memory_stats()
        if gpu_stats['available']:
            print(f"  GPU Memory (current): {gpu_stats['current_mb']:.1f} MB")
            print(f"  GPU Memory (peak): {gpu_stats['peak_mb']:.1f} MB")

            # Check GPU target
            gpu_target = 4000  # 4GB
            if gpu_stats['peak_mb'] < gpu_target:
                print(f"  ✅ GPU memory within target (<{gpu_target} MB)")
            else:
                print(f"  ⚠️  GPU memory exceeds target ({gpu_target} MB)")

        # Memory leak check
        leak_result = self.monitor.detect_memory_leak(window_seconds=60)
        print(f"\n💾 Memory Leak Check:")
        print(f"  Leak detected: {leak_result['leak_detected']}")
        if leak_result.get('growth_rate_mb_s') is not None:
            print(f"  Growth rate: {leak_result['growth_rate_mb_s']:.4f} MB/s")

        # Final verdict
        print(f"\n" + "=" * 80)

        all_passed = True
        failures = []

        if stats and stats['p95'] >= 2000:
            all_passed = False
            failures.append(f"Latency P95 too high: {stats['p95']:.1f}ms")

        if cpu_stats['mean'] >= 50.0:
            all_passed = False
            failures.append(f"CPU usage too high: {cpu_stats['mean']:.1f}%")

        if gpu_stats['available'] and gpu_stats['peak_mb'] >= 4000:
            all_passed = False
            failures.append(f"GPU memory too high: {gpu_stats['peak_mb']:.1f} MB")

        if leak_result['leak_detected']:
            all_passed = False
            failures.append("Memory leak detected")

        if all_passed:
            print("✅ ALL PERFORMANCE TARGETS MET!")
        else:
            print("❌ PERFORMANCE ISSUES DETECTED:")
            for failure in failures:
                print(f"  - {failure}")

        print("=" * 80)

        return all_passed


def test_latency_measurement():
    """Test latency measurement functionality"""
    print("=" * 80)
    print("🧪 LATENCY MEASUREMENT TEST")
    print("=" * 80)

    monitor = PerformanceMonitor()

    # Test basic measurement
    print("\n📏 Testing basic latency measurement...")

    measurement = monitor.start_latency_measurement('test_operation')
    time.sleep(0.1)  # 100ms operation
    monitor.measure_phase(measurement, 'phase1')
    time.sleep(0.05)  # 50ms operation
    monitor.measure_phase(measurement, 'phase2')
    monitor.end_latency_measurement('test_operation')

    stats = monitor.get_latency_stats('test_operation')

    print(f"\nResults:")
    print(f"  Total latency: ~{stats['mean']:.1f}ms (expected ~150ms)")
    print(f"  Phase 1: {measurement.phase_times.get('phase1', 0):.1f}ms (expected ~100ms)")
    print(f"  Phase 2: {measurement.phase_times.get('phase2', 0):.1f}ms (expected ~50ms)")

    # Check accuracy (within 10ms tolerance)
    tolerance = 10
    total_expected = 150
    phase1_expected = 100
    phase2_expected = 50

    checks = [
        (abs(stats['mean'] - total_expected) < tolerance, "Total latency"),
        (abs(measurement.phase_times.get('phase1', 0) - phase1_expected) < tolerance, "Phase 1"),
        (abs(measurement.phase_times.get('phase2', 0) - phase2_expected) < tolerance, "Phase 2"),
    ]

    all_passed = all(check[0] for check in checks)

    print(f"\n{'✅' if all_passed else '❌'} Latency measurement test {'PASSED' if all_passed else 'FAILED'}")

    return all_passed


if __name__ == '__main__':
    import argparse

    parser = argparse.ArgumentParser(description='Performance profiling test')
    parser.add_argument(
        '--chunks',
        type=int,
        default=100,
        help='Number of chunks to process (default: 100)'
    )
    parser.add_argument(
        '--chunk-size',
        type=int,
        default=400,
        help='Chunk size in milliseconds (default: 400)'
    )
    parser.add_argument(
        '--test-measurement',
        action='store_true',
        help='Test latency measurement functionality'
    )

    args = parser.parse_args()

    if args.test_measurement:
        # Test latency measurement
        success = test_latency_measurement()
        sys.exit(0 if success else 1)
    else:
        # Run profiling test
        profiler = StreamingProfiler()
        profiler.run_profiling_test(
            num_chunks=args.chunks,
            chunk_size_ms=args.chunk_size
        )
