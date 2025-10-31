#!/usr/bin/env python3
"""
Test transcription with a known audio file.
"""
import sys
sys.path.insert(0, 'src')

import torch
import librosa
import soundfile as sf
from nemo.collections.asr.models import EncDecMultiTaskModel
from pathlib import Path

def test_transcription(audio_file: str):
    """Test transcription with a test audio file"""
    print(f"Testing transcription with: {audio_file}")

    # Load model
    print("Loading model...")
    model = EncDecMultiTaskModel.from_pretrained('nvidia/canary-1b-flash')
    model.eval()

    if torch.cuda.is_available():
        model = model.cuda()
        print("Using GPU")

    # Convert audio to 16kHz WAV if needed
    print(f"\nLoading audio...")
    audio, sr = librosa.load(audio_file, sr=16000, mono=True)
    print(f"  Duration: {len(audio)/sr:.2f}s")
    print(f"  Sample rate: {sr} Hz")

    # Save as temp WAV for model
    temp_wav = '/tmp/test_audio.wav'
    sf.write(temp_wav, audio, sr)

    # Transcribe
    print("\nTranscribing...")
    hypothesis = model.transcribe(
        audio=temp_wav,
        source_lang='en',
        target_lang='en',
        pnc='yes',
        batch_size=1
    )[0]

    print(f"\nHypothesis type: {type(hypothesis)}")
    print(f"Hypothesis: {hypothesis}")

    if hasattr(hypothesis, 'text'):
        text = hypothesis.text
        print(f"\nTranscription: {text}")
    else:
        text = str(hypothesis)
        print(f"\nTranscription (str): {text}")

    return text

if __name__ == '__main__':
    # Use first available test file
    test_files = list(Path('/home/robert/Documents/python/translate-stream/examples').glob('en*.mp3'))

    if not test_files:
        print("No test files found!")
        sys.exit(1)

    test_file = str(test_files[0])
    text = test_transcription(test_file)

    if text and text.strip():
        print("\n✓ Transcription successful!")
        sys.exit(0)
    else:
        print("\n✗ Empty transcription!")
        sys.exit(1)
