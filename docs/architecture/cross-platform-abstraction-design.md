# Cross-Platform Abstraction Layer Design for Swictation

**Status**: Architectural Design
**Created**: 2025-11-28
**Target Platforms**: Linux, macOS, Windows

## Executive Summary

This document proposes a comprehensive cross-platform abstraction layer for Swictation to support Linux, macOS, and Windows. The current codebase has partial platform abstractions scattered across multiple modules. This design consolidates and extends them into a cohesive architecture.

## Current State Analysis

### Existing Platform-Specific Code

**Path Management** (Partial):
- `tauri-ui/src-tauri/src/utils/mod.rs` - Database paths only
- `tauri-ui/src-tauri/src/socket/socket_utils.rs` - Socket paths (Unix only)
- `rust-crates/swictation-daemon/src/socket_utils.rs` - Socket paths with security
- `rust-crates/swictation-daemon/src/config.rs` - Config paths, model directories
- `npm-package/src/socket-paths.js` - JavaScript socket path resolution

**IPC Mechanisms**:
- `rust-crates/swictation-daemon/src/ipc.rs` - Unix sockets only
- No Windows named pipe implementation

**Library Loading**:
- `rust-crates/swictation-stt/src/recognizer_ort.rs` - ONNX Runtime via ORT_DYLIB_PATH
- Uses environment variable, no cross-platform abstraction

**Text Injection**:
- `rust-crates/swictation-daemon/src/text_injection.rs` - Linux (xdotool/wtype/ydotool)
- `rust-crates/swictation-daemon/src/macos_text_inject.rs` - macOS Core Graphics
- Well-abstracted, complete

**Hotkey Handling**:
- `rust-crates/swictation-daemon/src/hotkey.rs` - Cross-platform via global-hotkey crate
- Special handling for Sway, GNOME Wayland
- Well-abstracted, complete

**Display Server Detection**:
- `rust-crates/swictation-daemon/src/display_server.rs` - Linux/macOS only
- Evidence-based X11/Wayland detection
- Excellent design pattern for platform abstraction

### Gaps Identified

1. **Windows Support**: No Windows implementation for any component
2. **IPC**: No Windows named pipe abstraction
3. **Library Loading**: No systematic cross-platform dynamic library resolution
4. **Path Management**: Fragmented across multiple files, no single source of truth
5. **Logging/Cache Directories**: Not explicitly managed

## Proposed Architecture

### 1. Path Abstraction Module (`swictation-paths` crate)

**Location**: `/opt/swictation/rust-crates/swictation-paths/`

**Purpose**: Single source of truth for all platform-specific paths.

**API Design**:

```rust
pub struct PlatformPaths {
    platform: Platform,
}

pub enum Platform {
    Linux,
    MacOS,
    Windows,
}

impl PlatformPaths {
    /// Get platform-specific paths
    pub fn new() -> Self;

    /// Data directory (metrics.db, models, learned corrections)
    /// - Linux: ~/.local/share/swictation/
    /// - macOS: ~/Library/Application Support/swictation/
    /// - Windows: %LOCALAPPDATA%\Swictation\
    pub fn data_dir() -> Result<PathBuf>;

    /// Config directory (config.toml, corrections.toml)
    /// - Linux: ~/.config/swictation/
    /// - macOS: ~/Library/Application Support/com.swictation.daemon/
    /// - Windows: %APPDATA%\Swictation\
    pub fn config_dir() -> Result<PathBuf>;

    /// Runtime/socket directory (Unix sockets, named pipes)
    /// - Linux: $XDG_RUNTIME_DIR or ~/.local/share/swictation/
    /// - macOS: ~/Library/Application Support/swictation/
    /// - Windows: \\.\pipe\swictation\ (named pipe namespace)
    pub fn runtime_dir() -> Result<PathBuf>;

    /// Log directory
    /// - Linux: ~/.local/share/swictation/logs/
    /// - macOS: ~/Library/Logs/swictation/
    /// - Windows: %LOCALAPPDATA%\Swictation\logs\
    pub fn log_dir() -> Result<PathBuf>;

    /// Cache directory (temporary files, download cache)
    /// - Linux: ~/.cache/swictation/
    /// - macOS: ~/Library/Caches/swictation/
    /// - Windows: %LOCALAPPDATA%\Swictation\cache\
    pub fn cache_dir() -> Result<PathBuf>;

    /// Model directory (ML models)
    /// Respects SWICTATION_MODEL_PATH env var
    pub fn model_dir() -> Result<PathBuf>;

    /// Database path (metrics.db)
    pub fn database_path() -> Result<PathBuf>;

    /// IPC endpoint path/identifier
    /// - Linux/macOS: PathBuf to Unix socket
    /// - Windows: Named pipe identifier (string)
    pub fn ipc_endpoint() -> Result<IpcEndpoint>;

    /// Metrics broadcast endpoint
    pub fn metrics_endpoint() -> Result<IpcEndpoint>;
}

pub enum IpcEndpoint {
    UnixSocket(PathBuf),
    NamedPipe(String), // e.g., "swictation" -> \\.\pipe\swictation
}

impl IpcEndpoint {
    pub fn as_str(&self) -> String;
}
```

