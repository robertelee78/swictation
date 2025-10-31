#!/usr/bin/env python3
"""
Directly test the daemon's transcription with a known audio file.
"""
import sys
sys.path.insert(0, 'src')

import librosa
import soundfile as sf
import numpy as np
from pathlib import Path

# Import daemon to use its transcription logic
from swictationd import SwictationDaemon

def test_direct():
    """Test transcription using daemon's methods"""
    print("Initializing daemon...")
    daemon = SwictationDaemon()

    # Load models
    print("Loading VAD...")
    daemon.load_vad_model()

    print("Loading STT...")
    daemon.load_stt_model()

    # Load test audio
    audio_file = '/home/robert/Documents/python/translate-stream/examples/en-short.mp3'
    print(f"\nLoading audio from {audio_file}...")

    audio, sr = librosa.load(audio_file, sr=16000, mono=True)
    print(f"  Duration: {len(audio)/sr:.2f}s")
    print(f"  Sample rate: {sr}")
    print(f"  Max level: {np.abs(audio).max():.4f}")

    # Save as WAV
    temp_wav = '/tmp/test_daemon_audio.wav'
    sf.write(temp_wav, audio, sr)

    # Test transcription directly
    print("\nTranscribing with model.transcribe()...")
    hypothesis = daemon.stt_model.transcribe(
        audio=temp_wav,
        source_lang='en',
        target_lang='en',
        pnc='yes',
        batch_size=1
    )[0]

    print(f"\nHypothesis type: {type(hypothesis)}")
    print(f"Hypothesis: {repr(hypothesis)}")

    if hasattr(hypothesis, 'text'):
        text = hypothesis.text
        print(f"\nhypothesis.text = '{text}'")
    elif hasattr(hypothesis, '__dict__'):
        print(f"\nhypothesis.__dict__ = {hypothesis.__dict__}")

    text_str = str(hypothesis)
    print(f"str(hypothesis) = '{text_str}'")

    return text_str

if __name__ == '__main__':
    try:
        result = test_direct()
        print(f"\n{'='*60}")
        print(f"FINAL RESULT: '{result}'")
        print(f"{'='*60}")
    except Exception as e:
        print(f"\n✗ Error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
