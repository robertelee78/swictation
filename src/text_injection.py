#!/usr/bin/env python3
"""
Text injection module for Swictation.
Uses wtype for Wayland-native text injection with clipboard fallback.
"""

import subprocess
import time
from typing import Optional, Callable
from enum import Enum


class InjectionMethod(Enum):
    """Text injection methods"""
    WTYPE = "wtype"           # Direct keyboard typing (preferred)
    CLIPBOARD = "clipboard"   # Clipboard paste (fallback)


class TextInjector:
    """
    Wayland-native text injection using wtype.
    Supports Unicode, special characters, and clipboard fallback.
    """

    def __init__(
        self,
        method: InjectionMethod = InjectionMethod.WTYPE,
        typing_delay: float = 0.0,
        on_inject_callback: Optional[Callable[[str], None]] = None
    ):
        """
        Initialize text injector.

        Args:
            method: Injection method (wtype or clipboard)
            typing_delay: Delay between characters in seconds (for wtype)
            on_inject_callback: Optional callback when text is injected
        """
        self.method = method
        self.typing_delay = typing_delay
        self.on_inject_callback = on_inject_callback

        # Verify required tools are available
        self._check_dependencies()

    def _check_dependencies(self):
        """Check if required system tools are installed"""
        if self.method == InjectionMethod.WTYPE:
            if not self._command_exists('wtype'):
                print("⚠ wtype not found. Install with: sudo apt install wtype")
                print("  Falling back to clipboard method")
                self.method = InjectionMethod.CLIPBOARD

        if self.method == InjectionMethod.CLIPBOARD:
            if not self._command_exists('wl-copy'):
                raise RuntimeError(
                    "wl-clipboard not found. Install with: sudo apt install wl-clipboard"
                )

    def _command_exists(self, command: str) -> bool:
        """Check if a command exists in PATH"""
        try:
            subprocess.run(
                ['which', command],
                capture_output=True,
                check=True,
                timeout=1
            )
            return True
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
            return False

    def inject(self, text: str, method: Optional[InjectionMethod] = None) -> bool:
        """
        Inject text into focused window.

        Args:
            text: Text to inject
            method: Override injection method (optional)

        Returns:
            True if successful, False otherwise
        """
        if not text:
            return True

        injection_method = method or self.method

        try:
            if injection_method == InjectionMethod.WTYPE:
                success = self._inject_wtype(text)
            else:
                success = self._inject_clipboard(text)

            if success and self.on_inject_callback:
                self.on_inject_callback(text)

            return success

        except Exception as e:
            print(f"✗ Text injection failed: {e}")
            # Try clipboard fallback if wtype fails
            if injection_method == InjectionMethod.WTYPE:
                print("  Attempting clipboard fallback...")
                return self._inject_clipboard(text)
            return False

    def _inject_wtype(self, text: str) -> bool:
        """
        Inject text using wtype (direct keyboard typing).
        Handles Unicode and special characters correctly.
        """
        try:
            # wtype expects UTF-8 text as stdin
            # Use '-' to read from stdin for proper Unicode handling
            process = subprocess.Popen(
                ['wtype', '-'],
                stdin=subprocess.PIPE,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE
            )

            # Send text as UTF-8 bytes
            stdout, stderr = process.communicate(
                input=text.encode('utf-8'),
                timeout=5
            )

            if process.returncode != 0:
                error_msg = stderr.decode('utf-8', errors='ignore')
                print(f"  wtype error: {error_msg}")
                return False

            # Optional typing delay
            if self.typing_delay > 0:
                time.sleep(self.typing_delay * len(text))

            return True

        except subprocess.TimeoutExpired:
            print("  wtype timeout")
            process.kill()
            return False
        except Exception as e:
            print(f"  wtype exception: {e}")
            return False

    def _inject_clipboard(self, text: str) -> bool:
        """
        Inject text using clipboard paste.
        Fallback method when wtype is unavailable.
        """
        try:
            # Copy text to clipboard
            process = subprocess.Popen(
                ['wl-copy'],
                stdin=subprocess.PIPE,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE
            )

            stdout, stderr = process.communicate(
                input=text.encode('utf-8'),
                timeout=2
            )

            if process.returncode != 0:
                error_msg = stderr.decode('utf-8', errors='ignore')
                print(f"  wl-copy error: {error_msg}")
                return False

            print("  ✓ Text copied to clipboard (paste with Ctrl+V or Cmd+V)")
            return True

        except subprocess.TimeoutExpired:
            print("  wl-copy timeout")
            process.kill()
            return False
        except Exception as e:
            print(f"  wl-copy exception: {e}")
            return False

    def inject_with_keys(self, keys: list[str]) -> bool:
        """
        Inject special key sequences using wtype.

        Supports:
        - Simple keys: ['Return', 'Tab', 'BackSpace']
        - Ctrl combinations: ['ctrl-u', 'ctrl-c', 'ctrl-s']

        Args:
            keys: List of key names (e.g., ['Return', 'Tab', 'Backspace', 'ctrl-u'])

        Returns:
            True if successful, False otherwise
        """
        if self.method != InjectionMethod.WTYPE:
            print("⚠ Key injection only supported with wtype method")
            return False

        try:
            # wtype uses -k flag for special keys
            for key in keys:
                # Check if this is a Ctrl combination (e.g., 'ctrl-u')
                if key.startswith('ctrl-'):
                    # Extract the letter (e.g., 'ctrl-u' -> 'u')
                    letter = key.split('-', 1)[1]

                    # Press Ctrl, type letter, release Ctrl
                    # wtype -M ctrl -k u -m ctrl
                    subprocess.run(
                        ['wtype', '-M', 'ctrl', '-k', letter, '-m', 'ctrl'],
                        capture_output=True,
                        check=True,
                        timeout=2
                    )
                else:
                    # Regular key (e.g., 'Return', 'BackSpace')
                    subprocess.run(
                        ['wtype', '-k', key],
                        capture_output=True,
                        check=True,
                        timeout=2
                    )

            return True

        except subprocess.CalledProcessError as e:
            print(f"✗ Key injection failed: {e}")
            return False
        except Exception as e:
            print(f"✗ Key injection exception: {e}")
            return False

    def stream_text(self, text_generator, chunk_size: int = 10) -> bool:
        """
        Stream text as it becomes available (for real-time transcription).

        Args:
            text_generator: Iterator/generator yielding text chunks
            chunk_size: Minimum characters to accumulate before injecting

        Returns:
            True if successful, False otherwise
        """
        try:
            buffer = ""

            for chunk in text_generator:
                buffer += chunk

                # Inject when buffer reaches chunk_size or generator ends
                if len(buffer) >= chunk_size:
                    if not self.inject(buffer):
                        return False
                    buffer = ""

            # Inject remaining buffer
            if buffer:
                return self.inject(buffer)

            return True

        except Exception as e:
            print(f"✗ Streaming injection failed: {e}")
            return False


