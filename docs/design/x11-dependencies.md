# X11 Support Dependencies and Implementation Plan

**Version:** 1.0
**Date:** 2025-11-14
**Author:** Coder Agent (Hive Mind)
**Project:** Swictation Display Server Abstraction

---

## 1. Required X11 Dependencies

### 1.1 System-Level Dependencies

#### Ubuntu/Debian
```bash
sudo apt install \
    xdotool \        # Text injection and key simulation
    xclip \          # Clipboard operations
    x11-utils \      # X11 utilities (xprop, xwininfo)
    libx11-dev \     # X11 development headers (if needed for native impl)
    libxtst-dev      # XTEST extension (for future native impl)
```

#### Arch/Manjaro
```bash
sudo pacman -S \
    xdotool \
    xclip \
    xorg-xprop \
    xorg-xwininfo \
    libx11 \
    libxtst
```

### 1.2 External Tools

| Tool | Purpose | Package | Alternatives |
|------|---------|---------|--------------|
| **xdotool** | Text injection, key simulation | `xdotool` | xte, xvkbd |
| **xclip** | Clipboard operations | `xclip` | xsel |
| xprop | Window property inspection | `x11-utils` | wmctrl |
| xwininfo | Window information | `x11-utils` | - |

### 1.3 Rust Crates (Future Native Implementation)

For eventual native X11 support without external tools:

```toml
[dependencies]
# X11 protocol bindings
x11 = { version = "2.21", features = ["xlib", "xtest"] }
x11-clipboard = "0.9"  # Clipboard support

# Alternative: xcb (X protocol C-language Binding)
xcb = { version = "1.4", features = ["xtest"] }

# Or use rust-xcb for better Rust integration
rust-xcb = "1.4"
```

**Note**: Initial implementation uses external tools (`xdotool`, `xclip`) for reliability and compatibility. Native X11 protocol implementation can be added later as an optimization.

---

## 2. Cargo.toml Changes

### 2.1 Initial Implementation (External Tools)

**No new Rust dependencies required** for the initial implementation, as it uses external tools via `std::process::Command`.

### 2.2 Feature Flags

Add feature flags to enable optional native X11 support later:

```toml
# rust-crates/swictation-daemon/Cargo.toml

[features]
default = ["external-tools"]

# Use external tools (xdotool, xclip)
external-tools = []

# Use native X11 protocol (future enhancement)
native-x11 = ["x11", "x11-clipboard"]

# Use native XCB protocol (alternative to Xlib)
native-xcb = ["xcb"]

[dependencies]
# Only included when native-x11 feature is enabled
x11 = { version = "2.21", features = ["xlib", "xtest"], optional = true }
x11-clipboard = { version = "0.9", optional = true }

# Only included when native-xcb feature is enabled
xcb = { version = "1.4", features = ["xtest"], optional = true }
```

---

## 3. Implementation Phases

### Phase 1: External Tools (Current)
**Timeline:** Week 1
**Effort:** Low
**Risk:** Low

✅ Use `xdotool` for text injection and key combinations
✅ Use `xclip` for clipboard operations
✅ Pure Rust wrapper with error handling
✅ No new Rust dependencies
✅ Battle-tested tools with wide compatibility

**Pros:**
- Quick implementation
- Highly compatible
- No linking issues
- Well-documented

**Cons:**
- Spawns external processes (performance overhead)
- Requires tools to be installed
- Less control over exact behavior

### Phase 2: Native Xlib (Future)
**Timeline:** Week 2-3
**Effort:** Medium
**Risk:** Medium

🔄 Direct X11 protocol via Xlib bindings
🔄 Eliminate external tool dependencies
🔄 Better performance (no process spawning)
🔄 More control over operations

**Dependencies:**
```toml
x11 = { version = "2.21", features = ["xlib", "xtest"] }
x11-clipboard = "0.9"
```

**Challenges:**
- Unsafe Rust code required
- X11 API complexity
- Error handling complexity
- Thread safety considerations

### Phase 3: XCB Alternative (Optional)
**Timeline:** Week 4+
**Effort:** High
**Risk:** High

🔄 Modern X11 protocol via XCB
🔄 Better async support
🔄 Cleaner API than Xlib

**Dependencies:**
```toml
xcb = { version = "1.4", features = ["xtest"] }
```

---

## 4. Installation Documentation Updates

### 4.1 README.md Additions

```markdown
### X11 Support (Optional)

For X11 display servers, install the following tools:

```bash
# Ubuntu/Debian
sudo apt install xdotool xclip

# Arch/Manjaro
sudo pacman -S xdotool xclip

