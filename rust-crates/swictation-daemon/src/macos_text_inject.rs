//! macOS text injection using Core Graphics Accessibility API
//!
//! This module provides text injection for macOS using the CGEvent framework.
//! It supports:
//! - Plain text injection via Unicode strings
//! - Accessibility permission checking
//!
//! Text is injected verbatim: see `text_injection` for why no marker syntax is
//! interpreted.
//!
//! ## CRITICAL: FFI Required
//!
//! The `core-graphics` crate does NOT expose `CGEventKeyboardSetUnicodeString`.
//! We must declare the FFI binding manually.

use anyhow::Result;
use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;
use core_graphics::event::{CGEvent, CGEventTapLocation};
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

    /// Non-prompting event-synthesis permission check, available since macOS 10.15.
    fn CGPreflightPostEventAccess() -> bool;
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

/// Maximum UTF-16 code units per CGEvent.
///
/// Apple's `CGEventKeyboardSetUnicodeString` documentation specifies a limit
/// of 20 UniChar per event. Exceeding this may cause the extra characters to
/// be silently dropped on some macOS versions.
const BATCH_UTF16_LIMIT: usize = 20;

/// Delay in milliseconds between batched CGEvent chunks.
///
/// Each chunk is delivered atomically by WindowServer, so we only need enough
/// delay for the event to be dispatched before posting the next one.
/// 5 ms is conservative — each chunk carries up to 20 characters, so even a
/// 100-character string only takes ~25 ms total (vs. 300 ms with the old
/// per-character approach).
const BATCH_DELAY_MS: u64 = 5;

/// Returns `true` if the UTF-16 code unit is a high surrogate (first half of
/// a surrogate pair for characters outside the Basic Multilingual Plane).
#[inline]
fn is_high_surrogate(code_unit: u16) -> bool {
    (0xD800..=0xDBFF).contains(&code_unit)
}

/// Split `text` into the UTF-16 payloads to be carried by successive CGEvents.
///
/// Every code unit of the input appears exactly once and in order, so the text
/// handed to the injector is the text that gets typed — angle brackets, colons
/// and anything resembling a marker included. Chunks stay within
/// [`BATCH_UTF16_LIMIT`] and never split a surrogate pair.
fn utf16_chunks(text: &str) -> Vec<Vec<u16>> {
    let utf16_all: Vec<u16> = text.encode_utf16().collect();
    let mut chunks = Vec::new();

    let mut offset = 0;
    while offset < utf16_all.len() {
        let remaining = utf16_all.len() - offset;
        let mut chunk_len = remaining.min(BATCH_UTF16_LIMIT);

        // If we'd split a surrogate pair, back off by one code unit so
        // the high surrogate moves to the next chunk.
        if chunk_len < remaining && is_high_surrogate(utf16_all[offset + chunk_len - 1]) {
            chunk_len -= 1;
        }

        chunks.push(utf16_all[offset..offset + chunk_len].to_vec());
        offset += chunk_len;
    }

    chunks
}

fn require_posting_access(allowed: bool) -> Result<()> {
    if !allowed {
        anyhow::bail!(
            "Accessibility event-posting permission required; enable Swictation in \
             System Settings → Privacy & Security → Accessibility"
        );
    }
    Ok(())
}

/// Stop on permission loss, including between batches; never replay posted text.
/// CGEventPost has no delivery acknowledgement, so this verifies permission,
/// not whether the focused application accepted the dispatched events.
fn post_text_batches(
    text: &str,
    mut allowed: impl FnMut() -> bool,
    mut post: impl FnMut(&[u16]) -> Result<()>,
) -> Result<()> {
    for chunk in utf16_chunks(text) {
        require_posting_access(allowed())?;
        post(&chunk)?;
    }
    Ok(())
}

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
        // Check the operation we actually need on every recovery attempt.
        // This does not prompt and does not rely on a cached AX-only decision.
        require_posting_access(Self::check_event_posting_access())?;

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

    fn check_event_posting_access() -> bool {
        unsafe { CGPreflightPostEventAccess() }
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

    /// Inject text into the active window, verbatim
    ///
    /// # Arguments
    ///
    /// * `text` - The text to type. Every character is typed as-is; no marker
    ///   or escape syntax is interpreted.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use swictation_daemon::macos_text_inject::MacOSTextInjector;
    /// let injector = MacOSTextInjector::new()?;
    ///
    /// injector.inject_text("Hello, world!")?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn inject_text(&self, text: &str) -> Result<()> {
        self.inject_plain_text(text)
    }

    /// Type text as Unicode characters
    ///
    /// Uses CGEventKeyboardSetUnicodeString to inject Unicode text in batched
    /// chunks. Each CGEvent carries up to [`BATCH_UTF16_LIMIT`] UTF-16 code
    /// units, drastically reducing the number of HID events compared to
    /// character-by-character injection.
    ///
    /// ## Why batching matters
    ///
    /// The previous character-by-character approach posted 2 CGEvents per
    /// character (key down + key up) with a 3 ms inter-character delay.
    /// macOS WindowServer processes HID events asynchronously, and under load
    /// individual events — especially space characters — could be dropped or
    /// coalesced, producing output with missing spaces.
    ///
    /// By packing multiple characters into a single CGEvent via
    /// `CGEventKeyboardSetUnicodeString`, the WindowServer receives each chunk
    /// atomically. Spaces inside a chunk cannot be dropped independently.
    ///
    /// ## Chunk size
    ///
    /// Apple's `CGEventKeyboardSetUnicodeString` accepts up to 20 UniChar
    /// (UTF-16 code units) per event. We respect this limit and never split
    /// a surrogate pair across chunk boundaries.
    fn inject_plain_text(&self, text: &str) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        let char_count = text.chars().count();
        debug!("Injecting {} characters (batched)", char_count);

        post_text_batches(text, Self::check_event_posting_access, |chunk| {
            // Key down event carrying the chunk's Unicode content
            let event = CGEvent::new_keyboard_event((*self.event_source).clone(), 0, true)
                .map_err(|_| anyhow::anyhow!("Failed to create key down event"))?;

            unsafe {
                CGEventKeyboardSetUnicodeString(
                    event.as_ptr() as *mut c_void,
                    chunk.len() as c_long,
                    chunk.as_ptr(),
                );
            }

            event.post(CGEventTapLocation::HID);

            // Key up event (no Unicode payload needed)
            let event_up = CGEvent::new_keyboard_event((*self.event_source).clone(), 0, false)
                .map_err(|_| anyhow::anyhow!("Failed to create key up event"))?;

            event_up.post(CGEventTapLocation::HID);

            // Inter-chunk delay for WindowServer to dispatch the event.
            // Each chunk is processed atomically, so we only need to wait
            // once per chunk rather than once per character.
            std::thread::sleep(std::time::Duration::from_millis(BATCH_DELAY_MS));
            Ok(())
        })
    }
}

#[cfg(test)]
#[path = "macos_text_inject_tests.rs"]
mod tests;