**Dependencies**:
- `dirs` crate (already in use) for standard directories
- `anyhow` for error handling

**Migration Strategy**:
1. Create new `swictation-paths` crate
2. Port existing path logic from `socket_utils.rs`, `config.rs`, `utils/mod.rs`
3. Update all consumers to use new API
4. Remove old path code

---

### 2. IPC Abstraction (`swictation-ipc` crate)

**Location**: `/opt/swictation/rust-crates/swictation-ipc/`

**Purpose**: Cross-platform IPC with Unix sockets on Linux/macOS, named pipes on Windows.

**API Design**:

```rust
pub trait IpcTransport: Send + Sync {
    async fn bind(endpoint: &IpcEndpoint) -> Result<Self> where Self: Sized;
    async fn accept(&mut self) -> Result<Box<dyn IpcStream>>;
}

pub trait IpcStream: AsyncRead + AsyncWrite + Send + Sync {
    fn peer_info(&self) -> Option<String>;
}

pub struct IpcServer {
    transport: Box<dyn IpcTransport>,
}

impl IpcServer {
    pub async fn new(endpoint: IpcEndpoint) -> Result<Self> {
        match endpoint {
            IpcEndpoint::UnixSocket(path) => {
                Ok(Self {
                    transport: Box::new(UnixSocketTransport::bind(&path).await?),
                })
            }
            IpcEndpoint::NamedPipe(name) => {
                Ok(Self {
                    transport: Box::new(NamedPipeTransport::bind(&name).await?),
                })
            }
        }
    }

    pub async fn accept(&mut self) -> Result<Box<dyn IpcStream>> {
        self.transport.accept().await
    }
}

pub struct IpcClient;

impl IpcClient {
    pub async fn connect(endpoint: &IpcEndpoint) -> Result<Box<dyn IpcStream>> {
        match endpoint {
            IpcEndpoint::UnixSocket(path) => {
                UnixSocketClient::connect(path).await
            }
            IpcEndpoint::NamedPipe(name) => {
                NamedPipeClient::connect(name).await
            }
        }
    }
}

// Platform-specific implementations
#[cfg(unix)]
mod unix_socket;

#[cfg(windows)]
mod named_pipe;
```

**Unix Socket Implementation** (Linux/macOS):
```rust
use tokio::net::{UnixListener, UnixStream};

struct UnixSocketTransport {
    listener: UnixListener,
}

impl IpcTransport for UnixSocketTransport {
    async fn bind(path: &Path) -> Result<Self> {
        // Remove existing socket
        let _ = std::fs::remove_file(path);

        let listener = UnixListener::bind(path)
            .context("Failed to bind Unix socket")?;

        // Set secure permissions (0600)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(path, perms)?;
        }

        Ok(Self { listener })
    }

    async fn accept(&mut self) -> Result<Box<dyn IpcStream>> {
        let (stream, _) = self.listener.accept().await?;
        Ok(Box::new(UnixSocketStream(stream)))
    }
}

struct UnixSocketStream(UnixStream);

impl IpcStream for UnixSocketStream {
    fn peer_info(&self) -> Option<String> {
        self.0.peer_addr().ok()
            .and_then(|addr| addr.as_pathname())
            .map(|p| p.display().to_string())
    }
}
```

**Named Pipe Implementation** (Windows):
```rust
use tokio::net::windows::named_pipe::{ServerOptions, ClientOptions, NamedPipeServer};

struct NamedPipeTransport {
    pipe_name: String,
    server: NamedPipeServer,
}

impl IpcTransport for NamedPipeTransport {
    async fn bind(name: &str) -> Result<Self> {
        let pipe_name = format!(r"\\.\pipe\{}", name);

        let server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(&pipe_name)
            .context("Failed to create named pipe")?;

        Ok(Self { pipe_name, server })
    }

    async fn accept(&mut self) -> Result<Box<dyn IpcStream>> {
        self.server.connect().await?;

        // Create next instance for next client
        let next_server = ServerOptions::new()
            .create(&self.pipe_name)?;

        let current = std::mem::replace(&mut self.server, next_server);
        Ok(Box::new(NamedPipeStream(current)))
    }
}

struct NamedPipeStream(NamedPipeServer);

impl IpcStream for NamedPipeStream {
    fn peer_info(&self) -> Option<String> {
        Some("named-pipe-client".to_string())
    }
}
```

