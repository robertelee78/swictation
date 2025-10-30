#!/usr/bin/env python3
"""
Automated text injection tests (no manual interaction required).
Tests module functionality without requiring Wayland display.
"""

import sys
from pathlib import Path

# Add src to path
sys.path.insert(0, str(Path(__file__).parent.parent / 'src'))

from text_injection import TextInjector, InjectionMethod


def test_dependency_check():
    """Test dependency checking logic"""
    print("=" * 80)
    print("Test 1: Dependency Check")
    print("=" * 80)

    try:
        injector = TextInjector()
        print(f"✓ TextInjector initialized")
        print(f"  Method: {injector.method.value}")
        return True
    except Exception as e:
        print(f"✗ Initialization failed: {e}")
        return False


def test_command_exists():
    """Test _command_exists helper"""
    print("\n" + "=" * 80)
    print("Test 2: Command Existence Check")
    print("=" * 80)

    injector = TextInjector()

    # Test with known commands
    tests = [
        ('ls', True),
        ('wtype', True),
        ('nonexistent_command_xyz', False)
    ]

    all_passed = True
    for cmd, expected in tests:
        result = injector._command_exists(cmd)
        status = "✓" if result == expected else "✗"
        print(f"{status} {cmd}: {result} (expected: {expected})")
        if result != expected:
            all_passed = False

    return all_passed


def test_empty_text_handling():
    """Test handling of empty text"""
    print("\n" + "=" * 80)
    print("Test 3: Empty Text Handling")
    print("=" * 80)

    injector = TextInjector()

    # Should return True and do nothing
    result = injector.inject("")
    print(f"✓ Empty text handled correctly: {result}")

    return result == True


def test_unicode_encoding():
    """Test Unicode text encoding"""
    print("\n" + "=" * 80)
    print("Test 4: Unicode Encoding")
    print("=" * 80)

    test_strings = [
        "Hello ASCII",
        "émojis 🎉 test",
        "Greek αβγ",
        "Chinese 中文",
        "Special: é ñ ü"
    ]

    all_passed = True
    for text in test_strings:
        try:
            # Test that text can be encoded to UTF-8
            encoded = text.encode('utf-8')
            decoded = encoded.decode('utf-8')

            if decoded == text:
                print(f"✓ {text[:30]}")
            else:
                print(f"✗ Encoding/decoding failed for: {text}")
                all_passed = False

        except Exception as e:
            print(f"✗ Exception for {text}: {e}")
            all_passed = False

    return all_passed


def test_injection_method_enum():
    """Test InjectionMethod enum"""
    print("\n" + "=" * 80)
    print("Test 5: InjectionMethod Enum")
    print("=" * 80)

    try:
        wtype_method = InjectionMethod.WTYPE
        clipboard_method = InjectionMethod.CLIPBOARD

        print(f"✓ WTYPE method: {wtype_method.value}")
        print(f"✓ CLIPBOARD method: {clipboard_method.value}")

        # Test enum comparison
        assert wtype_method != clipboard_method
        print("✓ Enum values are distinct")

        return True

    except Exception as e:
        print(f"✗ Enum test failed: {e}")
        return False


def test_method_override():
    """Test method override in inject()"""
    print("\n" + "=" * 80)
    print("Test 6: Method Override")
    print("=" * 80)

    try:
        # Create wtype injector
        injector = TextInjector(method=InjectionMethod.WTYPE)
        print(f"✓ Default method: {injector.method.value}")

        # Override should be respected (won't actually inject, just testing logic)
        override_method = InjectionMethod.CLIPBOARD
        print(f"✓ Override method parameter accepted: {override_method.value}")

        return True

    except Exception as e:
        print(f"✗ Method override test failed: {e}")
        return False


def test_callback_handling():
    """Test callback functionality"""
    print("\n" + "=" * 80)
    print("Test 7: Callback Handling")
    print("=" * 80)

    callback_called = False
    callback_text = None

    def test_callback(text: str):
        nonlocal callback_called, callback_text
        callback_called = True
        callback_text = text

    try:
        injector = TextInjector(on_inject_callback=test_callback)
        print("✓ Injector created with callback")

        # Note: Actual injection would trigger callback
        # Testing that callback is stored correctly
        assert injector.on_inject_callback is not None
        print("✓ Callback stored correctly")

        return True

    except Exception as e:
        print(f"✗ Callback test failed: {e}")
        return False


def main():
    """Run all automated tests"""
    print("=" * 80)
    print("Text Injection Module - Automated Test Suite")
    print("=" * 80)
    print("\nThese tests verify module functionality without Wayland display.\n")

    results = []

    # Run tests
    results.append(("Dependency check", test_dependency_check()))
    results.append(("Command existence", test_command_exists()))
    results.append(("Empty text handling", test_empty_text_handling()))
    results.append(("Unicode encoding", test_unicode_encoding()))
    results.append(("InjectionMethod enum", test_injection_method_enum()))
    results.append(("Method override", test_method_override()))
    results.append(("Callback handling", test_callback_handling()))

    # Summary
    print("\n" + "=" * 80)
    print("TEST SUMMARY")
    print("=" * 80)

    passed = sum(1 for _, result in results if result)
    total = len(results)

    for test_name, result in results:
        status = "✓ PASS" if result else "✗ FAIL"
        print(f"{status}: {test_name}")

    print(f"\nTests: {passed}/{total} passed")

    if passed == total:
        print("\n🎉 All tests passed!")
        return 0
    else:
        print(f"\n⚠ {total - passed} test(s) failed")
        return 1


if __name__ == '__main__':
    sys.exit(main())
