#!/usr/bin/env python3
"""
Integration test for transformer in swictationd.py
Tests that the _safe_transform method works correctly with the PyO3 module.
"""

import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'src'))

def test_imports():
    """Test that all required imports work"""
    print("Testing imports...")
    try:
        # This will trigger the module import and availability check
        import swictationd
        print("✅ swictationd imports successfully")
        return True
    except Exception as e:
        print(f"❌ Import failed: {e}")
        return False

def test_transform_available():
    """Test that transformer is available"""
    print("\nTesting transformer availability...")
    try:
        import midstreamer_transform
        test_result = midstreamer_transform.transform("test comma")
        expected = "test,"
        if test_result == expected:
            print(f"✅ Transformer available and working: '{test_result}'")
            return True
        else:
            print(f"❌ Unexpected result: got '{test_result}', expected '{expected}'")
            return False
    except ImportError:
        print("❌ midstreamer_transform module not installed")
        return False
    except Exception as e:
        print(f"❌ Transform test failed: {e}")
        return False

def test_safe_transform_mock():
    """Test the _safe_transform method logic with mocked daemon"""
    print("\nTesting _safe_transform method...")

    # Mock minimal daemon for testing the method
    class MockDaemon:
        def __init__(self):
            try:
                import midstreamer_transform
                self.transformer_available = True
            except ImportError:
                self.transformer_available = False

            self.transform_stats = {
                'total': 0,
                'changed': 0,
                'errors': 0,
                'latency_sum_us': 0.0,
                'max_latency_us': 0.0
            }

        def _safe_transform(self, text: str) -> str:
            """Copy of the _safe_transform implementation"""
            import time

            if not self.transformer_available:
                return text

            if not text or not text.strip():
                return text

            try:
                import midstreamer_transform
                start = time.perf_counter()
                result = midstreamer_transform.transform(text)
                elapsed_us = (time.perf_counter() - start) * 1_000_000

                self.transform_stats['total'] += 1
                self.transform_stats['latency_sum_us'] += elapsed_us
                self.transform_stats['max_latency_us'] = max(
                    self.transform_stats['max_latency_us'],
                    elapsed_us
                )

                if result != text:
                    self.transform_stats['changed'] += 1

                return result
            except Exception as e:
                self.transform_stats['errors'] += 1
                print(f"  ⚠️  Transform error: {e}")
                return text

    daemon = MockDaemon()

    if not daemon.transformer_available:
        print("⚠️  Transformer not available, skipping tests")
        return True

    # Test cases
    tests = [
        ("hello comma world", "hello, world"),
        ("test period", "test."),
        ("x equals y plus z", "x = y + z"),
        ("", ""),  # Empty string
        ("no changes", "no changes"),  # Text without commands
    ]

    passed = 0
    failed = 0

    for input_text, expected in tests:
        result = daemon._safe_transform(input_text)
        if result == expected:
            print(f"  ✅ '{input_text}' → '{result}'")
            passed += 1
        else:
            print(f"  ❌ '{input_text}' → '{result}' (expected '{expected}')")
            failed += 1

    # Check statistics
    print(f"\n  Statistics after {daemon.transform_stats['total']} transforms:")
    print(f"    Changed: {daemon.transform_stats['changed']}")
    print(f"    Errors: {daemon.transform_stats['errors']}")
    if daemon.transform_stats['total'] > 0:
        avg_us = daemon.transform_stats['latency_sum_us'] / daemon.transform_stats['total']
        print(f"    Avg latency: {avg_us:.2f}µs")
        print(f"    Max latency: {daemon.transform_stats['max_latency_us']:.2f}µs")

        if avg_us < 100:
            print(f"    Performance: ✅ Excellent (<100µs)")
        else:
            print(f"    Performance: ⚠️  Acceptable but above target")

    print(f"\n  Results: {passed} passed, {failed} failed")
    return failed == 0

def main():
    """Run all tests"""
    print("=" * 80)
    print("Swictationd Transformer Integration Tests")
    print("=" * 80)

    results = []

    # Run tests
    results.append(("Imports", test_imports()))
    results.append(("Transformer Available", test_transform_available()))
    results.append(("_safe_transform Method", test_safe_transform_mock()))

    # Print summary
    print("\n" + "=" * 80)
    print("Test Summary")
    print("=" * 80)

    passed = sum(1 for _, result in results if result)
    total = len(results)

    for test_name, result in results:
        status = "✅ PASS" if result else "❌ FAIL"
        print(f"  {status}: {test_name}")

    print(f"\n  Total: {passed}/{total} tests passed")

    if passed == total:
        print("\n✅ All tests passed!")
        return 0
    else:
        print(f"\n❌ {total - passed} test(s) failed")
        return 1

if __name__ == '__main__':
    sys.exit(main())
