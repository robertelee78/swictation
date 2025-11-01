#!/usr/bin/env python3
"""
Test that backspace commands don't get unwanted spaces after them
"""

import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'src'))

def test_trailing_space_logic():
    """Test the trailing space decision logic"""
    print("Testing trailing space logic...")
    import re

    def should_add_space(transformed_text):
        """Returns True if we should add trailing space"""
        # Don't add space if text ends with a key action
        return not re.search(r'<KEY:[^>]+>$', transformed_text)

    tests = [
        # (input, should_add_trailing_space)
        ("Hello, world", True),           # Normal text - add space
        ("<KEY:BackSpace>", False),       # Ends with key - no space
        ("hello <KEY:BackSpace>", False), # Ends with key - no space
        ("hello <KEY:BackSpace> ", True), # Ends with space - add space
        ("test <KEY:Return>", False),     # Ends with enter - no space
        ("<KEY:BackSpace> <KEY:BackSpace>", False), # Multiple keys - no space
    ]

    passed = 0
    failed = 0

    for text, expected in tests:
        result = should_add_space(text)
        if result == expected:
            space_str = "ADD space" if result else "NO space"
            print(f"  ✅ '{text}' → {space_str}")
            passed += 1
        else:
            expected_str = "ADD space" if expected else "NO space"
            got_str = "ADD space" if result else "NO space"
            print(f"  ❌ '{text}' → {got_str} (expected {expected_str})")
            failed += 1

    print(f"\n  Results: {passed} passed, {failed} failed")
    return failed == 0


def test_backspace_scenarios():
    """Test real-world backspace scenarios"""
    print("\nTesting backspace scenarios...")
    import midstreamer_transform
    import re

    def process_text(voice_input):
        """Simulate the full processing pipeline"""
        # Transform voice to text/keys
        transformed = midstreamer_transform.transform(voice_input)

        # Decide on trailing space
        if re.search(r'<KEY:[^>]+>$', transformed):
            final = transformed  # No space
        else:
            final = transformed + ' '  # Add space

        return transformed, final

    tests = [
        ("hello world", "hello world", "hello world "),
        ("backspace", "<KEY:BackSpace>", "<KEY:BackSpace>"),  # No space!
        ("backspace backspace", "<KEY:BackSpace> <KEY:BackSpace>", "<KEY:BackSpace> <KEY:BackSpace>"),  # No space!
        ("test backspace", "test <KEY:BackSpace>", "test <KEY:BackSpace>"),  # No space!
        ("hello comma world", "hello, world", "hello, world "),  # Normal text gets space
    ]

    passed = 0
    failed = 0

    for voice, expected_transform, expected_final in tests:
        transformed, final = process_text(voice)

        if transformed == expected_transform and final == expected_final:
            print(f"  ✅ '{voice}'")
            print(f"      Transform: '{transformed}'")
            print(f"      Final: '{repr(final)}'")
            passed += 1
        else:
            print(f"  ❌ '{voice}'")
            print(f"      Transform: '{transformed}' (expected '{expected_transform}')")
            print(f"      Final: '{repr(final)}' (expected '{repr(expected_final)}')")
            failed += 1

    print(f"\n  Results: {passed} passed, {failed} failed")
    return failed == 0


def main():
    """Run all tests"""
    print("=" * 80)
    print("Backspace Spacing Test Suite")
    print("=" * 80)

    results = []
    results.append(("Trailing space logic", test_trailing_space_logic()))
    results.append(("Backspace scenarios", test_backspace_scenarios()))

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
        print("\nExpected behavior:")
        print("  - 'backspace backspace' → BackSpace, BackSpace (NO extra spaces)")
        print("  - 'hello comma world' → 'hello, world ' (WITH trailing space)")
        return 0
    else:
        print(f"\n❌ {total - passed} test(s) failed")
        return 1


if __name__ == '__main__':
    sys.exit(main())
