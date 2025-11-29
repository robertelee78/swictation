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

use std::os::raw::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;
use tracing::{debug, info, warn};

// AVFoundation media type for audio
// This is the string "soun" (audio) used by AVCaptureDevice
const AV_MEDIA_TYPE_AUDIO: &str = "soun";

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

/// Check the current microphone authorization status
///
/// Returns the current permission state without triggering a dialog.
pub fn check_microphone_authorization_status() -> AVAuthorizationStatus {
    unsafe {
        // Get AVCaptureDevice class
        let class_name = b"AVCaptureDevice\0";
        let avcapturedevice = objc_getClass(class_name.as_ptr() as *const i8);
        if avcapturedevice.is_null() {
            warn!("Failed to get AVCaptureDevice class");
            return AVAuthorizationStatus::NotDetermined;
        }

        // Get selector for authorizationStatusForMediaType:
        let sel_name = b"authorizationStatusForMediaType:\0";
        let sel = sel_registerName(sel_name.as_ptr() as *const i8);

        // Create NSString for media type "soun" (audio)
        let nsstring_class_name = b"NSString\0";
        let nsstring_class = objc_getClass(nsstring_class_name.as_ptr() as *const i8);
        let string_sel = sel_registerName(b"stringWithUTF8String:\0".as_ptr() as *const i8);
        let media_type_str = b"soun\0";
        let media_type: *mut c_void =
            objc_msgSend(nsstring_class, string_sel, media_type_str.as_ptr());

        // Call [AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeAudio]
        let status: i32 = std::mem::transmute(objc_msgSend(avcapturedevice, sel, media_type));

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
    let (tx, rx) = mpsc::channel();
    let granted = AtomicBool::new(false);

    unsafe {
        // Get AVCaptureDevice class
        let class_name = b"AVCaptureDevice\0";
        let avcapturedevice = objc_getClass(class_name.as_ptr() as *const i8);
        if avcapturedevice.is_null() {
            warn!("Failed to get AVCaptureDevice class for permission request");
            return false;
        }

        // Get selector for requestAccessForMediaType:completionHandler:
        let sel_name = b"requestAccessForMediaType:completionHandler:\0";
        let sel = sel_registerName(sel_name.as_ptr() as *const i8);

        // Create NSString for media type "soun" (audio)
        let nsstring_class_name = b"NSString\0";
        let nsstring_class = objc_getClass(nsstring_class_name.as_ptr() as *const i8);
        let string_sel = sel_registerName(b"stringWithUTF8String:\0".as_ptr() as *const i8);
        let media_type_str = b"soun\0";
        let media_type: *mut c_void =
            objc_msgSend(nsstring_class, string_sel, media_type_str.as_ptr());

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

        let session_class_name = b"AVCaptureSession\0";
        let session_class = objc_getClass(session_class_name.as_ptr() as *const i8);
        let alloc_sel = sel_registerName(b"alloc\0".as_ptr() as *const i8);
        let init_sel = sel_registerName(b"init\0".as_ptr() as *const i8);

        if !session_class.is_null() {
            // Create an AVCaptureSession - this triggers the permission dialog
            let session = objc_msgSend(session_class, alloc_sel);
            let session = objc_msgSend(session, init_sel);

            if !session.is_null() {
                // Try to add an audio input device - this triggers the permission request
                let device_sel =
                    sel_registerName(b"defaultDeviceWithMediaType:\0".as_ptr() as *const i8);
                let audio_device = objc_msgSend(avcapturedevice, device_sel, media_type);

                if !audio_device.is_null() {
                    // Create device input
                    let input_class_name = b"AVCaptureDeviceInput\0";
                    let input_class = objc_getClass(input_class_name.as_ptr() as *const i8);
                    let input_sel =
                        sel_registerName(b"deviceInputWithDevice:error:\0".as_ptr() as *const i8);

                    // Try to create input - this is what triggers the permission dialog
                    let _input = objc_msgSend(input_class, input_sel, audio_device, nil);
                }

                // Release the session
                let release_sel = sel_registerName(b"release\0".as_ptr() as *const i8);
                // Note: Modern Objective-C uses ARC, so this may not be needed
                // objc_msgSend(session, release_sel);
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
pub fn has_microphone_permission() -> bool {
    matches!(
        check_microphone_authorization_status(),
        AVAuthorizationStatus::Authorized
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_authorization_status() {
        // This test verifies the FFI calls don't crash
        let status = check_microphone_authorization_status();
        println!("Current microphone authorization status: {:?}", status);
    }

    #[test]
    fn test_has_microphone_permission() {
        let has_permission = has_microphone_permission();
        println!("Has microphone permission: {}", has_permission);
    }
}
