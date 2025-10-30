#!/usr/bin/env python3
"""
Test NVIDIA Canary-1B-Flash STT model with sample audio files.
Measures accuracy (WER), latency, and GPU utilization.
"""

import time
import torch
from pathlib import Path
from nemo.collections.asr.models import EncDecMultiTaskModel

# Configuration
MODEL_NAME = 'nvidia/canary-1b-flash'
TEST_AUDIO_DIR = Path('/home/robert/Documents/python/translate-stream/examples')
TEST_FILES = [
    ('en-short.mp3', 'en-short.txt'),
    ('en-long.mp3', 'en-long.txt'),
]

def calculate_wer(reference, hypothesis):
    """Calculate Word Error Rate"""
    ref_words = reference.lower().split()
    hyp_words = hypothesis.lower().split()

    # Simple Levenshtein distance for words
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

def main():
    print("=" * 80)
    print("NVIDIA Canary-1B-Flash STT Test")
    print("=" * 80)

    # System info
    print(f"\nGPU: {torch.cuda.get_device_name(0) if torch.cuda.is_available() else 'CPU'}")
    print(f"CUDA Available: {torch.cuda.is_available()}")
    if torch.cuda.is_available():
        print(f"CUDA Version: {torch.version.cuda}")
        print(f"GPU Memory: {torch.cuda.get_device_properties(0).total_memory / 1e9:.2f} GB")

    # Load model
    print(f"\nLoading model: {MODEL_NAME}")
    load_start = time.time()
    model = EncDecMultiTaskModel.from_pretrained(MODEL_NAME)
    model.eval()
    if torch.cuda.is_available():
        model = model.cuda()
    load_time = time.time() - load_start
    print(f"✓ Model loaded in {load_time:.2f}s")

    # Test each audio file
    results = []
    for audio_file, text_file in TEST_FILES:
        print(f"\n{'=' * 80}")
        print(f"Testing: {audio_file}")
        print('=' * 80)

        audio_path = TEST_AUDIO_DIR / audio_file
        text_path = TEST_AUDIO_DIR / text_file

        if not audio_path.exists():
            print(f"✗ Audio file not found: {audio_path}")
            continue

        if not text_path.exists():
            print(f"✗ Text file not found: {text_path}")
            continue

        # Read reference text
        with open(text_path, 'r', encoding='utf-8') as f:
            reference_text = f.read().strip()

        print(f"\nReference text ({len(reference_text)} chars):")
        print(f"  {reference_text[:100]}..." if len(reference_text) > 100 else f"  {reference_text}")

        # Transcribe with GPU timing
        print(f"\nTranscribing with Canary-1B-Flash...")

        if torch.cuda.is_available():
            torch.cuda.synchronize()
            mem_before = torch.cuda.memory_allocated() / 1e6  # MB

        transcribe_start = time.time()

        # Canary-1B-Flash inference
        # For EncDecMultiTaskModel, use transcribe with proper arguments
        hypothesis = model.transcribe(
            audio=str(audio_path),
            source_lang='en',  # Source language
            target_lang='en',  # Target language (transcription, not translation)
            pnc='yes',  # Enable punctuation and capitalization ('yes' or 'no')
            batch_size=1
        )[0]

        # Extract text from Hypothesis object
        transcription = hypothesis.text if hasattr(hypothesis, 'text') else str(hypothesis)

        if torch.cuda.is_available():
            torch.cuda.synchronize()
            mem_after = torch.cuda.memory_allocated() / 1e6  # MB

        transcribe_time = time.time() - transcribe_start

        # Calculate metrics
        wer = calculate_wer(reference_text, transcription)
        audio_duration = len(reference_text.split()) * 0.6  # Rough estimate
        rtf = transcribe_time / audio_duration if audio_duration > 0 else 0

        print(f"\n✓ Transcription complete ({transcribe_time*1000:.0f}ms)")
        print(f"\nTranscription ({len(transcription)} chars):")
        print(f"  {transcription[:100]}..." if len(transcription) > 100 else f"  {transcription}")

        print(f"\n📊 Metrics:")
        print(f"  WER: {wer:.2f}%")
        print(f"  Latency: {transcribe_time*1000:.0f}ms")
        print(f"  RTF: {rtf:.3f}x (1.0 = realtime)")

        if torch.cuda.is_available():
            print(f"  GPU Memory Used: {mem_after - mem_before:.1f} MB")

        results.append({
            'file': audio_file,
            'wer': wer,
            'latency_ms': transcribe_time * 1000,
            'rtf': rtf,
            'reference': reference_text,
            'transcription': transcription
        })

    # Summary
    print(f"\n{'=' * 80}")
    print("SUMMARY")
    print('=' * 80)

    if results:
        avg_wer = sum(r['wer'] for r in results) / len(results)
        avg_latency = sum(r['latency_ms'] for r in results) / len(results)
        avg_rtf = sum(r['rtf'] for r in results) / len(results)

        print(f"\n✓ Tested {len(results)} audio files")
        print(f"  Average WER: {avg_wer:.2f}%")
        print(f"  Average Latency: {avg_latency:.0f}ms")
        print(f"  Average RTF: {avg_rtf:.3f}x")

        if avg_wer < 10:
            print(f"\n🎉 Excellent! WER < 10% - Model working perfectly!")
        elif avg_wer < 20:
            print(f"\n✓ Good! WER < 20% - Model working well")
        else:
            print(f"\n⚠ WER > 20% - Check audio quality or model configuration")

    print("\n" + "=" * 80)

if __name__ == '__main__':
    main()
