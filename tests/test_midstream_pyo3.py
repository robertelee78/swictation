#!/usr/bin/env python3
"""
Test suite for MidStream PyO3 Python bindings.

Tests the direct Rust->Python FFI integration for text transformation.
Target: <1ms overhead (actual: ~0.3μs)
"""

import pytest
import midstreamer_transform as mt


def test_import():
    """Test that the module imports successfully."""
    assert hasattr(mt, 'transform')
    assert hasattr(mt, 'get_stats')
    assert hasattr(mt, 'benchmark')


def test_basic_punctuation():
    """Test basic punctuation transformations."""
    assert mt.transform("Hello comma world period") == "Hello, world."
    assert mt.transform("Stop period") == "Stop."
    assert mt.transform("Really question mark") == "Really?"
    assert mt.transform("Wow exclamation point") == "Wow!"


def test_operators():
    """Test operator transformations."""
    assert mt.transform("x equals y") == "x = y"
    assert mt.transform("a plus b") == "a + b"
    assert mt.transform("c minus d") == "c - d"
    assert mt.transform("e asterisk f") == "e * f"


def test_brackets():
    """Test bracket transformations."""
    assert mt.transform("open paren x close paren") == "(x)"
    assert mt.transform("open bracket a close bracket") == "[a]"
    assert mt.transform("open brace b close brace") == "{b}"


def test_programming_symbols():
    """Test programming-specific symbols."""
    assert mt.transform("git commit hyphen m") == "git commit -m"
    assert mt.transform("if x double equals y") == "if x == y"
    assert mt.transform("not equals") == "!="


def test_quotes():
    """Test quote transformations."""
    assert mt.transform("quote Hello world quote") == '"Hello world"'
    assert mt.transform("single quote test single quote") == "'test'"


def test_empty_input_error():
    """Test that empty input raises ValueError."""
    with pytest.raises(ValueError, match="Input text cannot be empty"):
        mt.transform("")


def test_get_stats():
    """Test statistics function."""
    count, msg = mt.get_stats()
    assert isinstance(count, int)
    assert count > 0
    assert isinstance(msg, str)
    assert "rules loaded" in msg.lower()


def test_benchmark():
    """Test benchmark function and verify performance target."""
    avg_us, result = mt.benchmark("Hello comma world period", 1000)

    # Verify result is correct
    assert result == "Hello, world."

    # Verify performance: should be well under 1000μs (1ms)
    assert avg_us < 1000, f"Performance target missed: {avg_us:.2f}μs (target: <1000μs)"

    # Typically should be under 10μs
    print(f"Performance: {avg_us:.2f}μs per call")


def test_benchmark_custom_iterations():
    """Test benchmark with custom iteration count."""
    avg_us, result = mt.benchmark("test comma test", 100)
    assert result == "test, test"
    assert avg_us > 0


def test_complex_sentence():
    """Test complex sentence transformation."""
    input_text = "git commit hyphen m quote fix bug quote"
    expected = 'git commit -m "fix bug"'
    assert mt.transform(input_text) == expected


def test_multi_word_patterns():
    """Test multi-word pattern transformations."""
    assert mt.transform("What question mark") == "What?"
    assert mt.transform("Price dollar sign") == "Price $"
    assert mt.transform("Hash hashtag trending") == "# trending"


def test_performance_comparison():
    """Compare PyO3 performance against target."""
    # Run benchmark
    avg_us, _ = mt.benchmark("Hello comma world period", 5000)

    print(f"\n{'='*60}")
    print("PERFORMANCE COMPARISON")
    print(f"{'='*60}")
    print(f"PyO3 FFI:          {avg_us:>8.2f}μs per call")
    print(f"Target:            {1000:>8.2f}μs per call")
    print(f"Node.js subprocess: ~{75000:>8.2f}μs per call (estimated)")
    print(f"{'='*60}")
    print(f"PyO3 is {1000/avg_us:.0f}x faster than 1ms target")
    print(f"PyO3 is ~{75000/avg_us:.0f}x faster than subprocess approach")
    print(f"{'='*60}\n")


if __name__ == "__main__":
    # Run tests
    pytest.main([__file__, "-v", "-s"])