# Fedora
sudo dnf install xdotool xclip
```

**Auto-detection:** Swictation automatically detects your display server
and uses the appropriate backend (X11 or Wayland).

**Manual override:**
```toml
# ~/.config/swictation/config.toml
[display]
backend = "x11"  # Force X11 mode
```
```

### 4.2 npm Package postinstall Script

```javascript
// npm-package/scripts/postinstall.js

async function checkDisplayServer() {
    const isWayland = process.env.WAYLAND_DISPLAY !== undefined;
    const isX11 = process.env.DISPLAY !== undefined &&
                  process.env.XDG_SESSION_TYPE === 'x11';

    if (isWayland) {
        console.log('✓ Detected Wayland display server');
        await checkTool('wtype', 'sudo apt install wtype');
        await checkTool('wl-copy', 'sudo apt install wl-clipboard');
    } else if (isX11) {
        console.log('✓ Detected X11 display server');
        await checkTool('xdotool', 'sudo apt install xdotool');
        await checkTool('xclip', 'sudo apt install xclip');
    } else {
        console.warn('⚠ Could not detect display server');
        console.log('Install both Wayland and X11 tools for maximum compatibility');
    }
}
```

---

## 5. Testing Requirements

### 5.1 Environment Matrix

Test the abstraction on:

| Environment | Display Server | Tools | Test Status |
|-------------|----------------|-------|-------------|
| Sway/Wayland | Wayland | wtype, wl-clipboard | ✅ Primary |
| GNOME/Wayland | Wayland | wtype, wl-clipboard | 🔄 Test needed |
| KDE Plasma/Wayland | Wayland | wtype, wl-clipboard | 🔄 Test needed |
| i3/X11 | X11 | xdotool, xclip | 🔄 Test needed |
| GNOME/X11 | X11 | xdotool, xclip | 🔄 Test needed |
| KDE Plasma/X11 | X11 | xdotool, xclip | 🔄 Test needed |
| XWayland | Wayland+X11 | Both | 🔄 Test needed |

### 5.2 Test Cases

```rust
// tests/x11_backend.rs

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_x11_text_injection() {
    // Setup X11 test environment
    let backend = X11Backend;
    assert!(backend.validate_tools().is_ok());

    // Test text injection
    let result = backend.inject_text("test");
    assert!(result.is_ok());
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_x11_clipboard() {
    let backend = X11Backend;

    // Test clipboard operations
    backend.set_clipboard("test data").unwrap();
    let content = backend.get_clipboard().unwrap();
    assert_eq!(content, "test data");
}

#[test]
fn test_x11_key_combinations() {
    let backend = X11Backend;

    // Test various key combinations
    let combos = vec![
        "ctrl-c",
        "alt-tab",
        "super-d",
        "shift-insert",
    ];

    for combo in combos {
        let result = backend.send_key_combination(combo);
        assert!(result.is_ok(), "Failed: {}", combo);
    }
}
```

---

## 6. Security Considerations

### 6.1 Command Injection Prevention

```rust
impl X11Backend {
    fn inject_text(&self, text: &str) -> Result<()> {
        // ✅ Safe: xdotool type takes text as argument (no shell injection)
        Command::new("xdotool")
            .arg("type")
            .arg("--clearmodifiers")
            .arg("--")  // Separator to prevent flag injection
            .arg(text)  // Text is passed as argument, not shell string
            .output()
            .context("Failed to inject text with xdotool")?;
        Ok(())
    }
}
```

### 6.2 Tool Path Validation

```rust
pub fn validate_tool_path(tool: &str) -> Result<PathBuf> {
    // Only allow tools from standard system paths
    let allowed_paths = vec![
        "/usr/bin",
        "/usr/local/bin",
        "/bin",
    ];

    let path = which::which(tool)
        .context(format!("Tool not found: {}", tool))?;

    // Verify tool is in allowed path
    if !allowed_paths.iter().any(|p| path.starts_with(p)) {
        anyhow::bail!("Tool {} not in allowed path: {:?}", tool, path);
    }

    Ok(path)
}
```

---

## 7. Performance Benchmarks

### 7.1 Expected Latency

| Operation | External Tool | Native X11 | Improvement |
|-----------|---------------|------------|-------------|
| Text injection (10 chars) | ~5-10ms | ~1-2ms | 5-10x |
| Key combination | ~5-8ms | ~1ms | 5-8x |
| Clipboard set | ~3-5ms | ~0.5-1ms | 3-5x |
| Clipboard get | ~3-5ms | ~0.5-1ms | 3-5x |

### 7.2 Benchmark Implementation

