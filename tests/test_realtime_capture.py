#!/usr/bin/env python3
"""
Real-time audio capture test with simultaneous MP3 playback.
Tests the full pipeline: play MP3 → capture via loopback → transcribe → compare.
"""

import sys
import time
import subprocess
import threading
import numpy as np
from pathlib import Path

# Add src to path
sys.path.insert(0, str(Path(__file__).parent.parent / 'src'))

from audio_capture import AudioCapture, find_loopback_device
import librosa
import soundfile as sf
import torch
from nemo.collections.asr.models import EncDecMultiTaskModel

# Configuration
MODEL_NAME = 'nvidia/canary-1b-flash'
TEST_AUDIO_DIR = Path('/home/robert/Documents/python/translate-stream/examples')
SAMPLE_RATE = 16000


def calculate_wer(reference, hypothesis):
    """Calculate Word Error Rate"""
    ref_words = reference.lower().split()
    hyp_words = hypothesis.lower().split()

    d = [[0] * (len(hyp_words) + 1) for _ in range(len(ref_words) + 1)]

    for i in range(len(ref_words) + 1):
        d[i][0] = i
    for j in range(len(hyp_words) + 1):
        d[0][j] = j

    for i in range(1, len(ref_words) + 1):
        for j in range(1, len(hyp_words) + 1):
            if ref_words[i-1] == hyp_words[j-1]:
                d[i][j] = d[i-1][j-1]
            else:
                d[i][j] = min(d[i-1][j], d[i][j-1], d[i-1][j-1]) + 1

    wer = d[len(ref_words)][len(hyp_words)] / len(ref_words) if ref_words else 0
    return wer * 100


def play_audio_file(audio_path):
    """
    Play audio file using ffplay (silent background playback).
    Returns subprocess handle.
    """
    try:
        # Use ffplay for audio playback (part of ffmpeg)
        # -nodisp: no video display
        # -autoexit: exit after playback
        # -loglevel quiet: suppress output
        proc = subprocess.Popen(
            ['ffplay', '-nodisp', '-autoexit', '-loglevel', 'quiet', str(audio_path)],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL
        )
        return proc
    except Exception as e:
        print(f"✗ Failed to play audio: {e}")
        print("  Make sure ffmpeg/ffplay is installed: sudo apt install ffmpeg")
        return None


