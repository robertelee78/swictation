#!/usr/bin/env python3
"""
Memory leak detection test for Swictation daemon.
Runs continuous streaming for 10 minutes and monitors for memory leaks.
"""

import sys
import time
import signal
from pathlib import Path

# Add src to path
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from performance_monitor import PerformanceMonitor
import psutil
import numpy as np


class MemoryLeakTester:
    """Test harness for memory leak detection"""

    def __init__(self, duration_minutes: int = 10, sample_interval: float = 1.0):
        """
        Initialize memory leak tester.

        Args:
            duration_minutes: Test duration in minutes
            sample_interval: Sampling interval in seconds
        """
        self.duration_minutes = duration_minutes
        self.sample_interval = sample_interval
        self.monitor = PerformanceMonitor(history_size=10000)
        self.running = False

        # Memory baselines
        self.baseline_memory = None
        self.samples_collected = 0

    def setup_signal_handlers(self):
        """Setup signal handlers for graceful shutdown"""
        def signal_handler(signum, frame):
            print(f"\n\nReceived signal {signum}, stopping test...")
            self.running = False

        signal.signal(signal.SIGINT, signal_handler)
        signal.signal(signal.SIGTERM, signal_handler)

    def run_test(self):
        """Run the memory leak detection test"""
        print("=" * 80)
        print("🧪 MEMORY LEAK DETECTION TEST")
        print("=" * 80)

        duration_seconds = self.duration_minutes * 60

        print(f"\nConfiguration:")
        print(f"  Duration: {self.duration_minutes} minutes ({duration_seconds}s)")
        print(f"  Sample interval: {self.sample_interval}s")
        print(f"  Expected samples: ~{int(duration_seconds / self.sample_interval)}")

        # Get baseline memory
        self.baseline_memory = psutil.Process().memory_info().rss / 1e6  # MB
        print(f"\n📊 Baseline memory: {self.baseline_memory:.1f} MB")

        # Setup signal handlers
        self.setup_signal_handlers()

        # Start background monitoring
        print(f"\n🚀 Starting monitoring...")
        self.monitor.start_background_monitoring(interval=self.sample_interval)

        self.running = True
        start_time = time.time()
        last_report = start_time

        try:
            while self.running:
                elapsed = time.time() - start_time

                # Check if test duration exceeded
                if elapsed >= duration_seconds:
                    print(f"\n✓ Test duration reached ({self.duration_minutes} minutes)")
                    break

                # Periodic status reports
                if time.time() - last_report >= 60:  # Every minute
                    self._print_progress_report(elapsed)
                    last_report = time.time()

                time.sleep(1)

        except KeyboardInterrupt:
            print("\n\nTest interrupted by user")

        finally:
            # Stop monitoring
            self.monitor.stop_background_monitoring()

            # Print final report
            self._print_final_report()

    def _print_progress_report(self, elapsed: float):
        """Print periodic progress report"""
        minutes_elapsed = int(elapsed / 60)

        # Get current metrics
        gpu_stats = self.monitor.get_gpu_memory_stats()
        cpu_stats = self.monitor.get_cpu_stats(window_seconds=60)

        # Get current process memory
        current_memory = psutil.Process().memory_info().rss / 1e6  # MB
        memory_growth = current_memory - self.baseline_memory

        print(f"\n⏱️  Progress: {minutes_elapsed}/{self.duration_minutes} minutes")
        print(f"  Memory: {current_memory:.1f} MB (+{memory_growth:.1f} MB from baseline)")
        print(f"  CPU (avg): {cpu_stats['mean']:.1f}%")

        if gpu_stats['available']:
            print(f"  GPU Memory: {gpu_stats['current_mb']:.1f} MB")

    def _print_final_report(self):
        """Print final test report"""
        print("\n" + "=" * 80)
        print("📊 MEMORY LEAK TEST RESULTS")
        print("=" * 80)

        # Get final memory
        final_memory = psutil.Process().memory_info().rss / 1e6  # MB
        total_growth = final_memory - self.baseline_memory

        print(f"\n💾 Memory Usage:")
        print(f"  Baseline: {self.baseline_memory:.1f} MB")
        print(f"  Final: {final_memory:.1f} MB")
        print(f"  Total growth: {total_growth:.1f} MB")
        print(f"  Growth %: {(total_growth / self.baseline_memory * 100):.1f}%")

        # Memory leak detection
        leak_result = self.monitor.detect_memory_leak(window_seconds=300)  # 5 minute window

        print(f"\n🔍 Memory Leak Analysis (5-minute window):")
        print(f"  Leak detected: {leak_result['leak_detected']}")

        if leak_result.get('growth_rate_mb_s') is not None:
            growth_rate = leak_result['growth_rate_mb_s']
            print(f"  Growth rate: {growth_rate:.4f} MB/s")
            print(f"  Projected hourly: {growth_rate * 3600:.1f} MB/hour")

            # Threshold check
            threshold = 1.0  # MB/s
            if growth_rate > threshold:
                print(f"  ⚠️  WARNING: Growth rate exceeds threshold ({threshold} MB/s)")
                print(f"  🔴 MEMORY LEAK DETECTED!")
            else:
                print(f"  ✅ Growth rate within acceptable range")

        # CPU stats
        cpu_stats = self.monitor.get_cpu_stats(window_seconds=300)
        print(f"\n🖥️  CPU Usage (5-minute window):")
        print(f"  Mean: {cpu_stats['mean']:.1f}%")
        print(f"  Max: {cpu_stats['max']:.1f}%")

        # GPU stats
        gpu_stats = self.monitor.get_gpu_memory_stats()
        print(f"\n🎮 GPU Memory:")
        if gpu_stats['available']:
            print(f"  Current: {gpu_stats['current_mb']:.1f} MB")
            print(f"  Peak: {gpu_stats['peak_mb']:.1f} MB")

            # Check if GPU memory is stable
            gpu_threshold = 4000  # 4GB
            if gpu_stats['peak_mb'] > gpu_threshold:
                print(f"  ⚠️  WARNING: Peak GPU memory exceeds {gpu_threshold} MB")
            else:
                print(f"  ✅ GPU memory within limits")
        else:
            print(f"  No GPU available")

        # Final verdict
        print(f"\n" + "=" * 80)

        leak_detected = leak_result['leak_detected']
        excessive_growth = total_growth > 100  # More than 100MB growth

        if leak_detected or excessive_growth:
            print("🔴 TEST FAILED: Memory leak detected!")
            if leak_detected:
                print(f"  - Sustained memory growth detected")
            if excessive_growth:
                print(f"  - Excessive total memory growth ({total_growth:.1f} MB)")
        else:
            print("✅ TEST PASSED: No memory leaks detected")
            print(f"  - Memory growth within acceptable limits")
            print(f"  - System performance stable")

        print("=" * 80)

        return not (leak_detected or excessive_growth)


