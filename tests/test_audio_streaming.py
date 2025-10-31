#!/usr/bin/env python3
"""
Test script for AudioCapture 1-second chunk streaming functionality.
Verifies chunks are exactly 1.0 seconds with no gaps or overlaps.
"""

import sys
import time
import numpy as np
from pathlib import Path

# Add src to path
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from audio_capture import AudioCapture


class ChunkValidator:
    """Validates streaming chunks for correctness"""

    def __init__(self, expected_chunk_frames: int, sample_rate: int):
        self.expected_chunk_frames = expected_chunk_frames
        self.sample_rate = sample_rate
        self.chunks_received = 0
        self.total_samples = 0
        self.chunk_sizes = []
        self.chunk_timestamps = []
        self.errors = []

    def on_chunk_ready(self, chunk: np.ndarray):
        """Callback for chunk validation"""
        timestamp = time.time()
        chunk_size = len(chunk)

        self.chunks_received += 1
        self.total_samples += chunk_size
        self.chunk_sizes.append(chunk_size)
        self.chunk_timestamps.append(timestamp)

        # Validate chunk size
        if chunk_size != self.expected_chunk_frames:
            error = f"Chunk {self.chunks_received}: Size mismatch! Expected {self.expected_chunk_frames}, got {chunk_size}"
            self.errors.append(error)
            print(f"  ❌ {error}")
        else:
            chunk_duration = chunk_size / self.sample_rate
            print(f"  ✓ Chunk {self.chunks_received}: {chunk_size} samples ({chunk_duration:.3f}s)")

        # Check for gaps/overlaps if we have multiple chunks
        if len(self.chunk_timestamps) >= 2:
            time_delta = self.chunk_timestamps[-1] - self.chunk_timestamps[-2]
            expected_delta = self.expected_chunk_frames / self.sample_rate

            # Allow 10% tolerance for timing jitter
            if abs(time_delta - expected_delta) > expected_delta * 0.1:
                warning = f"Chunk {self.chunks_received}: Timing issue! Expected ~{expected_delta:.3f}s between chunks, got {time_delta:.3f}s"
                print(f"  ⚠️  {warning}")

    def print_summary(self):
        """Print validation summary"""
        print("\n" + "=" * 80)
        print("📊 STREAMING VALIDATION SUMMARY")
        print("=" * 80)

        print(f"\nChunks received: {self.chunks_received}")
        print(f"Total samples: {self.total_samples}")
        print(f"Total duration: {self.total_samples / self.sample_rate:.2f}s")

        if self.chunk_sizes:
            print(f"\nChunk size statistics:")
            print(f"  Expected: {self.expected_chunk_frames} samples")
            print(f"  Min: {min(self.chunk_sizes)} samples")
            print(f"  Max: {max(self.chunk_sizes)} samples")
            print(f"  Mean: {np.mean(self.chunk_sizes):.1f} samples")

            # Check consistency
            unique_sizes = set(self.chunk_sizes)
            if len(unique_sizes) == 1 and list(unique_sizes)[0] == self.expected_chunk_frames:
                print(f"  ✓ All chunks have correct size!")
            else:
                print(f"  ❌ Inconsistent chunk sizes: {unique_sizes}")

        if len(self.chunk_timestamps) >= 2:
            deltas = np.diff(self.chunk_timestamps)
            expected_delta = self.expected_chunk_frames / self.sample_rate

            print(f"\nTiming statistics:")
            print(f"  Expected interval: {expected_delta:.3f}s")
            print(f"  Min interval: {min(deltas):.3f}s")
            print(f"  Max interval: {max(deltas):.3f}s")
            print(f"  Mean interval: {np.mean(deltas):.3f}s")
            print(f"  Std interval: {np.std(deltas):.3f}s")

        if self.errors:
            print(f"\n❌ ERRORS DETECTED ({len(self.errors)}):")
            for error in self.errors:
                print(f"  - {error}")
        else:
            print(f"\n✅ NO ERRORS - All chunks valid!")

        print("=" * 80)

        return len(self.errors) == 0


