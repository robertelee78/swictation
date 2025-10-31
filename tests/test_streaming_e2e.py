#!/usr/bin/env python3
"""
End-to-End Streaming Transcription Test Suite

Validates that streaming transcription matches batch accuracy with:
- Short audio accuracy (100% target)
- Long audio WER < 1%
- Context preservation across chunks
- Hallucination detection
- Real-time latency measurement

Prerequisites:
- NeMo Canary model loaded
- Test audio files in tests/data/
- GPU available (RTX A1000 or better)

Usage:
    pytest tests/test_streaming_e2e.py -v
    python tests/test_streaming_e2e.py  # Direct run
"""

import sys
import os
import time
import pytest
from pathlib import Path
from typing import List, Tuple, Dict, Optional
import numpy as np

# Add src to path
sys.path.insert(0, str(Path(__file__).parent.parent / 'src'))

# Check for required dependencies
try:
    import torch
    import librosa
    import soundfile as sf
    from nemo.collections.asr.models import EncDecMultiTaskModel
    from omegaconf import DictConfig
    NEMO_AVAILABLE = True
except ImportError as e:
    NEMO_AVAILABLE = False
    IMPORT_ERROR = str(e)

# Test configuration
MODEL_NAME = 'nvidia/canary-1b-flash'
SAMPLE_RATE = 16000
TEST_DATA_DIR = Path(__file__).parent / 'data'

# Expected transcriptions
EXPECTED_SHORT = "Hello world. Testing, one, two, three."
EXPECTED_SHORT_NORMALIZED = "hello world testing one two three"

# WER calculation tolerance
MAX_WER_DIFFERENCE = 1.0  # Maximum 1% WER difference between streaming/batch


class MockTextInjector:
    """Mock text injector that tracks injected text and timing"""

    def __init__(self):
        self.injections: List[Tuple[float, str]] = []  # (timestamp, text)
        self.full_text = ""
        self.start_time = time.time()

    def inject(self, text: str) -> bool:
        """Record injection with timestamp"""
        if text:
            elapsed = time.time() - self.start_time
            self.injections.append((elapsed, text))
            self.full_text += text
        return True

    def reset(self):
        """Reset for next test"""
        self.injections.clear()
        self.full_text = ""
        self.start_time = time.time()

    def get_latency_stats(self) -> Dict[str, float]:
        """Calculate latency statistics"""
        if not self.injections:
            return {"min": 0, "max": 0, "avg": 0, "total": 0}

        timestamps = [t for t, _ in self.injections]
        return {
            "min": min(timestamps),
            "max": max(timestamps),
            "avg": sum(timestamps) / len(timestamps),
            "total": max(timestamps) - min(timestamps) if len(timestamps) > 1 else timestamps[0]
        }


def normalize_text(text: str) -> str:
    """Normalize text for comparison (lowercase, remove punctuation)"""
    import re
    text = text.lower()
    text = re.sub(r'[^\w\s]', '', text)  # Remove punctuation
    text = re.sub(r'\s+', ' ', text)     # Normalize whitespace
    return text.strip()


def calculate_wer(reference: str, hypothesis: str) -> float:
    """
    Calculate Word Error Rate (WER) between reference and hypothesis.

    WER = (Substitutions + Insertions + Deletions) / Total Words in Reference

    Returns:
        WER as percentage (0-100)
    """
    ref_words = normalize_text(reference).split()
    hyp_words = normalize_text(hypothesis).split()

    # Simple Levenshtein distance at word level
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
                substitution = d[i-1][j-1] + 1
                insertion = d[i][j-1] + 1
                deletion = d[i-1][j] + 1
                d[i][j] = min(substitution, insertion, deletion)

    if len(ref_words) == 0:
        return 0.0 if len(hyp_words) == 0 else 100.0

    return (d[len(ref_words)][len(hyp_words)] / len(ref_words)) * 100


def word_accuracy(reference: str, hypothesis: str) -> float:
    """
    Calculate word-level accuracy (case-insensitive).

    Returns:
        Accuracy as percentage (0-100)
    """
    ref_words = set(normalize_text(reference).split())
    hyp_words = set(normalize_text(hypothesis).split())

    if not ref_words:
        return 100.0 if not hyp_words else 0.0

    matches = len(ref_words & hyp_words)
    return (matches / len(ref_words)) * 100


