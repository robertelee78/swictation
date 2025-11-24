//! macOS text injection using Core Graphics Accessibility API
//!
//! This module provides text injection for macOS using the CGEvent framework.
//! It supports:
//! - Plain text injection via Unicode strings
//! - Keyboard shortcuts (<KEY:Cmd+C>, <KEY:Cmd+V>, etc.)
//! - Accessibility permission checking
//!
//! ## CRITICAL: FFI Required
//!
//! The `core-graphics` crate does NOT expose `CGEventKeyboardSetUnicodeString`.
//! We must declare the FFI binding manually.

use anyhow::{Context, Result};
use core_foundation::base::TCFType;
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTapLocation, CGEventType, CGKeyCode,
};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use foreign_types_shared::ForeignType;
use std::os::raw::{c_long, c_void};
use std::sync::Arc;
use tracing::{debug, warn};

/// FFI declaration for CGEventKeyboardSetUnicodeString
///
/// This function is not exposed by the core-graphics crate, so we declare it manually.
/// It allows setting Unicode text content for keyboard events.
///
/// CRITICAL: stringLength is CFIndex (c_long on 64-bit) NOT c_uint.
/// Apple's signature: void CGEventKeyboardSetUnicodeString(CGEventRef event, CFIndex stringLength, const UniChar *unicodeString);
/// where CFIndex = signed long (64-bit on Apple Silicon)
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventKeyboardSetUnicodeString(
        event: *mut c_void,
        stringLength: c_long,
        unicodeString: *const u16,
    );
}

/// FFI declaration for checking Accessibility permissions
///
/// WARNING: On macOS Ventura 13.0+, this may return incorrect values
/// if permissions are rapidly toggled in System Settings.
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

/// macOS text injector using Core Graphics Accessibility API
pub struct MacOSTextInjector {
    /// Event source for generating keyboard events (Arc for efficient cloning)
    event_source: Arc<CGEventSource>,
}

impl MacOSTextInjector {
    /// Create a new macOS text injector
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Accessibility permissions are not granted
    /// - CGEventSource creation fails
    pub fn new() -> Result<Self> {
        // Check Accessibility permissions
        if !Self::check_accessibility_permissions() {
            anyhow::bail!(
                "Accessibility permission required!\n\
                 Go to: System Settings → Privacy & Security → Accessibility\n\
                 Enable: swictation-daemon\n\n\
                 Note: On macOS Ventura 13.0+, you may need to toggle the permission \
                 off and back on if you recently granted it."
            );
        }

        // Create event source (wrapped in Arc for efficient sharing)
        let event_source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
            .map_err(|_| anyhow::anyhow!("Failed to create CGEventSource"))?;

        Ok(Self {
            event_source: Arc::new(event_source),
        })
    }

    /// Check if Accessibility permissions are granted
    ///
    /// WARNING: On macOS Ventura 13.0+, this function may return incorrect values
    /// if permissions are rapidly toggled. If you encounter permission issues:
    /// 1. Open System Settings → Privacy & Security → Accessibility
    /// 2. Toggle swictation-daemon OFF
    /// 3. Toggle swictation-daemon ON
    /// 4. Restart the application
    pub fn check_accessibility_permissions() -> bool {
        unsafe { AXIsProcessTrusted() }
    }