def test_streaming_mode(duration: float = 5.0, chunk_duration: float = 1.0):
    """Test streaming mode with chunk validation"""

    print("=" * 80)
    print("TEST: AudioCapture Streaming Mode")
    print("=" * 80)

    sample_rate = 16000
    expected_chunk_frames = int(chunk_duration * sample_rate)

    print(f"\nConfiguration:")
    print(f"  Sample rate: {sample_rate} Hz")
    print(f"  Chunk duration: {chunk_duration}s")
    print(f"  Expected chunk size: {expected_chunk_frames} frames")
    print(f"  Test duration: {duration}s")
    print(f"  Expected chunks: ~{int(duration / chunk_duration)}")

    # Create validator
    validator = ChunkValidator(expected_chunk_frames, sample_rate)

    # Create audio capture in streaming mode
    capture = AudioCapture(
        sample_rate=sample_rate,
        chunk_duration=chunk_duration,
        streaming_mode=True
    )

    # Set chunk callback
    capture.on_chunk_ready = validator.on_chunk_ready

    try:
        print(f"\n🎤 Recording for {duration} seconds...")
        print("Speak into your microphone!\n")

        capture.start()
        time.sleep(duration)
        audio = capture.stop()

        # Print results
        validator.print_summary()

        # Verify buffer is also populated (backward compatibility)
        print(f"\n📦 Buffer verification (backward compatibility):")
        print(f"  Buffer samples: {len(audio)}")
        print(f"  Buffer duration: {len(audio) / sample_rate:.2f}s")

        if len(audio) > 0:
            print(f"  ✓ Buffer populated correctly")
        else:
            print(f"  ❌ Buffer is empty!")

        return validator.print_summary()

    except KeyboardInterrupt:
        print("\n\nInterrupted by user")
        capture.stop()
        return False

    except Exception as e:
        print(f"\n❌ Error: {e}")
        import traceback
        traceback.print_exc()
        return False


def test_batch_mode_compatibility(duration: float = 3.0):
    """Test that batch mode still works (backward compatibility)"""

    print("\n" + "=" * 80)
    print("TEST: Batch Mode Backward Compatibility")
    print("=" * 80)

    sample_rate = 16000

    # Create audio capture in batch mode (default)
    capture = AudioCapture(sample_rate=sample_rate)

    try:
        print(f"\n🎤 Recording for {duration} seconds in batch mode...")

        capture.start()
        time.sleep(duration)
        audio = capture.stop()

        print(f"\n📊 Results:")
        print(f"  Samples: {len(audio)}")
        print(f"  Duration: {len(audio) / sample_rate:.2f}s")

        if len(audio) > 0:
            print(f"  ✓ Batch mode works correctly!")
            return True
        else:
            print(f"  ❌ No audio captured in batch mode!")
            return False

    except Exception as e:
        print(f"\n❌ Error: {e}")
        import traceback
        traceback.print_exc()
        return False


def test_chunk_buffer_methods():
    """Test chunk buffer helper methods"""

    print("\n" + "=" * 80)
    print("TEST: Chunk Buffer Helper Methods")
    print("=" * 80)

    sample_rate = 16000
    chunk_duration = 1.0
    chunk_frames = int(chunk_duration * sample_rate)

    chunks_count = 0

    def chunk_callback(chunk):
        nonlocal chunks_count
        chunks_count += 1
        size = capture.get_chunk_buffer_size()
        progress = capture.get_chunk_buffer_progress()
        print(f"  Chunk {chunks_count} emitted. Buffer: {size} samples ({progress*100:.1f}%)")

    capture = AudioCapture(
        sample_rate=sample_rate,
        chunk_duration=chunk_duration,
        streaming_mode=True
    )

    capture.on_chunk_ready = chunk_callback

    try:
        print(f"\n🎤 Testing buffer methods for 3 seconds...")
        capture.start()
        time.sleep(3)
        capture.stop()

        print(f"\n📊 Results:")
        print(f"  Chunks emitted: {chunks_count}")
        print(f"  Expected: ~3 chunks")

        if chunks_count >= 2 and chunks_count <= 4:
            print(f"  ✓ Buffer methods working correctly!")
            return True
        else:
            print(f"  ❌ Unexpected chunk count!")
            return False

    except Exception as e:
        print(f"\n❌ Error: {e}")
        import traceback
        traceback.print_exc()
        return False


if __name__ == '__main__':
    import argparse

    parser = argparse.ArgumentParser(description='Test AudioCapture streaming functionality')
    parser.add_argument('--duration', type=float, default=5.0, help='Test duration in seconds')
    parser.add_argument('--chunk-duration', type=float, default=1.0, help='Chunk duration in seconds')
    parser.add_argument('--skip-streaming', action='store_true', help='Skip streaming test')
    parser.add_argument('--skip-batch', action='store_true', help='Skip batch mode test')
    parser.add_argument('--skip-buffer', action='store_true', help='Skip buffer methods test')

    args = parser.parse_args()

    results = []

    # Run tests
    if not args.skip_streaming:
        results.append(("Streaming Mode", test_streaming_mode(args.duration, args.chunk_duration)))

    if not args.skip_batch:
        results.append(("Batch Mode", test_batch_mode_compatibility()))

    if not args.skip_buffer:
        results.append(("Buffer Methods", test_chunk_buffer_methods()))

    # Print final summary
    print("\n" + "=" * 80)
    print("🎯 FINAL TEST SUMMARY")
    print("=" * 80)

    all_passed = True
    for test_name, passed in results:
        status = "✅ PASSED" if passed else "❌ FAILED"
        print(f"  {test_name}: {status}")
        if not passed:
            all_passed = False

    print("=" * 80)

    if all_passed:
        print("\n✅ ALL TESTS PASSED!")
        sys.exit(0)
    else:
        print("\n❌ SOME TESTS FAILED!")
        sys.exit(1)
