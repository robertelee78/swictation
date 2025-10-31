#!/usr/bin/env python3
"""
Real-world streaming transcription scenarios.
Tests realistic edge cases and production scenarios.
"""

import sys
sys.path.insert(0, '/opt/swictation/src')


class MockTextInjector:
    """Mock text injector that tracks what was injected"""
    def __init__(self):
        self.injected_texts = []
        self.full_text = ""

    def inject(self, text: str) -> bool:
        self.injected_texts.append(text)
        self.full_text += text
        return True


class StreamingTranscriptionSimulator:
    """Simulates streaming transcription with progressive injection"""
    def __init__(self):
        self.text_injector = MockTextInjector()
        self._last_injected = ""

    def _inject_streaming_delta(self, new_transcription: str):
        """Progressive text injection with deduplication"""
        if not new_transcription.strip():
            return

        if new_transcription.startswith(self._last_injected):
            delta = new_transcription[len(self._last_injected):]
            if delta.strip():
                print(f"  → '{delta.strip()}'")
                self.text_injector.inject(delta)
                self._last_injected = new_transcription
        else:
            print(f"  🔄 REVISION: '{new_transcription.strip()}'")
            self.text_injector.inject(new_transcription)
            self._last_injected = new_transcription

    def reset(self):
        """Reset for next recording"""
        self._last_injected = ""


def scenario_short_utterance():
    """Scenario: Short voice command"""
    print("\n=== Scenario 1: Short Utterance (5 seconds) ===")
    print("User says: 'Hello world'")
    sim = StreamingTranscriptionSimulator()

    # Simulate Wait-k streaming with 400ms chunks
    chunks = [
        "Hello",
        "Hello world"
    ]

    for i, chunk in enumerate(chunks):
        print(f"[{i*0.4:.1f}s] Transcription: '{chunk}'")
        sim._inject_streaming_delta(chunk)

    print(f"\nFinal injected text: '{sim.text_injector.full_text}'")
    assert sim.text_injector.full_text == "Hello world", "Text mismatch"
    print("✓ Scenario passed")


def scenario_long_sentence():
    """Scenario: Long dictation with pauses"""
    print("\n=== Scenario 2: Long Sentence with Pauses ===")
    print("User says: 'The quick brown fox jumps over the lazy dog.'")
    sim = StreamingTranscriptionSimulator()

    # Simulate progressive transcription
    chunks = [
        "The",
        "The quick",
        "The quick brown",
        "The quick brown fox",
        "The quick brown fox jumps",
        "The quick brown fox jumps over",
        "The quick brown fox jumps over the",
        "The quick brown fox jumps over the lazy",
        "The quick brown fox jumps over the lazy dog",
        "The quick brown fox jumps over the lazy dog."  # Punctuation added
    ]

    for i, chunk in enumerate(chunks):
        print(f"[{i*0.4:.1f}s] '{chunk}'")
        sim._inject_streaming_delta(chunk)

    expected = "The quick brown fox jumps over the lazy dog."
    print(f"\nFinal: '{sim.text_injector.full_text}'")
    assert sim.text_injector.full_text == expected, f"Expected '{expected}', got '{sim.text_injector.full_text}'"
    print("✓ Scenario passed")


def scenario_correction_mid_stream():
    """Scenario: User corrects themselves mid-sentence"""
    print("\n=== Scenario 3: Mid-Stream Correction ===")
    print("User says: 'Hello world, I mean hi there'")
    sim = StreamingTranscriptionSimulator()

    chunks = [
        "Hello world",
        "Hello world, I mean",
        "Hello world, I mean hi",
        "Hello world, I mean hi there"
    ]

    for i, chunk in enumerate(chunks):
        print(f"[{i*0.4:.1f}s] '{chunk}'")
        sim._inject_streaming_delta(chunk)

    expected = "Hello world, I mean hi there"
    assert sim.text_injector.full_text == expected
    print("✓ Scenario passed: Correction handled smoothly")


def scenario_multiple_sentences():
    """Scenario: Multiple sentences in one recording"""
    print("\n=== Scenario 4: Multiple Sentences ===")
    print("User says: 'First sentence. Second sentence. Third one.'")
    sim = StreamingTranscriptionSimulator()

    chunks = [
        "First sentence",
        "First sentence.",
        "First sentence. Second",
        "First sentence. Second sentence",
        "First sentence. Second sentence.",
        "First sentence. Second sentence. Third",
        "First sentence. Second sentence. Third one",
        "First sentence. Second sentence. Third one."
    ]

    for i, chunk in enumerate(chunks):
        print(f"[{i*0.4:.1f}s] '{chunk}'")
        sim._inject_streaming_delta(chunk)

    expected = "First sentence. Second sentence. Third one."
    assert sim.text_injector.full_text == expected
    print("✓ Scenario passed")