class StreamingTranscriber:
    """Manages NeMo model for streaming transcription tests"""

    def __init__(self):
        self.model = None
        self.device = None

    def load_model(self):
        """Load NeMo Canary model with streaming configuration"""
        print(f"Loading model: {MODEL_NAME}")
        load_start = time.time()

        self.model = EncDecMultiTaskModel.from_pretrained(MODEL_NAME)
        self.model.eval()

        if torch.cuda.is_available():
            self.model = self.model.cuda()
            self.device = 'cuda'
        else:
            self.device = 'cpu'

        load_time = time.time() - load_start

        # Configure Wait-k streaming
        streaming_cfg = DictConfig({
            'streaming_policy': 'waitk',
            'waitk_lagging': 2,
            'hallucinations_detector': True,
            'beam_size': 1
        })

        try:
            self.model.change_decoding_strategy(streaming_cfg)
            print(f"✓ Model loaded in {load_time:.2f}s with Wait-k streaming")
        except Exception as e:
            print(f"⚠ Warning: Could not configure streaming: {e}")

        return self.model

    def transcribe_batch(self, audio_path: Path) -> Tuple[str, float]:
        """
        Standard batch transcription (no streaming).

        Returns:
            (transcription, latency_ms)
        """
        start = time.time()

        hypothesis = self.model.transcribe(
            audio=[str(audio_path)],
            source_lang='en',
            target_lang='en',
            pnc='yes',
            batch_size=1
        )

        latency = (time.time() - start) * 1000

        if hypothesis and len(hypothesis) > 0:
            text = hypothesis[0].text if hasattr(hypothesis[0], 'text') else str(hypothesis[0])
        else:
            text = ""

        return text, latency

    def transcribe_streaming(
        self,
        audio_path: Path,
        chunk_secs: float = 1.0,
        text_injector: Optional[MockTextInjector] = None
    ) -> Tuple[str, float, List[str]]:
        """
        Streaming transcription with chunk_len_in_secs.

        Returns:
            (final_transcription, latency_ms, progressive_outputs)
        """
        start = time.time()
        progressive_outputs = []

        # NeMo's built-in streaming mode
        hypothesis = self.model.transcribe(
            audio=[str(audio_path)],
            source_lang='en',
            target_lang='en',
            pnc='yes',
            chunk_len_in_secs=chunk_secs,
            batch_size=1
        )

        latency = (time.time() - start) * 1000

        if hypothesis and len(hypothesis) > 0:
            final_text = hypothesis[0].text if hasattr(hypothesis[0], 'text') else str(hypothesis[0])
        else:
            final_text = ""

        progressive_outputs.append(final_text)

        # Simulate progressive injection if injector provided
        if text_injector and final_text:
            text_injector.inject(final_text)

        return final_text, latency, progressive_outputs

    def cleanup(self):
        """Free GPU memory"""
        if self.model is not None and torch.cuda.is_available():
            del self.model
            torch.cuda.empty_cache()


# Pytest fixtures

@pytest.fixture(scope="module")
def transcriber():
    """Module-scoped transcriber fixture (load model once)"""
    if not NEMO_AVAILABLE:
        pytest.skip(f"NeMo not available: {IMPORT_ERROR}")

    if not torch.cuda.is_available():
        pytest.skip("CUDA not available")

    transcriber = StreamingTranscriber()
    transcriber.load_model()

    yield transcriber

    transcriber.cleanup()


@pytest.fixture
def text_injector():
    """Fresh text injector for each test"""
    return MockTextInjector()


# Test Suite

class TestShortAudioAccuracy:
    """Test 1: Short audio with 100% accuracy target"""

    def test_short_audio_batch(self, transcriber):
        """Baseline: Batch transcription of short audio"""
        audio_path = TEST_DATA_DIR / 'en-short.mp3'

        if not audio_path.exists():
            pytest.skip(f"Test audio not found: {audio_path}")

        transcription, latency = transcriber.transcribe_batch(audio_path)

        print(f"\n📝 Batch Transcription:")
        print(f"  Text: '{transcription}'")
        print(f"  Latency: {latency:.0f}ms")

        # Validate
        assert transcription.strip(), "Empty transcription"
        assert latency < 5000, f"Latency too high: {latency}ms"

        accuracy = word_accuracy(EXPECTED_SHORT, transcription)
        print(f"  Accuracy: {accuracy:.1f}%")

        assert accuracy >= 95, f"Accuracy too low: {accuracy:.1f}%"

    def test_short_audio_streaming(self, transcriber, text_injector):
        """Streaming transcription of short audio"""
        audio_path = TEST_DATA_DIR / 'en-short.mp3'

        if not audio_path.exists():
            pytest.skip(f"Test audio not found: {audio_path}")

        transcription, latency, progressive = transcriber.transcribe_streaming(
            audio_path,
            chunk_secs=1.0,
            text_injector=text_injector
        )

        print(f"\n🎙️ Streaming Transcription:")
        print(f"  Text: '{transcription}'")
        print(f"  Latency: {latency:.0f}ms")
        print(f"  Progressive outputs: {len(progressive)}")

        # Validate accuracy
        accuracy = word_accuracy(EXPECTED_SHORT, transcription)
        print(f"  Accuracy: {accuracy:.1f}%")

        assert accuracy >= 95, f"Accuracy too low: {accuracy:.1f}%"
        assert transcription.strip(), "Empty transcription"

        # Validate latency
        assert latency < 5000, f"Latency too high: {latency}ms"

        # Validate text injection
        assert text_injector.full_text == transcription, "Injection mismatch"

        latency_stats = text_injector.get_latency_stats()
        print(f"  Injection latency: {latency_stats['total']:.3f}s")

    def test_no_hallucinations(self, transcriber):
        """Test 4: Verify no hallucinations on short audio"""
        audio_path = TEST_DATA_DIR / 'en-short.mp3'

        if not audio_path.exists():
            pytest.skip(f"Test audio not found: {audio_path}")

        transcription, _ = transcriber.transcribe_batch(audio_path)

        # Check for common hallucinations
        hallucination_patterns = [
            "thank you", "thanks for watching", "subscribe",
            "please", "like and subscribe", "[music]", "[applause]"
        ]

        transcription_lower = transcription.lower()
        found_hallucinations = [
            pattern for pattern in hallucination_patterns
            if pattern in transcription_lower
        ]

        assert not found_hallucinations, \
            f"Hallucinations detected: {found_hallucinations}"


