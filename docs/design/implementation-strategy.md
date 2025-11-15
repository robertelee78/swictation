# X11 Support Implementation Strategy

**Version:** 1.0
**Date:** 2025-11-14
**Author:** Coder Agent (Hive Mind)
**Project:** Swictation Display Server Abstraction

---

## 1. Implementation Phases

### Phase 1: Foundation (Week 1)
**Goal:** Create trait-based abstraction with Wayland backend

#### Tasks:
1. Create `display` module structure
2. Define `DisplayServerBackend` trait
3. Implement `WaylandBackend` (refactor from existing code)
4. Implement detection logic
5. Add unit tests for detection

#### Files to Create:
```
rust-crates/swictation-daemon/src/display/
├── mod.rs              # Module exports
├── backend.rs          # DisplayServerBackend trait
├── wayland.rs          # WaylandBackend implementation
├── detection.rs        # detect_display_server()
└── types.rs            # DisplayServerType, DetectionResult
```

#### Implementation Order:
```rust
// Step 1: types.rs - Define core types
pub enum DisplayServerType { X11, Wayland, Unknown }
pub struct DetectionResult { ... }

// Step 2: backend.rs - Define trait
pub trait DisplayServerBackend { ... }

// Step 3: detection.rs - Implement detection
pub fn detect_display_server() -> DetectionResult { ... }

// Step 4: wayland.rs - Port existing code
pub struct WaylandBackend;
impl DisplayServerBackend for WaylandBackend { ... }

// Step 5: mod.rs - Export public API
pub use backend::DisplayServerBackend;
pub use wayland::WaylandBackend;
pub use detection::detect_display_server;
```

### Phase 2: X11 Backend (Week 1-2)
**Goal:** Implement X11 support using external tools

#### Tasks:
1. Create `X11Backend` implementation
2. Add X11 tool validation
3. Implement key combination parsing for xdotool
4. Add clipboard support with xclip
5. Integration tests on X11 system

#### Files to Create:
```
rust-crates/swictation-daemon/src/display/
├── x11.rs              # X11Backend implementation
└── tools.rs            # External tool validation utilities
```

#### Implementation Details:
```rust
// x11.rs
pub struct X11Backend;

impl DisplayServerBackend for X11Backend {
    fn inject_text(&self, text: &str) -> Result<()> {
        Command::new("xdotool")
            .arg("type")
            .arg("--clearmodifiers")
            .arg("--")
            .arg(text)
            .output()?;
        Ok(())
    }

    fn send_key_combination(&self, combo: &str) -> Result<()> {
        let xdo_combo = combo.replace('-', "+");
        Command::new("xdotool")
            .arg("key")
            .arg(xdo_combo)
            .output()?;
        Ok(())
    }

    // ... clipboard operations
}
```

### Phase 3: Manager Layer (Week 2)
**Goal:** Create high-level manager with auto-detection

#### Tasks:
1. Implement `DisplayServerManager`
2. Add backend caching
3. Implement fallback logic
4. Add configuration support
5. Integration tests

#### Files to Create:
```
rust-crates/swictation-daemon/src/display/
├── manager.rs          # DisplayServerManager
├── error.rs            # DisplayServerError types
└── config.rs           # Configuration structures
```

#### Implementation:
```rust
pub struct DisplayServerManager {
    backend: Arc<dyn DisplayServerBackend>,
    detection: DetectionResult,
}

impl DisplayServerManager {
    pub fn new() -> Result<Self> {
        let detection = detect_display_server();
        let backend = Self::create_backend(&detection)?;
        Ok(Self { backend, detection })
    }

    fn create_backend(detection: &DetectionResult) -> Result<Arc<dyn DisplayServerBackend>> {
        match detection.server_type {
            DisplayServerType::Wayland => {
                let backend = WaylandBackend;
                backend.validate_tools()?;
                Ok(Arc::new(backend))
            }
            DisplayServerType::X11 => {
                let backend = X11Backend;
                backend.validate_tools()?;
                Ok(Arc::new(backend))
            }
            DisplayServerType::Unknown => {
                // Try Wayland first, fallback to X11
                Self::try_fallback_backend()
            }
        }
    }
}
```

### Phase 4: Integration (Week 2-3)
**Goal:** Replace existing TextInjector with new abstraction

#### Tasks:
1. Update `text_injection.rs` to use DisplayServerManager
2. Migrate `<KEY:...>` marker handling
3. Update daemon initialization
4. Comprehensive end-to-end testing
5. Performance benchmarking

