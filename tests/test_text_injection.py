#!/usr/bin/env python3
"""
Test text injection module with automated verification.
Tests wtype integration, Unicode handling, and clipboard fallback.
"""

import sys
import time
import subprocess
from pathlib import Path

# Add src to path
sys.path.insert(0, str(Path(__file__).parent.parent / 'src'))

from text_injection import TextInjector, InjectionMethod


def test_wtype_availability():
    """Test if wtype is installed and working"""
    print("=" * 80)
    print("Test 1: wtype Availability")
    print("=" * 80)

    try:
        result = subprocess.run(
            ['which', 'wtype'],
            capture_output=True,
            timeout=1
        )

        if result.returncode == 0:
            wtype_path = result.stdout.decode('utf-8').strip()
            print(f"✓ wtype found: {wtype_path}")

            # Test wtype version
            version_result = subprocess.run(
                ['wtype', '--version'],
                capture_output=True,
                timeout=1
            )
            version = version_result.stdout.decode('utf-8').strip()
            print(f"  Version: {version}")

            return True
        else:
            print("✗ wtype not found")
            print("  Install with: sudo apt install wtype")
            return False

    except Exception as e:
        print(f"✗ Error checking wtype: {e}")
        return False


def test_clipboard_availability():
    """Test if wl-clipboard is installed"""
    print("\n" + "=" * 80)
    print("Test 2: wl-clipboard Availability")
    print("=" * 80)

    try:
        result = subprocess.run(
            ['which', 'wl-copy'],
            capture_output=True,
            timeout=1
        )

        if result.returncode == 0:
            wl_copy_path = result.stdout.decode('utf-8').strip()
            print(f"✓ wl-copy found: {wl_copy_path}")
            return True
        else:
            print("✗ wl-copy not found")
            print("  Install with: sudo apt install wl-clipboard")
            return False

    except Exception as e:
        print(f"✗ Error checking wl-clipboard: {e}")
        return False


def test_injector_initialization():
    """Test TextInjector initialization"""
    print("\n" + "=" * 80)
    print("Test 3: TextInjector Initialization")
    print("=" * 80)

    try:
        # Test wtype method
        injector = TextInjector(method=InjectionMethod.WTYPE)
        print(f"✓ Initialized with method: {injector.method.value}")

        # Test clipboard method
        clipboard_injector = TextInjector(method=InjectionMethod.CLIPBOARD)
        print(f"✓ Initialized clipboard method: {clipboard_injector.method.value}")

        return True

    except Exception as e:
        print(f"✗ Initialization failed: {e}")
        return False


def test_text_injection_manual():
    """Manual test requiring user to focus a text editor"""
    print("\n" + "=" * 80)
    print("Test 4: Manual Text Injection (Interactive)")
    print("=" * 80)
    print("\n⚠ This test requires manual verification:")
    print("  1. Open a text editor (gedit, kate, vim, etc.)")
    print("  2. Focus the editor window")
    print("  3. Press Enter to start test\n")

    input("Press Enter when ready...")

    injector = TextInjector()

    # Test 1: Simple ASCII text
    print("\n📝 Test 4.1: Simple ASCII text")
    print("  You should see this text appear in your editor in 3 seconds...")
    time.sleep(3)

    test_text = "Hello from Swictation! This is a test."
    success = injector.inject(test_text)

    if success:
        print(f"  ✓ Injected: {test_text}")
    else:
        print("  ✗ Injection failed")

    time.sleep(2)

    # Test 2: Unicode characters
    print("\n📝 Test 4.2: Unicode characters")
    print("  Testing special characters...")
    time.sleep(1)

    unicode_text = "\n\nUnicode test: émojis 🎉 spëcial çharacters αβγ 中文"
    success = injector.inject(unicode_text)

    if success:
        print(f"  ✓ Injected: {unicode_text}")
    else:
        print("  ✗ Unicode injection failed")

    time.sleep(2)

    # Test 3: Newlines and formatting
    print("\n📝 Test 4.3: Newlines and formatting")
    multiline_text = "\n\nMultiline test:\n- Line 1\n- Line 2\n- Line 3\n"
    success = injector.inject(multiline_text)

    if success:
        print("  ✓ Injected multiline text")
    else:
        print("  ✗ Multiline injection failed")

    print("\n" + "=" * 80)
    print("Manual test complete! Check your text editor for results.")
    print("=" * 80)


def test_clipboard_method():
    """Test clipboard fallback method"""
    print("\n" + "=" * 80)
    print("Test 5: Clipboard Method")
    print("=" * 80)

    try:
        injector = TextInjector(method=InjectionMethod.CLIPBOARD)

        test_text = "Clipboard test - this should be in your clipboard"
        success = injector.inject(test_text)

        if success:
            print(f"✓ Text copied to clipboard: {test_text}")
            print("  You can paste with Ctrl+V or Cmd+V")

            # Verify clipboard contents
            result = subprocess.run(
                ['wl-paste'],
                capture_output=True,
                timeout=2
            )

            clipboard_content = result.stdout.decode('utf-8')
            if clipboard_content == test_text:
                print("✓ Clipboard verification successful")
                return True
            else:
                print(f"⚠ Clipboard content mismatch")
                print(f"  Expected: {test_text}")
                print(f"  Got: {clipboard_content}")
                return False
        else:
            print("✗ Clipboard copy failed")
            return False

    except Exception as e:
        print(f"✗ Clipboard test failed: {e}")
        return False


def test_special_keys():
    """Test special key injection"""
    print("\n" + "=" * 80)
    print("Test 6: Special Key Injection (Manual)")
    print("=" * 80)
    print("\n⚠ Focus a text editor and press Enter...")

    input("Press Enter when ready...")

    injector = TextInjector()

    print("\n  Injecting text with Return keys...")
    time.sleep(2)

    # Inject text followed by Enter keys
    injector.inject("Line 1")
    injector.inject_with_keys(['Return'])
    time.sleep(0.2)

    injector.inject("Line 2")
    injector.inject_with_keys(['Return'])
    time.sleep(0.2)

    injector.inject("Line 3")
    injector.inject_with_keys(['Return', 'Return'])

    print("✓ Special key injection complete")
    print("  Check your editor for 3 lines with proper newlines")


def main():
    """Run all tests"""
    print("=" * 80)
    print("Swictation Text Injection Test Suite")
    print("=" * 80)

    # Automated tests
    results = []

    results.append(("wtype availability", test_wtype_availability()))
    results.append(("wl-clipboard availability", test_clipboard_availability()))
    results.append(("TextInjector initialization", test_injector_initialization()))
    results.append(("Clipboard method", test_clipboard_method()))

    # Manual tests (optional)
    print("\n" + "=" * 80)
    print("Manual Tests (Optional)")
    print("=" * 80)
    print("\nThe following tests require manual verification.")
    print("They will inject text into your focused window.\n")

    response = input("Run manual tests? (y/N): ").strip().lower()

    if response == 'y':
        test_text_injection_manual()
        test_special_keys()

    # Summary
    print("\n" + "=" * 80)
    print("TEST SUMMARY")
    print("=" * 80)

    passed = sum(1 for _, result in results if result)
    total = len(results)

    for test_name, result in results:
        status = "✓ PASS" if result else "✗ FAIL"
        print(f"{status}: {test_name}")

    print(f"\nAutomated tests: {passed}/{total} passed")

    if passed == total:
        print("\n🎉 All automated tests passed!")
    else:
        print(f"\n⚠ {total - passed} test(s) failed")

    print("\n" + "=" * 80)


if __name__ == '__main__':
    main()
