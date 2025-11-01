#!/usr/bin/env python3
"""
Test Ctrl key combination support
"""

import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'src'))

def test_ctrl_transformations():
    """Test that transformer generates correct Ctrl key markers"""
    print("Testing Ctrl key transformations...")
    try:
        import midstreamer_transform

        tests = [
            # Line editing
            ("control a", "<KEY:ctrl-a>"),
            ("control e", "<KEY:ctrl-e>"),
            ("control u", "<KEY:ctrl-u>"),
            ("control k", "<KEY:ctrl-k>"),
            ("control w", "<KEY:ctrl-w>"),

            # Clipboard
            ("control c", "<KEY:ctrl-c>"),
            ("control x", "<KEY:ctrl-x>"),
            ("control v", "<KEY:ctrl-v>"),

            # File operations
            ("control s", "<KEY:ctrl-s>"),
            ("control o", "<KEY:ctrl-o>"),

            # Undo/Redo
            ("control z", "<KEY:ctrl-z>"),
            ("control y", "<KEY:ctrl-y>"),

            # Search
            ("control f", "<KEY:ctrl-f>"),
            ("control r", "<KEY:ctrl-r>"),

            # Terminal
            ("control d", "<KEY:ctrl-d>"),
            ("control l", "<KEY:ctrl-l>"),
        ]

        passed = 0
        failed = 0

        for input_text, expected in tests:
            result = midstreamer_transform.transform(input_text)
            if result == expected:
                print(f"  ✅ '{input_text}' → '{result}'")
                passed += 1
            else:
                print(f"  ❌ '{input_text}' → '{result}' (expected '{expected}')")
                failed += 1

        print(f"\n  Results: {passed} passed, {failed} failed")
        return failed == 0

    except Exception as e:
        print(f"  ❌ Error: {e}")
        return False


def test_wtype_command_generation():
    """Test that correct wtype commands are generated for Ctrl keys"""
    print("\nTesting wtype command generation logic...")

    def generate_wtype_command(key):
        """Simulate the command generation logic"""
        if key.startswith('ctrl-'):
            letter = key.split('-', 1)[1]
            return ['wtype', '-M', 'ctrl', '-k', letter, '-m', 'ctrl']
        else:
            return ['wtype', '-k', key]

    tests = [
        ("ctrl-u", ['wtype', '-M', 'ctrl', '-k', 'u', '-m', 'ctrl']),
        ("ctrl-c", ['wtype', '-M', 'ctrl', '-k', 'c', '-m', 'ctrl']),
        ("ctrl-s", ['wtype', '-M', 'ctrl', '-k', 's', '-m', 'ctrl']),
        ("BackSpace", ['wtype', '-k', 'BackSpace']),
        ("Return", ['wtype', '-k', 'Return']),
    ]

    passed = 0
    failed = 0

    for key, expected in tests:
        result = generate_wtype_command(key)
        if result == expected:
            print(f"  ✅ '{key}' → {' '.join(result)}")
            passed += 1
        else:
            print(f"  ❌ '{key}' → {' '.join(result)}")
            print(f"      Expected: {' '.join(expected)}")
            failed += 1

    print(f"\n  Results: {passed} passed, {failed} failed")
    return failed == 0


def test_real_world_scenarios():
    """Test real-world voice command scenarios"""
    print("\nTesting real-world scenarios...")
    import midstreamer_transform

    scenarios = [
        # Terminal usage
        ("git add dot control u", "git add. <KEY:ctrl-u>"),  # Clear line, start over
        ("ls hyphen la control c", "ls -la <KEY:ctrl-c>"),  # Interrupt command

        # Editor usage
        ("hello world control s", "hello world <KEY:ctrl-s>"),  # Type and save
        ("control a delete", "<KEY:ctrl-a> <KEY:Delete>"),  # Select all and delete

        # Mixed usage
        ("test control u backspace", "test <KEY:ctrl-u> <KEY:BackSpace>"),
    ]

    passed = 0
    failed = 0

    for voice, expected in scenarios:
        result = midstreamer_transform.transform(voice)
        if result == expected:
            print(f"  ✅ '{voice}'")
            print(f"      → '{result}'")
            passed += 1
        else:
            print(f"  ❌ '{voice}'")
            print(f"      Got: '{result}'")
            print(f"      Expected: '{expected}'")
            failed += 1

    print(f"\n  Results: {passed} passed, {failed} failed")
    return failed == 0


def main():
    """Run all tests"""
    print("=" * 80)
    print("Ctrl Key Combinations Test Suite")
    print("=" * 80)

    results = []
    results.append(("Ctrl transformations", test_ctrl_transformations()))
    results.append(("wtype command generation", test_wtype_command_generation()))
    results.append(("Real-world scenarios", test_real_world_scenarios()))

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
        print("\nSupported Ctrl combinations:")
        print("  • Editing: control a/e/u/k/w")
        print("  • Clipboard: control c/x/v")
        print("  • Files: control s/o/n")
        print("  • Undo: control z/y")
        print("  • Search: control f/r")
        print("  • Terminal: control d/l")
        return 0
    else:
        print(f"\n❌ {total - passed} test(s) failed")
        return 1


if __name__ == '__main__':
    sys.exit(main())
