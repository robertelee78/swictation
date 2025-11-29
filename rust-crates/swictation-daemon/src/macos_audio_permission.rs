//! macOS microphone permission handling using AVFoundation
//!
//! This module provides microphone permission request for macOS using AVFoundation APIs.
//! It parallels the accessibility permission handling in macos_text_inject.rs.
//!
//! ## Why This Is Needed
//!
//! The cpal audio library uses CoreAudio directly, which does NOT trigger macOS
//! permission dialogs. Without explicit permission request via AVFoundation's
//! `AVCaptureDevice.requestAccessForMediaType:`, audio capture will either:
//! - Block indefinitely waiting for permission
//! - Silently fail to capture any audio
//! - Return an error from CoreAudio
//!
//! ## Usage
//!
//! Call `request_microphone_permission()` at daemon startup, BEFORE attempting
//! any audio capture operations.

use std::ffi::CStr;
use std::os::raw::c_void;
use std::time::Duration;
use tracing::{debug, info, warn};

/// AVAuthorizationStatus enum values from AVFoundation
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AVAuthorizationStatus {
    /// User has not yet made a choice (permission dialog not shown)
    NotDetermined = 0,
    /// User cannot grant permission (e.g., parental controls)
    Restricted = 1,
    /// User explicitly denied permission
    Denied = 2,
    /// User granted permission
    Authorized = 3,
}

impl From<i32> for AVAuthorizationStatus {
    fn from(value: i32) -> Self {
        match value {
            0 => AVAuthorizationStatus::NotDetermined,
            1 => AVAuthorizationStatus::Restricted,
            2 => AVAuthorizationStatus::Denied,
            3 => AVAuthorizationStatus::Authorized,
            _ => AVAuthorizationStatus::NotDetermined,
        }
    }
}

// FFI declarations for AVFoundation permission APIs
//
// These use Objective-C runtime to call AVCaptureDevice class methods.
// We use the objc crate for safe Objective-C interop.
#[link(name = "AVFoundation", kind = "framework")]
extern "C" {}

#[link(name = "objc", kind = "dylib")]
extern "C" {
    fn objc_getClass(name: *const i8) -> *mut c_void;
    fn sel_registerName(name: *const i8) -> *mut c_void;
    fn objc_msgSend(obj: *mut c_void, sel: *mut c_void, ...) -> *mut c_void;
}

// Helper to convert CStr to *const i8
fn cstr_ptr(s: &CStr) -> *const i8 {
    s.as_ptr()
}

/// Check the current microphone authorization status
///
/// Returns the current permission state without triggering a dialog.
pub fn check_microphone_authorization_status() -> AVAuthorizationStatus {
    unsafe {
        // Get AVCaptureDevice class
        let class_name = c"AVCaptureDevice";
        let avcapturedevice = objc_getClass(cstr_ptr(class_name));
        if avcapturedevice.is_null() {
            warn!("Failed to get AVCaptureDevice class");
            return AVAuthorizationStatus::NotDetermined;
        }

        // Get selector for authorizationStatusForMediaType:
        let sel_name = c"authorizationStatusForMediaType:";
        let sel = sel_registerName(cstr_ptr(sel_name));

        // Create NSString for media type "soun" (audio)
        let nsstring_class = objc_getClass(cstr_ptr(c"NSString"));
        let string_sel = sel_registerName(cstr_ptr(c"stringWithUTF8String:"));
        let media_type_str = c"soun";
        let media_type: *mut c_void =
            objc_msgSend(nsstring_class, string_sel, cstr_ptr(media_type_str));

        // Call [AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeAudio]
        // The result is an NSInteger (i64 on 64-bit), returned in the pointer.
        // We cast through isize to handle the pointer-to-integer conversion safely.
        let status_ptr = objc_msgSend(avcapturedevice, sel, media_type);
        let status: i32 = (status_ptr as isize) as i32;

        debug!("Microphone authorization status: {:?}", status);
        AVAuthorizationStatus::from(status)
    }
}

