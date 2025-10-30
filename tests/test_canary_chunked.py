#!/usr/bin/env python3
"""
Test NVIDIA Canary-1B-Flash STT with intelligent chunking for memory optimization.
Handles longer audio files on RTX A1000 (4GB VRAM) by processing in chunks.
"""

import time
import torch
import gc
import librosa
import soundfile as sf
from pathlib import Path
from nemo.collections.asr.models import EncDecMultiTaskModel
import numpy as np

# Configuration
MODEL_NAME = 'nvidia/canary-1b-flash'
TEST_AUDIO_DIR = Path('/home/robert/Documents/python/translate-stream/examples')
TEST_FILES = [
    ('en-short.mp3', 'en-short.txt'),
    ('en-long.mp3', 'en-long.txt'),
]

# Chunking configuration
CHUNK_DURATION = 10.0  # seconds per chunk (increased from 5s for better context)
CHUNK_OVERLAP = 1.0    # seconds of overlap between chunks (for word boundary handling)
SAMPLE_RATE = 16000    # Canary-1B-Flash expects 16kHz

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

def clear_gpu_cache():
    """Clear GPU cache to free memory"""
    if torch.cuda.is_available():
        torch.cuda.empty_cache()
        gc.collect()

def get_gpu_memory_mb():
    """Get current GPU memory usage in MB"""
    if torch.cuda.is_available():
        return torch.cuda.memory_allocated() / 1e6
    return 0

def chunk_audio(audio_path, chunk_duration=10.0, overlap=1.0):
    """
    Load and chunk audio file with overlap for context continuity.

    Args:
        audio_path: Path to audio file
        chunk_duration: Duration of each chunk in seconds
        overlap: Overlap between chunks in seconds

    Returns:
        List of (audio_chunk, start_time, end_time) tuples
    """
    # Load audio with librosa (handles various formats, resamples to target rate)
    audio, sr = librosa.load(audio_path, sr=SAMPLE_RATE, mono=True)

    total_duration = len(audio) / sr
    chunk_samples = int(chunk_duration * sr)
    overlap_samples = int(overlap * sr)
    stride = chunk_samples - overlap_samples

    chunks = []
    start_sample = 0

    while start_sample < len(audio):
        end_sample = min(start_sample + chunk_samples, len(audio))
        chunk = audio[start_sample:end_sample]

        start_time = start_sample / sr
        end_time = end_sample / sr

        chunks.append((chunk, start_time, end_time))

        # Move to next chunk
        start_sample += stride

        # Break if this was the last chunk
        if end_sample >= len(audio):
            break

    return chunks, total_duration, sr

def transcribe_chunk(model, audio_chunk, temp_path):
    """Transcribe a single audio chunk"""
    # Save chunk to temporary file (NeMo requires file path)
    sf.write(temp_path, audio_chunk, SAMPLE_RATE)

    # Clear cache before transcription
    clear_gpu_cache()

    # Track memory
    mem_before = get_gpu_memory_mb()

    # Transcribe
    hypothesis = model.transcribe(
        audio=str(temp_path),
        source_lang='en',
        target_lang='en',
        pnc='yes',
        batch_size=1
    )[0]

    mem_after = get_gpu_memory_mb()
    mem_used = mem_after - mem_before

    # Extract text from Hypothesis object
    transcription = hypothesis.text if hasattr(hypothesis, 'text') else str(hypothesis)

    return transcription, mem_used

def merge_chunk_transcriptions(chunk_results, overlap_duration=1.0):
    """
    Merge transcriptions from overlapping chunks intelligently.

    Strategy: Use overlap region to detect word boundaries and avoid duplicates.
    For now, simple concatenation with space. Can be improved with word matching.
    """
    if not chunk_results:
        return ""

    if len(chunk_results) == 1:
        return chunk_results[0]['text']

    # Simple merge: concatenate with space
    # TODO: Implement smart overlap handling using word boundary detection
    merged = []
    for i, result in enumerate(chunk_results):
        text = result['text'].strip()
        if i == 0:
            merged.append(text)
        else:
            # For overlapping chunks, simple concatenation
            # In production, should analyze overlap region for duplicate words
            merged.append(text)

    return ' '.join(merged)

