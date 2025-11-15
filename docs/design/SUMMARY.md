# Display Server Abstraction - Design Summary

**Project:** Swictation Voice-to-Text Daemon
**Component:** Display Server Abstraction Layer
**Version:** 1.0
**Date:** 2025-11-14
**Status:** Design Complete

---

## Quick Links

1. **[Display Server Abstraction](display-server-abstraction.md)** - Complete trait design and backend implementations
2. **[X11 Dependencies](x11-dependencies.md)** - Required dependencies and migration phases
3. **[Implementation Strategy](implementation-strategy.md)** - Detailed implementation plan and timeline
4. **[Detection Mechanism](detection-mechanism.md)** - Runtime display server detection algorithm

---

## Executive Summary

The Coder Agent has designed a comprehensive **trait-based display server abstraction layer** that enables dual X11/Wayland support for Swictation while maintaining backward compatibility.

### Key Design Decisions

✅ **Trait-Based Architecture**
- `DisplayServerBackend` trait for clean abstraction
- Separate implementations for Wayland and X11
- Easy to extend with future backends (macOS, Windows, native protocols)

✅ **Runtime Detection**
- Evidence-based scoring system (0-10 points)
- Confidence levels (High/Medium/Low)
- Handles edge cases (XWayland, SSH sessions, headless)

✅ **External Tools First**
- Phase 1: Use `xdotool`/`xclip` (no new Rust dependencies)
- Phase 2: Optional native X11 via `x11` crate (future)
- Phase 3: Optional native Wayland via `libei` (future)

✅ **Backward Compatibility**
- Existing Wayland code refactored into new abstraction
- No breaking changes to public API
- Existing `TextInjector` updated to use new backend

---

## Architecture Overview

```
┌─────────────────────────────────────────────────┐
│           TextInjector (High-Level)             │
│  - Handles <KEY:...> markers                    │
│  - Delegates to DisplayServerManager            │
└─────────────────────┬───────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────┐
│        DisplayServerManager (Facade)            │
│  - Auto-detection or manual override            │
│  - Backend caching                              │
│  - Error handling and fallback                  │
└─────────────────────┬───────────────────────────┘
                      │
         ┌────────────┴────────────┐
         ▼                         ▼
┌──────────────────┐      ┌──────────────────┐
│ WaylandBackend   │      │   X11Backend     │
│ - wtype          │      │ - xdotool        │
│ - wl-clipboard   │      │ - xclip          │
└──────────────────┘      └──────────────────┘
         │                         │
         └─────────┬───────────────┘
                   ▼
        ┌─────────────────────┐
        │ DisplayServerBackend│
        │       (Trait)       │
        └─────────────────────┘
```

---

## Implementation Timeline

| Week | Phase | Deliverables |
|------|-------|--------------|
| **1** | Foundation | Trait, Wayland backend, detection |
| **1-2** | X11 Backend | X11 implementation, tool validation |
| **2** | Manager | DisplayServerManager, config |
| **2-3** | Integration | Update TextInjector, testing |
| **3** | Polish | Documentation, benchmarks, release |

**Total Effort:** 3 weeks (part-time)

---

## File Structure

### New Files Created

```
rust-crates/swictation-daemon/src/display/
├── mod.rs              # Module exports
├── backend.rs          # DisplayServerBackend trait
├── types.rs            # DisplayServerType, DetectionResult
├── wayland.rs          # WaylandBackend implementation
├── x11.rs              # X11Backend implementation
├── detection.rs        # detect_display_server(), is_xwayland()
├── manager.rs          # DisplayServerManager
├── error.rs            # DisplayServerError types
├── config.rs           # DisplayConfig structures
└── tools.rs            # External tool validation
```

### Modified Files

```
rust-crates/swictation-daemon/src/
├── main.rs             # Add display module, use DisplayServerManager
├── text_injection.rs   # Update to use DisplayServerManager
└── config.rs           # Add display configuration section
```

### Documentation Files

```
docs/design/
├── SUMMARY.md                      # This file
├── display-server-abstraction.md   # Complete design spec
├── x11-dependencies.md             # Dependencies and phases
├── implementation-strategy.md      # Implementation plan
└── detection-mechanism.md          # Detection algorithm
```

---

## Key Components

### 1. DisplayServerBackend Trait

