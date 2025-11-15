# Display Server Abstraction Layer Design

**Version:** 1.0
**Date:** 2025-11-14
**Author:** Coder Agent (Hive Mind)
**Status:** Design Phase

---

## Executive Summary

This document outlines the design for a trait-based display server abstraction layer that provides dual X11/Wayland support for the Swictation voice-to-text daemon. The abstraction maintains backward compatibility with existing Wayland code while enabling X11 support through a clean, extensible interface.

---

## 1. Core Trait Design

### 1.1 Primary Trait: `DisplayServerBackend`

```rust
use anyhow::Result;

/// Core trait for display server operations
pub trait DisplayServerBackend: Send + Sync {
    /// Display server identifier
    fn name(&self) -> &'static str;

    /// Inject plain text into the focused window
    fn inject_text(&self, text: &str) -> Result<()>;

    /// Send a keyboard shortcut (e.g., "ctrl-c", "super-Right")
    fn send_key_combination(&self, combo: &str) -> Result<()>;

    /// Copy text to clipboard
    fn set_clipboard(&self, text: &str) -> Result<()>;

    /// Get text from clipboard
    fn get_clipboard(&self) -> Result<String>;

    /// Check if the backend is available on this system
    fn is_available() -> bool where Self: Sized;

    /// Validate required external tools are installed
    fn validate_tools(&self) -> Result<()>;

    /// Get backend capabilities
    fn capabilities(&self) -> BackendCapabilities;
}

/// Backend capability flags
#[derive(Debug, Clone)]
pub struct BackendCapabilities {
    pub supports_text_injection: bool,
    pub supports_key_combinations: bool,
    pub supports_clipboard: bool,
    pub supports_unicode: bool,
    pub requires_external_tools: bool,
}
```

### 1.2 Supporting Types

```rust
/// Display server type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayServerType {
    X11,
    Wayland,
    Unknown,
}

/// Detection result with confidence
#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub server_type: DisplayServerType,
    pub confidence: DetectionConfidence,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DetectionConfidence {
    High,      // Multiple confirming signals
    Medium,    // Single reliable signal
    Low,       // Fallback/guessing
}
```

---

## 2. Backend Implementations

### 2.1 Wayland Backend

```rust
/// Wayland display server backend using wtype/wl-clipboard
pub struct WaylandBackend;

impl DisplayServerBackend for WaylandBackend {
    fn name(&self) -> &'static str {
        "Wayland"
    }

    fn inject_text(&self, text: &str) -> Result<()> {
        Command::new("wtype")
            .arg(text)
            .output()
            .context("Failed to inject text with wtype")?;
        Ok(())
    }

    fn send_key_combination(&self, combo: &str) -> Result<()> {
        // Parse combo: "ctrl-c" -> ["ctrl", "c"]
        let parts: Vec<&str> = combo.split('-').collect();
        let mut cmd = Command::new("wtype");

        // Add modifiers
        for part in &parts[..parts.len() - 1] {
            let modifier = match part.to_lowercase().as_str() {
                "super" | "mod4" => "logo",
                "ctrl" | "control" => "ctrl",
                "alt" => "alt",
                "shift" => "shift",
                _ => continue,
            };
            cmd.arg("-M").arg(modifier);
        }

        // Add key
        if let Some(key) = parts.last() {
            cmd.arg("-k").arg(key);
        }

        cmd.output()
            .context(format!("Failed to send key: {}", combo))?;
        Ok(())
    }

    fn set_clipboard(&self, text: &str) -> Result<()> {
        Command::new("wl-copy")
            .arg(text)
            .output()
            .context("Failed to set clipboard")?;
        Ok(())
    }

    fn get_clipboard(&self) -> Result<String> {
        let output = Command::new("wl-paste")
            .output()
            .context("Failed to get clipboard")?;
        Ok(String::from_utf8(output.stdout)?)
    }

    fn is_available() -> bool {
        std::env::var("WAYLAND_DISPLAY").is_ok()
    }

    fn validate_tools(&self) -> Result<()> {
        // Check wtype
        Command::new("which")
            .arg("wtype")
            .output()
            .context("wtype not found. Install: sudo apt install wtype")?;

        // Check wl-clipboard
        Command::new("which")
            .arg("wl-copy")
            .output()
            .context("wl-clipboard not found. Install: sudo apt install wl-clipboard")?;

        Ok(())
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            supports_text_injection: true,
            supports_key_combinations: true,
            supports_clipboard: true,
            supports_unicode: true,
            requires_external_tools: true,
        }
    }
}
```

