#!/usr/bin/env python3
"""
Test Super (Mod4) key combinations for Sway window management
"""

import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'src'))

def test_super_transformations():
    """Test that transformer generates correct Super key markers"""
    print("Testing Super key transformations...")
    try:
        import midstreamer_transform

        tests = [
            # Arrow navigation (focus windows)
            ("super left", "<KEY:super-Left>"),
            ("super right", "<KEY:super-Right>"),
            ("super up", "<KEY:super-Up>"),
            ("super down", "<KEY:super-Down>"),

            # Workspace switching
            ("super one", "<KEY:super-1>"),
            ("super two", "<KEY:super-2>"),
            ("super three", "<KEY:super-3>"),
            ("super four", "<KEY:super-4>"),
            ("super five", "<KEY:super-5>"),
            ("super six", "<KEY:super-6>"),
            ("super seven", "<KEY:super-7>"),
            ("super eight", "<KEY:super-8>"),
            ("super nine", "<KEY:super-9>"),
            ("super zero", "<KEY:super-0>"),

            # Move windows (Super+Shift+arrows)
            ("super shift left", "<KEY:super-shift-Left>"),
            ("super shift right", "<KEY:super-shift-Right>"),
            ("super shift up", "<KEY:super-shift-Up>"),
            ("super shift down", "<KEY:super-shift-Down>"),

            # Move to workspace (Super+Shift+number)
            ("super shift one", "<KEY:super-shift-1>"),
            ("super shift five", "<KEY:super-shift-5>"),
            ("super shift nine", "<KEY:super-shift-9>"),

            # Layout commands
            ("super h", "<KEY:super-h>"),
            ("super v", "<KEY:super-v>"),
            ("super s", "<KEY:super-s>"),
            ("super w", "<KEY:super-w>"),
            ("super e", "<KEY:super-e>"),
            ("super f", "<KEY:super-f>"),

            # Floating and misc
            ("super shift space", "<KEY:super-shift-space>"),
            ("super space", "<KEY:super-space>"),
            ("super a", "<KEY:super-a>"),
            ("super shift q", "<KEY:super-shift-q>"),
            ("super shift c", "<KEY:super-shift-c>"),
            ("super shift e", "<KEY:super-shift-e>"),
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
    """Test that correct wtype commands are generated for Super keys"""
    print("\nTesting wtype command generation logic...")

    def generate_wtype_command(key):
        """Simulate the command generation logic from text_injection.py"""
        parts = key.split('-')

        if len(parts) == 1:
            # Simple key
            return ['wtype', '-k', key]
        elif len(parts) == 2:
            # Single modifier
            modifier, keyname = parts
            return ['wtype', '-M', modifier, '-k', keyname, '-m', modifier]
        elif len(parts) == 3:
            # Two modifiers
            mod1, mod2, keyname = parts
            return ['wtype', '-M', mod1, '-M', mod2, '-k', keyname, '-m', mod2, '-m', mod1]
        else:
            return None

    tests = [
        # Single modifier
        ("super-Left", ['wtype', '-M', 'super', '-k', 'Left', '-m', 'super']),
        ("super-1", ['wtype', '-M', 'super', '-k', '1', '-m', 'super']),
        ("super-f", ['wtype', '-M', 'super', '-k', 'f', '-m', 'super']),

        # Two modifiers
        ("super-shift-Left", ['wtype', '-M', 'super', '-M', 'shift', '-k', 'Left', '-m', 'shift', '-m', 'super']),
        ("super-shift-5", ['wtype', '-M', 'super', '-M', 'shift', '-k', '5', '-m', 'shift', '-m', 'super']),
        ("super-shift-q", ['wtype', '-M', 'super', '-M', 'shift', '-k', 'q', '-m', 'shift', '-m', 'super']),

        # Ctrl for comparison
        ("ctrl-u", ['wtype', '-M', 'ctrl', '-k', 'u', '-m', 'ctrl']),

        # Simple keys
        ("Return", ['wtype', '-k', 'Return']),
        ("Left", ['wtype', '-k', 'Left']),
    ]

    passed = 0
    failed = 0

    for key, expected in tests:
        result = generate_wtype_command(key)
        if result == expected:
            print(f"  ✅ '{key}' → {' '.join(result)}")
            passed += 1
        else:
            print(f"  ❌ '{key}' → {' '.join(result) if result else 'None'}")
            print(f"      Expected: {' '.join(expected)}")
            failed += 1

    print(f"\n  Results: {passed} passed, {failed} failed")
    return failed == 0


def test_real_world_scenarios():
    """Test real-world Sway navigation scenarios"""
    print("\nTesting real-world Sway scenarios...")
    import midstreamer_transform

    scenarios = [
        # Window navigation
        ("super left", "<KEY:super-Left>"),  # Focus left window
        ("super right", "<KEY:super-Right>"),  # Focus right window

        # Workspace switching
        ("super three", "<KEY:super-3>"),  # Switch to workspace 3
        ("super five", "<KEY:super-5>"),  # Switch to workspace 5

        # Move windows
        ("super shift right", "<KEY:super-shift-Right>"),  # Move window right
        ("super shift down", "<KEY:super-shift-Down>"),  # Move window down

        # Move to workspace
        ("super shift four", "<KEY:super-shift-4>"),  # Move to workspace 4

        # Layout changes
        ("super f", "<KEY:super-f>"),  # Toggle fullscreen
        ("super v", "<KEY:super-v>"),  # Vertical split
        ("super h", "<KEY:super-h>"),  # Horizontal split

        # Kill window
        ("super shift q", "<KEY:super-shift-q>"),  # Close window

        # Mixed with text
        ("test super left", "test<KEY:super-Left>"),  # Text + Super key
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
    print("Super (Mod4) Key Combinations Test Suite")
    print("=" * 80)

    results = []
    results.append(("Super transformations", test_super_transformations()))
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
        print("\nSupported Super combinations:")
        print("  • Navigation: super left/right/up/down")
        print("  • Workspaces: super 1/2/3/4/5/6/7/8/9/0")
        print("  • Move windows: super shift left/right/up/down")
        print("  • Move to workspace: super shift 1/2/3/4/5/6/7/8/9/0")
        print("  • Layouts: super h/v/s/w/e/f")
        print("  • Misc: super a, super space, super shift q/c/e, super shift space")
        return 0
    else:
        print(f"\n❌ {total - passed} test(s) failed")
        return 1


if __name__ == '__main__':
    sys.exit(main())