```rust
pub trait DisplayServerBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn inject_text(&self, text: &str) -> Result<()>;
    fn send_key_combination(&self, combo: &str) -> Result<()>;
    fn set_clipboard(&self, text: &str) -> Result<()>;
    fn get_clipboard(&self) -> Result<String>;
    fn is_available() -> bool where Self: Sized;
    fn validate_tools(&self) -> Result<()>;
    fn capabilities(&self) -> BackendCapabilities;
}
```

### 2. Detection Algorithm

**Scoring System:**
- `WAYLAND_DISPLAY` set → +3 points (Wayland)
- `XDG_SESSION_TYPE=wayland` → +3 points (Wayland)
- `XDG_SESSION_TYPE=x11` → +3 points (X11)
- `WAYLAND_COMPOSITOR` → +2 points (Wayland)
- `DISPLAY` set → +1 point (X11)
- `GDK_BACKEND` → +1 point (either)

**Confidence:**
- Score ≥ 4 → High confidence
- Score ≥ 2 → Medium confidence
- Score < 2 → Low confidence

### 3. X11 Dependencies

**System Packages:**
```bash
# Ubuntu/Debian
sudo apt install xdotool xclip

# Arch/Manjaro
sudo pacman -S xdotool xclip
```

**Rust Crates (Future):**
```toml
# Optional, for native X11 implementation
x11 = { version = "2.21", features = ["xlib", "xtest"], optional = true }
x11-clipboard = { version = "0.9", optional = true }
```

### 4. Configuration

```toml
# ~/.config/swictation/config.toml
[display]
backend = "auto"  # Options: "auto", "wayland", "x11"
enable_clipboard_fallback = true
retry_attempts = 3
retry_delay_ms = 100
```

---

## Testing Strategy

### Unit Tests
- Detection logic with various env combinations
- XWayland detection
- Backend capabilities
- Tool validation

### Integration Tests
- Text injection on X11 and Wayland
- Clipboard operations
- Key combinations
- Error handling and fallbacks

### Test Matrix
- Sway/Wayland ✅
- GNOME/Wayland 🔄
- KDE Plasma/Wayland 🔄
- i3/X11 🔄
- GNOME/X11 🔄
- KDE Plasma/X11 🔄
- XWayland 🔄

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| X11 tool availability | Check during init, clear error messages |
| Detection accuracy | Evidence-based scoring, manual override |
| Breaking existing code | Backward compatibility, comprehensive tests |
| Performance regression | Benchmark before/after, cache backend |

---

## Success Criteria

### Must Have ✅
- Wayland support unchanged
- X11 text injection works
- X11 clipboard works
- Auto-detection accurate (≥95%)
- Tests pass on both X11 and Wayland

### Should Have 🔄
- Configuration support
- Clear error messages
- Documentation updated
- npm package works on X11

### Nice to Have 🔄
- Native X11 implementation (future)
- XWayland optimization
- Performance benchmarks
- CI/CD for multiple display servers

---

## Design Deliverables

All designs stored in collective memory under namespace `workers/coder/`:

1. ✅ **abstraction_trait_design** - Complete trait and backend design
2. ✅ **x11_dependencies** - Dependencies and migration phases
3. ✅ **implementation_strategy** - Detailed implementation plan
4. ✅ **detection_mechanism** - Runtime detection algorithm

---

## Next Steps

### Immediate (For Planner/Architect)
1. Review design documents
2. Create implementation task breakdown
3. Assign tasks to specialized agents
4. Set up X11 test environment

### Short-Term (Week 1)
1. Implement core trait and types
2. Refactor existing Wayland code
3. Implement detection logic
4. Add unit tests

### Medium-Term (Week 2-3)
1. Implement X11 backend
2. Create DisplayServerManager
3. Update TextInjector
4. Comprehensive testing

### Long-Term (Future)
1. Native X11 implementation
2. Native Wayland protocol
3. macOS/Windows support
4. Performance optimizations

---

## Conclusion

The design provides a **solid foundation** for dual X11/Wayland support:

✅ **Clean abstraction** via trait-based design
✅ **Runtime detection** with evidence-based confidence
✅ **Backward compatibility** with existing code
✅ **Extensibility** for future backends
✅ **Robust testing** strategy
✅ **Clear migration** path

The approach is **incremental, testable, and reversible**, minimizing risk while establishing a maintainable architecture for cross-platform display server support.

---

**Design Status:** ✅ **COMPLETE**
**Next Phase:** Implementation (awaiting task assignment)
**Collective Memory:** All designs stored in `workers/coder/` namespace