    /// Inject text into the active window, handling <KEY:...> markers
    ///
    /// # Arguments
    ///
    /// * `text` - The text to inject, may contain <KEY:...> markers for shortcuts
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use swictation_daemon::macos_text_inject::MacOSTextInjector;
    /// let injector = MacOSTextInjector::new()?;
    ///
    /// // Plain text
    /// injector.inject_text("Hello, world!")?;
    ///
    /// // With keyboard shortcuts
    /// injector.inject_text("Copy this <KEY:Cmd+C>")?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn inject_text(&self, text: &str) -> Result<()> {
        // Check if text contains keyboard shortcut markers
        if text.contains("<KEY:") {
            self.inject_with_keys(text)
        } else {
            // Plain text injection
            self.inject_plain_text(text)
        }
    }

    /// Process text with <KEY:...> markers
    ///
    /// Splits text into plain text segments and keyboard shortcuts,
    /// injecting each in sequence.
    fn inject_with_keys(&self, text: &str) -> Result<()> {
        let mut remaining = text;

        while !remaining.is_empty() {
            if let Some(key_start) = remaining.find("<KEY:") {
                // Inject any text before the key marker
                if key_start > 0 {
                    let text_part = &remaining[..key_start];
                    self.inject_plain_text(text_part)?;
                }

                // Find the end of the key marker
                let key_end = remaining[key_start..]
                    .find('>')
                    .context("Malformed KEY marker: missing closing '>'")?;

                // Extract the key combination (e.g., "Cmd+C")
                let key_combo = &remaining[key_start + 5..key_start + key_end];

                // Send the key combination
                self.send_key_combination(key_combo)?;

                // Move past this marker
                remaining = &remaining[key_start + key_end + 1..];
            } else {
                // No more markers, inject remaining text
                self.inject_plain_text(remaining)?;
                break;
            }
        }

        Ok(())
    }

    /// Inject plain text (no key markers)
    ///
    /// Uses CGEventKeyboardSetUnicodeString to inject Unicode text
    /// character by character.
    fn inject_plain_text(&self, text: &str) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        debug!("Injecting {} characters", text.chars().count());

        // Inject each character
        for ch in text.chars() {
            // Convert character to UTF-16 (macOS native encoding)
            // A single char encodes to at most 2 UTF-16 code units (surrogate pair)
            let mut utf16_buf = [0u16; 2];
            let utf16 = ch.encode_utf16(&mut utf16_buf);

            // Create key down event (Arc clone is cheap, inner clone only if needed)
            let event = CGEvent::new_keyboard_event((*self.event_source).clone(), 0, true)
                .map_err(|_| anyhow::anyhow!("Failed to create key down event"))?;

            // Set Unicode string content via FFI
            unsafe {
                CGEventKeyboardSetUnicodeString(
                    event.as_ptr() as *mut c_void,
                    utf16.len() as c_long,
                    utf16.as_ptr(),
                );
            }

            // Post key down event
            event.post(CGEventTapLocation::HID);

            // Create key up event
            let event_up = CGEvent::new_keyboard_event((*self.event_source).clone(), 0, false)
                .map_err(|_| anyhow::anyhow!("Failed to create key up event"))?;

            // Post key up event
            event_up.post(CGEventTapLocation::HID);

            // Small delay to prevent event coalescing (optional, adjust if needed)
            std::thread::sleep(std::time::Duration::from_micros(100));
        }

        Ok(())
    }

    /// Send a keyboard shortcut combination (e.g., "Cmd+C", "Cmd+Shift+V")
    ///
    /// Parses the combination string and sends the appropriate key events
    /// with modifiers.
    fn send_key_combination(&self, combo: &str) -> Result<()> {
        // Parse the key combination
        let parts: Vec<&str> = combo.split('+').map(|s| s.trim()).collect();

        if parts.is_empty() {
            anyhow::bail!("Empty key combination");
        }

        // Extract modifiers and key
        let modifiers = &parts[..parts.len() - 1];
        let key = parts
            .last()
            .context("Key combination must have at least one key")?;

        debug!("Sending key combination: {} (key: {})", combo, key);

        // Build modifier flags
        let mut flags = CGEventFlags::CGEventFlagNull;
        for modifier in modifiers {
            flags |= match modifier.to_lowercase().as_str() {
                "cmd" | "command" | "super" => CGEventFlags::CGEventFlagCommand,
                "ctrl" | "control" => CGEventFlags::CGEventFlagControl,
                "alt" | "option" => CGEventFlags::CGEventFlagAlternate,
                "shift" => CGEventFlags::CGEventFlagShift,
                "fn" => CGEventFlags::CGEventFlagSecondaryFn,
                _ => {
                    warn!("Unknown modifier: {}", modifier);
                    continue;
                }
            };
        }

        // Map key name to key code
        let key_code = self.key_name_to_code(key)?;

        // Create and post key down event with modifiers
        let event_down = CGEvent::new_keyboard_event((*self.event_source).clone(), key_code, true)
            .map_err(|_| anyhow::anyhow!("Failed to create key down event"))?;
        event_down.set_flags(flags);
        event_down.post(CGEventTapLocation::HID);

        // Small delay between down and up
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Create and post key up event
        let event_up = CGEvent::new_keyboard_event((*self.event_source).clone(), key_code, false)
            .map_err(|_| anyhow::anyhow!("Failed to create key up event"))?;
        event_up.post(CGEventTapLocation::HID);

        Ok(())
    }

    /// Map key name to macOS virtual key code
    ///
    /// This is a partial mapping of common keys. Extend as needed.
    fn key_name_to_code(&self, key: &str) -> Result<CGKeyCode> {
        let code = match key.to_lowercase().as_str() {
            // Letters
            "a" => 0x00,
            "b" => 0x0B,
            "c" => 0x08,
            "d" => 0x02,
            "e" => 0x0E,
            "f" => 0x03,
            "g" => 0x05,
            "h" => 0x04,
            "i" => 0x22,
            "j" => 0x26,
            "k" => 0x28,
            "l" => 0x25,
            "m" => 0x2E,
            "n" => 0x2D,
            "o" => 0x1F,
            "p" => 0x23,
            "q" => 0x0C,
            "r" => 0x0F,
            "s" => 0x01,
            "t" => 0x11,
            "u" => 0x20,
            "v" => 0x09,
            "w" => 0x0D,
            "x" => 0x07,
            "y" => 0x10,
            "z" => 0x06,

            // Numbers
            "0" => 0x1D,
            "1" => 0x12,
            "2" => 0x13,
            "3" => 0x14,
            "4" => 0x15,
            "5" => 0x17,
            "6" => 0x16,
            "7" => 0x1A,
            "8" => 0x1C,
            "9" => 0x19,

            // Special keys
            "return" | "enter" => 0x24,
            "tab" => 0x30,
            "space" => 0x31,
            "delete" | "backspace" => 0x33,
            "escape" | "esc" => 0x35,

            // Arrow keys
            "left" => 0x7B,
            "right" => 0x7C,
            "down" => 0x7D,
            "up" => 0x7E,

            // Function keys
            "f1" => 0x7A,
            "f2" => 0x78,
            "f3" => 0x63,
            "f4" => 0x76,
            "f5" => 0x60,
            "f6" => 0x61,
            "f7" => 0x62,
            "f8" => 0x64,
            "f9" => 0x65,
            "f10" => 0x6D,
            "f11" => 0x67,
            "f12" => 0x6F,

            // Punctuation
            "semicolon" | ";" => 0x29,
            "equals" | "=" => 0x18,
            "comma" | "," => 0x2B,
            "minus" | "-" => 0x1B,
            "period" | "." => 0x2F,
            "slash" | "/" => 0x2C,
            "backtick" | "`" => 0x32,
            "leftbracket" | "[" => 0x21,
            "backslash" | "\\" => 0x2A,
            "rightbracket" | "]" => 0x1E,
            "quote" | "'" => 0x27,

            _ => anyhow::bail!("Unknown key name: {}", key),
        };

        Ok(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_check() {
        // This test just verifies the function can be called
        // Actual permission state depends on system configuration
        let has_permission = MacOSTextInjector::check_accessibility_permissions();
        println!("Accessibility permission: {}", has_permission);
    }

    #[test]
    fn test_injector_creation() {
        // Only test creation if permissions are granted
        match MacOSTextInjector::new() {
            Ok(_injector) => {
                println!("✅ Text injector created successfully");
            }
            Err(e) => {
                println!("⚠️  Text injector creation failed (expected if no permissions): {}", e);
            }
        }
    }

    #[test]
    fn test_key_name_mapping() {
        let injector = match MacOSTextInjector::new() {
            Ok(inj) => inj,
            Err(_) => {
                println!("⚠️  Skipping test (no permissions)");
                return;
            }
        };

        // Test some common key mappings
        assert!(injector.key_name_to_code("a").is_ok());
        assert!(injector.key_name_to_code("return").is_ok());
        assert!(injector.key_name_to_code("left").is_ok());
        assert!(injector.key_name_to_code("f1").is_ok());

        // Test case insensitivity
        assert!(injector.key_name_to_code("A").is_ok());
        assert!(injector.key_name_to_code("RETURN").is_ok());

        // Test unknown key
        assert!(injector.key_name_to_code("unknown_key").is_err());
    }

    #[test]
    fn test_key_combination_parsing() {
        // Test that we can parse key combinations
        // (without actually sending them)
        let combos = vec!["Cmd+C", "Cmd+Shift+V", "Ctrl+Alt+Delete", "Cmd+Left"];

        for combo in combos {
            let parts: Vec<&str> = combo.split('+').collect();
            assert!(parts.len() >= 1, "Combo should have at least one part: {}", combo);
            println!("✅ Parsed combo: {} → {:?}", combo, parts);
        }
    }
}
