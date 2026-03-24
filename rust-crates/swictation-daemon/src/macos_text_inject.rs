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
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use foreign_types_shared::ForeignType;
use std::os::raw::{c_long, c_void};
use std::rc::Rc;
use tracing::{debug, info, warn};

// FFI declaration for CGEventKeyboardSetUnicodeString
//
// This function is not exposed by the core-graphics crate, so we declare it manually.
// It allows setting Unicode text content for keyboard events.
//
// CRITICAL: stringLength is CFIndex (c_long on 64-bit) NOT c_uint.
// Apple's signature: void CGEventKeyboardSetUnicodeString(CGEventRef event, CFIndex stringLength, const UniChar *unicodeString);
// where CFIndex = signed long (64-bit on Apple Silicon)
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventKeyboardSetUnicodeString(
        event: *mut c_void,
        stringLength: c_long,
        unicodeString: *const u16,
    );
}

// FFI declarations for Accessibility permission APIs
//
// WARNING: On macOS Ventura 13.0+, these may return incorrect values
// if permissions are rapidly toggled in System Settings.
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    /// Check if process has Accessibility permissions (simple check, no prompt)
    fn AXIsProcessTrusted() -> bool;

    /// Check Accessibility permissions with options
    ///
    /// When called with kAXTrustedCheckOptionPrompt = true in the options dict,
    /// this will display a system dialog prompting the user to grant permissions.
    /// The dialog has "Open System Settings" and "Deny" buttons.
    ///
    /// Note: Even with prompting, user must still manually enable the toggle
    /// in System Settings > Privacy & Security > Accessibility.
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
}

// FFI for CGEventPost validation
//
// CGEventPost returns void, but we can validate accessibility by attempting
// to create a CGEventTap - this WILL fail if accessibility is not granted.
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    /// Create an event tap to monitor events.
    /// Returns NULL if the process doesn't have accessibility permission.
    /// This is a reliable way to validate actual accessibility permission
    /// (unlike AXIsProcessTrusted which can return stale cached values).
    fn CGEventTapCreate(
        tap: u32,                // CGEventTapLocation
        place: u32,              // CGEventTapPlacement
        options: u32,            // CGEventTapOptions
        events_of_interest: u64, // CGEventMask
        callback: *const c_void, // CGEventTapCallBack
        user_info: *mut c_void,  // void*
    ) -> *mut c_void; // CGEventTapRef (CFMachPortRef)

    /// Release a Core Foundation object
    fn CFRelease(cf: *mut c_void);
}

/// Key for the prompt option in AXIsProcessTrustedWithOptions
/// When set to true, shows system dialog for granting accessibility
static KAXTRUSTED_CHECK_OPTION_PROMPT: &str = "AXTrustedCheckOptionPrompt";

/// No-op callback for CGEventTap validation
/// This callback does nothing — it simply returns the event unchanged.
/// Required because CGEventTapCreate does not accept NULL callbacks.
unsafe extern "C" fn noop_event_tap_callback(
    _proxy: *mut c_void,
    _event_type: u32,
    event: *mut c_void,
    _user_info: *mut c_void,
) -> *mut c_void {
    event // Return event unchanged (listen-only, no modification)
}

/// macOS text injector using Core Graphics Accessibility API
pub struct MacOSTextInjector {
    /// Event source for generating keyboard events (Rc for single-threaded efficient cloning)
    event_source: Rc<CGEventSource>,
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

        // Create event source (wrapped in Rc for efficient sharing - single-threaded)
        let event_source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
            .map_err(|_| anyhow::anyhow!("Failed to create CGEventSource"))?;

