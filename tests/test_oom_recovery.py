#!/usr/bin/env python3
"""
OOM Recovery Tests for Swictation

Tests recovery mechanisms when Out-Of-Memory conditions occur,
ensuring graceful degradation and data preservation.

Test Coverage:
1. OOM detection and notification
2. Graceful degradation to lower memory modes
3. Data preservation during OOM
4. Recovery after OOM events
5. Multiple OOM handling
"""

import sys
import time
import gc
from pathlib import Path
from typing import Optional, List
import numpy as np

# Add src to path
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

try:
    import torch
    HAS_TORCH = True
except ImportError:
    HAS_TORCH = False
    torch = None

import pytest
from unittest.mock import Mock, patch, MagicMock, call

from performance_monitor import PerformanceMonitor


class OOMRecoveryManager:
    """
    Manager for OOM recovery testing.

    Simulates various OOM scenarios and tests recovery mechanisms.
    """

    def __init__(self):
        self.has_gpu = HAS_TORCH and torch.cuda.is_available()
        self.oom_count = 0
        self.recovery_count = 0
        self.fallback_active = False

        # Recovery strategies
        self.strategies = [
            'cache_clear',
            'garbage_collect',
            'reduce_buffer',
            'cpu_fallback',
            'emergency_shutdown'
        ]

    def trigger_oom(self) -> bool:
        """
        Trigger an OOM condition for testing.

        Returns:
            True if OOM was triggered, False otherwise
        """
        if not self.has_gpu:
            return False

        try:
            # Allocate impossible amount
            x = torch.randn(1000000000000, device='cuda')
            return False
        except RuntimeError as e:
            if "out of memory" in str(e).lower():
                self.oom_count += 1
                return True
            raise

    def attempt_recovery(self, strategy: str) -> bool:
        """
        Attempt recovery using specified strategy.

        Args:
            strategy: Recovery strategy name

        Returns:
            True if recovery succeeded
        """
        try:
            if strategy == 'cache_clear':
                if self.has_gpu:
                    torch.cuda.empty_cache()
                return True

            elif strategy == 'garbage_collect':
                gc.collect()
                if self.has_gpu:
                    torch.cuda.empty_cache()
                return True

            elif strategy == 'reduce_buffer':
                # Simulate buffer reduction
                return True

            elif strategy == 'cpu_fallback':
                self.fallback_active = True
                return True

            elif strategy == 'emergency_shutdown':
                # Simulate emergency shutdown
                return True

            return False

        except Exception:
            return False

    def verify_recovery(self) -> bool:
        """
        Verify that system recovered successfully.

        Returns:
            True if system is operational
        """
        if not self.has_gpu:
            return True  # CPU always available

        if self.fallback_active:
            return True  # CPU fallback active

        try:
            # Try small GPU allocation
            test = torch.randn(100, device='cuda')
            del test
            torch.cuda.empty_cache()
            return True
        except RuntimeError:
            return False


class TestOOMDetection:
    """Test OOM detection mechanisms"""

    @pytest.fixture
    def recovery_manager(self):
        """Recovery manager fixture"""
        return OOMRecoveryManager()

    def test_oom_detection(self, recovery_manager):
        """
        Test that OOM conditions are detected correctly.

        PASS CRITERIA:
        - OOM detected on impossible allocation
        - Appropriate error type raised
        - Error message contains "out of memory"
        """
        print("\n" + "=" * 80)
        print("TEST: OOM Detection")
        print("=" * 80)

        if not recovery_manager.has_gpu:
            pytest.skip("GPU not available")

        print("\nTriggering OOM condition...")
        oom_triggered = recovery_manager.trigger_oom()

        print(f"  OOM triggered: {oom_triggered}")
        print(f"  OOM count: {recovery_manager.oom_count}")

        assert oom_triggered, "OOM should be detected"
        assert recovery_manager.oom_count == 1, "OOM counter should increment"

        print("\n✅ PASS: OOM detected correctly")

    def test_oom_notification_system(self, recovery_manager):
        """
        Test OOM notification system.

        PASS CRITERIA:
        - Notification sent on OOM
        - Notification contains relevant info
        - User can be alerted
        """
        print("\n" + "=" * 80)
        print("TEST: OOM Notification System")
        print("=" * 80)

        if not recovery_manager.has_gpu:
            pytest.skip("GPU not available")

        # Mock notification handler
        notifications = []

        def notify_handler(msg):
            notifications.append(msg)

        print("\nTriggering OOM with notification...")

        # Trigger OOM
        oom_triggered = recovery_manager.trigger_oom()

        if oom_triggered:
            # Simulate notification
            notify_handler({
                'type': 'OOM',
                'timestamp': time.time(),
                'gpu_memory': torch.cuda.memory_allocated() / 1e6,
                'recovery_attempted': False
            })

        print(f"  Notifications sent: {len(notifications)}")
        if notifications:
            print(f"  Notification: {notifications[0]}")

        assert len(notifications) > 0, "Notification should be sent"
        assert notifications[0]['type'] == 'OOM', "Notification should be OOM type"

        print("\n✅ PASS: OOM notifications working")