/// Request microphone permission from the user
///
/// This function will:
/// 1. Check if permission is already granted
/// 2. If not determined, display the system permission dialog
/// 3. Wait for the user's response (with timeout)
///
/// Returns true if permission is granted, false otherwise.
///
/// ## Important Notes
///
/// - This MUST be called BEFORE any audio capture operations
/// - The permission dialog is modal and blocks UI interaction
/// - If permission was previously denied, returns false immediately
///   (user must manually enable in System Settings)
pub fn request_microphone_permission() -> bool {
    info!("🎤 Checking microphone permission...");

    let current_status = check_microphone_authorization_status();

    match current_status {
        AVAuthorizationStatus::Authorized => {
            info!("✅ Microphone permission already granted");
            return true;
        }
        AVAuthorizationStatus::Denied => {
            warn!("❌ Microphone permission was denied");
            warn!("   Please enable in: System Settings → Privacy & Security → Microphone");
            return false;
        }
        AVAuthorizationStatus::Restricted => {
            warn!("🚫 Microphone access is restricted (parental controls or MDM)");
            return false;
        }
        AVAuthorizationStatus::NotDetermined => {
            info!("📋 Microphone permission not yet determined, requesting...");
        }
    }

    // Request permission - this shows the system dialog
    unsafe {
        // Get AVCaptureDevice class
        let avcapturedevice = objc_getClass(cstr_ptr(c"AVCaptureDevice"));
        if avcapturedevice.is_null() {
            warn!("Failed to get AVCaptureDevice class for permission request");
            return false;
        }

        // Get selector for requestAccessForMediaType:completionHandler:
        // Note: We define but don't use this selector directly since we use
        // an alternative approach via AVCaptureSession to trigger the dialog.
        let _sel = sel_registerName(cstr_ptr(c"requestAccessForMediaType:completionHandler:"));

        // Create NSString for media type "soun" (audio)
        let nsstring_class = objc_getClass(cstr_ptr(c"NSString"));
        let string_sel = sel_registerName(cstr_ptr(c"stringWithUTF8String:"));
        let media_type: *mut c_void = objc_msgSend(nsstring_class, string_sel, cstr_ptr(c"soun"));

        // Create a block for the completion handler
        // This is complex because Objective-C blocks have specific ABI requirements.
        // For simplicity, we'll use a polling approach instead.

        // Call [AVCaptureDevice requestAccessForMediaType:AVMediaTypeAudio completionHandler:nil]
        // Then poll the status
        info!("Showing microphone permission dialog...");

        // Unfortunately, we can't easily pass a Rust closure as an Objective-C block.
        // Instead, we'll trigger the request and then poll for the result.
        // The user will see the dialog and we check the status after.

        // Trigger the permission request (with nil handler - just shows dialog)
        let nil: *mut c_void = std::ptr::null_mut();

        // We need to use a different approach - directly invoke with block
        // For now, let's use a simpler method: just check status after triggering

        // This selector actually requires a block, so let's use a workaround:
        // We can use NSRunLoop to process events while waiting

        // First, trigger the request through a different mechanism
        // by attempting to create an AVCaptureSession - this triggers the dialog

        let session_class = objc_getClass(cstr_ptr(c"AVCaptureSession"));
        let alloc_sel = sel_registerName(cstr_ptr(c"alloc"));
        let init_sel = sel_registerName(cstr_ptr(c"init"));

        if !session_class.is_null() {
            // Create an AVCaptureSession - this triggers the permission dialog
            let session = objc_msgSend(session_class, alloc_sel);
            let session = objc_msgSend(session, init_sel);

            if !session.is_null() {
                // Try to add an audio input device - this triggers the permission request
                let device_sel = sel_registerName(cstr_ptr(c"defaultDeviceWithMediaType:"));
                let audio_device = objc_msgSend(avcapturedevice, device_sel, media_type);

                if !audio_device.is_null() {
                    // Create device input
                    let input_class = objc_getClass(cstr_ptr(c"AVCaptureDeviceInput"));
                    let input_sel = sel_registerName(cstr_ptr(c"deviceInputWithDevice:error:"));

                    // Try to create input - this is what triggers the permission dialog
                    let _input = objc_msgSend(input_class, input_sel, audio_device, nil);
                }

                // Note: Modern Objective-C uses ARC, so explicit release is not needed
                // and would cause issues with ARC-managed objects.
            }
        }
    }

    // Wait for user response by polling the status
    // The dialog is asynchronous, so we poll until status changes
    info!("Waiting for user response to microphone permission dialog...");

    let timeout = Duration::from_secs(60); // 60 second timeout
    let poll_interval = Duration::from_millis(500);
    let start = std::time::Instant::now();

    loop {
        std::thread::sleep(poll_interval);

        let status = check_microphone_authorization_status();
        match status {
            AVAuthorizationStatus::Authorized => {
                info!("✅ Microphone permission granted by user");
                return true;
            }
            AVAuthorizationStatus::Denied => {
                warn!("❌ Microphone permission denied by user");
                warn!("   The daemon will continue, but voice dictation will not work");
                warn!("   Enable in: System Settings → Privacy & Security → Microphone");
                return false;
            }
            AVAuthorizationStatus::NotDetermined => {
                // Still waiting for user response
                if start.elapsed() > timeout {
                    warn!("⏱️  Microphone permission request timed out");
                    warn!("   Please respond to the permission dialog");
                    return false;
                }
                // Continue polling
            }
            AVAuthorizationStatus::Restricted => {
                warn!("🚫 Microphone access became restricted");
                return false;
            }
        }
    }
}

