# macOS TCC Permissions Research for Swictation

**Date:** 2025-11-29
**Project:** Swictation Daemon
**Research Focus:** Programmatic TCC permission requests for microphone and accessibility

---

## Executive Summary

This document analyzes the current state of macOS TCC (Transparency, Consent, and Control) permission handling in Swictation and provides recommendations for proper implementation.

**Current State:** ✅ Accessibility partially implemented, ❌ Microphone permission NOT requested
**Critical Gap:** Bare binary daemon lacks Info.plist for TCC compliance
**Recommended Solution:** Bundle daemon as .app bundle with proper Info.plist

---

## 1. Current State Analysis

### ✅ What Exists

#### Accessibility Permission Handling
**File:** `/opt/swictation/rust-crates/swictation-daemon/src/macos_text_inject.rs`

```rust
// Current implementation uses FFI to check/request accessibility
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
}

pub fn check_accessibility_permissions() -> bool {
    unsafe { AXIsProcessTrusted() }
}

pub fn request_accessibility_permissions() -> bool {
    // Creates CFDictionary with kAXTrustedCheckOptionPrompt = true
    // Shows system dialog: "Open System Settings" / "Deny"
    // User must manually enable in System Settings
}
```

**Status:** ✅ **Working** - Shows permission dialog at startup (lines 273-293 in main.rs)

**Verification:**
```bash
# Currently called in main.rs:276-289
#[cfg(target_os = "macos")]
{
    if !MacOSTextInjector::request_accessibility_permissions() {
        warn!("⚠️  Accessibility permission not yet granted");
        warn!("   Please enable in: System Settings → Privacy & Security → Accessibility");
    }
}
```

#### Microphone Permission Awareness
**File:** `/opt/swictation/rust-crates/swictation-daemon/src/macos_audio_permission.rs`

**Status:** ⚠️ **EXISTS BUT NOT CALLED** - Module implements AVFoundation permission request, but is:
- NOT imported in main.rs
- NOT called at startup
- NOT exported in lib.rs

**Code Analysis:**
```rust
// Lines 103-265: Complete implementation using AVFoundation
// - check_microphone_authorization_status() -> AVAuthorizationStatus
// - request_microphone_permission() -> bool
// - has_microphone_permission() -> bool

// Uses AVCaptureDevice.requestAccessForMediaType: via FFI
// Polls for user response with 60-second timeout
// Provides helpful error messages
```

**Issue:** No usage string in Info.plist, so TCC will show generic message.

---

## 2. What's Missing

### ❌ Critical Gaps

#### 1. No Info.plist for Bare Binary
**Problem:** The daemon binary (`swictation-daemon-macos`) is deployed as a **bare executable**, not bundled as a `.app`.