### 2.2 X11 Backend

```rust
/// X11 display server backend using xdotool/xclip
pub struct X11Backend;

impl DisplayServerBackend for X11Backend {
    fn name(&self) -> &'static str {
        "X11"
    }

    fn inject_text(&self, text: &str) -> Result<()> {
        Command::new("xdotool")
            .arg("type")
            .arg("--clearmodifiers")
            .arg("--")
            .arg(text)
            .output()
            .context("Failed to inject text with xdotool")?;
        Ok(())
    }

    fn send_key_combination(&self, combo: &str) -> Result<()> {
        // Convert "ctrl-c" -> "ctrl+c" for xdotool
        let xdo_combo = combo.replace('-', "+");

        Command::new("xdotool")
            .arg("key")
            .arg(xdo_combo)
            .output()
            .context(format!("Failed to send key: {}", combo))?;
        Ok(())
    }

    fn set_clipboard(&self, text: &str) -> Result<()> {
        let mut child = Command::new("xclip")
            .arg("-selection")
            .arg("clipboard")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .context("Failed to start xclip")?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(text.as_bytes())?;
        }

        child.wait()?;
        Ok(())
    }

    fn get_clipboard(&self) -> Result<String> {
        let output = Command::new("xclip")
            .arg("-selection")
            .arg("clipboard")
            .arg("-o")
            .output()
            .context("Failed to get clipboard")?;

        Ok(String::from_utf8(output.stdout)?)
    }

    fn is_available() -> bool {
        std::env::var("DISPLAY").is_ok()
    }

    fn validate_tools(&self) -> Result<()> {
        // Check xdotool
        Command::new("which")
            .arg("xdotool")
            .output()
            .context("xdotool not found. Install: sudo apt install xdotool")?;

        // Check xclip
        Command::new("which")
            .arg("xclip")
            .output()
            .context("xclip not found. Install: sudo apt install xclip")?;

        Ok(())
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            supports_text_injection: true,
            supports_key_combinations: true,
            supports_clipboard: true,
            supports_unicode: true,
            requires_external_tools: true,
        }
    }
}
```

---

## 3. Detection Mechanism

### 3.1 Runtime Detection Strategy

```rust
/// Detect display server with evidence-based confidence scoring
pub fn detect_display_server() -> DetectionResult {
    let mut evidence = Vec::new();
    let mut wayland_score = 0;
    let mut x11_score = 0;

    // Check WAYLAND_DISPLAY (strongest Wayland signal)
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        evidence.push("WAYLAND_DISPLAY set".to_string());
        wayland_score += 3;
    }

    // Check DISPLAY (X11 signal, but also present in XWayland)
    if let Ok(display) = std::env::var("DISPLAY") {
        evidence.push(format!("DISPLAY={}", display));
        x11_score += 1;
    }

    // Check XDG_SESSION_TYPE (reliable but can be absent)
    if let Ok(session_type) = std::env::var("XDG_SESSION_TYPE") {
        evidence.push(format!("XDG_SESSION_TYPE={}", session_type));
        match session_type.as_str() {
            "wayland" => wayland_score += 3,
            "x11" => x11_score += 3,
            _ => {},
        }
    }

    // Check for Wayland compositor
    if let Ok(wc) = std::env::var("WAYLAND_COMPOSITOR") {
        evidence.push(format!("WAYLAND_COMPOSITOR={}", wc));
        wayland_score += 2;
    }

    // Check GDK backend hint
    if let Ok(backend) = std::env::var("GDK_BACKEND") {
        evidence.push(format!("GDK_BACKEND={}", backend));
        match backend.as_str() {
            "wayland" => wayland_score += 1,
            "x11" => x11_score += 1,
            _ => {},
        }
    }

    // Determine winner and confidence
    let (server_type, confidence) = if wayland_score > x11_score {
        let conf = if wayland_score >= 4 {
            DetectionConfidence::High
        } else if wayland_score >= 2 {
            DetectionConfidence::Medium
        } else {
            DetectionConfidence::Low
        };
        (DisplayServerType::Wayland, conf)
    } else if x11_score > wayland_score {
        let conf = if x11_score >= 4 {
            DetectionConfidence::High
        } else if x11_score >= 2 {
            DetectionConfidence::Medium
        } else {
            DetectionConfidence::Low
        };
        (DisplayServerType::X11, conf)
    } else {
        (DisplayServerType::Unknown, DetectionConfidence::Low)
    };

    DetectionResult {
        server_type,
        confidence,
        evidence,
    }
}
```

