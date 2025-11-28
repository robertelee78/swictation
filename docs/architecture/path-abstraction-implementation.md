# Path Abstraction Implementation Guide

## Complete Implementation: `swictation-paths` Crate

This document provides the complete, production-ready implementation of the cross-platform path abstraction module.

---

## File Structure

```
rust-crates/swictation-paths/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs          # Public API
    ├── platform.rs     # Platform detection
    ├── linux.rs        # Linux-specific paths
    ├── macos.rs        # macOS-specific paths
    ├── windows.rs      # Windows-specific paths
    ├── ipc.rs          # IPC endpoint abstraction
    └── tests.rs        # Integration tests
```

---

## `Cargo.toml`

```toml
[package]
name = "swictation-paths"
version = "0.1.0"
edition = "2021"
authors = ["Swictation Contributors"]
description = "Cross-platform path abstraction for Swictation"
license = "MIT OR Apache-2.0"

[dependencies]
anyhow = "1.0"
thiserror = "2.0"
dirs = "5.0"

[dev-dependencies]
tempfile = "3.0"
```

---

## `src/lib.rs`

```rust
//! Cross-platform path abstraction for Swictation
//!
//! Provides a single source of truth for all platform-specific paths:
//! - Data directory (metrics.db, models, corrections)
//! - Config directory (config.toml)
//! - Runtime directory (Unix sockets, named pipes)
//! - Log directory
//! - Cache directory
//!
//! # Platform Support
//!
//! - **Linux**: XDG Base Directory Specification
//! - **macOS**: Apple File System Programming Guide
//! - **Windows**: Known Folders (LOCALAPPDATA, APPDATA)
//!
//! # Example
//!
//! ```rust
//! use swictation_paths::PlatformPaths;
//!
//! let db_path = PlatformPaths::database_path()?;
//! println!("Database: {}", db_path.display());
//! ```

pub mod platform;
pub mod ipc;

mod linux;
mod macos;
mod windows;

use anyhow::{Context, Result};
use std::path::PathBuf;

pub use ipc::IpcEndpoint;
pub use platform::Platform;

/// Cross-platform path resolution for Swictation
pub struct PlatformPaths;

impl PlatformPaths {
    /// Get the current platform
    pub fn platform() -> Platform {
        Platform::detect()
    }

    /// Data directory (persistent application data)
    ///
    /// # Platform Paths
    ///
    /// - Linux: `~/.local/share/swictation/`
    /// - macOS: `~/Library/Application Support/swictation/`
    /// - Windows: `%LOCALAPPDATA%\Swictation\`
    ///
    /// # Usage
    ///
    /// Store: metrics.db, models/, learned corrections, user data
    pub fn data_dir() -> Result<PathBuf> {
        let dir = match Platform::detect() {
            Platform::Linux => linux::data_dir(),
            Platform::MacOS => macos::data_dir(),
            Platform::Windows => windows::data_dir(),
            Platform::Unknown => anyhow::bail!("Unsupported platform"),
        }?;

        ensure_directory_exists(&dir)?;
        Ok(dir)
    }

    /// Config directory (configuration files)
    ///
    /// # Platform Paths
    ///
    /// - Linux: `~/.config/swictation/`
    /// - macOS: `~/Library/Application Support/com.swictation.daemon/`
    /// - Windows: `%APPDATA%\Swictation\`
    ///
    /// # Usage
    ///
    /// Store: config.toml, corrections.toml, user preferences
    pub fn config_dir() -> Result<PathBuf> {
        let dir = match Platform::detect() {
            Platform::Linux => linux::config_dir(),
            Platform::MacOS => macos::config_dir(),
            Platform::Windows => windows::config_dir(),
            Platform::Unknown => anyhow::bail!("Unsupported platform"),
        }?;

        ensure_directory_exists(&dir)?;
        Ok(dir)
    }

    /// Runtime directory (IPC sockets, temporary runtime files)
    ///
    /// # Platform Paths
    ///
    /// - Linux: `$XDG_RUNTIME_DIR` or `~/.local/share/swictation/`
    /// - macOS: `~/Library/Application Support/swictation/`
    /// - Windows: `%TEMP%\swictation\` (ephemeral)
    ///
    /// # Usage
    ///
    /// Store: Unix sockets, named pipe identifiers
    ///
    /// # Security
    ///
    /// On Unix, XDG_RUNTIME_DIR is preferred (auto-cleaned on logout, secure permissions).
    /// Directory permissions set to 0700 (owner-only).
    pub fn runtime_dir() -> Result<PathBuf> {
        let dir = match Platform::detect() {
            Platform::Linux => linux::runtime_dir(),
            Platform::MacOS => macos::runtime_dir(),
            Platform::Windows => windows::runtime_dir(),
            Platform::Unknown => anyhow::bail!("Unsupported platform"),
        }?;

        ensure_directory_exists(&dir)?;
        secure_directory_permissions(&dir)?;
        Ok(dir)
    }