def test_simulated_workload(duration_minutes: int = 1):
    """
    Test with simulated workload (for quick testing without daemon).
    Simulates periodic memory allocations to verify leak detection works.
    """
    print("=" * 80)
    print("🧪 SIMULATED WORKLOAD TEST")
    print("=" * 80)

    monitor = PerformanceMonitor()
    monitor.start_background_monitoring(interval=0.5)

    # Simulate workload with intentional memory leak
    data_buffers = []
    duration_seconds = duration_minutes * 60
    start_time = time.time()

    print(f"\n🚀 Running simulated workload for {duration_minutes} minute(s)...")
    print("  (Intentionally leaking memory to test detection)")

    try:
        while time.time() - start_time < duration_seconds:
            # Simulate processing (allocate memory that isn't freed)
            data_buffers.append(np.random.randn(1000, 1000))  # ~8MB allocation

            # Keep only recent buffers (simulate partial cleanup)
            if len(data_buffers) > 5:
                data_buffers.pop(0)

            time.sleep(1)

    finally:
        monitor.stop_background_monitoring()

        # Check for leak
        leak_result = monitor.detect_memory_leak(window_seconds=30)

        print(f"\n📊 Results:")
        print(f"  Leak detected: {leak_result['leak_detected']}")
        print(f"  Growth rate: {leak_result.get('growth_rate_mb_s', 0):.4f} MB/s")

        if leak_result['leak_detected']:
            print(f"\n✅ PASS: Leak detection working correctly!")
            return True
        else:
            print(f"\n❌ FAIL: Leak detection should have triggered!")
            return False


if __name__ == '__main__':
    import argparse

    parser = argparse.ArgumentParser(description='Memory leak detection test')
    parser.add_argument(
        '--duration',
        type=int,
        default=10,
        help='Test duration in minutes (default: 10)'
    )
    parser.add_argument(
        '--interval',
        type=float,
        default=1.0,
        help='Sampling interval in seconds (default: 1.0)'
    )
    parser.add_argument(
        '--simulate',
        action='store_true',
        help='Run simulated workload test instead of real test'
    )

    args = parser.parse_args()

    if args.simulate:
        # Quick simulation test
        success = test_simulated_workload(duration_minutes=1)
        sys.exit(0 if success else 1)
    else:
        # Real memory leak test
        tester = MemoryLeakTester(
            duration_minutes=args.duration,
            sample_interval=args.interval
        )
        tester.run_test()