```rust
// benches/display_backend.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_text_injection(c: &mut Criterion) {
    let manager = DisplayServerManager::new().unwrap();

    c.bench_function("inject_10_chars", |b| {
        b.iter(|| {
            manager.inject_text(black_box("hello test")).unwrap();
        });
    });
}

criterion_group!(benches, bench_text_injection);
criterion_main!(benches);
```

---

## 8. Migration Checklist

### Pre-Implementation
- [ ] Review existing text_injection.rs implementation
- [ ] Document current behavior and edge cases
- [ ] Create feature branch: `feature/x11-support`
- [ ] Set up X11 test environment

### Implementation
- [ ] Create display module structure
- [ ] Implement DisplayServerBackend trait
- [ ] Implement WaylandBackend
- [ ] Implement X11Backend
- [ ] Implement detection logic
- [ ] Implement DisplayServerManager
- [ ] Update text_injection.rs to use new abstraction
- [ ] Add configuration support

### Testing
- [ ] Unit tests for detection logic
- [ ] Unit tests for each backend
- [ ] Integration tests on X11
- [ ] Integration tests on Wayland
- [ ] Integration tests on XWayland
- [ ] Performance benchmarks

### Documentation
- [ ] Update README.md with X11 instructions
- [ ] Update installation docs
- [ ] Add troubleshooting section for X11
- [ ] Document configuration options
- [ ] Add architecture diagram

### Deployment
- [ ] Update npm postinstall script
- [ ] Update systemd service examples
- [ ] Test on clean Ubuntu 24.04 (X11 + Wayland)
- [ ] Test on Arch Linux
- [ ] Create release notes
- [ ] Tag version: v0.3.x

---

## 9. Rollback Plan

### If X11 Support Fails

1. **Keep feature flag disabled by default**
   ```toml
   [features]
   default = []  # Don't enable X11 by default
   x11-support = ["dep:x11", "dep:x11-clipboard"]
   ```

2. **Maintain backward compatibility**
   - Existing Wayland code unchanged
   - X11 code is additive, not replacing

3. **Graceful degradation**
   ```rust
   match DisplayServerManager::new() {
       Ok(manager) => manager,
       Err(e) => {
           warn!("Display server detection failed: {}", e);
           warn!("Falling back to Wayland-only mode");
           // Use old TextInjector
       }
   }
   ```

---

## 10. Future Enhancements

### 10.1 Native Protocol Implementation

```rust
// Future: native X11 implementation
#[cfg(feature = "native-x11")]
pub struct NativeX11Backend {
    display: *mut x11::xlib::Display,
}

#[cfg(feature = "native-x11")]
impl NativeX11Backend {
    pub fn new() -> Result<Self> {
        unsafe {
            let display = x11::xlib::XOpenDisplay(std::ptr::null());
            if display.is_null() {
                anyhow::bail!("Failed to open X11 display");
            }
            Ok(Self { display })
        }
    }

    fn inject_text_native(&self, text: &str) -> Result<()> {
        // Use XTestFakeKeyEvent for direct protocol injection
        unsafe {
            for ch in text.chars() {
                let keycode = self.char_to_keycode(ch)?;
                x11::xtest::XTestFakeKeyEvent(
                    self.display,
                    keycode,
                    1, // Press
                    0, // No delay
                );
                x11::xtest::XTestFakeKeyEvent(
                    self.display,
                    keycode,
                    0, // Release
                    0,
                );
            }
            x11::xlib::XFlush(self.display);
        }
        Ok(())
    }
}
```

### 10.2 Wayland Native Protocol

```rust
// Future: native Wayland implementation using libei
#[cfg(feature = "native-wayland")]
pub struct NativeWaylandBackend {
    // Direct libei integration
}
```

---

## Conclusion

### Summary

✅ **Phase 1 (Immediate):** External tools (`xdotool`, `xclip`)
- Zero new Rust dependencies
- Quick implementation
- High compatibility

🔄 **Phase 2 (Future):** Native X11 protocol
- Add `x11` and `x11-clipboard` crates
- Better performance
- Optional feature flag

🔄 **Phase 3 (Optional):** Native Wayland protocol
- Investigate libei integration
- Future-proof architecture

### Recommended Approach

1. **Start with external tools** - proven, reliable, fast to implement
2. **Add feature flags** - prepare for future native implementation
3. **Benchmark and optimize** - only add native impl if performance matters
4. **Maintain backward compatibility** - never break existing Wayland support

The design prioritizes **reliability and compatibility** over raw performance, which is appropriate for a voice-to-text daemon where user typing latency (0.8s silence threshold) far exceeds tool invocation overhead (5-10ms).