    /// Log directory (application logs)
    ///
    /// # Platform Paths
    ///
    /// - Linux: `~/.local/share/swictation/logs/`
    /// - macOS: `~/Library/Logs/swictation/`
    /// - Windows: `%LOCALAPPDATA%\Swictation\logs\`
    ///
    /// # Usage
    ///
    /// Store: daemon.log, ui.log, crash reports
    pub fn log_dir() -> Result<PathBuf> {
        let dir = match Platform::detect() {
            Platform::Linux => linux::log_dir(),
            Platform::MacOS => macos::log_dir(),
            Platform::Windows => windows::log_dir(),
            Platform::Unknown => anyhow::bail!("Unsupported platform"),
        }?;

        ensure_directory_exists(&dir)?;
        Ok(dir)
    }

    /// Cache directory (temporary cached data)
    ///
    /// # Platform Paths
    ///
    /// - Linux: `~/.cache/swictation/`
    /// - macOS: `~/Library/Caches/swictation/`
    /// - Windows: `%LOCALAPPDATA%\Swictation\cache\`
    ///
    /// # Usage
    ///
    /// Store: downloaded models (before installation), temporary audio files
    pub fn cache_dir() -> Result<PathBuf> {
        let dir = match Platform::detect() {
            Platform::Linux => linux::cache_dir(),
            Platform::MacOS => macos::cache_dir(),
            Platform::Windows => windows::cache_dir(),
            Platform::Unknown => anyhow::bail!("Unsupported platform"),
        }?;

        ensure_directory_exists(&dir)?;
        Ok(dir)
    }

    /// Model directory (ML models)
    ///
    /// Respects `SWICTATION_MODEL_PATH` environment variable for override.
    ///
    /// # Default Paths
    ///
    /// - Linux: `~/.local/share/swictation/models/`
    /// - macOS: `~/Library/Application Support/swictation/models/`
    /// - Windows: `%LOCALAPPDATA%\Swictation\models\`
    ///
    /// # Usage
    ///
    /// Store: parakeet-tdt-0.6b-v3-onnx/, parakeet-tdt-1.1b-onnx/, silero-vad/
    pub fn model_dir() -> Result<PathBuf> {
        // Check environment variable override
        if let Ok(custom_path) = std::env::var("SWICTATION_MODEL_PATH") {
            let path = PathBuf::from(custom_path);
            if !path.exists() {
                anyhow::bail!(
                    "SWICTATION_MODEL_PATH points to non-existent directory: {}",
                    path.display()
                );
            }
            return Ok(path);
        }

        // Use platform-specific default
        let dir = Self::data_dir()?.join("models");
        ensure_directory_exists(&dir)?;
        Ok(dir)
    }

    /// Database path (metrics database)
    ///
    /// # Path
    ///
    /// `<data_dir>/metrics.db`
    ///
    /// # Usage
    ///
    /// SQLite database for metrics, corrections, learned patterns
    pub fn database_path() -> Result<PathBuf> {
        Ok(Self::data_dir()?.join("metrics.db"))
    }

    /// Config file path (main configuration)
    ///
    /// # Path
    ///
    /// `<config_dir>/config.toml`
    pub fn config_file_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    /// Corrections file path (learned corrections)
    ///
    /// # Path
    ///
    /// `<config_dir>/corrections.toml`
    pub fn corrections_file_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("corrections.toml"))
    }

    /// IPC endpoint for main daemon communication
    ///
    /// # Platform Endpoints
    ///
    /// - Linux/macOS: Unix socket at `<runtime_dir>/swictation.sock`
    /// - Windows: Named pipe `\\.\pipe\swictation`
    ///
    /// # Usage
    ///
    /// Command IPC (toggle, status, quit)
    pub fn ipc_endpoint() -> Result<IpcEndpoint> {
        match Platform::detect() {
            Platform::Linux | Platform::MacOS => {
                let socket_path = Self::runtime_dir()?.join("swictation.sock");
                Ok(IpcEndpoint::UnixSocket(socket_path))
            }
            Platform::Windows => Ok(IpcEndpoint::NamedPipe("swictation".to_string())),
            Platform::Unknown => anyhow::bail!("Unsupported platform"),
        }
    }

    /// IPC endpoint for metrics broadcast
    ///
    /// # Platform Endpoints
    ///
    /// - Linux/macOS: Unix socket at `<runtime_dir>/swictation_metrics.sock`
    /// - Windows: Named pipe `\\.\pipe\swictation_metrics`
    ///
    /// # Usage
    ///
    /// Real-time metrics streaming to UI clients
    pub fn metrics_endpoint() -> Result<IpcEndpoint> {
        match Platform::detect() {
            Platform::Linux | Platform::MacOS => {
                let socket_path = Self::runtime_dir()?.join("swictation_metrics.sock");
                Ok(IpcEndpoint::UnixSocket(socket_path))
            }
            Platform::Windows => {
                Ok(IpcEndpoint::NamedPipe("swictation_metrics".to_string()))
            }
            Platform::Unknown => anyhow::bail!("Unsupported platform"),
        }
    }
}