### 3.2 XWayland Detection

```rust
/// Detect if running under XWayland (X11 compatibility on Wayland)
pub fn is_xwayland() -> bool {
    // XWayland has both WAYLAND_DISPLAY and DISPLAY set
    std::env::var("WAYLAND_DISPLAY").is_ok()
        && std::env::var("DISPLAY").is_ok()
        && std::env::var("XDG_SESSION_TYPE")
            .map(|t| t == "wayland")
            .unwrap_or(false)
}
```

---

## 4. Manager/Factory Pattern

### 4.1 Display Server Manager

```rust
use std::sync::Arc;

/// Manager for display server backends with auto-detection
pub struct DisplayServerManager {
    backend: Arc<dyn DisplayServerBackend>,
    detection: DetectionResult,
}

impl DisplayServerManager {
    /// Create manager with auto-detected backend
    pub fn new() -> Result<Self> {
        let detection = detect_display_server();

        let backend: Arc<dyn DisplayServerBackend> = match detection.server_type {
            DisplayServerType::Wayland => {
                let backend = WaylandBackend;
                backend.validate_tools()?;
                Arc::new(backend)
            }
            DisplayServerType::X11 => {
                let backend = X11Backend;
                backend.validate_tools()?;
                Arc::new(backend)
            }
            DisplayServerType::Unknown => {
                // Try Wayland first, fallback to X11
                if WaylandBackend::is_available() {
                    let backend = WaylandBackend;
                    if backend.validate_tools().is_ok() {
                        Arc::new(backend)
                    } else if X11Backend::is_available() {
                        Arc::new(X11Backend)
                    } else {
                        anyhow::bail!("No supported display server detected");
                    }
                } else if X11Backend::is_available() {
                    Arc::new(X11Backend)
                } else {
                    anyhow::bail!("No supported display server detected");
                }
            }
        };

        Ok(Self { backend, detection })
    }

    /// Create manager with explicit backend type
    pub fn with_backend(server_type: DisplayServerType) -> Result<Self> {
        let backend: Arc<dyn DisplayServerBackend> = match server_type {
            DisplayServerType::Wayland => {
                let backend = WaylandBackend;
                backend.validate_tools()?;
                Arc::new(backend)
            }
            DisplayServerType::X11 => {
                let backend = X11Backend;
                backend.validate_tools()?;
                Arc::new(backend)
            }
            DisplayServerType::Unknown => {
                anyhow::bail!("Cannot create backend for Unknown type");
            }
        };

        let detection = DetectionResult {
            server_type,
            confidence: DetectionConfidence::High,
            evidence: vec!["Explicit selection".to_string()],
        };

        Ok(Self { backend, detection })
    }

    /// Get reference to backend
    pub fn backend(&self) -> &Arc<dyn DisplayServerBackend> {
        &self.backend
    }

    /// Get detection information
    pub fn detection(&self) -> &DetectionResult {
        &self.detection
    }

    /// Inject text using the active backend
    pub fn inject_text(&self, text: &str) -> Result<()> {
        self.backend.inject_text(text)
    }

    /// Send key combination using the active backend
    pub fn send_key_combination(&self, combo: &str) -> Result<()> {
        self.backend.send_key_combination(combo)
    }

    /// Set clipboard content
    pub fn set_clipboard(&self, text: &str) -> Result<()> {
        self.backend.set_clipboard(text)
    }

    /// Get clipboard content
    pub fn get_clipboard(&self) -> Result<String> {
        self.backend.get_clipboard()
    }
}
```

---

## 5. Migration Strategy

### 5.1 Backward Compatibility

The existing `TextInjector` in `text_injection.rs` will be refactored to use the new abstraction:

```rust
use crate::display::{DisplayServerManager, DisplayServerBackend};

pub struct TextInjector {
    manager: DisplayServerManager,
}

impl TextInjector {
    pub fn new() -> Result<Self> {
        let manager = DisplayServerManager::new()?;
        Ok(Self { manager })
    }

    pub fn inject_text(&self, text: &str) -> Result<()> {
        // Handle <KEY:...> markers
        if text.contains("<KEY:") {
            self.inject_with_keys(text)
        } else {
            self.manager.inject_text(text)
        }
    }

    // ... rest of existing implementation
}
```

### 5.2 Module Structure