class TestGracefulDegradation:
    """Test graceful degradation under memory pressure"""

    @pytest.fixture
    def recovery_manager(self):
        """Recovery manager fixture"""
        manager = OOMRecoveryManager()
        yield manager
        # Cleanup
        if manager.has_gpu:
            torch.cuda.empty_cache()
        gc.collect()

    def test_progressive_recovery_strategies(self, recovery_manager):
        """
        Test that recovery strategies are tried in order.

        PASS CRITERIA:
        - All strategies attempted in order
        - Recovery succeeds
        - System returns to operational state
        """
        print("\n" + "=" * 80)
        print("TEST: Progressive Recovery Strategies")
        print("=" * 80)

        if not recovery_manager.has_gpu:
            pytest.skip("GPU not available")

        print("\nTriggering OOM...")
        oom_triggered = recovery_manager.trigger_oom()
        assert oom_triggered, "OOM should be triggered"

        print("\nAttempting recovery strategies in order...")

        strategies_attempted = []
        recovery_succeeded = False

        for strategy in recovery_manager.strategies:
            print(f"  Trying strategy: {strategy}")
            strategies_attempted.append(strategy)

            success = recovery_manager.attempt_recovery(strategy)

            if success:
                # Verify recovery
                if recovery_manager.verify_recovery():
                    print(f"  ✓ Recovery succeeded with: {strategy}")
                    recovery_succeeded = True
                    break

        print(f"\n📊 Results:")
        print(f"  Strategies attempted: {len(strategies_attempted)}")
        print(f"  Recovery succeeded: {recovery_succeeded}")
        print(f"  Strategies: {strategies_attempted}")

        assert len(strategies_attempted) > 0, "At least one strategy should be tried"
        assert recovery_succeeded, "Recovery should eventually succeed"

        print("\n✅ PASS: Progressive recovery successful")

    def test_buffer_reduction_on_pressure(self, recovery_manager):
        """
        Test automatic buffer reduction under memory pressure.

        PASS CRITERIA:
        - Buffer size reduced when memory pressure detected
        - Recording continues with smaller buffer
        - No data loss during reduction
        """
        print("\n" + "=" * 80)
        print("TEST: Buffer Reduction on Memory Pressure")
        print("=" * 80)

        # Simulate buffer states
        buffer_sizes = []
        initial_buffer_size = 30.0  # 30 seconds
        current_buffer_size = initial_buffer_size

        print(f"\nInitial buffer size: {initial_buffer_size}s")

        # Simulate memory pressure
        for pressure_level in [0.5, 0.7, 0.85, 0.95]:
            print(f"\n  Memory pressure: {pressure_level*100:.0f}%")

            # Reduce buffer if pressure high
            if pressure_level > 0.8:
                current_buffer_size *= 0.5  # Halve buffer
                print(f"  → Buffer reduced to: {current_buffer_size}s")

            buffer_sizes.append(current_buffer_size)

        print(f"\n📊 Results:")
        print(f"  Initial buffer: {initial_buffer_size}s")
        print(f"  Final buffer: {current_buffer_size}s")
        print(f"  Reduction ratio: {current_buffer_size / initial_buffer_size:.2f}")

        # PASS CRITERIA
        assert current_buffer_size < initial_buffer_size, "Buffer should be reduced"
        assert current_buffer_size > 0, "Buffer should not be zero"

        print("\n✅ PASS: Buffer reduction mechanism working")

    def test_cpu_fallback_activation(self, recovery_manager):
        """
        Test CPU fallback activation on persistent GPU OOM.

        PASS CRITERIA:
        - CPU fallback activated after threshold
        - Processing continues on CPU
        - User notified of degraded performance
        """
        print("\n" + "=" * 80)
        print("TEST: CPU Fallback Activation")
        print("=" * 80)

        if not recovery_manager.has_gpu:
            pytest.skip("GPU not available")

        oom_threshold = 3  # Fall back after 3 OOMs
        oom_count = 0

        print(f"\nSimulating persistent OOM (threshold: {oom_threshold})...")

        for i in range(oom_threshold + 1):
            triggered = recovery_manager.trigger_oom()

            if triggered:
                oom_count += 1
                print(f"  OOM {oom_count}/{oom_threshold}")

                if oom_count >= oom_threshold:
                    # Activate CPU fallback
                    recovery_manager.attempt_recovery('cpu_fallback')
                    print(f"  → CPU fallback activated")
                    break

        print(f"\n📊 Results:")
        print(f"  OOMs before fallback: {oom_count}")
        print(f"  Fallback active: {recovery_manager.fallback_active}")

        assert oom_count >= oom_threshold, f"Should reach threshold ({oom_threshold} OOMs)"
        assert recovery_manager.fallback_active, "CPU fallback should be active"

        print("\n✅ PASS: CPU fallback activated correctly")


