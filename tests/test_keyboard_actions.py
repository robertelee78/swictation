#!/usr/bin/env python3
"""
Test keyboard actions like backspace, enter, delete
"""

import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'src'))

def test_transformer_keyboard_markers():
    """Test that transformer generates correct key markers"""
    print("Testing transformer keyboard markers...")
    try:
        import midstreamer_transform

        tests = [
            ("backspace", "<KEY:BackSpace>"),
            ("hello backspace world", "hello <KEY:BackSpace> world"),
            ("delete", "<KEY:Delete>"),
            ("enter", "<KEY:Return>"),
            ("return", "<KEY:Return>"),
            ("escape", "<KEY:Escape>"),
            ("tab key", "<KEY:Tab>"),
            ("new line", "\n"),
        ]

        passed = 0
        failed = 0

        for input_text, expected in tests:
            result = midstreamer_transform.transform(input_text)
            if result == expected:
                print(f"  ✅ '{input_text}' → '{repr(result)}'")
                passed += 1
            else:
                print(f"  ❌ '{input_text}' → '{repr(result)}' (expected '{repr(expected)}')")
                failed += 1

        print(f"\n  Results: {passed} passed, {failed} failed")
        return failed == 0

    except Exception as e:
        print(f"  ❌ Error: {e}")
        return False


def test_inject_with_keys_parsing():
    """Test the _inject_text_with_keys parsing logic"""
    print("\nTesting key injection parsing logic...")
    import re

    key_pattern = re.compile(r'<KEY:([^>]+)>')

    def parse_text(text):
        """Simplified version of _inject_text_with_keys parsing"""
        parts = []
        last_end = 0

        for match in key_pattern.finditer(text):
            if match.start() > last_end:
                text_part = text[last_end:match.start()]
                if text_part:
                    parts.append(('text', text_part))

            key_name = match.group(1)
            parts.append(('key', key_name))
            last_end = match.end()

        if last_end < len(text):
            remaining = text[last_end:]
            if remaining:
                parts.append(('text', remaining))

        return parts

    tests = [
        ("Hello, world", [('text', 'Hello, world')]),
        ("<KEY:BackSpace>", [('key', 'BackSpace')]),
        ("hello <KEY:BackSpace> world", [('text', 'hello '), ('key', 'BackSpace'), ('text', ' world')]),
        ("git commit<KEY:Return>", [('text', 'git commit'), ('key', 'Return')]),
    ]

    passed = 0
    failed = 0

    for input_text, expected in tests:
        result = parse_text(input_text)
        if result == expected:
            print(f"  ✅ '{input_text}' → {result}")
            passed += 1
        else:
            print(f"  ❌ '{input_text}' → {result}")
            print(f"      Expected: {expected}")
            failed += 1

    print(f"\n  Results: {passed} passed, {failed} failed")
    return failed == 0


def main():
    """Run all tests"""
    print("=" * 80)
    print("Keyboard Actions Test Suite")
    print("=" * 80)

    results = []
    results.append(("Transformer markers", test_transformer_keyboard_markers()))
    results.append(("Parsing logic", test_inject_with_keys_parsing()))

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