```
rust-crates/swictation-daemon/src/
├── display/
│   ├── mod.rs              # Re-exports
│   ├── backend.rs          # DisplayServerBackend trait
│   ├── wayland.rs          # WaylandBackend implementation
│   ├── x11.rs              # X11Backend implementation
│   ├── detection.rs        # detect_display_server(), is_xwayland()
│   └── manager.rs          # DisplayServerManager
├── text_injection.rs       # Updated to use display::DisplayServerManager
└── ...
```

---

## 6. Error Handling Strategy

### 6.1 Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DisplayServerError {
    #[error("Display server not detected")]
    NotDetected,

    #[error("Backend not available: {0}")]
    BackendNotAvailable(String),

    #[error("Required tool not found: {0}")]
    ToolNotFound(String),

    #[error("Text injection failed: {0}")]
    InjectionFailed(String),

    #[error("Clipboard operation failed: {0}")]
    ClipboardFailed(String),

    #[error("Invalid key combination: {0}")]
    InvalidKeyCombination(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}
```

### 6.2 Fallback Strategies

```rust
impl DisplayServerManager {
    /// Try operation with automatic fallback
    pub fn inject_text_with_fallback(&self, text: &str) -> Result<()> {
        match self.inject_text(text) {
            Ok(()) => Ok(()),
            Err(e) => {
                warn!("Primary injection failed: {}, trying clipboard fallback", e);
                // Fallback: copy to clipboard
                self.set_clipboard(text)?;
                // Simulate Ctrl+V
                self.send_key_combination("ctrl-v")?;
                Ok(())
            }
        }
    }
}
```

---

## 7. Testing Strategy

### 7.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detection_logic() {
        // Test detection with various env combinations
        std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
        let result = detect_display_server();
        assert_eq!(result.server_type, DisplayServerType::Wayland);
        assert!(result.confidence >= DetectionConfidence::Medium);
    }

    #[test]
    fn test_xwayland_detection() {
        std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
        std::env::set_var("DISPLAY", ":0");
        std::env::set_var("XDG_SESSION_TYPE", "wayland");
        assert!(is_xwayland());
    }

    #[test]
    fn test_backend_capabilities() {
        let wayland = WaylandBackend;
        let caps = wayland.capabilities();
        assert!(caps.supports_text_injection);
        assert!(caps.supports_clipboard);
    }
}
```

### 7.2 Integration Tests

```rust
// tests/display_server_integration.rs
#[tokio::test]
async fn test_text_injection_e2e() {
    // Requires actual display server
    if std::env::var("CI").is_ok() {
        return; // Skip in CI
    }

    let manager = DisplayServerManager::new().unwrap();

    // Test plain text
    manager.inject_text("Hello, world!").unwrap();

    // Test clipboard
    manager.set_clipboard("test").unwrap();
    let content = manager.get_clipboard().unwrap();
    assert_eq!(content, "test");
}
```

---

## 8. Performance Considerations

### 8.1 Caching and Optimization

- **Backend instance caching**: Create once, reuse throughout daemon lifetime
- **Detection caching**: Run detection once at startup
- **Tool validation**: Validate external tools once, cache results

### 8.2 Async Support (Future Enhancement)

```rust
#[async_trait]
pub trait AsyncDisplayServerBackend: Send + Sync {
    async fn inject_text(&self, text: &str) -> Result<()>;
    async fn send_key_combination(&self, combo: &str) -> Result<()>;
    // ... async versions of all operations
}
```

---

## 9. Configuration Integration

### 9.1 Config TOML Extension

```toml
[display]
# Force specific backend (auto-detect if not set)
backend = "auto"  # Options: "auto", "wayland", "x11"

# Fallback behavior
enable_clipboard_fallback = true
retry_attempts = 3
retry_delay_ms = 100

# Tool paths (auto-detect if not set)
wtype_path = "/usr/bin/wtype"
xdotool_path = "/usr/bin/xdotool"
```

---

## 10. Future Extensions

### 10.1 Additional Backends

- **macOS**: CGEventPost API
- **Windows**: SendInput API
- **Direct protocols**: libei, libinput

### 10.2 Advanced Features

- **Multi-monitor support**: Inject to specific screen
- **Window targeting**: Inject to specific window by title/class
- **Paste mode detection**: Detect when clipboard approach is needed
- **Virtual keyboard**: Native protocol implementation (no external tools)

---

## Conclusion

This abstraction layer provides:

✅ **Clean trait-based interface**
✅ **Runtime display server detection**
✅ **Dual X11/Wayland support**
✅ **Backward compatibility**
✅ **Extensibility for future backends**
✅ **Robust error handling**
✅ **Comprehensive testing strategy**

The design maintains the existing functionality while enabling X11 support and establishing a foundation for future display server backends.