/// Ensure directory exists with proper error context
fn ensure_directory_exists(path: &PathBuf) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path).context(format!(
            "Failed to create directory: {}",
            path.display()
        ))?;
    }
    Ok(())
}

/// Set secure permissions on directory (owner-only: 0700)
fn secure_directory_permissions(path: &PathBuf) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o700);
        std::fs::set_permissions(path, perms).context(format!(
            "Failed to set permissions on: {}",
            path.display()
        ))?;
    }

    #[cfg(not(unix))]
    {
        // Windows: Use ACLs (requires additional implementation)
        // For now, directory inherits parent permissions
        let _ = path; // Suppress unused variable warning
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = PlatformPaths::platform();
        println!("Detected platform: {:?}", platform);
        assert!(!matches!(platform, Platform::Unknown));
    }

    #[test]
    fn test_data_dir() {
        let data_dir = PlatformPaths::data_dir().unwrap();
        println!("Data directory: {}", data_dir.display());
        assert!(data_dir.is_absolute());
        assert!(data_dir.exists());
    }

    #[test]
    fn test_config_dir() {
        let config_dir = PlatformPaths::config_dir().unwrap();
        println!("Config directory: {}", config_dir.display());
        assert!(config_dir.is_absolute());
        assert!(config_dir.exists());
    }

    #[test]
    fn test_runtime_dir() {
        let runtime_dir = PlatformPaths::runtime_dir().unwrap();
        println!("Runtime directory: {}", runtime_dir.display());
        assert!(runtime_dir.is_absolute());
        assert!(runtime_dir.exists());
    }

    #[test]
    fn test_model_dir() {
        let model_dir = PlatformPaths::model_dir().unwrap();
        println!("Model directory: {}", model_dir.display());
        assert!(model_dir.is_absolute());
        assert!(model_dir.ends_with("models"));
    }

    #[test]
    fn test_database_path() {
        let db_path = PlatformPaths::database_path().unwrap();
        println!("Database path: {}", db_path.display());
        assert!(db_path.is_absolute());
        assert!(db_path.ends_with("metrics.db"));
    }

    #[test]
    fn test_ipc_endpoint() {
        let endpoint = PlatformPaths::ipc_endpoint().unwrap();
        println!("IPC endpoint: {}", endpoint.as_str());

        #[cfg(unix)]
        assert!(matches!(endpoint, IpcEndpoint::UnixSocket(_)));

        #[cfg(windows)]
        assert!(matches!(endpoint, IpcEndpoint::NamedPipe(_)));
    }

    #[test]
    fn test_all_paths_exist() {
        // This test ensures all directories can be created
        let _ = PlatformPaths::data_dir().unwrap();
        let _ = PlatformPaths::config_dir().unwrap();
        let _ = PlatformPaths::runtime_dir().unwrap();
        let _ = PlatformPaths::log_dir().unwrap();
        let _ = PlatformPaths::cache_dir().unwrap();
        let _ = PlatformPaths::model_dir().unwrap();
    }
}
```

---

## `src/platform.rs`

```rust
//! Platform detection

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Linux,
    MacOS,
    Windows,
    Unknown,
}

impl Platform {
    pub fn detect() -> Self {
        #[cfg(target_os = "linux")]
        return Self::Linux;

        #[cfg(target_os = "macos")]
        return Self::MacOS;

        #[cfg(target_os = "windows")]
        return Self::Windows;

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        Self::Unknown
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Linux => "Linux",
            Self::MacOS => "macOS",
            Self::Windows => "Windows",
            Self::Unknown => "Unknown",
        }
    }
}
```

---

## `src/linux.rs`

```rust
//! Linux-specific path resolution (XDG Base Directory Specification)

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Data directory: ~/.local/share/swictation/
pub fn data_dir() -> Result<PathBuf> {
    Ok(dirs::data_local_dir()
        .context("Failed to get XDG data directory")?
        .join("swictation"))
}