def test_text_injection():
    """Test text injection functionality"""
    print("=" * 80)
    print("Text Injection Test")
    print("=" * 80)

    # Test wtype availability
    print("\n1️⃣ Checking dependencies...")
    injector = TextInjector()
    print(f"✓ Method: {injector.method.value}")

    # Test simple text
    print("\n2️⃣ Testing simple text injection...")
    print("  Focus a text editor window within 3 seconds...")
    time.sleep(3)

    test_text = "Hello from Swictation!"
    success = injector.inject(test_text)
    if success:
        print(f"✓ Injected: {test_text}")
    else:
        print("✗ Injection failed")

    time.sleep(2)

    # Test Unicode
    print("\n3️⃣ Testing Unicode injection...")
    unicode_text = "Testing émojis 🎉 and spëcial çharacters"
    success = injector.inject(unicode_text)
    if success:
        print(f"✓ Injected: {unicode_text}")
    else:
        print("✗ Unicode injection failed")

    time.sleep(2)

    # Test special keys
    print("\n4️⃣ Testing special key injection...")
    success = injector.inject_with_keys(['Return', 'Return'])
    if success:
        print("✓ Injected: 2x Return key")
    else:
        print("✗ Key injection failed")

    # Test clipboard fallback
    print("\n5️⃣ Testing clipboard method...")
    clipboard_injector = TextInjector(method=InjectionMethod.CLIPBOARD)
    success = clipboard_injector.inject("Clipboard test - paste this manually")
    if success:
        print("✓ Text copied to clipboard")
    else:
        print("✗ Clipboard method failed")

    print("\n" + "=" * 80)
    print("Test complete!")
    print("=" * 80)


if __name__ == '__main__':
    test_text_injection()