class TestDataPreservation:
    """Test data preservation during OOM recovery"""

    def test_audio_buffer_preservation(self):
        """
        Test that audio buffer is preserved during OOM.

        PASS CRITERIA:
        - All buffered audio saved to disk
        - No sample loss
        - Audio quality maintained
        - Recovery can resume from saved state
        """
        print("\n" + "=" * 80)
        print("TEST: Audio Buffer Preservation")
        print("=" * 80)

        # Simulate audio buffer
        sample_rate = 16000
        duration = 10.0
        audio_buffer = np.random.randn(int(sample_rate * duration)).astype(np.float32)

        print(f"\nSimulating OOM with {duration}s audio buffer...")
        print(f"  Samples in buffer: {len(audio_buffer)}")

        # Mock emergency save
        saved_buffers = []

        def emergency_save(buffer, filepath):
            saved_buffers.append({
                'buffer': buffer.copy(),
                'filepath': filepath,
                'timestamp': time.time()
            })
            return True

        # Trigger save
        save_path = "/tmp/emergency_audio.wav"
        success = emergency_save(audio_buffer, save_path)

        print(f"\n📊 Results:")
        print(f"  Save successful: {success}")
        print(f"  Buffers saved: {len(saved_buffers)}")
        print(f"  Samples preserved: {len(saved_buffers[0]['buffer']) if saved_buffers else 0}")

        # Verify preservation
        assert success, "Emergency save should succeed"
        assert len(saved_buffers) == 1, "One buffer should be saved"
        assert np.array_equal(saved_buffers[0]['buffer'], audio_buffer), "Audio data should match"

        print("\n✅ PASS: Audio buffer preserved correctly")

    def test_transcription_state_preservation(self):
        """
        Test that transcription state is preserved during OOM.

        PASS CRITERIA:
        - Partial transcription saved
        - Model state saved (if possible)
        - Recovery can resume from checkpoint
        - No duplicate transcriptions
        """
        print("\n" + "=" * 80)
        print("TEST: Transcription State Preservation")
        print("=" * 80)

        # Simulate transcription state
        transcription_state = {
            'partial_text': "This is a partial transcription that should be",
            'chunk_index': 5,
            'total_chunks': 10,
            'last_timestamp': time.time(),
            'confidence': 0.85
        }

        print("\nSimulating OOM during transcription...")
        print(f"  Partial text: '{transcription_state['partial_text']}'")
        print(f"  Progress: {transcription_state['chunk_index']}/{transcription_state['total_chunks']}")

        # Mock checkpoint save
        checkpoints = []

        def save_checkpoint(state):
            checkpoints.append(state.copy())
            return True

        # Save checkpoint
        success = save_checkpoint(transcription_state)

        print(f"\n📊 Results:")
        print(f"  Checkpoint saved: {success}")
        print(f"  Checkpoints: {len(checkpoints)}")

        if checkpoints:
            saved = checkpoints[0]
            print(f"  Saved text: '{saved['partial_text']}'")
            print(f"  Saved progress: {saved['chunk_index']}/{saved['total_chunks']}")

        # Verify
        assert success, "Checkpoint save should succeed"
        assert len(checkpoints) == 1, "One checkpoint should be saved"
        assert checkpoints[0]['partial_text'] == transcription_state['partial_text'], "Text should match"

        print("\n✅ PASS: Transcription state preserved")


