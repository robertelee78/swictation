# macOS Support Specification for Swictation

**Version:** 1.0
**Target:** Apple Silicon (M1/M2/M3/M4) macOS 13.0+
**Status:** Planning Phase
**Estimated Effort:** 2-3 weeks

---

## Executive Summary

This specification outlines the requirements and implementation plan to add **macOS Apple Silicon support** to Swictation while maintaining full backward compatibility with existing Linux (x86_64) functionality. The project already has excellent cross-platform architecture via `cpal` (audio), `global-hotkey` (hotkeys), and conditional macOS GPU detection code. The primary work involves implementing macOS-specific text injection, service management, and binary distribution.

**Key Constraints:**
- ✅ Must maintain 100% Linux functionality
- ✅ Target only Apple Silicon (M1+) - no Intel Mac support
- ✅ Use CoreML for GPU acceleration on Apple Silicon
- ✅ Leverage existing cross-platform architecture

---

## Table of Contents

1. [Current State Analysis](#1-current-state-analysis)
2. [Platform Support Matrix](#2-platform-support-matrix)
3. [Implementation Phases](#3-implementation-phases)
4. [Detailed Component Changes](#4-detailed-component-changes)
5. [Build and Distribution](#5-build-and-distribution)
6. [Testing Strategy](#6-testing-strategy)
7. [Documentation Updates](#7-documentation-updates)
8. [Risk Analysis](#8-risk-analysis)

---

## 1. Current State Analysis

### 1.1 Already macOS-Compatible Components ✅

| Component | Library/Crate | Status | Notes |
|-----------|--------------|--------|-------|
| **Audio Capture** | `cpal = "0.15"` | ✅ Ready | Full CoreAudio support on macOS |
| **Global Hotkeys** | `global-hotkey = "0.6"` | ✅ Ready | Cross-platform (X11, Windows, macOS) |
| **GPU Detection** | Custom code in `gpu.rs` | ✅ Ready | Lines 14-102 already implement CoreML detection |
| **Audio Resampling** | `rubato = "0.15"` | ✅ Ready | Pure Rust, platform-agnostic |
| **VAD (Voice Activity)** | Silero VAD ONNX | ✅ Ready | Platform-agnostic ONNX model |
| **STT (Speech-to-Text)** | Parakeet-TDT ONNX | ✅ Ready | Platform-agnostic ONNX model |
| **Text Transform** | MidStream (submodule) | ✅ Ready | Pure Rust, no platform deps |

### 1.2 Components Requiring macOS Implementation 🔧

| Component | Current State | Required Changes |
|-----------|--------------|------------------|
| **Text Injection** | Linux-only (`xdotool`, `wtype`, `ydotool`) | Implement CGEvent API for macOS |
| **Display Server Detection** | X11/Wayland detection | Add Quartz/AppKit detection |
| **Service Management** | systemd user services | Implement launchd services |
| **ONNX Runtime Binaries** | Linux `.so` files | Add macOS `.dylib` files (ARM64) |
| **Build Scripts** | Linux tools (`sha256sum`, `ldd`) | Add macOS tools (`shasum`, `otool`) |
| **npm Package Metadata** | `"os": ["linux"]` | Update to `["linux", "darwin"]` |

---

## 2. Platform Support Matrix

### 2.1 Target Platform

| Attribute | Value |
|-----------|-------|
| **Operating System** | macOS 13.0 (Ventura) or later |
| **Architecture** | ARM64 (Apple Silicon) only |
| **Supported CPUs** | M1, M1 Pro, M1 Max, M1 Ultra, M2, M2 Pro, M2 Max, M2 Ultra, M3, M3 Pro, M3 Max, M4, M4 Pro, M4 Max |
| **GPU Acceleration** | CoreML (Apple Neural Engine) |
| **Minimum RAM** | 8GB (16GB recommended) |
| **Minimum VRAM** | Unified memory (shared with system RAM) |

### 2.2 Explicitly NOT Supported

- ❌ Intel Macs (x86_64)
- ❌ macOS 12.x or earlier
- ❌ Rosetta 2 emulation (ARM64 binaries only)

### 2.3 Feature Parity Table

| Feature | Linux | macOS (M1+) |
|---------|-------|-------------|
| Audio Capture | ✅ PipeWire/ALSA via cpal | ✅ CoreAudio via cpal |
| Global Hotkeys | ✅ X11/Wayland via global-hotkey | ✅ macOS Events via global-hotkey |
| GPU Acceleration | ✅ NVIDIA CUDA | ✅ Apple CoreML/Metal |
| Text Injection | ✅ xdotool/wtype/ydotool | ✅ CGEvent API |
| Service Management | ✅ systemd user services | ✅ launchd agents |
| Startup on Boot | ✅ systemd enable | ✅ launchd RunAtLoad |
| ONNX Runtime | ✅ Linux x86_64 binaries | ✅ macOS ARM64 binaries |
| Secretary Mode | ✅ Full support | ✅ Full support |
| Metrics API | ✅ Unix socket | ✅ Unix socket |
| Tauri UI | ✅ Full support | ✅ Full support |

---

## 3. Implementation Phases

### Phase 1: Core Platform Support (Week 1)

**Objective:** Make the daemon compile and run on macOS with basic functionality.

**Tasks:**
1. Add macOS text injection via CGEvent API
2. Update display server detection for macOS
3. Add conditional compilation for platform-specific code
4. Update Cargo dependencies for macOS
5. Test basic audio → VAD → STT → text injection pipeline

**Deliverables:**
- ✅ Daemon compiles on macOS ARM64
- ✅ Basic dictation works (hotkey → speak → text appears)
- ✅ GPU detection recognizes Apple Silicon

**Estimated Time:** 4-5 days

---

### Phase 2: Service Management & Distribution (Week 2)

**Objective:** Implement macOS service management and binary distribution.

**Tasks:**
1. Create launchd plist templates
2. Update `postinstall.js` for macOS detection
3. Download and bundle macOS ONNX Runtime binaries
4. Update build scripts for macOS
5. Test installation flow on macOS

**Deliverables:**
- ✅ `npm install -g swictation` works on macOS
- ✅ Daemon starts automatically via launchd
- ✅ ONNX Runtime binaries load correctly

**Estimated Time:** 3-4 days

---

### Phase 3: Testing, Documentation & Polish (Week 3)

**Objective:** Comprehensive testing and documentation for macOS users.

**Tasks:**
1. Test on multiple Apple Silicon chips (M1, M2, M3, M4)
2. Performance benchmarking (latency, memory, accuracy)
3. Update README with macOS installation instructions
4. Create macOS troubleshooting guide
5. Add macOS-specific configuration examples
6. Test edge cases (permissions, sandboxing, notarization)

**Deliverables:**
- ✅ Comprehensive macOS documentation
- ✅ Performance benchmarks published
- ✅ All edge cases handled

**Estimated Time:** 4-5 days

---

## 4. Detailed Component Changes

### 4.1 Text Injection System (CRITICAL)

#### 4.1.1 Current Architecture

**Files:**
- `rust-crates/swictation-daemon/src/text_injection.rs` (344 lines)
- `rust-crates/swictation-daemon/src/display_server.rs` (449 lines)

**Current Implementation:**
```rust
pub enum TextInjectionTool {
    Xdotool,   // X11 only
    Wtype,     // Wayland only
    Ydotool,   // Universal Linux
}

impl TextInjector {
    fn inject_xdotool_text(&self, text: &str) -> Result<()> {
        Command::new("xdotool").arg("type").arg(text).output()?;
        Ok(())
    }
    // Similar for wtype, ydotool
}
```

#### 4.1.2 Required Changes

**Add new macOS variant:**

```rust
// text_injection.rs
pub enum TextInjectionTool {
    Xdotool,   // X11 only (Linux)
    Wtype,     // Wayland only (Linux)
    Ydotool,   // Universal Linux
    CGEvent,   // macOS only (NEW)
}

impl TextInjectionTool {
    pub fn command(&self) -> &'static str {
        match self {
            Self::Xdotool => "xdotool",
            Self::Wtype => "wtype",
            Self::Ydotool => "ydotool",
            Self::CGEvent => "cgevent",  // Not a real command, handled natively
        }
    }
}

#[cfg(target_os = "macos")]
impl TextInjector {
    /// Inject text using macOS CGEvent API
    fn inject_cgevent_text(&self, text: &str) -> Result<()> {
        use core_graphics::event::{CGEvent, CGEventTapLocation, EventField, CGKeyCode};
        use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .context("Failed to create CGEventSource")?;

        for ch in text.chars() {
            // Create keyboard event for each character
            let event = CGEvent::new_keyboard_event(
                source.clone(),
                char_to_keycode(ch),
                true  // key down
            ).context("Failed to create keyboard event")?;

            event.post(CGEventTapLocation::HID);

            // Key up event
            let event = CGEvent::new_keyboard_event(
                source.clone(),
                char_to_keycode(ch),
                false  // key up
            ).context("Failed to create keyboard event")?;

            event.post(CGEventTapLocation::HID);
        }

        Ok(())
    }

    /// Send key combination using CGEvent (for <KEY:...> markers)
    fn send_cgevent_keys(&self, combo: &str) -> Result<()> {
        // Parse combo string (e.g., "super-Right", "ctrl-c")
        let parts: Vec<&str> = combo.split('-').collect();

        let modifiers = parse_modifiers(&parts[..parts.len()-1])?;
        let key = parse_keycode(parts.last().unwrap())?;

        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)?;

        // Create event with modifiers
        let event = CGEvent::new_keyboard_event(source, key, true)?;
        event.set_flags(modifiers);
        event.post(CGEventTapLocation::HID);

        // Release
        let event = CGEvent::new_keyboard_event(source, key, false)?;
        event.post(CGEventTapLocation::HID);

        Ok(())
    }
}

/// Helper: Map character to macOS virtual keycode
#[cfg(target_os = "macos")]
fn char_to_keycode(ch: char) -> CGKeyCode {
    // Map common characters to macOS virtual keycodes
    // Reference: /System/Library/Frameworks/Carbon.framework/Versions/A/Frameworks/HIToolbox.framework/Versions/A/Headers/Events.h
    match ch {
        'a' => 0x00,
        'b' => 0x0B,
        // ... full mapping needed for all printable chars
        ' ' => 0x31,  // Space
        '\n' => 0x24, // Return
        _ => 0x00,    // Fallback
    }
}

/// Helper: Parse modifier keys
#[cfg(target_os = "macos")]
fn parse_modifiers(modifiers: &[&str]) -> Result<CGEventFlags> {
    use core_graphics::event::CGEventFlags;

    let mut flags = CGEventFlags::empty();

    for modifier in modifiers {
        match modifier.to_lowercase().as_str() {
            "super" | "cmd" | "command" => flags |= CGEventFlags::CGEventFlagCommand,
            "ctrl" | "control" => flags |= CGEventFlags::CGEventFlagControl,
            "alt" | "option" => flags |= CGEventFlags::CGEventFlagAlternate,
            "shift" => flags |= CGEventFlags::CGEventFlagShift,
            _ => anyhow::bail!("Unknown modifier: {}", modifier),
        }
    }

    Ok(flags)
}
```

**Update `TextInjector::new()` to detect macOS:**

```rust
impl TextInjector {
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            info!("Running on macOS - using CGEvent for text injection");
            return Ok(Self {
                display_server_info: DisplayServerInfo {
                    server_type: DisplayServer::Quartz,
                    desktop_environment: Some("macOS".to_string()),
                    is_gnome_wayland: false,
                    confidence: ConfidenceLevel::High,
                },
                selected_tool: TextInjectionTool::CGEvent,
            });
        }

        #[cfg(target_os = "linux")]
        {
            // Existing Linux detection logic
            let display_server_info = detect_display_server();
            let available_tools = detect_available_tools();

            if available_tools.is_empty() {
                anyhow::bail!(
                    "No text injection tools found. Please install xdotool, wtype, or ydotool"
                );
            }

            let selected_tool = select_best_tool(&display_server_info, &available_tools)?;

            Ok(Self {
                display_server_info,
                selected_tool,
            })
        }
    }
}
```

#### 4.1.3 Dependencies to Add

**Update `Cargo.toml`:**

```toml
[target.'cfg(target_os = "macos")'.dependencies]
core-graphics = "0.23"  # For CGEvent API
core-foundation = "0.9" # For CF types
cocoa = "0.25"          # For window management
objc = "0.2"            # For Objective-C runtime
```

#### 4.1.4 Testing Plan

1. **Unit Tests:**
   - Test character → keycode mapping
   - Test modifier parsing
   - Test CGEvent creation (mock)

2. **Integration Tests:**
   - Test text injection into TextEdit
   - Test keyboard shortcuts (Cmd+C, Cmd+V)
   - Test special characters (punctuation, symbols)
   - Test Secretary Mode commands

3. **Manual Tests:**
   - Test in various macOS apps (TextEdit, Notes, VS Code, Terminal)
   - Test with different keyboard layouts
   - Test with accessibility permissions

---

### 4.2 Display Server Detection

#### 4.2.1 Current Implementation

```rust
// display_server.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayServer {
    X11,
    Wayland,
    Unknown,
}
```

#### 4.2.2 Required Changes

**Add macOS variant:**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayServer {
    X11,       // Linux X11
    Wayland,   // Linux Wayland
    Quartz,    // macOS (NEW)
    Unknown,
}

/// Detect display server - macOS always uses Quartz
pub fn detect_display_server() -> DisplayServerInfo {
    #[cfg(target_os = "macos")]
    {
        return DisplayServerInfo {
            server_type: DisplayServer::Quartz,
            desktop_environment: Some("macOS".to_string()),
            is_gnome_wayland: false,
            confidence: ConfidenceLevel::High,
        };
    }

    #[cfg(target_os = "linux")]
    {
        // Existing Linux detection logic
        detect_display_server_with_env(&SystemEnv)
    }
}
```

---

### 4.3 Service Management (systemd → launchd)

#### 4.3.1 Current Implementation

**Linux systemd service:**

```ini
# ~/.config/systemd/user/swictation-daemon.service
[Unit]
Description=Swictation Voice Dictation Daemon
After=default.target

[Service]
Type=simple
ExecStart=/home/user/.npm-global/bin/swictation-daemon
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
```

#### 4.3.2 macOS launchd Implementation

**Create launchd plist template:**

**File:** `npm-package/templates/com.swictation.daemon.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.swictation.daemon</string>

    <key>ProgramArguments</key>
    <array>
        <string>{{DAEMON_PATH}}</string>
    </array>

    <key>RunAtLoad</key>
    <true/>

    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>

    <key>StandardOutPath</key>
    <string>{{LOG_DIR}}/swictation-daemon.log</string>

    <key>StandardErrorPath</key>
    <string>{{LOG_DIR}}/swictation-daemon-error.log</string>

    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin</string>
    </dict>

    <key>ProcessType</key>
    <string>Interactive</string>

    <key>Nice</key>
    <integer>-5</integer>
</dict>
</plist>
```

#### 4.3.3 Update `postinstall.js`

**Add macOS service generation:**

```javascript
// postinstall.js

async function setupServices() {
  if (process.platform === 'darwin') {
    await setupLaunchdService();
  } else if (process.platform === 'linux') {
    await setupSystemdService();
  }
}

async function setupLaunchdService() {
  log('cyan', '\n⚙️  Generating launchd service file...');

  const launchAgentsDir = path.join(os.homedir(), 'Library', 'LaunchAgents');
  const plistPath = path.join(launchAgentsDir, 'com.swictation.daemon.plist');

  // Create LaunchAgents directory
  if (!fs.existsSync(launchAgentsDir)) {
    fs.mkdirSync(launchAgentsDir, { recursive: true });
    log('green', `✓ Created ${launchAgentsDir}`);
  }

  // Load template
  const templatePath = path.join(__dirname, 'templates', 'com.swictation.daemon.plist');
  let plistContent = fs.readFileSync(templatePath, 'utf8');

  // Replace placeholders
  const daemonPath = path.join(getNpmGlobalBin(), 'swictation-daemon');
  const logDir = path.join(os.homedir(), 'Library', 'Logs', 'Swictation');

  plistContent = plistContent
    .replace('{{DAEMON_PATH}}', daemonPath)
    .replace(/{{LOG_DIR}}/g, logDir);

  // Create log directory
  if (!fs.existsSync(logDir)) {
    fs.mkdirSync(logDir, { recursive: true });
  }

  // Write plist file
  fs.writeFileSync(plistPath, plistContent);
  log('green', `✓ Generated launchd service: ${plistPath}`);

  // Load service
  try {
    execSync(`launchctl load ${plistPath}`, { stdio: 'inherit' });
    log('green', '✓ Loaded launchd service');
    log('cyan', '\nService commands:');
    log('cyan', `  Start:   launchctl start com.swictation.daemon`);
    log('cyan', `  Stop:    launchctl stop com.swictation.daemon`);
    log('cyan', `  Unload:  launchctl unload ${plistPath}`);
  } catch (err) {
    log('yellow', '⚠️  Could not load service automatically');
    log('cyan', `  Run manually: launchctl load ${plistPath}`);
  }
}
```

#### 4.3.4 CLI Commands

**Wrapper script updates:**

**File:** `npm-package/bin/swictation`

```javascript
#!/usr/bin/env node

const os = require('os');
const { execSync } = require('child_process');

function startDaemon() {
  if (process.platform === 'darwin') {
    // macOS - use launchctl
    try {
      execSync('launchctl start com.swictation.daemon', { stdio: 'inherit' });
      console.log('✓ Started swictation-daemon via launchctl');
    } catch (err) {
      console.error('Failed to start daemon:', err.message);
      process.exit(1);
    }
  } else if (process.platform === 'linux') {
    // Linux - use systemctl
    try {
      execSync('systemctl --user start swictation-daemon.service', { stdio: 'inherit' });
      console.log('✓ Started swictation-daemon.service');
    } catch (err) {
      console.error('Failed to start daemon:', err.message);
      process.exit(1);
    }
  }
}

function stopDaemon() {
  if (process.platform === 'darwin') {
    execSync('launchctl stop com.swictation.daemon', { stdio: 'inherit' });
  } else if (process.platform === 'linux') {
    execSync('systemctl --user stop swictation-daemon.service', { stdio: 'inherit' });
  }
}

function statusDaemon() {
  if (process.platform === 'darwin') {
    execSync('launchctl list | grep swictation', { stdio: 'inherit' });
  } else if (process.platform === 'linux') {
    execSync('systemctl --user status swictation-daemon.service', { stdio: 'inherit' });
  }
}

// Main CLI dispatcher
const command = process.argv[2];

switch (command) {
  case 'start':
    startDaemon();
    break;
  case 'stop':
    stopDaemon();
    break;
  case 'status':
    statusDaemon();
    break;
  case 'restart':
    stopDaemon();
    setTimeout(() => startDaemon(), 1000);
    break;
  default:
    console.log('Usage: swictation {start|stop|status|restart}');
    process.exit(1);
}
```

---

### 4.4 ONNX Runtime Binaries

#### 4.4.1 Current State

**Linux binaries in `npm-package/lib/native/`:**
- `libonnxruntime.so` (27.8 MB) - Core runtime
- `libonnxruntime_providers_cuda.so` (194 MB) - NVIDIA CUDA provider
- `libonnxruntime_providers_shared.so` (15 KB) - Shared provider interface
- `libonnxruntime_providers_tensorrt.so` (805 KB) - TensorRT optimization

**Total size:** ~223 MB

#### 4.4.2 macOS Requirements

**Need to download ARM64 binaries:**

**Source:** https://github.com/microsoft/onnxruntime/releases

**Required files:**
- `libonnxruntime.1.21.0.dylib` - Core runtime (ARM64)
- `libonnxruntime_providers_coreml.dylib` - CoreML provider (optional)

**Expected size:** ~30-40 MB (no CUDA)

#### 4.4.3 Download Strategy

**Update `postinstall.js`:**

```javascript
async function downloadOnnxRuntime() {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === 'darwin' && arch === 'arm64') {
    log('cyan', '\n📦 Downloading ONNX Runtime for macOS ARM64...');

    const version = '1.21.0';
    const baseUrl = `https://github.com/microsoft/onnxruntime/releases/download/v${version}`;
    const filename = `onnxruntime-osx-arm64-${version}.tgz`;
    const downloadUrl = `${baseUrl}/${filename}`;

    const nativeDir = path.join(__dirname, 'lib', 'native');
    const downloadPath = path.join(nativeDir, filename);

    // Download tarball
    await downloadFile(downloadUrl, downloadPath);

    // Extract
    execSync(`tar -xzf ${downloadPath} -C ${nativeDir}`, { stdio: 'inherit' });

    // Move libraries to correct location
    const extractedDir = path.join(nativeDir, `onnxruntime-osx-arm64-${version}`);
    const libDir = path.join(extractedDir, 'lib');

    fs.readdirSync(libDir).forEach(file => {
      if (file.endsWith('.dylib')) {
        fs.renameSync(
          path.join(libDir, file),
          path.join(nativeDir, file)
        );
      }
    });

    // Cleanup
    fs.rmSync(extractedDir, { recursive: true });
    fs.unlinkSync(downloadPath);

    log('green', '✓ ONNX Runtime installed for macOS ARM64');
  }
  // ... existing Linux download logic
}
```

#### 4.4.4 Library Loading

**Update Rust code to load correct library:**

```rust
// swictation-stt/src/lib.rs or relevant module

#[cfg(target_os = "macos")]
const ONNX_LIB_NAME: &str = "libonnxruntime.1.21.0.dylib";

#[cfg(target_os = "linux")]
const ONNX_LIB_NAME: &str = "libonnxruntime.so";

fn load_onnx_runtime() -> Result<()> {
    let lib_path = if cfg!(target_os = "macos") {
        // macOS: Check common locations
        vec![
            "/usr/local/lib/libonnxruntime.dylib",
            format!("{}/lib/native/libonnxruntime.1.21.0.dylib", env!("CARGO_MANIFEST_DIR")),
        ]
    } else {
        // Linux: existing logic
        vec![
            "/usr/lib/libonnxruntime.so",
            format!("{}/lib/native/libonnxruntime.so", env!("CARGO_MANIFEST_DIR")),
        ]
    };

    // Try each path
    for path in &lib_path {
        if std::path::Path::new(path).exists() {
            unsafe {
                // Load library dynamically
                let lib = libloading::Library::new(path)?;
                info!("Loaded ONNX Runtime from: {}", path);
                return Ok(());
            }
        }
    }

    anyhow::bail!("ONNX Runtime not found. Checked: {:?}", lib_path);
}
```

---

### 4.5 Build Scripts

#### 4.5.1 Current Build Script

**File:** `npm-package/scripts/build-release.sh`

**Linux-specific tools:**
- `sha256sum` - Calculate checksums
- `ldd` - Check dynamic dependencies
- `strings` - Search binary strings

#### 4.5.2 Cross-Platform Build Script

**Create universal build script:**

```bash
#!/bin/bash
# build-release.sh - Cross-platform build script

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
NPM_NATIVE_DIR="$REPO_ROOT/npm-package/lib/native"

echo "🔨 Building Swictation binaries for release..."
echo "Platform: $(uname -s) $(uname -m)"
echo ""

# 1. Build Rust binaries
echo "📦 Building Rust workspace in release mode..."
cd "$REPO_ROOT/rust-crates"
cargo build --release --workspace
echo "✓ Rust build complete"
echo ""

# 2. Copy binaries
echo "📋 Copying binaries to npm package..."
NPM_BIN_DIR="$REPO_ROOT/npm-package/bin"
mkdir -p "$NPM_BIN_DIR"
mkdir -p "$NPM_NATIVE_DIR"

# Daemon binary
cp "$REPO_ROOT/rust-crates/target/release/swictation-daemon" \
   "$NPM_BIN_DIR/swictation-daemon"
chmod +x "$NPM_BIN_DIR/swictation-daemon"

cp "$REPO_ROOT/rust-crates/target/release/swictation-daemon" \
   "$NPM_NATIVE_DIR/swictation-daemon.bin"
chmod +x "$NPM_NATIVE_DIR/swictation-daemon.bin"

echo "✓ Copied swictation-daemon"
echo ""

# 3. Verify binary integrity
echo "🔍 Verifying binary integrity..."

# Use platform-appropriate checksum tool
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    CHECKSUM_CMD="shasum -a 256"
    DEP_CHECK_CMD="otool -L"
else
    # Linux
    CHECKSUM_CMD="sha256sum"
    DEP_CHECK_CMD="ldd"
fi

SOURCE_HASH=$($CHECKSUM_CMD "$REPO_ROOT/rust-crates/target/release/swictation-daemon" | awk '{print $1}')
BIN_HASH=$($CHECKSUM_CMD "$NPM_BIN_DIR/swictation-daemon" | awk '{print $1}')
NATIVE_HASH=$($CHECKSUM_CMD "$NPM_NATIVE_DIR/swictation-daemon.bin" | awk '{print $1}')

if [ "$SOURCE_HASH" != "$BIN_HASH" ] || [ "$SOURCE_HASH" != "$NATIVE_HASH" ]; then
    echo "❌ ERROR: Binary checksums don't match!"
    echo "   Source:  $SOURCE_HASH"
    echo "   bin/:    $BIN_HASH"
    echo "   native/: $NATIVE_HASH"
    exit 1
fi

echo "✓ Binary checksums match"
echo ""

# 4. Show binary info
echo "📊 Binary info:"
ls -lh "$NPM_BIN_DIR/swictation-daemon"
echo ""

# 5. Check dependencies
echo "🔗 Dynamic library dependencies:"
$DEP_CHECK_CMD "$NPM_BIN_DIR/swictation-daemon" | grep -E "(onnx|sherpa)" || echo "  (No ONNX/Sherpa dynamic deps - good, using load-dynamic)"
echo ""

echo "✅ Build complete!"
echo "   Platform: $(uname -s) $(uname -m)"
echo "   Ready for npm publish"
```

---

### 4.6 npm Package Metadata

#### 4.6.1 Update `package.json`

```json
{
  "name": "swictation",
  "version": "0.5.0",
  "description": "Voice-to-text dictation with Wayland/X11 support (GNOME, Sway, i3), macOS support (Apple Silicon), Secretary Mode (60+ natural language commands), GPU acceleration, and pure Rust performance",
  "keywords": [
    "voice-to-text",
    "speech-recognition",
    "dictation",
    "secretary-mode",
    "transcription",
    "stt",
    "rust",
    "parakeet-tdt",
    "silero-vad",
    "wayland",
    "x11",
    "macos",
    "apple-silicon",
    "coreml",
    "onnx"
  ],
  "os": [
    "linux",
    "darwin"
  ],
  "cpu": [
    "x64",
    "arm64"
  ],
  "engines": {
    "node": ">=18.0.0"
  }
}
```

---

## 5. Build and Distribution

### 5.1 Build Matrix

| Platform | Architecture | Target Triple | ONNX Runtime | GPU Provider |
|----------|-------------|---------------|--------------|--------------|
| Linux | x86_64 | x86_64-unknown-linux-gnu | 1.21.0 (x86_64) | CUDA 12.x |
| macOS | ARM64 | aarch64-apple-darwin | 1.21.0 (ARM64) | CoreML/Metal |

### 5.2 Build Requirements

#### 5.2.1 Linux (Existing)
- Ubuntu 24.04+ (GLIBC 2.39+)
- Rust 1.70+
- CUDA 12.x (optional, for GPU)
- Node.js 18+

#### 5.2.2 macOS (New)
- macOS 13.0+ (Ventura or later)
- Xcode Command Line Tools
- Rust 1.70+
- Node.js 18+
- No external GPU drivers needed (CoreML built-in)

### 5.3 Cross-Compilation Strategy

**NOT RECOMMENDED:** Cross-compiling between Linux and macOS is complex.

**RECOMMENDED:** Build on native platforms:
- Build Linux binaries on Linux CI runner
- Build macOS binaries on macOS CI runner (GitHub Actions supports M1 runners)

**GitHub Actions example:**

```yaml
name: Build and Publish

on:
  push:
    tags:
      - 'v*'

jobs:
  build-linux:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v3
      - name: Build Linux x86_64
        run: |
          cargo build --release
          ./npm-package/scripts/build-release.sh
      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: linux-x86_64
          path: npm-package/lib/native/

  build-macos:
    runs-on: macos-14  # M1 runner
    steps:
      - uses: actions/checkout@v3
      - name: Build macOS ARM64
        run: |
          cargo build --release
          ./npm-package/scripts/build-release.sh
      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: macos-arm64
          path: npm-package/lib/native/

  publish:
    needs: [build-linux, build-macos]
    runs-on: ubuntu-latest
    steps:
      - name: Download all artifacts
      - name: Assemble npm package
      - name: Publish to npm
```

### 5.4 Binary Distribution Strategy

**Option 1: Multi-platform single package (RECOMMENDED)**

Structure:
```
npm-package/
  lib/
    native/
      linux-x64/
        swictation-daemon.bin
        libonnxruntime.so
        libonnxruntime_providers_cuda.so
      darwin-arm64/
        swictation-daemon.bin
        libonnxruntime.1.21.0.dylib
```

**Postinstall script selects correct binaries based on platform.**

**Option 2: Platform-specific packages**

Separate npm packages:
- `swictation-linux` (current package)
- `swictation-macos` (new package)
- `swictation` (meta-package that installs correct version)

**Recommendation:** Use Option 1 for simplicity.

---

## 6. Testing Strategy

### 6.1 Unit Tests

**Files to test:**
- `text_injection.rs` - CGEvent text injection
- `display_server.rs` - macOS Quartz detection
- `gpu.rs` - CoreML detection
- `config.rs` - Config path resolution on macOS

**Run tests:**
```bash
cargo test --target aarch64-apple-darwin
```

### 6.2 Integration Tests

**Test Scenarios:**

1. **Basic Dictation:**
   - Start daemon
   - Press hotkey
   - Speak "Hello world"
   - Verify text appears in TextEdit

2. **GPU Acceleration:**
   - Verify CoreML provider loads
   - Check GPU memory usage
   - Benchmark transcription latency

3. **Secretary Mode:**
   - Test punctuation commands ("comma", "period")
   - Test formatting commands ("new paragraph")
   - Test special characters ("at sign")

4. **Service Management:**
   - Install via `npm install -g swictation`
   - Verify launchd service created
   - Test `swictation start/stop/status`
   - Reboot and verify auto-start

5. **Hotkey Registration:**
   - Test default hotkey (Cmd+Shift+D)
   - Test custom hotkey from config
   - Test hotkey conflicts

### 6.3 Performance Benchmarks

**Metrics to measure:**

| Metric | Linux (CUDA) | macOS (CoreML) |
|--------|--------------|----------------|
| Model load time | ? | ? |
| Transcription latency (0.6B) | 100-150ms | ? |
| Transcription latency (1.1B) | 150-250ms | ? |
| Memory usage (RAM) | 150MB | ? |
| Memory usage (VRAM/Unified) | 2.2GB | ? |
| CPU usage (idle) | <5% | ? |
| CPU usage (transcribing) | 10-20% | ? |

### 6.4 Testing Hardware

**Minimum coverage:**
- M1 MacBook Air (base model, 8GB RAM)
- M2 MacBook Pro (16GB RAM)
- M3 MacBook Pro (24GB RAM)

**Nice to have:**
- M1 Max (32-core GPU)
- M4 Mac Mini

### 6.5 Accessibility Permissions Testing

**macOS 10.14+ requires explicit permission for:**
- ✅ **Accessibility** (required for CGEvent text injection)
- ⚠️ **Input Monitoring** (may be required for global hotkeys)

**Test flow:**
1. Install swictation
2. Start daemon
3. Trigger text injection
4. Verify system prompts for accessibility permission
5. Grant permission
6. Verify dictation works

---

## 7. Documentation Updates

### 7.1 README.md Updates

**Add macOS section:**

```markdown
## Quick Start

### Prerequisites

#### Linux (Ubuntu 24.04+)
- **NVIDIA GPU** with 4GB+ VRAM (or CPU fallback)
- **Text injection tool:** xdotool (X11) or wtype/ydotool (Wayland)
- **Node.js 18+**

#### macOS (Apple Silicon only)
- **macOS 13.0+** (Ventura or later)
- **M1/M2/M3/M4** chip (Apple Silicon)
- **Node.js 18+**
- **Accessibility permissions** (granted during first use)

### Install

#### Linux
```bash
# One-time npm setup
echo "prefix=$HOME/.npm-global" > ~/.npmrc
export PATH="$HOME/.npm-global/bin:$PATH"

# Install
npm install -g swictation --foreground-scripts

# Install text injection tool
sudo apt install xdotool  # X11
# OR
sudo apt install wtype    # Wayland (KDE/Sway)
# OR
sudo apt install ydotool && sudo usermod -aG input $USER  # Wayland (GNOME)
```

#### macOS
```bash
# One-time npm setup
echo "prefix=$HOME/.npm-global" > ~/.npmrc
export PATH="$HOME/.npm-global/bin:$PATH"

# Install
npm install -g swictation --foreground-scripts

# Start
swictation start
```

**Note:** macOS will prompt for Accessibility permissions on first use. Grant permission in System Settings > Privacy & Security > Accessibility.

### First Use

#### Linux
1. Open any text editor
2. Press `Super+Shift+D`
3. Speak: "Hello world." [pause]
4. Text appears after 0.8s silence

#### macOS
1. Open any text editor (TextEdit, Notes, etc.)
2. Press `Cmd+Shift+D`
3. Speak: "Hello world." [pause]
4. Text appears after 0.8s silence

### Configuration

Edit `~/.config/swictation/config.toml`:

```toml
[hotkey]
# Linux: "super" = Super/Windows key
# macOS: "super" = Command key
modifiers = ["super", "shift"]
key = "d"
```

### Platform-Specific Notes

#### macOS Permissions
- **Accessibility:** Required for text injection (system prompt on first use)
- **Input Monitoring:** May be required for global hotkeys (grant in System Settings)

#### macOS Hotkey Defaults
- Default: `Cmd+Shift+D` (equivalent to `Super+Shift+D` on Linux)
- Change in config: `modifiers = ["super", "shift"]` or `["command", "shift"]`

#### GPU Acceleration
- **Linux:** NVIDIA CUDA (auto-detected)
- **macOS:** Apple CoreML/Metal (auto-detected)
```

### 7.2 Create macOS Troubleshooting Guide

**File:** `docs/troubleshooting-macos.md`

```markdown
# macOS Troubleshooting Guide

## Installation Issues

### "swictation: command not found"

**Cause:** npm global bin directory not in PATH.

**Solution:**
```bash
echo "prefix=$HOME/.npm-global" > ~/.npmrc
export PATH="$HOME/.npm-global/bin:$PATH"
echo 'export PATH="$HOME/.npm-global/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### "Unsupported platform: darwin arm64"

**Cause:** You're on an Intel Mac (not supported).

**Solution:** Swictation only supports Apple Silicon (M1/M2/M3/M4). Intel Macs are not supported.

## Permission Issues

### "Text injection failed: Permission denied"

**Cause:** Accessibility permissions not granted.

**Solution:**
1. Open **System Settings** > **Privacy & Security** > **Accessibility**
2. Find `swictation-daemon` or `Terminal` (if running manually)
3. Enable the toggle
4. Restart swictation: `swictation restart`

### "Hotkey not working"

**Cause:** Input Monitoring permission not granted.

**Solution:**
1. Open **System Settings** > **Privacy & Security** > **Input Monitoring**
2. Enable `swictation-daemon`
3. Restart swictation

## Performance Issues

### "Transcription is slow"

**Check CoreML acceleration:**
```bash
swictation status
# Should show: GPU: CoreML (Apple Silicon)
```

**Check memory:**
```bash
# Unified memory should have 4GB+ free
vm_stat
```

### "Daemon crashes on startup"

**Check logs:**
```bash
tail -f ~/Library/Logs/Swictation/swictation-daemon.log
tail -f ~/Library/Logs/Swictation/swictation-daemon-error.log
```

**Common causes:**
- Insufficient memory (need 4GB+ free unified memory)
- ONNX Runtime not installed (run `npm install -g swictation --foreground-scripts` again)
- Corrupted model files (delete `~/.local/share/swictation/models` and reinstall)

## Service Management

### Check if daemon is running

```bash
launchctl list | grep swictation
```

**Expected output:**
```
12345   0   com.swictation.daemon
```

### Start daemon manually

```bash
swictation start
# OR
launchctl start com.swictation.daemon
```

### Stop daemon

```bash
swictation stop
# OR
launchctl stop com.swictation.daemon
```

### Disable auto-start on boot

```bash
launchctl unload ~/Library/LaunchAgents/com.swictation.daemon.plist
```

### Re-enable auto-start

```bash
launchctl load ~/Library/LaunchAgents/com.swictation.daemon.plist
```

## Model Issues

### "Model not found" error

**Solution:**
```bash
# Re-download models
cd ~/.local/share/swictation/models
rm -rf *
npm install -g swictation --foreground-scripts
```

### Check model files

```bash
ls -lh ~/.local/share/swictation/models/
```

**Expected:**
```
silero-vad/
  silero_vad.onnx (629 KB)
sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx/
  encoder.onnx
  decoder.onnx
  joiner.onnx
  tokens.txt
```

## Configuration Issues

### Reset configuration

```bash
rm ~/.config/swictation/config.toml
swictation start  # Will regenerate default config
```

### Check configuration

```bash
cat ~/.config/swictation/config.toml
```

## Reporting Bugs

Include this information:
1. macOS version: `sw_vers`
2. Chip: `sysctl -n machdep.cpu.brand_string`
3. Swictation version: `npm list -g swictation`
4. Logs: `cat ~/Library/Logs/Swictation/swictation-daemon-error.log`
5. Error output from terminal

Open issue at: https://github.com/robertelee78/swictation/issues
```

### 7.3 Update Architecture Documentation

**File:** `docs/architecture.md`

**Add platform comparison section:**

```markdown
## Platform Support

### Linux (x86_64)
- **Display Server:** X11, Wayland (GNOME, KDE, Sway, i3, Hyprland)
- **Text Injection:** xdotool (X11), wtype (Wayland), ydotool (universal)
- **GPU:** NVIDIA CUDA (GTX 750+)
- **Audio:** PipeWire, PulseAudio, ALSA (via cpal)
- **Service Management:** systemd user services
- **Hotkey:** global-hotkey crate (X11/Wayland events)

### macOS (ARM64)
- **Display Server:** Quartz (AppKit/CoreGraphics)
- **Text Injection:** CGEvent API (native)
- **GPU:** Apple CoreML/Metal (M1/M2/M3/M4)
- **Audio:** CoreAudio (via cpal)
- **Service Management:** launchd agents
- **Hotkey:** global-hotkey crate (macOS events)

### Platform-Agnostic Components
- **VAD:** Silero VAD v6 (ONNX)
- **STT:** Parakeet-TDT 0.6B/1.1B (ONNX)
- **Transform:** MidStream text-transform (pure Rust)
- **Metrics:** Unix socket broadcaster (cross-platform)
```

---

## 8. Risk Analysis

### 8.1 High-Risk Items 🔴

| Risk | Impact | Mitigation |
|------|--------|------------|
| **CGEvent API complexity** | Text injection may not work in all apps | Extensive testing in major apps; fallback to AppleScript |
| **Accessibility permissions** | Users may deny permissions | Clear UI prompts; comprehensive documentation |
| **CoreML performance** | Unknown if CoreML matches CUDA performance | Benchmark on M1/M2/M3; may need to optimize models |
| **Unified memory limits** | M1 base (8GB) may struggle with 1.1B model | Smart model selection; fallback to 0.6B |

### 8.2 Medium-Risk Items 🟡

| Risk | Impact | Mitigation |
|------|--------|------------|
| **ONNX Runtime ARM64 support** | Libraries may have bugs on Apple Silicon | Test thoroughly; report issues upstream |
| **launchd service reliability** | Service may not auto-restart | Add KeepAlive policy; test crash recovery |
| **Global hotkey conflicts** | May conflict with system shortcuts | Allow custom hotkey configuration |
| **App sandboxing** | Future App Store distribution may require sandbox | Design architecture to support sandboxing |

### 8.3 Low-Risk Items 🟢

| Risk | Impact | Mitigation |
|------|--------|------------|
| **Build script portability** | Minor tool differences (sha256sum vs shasum) | Already handled with platform detection |
| **Path differences** | Unix paths mostly compatible | Standard Node.js path handling |
| **Audio capture** | cpal is mature on macOS | Low risk, well-tested |

---

## 9. Release Plan

### 9.1 Version Strategy

**Current:** v0.4.x (Linux-only)

**macOS MVP:** v0.5.0
- Add macOS support (Apple Silicon only)
- Maintain Linux compatibility
- No breaking changes to Linux users

**Future:** v0.6.0+
- Refinements based on user feedback
- Performance optimizations
- Potential Intel Mac support (if requested)

### 9.2 Beta Testing

**Phase 1 (Internal):** Test on team's own Macs
- Verify basic functionality
- Fix critical bugs

**Phase 2 (Limited Beta):** Invite 5-10 macOS users
- Provide detailed testing instructions
- Gather feedback on installation, permissions, performance

**Phase 3 (Public Beta):** npm tag as `@beta`
```bash
npm install -g swictation@beta
```

**Phase 4 (Stable):** Promote to latest
```bash
npm install -g swictation
```

### 9.3 Communication Plan

**Announcement channels:**
1. GitHub Release Notes
2. npm package description
3. Reddit r/macOS, r/linux
4. Twitter/X announcement
5. Hacker News (if traction)

**Key messaging:**
- "Now supports Apple Silicon Macs!"
- "Pure Rust, no Python runtime"
- "GPU-accelerated via CoreML"
- "Same great Secretary Mode on macOS"

---

## 10. Success Criteria

### 10.1 Functional Requirements

- ✅ Daemon compiles and runs on macOS ARM64
- ✅ Text injection works in macOS apps (TextEdit, Notes, VS Code, Terminal)
- ✅ Hotkey registration works (Cmd+Shift+D)
- ✅ Audio capture works (CoreAudio via cpal)
- ✅ GPU acceleration via CoreML
- ✅ Model selection based on unified memory
- ✅ Secretary Mode commands work
- ✅ launchd service auto-starts on boot
- ✅ `swictation start/stop/status` CLI works
- ✅ npm installation succeeds
- ✅ Linux functionality remains 100% intact

### 10.2 Performance Requirements

**Target (comparable to Linux):**
- Model load time: <5 seconds
- Transcription latency: <300ms (0.6B model)
- Memory usage: <1GB RAM + <3GB unified memory
- CPU idle: <5%
- Accuracy: WER <6% (same as Linux)

### 10.3 User Experience Requirements

- Installation takes <10 minutes
- Clear permission prompts
- Works "out of the box" after permissions granted
- Comprehensive error messages
- macOS-specific documentation available

---

## 11. Appendix

### 11.1 Key Files Modified

**Rust files:**
- `rust-crates/swictation-daemon/src/text_injection.rs` (major changes)
- `rust-crates/swictation-daemon/src/display_server.rs` (moderate changes)
- `rust-crates/swictation-daemon/src/gpu.rs` (minor testing)
- `rust-crates/swictation-daemon/Cargo.toml` (add macOS deps)
- `rust-crates/swictation-audio/Cargo.toml` (no changes needed)

**npm package files:**
- `npm-package/package/package.json` (update os/cpu metadata)
- `npm-package/package/postinstall.js` (add macOS logic)
- `npm-package/package/preuninstall.js` (add launchd cleanup)
- `npm-package/bin/swictation` (add launchctl commands)
- `npm-package/scripts/build-release.sh` (make cross-platform)
- `npm-package/templates/com.swictation.daemon.plist` (NEW)

**Documentation:**
- `README.md` (add macOS section)
- `docs/troubleshooting-macos.md` (NEW)
- `docs/architecture.md` (add platform comparison)
- `docs/installation-macos.md` (NEW)

### 11.2 External Dependencies

**Rust crates to add:**
```toml
[target.'cfg(target_os = "macos")'.dependencies]
core-graphics = "0.23"
core-foundation = "0.9"
cocoa = "0.25"
objc = "0.2"
```

**ONNX Runtime:**
- Download from: https://github.com/microsoft/onnxruntime/releases/tag/v1.21.0
- File: `onnxruntime-osx-arm64-1.21.0.tgz` (~40 MB)

### 11.3 Testing Checklist

**Pre-release checklist:**

- [ ] Code compiles on macOS ARM64
- [ ] All Rust unit tests pass
- [ ] Text injection works in 5+ apps
- [ ] Hotkey registration works
- [ ] Audio capture works
- [ ] CoreML GPU detection works
- [ ] Model loading succeeds (0.6B and 1.1B)
- [ ] Transcription accuracy acceptable (manual testing)
- [ ] Secretary Mode commands work
- [ ] launchd service installs correctly
- [ ] Service auto-starts on reboot
- [ ] `swictation` CLI works (start/stop/status/restart)
- [ ] npm installation completes
- [ ] Accessibility permission prompt appears
- [ ] Documentation complete
- [ ] Linux regression tests pass
- [ ] Performance benchmarks collected

---

## Summary

This specification provides a comprehensive roadmap for adding macOS Apple Silicon support to Swictation. The project's excellent cross-platform architecture means most components already work on macOS. The main implementation work involves:

1. **Text injection via CGEvent API** (3-5 days)
2. **launchd service management** (1-2 days)
3. **ONNX Runtime binary distribution** (1 day)
4. **Testing and documentation** (4-5 days)

**Total estimated effort: 2-3 weeks**

The specification ensures:
- ✅ 100% backward compatibility with Linux
- ✅ Clear implementation plan with code examples
- ✅ Comprehensive testing strategy
- ✅ Risk mitigation for known issues
- ✅ Documentation updates for users

**Next steps:**
1. Review and approve specification
2. Create GitHub issue tracking implementation
3. Begin Phase 1 development
4. Set up macOS CI runner for automated builds