def test_realtime_capture_and_transcribe(audio_file, text_file, use_loopback=True):
    """
    Test real-time audio capture with simultaneous transcription.

    Args:
        audio_file: Path to test MP3 file
        text_file: Path to reference text file
        use_loopback: If True, capture from loopback device (system audio)
                     If False, capture from default microphone
    """
    print("=" * 80)
    print("Real-Time Audio Capture & Transcription Test")
    print("=" * 80)

    # Read reference text
    audio_path = TEST_AUDIO_DIR / audio_file
    text_path = TEST_AUDIO_DIR / text_file

    if not audio_path.exists():
        print(f"✗ Audio file not found: {audio_path}")
        return

    if not text_path.exists():
        print(f"✗ Text file not found: {text_path}")
        return

    with open(text_path, 'r', encoding='utf-8') as f:
        reference_text = f.read().strip()

    print(f"\nTest File: {audio_file}")
    print(f"Reference text ({len(reference_text)} chars):")
    print(f"  {reference_text[:100]}..." if len(reference_text) > 100 else f"  {reference_text}")

    # Get audio duration
    audio, sr = librosa.load(audio_path, sr=None, mono=False)
    audio_duration = len(audio) / sr if audio.ndim == 1 else audio.shape[1] / sr

    print(f"\nAudio duration: {audio_duration:.2f}s")

    # Find capture device
    if use_loopback:
        loopback_device = find_loopback_device()
        if loopback_device is None:
            print("\n⚠ No loopback device found! Using default microphone instead.")
            print("   Note: You'll hear the audio playing and it will be captured from mic.")
            capture_device = None
        else:
            print(f"\n✓ Found loopback device: {loopback_device}")
            capture_device = loopback_device
    else:
        print("\n🎤 Using default microphone (you'll hear the audio)")
        capture_device = None

    # Initialize audio capture
    capture = AudioCapture(
        sample_rate=SAMPLE_RATE,
        device=capture_device,
        buffer_duration=audio_duration + 2.0  # Extra 2s for buffer
    )

    # List devices
    capture.list_devices()

    print(f"\n{'=' * 80}")
    print("Starting Real-Time Capture & Playback")
    print('=' * 80)

    # Start capture
    print("\n1️⃣ Starting audio capture...")
    capture.start()
    time.sleep(0.5)  # Let capture initialize

    # Start playback
    print("2️⃣ Playing audio file...")
    playback_proc = play_audio_file(audio_path)

    if playback_proc is None:
        capture.stop()
        return

    # Monitor playback
    start_time = time.time()
    while playback_proc.poll() is None:
        elapsed = time.time() - start_time
        buffer_duration = capture.get_buffer_duration()
        print(f"\r   ⏱️ Recording: {elapsed:.1f}s | Buffered: {buffer_duration:.1f}s", end='', flush=True)
        time.sleep(0.1)

    # Wait a bit more to ensure all audio is captured
    time.sleep(0.5)

    # Stop capture
    print("\n\n3️⃣ Stopping capture...")
    captured_audio = capture.stop()

    if len(captured_audio) == 0:
        print("✗ No audio captured!")
        return

    # Analyze captured audio
    rms = np.sqrt(np.mean(captured_audio**2))
    db = 20 * np.log10(rms) if rms > 0 else -100
    captured_duration = len(captured_audio) / SAMPLE_RATE

    print(f"\n📊 Captured Audio Analysis:")
    print(f"   Duration: {captured_duration:.2f}s")
    print(f"   Samples: {len(captured_audio)}")
    print(f"   RMS Energy: {db:.2f} dB")

    if db < -40:
        print(f"\n⚠ WARNING: Very quiet audio captured!")
        print(f"   This may indicate a problem with loopback/capture setup.")

    # Save captured audio for inspection
    temp_capture_path = Path('/tmp/swictation_captured.wav')
    sf.write(temp_capture_path, captured_audio, SAMPLE_RATE)
    print(f"\n💾 Saved captured audio to: {temp_capture_path}")

    # Load STT model
    print(f"\n{'=' * 80}")
    print("Loading STT Model")
    print('=' * 80)

    print(f"\n4️⃣ Loading {MODEL_NAME}...")
    load_start = time.time()
    model = EncDecMultiTaskModel.from_pretrained(MODEL_NAME)
    model.eval()
    if torch.cuda.is_available():
        model = model.cuda()
    load_time = time.time() - load_start
    print(f"✓ Model loaded in {load_time:.2f}s")

    # Transcribe captured audio
    print(f"\n5️⃣ Transcribing captured audio...")
    transcribe_start = time.time()

    hypothesis = model.transcribe(
        audio=str(temp_capture_path),
        source_lang='en',
        target_lang='en',
        pnc='yes',
        batch_size=1
    )[0]

    transcription = hypothesis.text if hasattr(hypothesis, 'text') else str(hypothesis)
    transcribe_time = time.time() - transcribe_start

    print(f"✓ Transcribed in {transcribe_time*1000:.0f}ms")

    # Results
    print(f"\n{'=' * 80}")
    print("RESULTS")
    print('=' * 80)

    print(f"\nTranscription ({len(transcription)} chars):")
    print(f"  {transcription}")

    wer = calculate_wer(reference_text, transcription)
    rtf = transcribe_time / captured_duration if captured_duration > 0 else 0

    print(f"\n📊 Metrics:")
    print(f"  WER: {wer:.2f}%")
    print(f"  Transcription Latency: {transcribe_time*1000:.0f}ms")
    print(f"  RTF: {rtf:.3f}x (1.0 = realtime)")
    print(f"  Total Pipeline Time: {(time.time() - start_time):.2f}s")

    if wer < 20:
        print(f"\n🎉 SUCCESS! End-to-end pipeline working!")
        if use_loopback:
            print(f"   Loopback capture + transcription = {wer:.1f}% WER")
    else:
        print(f"\n⚠ High WER. Check:")
        print(f"   - Loopback device configuration")
        print(f"   - Audio capture quality")
        print(f"   - System audio routing")

    print("\n" + "=" * 80)


if __name__ == '__main__':
    import argparse

    parser = argparse.ArgumentParser(description='Test real-time audio capture and transcription')
    parser.add_argument('--file', default='en-short.mp3', help='Test audio file (default: en-short.mp3)')
    parser.add_argument('--text', default='en-short.txt', help='Reference text file')
    parser.add_argument('--mic', action='store_true', help='Use microphone instead of loopback')
    parser.add_argument('--list-devices', action='store_true', help='List audio devices and exit')

    args = parser.parse_args()

    if args.list_devices:
        capture = AudioCapture()
        capture.list_devices()
        sys.exit(0)

    test_realtime_capture_and_transcribe(
        args.file,
        args.text,
        use_loopback=not args.mic
    )