**Migration Strategy**:
1. Create `swictation-ipc` crate with trait-based design
2. Implement Unix socket backend (port from existing `ipc.rs`)
3. Implement Windows named pipe backend
4. Add integration tests for both
5. Update `swictation-daemon` to use new abstraction

---

### 3. Library Loading Abstraction (`swictation-libs` crate)

**Location**: `/opt/swictation/rust-crates/swictation-libs/`

**Purpose**: Cross-platform dynamic library loading for ONNX Runtime and other native dependencies.

**API Design**:

```rust
pub struct LibraryLoader {
    search_paths: Vec<PathBuf>,
}

pub enum LibraryType {
    OnnxRuntime,
    Cuda,
    Rocm,
    CoreML,
}

impl LibraryLoader {
    pub fn new() -> Self;

    /// Add search path for library resolution
    pub fn add_search_path(&mut self, path: PathBuf);

    /// Resolve library path for the current platform
    /// Returns absolute path to the library file
    pub fn resolve(&self, lib_type: LibraryType) -> Result<PathBuf>;

    /// Get platform-specific library name
    /// - Linux: libonnxruntime.so, libonnxruntime.so.1.23.2
    /// - macOS: libonnxruntime.dylib, libonnxruntime.1.23.2.dylib
    /// - Windows: onnxruntime.dll, onnxruntime.1.23.2.dll
    pub fn library_name(lib_type: LibraryType) -> Vec<String>;

    /// Detect library via common installation methods
    /// 1. Environment variable (ORT_DYLIB_PATH)
    /// 2. System package manager locations
    /// 3. Python package installation
    /// 4. Local build directories
    pub fn auto_detect(lib_type: LibraryType) -> Result<PathBuf>;
}

// Platform-specific search locations
#[cfg(target_os = "linux")]
const ONNX_SEARCH_PATHS: &[&str] = &[
    "/usr/lib",
    "/usr/local/lib",
    "/usr/lib/x86_64-linux-gnu",
    "~/.local/lib",
];

#[cfg(target_os = "macos")]
const ONNX_SEARCH_PATHS: &[&str] = &[
    "/usr/local/lib",
    "/opt/homebrew/lib",
    "~/Library/Frameworks",
];

#[cfg(target_os = "windows")]
const ONNX_SEARCH_PATHS: &[&str] = &[
    r"C:\Program Files\onnxruntime\lib",
    r"C:\Program Files (x86)\onnxruntime\lib",
];

impl LibraryLoader {
    pub fn auto_detect(lib_type: LibraryType) -> Result<PathBuf> {
        match lib_type {
            LibraryType::OnnxRuntime => {
                // 1. Check environment variable
                if let Ok(path) = std::env::var("ORT_DYLIB_PATH") {
                    let path = PathBuf::from(path);
                    if path.exists() {
                        return Ok(path);
                    }
                }

                // 2. Check Python installation
                if let Ok(python_path) = detect_python_onnxruntime() {
                    return Ok(python_path);
                }

                // 3. Search standard locations
                let lib_names = Self::library_name(LibraryType::OnnxRuntime);
                for search_path in ONNX_SEARCH_PATHS {
                    let expanded = shellexpand::tilde(search_path);
                    let search_dir = PathBuf::from(expanded.as_ref());

                    for lib_name in &lib_names {
                        let lib_path = search_dir.join(lib_name);
                        if lib_path.exists() {
                            return Ok(lib_path);
                        }
                    }
                }

                Err(anyhow::anyhow!("ONNX Runtime library not found"))
            }
            _ => todo!("Other library types")
        }
    }
}

fn detect_python_onnxruntime() -> Result<PathBuf> {
    use std::process::Command;

    let script = r#"
import onnxruntime
import os
import sys
lib_dir = os.path.dirname(onnxruntime.__file__)
capi_dir = os.path.join(lib_dir, 'capi')
"#;

    #[cfg(unix)]
    let lib_name = "libonnxruntime.so";
    #[cfg(target_os = "macos")]
    let lib_name = "libonnxruntime.dylib";
    #[cfg(windows)]
    let lib_name = "onnxruntime.dll";

    let output = Command::new("python3")
        .arg("-c")
        .arg(format!("{}; print(os.path.join(capi_dir, '{}'))", script, lib_name))
        .output()?;

    if output.status.success() {
        let path_str = String::from_utf8(output.stdout)?.trim().to_string();
        let path = PathBuf::from(path_str);
        if path.exists() {
            return Ok(path);
        }
    }

    Err(anyhow::anyhow!("Python ONNX Runtime not found"))
}
```