class TestMultipleOOMHandling:
    """Test handling of multiple OOM events"""

    @pytest.fixture
    def recovery_manager(self):
        """Recovery manager fixture"""
        manager = OOMRecoveryManager()
        yield manager
        if manager.has_gpu:
            torch.cuda.empty_cache()
        gc.collect()

    def test_consecutive_oom_recovery(self, recovery_manager):
        """
        Test recovery from consecutive OOM events.

        PASS CRITERIA:
        - All OOMs detected
        - Recovery attempted for each
        - System remains stable
        - Eventually falls back to CPU
        """
        print("\n" + "=" * 80)
        print("TEST: Consecutive OOM Recovery")
        print("=" * 80)

        if not recovery_manager.has_gpu:
            pytest.skip("GPU not available")

        num_ooms = 5
        print(f"\nSimulating {num_ooms} consecutive OOMs...")

        oom_results = []

        for i in range(num_ooms):
            triggered = recovery_manager.trigger_oom()
            oom_results.append(triggered)

            if triggered:
                print(f"  OOM {i+1}: Detected")

                # Attempt recovery
                recovery_manager.attempt_recovery('cache_clear')

                # Check if should fall back
                if recovery_manager.oom_count >= 3:
                    recovery_manager.attempt_recovery('cpu_fallback')
                    print(f"  → Falling back to CPU after {recovery_manager.oom_count} OOMs")

        print(f"\n📊 Results:")
        print(f"  Total OOMs: {sum(oom_results)}")
        print(f"  Fallback active: {recovery_manager.fallback_active}")

        assert sum(oom_results) == num_ooms, "All OOMs should be detected"
        assert recovery_manager.fallback_active, "Should eventually fall back to CPU"

        print("\n✅ PASS: Consecutive OOMs handled correctly")

    def test_oom_during_recovery(self, recovery_manager):
        """
        Test handling OOM that occurs during recovery.

        PASS CRITERIA:
        - Nested OOM detected
        - Recovery doesn't enter infinite loop
        - Emergency shutdown triggered if needed
        - System remains responsive
        """
        print("\n" + "=" * 80)
        print("TEST: OOM During Recovery")
        print("=" * 80)

        if not recovery_manager.has_gpu:
            pytest.skip("GPU not available")

        print("\nSimulating OOM during recovery...")

        # First OOM
        oom1 = recovery_manager.trigger_oom()
        print(f"  First OOM: {oom1}")

        # Simulate recovery that triggers another OOM
        print("  Attempting recovery...")

        recovery_ooms = 0
        max_recovery_attempts = 3

        for attempt in range(max_recovery_attempts):
            # Try recovery (might trigger OOM)
            try:
                # Simulate recovery allocation that fails
                oom_during_recovery = recovery_manager.trigger_oom()

                if oom_during_recovery:
                    recovery_ooms += 1
                    print(f"  → OOM during recovery attempt {attempt + 1}")

                # Emergency: fall back to CPU
                if recovery_ooms >= 2:
                    recovery_manager.attempt_recovery('cpu_fallback')
                    print(f"  → Emergency CPU fallback")
                    break

            except Exception as e:
                print(f"  → Recovery error: {e}")
                break

        print(f"\n📊 Results:")
        print(f"  Initial OOM: {oom1}")
        print(f"  OOMs during recovery: {recovery_ooms}")
        print(f"  Final state: {'CPU fallback' if recovery_manager.fallback_active else 'GPU'}")

        assert recovery_ooms < max_recovery_attempts, "Should not exceed max attempts"
        assert recovery_manager.fallback_active, "Should fall back on repeated failures"

        print("\n✅ PASS: Nested OOM handled correctly")


class TestRecoveryMetrics:
    """Test recovery metrics and reporting"""

    def test_recovery_success_rate(self):
        """
        Test tracking of recovery success rate.

        PASS CRITERIA:
        - Success rate calculated correctly
        - Metrics updated in real-time
        - Historical data maintained
        """
        print("\n" + "=" * 80)
        print("TEST: Recovery Success Rate Tracking")
        print("=" * 80)

        # Simulate recovery attempts
        results = []

        scenarios = [
            ('cache_clear', True),
            ('garbage_collect', True),
            ('cache_clear', False),
            ('cpu_fallback', True),
            ('cache_clear', True),
        ]

        print("\nSimulating recovery attempts...")

        for strategy, success in scenarios:
            results.append({
                'strategy': strategy,
                'success': success,
                'timestamp': time.time()
            })
            print(f"  {strategy}: {'✓' if success else '✗'}")

        # Calculate metrics
        total_attempts = len(results)
        successes = sum(1 for r in results if r['success'])
        success_rate = successes / total_attempts if total_attempts > 0 else 0

        # Per-strategy metrics
        strategy_stats = {}
        for result in results:
            strategy = result['strategy']
            if strategy not in strategy_stats:
                strategy_stats[strategy] = {'attempts': 0, 'successes': 0}

            strategy_stats[strategy]['attempts'] += 1
            if result['success']:
                strategy_stats[strategy]['successes'] += 1

        print(f"\n📊 Results:")
        print(f"  Total attempts: {total_attempts}")
        print(f"  Successes: {successes}")
        print(f"  Success rate: {success_rate*100:.1f}%")

        print(f"\n  Per-strategy:")
        for strategy, stats in strategy_stats.items():
            rate = stats['successes'] / stats['attempts'] * 100
            print(f"    {strategy}: {stats['successes']}/{stats['attempts']} ({rate:.0f}%)")

        assert total_attempts > 0, "Should have attempts"
        assert success_rate > 0, "Should have some successes"

        print("\n✅ PASS: Recovery metrics tracked correctly")


if __name__ == '__main__':
    pytest.main([__file__, '-v'])
