#!/usr/bin/env python3
"""
Example: Using AudioCapture in streaming mode for Wait-k streaming transcription.

This example demonstrates how to use the refactored AudioCapture class
to yield 1-second chunks suitable for NeMo Wait-k streaming.
"""

import sys
import time
import numpy as np
from pathlib import Path

# Add src to path
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from audio_capture import AudioCapture


def simulate_transcription(chunk: np.ndarray):
    """
    Simulate transcription processing on a chunk.
    In real usage, this would call NeMo streaming transcription.
    """
    chunk_duration = len(chunk) / 16000
    rms = np.sqrt(np.mean(chunk**2))
    db = 20 * np.log10(rms) if rms > 0 else -100

    print(f"  📝 Processing chunk: {len(chunk)} samples ({chunk_duration:.3f}s), RMS: {db:.1f} dB")

    # Simulate some processing delay
    time.sleep(0.05)  # 50ms processing time

    return f"Transcribed text from {chunk_duration:.1f}s chunk"


def main():
    """Main streaming example"""

    print("=" * 80)
    print("AudioCapture Streaming Mode Example")
    print("Wait-k Streaming for NeMo Transcription")
    print("=" * 80)

    # Configuration for Wait-k streaming
    sample_rate = 16000  # 16kHz required by NeMo
    chunk_duration = 1.0  # 1-second chunks
    total_duration = 10.0  # Record for 10 seconds

    print(f"\n📋 Configuration:")
    print(f"   Sample rate: {sample_rate} Hz")
    print(f"   Chunk duration: {chunk_duration}s ({int(chunk_duration * sample_rate)} frames)")
    print(f"   Recording duration: {total_duration}s")
    print(f"   Expected chunks: ~{int(total_duration / chunk_duration)}")

    # Create audio capture in streaming mode
    capture = AudioCapture(
        sample_rate=sample_rate,
        chunk_duration=chunk_duration,
        streaming_mode=True  # Enable streaming mode
    )

    # Track chunks
    chunks_received = 0
    transcriptions = []

    def on_chunk_ready(chunk: np.ndarray):
        """Callback when a chunk is ready"""
        nonlocal chunks_received
        chunks_received += 1

        print(f"\n🎤 Chunk {chunks_received} ready!")

        # Process chunk (simulate transcription)
        transcription = simulate_transcription(chunk)
        transcriptions.append(transcription)

        # In real usage, you would:
        # 1. Send chunk to NeMo streaming API
        # 2. Get partial transcription result
        # 3. Update UI with streaming text
        # 4. Implement Wait-k logic for context

    # Set the chunk callback
    capture.on_chunk_ready = on_chunk_ready

    try:
        print(f"\n🎤 Starting streaming capture...")
        print("Speak into your microphone!\n")

        # Start recording
        capture.start()

        # Record for specified duration
        start_time = time.time()
        while time.time() - start_time < total_duration:
            # Show progress
            elapsed = time.time() - start_time
            progress = elapsed / total_duration * 100
            buffer_progress = capture.get_chunk_buffer_progress() * 100

            sys.stdout.write(f"\r⏱️  Recording: {elapsed:.1f}s / {total_duration:.1f}s ({progress:.0f}%) | Buffer: {buffer_progress:.0f}%  ")
            sys.stdout.flush()

            time.sleep(0.1)

        print("\n")

        # Stop recording
        audio = capture.stop()

        # Print summary
        print("\n" + "=" * 80)
        print("📊 STREAMING SUMMARY")
        print("=" * 80)
        print(f"\nChunks received: {chunks_received}")
        print(f"Transcriptions generated: {len(transcriptions)}")
        print(f"Total audio captured: {len(audio)} samples ({len(audio) / sample_rate:.2f}s)")

        if transcriptions:
            print("\n📝 Sample transcriptions:")
            for i, text in enumerate(transcriptions[:5], 1):
                print(f"  {i}. {text}")

        print("\n✅ Streaming example completed successfully!")

    except KeyboardInterrupt:
        print("\n\n⚠️  Interrupted by user")
        capture.stop()

    except Exception as e:
        print(f"\n❌ Error: {e}")
        import traceback
        traceback.print_exc()
        capture.stop()


if __name__ == '__main__':
    main()