**Integration with `ort` crate**:

```rust
// In swictation-stt/src/lib.rs or recognizer_ort.rs

use swictation_libs::{LibraryLoader, LibraryType};

pub fn initialize_onnx_runtime() -> Result<()> {
    let lib_path = LibraryLoader::auto_detect(LibraryType::OnnxRuntime)
        .context("Failed to locate ONNX Runtime library")?;

    // Set ORT_DYLIB_PATH for the ort crate
    std::env::set_var("ORT_DYLIB_PATH", lib_path.to_str().unwrap());

    info!("Using ONNX Runtime at: {}", lib_path.display());

    Ok(())
}
```

**Migration Strategy**:
1. Create `swictation-libs` crate
2. Implement library detection logic for each platform
3. Add Python ONNX Runtime detection
4. Update `swictation-stt` to call initialization
5. Test on all platforms with various installation methods

---

### 4. Platform Detection Module (Enhancement)

**Location**: Enhance existing `rust-crates/swictation-daemon/src/display_server.rs`

**Current State**: Excellent evidence-based detection for Linux/macOS
**Proposed**: Extend to Windows

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    LinuxX11,
    LinuxWayland { is_gnome: bool },
    MacOS,
    Windows,
    Unknown,
}

impl Platform {
    pub fn detect() -> Self {
        #[cfg(target_os = "windows")]
        return Self::Windows;

        #[cfg(target_os = "macos")]
        return Self::MacOS;

        #[cfg(target_os = "linux")]
        {
            let info = detect_display_server();
            match info.server_type {
                DisplayServer::X11 => Self::LinuxX11,
                DisplayServer::Wayland => Self::LinuxWayland {
                    is_gnome: info.is_gnome_wayland,
                },
                _ => Self::Unknown,
            }
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        Self::Unknown
    }

    pub fn supports_unix_sockets(&self) -> bool {
        matches!(self, Self::LinuxX11 | Self::LinuxWayland { .. } | Self::MacOS)
    }

    pub fn supports_global_hotkeys(&self) -> bool {
        matches!(self, Self::LinuxX11 | Self::MacOS | Self::Windows)
    }
}
```

---

### 5. Windows-Specific Implementations

#### Text Injection (Windows)

**Location**: `rust-crates/swictation-daemon/src/windows_text_inject.rs`

```rust
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_UNICODE,
};

pub struct WindowsTextInjector;

impl WindowsTextInjector {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub fn inject_text(&self, text: &str) -> Result<()> {
        for ch in text.chars() {
            self.send_unicode_char(ch)?;
        }
        Ok(())
    }

    fn send_unicode_char(&self, ch: char) -> Result<()> {
        let mut inputs = Vec::new();

        // Key down
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: 0,
                    wScan: ch as u16,
                    dwFlags: KEYEVENTF_UNICODE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });

        // Key up
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: 0,
                    wScan: ch as u16,
                    dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });

        unsafe {
            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        }

        Ok(())
    }
}
```

#### Hotkey Handling (Windows)

Already handled by `global-hotkey` crate, but needs testing and Windows-specific configuration.

---

## Directory Structure

```
/opt/swictation/rust-crates/
├── swictation-paths/           # NEW: Cross-platform path abstraction
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── linux.rs
│   │   ├── macos.rs
│   │   ├── windows.rs
│   │   └── tests.rs
│   └── README.md
│
├── swictation-ipc/             # NEW: Cross-platform IPC
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── unix_socket.rs      # Linux/macOS
│   │   ├── named_pipe.rs       # Windows
│   │   ├── transport.rs        # Trait definitions
│   │   └── tests.rs
│   └── README.md
│
├── swictation-libs/            # NEW: Library loading abstraction
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── onnx.rs
│   │   ├── cuda.rs
│   │   ├── detection.rs
│   │   └── tests.rs
│   └── README.md
│
├── swictation-daemon/
│   ├── src/
│   │   ├── text_injection.rs   # MODIFY: Add Windows support
│   │   ├── windows_text_inject.rs  # NEW
│   │   ├── display_server.rs   # ENHANCE: Windows detection
│   │   ├── hotkey.rs           # ENHANCE: Windows testing
│   │   ├── socket_utils.rs     # REMOVE: Replaced by swictation-paths
│   │   └── config.rs           # MODIFY: Use swictation-paths
│
└── (existing crates...)
```

---

## Implementation Plan

### Phase 1: Foundation (Week 1-2)
1. Create `swictation-paths` crate
2. Migrate all path logic from existing files
3. Update all consumers to use new API
4. Add comprehensive tests for Linux/macOS/Windows paths

### Phase 2: IPC Abstraction (Week 3-4)
1. Create `swictation-ipc` crate
2. Implement Unix socket backend (port from existing)
3. Implement Windows named pipe backend
4. Integration tests on all platforms
5. Update daemon to use new IPC abstraction

### Phase 3: Library Loading (Week 5)
1. Create `swictation-libs` crate
2. Implement ONNX Runtime detection for all platforms
3. Add Python package detection
4. Update STT module initialization
5. Test with various installation methods

### Phase 4: Windows Support (Week 6-7)
1. Implement Windows text injection
2. Test hotkey handling on Windows
3. Platform-specific daemon adjustments
4. Windows installer/packaging

### Phase 5: Testing & Documentation (Week 8)
1. Comprehensive cross-platform testing
2. Update all documentation
3. CI/CD for all platforms
4. Migration guide for existing users

---

## Testing Strategy

### Unit Tests
- Each platform module has isolated tests
- Mock environment variables for path tests
- Mock filesystem for IPC tests

### Integration Tests
- Real Unix socket communication
- Real named pipe communication
- Cross-platform library detection
- End-to-end daemon startup on each platform

### Platform Matrix
```
          Linux (X11)  |  Linux (Wayland)  |  macOS  |  Windows