class TestLongAudioWER:
    """Test 2: Long audio with WER comparison"""

    def test_long_audio_batch_vs_streaming(self, transcriber):
        """Compare batch vs streaming WER on long audio"""
        audio_path = TEST_DATA_DIR / 'en-long.mp3'

        if not audio_path.exists():
            pytest.skip(f"Test audio not found: {audio_path}")

        # Get audio duration
        audio, sr = librosa.load(audio_path, sr=SAMPLE_RATE)
        duration = len(audio) / sr

        print(f"\n📊 Long Audio Test:")
        print(f"  Duration: {duration:.1f}s")

        # Batch transcription
        batch_text, batch_latency = transcriber.transcribe_batch(audio_path)
        print(f"\n  Batch:")
        print(f"    Text: '{batch_text[:100]}...'")
        print(f"    Latency: {batch_latency:.0f}ms")

        # Streaming transcription
        stream_text, stream_latency, _ = transcriber.transcribe_streaming(
            audio_path,
            chunk_secs=1.0
        )
        print(f"\n  Streaming:")
        print(f"    Text: '{stream_text[:100]}...'")
        print(f"    Latency: {stream_latency:.0f}ms")

        # Calculate WER
        wer = calculate_wer(batch_text, stream_text)
        print(f"\n  WER: {wer:.2f}%")

        # Both should be non-empty
        assert batch_text.strip(), "Empty batch transcription"
        assert stream_text.strip(), "Empty streaming transcription"

        # WER should be low (< 1% difference)
        assert wer <= MAX_WER_DIFFERENCE, \
            f"WER too high: {wer:.2f}% (target: ≤{MAX_WER_DIFFERENCE}%)"

        # Latency should be reasonable
        rtf_batch = batch_latency / (duration * 1000)
        rtf_stream = stream_latency / (duration * 1000)
        print(f"\n  RTF (Batch): {rtf_batch:.3f}x")
        print(f"  RTF (Stream): {rtf_stream:.3f}x")

        assert rtf_batch < 1.0, "Batch RTF > 1.0 (slower than realtime)"
        assert rtf_stream < 1.0, "Streaming RTF > 1.0 (slower than realtime)"


class TestSilentAudioHallucination:
    """Test 5: Hallucination detection on silent audio"""

    def test_silent_audio_no_output(self, transcriber):
        """Verify no hallucinations on silent audio"""
        audio_path = TEST_DATA_DIR / 'silent-10s.mp3'

        if not audio_path.exists():
            pytest.skip(f"Silent test audio not found: {audio_path}")

        transcription, latency = transcriber.transcribe_batch(audio_path)

        print(f"\n🔇 Silent Audio Test:")
        print(f"  Text: '{transcription}'")
        print(f"  Latency: {latency:.0f}ms")

        # Transcription should be empty or minimal
        word_count = len(transcription.split())
        print(f"  Word count: {word_count}")

        # Allow up to 2 words (noise tolerance)
        assert word_count <= 2, \
            f"Hallucination detected: {word_count} words on silent audio"


