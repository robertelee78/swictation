#!/usr/bin/env python3
"""
Test audio recording to verify microphone is working.
"""
import sys
import sounddevice as sd
import numpy as np
import soundfile as sf
from pathlib import Path

def test_recording():
    """Record 3 seconds and save to file"""
    print("Testing audio recording...")
    print(f"Default input device: {sd.query_devices(kind='input')['name']}")

    sample_rate = 16000
    duration = 3  # seconds

    print(f"\nRecording {duration} seconds...")
    print("Please speak now!")

    # Record
    audio = sd.rec(
        int(duration * sample_rate),
        samplerate=sample_rate,
        channels=1,
        dtype='float32',
        device=None  # Use default
    )
    sd.wait()

    print("✓ Recording complete")

    # Check audio level
    max_level = np.abs(audio).max()
    rms_level = np.sqrt(np.mean(audio**2))

    print(f"\nAudio statistics:")
    print(f"  Max level: {max_level:.4f}")
    print(f"  RMS level: {rms_level:.4f}")
    print(f"  Samples: {len(audio)}")

    if max_level < 0.01:
        print("⚠ WARNING: Audio level very low! Check microphone.")
    elif max_level > 0.1:
        print("✓ Good audio level detected")
    else:
        print("⚠ Audio level marginal")

    # Save to file
    output_path = Path('/tmp/test_recording.wav')
    sf.write(output_path, audio, sample_rate)
    print(f"\n✓ Saved to {output_path}")
    print(f"  You can play it with: aplay {output_path}")

    return max_level > 0.01

if __name__ == '__main__':
    success = test_recording()
    sys.exit(0 if success else 1)