def main():
    print("=" * 80)
    print("NVIDIA Canary-1B-Flash STT Test (Memory-Optimized Chunking)")
    print("=" * 80)

    # System info
    print(f"\nGPU: {torch.cuda.get_device_name(0) if torch.cuda.is_available() else 'CPU'}")
    print(f"CUDA Available: {torch.cuda.is_available()}")
    if torch.cuda.is_available():
        print(f"CUDA Version: {torch.version.cuda}")
        print(f"GPU Memory: {torch.cuda.get_device_properties(0).total_memory / 1e9:.2f} GB")
        print(f"GPU Memory Available: {(torch.cuda.get_device_properties(0).total_memory - torch.cuda.memory_allocated()) / 1e9:.2f} GB")

    # Chunking configuration
    print(f"\nChunking Configuration:")
    print(f"  Chunk Duration: {CHUNK_DURATION}s")
    print(f"  Chunk Overlap: {CHUNK_OVERLAP}s")
    print(f"  Sample Rate: {SAMPLE_RATE} Hz")

    # Load model
    print(f"\nLoading model: {MODEL_NAME}")
    load_start = time.time()
    model = EncDecMultiTaskModel.from_pretrained(MODEL_NAME)
    model.eval()
    if torch.cuda.is_available():
        model = model.cuda()
    load_time = time.time() - load_start

    model_memory = get_gpu_memory_mb()
    print(f"✓ Model loaded in {load_time:.2f}s")
    print(f"  Model GPU Memory: {model_memory:.1f} MB")

    # Create temporary directory for chunks
    temp_dir = Path('/tmp/swictation_chunks')
    temp_dir.mkdir(exist_ok=True)
    temp_chunk_path = temp_dir / 'chunk.wav'

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

        # Chunk audio
        print(f"\nChunking audio file...")
        chunks, total_duration, sr = chunk_audio(audio_path, CHUNK_DURATION, CHUNK_OVERLAP)
        print(f"✓ Audio chunked into {len(chunks)} parts ({total_duration:.1f}s total)")

        # Transcribe chunks
        print(f"\nTranscribing {len(chunks)} chunks...")
        chunk_results = []
        total_transcribe_time = 0

        for i, (chunk, start_time, end_time) in enumerate(chunks, 1):
            chunk_duration = end_time - start_time
            print(f"\n  Chunk {i}/{len(chunks)} ({start_time:.1f}s-{end_time:.1f}s, {chunk_duration:.1f}s)")

            chunk_start = time.time()

            try:
                transcription, mem_used = transcribe_chunk(model, chunk, temp_chunk_path)
                chunk_time = time.time() - chunk_start
                total_transcribe_time += chunk_time

                print(f"    ✓ Transcribed in {chunk_time*1000:.0f}ms (RTF: {chunk_time/chunk_duration:.3f}x)")
                print(f"    Memory: {mem_used:.1f} MB")
                print(f"    Text: {transcription[:80]}..." if len(transcription) > 80 else f"    Text: {transcription}")

                chunk_results.append({
                    'text': transcription,
                    'start_time': start_time,
                    'end_time': end_time,
                    'latency_ms': chunk_time * 1000,
                    'mem_mb': mem_used
                })

            except torch.cuda.OutOfMemoryError as e:
                print(f"    ✗ CUDA OOM Error: {e}")
                print(f"    Attempting GPU cache clear and retry...")
                clear_gpu_cache()
                time.sleep(0.5)

                try:
                    transcription, mem_used = transcribe_chunk(model, chunk, temp_chunk_path)
                    chunk_time = time.time() - chunk_start
                    total_transcribe_time += chunk_time
                    print(f"    ✓ Retry succeeded in {chunk_time*1000:.0f}ms")

                    chunk_results.append({
                        'text': transcription,
                        'start_time': start_time,
                        'end_time': end_time,
                        'latency_ms': chunk_time * 1000,
                        'mem_mb': mem_used
                    })
                except Exception as retry_error:
                    print(f"    ✗ Retry failed: {retry_error}")
                    continue

        # Merge transcriptions
        print(f"\n✓ All chunks transcribed in {total_transcribe_time*1000:.0f}ms total")
        merged_transcription = merge_chunk_transcriptions(chunk_results, CHUNK_OVERLAP)

        print(f"\nMerged Transcription ({len(merged_transcription)} chars):")
        print(f"  {merged_transcription[:200]}..." if len(merged_transcription) > 200 else f"  {merged_transcription}")

        # Calculate metrics
        wer = calculate_wer(reference_text, merged_transcription)
        avg_chunk_latency = sum(r['latency_ms'] for r in chunk_results) / len(chunk_results) if chunk_results else 0
        max_chunk_mem = max(r['mem_mb'] for r in chunk_results) if chunk_results else 0
        rtf = total_transcribe_time / total_duration if total_duration > 0 else 0

        print(f"\n📊 Metrics:")
        print(f"  WER: {wer:.2f}%")
        print(f"  Total Latency: {total_transcribe_time*1000:.0f}ms")
        print(f"  Average Chunk Latency: {avg_chunk_latency:.0f}ms")
        print(f"  RTF: {rtf:.3f}x (1.0 = realtime)")
        print(f"  Chunks Processed: {len(chunk_results)}/{len(chunks)}")
        print(f"  Max Chunk Memory: {max_chunk_mem:.1f} MB")
        print(f"  Model Base Memory: {model_memory:.1f} MB")

        results.append({
            'file': audio_file,
            'wer': wer,
            'total_latency_ms': total_transcribe_time * 1000,
            'avg_chunk_latency_ms': avg_chunk_latency,
            'rtf': rtf,
            'chunks_processed': len(chunk_results),
            'chunks_total': len(chunks),
            'max_chunk_mem_mb': max_chunk_mem,
            'reference': reference_text,
            'transcription': merged_transcription
        })

    # Summary
    print(f"\n{'=' * 80}")
    print("SUMMARY")
    print('=' * 80)

    if results:
        avg_wer = sum(r['wer'] for r in results) / len(results)
        avg_latency = sum(r['total_latency_ms'] for r in results) / len(results)
        avg_rtf = sum(r['rtf'] for r in results) / len(results)
        total_chunks = sum(r['chunks_processed'] for r in results)

        print(f"\n✓ Tested {len(results)} audio files")
        print(f"  Average WER: {avg_wer:.2f}%")
        print(f"  Average Total Latency: {avg_latency:.0f}ms")
        print(f"  Average RTF: {avg_rtf:.3f}x")
        print(f"  Total Chunks Processed: {total_chunks}")

        if avg_wer < 10:
            print(f"\n🎉 Excellent! WER < 10% - Model working perfectly!")
        elif avg_wer < 20:
            print(f"\n✓ Good! WER < 20% - Model working well")
        else:
            print(f"\n⚠ WER > 20% - May need tuning or better chunk merging")

        print(f"\n💡 Memory Optimization Success:")
        print(f"  Chunking strategy allows processing long audio on 4GB VRAM")
        print(f"  Max chunk memory: {max([r['max_chunk_mem_mb'] for r in results]):.1f} MB")
        print(f"  Model base memory: {model_memory:.1f} MB")

    # Cleanup
    temp_chunk_path.unlink(missing_ok=True)

    print("\n" + "=" * 80)

if __name__ == '__main__':
    main()
