#!/usr/bin/env python3
"""
Test streaming deduplication logic for progressive text injection.
Validates that only new words are injected as transcription grows.
"""

import sys
sys.path.insert(0, '/opt/swictation/src')


class MockTextInjector:
    """Mock text injector for testing"""
    def __init__(self):
        self.injected_texts = []

    def inject(self, text: str) -> bool:
        self.injected_texts.append(text)
        return True


class StreamingDeduplicationTester:
    """Simulates streaming transcription with deduplication"""
    def __init__(self):
        self.text_injector = MockTextInjector()
        self._last_injected = ""

    def _inject_streaming_delta(self, new_transcription: str):
        """
        Inject only new words from cumulative transcription.
        Handles progressive text with deduplication.
        """
        if not new_transcription.strip():
            return

        # Check if this is an extension of previous text
        if new_transcription.startswith(self._last_injected):
            # Calculate delta (new portion only)
            delta = new_transcription[len(self._last_injected):]

            if delta.strip():  # Only inject if there's new content
                print(f"  🎤→ {delta.strip()}")
                self.text_injector.inject(delta)
                self._last_injected = new_transcription
        else:
            # Transcription changed (correction/revision)
            print(f"  🔄 Revision detected, injecting full text: {new_transcription.strip()}")
            self.text_injector.inject(new_transcription)
            self._last_injected = new_transcription


def test_progressive_extension():
    """Test normal progressive transcription: Hello → Hello world → Hello world testing"""
    print("\n=== Test 1: Progressive Extension ===")
    tester = StreamingDeduplicationTester()

    # Simulate cumulative transcription
    chunks = [
        "Hello",
        "Hello world",
        "Hello world testing"
    ]

    for chunk in chunks:
        print(f"Chunk: '{chunk}'")
        tester._inject_streaming_delta(chunk)

    # Verify only deltas were injected
    expected = ["Hello", " world", " testing"]
    assert tester.text_injector.injected_texts == expected, \
        f"Expected {expected}, got {tester.text_injector.injected_texts}"
    print("✓ Test passed: Only deltas injected")


def test_empty_delta():
    """Test empty delta (no new words yet)"""
    print("\n=== Test 2: Empty Delta ===")
    tester = StreamingDeduplicationTester()

    chunks = [
        "Hello",
        "Hello",  # Same as before, should skip
        "Hello world"
    ]

    for chunk in chunks:
        print(f"Chunk: '{chunk}'")
        tester._inject_streaming_delta(chunk)

    expected = ["Hello", " world"]
    assert tester.text_injector.injected_texts == expected, \
        f"Expected {expected}, got {tester.text_injector.injected_texts}"
    print("✓ Test passed: Empty delta skipped")


def test_revision():
    """Test correction/revision (transcription changes mid-stream)"""
    print("\n=== Test 3: Revision ===")
    tester = StreamingDeduplicationTester()

    chunks = [
        "Hello world",
        "Hi there"  # Changed, not an extension
    ]

    for chunk in chunks:
        print(f"Chunk: '{chunk}'")
        tester._inject_streaming_delta(chunk)

    expected = ["Hello world", "Hi there"]
    assert tester.text_injector.injected_texts == expected, \
        f"Expected {expected}, got {tester.text_injector.injected_texts}"
    print("✓ Test passed: Revision handled (full text re-injected)")


def test_punctuation_addition():
    """Test punctuation added to existing text"""
    print("\n=== Test 4: Punctuation Addition ===")
    tester = StreamingDeduplicationTester()

    chunks = [
        "Hello world",
        "Hello world."  # Added period
    ]

    for chunk in chunks:
        print(f"Chunk: '{chunk}'")
        tester._inject_streaming_delta(chunk)

    expected = ["Hello world", "."]
    assert tester.text_injector.injected_texts == expected, \
        f"Expected {expected}, got {tester.text_injector.injected_texts}"
    print("✓ Test passed: Punctuation delta injected")


def test_capitalization_change():
    """Test capitalization change (counts as revision)"""
    print("\n=== Test 5: Capitalization Change ===")
    tester = StreamingDeduplicationTester()

    chunks = [
        "hello world",
        "Hello world"  # Changed capitalization, not an extension
    ]

    for chunk in chunks:
        print(f"Chunk: '{chunk}'")
        tester._inject_streaming_delta(chunk)

    # Capitalization change means revision (full text re-injected)
    expected = ["hello world", "Hello world"]
    assert tester.text_injector.injected_texts == expected, \
        f"Expected {expected}, got {tester.text_injector.injected_texts}"
    print("✓ Test passed: Capitalization change treated as revision")


def test_long_sentence():
    """Test long progressive sentence"""
    print("\n=== Test 6: Long Sentence ===")
    tester = StreamingDeduplicationTester()

    chunks = [
        "The",
        "The quick",
        "The quick brown",
        "The quick brown fox",
        "The quick brown fox jumps",
        "The quick brown fox jumps over",
        "The quick brown fox jumps over the",
        "The quick brown fox jumps over the lazy",
        "The quick brown fox jumps over the lazy dog"
    ]

    for chunk in chunks:
        print(f"Chunk: '{chunk}'")
        tester._inject_streaming_delta(chunk)

    expected = [
        "The",
        " quick",
        " brown",
        " fox",
        " jumps",
        " over",
        " the",
        " lazy",
        " dog"
    ]
    assert tester.text_injector.injected_texts == expected, \
        f"Expected {expected}, got {tester.text_injector.injected_texts}"
    print("✓ Test passed: Long progressive sentence handled correctly")


def test_empty_transcription():
    """Test empty transcription (silence)"""
    print("\n=== Test 7: Empty Transcription ===")
    tester = StreamingDeduplicationTester()

    chunks = [
        "",
        "   ",  # Whitespace only
        "Hello"
    ]

    for chunk in chunks:
        print(f"Chunk: '{chunk}' (empty={not chunk.strip()})")
        tester._inject_streaming_delta(chunk)

    expected = ["Hello"]
    assert tester.text_injector.injected_texts == expected, \
        f"Expected {expected}, got {tester.text_injector.injected_texts}"
    print("✓ Test passed: Empty transcriptions skipped")


def run_all_tests():
    """Run all deduplication tests"""
    print("=" * 60)
    print("STREAMING DEDUPLICATION TEST SUITE")
    print("=" * 60)

    tests = [
        test_progressive_extension,
        test_empty_delta,
        test_revision,
        test_punctuation_addition,
        test_capitalization_change,
        test_long_sentence,
        test_empty_transcription
    ]

    passed = 0
    failed = 0

    for test in tests:
        try:
            test()
            passed += 1
        except AssertionError as e:
            print(f"✗ Test failed: {e}")
            failed += 1
        except Exception as e:
            print(f"✗ Test error: {e}")
            failed += 1

    print("\n" + "=" * 60)
    print(f"RESULTS: {passed}/{len(tests)} tests passed")
    if failed > 0:
        print(f"         {failed}/{len(tests)} tests FAILED")
        sys.exit(1)
    else:
        print("         ALL TESTS PASSED ✓")
        sys.exit(0)


if __name__ == '__main__':
    run_all_tests()