#### Migration:
```rust
// Old: text_injection.rs
pub struct TextInjector {
    display_server: DisplayServer,  // Old enum
}

// New: text_injection.rs
use crate::display::DisplayServerManager;

pub struct TextInjector {
    manager: DisplayServerManager,
}

impl TextInjector {
    pub fn new() -> Result<Self> {
        let manager = DisplayServerManager::new()?;
        info!("Initialized display server: {} (confidence: {:?})",
              manager.backend().name(),
              manager.detection().confidence);
        Ok(Self { manager })
    }

    pub fn inject_text(&self, text: &str) -> Result<()> {
        if text.contains("<KEY:") {
            self.inject_with_keys(text)
        } else {
            self.manager.inject_text(text)
        }
    }

    fn inject_with_keys(&self, text: &str) -> Result<()> {
        // Parse <KEY:...> markers
        // Use manager.send_key_combination() for keys
        // Use manager.inject_text() for text parts
    }
}
```

### Phase 5: Configuration & Polish (Week 3)
**Goal:** Add configuration, documentation, and testing

#### Tasks:
1. Extend config.toml with display section
2. Update README.md with X11 instructions
3. Update npm postinstall script
4. Add troubleshooting documentation
5. Create integration test suite
6. Performance benchmarks

#### Configuration:
```toml
# ~/.config/swictation/config.toml

[display]
# Backend selection: "auto", "wayland", "x11"
backend = "auto"

# Tool paths (optional, auto-detected if not set)
wtype_path = "/usr/bin/wtype"
xdotool_path = "/usr/bin/xdotool"
wl_copy_path = "/usr/bin/wl-copy"
xclip_path = "/usr/bin/xclip"

# Fallback behavior
enable_clipboard_fallback = true
retry_attempts = 3
retry_delay_ms = 100

# Logging
log_backend_selection = true
log_tool_execution = false  # Debug mode only
```

---

## 2. Detailed Implementation Steps

### Step 2.1: Create Module Structure

```bash
# Create directory structure
mkdir -p rust-crates/swictation-daemon/src/display

# Create files
touch rust-crates/swictation-daemon/src/display/{mod.rs,backend.rs,wayland.rs,x11.rs,detection.rs,manager.rs,types.rs,error.rs,config.rs,tools.rs}
```

### Step 2.2: Implement Core Types

**File:** `display/types.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplayServerType {
    X11,
    Wayland,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DetectionConfidence {
    High,      // Multiple confirming signals
    Medium,    // Single reliable signal
    Low,       // Fallback/guessing
}

#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub server_type: DisplayServerType,
    pub confidence: DetectionConfidence,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BackendCapabilities {
    pub supports_text_injection: bool,
    pub supports_key_combinations: bool,
    pub supports_clipboard: bool,
    pub supports_unicode: bool,
    pub requires_external_tools: bool,
}
```

### Step 2.3: Implement Backend Trait

**File:** `display/backend.rs`

```rust
use anyhow::Result;
use crate::display::types::BackendCapabilities;

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
```

### Step 2.4: Implement Detection

**File:** `display/detection.rs`

```rust
use crate::display::types::{DetectionResult, DetectionConfidence, DisplayServerType};

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

    // Determine winner and confidence
    let (server_type, confidence) = if wayland_score > x11_score {
        let conf = match wayland_score {
            s if s >= 4 => DetectionConfidence::High,
            s if s >= 2 => DetectionConfidence::Medium,
            _ => DetectionConfidence::Low,
        };
        (DisplayServerType::Wayland, conf)
    } else if x11_score > wayland_score {
        let conf = match x11_score {
            s if s >= 4 => DetectionConfidence::High,
            s if s >= 2 => DetectionConfidence::Medium,
            _ => DetectionConfidence::Low,
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

pub fn is_xwayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
        && std::env::var("DISPLAY").is_ok()
        && std::env::var("XDG_SESSION_TYPE")
            .map(|t| t == "wayland")
            .unwrap_or(false)
}
```

### Step 2.5: Module Exports

**File:** `display/mod.rs`

```rust
mod backend;
mod types;
mod detection;
mod wayland;
mod x11;
mod manager;
mod error;
mod config;
mod tools;

// Public API
pub use backend::DisplayServerBackend;
pub use types::{DisplayServerType, DetectionResult, DetectionConfidence, BackendCapabilities};
pub use detection::{detect_display_server, is_xwayland};
pub use wayland::WaylandBackend;
pub use x11::X11Backend;
pub use manager::DisplayServerManager;
pub use error::DisplayServerError;
pub use config::DisplayConfig;
```