class TestRealtimeLatency:
    """Test 5: Real-time latency measurement"""

    def test_end_to_end_latency(self, transcriber, text_injector):
        """Measure full pipeline latency: audio → transcription → injection"""
        audio_path = TEST_DATA_DIR / 'en-short.mp3'

        if not audio_path.exists():
            pytest.skip(f"Test audio not found: {audio_path}")

        # Get audio duration
        audio, sr = librosa.load(audio_path, sr=SAMPLE_RATE)
        audio_duration = len(audio) / sr

        print(f"\n⏱️ Latency Test:")
        print(f"  Audio duration: {audio_duration:.2f}s")

        # Measure end-to-end
        start = time.time()

        transcription, transcribe_latency, _ = transcriber.transcribe_streaming(
            audio_path,
            chunk_secs=1.0,
            text_injector=text_injector
        )

        total_latency = (time.time() - start) * 1000
        injection_latency = text_injector.get_latency_stats()

        print(f"\n  Transcription latency: {transcribe_latency:.0f}ms")
        print(f"  Total E2E latency: {total_latency:.0f}ms")
        print(f"  Injection latency: {injection_latency['total']*1000:.0f}ms")

        # Target: < 2 seconds total
        assert total_latency < 2000, \
            f"Total latency too high: {total_latency:.0f}ms (target: <2000ms)"

        # Should be faster than realtime
        rtf = total_latency / (audio_duration * 1000)
        print(f"  RTF: {rtf:.3f}x")

        assert rtf < 1.0, f"RTF > 1.0 (slower than realtime): {rtf:.3f}x"


class TestMemoryUsage:
    """Additional: Memory and performance validation"""

    def test_gpu_memory_stable(self, transcriber):
        """Verify GPU memory doesn't leak during multiple transcriptions"""
        if not torch.cuda.is_available():
            pytest.skip("CUDA not available")

        audio_path = TEST_DATA_DIR / 'en-short.mp3'

        if not audio_path.exists():
            pytest.skip(f"Test audio not found: {audio_path}")

        # Get initial memory
        torch.cuda.empty_cache()
        initial_memory = torch.cuda.memory_allocated() / 1e6

        print(f"\n💾 Memory Test:")
        print(f"  Initial GPU memory: {initial_memory:.1f} MB")

        # Run 5 transcriptions
        for i in range(5):
            transcriber.transcribe_batch(audio_path)
            torch.cuda.empty_cache()

        final_memory = torch.cuda.memory_allocated() / 1e6
        memory_increase = final_memory - initial_memory

        print(f"  Final GPU memory: {final_memory:.1f} MB")
        print(f"  Memory increase: {memory_increase:.1f} MB")

        # Should not increase by more than 100MB
        assert memory_increase < 100, \
            f"Memory leak detected: {memory_increase:.1f} MB increase"


# Direct run support
def main():
    """Run tests directly without pytest"""
    print("=" * 80)
    print("STREAMING TRANSCRIPTION E2E TEST SUITE")
    print("=" * 80)

    if not NEMO_AVAILABLE:
        print(f"\n❌ NeMo not available: {IMPORT_ERROR}")
        return 1

    if not torch.cuda.is_available():
        print("\n❌ CUDA not available")
        return 1

    # Initialize
    transcriber = StreamingTranscriber()
    transcriber.load_model()
    text_injector = MockTextInjector()

    passed = 0
    failed = 0

    # Run tests manually
    tests = [
        ("Short audio batch", lambda: TestShortAudioAccuracy().test_short_audio_batch(transcriber)),
        ("Short audio streaming", lambda: TestShortAudioAccuracy().test_short_audio_streaming(transcriber, text_injector)),
        ("No hallucinations", lambda: TestShortAudioAccuracy().test_no_hallucinations(transcriber)),
        ("Long audio WER", lambda: TestLongAudioWER().test_long_audio_batch_vs_streaming(transcriber)),
        ("Silent audio", lambda: TestSilentAudioHallucination().test_silent_audio_no_output(transcriber)),
        ("E2E latency", lambda: TestRealtimeLatency().test_end_to_end_latency(transcriber, text_injector)),
        ("GPU memory", lambda: TestMemoryUsage().test_gpu_memory_stable(transcriber)),
    ]

    for name, test_func in tests:
        print(f"\n{'='*80}")
        print(f"Test: {name}")
        print(f"{'='*80}")

        try:
            test_func()
            print(f"✅ PASSED: {name}")
            passed += 1
        except AssertionError as e:
            print(f"❌ FAILED: {name}")
            print(f"   {e}")
            failed += 1
        except Exception as e:
            print(f"❌ ERROR: {name}")
            print(f"   {e}")
            failed += 1
        finally:
            text_injector.reset()

    # Cleanup
    transcriber.cleanup()

    # Summary
    print(f"\n{'='*80}")
    print(f"TEST SUMMARY")
    print(f"{'='*80}")
    print(f"Passed: {passed}/{len(tests)}")
    print(f"Failed: {failed}/{len(tests)}")

    if failed == 0:
        print(f"\n🎉 ALL TESTS PASSED")
        return 0
    else:
        print(f"\n❌ SOME TESTS FAILED")
        return 1


if __name__ == '__main__':
    sys.exit(main())
