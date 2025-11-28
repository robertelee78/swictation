# Cross-Platform Architecture Summary

## Overview

This document summarizes the proposed cross-platform abstraction layer for Swictation supporting Linux, macOS, and Windows.

## Documents Created

1. **`cross-platform-abstraction-design.md`** - Complete architectural design
2. **`path-abstraction-implementation.md`** - Concrete implementation guide for path module

## Key Design Decisions

### 1. Path Abstraction Module (`swictation-paths`)

**Status**: 🟢 Complete Design + Implementation Ready

**Purpose**: Single source of truth for all platform-specific paths

**API Highlights**:
- `PlatformPaths::data_dir()` - Application data (metrics.db, models)
- `PlatformPaths::config_dir()` - Configuration files
- `PlatformPaths::runtime_dir()` - IPC sockets/pipes
- `PlatformPaths::log_dir()` - Application logs
- `PlatformPaths::cache_dir()` - Temporary cache
- `PlatformPaths::ipc_endpoint()` - Cross-platform IPC identifier

**Implementation**: See `path-abstraction-implementation.md` for complete code

### 2. IPC Abstraction Module (`swictation-ipc`)

**Status**: 🟡 Design Complete, Implementation Pending

**Purpose**: Cross-platform IPC with Unix sockets (Linux/macOS) and named pipes (Windows)

**Key Features**:
- Trait-based design (`IpcTransport`, `IpcStream`)
- Async/await with Tokio
- Automatic platform selection
- Secure permissions (Unix: 0600, Windows: ACLs)

**Current Gap**: No Windows named pipe implementation exists

### 3. Library Loading Abstraction (`swictation-libs`)

**Status**: 🟡 Design Complete, Implementation Pending

**Purpose**: Cross-platform dynamic library loading for ONNX Runtime

**Key Features**:
- Auto-detection of ONNX Runtime (.so/.dylib/.dll)
- Python package detection
- Environment variable override (ORT_DYLIB_PATH)
- Platform-specific search paths

**Current Gap**: Uses environment variable only, no systematic detection

### 4. Text Injection

**Status**: 🟢 Linux/macOS Complete, Windows Pending

**Current State**:
- ✅ Linux: xdotool/wtype/ydotool (comprehensive)
- ✅ macOS: Core Graphics API (complete)
- ❌ Windows: Not implemented

**Required**: Windows SendInput API implementation

### 5. Hotkey Handling

**Status**: 🟢 Cross-platform Complete

**Current State**:
- ✅ Uses `global-hotkey` crate (X11, macOS, Windows)
- ✅ Special handling for Wayland (GNOME, Sway)
- ✅ Excellent platform abstraction already in place

**No changes needed** - well-designed and complete

## Platform Support Matrix

| Component | Linux | macOS | Windows | Status |
|-----------|-------|-------|---------|--------|
| **Paths** | ✅ Partial | ✅ Partial | ❌ None | 🟡 Design Ready |
| **IPC** | ✅ Complete | ✅ Complete | ❌ None | 🟡 Design Ready |
| **Libraries** | ⚠️ Manual | ⚠️ Manual | ❌ None | 🟡 Design Ready |
| **Text Inject** | ✅ Complete | ✅ Complete | ❌ None | 🟡 Design Ready |
| **Hotkeys** | ✅ Complete | ✅ Complete | ⚠️ Untested | 🟢 Implemented |

Legend:
- ✅ Complete - Fully implemented and tested
- ⚠️ Manual - Works but requires manual setup
- ❌ None - Not implemented
- 🟢 No action needed
- 🟡 Design ready, implementation pending

## Current Codebase Analysis

### Well-Designed Components

1. **Display Server Detection** (`display_server.rs`)
   - Evidence-based platform detection
   - Excellent pattern for abstraction
   - Comprehensive Wayland handling (GNOME, KDE, Sway)

2. **Hotkey System** (`hotkey.rs`)
   - Cross-platform via `global-hotkey` crate
   - Smart fallback to IPC for Wayland
   - Auto-configuration for GNOME/Sway

3. **Text Injection** (`text_injection.rs`, `macos_text_inject.rs`)
   - Clean separation of concerns
   - Platform-specific modules
   - Keyboard shortcut support

### Components Needing Consolidation

1. **Path Management** (Fragmented)
   - Currently in: `socket_utils.rs`, `config.rs`, `utils/mod.rs`, `socket-paths.js`
   - Should be: Single `swictation-paths` crate
   - Benefit: Consistency, testability, Windows support

2. **IPC** (Unix-only)
   - Currently: `ipc.rs` (Unix sockets only)
   - Should be: `swictation-ipc` crate with trait abstraction
   - Benefit: Windows named pipe support