/// Check if microphone permission is currently granted
///
/// Returns true only if authorization status is Authorized.
#[allow(dead_code)]
pub fn has_microphone_permission() -> bool {
    matches!(
        check_microphone_authorization_status(),
        AVAuthorizationStatus::Authorized
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that AVAuthorizationStatus conversion works correctly.
    #[test]
    fn test_authorization_status_from_i32() {
        assert_eq!(
            AVAuthorizationStatus::from(0),
            AVAuthorizationStatus::NotDetermined
        );
        assert_eq!(
            AVAuthorizationStatus::from(1),
            AVAuthorizationStatus::Restricted
        );
        assert_eq!(
            AVAuthorizationStatus::from(2),
            AVAuthorizationStatus::Denied
        );
        assert_eq!(
            AVAuthorizationStatus::from(3),
            AVAuthorizationStatus::Authorized
        );
        // Unknown values should default to NotDetermined
        assert_eq!(
            AVAuthorizationStatus::from(99),
            AVAuthorizationStatus::NotDetermined
        );
        assert_eq!(
            AVAuthorizationStatus::from(-1),
            AVAuthorizationStatus::NotDetermined
        );
    }

    /// Test that the Objective-C runtime FFI bindings work.
    /// This tests the basic FFI plumbing without requiring audio entitlements.
    #[test]
    fn test_objc_runtime_bindings() {
        unsafe {
            // Test that we can look up Objective-C classes
            let nsstring_class = objc_getClass(cstr_ptr(c"NSString"));
            assert!(!nsstring_class.is_null(), "NSString class should exist");

            let nsobject_class = objc_getClass(cstr_ptr(c"NSObject"));
            assert!(!nsobject_class.is_null(), "NSObject class should exist");

            // Test that AVCaptureDevice class exists (doesn't require entitlements to look up)
            let avcapturedevice = objc_getClass(cstr_ptr(c"AVCaptureDevice"));
            assert!(
                !avcapturedevice.is_null(),
                "AVCaptureDevice class should exist"
            );

            // Test that we can register selectors
            let sel = sel_registerName(cstr_ptr(c"description"));
            assert!(
                !sel.is_null(),
                "description selector should be registerable"
            );

            let sel2 = sel_registerName(cstr_ptr(c"authorizationStatusForMediaType:"));
            assert!(
                !sel2.is_null(),
                "authorizationStatusForMediaType: selector should be registerable"
            );
        }
    }

    /// Test that checking authorization status doesn't crash.
    /// Note: The actual status depends on system state, but the call should not crash.
    /// This works because checking status doesn't require the audio-input entitlement,
    /// only actually accessing the microphone does.
    #[test]
    fn test_check_authorization_status_no_crash() {
        // This should not crash, even without entitlements
        // The status will vary based on system state
        let status = check_microphone_authorization_status();
        println!("Current microphone authorization status: {:?}", status);

        // Verify it's one of the valid values
        assert!(matches!(
            status,
            AVAuthorizationStatus::NotDetermined
                | AVAuthorizationStatus::Restricted
                | AVAuthorizationStatus::Denied
                | AVAuthorizationStatus::Authorized
        ));
    }

    /// Test has_microphone_permission returns a boolean without crashing.
    #[test]
    fn test_has_microphone_permission_no_crash() {
        // This should not crash, even without entitlements
        let result = has_microphone_permission();
        println!("Has microphone permission: {}", result);
        // The function executed successfully if we get here
        // Result depends on system state (true if authorized, false otherwise)
        drop(result); // Silence unused warning while proving the call succeeded
    }
}