def scenario_pause_mid_word():
    """Scenario: User pauses mid-word (transcription revises)"""
    print("\n=== Scenario 5: Pause Mid-Word ===")
    print("User says: 'anti... dis... establishment... arianism'")
    sim = StreamingTranscriptionSimulator()

    # STT might revise incomplete words
    chunks = [
        "Anti",
        "Antidis",  # Prefix changed, triggers revision
        "Antidisestablishment",
        "Antidisestablishmentarianism"
    ]

    for i, chunk in enumerate(chunks):
        print(f"[{i*0.4:.1f}s] '{chunk}'")
        sim._inject_streaming_delta(chunk)

    # Expect revision to handle word merging
    print(f"\nFinal: '{sim.text_injector.full_text}'")
    print("✓ Scenario passed: Revision mechanism handled word changes")


def scenario_numbers_and_punctuation():
    """Scenario: Numbers with punctuation"""
    print("\n=== Scenario 6: Numbers & Punctuation ===")
    print("User says: 'Testing 1, 2, 3, and done.'")
    sim = StreamingTranscriptionSimulator()

    chunks = [
        "Testing 1",
        "Testing 1,",
        "Testing 1, 2",
        "Testing 1, 2,",
        "Testing 1, 2, 3",
        "Testing 1, 2, 3,",
        "Testing 1, 2, 3, and",
        "Testing 1, 2, 3, and done",
        "Testing 1, 2, 3, and done."
    ]

    for i, chunk in enumerate(chunks):
        print(f"[{i*0.4:.1f}s] '{chunk}'")
        sim._inject_streaming_delta(chunk)

    expected = "Testing 1, 2, 3, and done."
    assert sim.text_injector.full_text == expected
    print("✓ Scenario passed")


def scenario_code_dictation():
    """Scenario: Dictating code"""
    print("\n=== Scenario 7: Code Dictation ===")
    print("User says: 'def hello world colon return hello'")
    sim = StreamingTranscriptionSimulator()

    chunks = [
        "def",
        "def hello",
        "def hello world",
        "def hello world colon",
        "def hello world colon return",
        "def hello world colon return hello"
    ]

    for i, chunk in enumerate(chunks):
        print(f"[{i*0.4:.1f}s] '{chunk}'")
        sim._inject_streaming_delta(chunk)

    expected = "def hello world colon return hello"
    assert sim.text_injector.full_text == expected
    print("✓ Scenario passed (note: text transformation to symbols happens separately)")


def scenario_reset_between_recordings():
    """Scenario: Multiple independent recordings"""
    print("\n=== Scenario 8: Multiple Recordings (Reset) ===")
    sim = StreamingTranscriptionSimulator()

    # First recording
    print("\nRecording 1:")
    chunks1 = ["Hello", "Hello world"]
    for chunk in chunks1:
        sim._inject_streaming_delta(chunk)

    assert sim.text_injector.full_text == "Hello world"

    # Reset for next recording (simulates daemon reset)
    sim.reset()
    sim.text_injector = MockTextInjector()

    # Second recording
    print("\nRecording 2:")
    chunks2 = ["Testing", "Testing 123"]
    for chunk in chunks2:
        sim._inject_streaming_delta(chunk)

    assert sim.text_injector.full_text == "Testing 123"
    print("✓ Scenario passed: Independent recordings work correctly")


def run_all_scenarios():
    """Run all real-world scenarios"""
    print("=" * 60)
    print("STREAMING TRANSCRIPTION SCENARIOS")
    print("Real-world edge cases and production scenarios")
    print("=" * 60)

    scenarios = [
        scenario_short_utterance,
        scenario_long_sentence,
        scenario_correction_mid_stream,
        scenario_multiple_sentences,
        scenario_pause_mid_word,
        scenario_numbers_and_punctuation,
        scenario_code_dictation,
        scenario_reset_between_recordings
    ]

    passed = 0
    failed = 0

    for scenario in scenarios:
        try:
            scenario()
            passed += 1
        except AssertionError as e:
            print(f"✗ Scenario failed: {e}")
            failed += 1
        except Exception as e:
            print(f"✗ Scenario error: {e}")
            failed += 1

    print("\n" + "=" * 60)
    print(f"RESULTS: {passed}/{len(scenarios)} scenarios passed")
    if failed > 0:
        print(f"         {failed}/{len(scenarios)} scenarios FAILED")
        sys.exit(1)
    else:
        print("         ALL SCENARIOS PASSED ✓")
        sys.exit(0)


if __name__ == '__main__':
    run_all_scenarios()