### Step 2.6: Update main.rs

**File:** `main.rs`

```rust
mod display;  // Add new module
mod pipeline;
mod gpu;
// ... rest

use display::DisplayServerManager;

// In daemon initialization
let display_manager = DisplayServerManager::new()
    .context("Failed to initialize display server")?;

info!("Display server: {} (confidence: {:?})",
      display_manager.backend().name(),
      display_manager.detection().confidence);
```

---

## 3. Testing Strategy

### 3.1 Unit Tests

**File:** `display/wayland.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wayland_detection() {
        std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
        assert!(WaylandBackend::is_available());
    }

    #[test]
    fn test_capabilities() {
        let backend = WaylandBackend;
        let caps = backend.capabilities();
        assert!(caps.supports_text_injection);
        assert!(caps.supports_clipboard);
    }
}
```

### 3.2 Integration Tests

**File:** `tests/display_integration.rs`

```rust
use swictation_daemon::display::*;

#[tokio::test]
async fn test_auto_detection() {
    let manager = DisplayServerManager::new().unwrap();
    assert!(manager.backend().name() == "Wayland" || manager.backend().name() == "X11");
}

#[tokio::test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
async fn test_text_injection() {
    let manager = DisplayServerManager::new().unwrap();

    // Test plain text
    manager.inject_text("test").unwrap();

    // Test clipboard
    manager.set_clipboard("clipboard test").unwrap();
    let content = manager.get_clipboard().unwrap();
    assert_eq!(content, "clipboard test");
}
```

---

## 4. Risk Mitigation

### Risk 1: X11 Tool Availability
**Mitigation:**
- Check tools during initialization
- Provide clear error messages with installation instructions
- npm postinstall script checks and warns

### Risk 2: Detection Accuracy
**Mitigation:**
- Evidence-based scoring system
- Multiple detection methods
- Manual override in config
- Log detection details

### Risk 3: Breaking Existing Functionality
**Mitigation:**
- Maintain backward compatibility
- Wayland backend is refactor, not rewrite
- Comprehensive test suite
- Feature flag for X11 support

### Risk 4: Performance Regression
**Mitigation:**
- Benchmark before and after
- Cache backend instance
- Use Arc for cheap cloning
- Profile with criterion

---

## 5. Success Criteria

### Must Have:
- ✅ Wayland support unchanged
- ✅ X11 text injection works
- ✅ X11 clipboard works
- ✅ Auto-detection accurate
- ✅ Tests pass on both X11 and Wayland

### Should Have:
- ✅ Configuration support
- ✅ Clear error messages
- ✅ Documentation updated
- ✅ npm package works on X11

### Nice to Have:
- 🔄 Native X11 implementation (future)
- 🔄 XWayland optimization
- 🔄 Performance benchmarks
- 🔄 CI/CD for multiple display servers

---

## 6. Timeline

| Week | Phase | Deliverables |
|------|-------|--------------|
| 1 | Foundation | Trait, Wayland backend, detection |
| 1-2 | X11 Backend | X11 implementation, tool validation |
| 2 | Manager | DisplayServerManager, config support |
| 2-3 | Integration | Update TextInjector, testing |
| 3 | Polish | Documentation, benchmarks, release |

**Total Estimated Effort:** 3 weeks (part-time)

---

## 7. Rollout Plan

### Step 1: Development (Week 1-3)
- Implement in feature branch: `feature/x11-support`
- Regular testing on X11 and Wayland
- Code review before merge

### Step 2: Beta Testing (Week 3-4)
- Release as beta: v0.3.0-beta.1
- Community testing
- Gather feedback
- Fix issues

### Step 3: Stable Release (Week 4-5)
- Release v0.3.0
- Update documentation
- Announce in README
- Update npm package

### Step 4: Monitoring (Ongoing)
- Track GitHub issues
- Monitor user reports
- Performance metrics
- Plan future improvements

---

## Conclusion

This implementation strategy provides:

✅ **Clear phases** with defined deliverables
✅ **Risk mitigation** for common pitfalls
✅ **Testing strategy** for quality assurance
✅ **Timeline** for planning and tracking
✅ **Success criteria** for validation

The approach is **incremental, testable, and reversible**, minimizing risk while maximizing compatibility and maintainability.