Paths         ✓              ✓                 ✓         ✓
IPC           ✓              ✓                 ✓         ✓
Libraries     ✓              ✓                 ✓         ✓
Text Inject   ✓              ✓                 ✓         ✓
Hotkeys       ✓              ✓                 ✓         ✓
```

---

## Migration Guide

### For Existing Code

**Before**:
```rust
use crate::socket_utils::get_ipc_socket_path;

let socket_path = get_ipc_socket_path()?;
let listener = UnixListener::bind(&socket_path)?;
```

**After**:
```rust
use swictation_paths::PlatformPaths;
use swictation_ipc::IpcServer;

let endpoint = PlatformPaths::ipc_endpoint()?;
let server = IpcServer::new(endpoint).await?;
```

### For Users

No user-facing changes required. All path locations remain the same on Linux/macOS. Windows users get new standard locations.

---

## Security Considerations

1. **Unix Socket Permissions**: Maintain 0600 (owner-only) permissions
2. **Named Pipe Security**: Use appropriate Windows ACLs
3. **Path Traversal**: Validate all user-provided paths
4. **Library Loading**: Verify library signatures when available
5. **Environment Variables**: Sanitize and validate all env var inputs

---

## Performance Impact

- **Minimal**: Path resolution cached at startup
- **IPC**: No performance difference (trait-based dispatch inlined)
- **Library Loading**: One-time initialization cost
- **Cross-platform overhead**: ~0.1% (compile-time dispatch via cfg!)

---

## Dependencies

### New Crates Required

**swictation-paths**:
- `dirs = "5.0"` (already in use)
- `anyhow` (already in use)

**swictation-ipc**:
- `tokio = { version = "1.0", features = ["net"] }` (already in use)
- `async-trait = "0.1"`

**swictation-libs**:
- `libloading = "0.8"` (optional, for dynamic loading)
- `shellexpand = "3.0"` (for path expansion)

**Windows-specific**:
- `windows = { version = "0.58", features = ["Win32_UI_Input_KeyboardAndMouse", "Win32_System_Pipes"] }`

---

## Future Enhancements

1. **Plugin System**: Abstract platform-specific plugins
2. **Hot-reload**: Dynamic library reloading without restart
3. **Multi-monitor**: Platform-specific monitor detection
4. **Accessibility**: Enhanced platform accessibility APIs
5. **Wayland Protocols**: Native Wayland text injection (future protocol support)

---

## Success Criteria

- [ ] All existing functionality works on Linux (X11/Wayland)
- [ ] All existing functionality works on macOS
- [ ] Full Windows support (text injection, hotkeys, IPC)
- [ ] Zero breaking changes for existing users
- [ ] Comprehensive test coverage (>80%)
- [ ] CI/CD builds for all platforms
- [ ] Documentation complete and accurate

---

## References

- XDG Base Directory Specification: https://specifications.freedesktop.org/basedir-spec/
- macOS File System Programming Guide: https://developer.apple.com/library/archive/documentation/FileManagement/
- Windows Known Folders: https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid
- ONNX Runtime API: https://onnxruntime.ai/docs/api/