3. **Library Loading** (Environment variable only)
   - Currently: Manual ORT_DYLIB_PATH setup
   - Should be: `swictation-libs` with auto-detection
   - Benefit: Better UX, multi-platform support

## Implementation Roadmap

### Phase 1: Path Abstraction (Week 1-2)
- ✅ Design complete
- ✅ Implementation guide ready
- Next: Create crate, port existing code

### Phase 2: IPC Abstraction (Week 3-4)
- ✅ Design complete
- Next: Implement Unix socket backend (port existing)
- Next: Implement Windows named pipe backend

### Phase 3: Library Loading (Week 5)
- ✅ Design complete
- Next: Implement auto-detection
- Next: Add Python package detection

### Phase 4: Windows Support (Week 6-7)
- ✅ Text injection design ready
- Next: Implement SendInput API
- Next: Test hotkeys on Windows

### Phase 5: Testing & Integration (Week 8)
- Cross-platform CI/CD
- Comprehensive testing
- Documentation updates
- Migration guide

## Key Architectural Patterns

### 1. Platform Detection at Compile Time

```rust
#[cfg(target_os = "linux")]
fn linux_impl() { ... }

#[cfg(target_os = "macos")]
fn macos_impl() { ... }

#[cfg(target_os = "windows")]
fn windows_impl() { ... }
```

**Benefit**: Zero runtime overhead, type-safe platform selection

### 2. Trait-Based Abstraction

```rust
pub trait IpcTransport {
    async fn bind(endpoint: &IpcEndpoint) -> Result<Self>;
    async fn accept(&mut self) -> Result<Box<dyn IpcStream>>;
}
```

**Benefit**: Testability, extensibility, consistent API

### 3. Evidence-Based Detection

From `display_server.rs`:
```rust
// Score-based detection
let mut x11_score = 0;
let mut wayland_score = 0;

if session_type == "wayland" { wayland_score += 4; }
if wayland_display.is_some() { wayland_score += 2; }
```

**Benefit**: Robust detection in ambiguous environments (XWayland)

### 4. Secure by Default

```rust
// Unix socket: 0600 permissions (owner-only)
let permissions = std::fs::Permissions::from_mode(0o600);
std::fs::set_permissions(socket_path, permissions)?;
```

**Benefit**: Security hardening, follows best practices

## File Organization

```
rust-crates/
├── swictation-paths/       # NEW - Path abstraction
├── swictation-ipc/         # NEW - IPC abstraction
├── swictation-libs/        # NEW - Library loading
├── swictation-daemon/
│   ├── src/
│   │   ├── windows_text_inject.rs  # NEW - Windows text injection
│   │   ├── text_injection.rs       # MODIFY - Add Windows support
│   │   ├── display_server.rs       # ENHANCE - Windows detection
│   │   ├── hotkey.rs               # ENHANCE - Windows testing
│   │   ├── socket_utils.rs         # REMOVE - Use swictation-paths
│   │   └── config.rs               # MODIFY - Use swictation-paths
└── (existing crates...)
```

## Migration Impact

### Breaking Changes
**None** - All changes are internal, no user-facing API changes

### Path Locations
**No changes** - Linux/macOS paths remain the same

### Dependencies
**New crates**:
- `swictation-paths` - Zero new dependencies (uses existing `dirs`)
- `swictation-ipc` - Uses existing `tokio`
- `swictation-libs` - Minimal (`libloading`, `shellexpand`)

### Performance
**Impact**: Negligible (<0.1% overhead from trait dispatch, compile-time optimized)

## Success Criteria

- [ ] All existing Linux functionality preserved
- [ ] All existing macOS functionality preserved
- [ ] Full Windows support (IPC, text injection, hotkeys)
- [ ] Comprehensive test coverage (>80%)
- [ ] CI/CD for all platforms
- [ ] Documentation complete
- [ ] Zero breaking changes for users

## Quick Start for Implementation

1. **Read**: `cross-platform-abstraction-design.md` for architecture
2. **Implement**: `path-abstraction-implementation.md` for first module
3. **Test**: Run on Linux/macOS/Windows
4. **Iterate**: IPC → Libraries → Windows features

## Questions & Decisions

### Resolved
- ✅ Use trait-based abstraction for IPC
- ✅ Use compile-time platform selection for paths
- ✅ Maintain backward compatibility
- ✅ Follow XDG spec on Linux

### Pending
- ⏳ Windows installer/packaging strategy
- ⏳ Windows service vs. background app
- ⏳ Windows hotkey registration (startup)
- ⏳ Code signing for Windows binaries

## References

- **Design Doc**: `cross-platform-abstraction-design.md`
- **Implementation**: `path-abstraction-implementation.md`
- **Existing Code**: See analysis in design doc
- **Standards**: XDG Base Directory, Apple FSP, Windows Known Folders