**Impact:**
- ❌ Cannot define `NSMicrophoneUsageDescription` (required for permission prompt)
- ❌ Cannot define `NSAccessibilityUsageDescription` (for better UX)
- ❌ TCC shows generic/confusing permission messages
- ❌ System Settings UI may not display bare binary properly ([source](https://github.com/koekeishiya/yabai/issues/2688))
- ❌ Cannot be identified by bundle ID (uses full path instead) ([source](https://stackoverflow.com/questions/62158598/can-i-use-macos-tccutil-to-reset-screenrecording-permissions-for-a-daemon-that-h))

**Evidence:**
```bash
# Current LaunchAgent configuration
<key>ProgramArguments</key>
<array>
    <string>{{DAEMON_PATH}}</string>  # Points to bare binary
</array>
```

From web research: "TCC does NOT consult embedded Info.plist files in single-file binaries, and does not identify bare binaries by bundle-ID, but rather by their full-path." ([source](https://stackoverflow.com/questions/62158598/can-i-use-macos-tccutil-to-reset-screenrecording-permissions-for-a-daemon-that-h))

#### 2. Microphone Permission Not Requested
**File:** `/opt/swictation/rust-crates/swictation-daemon/src/main.rs`

**Lines 273-293:** Only requests Accessibility permission, NOT microphone.

```rust
#[cfg(target_os = "macos")]
{
    // ✅ DOES request accessibility
    if !MacOSTextInjector::request_accessibility_permissions() {
        warn!("⚠️  Accessibility permission not yet granted");
    }

    // ❌ DOES NOT request microphone permission
    // macos_audio_permission module exists but is not imported or used!
}
```

**Evidence:**
```bash
$ grep -n "macos_audio_permission" /opt/swictation/rust-crates/swictation-daemon/src/main.rs
# No results - module never imported!

$ grep -n "mod macos_audio_permission" /opt/swictation/rust-crates/swictation-daemon/src/*.rs
# Only defined in macos_audio_permission.rs itself
```

#### 3. No Entitlements File
**Status:** ❌ **Missing**

While entitlements are not strictly required for microphone/accessibility on LaunchAgents, they are recommended for:
- App sandboxing (future-proofing)
- Hardened runtime (Gatekeeper/notarization)
- Audio input entitlement (`com.apple.security.device.audio-input`)

**Evidence:**
```bash
$ find /opt/swictation -name "*.entitlements"
# No results
```

#### 4. No Usage Description Strings
**Required Keys for Info.plist:**

```xml
<!-- MISSING: Required for microphone permission dialog -->
<key>NSMicrophoneUsageDescription</key>
<string>Swictation needs microphone access to transcribe your voice into text.</string>

<!-- MISSING: Improves accessibility permission dialog (optional but recommended) -->
<key>NSAccessibilityUsageDescription</key>
<string>Swictation needs accessibility permission to type transcribed text into applications.</string>
```

**Current State:** None of these exist. The daemon has NO Info.plist whatsoever.

---

## 3. How macOS TCC Works

### Permission Request Flow

```
┌─────────────────────────────────────────────────────────┐
│ 1. Application Attempts Protected API Access           │
│    (AVCaptureDevice, AXIsProcessTrusted, etc.)         │
└─────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────┐
│ 2. TCC Framework Checks Database                       │
│    Location: ~/Library/Application Support/            │
│              com.apple.TCC/TCC.db                      │
└─────────────────────────────────────────────────────────┘
                         ↓
         ┌───────────────┴───────────────┐
         ↓                               ↓
┌──────────────────┐          ┌──────────────────────┐
│ Permission       │          │ Permission           │
│ Already Granted  │          │ Not Determined       │
│ → Allow Access   │          │ → Show Dialog        │
└──────────────────┘          └──────────────────────┘
                                        ↓
                         ┌──────────────────────────┐
                         │ TCC Looks for Info.plist │
                         │ Reads Usage Description  │
                         └──────────────────────────┘
                                        ↓
                         ┌──────────────────────────┐
                         │ Shows Permission Dialog  │
                         │ with Usage String        │
                         └──────────────────────────┘
```

### Bundle vs Bare Binary Behavior

| Aspect | .app Bundle | Bare Binary (Current) |
|--------|-------------|----------------------|
| **TCC Identification** | Bundle ID (`com.swictation.daemon`) | Full path (`/Users/x/.npm-global/...`) |
| **Info.plist** | Automatically loaded | ❌ NOT loaded ([source](https://chrispaynter.medium.com/what-to-do-when-your-macos-daemon-gets-blocked-by-tcc-dialogues-d3a1b991151f)) |
| **Usage Strings** | From Info.plist | ❌ Generic/missing |
| **System Settings UI** | Shows app name/icon | May not display properly ([source](https://github.com/koekeishiya/yabai/issues/2688)) |
| **tccutil Reset** | `tccutil reset Microphone com.swictation.daemon` | Must use full path (awkward) |
| **Permission Inheritance** | Helper tools inherit app permissions | N/A |

**Key Finding:** "If your product uses a script as its main executable, you're likely to encounter TCC problems. The supported way to grant permissions is through System Settings → Privacy & Security. However, the interface may not let you select bare executables, and the direct solution to that is simply embed your daemon executable inside an app bundle." ([source](https://stackoverflow.com/questions/75918835/how-to-adjust-what-is-displayed-in-the-login-items-list-for-my-launch-daemon-age))

---

## 4. LaunchAgent vs LaunchDaemon Considerations

### Current Configuration
**Type:** LaunchAgent (user-level, GUI session)
**Location:** `~/Library/LaunchAgents/com.swictation.daemon.plist`
**Session:** `Aqua` (GUI session required)

```xml
<!-- Current plist configuration -->
<key>LimitLoadToSessionType</key>
<string>Aqua</string>

<key>ProcessType</key>
<string>Adaptive</string>
```

### Why LaunchAgent is Correct

✅ **Pros (Current Choice):**
- Runs in user session (has GUI access)
- Can show TCC permission dialogs
- Can access CoreML/Metal (GPU)
- Can access user's microphone
- Can inject keyboard events via Accessibility API
- Proper for user-facing app

❌ **LaunchDaemon Would Not Work:**
- Runs as root before GUI loads
- **Cannot show TCC prompts** ([source](https://chrispaynter.medium.com/what-to-do-when-your-macos-daemon-gets-blocked-by-tcc-dialogues-d3a1b991151f))
- No access to user's audio devices
- No access to user's keyboard/accessibility

**Quote from Research:** "Daemons are run as the root user, which does not have a GUI session. Daemons are typically loaded by the root user before the GUI is even loaded. If the code being run is accessing TCC-protected SDKs, there's no way for macOS to prompt the user for permission. A common workaround is that 'the agent triggers the TCC prompt, and at some point later on the daemon takes over.'" ([source](https://chrispaynter.medium.com/what-to-do-when-your-macos-daemon-gets-blocked-by-tcc-dialogues-d3a1b991151f))

**Verdict:** ✅ Current LaunchAgent configuration is correct for this use case.

---

## 5. Rust Implementation Details

### Working Examples from Codebase

#### Accessibility Permission (Currently Working)

**File:** `/opt/swictation/rust-crates/swictation-daemon/src/macos_text_inject.rs`

**Dependencies:**
```toml
[target.'cfg(target_os = "macos")'.dependencies]
core-foundation = "0.9"
core-graphics = "0.23"
foreign-types-shared = "0.3"
```

**Implementation:**
```rust
use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
}

pub fn request_accessibility_permissions() -> bool {
    // Create options dictionary with prompt = true
    let key = CFString::new("AXTrustedCheckOptionPrompt");
    let value = CFBoolean::true_value();
    let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), value.as_CFType())]);

    // Show system dialog
    let is_trusted = unsafe {
        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef() as *const c_void)
    };

    is_trusted
}
```

**Result:** ✅ Shows dialog → "swictation-daemon-macos would like to control this computer using accessibility features."

#### Microphone Permission (Exists But Not Used)

**File:** `/opt/swictation/rust-crates/swictation-daemon/src/macos_audio_permission.rs`

**Dependencies:**
```toml
[target.'cfg(target_os = "macos")'.dependencies]
objc = "0.2"  # Already in Cargo.toml
```

**Implementation:**
```rust
#[link(name = "AVFoundation", kind = "framework")]
extern "C" {}

#[link(name = "objc", kind = "dylib")]
extern "C" {
    fn objc_getClass(name: *const i8) -> *mut c_void;
    fn sel_registerName(name: *const i8) -> *mut c_void;
    fn objc_msgSend(obj: *mut c_void, sel: *mut c_void, ...) -> *mut c_void;
}

pub fn request_microphone_permission() -> bool {
    // 1. Check current status
    let status = check_microphone_authorization_status();

    // 2. If already granted, return true
    if status == AVAuthorizationStatus::Authorized {
        return true;
    }

    // 3. Trigger permission request by creating AVCaptureSession
    // This shows the system dialog
    unsafe {
        let session_class = objc_getClass(b"AVCaptureSession\0".as_ptr() as *const i8);
        let session = objc_msgSend(session_class, sel_registerName(b"alloc\0".as_ptr() as *const i8));
        let session = objc_msgSend(session, sel_registerName(b"init\0".as_ptr() as *const i8));

        // Attempt to add audio input - triggers permission dialog
        // ...
    }

    // 4. Poll for user response (60 second timeout)
    for _ in 0..120 {
        std::thread::sleep(Duration::from_millis(500));
        if check_microphone_authorization_status() == AVAuthorizationStatus::Authorized {
            return true;
        }
    }

    false
}
```

**Current Problem:** This code exists and is well-implemented, but:
1. ❌ Never imported in main.rs
2. ❌ Never called at startup
3. ❌ No Info.plist usage string → shows generic message

---

## 6. Recommended Solutions

### Solution 1: Bundle Daemon as .app (Recommended)

**Why This is Best:**
- ✅ Proper TCC compliance
- ✅ Can define usage description strings
- ✅ Better System Settings UI integration
- ✅ Industry standard approach
- ✅ Enables future features (app icon, preferences, etc.)

**Implementation:**

#### Step 1: Create .app Bundle Structure
```
swictation-daemon.app/
├── Contents/
    ├── Info.plist                  # NEW: TCC usage strings
    ├── MacOS/
    │   └── swictation-daemon       # Existing binary (renamed)
    ├── Resources/
    │   └── AppIcon.icns            # Optional: App icon
    └── _CodeSignature/             # Future: Code signing
```

#### Step 2: Create Info.plist
**File:** `/opt/swictation/rust-crates/swictation-daemon/resources/macos/Info.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <!-- Bundle identification -->
    <key>CFBundleIdentifier</key>
    <string>com.swictation.daemon</string>

    <key>CFBundleName</key>
    <string>Swictation Daemon</string>

    <key>CFBundleDisplayName</key>
    <string>Swictation</string>

    <key>CFBundleVersion</key>
    <string>0.7.5</string>

    <key>CFBundleShortVersionString</key>
    <string>0.7.5</string>

    <key>CFBundleExecutable</key>
    <string>swictation-daemon</string>

    <key>CFBundlePackageType</key>
    <string>APPL</string>

    <!-- TCC USAGE DESCRIPTIONS (CRITICAL) -->

    <!-- Microphone permission (for voice transcription) -->
    <key>NSMicrophoneUsageDescription</key>
    <string>Swictation needs microphone access to transcribe your voice into text for hands-free dictation.</string>

    <!-- Accessibility permission (for text injection) -->
    <key>NSAccessibilityUsageDescription</key>
    <string>Swictation needs accessibility permission to type transcribed text into your applications.</string>

    <!-- Background modes -->
    <key>LSBackgroundOnly</key>
    <false/>  <!-- False = can have UI, True = pure background -->

    <key>LSUIElement</key>
    <true/>  <!-- True = no dock icon (agent-style) -->

    <!-- Minimum macOS version -->
    <key>LSMinimumSystemVersion</key>
    <string>14.0</string>

    <!-- Process type -->
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.utilities</string>
</dict>
</plist>
```

#### Step 3: Update build.rs to Create Bundle
**File:** `/opt/swictation/rust-crates/swictation-daemon/build.rs`

```rust
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());

    // Only create .app bundle for macOS builds
    if target.contains("apple-darwin") {
        create_macos_app_bundle();
    }

    // ... existing build script code ...
}

fn create_macos_app_bundle() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let profile = env::var("PROFILE").unwrap();
    let target_dir = PathBuf::from(&out_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let app_name = "swictation-daemon.app";
    let app_bundle = target_dir.join(&profile).join(app_name);

    // Create bundle structure
    let contents = app_bundle.join("Contents");
    let macos = contents.join("MacOS");
    let resources = contents.join("Resources");

    fs::create_dir_all(&macos).unwrap();
    fs::create_dir_all(&resources).unwrap();

    // Copy Info.plist from resources
    let info_plist_src = PathBuf::from("resources/macos/Info.plist");
    let info_plist_dst = contents.join("Info.plist");

    if info_plist_src.exists() {
        fs::copy(&info_plist_src, &info_plist_dst).unwrap();
        println!("cargo:rerun-if-changed=resources/macos/Info.plist");
    } else {
        eprintln!("Warning: Info.plist not found at {:?}", info_plist_src);
    }

    println!("cargo:warning=Created .app bundle at {:?}", app_bundle);
}
```

#### Step 4: Update LaunchAgent plist
**File:** `/opt/swictation/npm-package/templates/macos/com.swictation.daemon.plist`

```xml
<!-- CHANGE: Point to .app bundle instead of bare binary -->
<key>ProgramArguments</key>
<array>
    <string>{{DAEMON_PATH}}/Contents/MacOS/swictation-daemon</string>
</array>

<!-- NEW: Set bundle identifier for TCC -->
<key>ProcessBundleIdentifier</key>
<string>com.swictation.daemon</string>
```

#### Step 5: Enable Microphone Permission Request
**File:** `/opt/swictation/rust-crates/swictation-daemon/src/main.rs`

```rust
// Add module import (line ~19)
#[cfg(target_os = "macos")]
mod macos_audio_permission;

// In main() function, after accessibility check (line ~293)
#[cfg(target_os = "macos")]
{
    use crate::macos_text_inject::MacOSTextInjector;
    use crate::macos_audio_permission;  // NEW

    info!("🔐 Checking macOS permissions...");

    // Request Accessibility permission
    if !MacOSTextInjector::request_accessibility_permissions() {
        warn!("⚠️  Accessibility permission not yet granted");
        warn!("   Please enable in: System Settings → Privacy & Security → Accessibility");
    } else {
        info!("✅ Accessibility permission granted");
    }

    // NEW: Request Microphone permission
    if !macos_audio_permission::request_microphone_permission() {
        warn!("⚠️  Microphone permission not yet granted");
        warn!("   Please enable in: System Settings → Privacy & Security → Microphone");
        warn!("   The daemon will continue, but voice dictation will not work");
    } else {
        info!("✅ Microphone permission granted");
    }
}
```

---

### Solution 2: Embed Info.plist in Binary (Alternative)

**Why This Might Work:**
- Simpler build process
- No .app bundle needed
- Uses linker to embed Info.plist

**How It Works:**
Xcode allows embedding Info.plist in the `__TEXT` segment using linker flags.

**Implementation:**

#### Update build.rs
```rust
fn main() {
    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());

    if target.contains("apple-darwin") {
        embed_info_plist_in_binary();
    }
}

fn embed_info_plist_in_binary() {
    // Tell cargo to pass linker flags
    println!("cargo:rustc-link-arg=-Wl,-sectcreate,__TEXT,__info_plist,resources/macos/Info.plist");
}
```

**Limitations:**
- ⚠️ Non-standard approach
- ⚠️ May not work with TCC on recent macOS versions
- ⚠️ System Settings UI issues persist
- ⚠️ Not recommended by Apple

**Research Quote:** "You can embed the Info.plist directly into the binary, which opens up features requiring a bundle identifier. In Xcode, enable the 'Create Info.plist Section in Binary' target setting, or manually add the `--sectcreate __info_plist` option during linking." ([source](https://chrispaynter.medium.com/what-to-do-when-your-macos-daemon-gets-blocked-by-tcc-dialogues-d3a1b991151f))

**Verdict:** ⚠️ Works as a hack, but Solution 1 (.app bundle) is strongly recommended.

---

### Solution 3: Hybrid Approach (Not Recommended)

Use a small .app bundle GUI helper to request permissions, then launch the daemon.

**Why This Exists:**
Some developers use a GUI helper app to request TCC permissions, then have it launch a bare daemon.

**Problems:**
- ❌ More complex architecture
- ❌ Two separate processes to maintain
- ❌ Daemon still runs without bundle benefits
- ❌ Permission inheritance unclear

**Verdict:** ❌ Unnecessary complexity for this project.

---

## 7. Implementation Checklist

### Phase 1: Create .app Bundle (High Priority)
- [ ] Create `/opt/swictation/rust-crates/swictation-daemon/resources/macos/` directory
- [ ] Write `Info.plist` with NSMicrophoneUsageDescription and NSAccessibilityUsageDescription
- [ ] Update `build.rs` to create .app bundle structure
- [ ] Test: `cargo build --release --target aarch64-apple-darwin`
- [ ] Verify: `ls -la target/release/swictation-daemon.app/Contents/`

### Phase 2: Enable Microphone Permission (High Priority)
- [ ] Add `mod macos_audio_permission;` to main.rs
- [ ] Export module in lib.rs for testing
- [ ] Call `request_microphone_permission()` in main() at startup
- [ ] Test: Run daemon and verify permission dialog appears
- [ ] Verify: Check System Settings → Privacy → Microphone shows Swictation

### Phase 3: Update npm Package (Medium Priority)
- [ ] Update postinstall.js to deploy .app bundle instead of bare binary
- [ ] Update LaunchAgent plist template to point to .app/Contents/MacOS/daemon
- [ ] Add `ProcessBundleIdentifier` key to plist
- [ ] Test: `npm pack` and install on clean macOS system
- [ ] Verify: `launchctl list | grep swictation` shows bundle ID

### Phase 4: Add Entitlements (Low Priority - Future)
- [ ] Create `entitlements.plist` with audio input entitlement
- [ ] Update build process to apply entitlements
- [ ] Research code signing for distribution
- [ ] Plan for Apple Developer ID signing

### Phase 5: Documentation (Medium Priority)
- [ ] Update README with permission requirements
- [ ] Update macOS setup guide with TCC instructions
- [ ] Add troubleshooting section for permission issues
- [ ] Create screenshots of permission dialogs

---

## 8. Testing Plan

### Manual Testing Steps

#### Test 1: Permission Dialogs Appear
```bash
# 1. Build daemon as .app bundle
cd /opt/swictation/rust-crates/swictation-daemon
cargo build --release --target aarch64-apple-darwin

# 2. Reset TCC permissions for testing
tccutil reset Microphone com.swictation.daemon
tccutil reset Accessibility com.swictation.daemon

# 3. Run daemon
./target/release/swictation-daemon.app/Contents/MacOS/swictation-daemon

# 4. Verify permission dialogs appear:
#    - First dialog: Accessibility permission with usage string
#    - Second dialog: Microphone permission with usage string

# 5. Check System Settings
#    Settings → Privacy & Security → Accessibility
#    Should show "Swictation" (not bare path)
#
#    Settings → Privacy & Security → Microphone
#    Should show "Swictation" (not bare path)
```

#### Test 2: LaunchAgent Integration
```bash
# 1. Install via npm (will deploy .app bundle)
npm pack
npm install -g ./swictation-*.tgz

# 2. Start via LaunchAgent
swictation start

# 3. Verify running as .app bundle
ps aux | grep swictation-daemon
# Should show: .../swictation-daemon.app/Contents/MacOS/swictation-daemon

# 4. Check logs for permission status
tail -f ~/Library/Logs/swictation/daemon.log | grep permission
# Should show: "✅ Accessibility permission granted"
#              "✅ Microphone permission granted"
```

#### Test 3: TCC Database Verification
```bash
# Check TCC database entries
sqlite3 ~/Library/Application\ Support/com.apple.TCC/TCC.db \
  "SELECT service, client, auth_value FROM access WHERE client LIKE '%swictation%';"

# Expected output:
# kTCCServiceAccessibility|com.swictation.daemon|2
# kTCCServiceMicrophone|com.swictation.daemon|2
#
# auth_value = 2 means "Allowed"
```

### Automated Testing

#### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "macos")]
    fn test_accessibility_permission_check() {
        // Just verify the function can be called
        let _ = macos_text_inject::check_accessibility_permissions();
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_microphone_permission_check() {
        // Just verify the function can be called
        let _ = macos_audio_permission::check_microphone_authorization_status();
    }

    #[test]
    fn test_app_bundle_info_plist_exists() {
        let plist_path = "resources/macos/Info.plist";
        assert!(std::path::Path::new(plist_path).exists(),
                "Info.plist must exist at {}", plist_path);
    }
}
```

---

## 9. Known Issues & Workarounds

### Issue 1: cpal Triggers Microphone Permission
**Problem:** The cpal audio library triggers microphone permission when creating audio devices.

**Reference:** [RustAudio/cpal#901](https://github.com/RustAudio/cpal/issues/901)

**Current Behavior:**
```rust
// When swictation-audio calls cpal::default_input_device()
// macOS automatically shows microphone permission dialog
// This happens BEFORE our explicit request
```

**Impact:**
- ⚠️ Permission dialog may appear at wrong time
- ⚠️ Without Info.plist usage string, shows generic message

**Workaround:** Our explicit `request_microphone_permission()` call BEFORE audio initialization ensures:
1. Usage string from Info.plist is shown
2. We control timing of permission request
3. Better error handling and user messaging

### Issue 2: macOS Ventura Permission Toggle Bug
**Problem:** On macOS 13.0+, rapidly toggling permissions in System Settings may cause stale cache.

**Reference:** From macos_text_inject.rs comments (lines 104-109)

**Symptoms:**
- Permission check returns false even after granting
- Must toggle OFF and back ON in System Settings
- Restart application

**Workaround:**
```rust
// Current code already handles this:
pub fn check_accessibility_permissions() -> bool {
    // WARNING: On macOS Ventura 13.0+, this function may return incorrect values
    // if permissions are rapidly toggled.
    unsafe { AXIsProcessTrusted() }
}
```

**User Instructions:** Already documented in warning messages.

### Issue 3: System Settings UI for Bare Binaries
**Problem:** macOS System Settings may not properly display bare binaries in permission lists.

**Reference:** [yabai Issue #2688](https://github.com/koekeishiya/yabai/issues/2688)

**Solution:** ✅ Use .app bundle (Solution 1) to fix this.

---

## 10. Additional Resources

### Apple Developer Documentation
- [Requesting Authorization for Media Capture on macOS](https://developer.apple.com/documentation/bundleresources/information_property_list/protected_resources/requesting_authorization_for_media_capture_on_macos?language=objc)
- [Creating Launch Daemons and Agents](https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/CreatingLaunchdJobs.html)

### Web Research Sources
- [tauri-plugin-macos-permissions](https://crates.io/crates/tauri-plugin-macos-permissions) - Rust crate for TCC permissions
- [macOS TCC Bare Binaries](https://stackoverflow.com/questions/62158598/can-i-use-macos-tccutil-to-reset-screenrecording-permissions-for-a-daemon-that-h)
- [What to do when your macOS daemon gets blocked by TCC](https://chrispaynter.medium.com/what-to-do-when-your-macos-daemon-gets-blocked-by-tcc-dialogues-d3a1b991151f)
- [System Settings UI not showing binary TCC entries](https://github.com/koekeishiya/yabai/issues/2688)
- [Accessibility Permission in macOS](https://jano.dev/apple/macos/swift/2025/01/08/Accessibility-Permission.html)
- [Full Transparency: Controlling Apple's TCC (Part 2)](https://www.huntress.com/blog/full-transparency-controlling-apples-tcc-part-ii)

### Rust Crates
- `core-foundation` - Core Foundation bindings (already in use)
- `core-graphics` - Core Graphics bindings (already in use)
- `objc` - Objective-C runtime (already in use)
- `cocoa` - Higher-level Cocoa bindings (optional, not needed)
- `tauri-plugin-macos-permissions` - Pre-built TCC handling (could be used as reference)

---

## 11. Conclusion

### Summary of Findings

**Current State:**
- ✅ Accessibility permission request implemented and working
- ❌ Microphone permission request implemented but NOT used
- ❌ No Info.plist → generic TCC messages
- ❌ Bare binary deployment → poor System Settings integration

**Recommended Path Forward:**
1. **HIGH PRIORITY:** Bundle daemon as .app with Info.plist (Solution 1)
2. **HIGH PRIORITY:** Enable microphone permission request in main.rs
3. **MEDIUM PRIORITY:** Update npm package to deploy .app bundle
4. **LOW PRIORITY:** Add entitlements and code signing (future enhancement)

**Estimated Effort:**
- Creating .app bundle structure: 2-3 hours
- Updating build.rs and npm scripts: 2-3 hours
- Testing on macOS: 1-2 hours
- Documentation updates: 1 hour
- **Total:** ~6-9 hours

**Risk Assessment:**
- **Low Risk:** Changes are isolated to build process and packaging
- **No Code Changes:** Core Rust code already implements permissions correctly
- **Rollback Easy:** Can revert to bare binary if issues arise
- **Well-Documented:** Industry-standard approach with many examples

### Next Steps

1. Review this document with team
2. Approve Solution 1 (.app bundle approach)
3. Create implementation branch
4. Follow Phase 1-5 checklist
5. Test on fresh macOS install
6. Update documentation
7. Release as minor version bump (v0.8.0)

---

**End of Research Document**