/// Config directory: ~/.config/swictation/
pub fn config_dir() -> Result<PathBuf> {
    Ok(dirs::config_dir()
        .context("Failed to get XDG config directory")?
        .join("swictation"))
}

/// Runtime directory: $XDG_RUNTIME_DIR or ~/.local/share/swictation/
///
/// XDG_RUNTIME_DIR is preferred (secure, auto-cleaned on logout)
pub fn runtime_dir() -> Result<PathBuf> {
    // Try XDG_RUNTIME_DIR first (best practice for sockets)
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        let path = PathBuf::from(runtime_dir);
        if path.exists() {
            return Ok(path);
        }
    }

    // Fallback to data directory
    data_dir()
}

/// Log directory: ~/.local/share/swictation/logs/
pub fn log_dir() -> Result<PathBuf> {
    Ok(data_dir()?.join("logs"))
}

/// Cache directory: ~/.cache/swictation/
pub fn cache_dir() -> Result<PathBuf> {
    Ok(dirs::cache_dir()
        .context("Failed to get XDG cache directory")?
        .join("swictation"))
}
```

---

## `src/macos.rs`

```rust
//! macOS-specific path resolution (Apple File System Programming Guide)

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Data directory: ~/Library/Application Support/swictation/
pub fn data_dir() -> Result<PathBuf> {
    Ok(dirs::data_local_dir()
        .context("Failed to get Application Support directory")?
        .join("swictation"))
}

/// Config directory: ~/Library/Application Support/com.swictation.daemon/
///
/// Uses reverse-DNS naming convention for macOS apps
pub fn config_dir() -> Result<PathBuf> {
    Ok(dirs::config_dir()
        .context("Failed to get Application Support directory")?
        .join("com.swictation.daemon"))
}

/// Runtime directory: ~/Library/Application Support/swictation/
///
/// macOS doesn't have XDG_RUNTIME_DIR, use Application Support
pub fn runtime_dir() -> Result<PathBuf> {
    data_dir()
}

/// Log directory: ~/Library/Logs/swictation/
pub fn log_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Failed to get home directory")?;
    Ok(home.join("Library").join("Logs").join("swictation"))
}

/// Cache directory: ~/Library/Caches/swictation/
pub fn cache_dir() -> Result<PathBuf> {
    Ok(dirs::cache_dir()
        .context("Failed to get Caches directory")?
        .join("swictation"))
}
```

---

## `src/windows.rs`

```rust
//! Windows-specific path resolution (Known Folders)

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Data directory: %LOCALAPPDATA%\Swictation\
pub fn data_dir() -> Result<PathBuf> {
    Ok(dirs::data_local_dir()
        .context("Failed to get LOCALAPPDATA directory")?
        .join("Swictation"))
}

/// Config directory: %APPDATA%\Swictation\
///
/// APPDATA is roaming (synced across machines in domain environments)
pub fn config_dir() -> Result<PathBuf> {
    Ok(dirs::config_dir()
        .context("Failed to get APPDATA directory")?
        .join("Swictation"))
}

/// Runtime directory: %TEMP%\swictation\
///
/// Windows uses named pipes (not filesystem paths) for IPC
/// This directory is for temporary runtime files only
pub fn runtime_dir() -> Result<PathBuf> {
    let temp = std::env::temp_dir();
    Ok(temp.join("swictation"))
}

/// Log directory: %LOCALAPPDATA%\Swictation\logs\
pub fn log_dir() -> Result<PathBuf> {
    Ok(data_dir()?.join("logs"))
}

/// Cache directory: %LOCALAPPDATA%\Swictation\cache\
pub fn cache_dir() -> Result<PathBuf> {
    Ok(data_dir()?.join("cache"))
}
```

---

## `src/ipc.rs`

```rust
//! IPC endpoint abstraction

use std::fmt;
use std::path::PathBuf;

/// Cross-platform IPC endpoint identifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcEndpoint {
    /// Unix domain socket (Linux/macOS)
    UnixSocket(PathBuf),
    /// Named pipe (Windows)
    NamedPipe(String),
}

impl IpcEndpoint {
    /// Get string representation of endpoint
    ///
    /// - Unix socket: absolute path as string
    /// - Named pipe: pipe name (without \\.\pipe\ prefix)
    pub fn as_str(&self) -> String {
        match self {
            Self::UnixSocket(path) => path.to_string_lossy().to_string(),
            Self::NamedPipe(name) => name.clone(),
        }
    }