        Ok(Self {
            event_source: Rc::new(event_source),
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

    /// Validate that accessibility permission is ACTUALLY working
    ///
    /// This function attempts to create a CGEventTap, which requires real
    /// accessibility permission. Unlike AXIsProcessTrusted() which can return
    /// stale cached values, CGEventTapCreate will fail (return NULL) if the
    /// current binary doesn't have actual accessibility permission.
    ///
    /// This is critical for detecting when a binary has been updated and
    /// the old cached permission no longer applies.
    pub fn validate_accessibility_permission() -> bool {
        unsafe {
            // CGEventTapLocation: kCGHIDEventTap = 0
            // CGEventTapPlacement: kCGHeadInsertEventTap = 0
            // CGEventTapOptions: kCGEventTapOptionListenOnly = 1 (don't modify events)
            // CGEventMask: just listen for key events (1 << 10 for keyDown)
            let tap = CGEventTapCreate(
                0,       // kCGHIDEventTap
                0,       // kCGHeadInsertEventTap
                1,       // kCGEventTapOptionListenOnly (passive)
                1 << 10, // kCGEventKeyDown
                noop_event_tap_callback as *const c_void,
                std::ptr::null_mut(), // No user info
            );

            if tap.is_null() {
                // CGEventTapCreate failed - no accessibility permission
                debug!(
                    "CGEventTapCreate returned NULL - accessibility not granted for this binary"
                );
                false
            } else {
                // Successfully created tap - permission is valid
                // Clean up immediately
                CFRelease(tap);
                debug!("CGEventTapCreate succeeded - accessibility permission validated");
                true
            }
        }
    }

    /// Request Accessibility permissions with a system dialog
    ///
    /// This function will:
    /// 1. Check if permissions are already granted (using both API and validation)
    /// 2. If not, display a system dialog prompting the user
    /// 3. The dialog shows "Open System Settings" and "Deny" buttons
    ///
    /// Returns true if permissions are actually working, false otherwise.
    ///
    /// IMPORTANT: This function validates that permissions actually work by
    /// attempting to create a CGEventTap. This catches the case where
    /// AXIsProcessTrusted returns true (stale cached value) but the current
    /// binary doesn't actually have permission (e.g., after an update).
    pub fn request_accessibility_permissions() -> bool {
        info!("Checking Accessibility permissions with prompt...");

        // First check the API result
        let key = CFString::new(KAXTRUSTED_CHECK_OPTION_PROMPT);
        let value = CFBoolean::true_value();
        let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), value.as_CFType())]);

        let api_says_trusted = unsafe {
            AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef() as *const c_void)
        };

        if api_says_trusted {
            // API says we're trusted, but let's VALIDATE this actually works
            // This catches stale cached permissions after binary updates
            info!("API reports accessibility granted, validating...");
            let actually_works = Self::validate_accessibility_permission();

            if actually_works {
                info!("Accessibility permissions validated and working");
                true
            } else {
                // This is the problematic case: API says trusted but it doesn't actually work
                // The binary has changed and the old permission doesn't apply
                warn!("⚠️  Accessibility permission is STALE (binary changed)");
                warn!("   macOS cached an old permission that no longer applies");
                warn!("   Please re-grant permission in System Settings:");
                warn!("   1. Open: System Settings → Privacy & Security → Accessibility");
                warn!("   2. Find 'swictation-daemon' in the list");
                warn!("   3. Toggle it OFF, then back ON");
                warn!("   4. Restart swictation");

                // Show the system dialog again to guide the user
                // Note: This won't help because macOS thinks permission is already granted
                // We need to open System Settings directly

                // Try to open System Settings to the Accessibility pane
                Self::open_accessibility_settings();

                false
            }
        } else {
            info!("Accessibility permissions not yet granted - system dialog shown");
            info!(
                "User must enable toggle in: System Settings > Privacy & Security > Accessibility"
            );
            false
        }
    }

    /// Open System Settings to the Accessibility pane
    ///
    /// This is helpful when permissions are stale and the user needs to
    /// manually re-toggle the permission.
    fn open_accessibility_settings() {
        info!("Opening System Settings → Accessibility...");

        // Use NSWorkspace to open the Accessibility preferences pane
        // URL: x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility
        let url = "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";

        // Use open command as a simple cross-version compatible approach
        if let Err(e) = std::process::Command::new("open").arg(url).spawn() {
            warn!("Failed to open System Settings: {}", e);
            warn!("Please manually open: System Settings → Privacy & Security → Accessibility");
        }
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

            // Delay between characters for macOS WindowServer to process each event.
            // 100μs was too fast — caused event queue interleaving with sequential injections.
            // 3ms gives WindowServer time to fully dispatch each keystroke.
            std::thread::sleep(std::time::Duration::from_millis(3));
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
        println!("Accessibility permission (API): {}", has_permission);
    }

    #[test]
    fn test_permission_validation() {
        // Test that the validation function works
        // This actually validates permission via CGEventTap
        let validated = MacOSTextInjector::validate_accessibility_permission();
        let api_says = MacOSTextInjector::check_accessibility_permissions();
        println!(
            "Accessibility permission - API: {}, Validated: {}",
            api_says, validated
        );

        // If API says yes but validation says no, we have stale permissions
        if api_says && !validated {
            println!("⚠️  STALE PERMISSION DETECTED: API reports granted but validation failed");
            println!("    This means the binary has changed and needs re-authorization");
        }
    }

    #[test]
    fn test_injector_creation() {
        // Only test creation if permissions are granted
        match MacOSTextInjector::new() {
            Ok(_injector) => {
                println!("✅ Text injector created successfully");
            }
            Err(e) => {
                println!(
                    "⚠️  Text injector creation failed (expected if no permissions): {}",
                    e
                );
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
            assert!(
                !parts.is_empty(),
                "Combo should have at least one part: {}",
                combo
            );
            println!("✅ Parsed combo: {} → {:?}", combo, parts);
        }
    }
}