    /// Get full Windows named pipe path
    ///
    /// Returns `\\.\pipe\<name>` for Windows named pipes
    pub fn windows_pipe_path(&self) -> Option<String> {
        match self {
            Self::NamedPipe(name) => Some(format!(r"\\.\pipe\{}", name)),
            _ => None,
        }
    }

    /// Check if endpoint is a Unix socket
    pub fn is_unix_socket(&self) -> bool {
        matches!(self, Self::UnixSocket(_))
    }

    /// Check if endpoint is a named pipe
    pub fn is_named_pipe(&self) -> bool {
        matches!(self, Self::NamedPipe(_))
    }
}

impl fmt::Display for IpcEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnixSocket(path) => write!(f, "unix:{}", path.display()),
            Self::NamedPipe(name) => write!(f, "pipe:{}", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unix_socket() {
        let endpoint = IpcEndpoint::UnixSocket(PathBuf::from("/tmp/test.sock"));
        assert!(endpoint.is_unix_socket());
        assert!(!endpoint.is_named_pipe());
        assert_eq!(endpoint.as_str(), "/tmp/test.sock");
        assert!(endpoint.to_string().starts_with("unix:"));
    }

    #[test]
    fn test_named_pipe() {
        let endpoint = IpcEndpoint::NamedPipe("test_pipe".to_string());
        assert!(endpoint.is_named_pipe());
        assert!(!endpoint.is_unix_socket());
        assert_eq!(endpoint.as_str(), "test_pipe");
        assert_eq!(endpoint.windows_pipe_path(), Some(r"\\.\pipe\test_pipe".to_string()));
        assert!(endpoint.to_string().starts_with("pipe:"));
    }
}
```

---

## Usage Examples

### Basic Path Resolution

```rust
use swictation_paths::PlatformPaths;

fn main() -> anyhow::Result<()> {
    // Get database path
    let db_path = PlatformPaths::database_path()?;
    println!("Database: {}", db_path.display());

    // Get model directory
    let model_dir = PlatformPaths::model_dir()?;
    println!("Models: {}", model_dir.display());

    // Get IPC endpoint
    let ipc = PlatformPaths::ipc_endpoint()?;
    println!("IPC endpoint: {}", ipc);

    Ok(())
}
```

### Migrating from Old Code

**Before** (`swictation-daemon/src/config.rs`):
```rust
fn get_default_model_dir() -> PathBuf {
    env::var("SWICTATION_MODEL_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = env::var("HOME").expect("HOME not set");
            PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("swictation")
                .join("models")
        })
}
```

**After**:
```rust
use swictation_paths::PlatformPaths;

fn get_default_model_dir() -> PathBuf {
    PlatformPaths::model_dir()
        .expect("Failed to determine model directory")
}
```

---

## Platform Behavior Matrix

| Function | Linux | macOS | Windows |
|----------|-------|-------|---------|
| `data_dir()` | `~/.local/share/swictation/` | `~/Library/Application Support/swictation/` | `%LOCALAPPDATA%\Swictation\` |
| `config_dir()` | `~/.config/swictation/` | `~/Library/Application Support/com.swictation.daemon/` | `%APPDATA%\Swictation\` |
| `runtime_dir()` | `$XDG_RUNTIME_DIR` or data_dir | data_dir | `%TEMP%\swictation\` |
| `log_dir()` | data_dir/logs | `~/Library/Logs/swictation/` | data_dir/logs |
| `cache_dir()` | `~/.cache/swictation/` | `~/Library/Caches/swictation/` | data_dir/cache |
| `ipc_endpoint()` | Unix socket | Unix socket | Named pipe |

---

## Testing

```bash
# Run tests on current platform
cargo test -p swictation-paths

# Run with output
cargo test -p swictation-paths -- --nocapture

# Test specific function
cargo test -p swictation-paths test_data_dir
```

Expected output:
```
Detected platform: Linux
Data directory: /home/user/.local/share/swictation
Config directory: /home/user/.config/swictation
Runtime directory: /run/user/1000
Model directory: /home/user/.local/share/swictation/models
Database path: /home/user/.local/share/swictation/metrics.db
IPC endpoint: unix:/run/user/1000/swictation.sock
```

---

## Integration Checklist

- [ ] Create `swictation-paths` crate
- [ ] Implement all platform modules (linux.rs, macos.rs, windows.rs)
- [ ] Add comprehensive tests
- [ ] Update `swictation-daemon/Cargo.toml` to depend on `swictation-paths`
- [ ] Migrate `config.rs` to use `PlatformPaths`
- [ ] Migrate `socket_utils.rs` to use `PlatformPaths`
- [ ] Remove old path resolution code
- [ ] Update documentation
- [ ] Test on all platforms
