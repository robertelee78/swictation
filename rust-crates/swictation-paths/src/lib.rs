//! Cross-platform path utilities for Swictation.
//!
//! Provides unified path handling across Linux, macOS, and (future) Windows.
//! This is the single source of truth for all Swictation path logic.
//!
//! # Platform Behavior
//!
//! | Platform | Data Directory | Socket Directory |
//! |----------|----------------|------------------|
//! | Linux    | `~/.local/share/swictation` | `$XDG_RUNTIME_DIR` or data dir |
//! | macOS    | `~/Library/Application Support/swictation` | Same as data dir |
//! | Windows  | `%APPDATA%/swictation` | Named pipes (future) |

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use thiserror::Error;

/// Errors specific to path operations.
#[derive(Error, Debug)]
pub enum PathError {
    #[error("Could not determine home directory")]
    NoHomeDirectory,

    #[error("Could not determine data directory")]
    NoDataDirectory,

    #[error("Could not create directory: {0}")]
    DirectoryCreation(PathBuf),

    #[error("Invalid socket path: {0}")]
    InvalidSocketPath(String),
}

/// Application identifier used in path construction.
const APP_NAME: &str = "swictation";

/// Socket file name for IPC communication.
const IPC_SOCKET_NAME: &str = "swictation.sock";

/// Socket file name for metrics communication.
const METRICS_SOCKET_NAME: &str = "swictation_metrics.sock";

/// Get the application data directory.
///
/// Creates the directory if it doesn't exist with secure permissions (0o700).
///
/// # Platform Behavior
/// - **Linux**: `~/.local/share/swictation`
/// - **macOS**: `~/Library/Application Support/swictation`
/// - **Windows**: `%APPDATA%/swictation`
///
/// # Errors
/// Returns an error if the directory cannot be determined or created.
pub fn get_data_dir() -> Result<PathBuf> {
    let base_dir = dirs::data_dir().ok_or(PathError::NoDataDirectory)?;
    let data_dir = base_dir.join(APP_NAME);

    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)
            .with_context(|| format!("Failed to create data directory: {}", data_dir.display()))?;

        // Set secure permissions on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o700);
            fs::set_permissions(&data_dir, perms)
                .with_context(|| format!("Failed to set permissions on {}", data_dir.display()))?;
        }
    }

    Ok(data_dir)
}

/// Get the socket directory for IPC sockets.
///
/// Creates the directory if it doesn't exist with secure permissions.
///
/// # Platform Behavior
/// - **Linux**: Prefers `$XDG_RUNTIME_DIR` (e.g., `/run/user/1000`), falls back to data dir
/// - **macOS**: Uses application support directory
/// - **Windows**: Returns data dir (named pipes don't need a directory)
///
/// # Errors
/// Returns an error if the directory cannot be determined or created.
pub fn get_socket_dir() -> Result<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        // On Linux, prefer XDG_RUNTIME_DIR for sockets (best practice)
        if let Some(runtime_dir) = dirs::runtime_dir() {
            if runtime_dir.exists() {
                return Ok(runtime_dir);
            }
        }
        // Fall back to data directory
        get_data_dir()
    }

    #[cfg(target_os = "macos")]
    {
        // macOS: Use Application Support directory for sockets
        get_data_dir()
    }

    #[cfg(target_os = "windows")]
    {
        // Windows: Named pipes don't use filesystem paths the same way
        // Return data dir for now, actual IPC will use \\.\pipe\swictation
        get_data_dir()
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        get_data_dir()
    }
}

/// Get the path to the main IPC socket.
///
/// # Platform Behavior
/// - **Linux/macOS**: Unix domain socket path
/// - **Windows**: Will return a path, but actual IPC uses named pipes (future)
///
/// # Errors
/// Returns an error if the socket directory cannot be determined.
pub fn get_ipc_socket_path() -> Result<PathBuf> {
    let socket_dir = get_socket_dir()?;
    Ok(socket_dir.join(IPC_SOCKET_NAME))
}

/// Get the path to the metrics socket.
///
/// # Errors
/// Returns an error if the socket directory cannot be determined.
pub fn get_metrics_socket_path() -> Result<PathBuf> {
    let socket_dir = get_socket_dir()?;
    Ok(socket_dir.join(METRICS_SOCKET_NAME))
}

/// Get the models directory.
///
/// # Platform Behavior
/// - All platforms: `<data_dir>/models`
pub fn get_models_dir() -> Result<PathBuf> {
    let data_dir = get_data_dir()?;
    let models_dir = data_dir.join("models");

    if !models_dir.exists() {
        fs::create_dir_all(&models_dir).with_context(|| {
            format!(
                "Failed to create models directory: {}",
                models_dir.display()
            )
        })?;
    }

    Ok(models_dir)
}

/// Get the logs directory.
///
/// # Platform Behavior
/// - **Linux**: `~/.local/share/swictation/logs` or `$XDG_STATE_HOME/swictation`
/// - **macOS**: `~/Library/Logs/swictation`
/// - **Windows**: `%APPDATA%/swictation/logs`
pub fn get_logs_dir() -> Result<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        // macOS convention: ~/Library/Logs/<app>
        let home = dirs::home_dir().ok_or(PathError::NoHomeDirectory)?;
        let logs_dir = home.join("Library").join("Logs").join(APP_NAME);

        if !logs_dir.exists() {
            fs::create_dir_all(&logs_dir).with_context(|| {
                format!("Failed to create logs directory: {}", logs_dir.display())
            })?;
        }

        Ok(logs_dir)
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Linux/Windows: Use data directory
        let data_dir = get_data_dir()?;
        let logs_dir = data_dir.join("logs");

        if !logs_dir.exists() {
            fs::create_dir_all(&logs_dir).with_context(|| {
                format!("Failed to create logs directory: {}", logs_dir.display())
            })?;
        }

        Ok(logs_dir)
    }
}

/// Get the configuration directory.
///
/// # Platform Behavior
/// - **Linux**: `~/.config/swictation`
/// - **macOS**: `~/Library/Application Support/swictation`
/// - **Windows**: `%APPDATA%/swictation`
pub fn get_config_dir() -> Result<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        let config_base = dirs::config_dir().ok_or(PathError::NoDataDirectory)?;
        let config_dir = config_base.join(APP_NAME);

        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).with_context(|| {
                format!(
                    "Failed to create config directory: {}",
                    config_dir.display()
                )
            })?;

            // Secure permissions
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o700);
            fs::set_permissions(&config_dir, perms).ok();
        }

        Ok(config_dir)
    }

    #[cfg(not(target_os = "linux"))]
    {
        // macOS and Windows: config lives with data
        get_data_dir()
    }
}

/// Get the database directory for metrics and learning data.
///
/// # Platform Behavior
/// - All platforms: `<data_dir>/db`
pub fn get_db_dir() -> Result<PathBuf> {
    let data_dir = get_data_dir()?;
    let db_dir = data_dir.join("db");

    if !db_dir.exists() {
        fs::create_dir_all(&db_dir).with_context(|| {
            format!("Failed to create database directory: {}", db_dir.display())
        })?;
    }

    Ok(db_dir)
}

/// Get the GPU libraries directory.
///
/// # Platform Behavior
/// - All platforms: `<data_dir>/gpu-libs`
pub fn get_gpu_libs_dir() -> Result<PathBuf> {
    let data_dir = get_data_dir()?;
    let gpu_dir = data_dir.join("gpu-libs");

    if !gpu_dir.exists() {
        fs::create_dir_all(&gpu_dir).with_context(|| {
            format!("Failed to create GPU libs directory: {}", gpu_dir.display())
        })?;
    }

    Ok(gpu_dir)
}

/// Set secure Unix socket permissions.
///
/// Sets the socket to mode 0o600 (owner read/write only).
///
/// # Platform Behavior
/// - **Linux/macOS**: Sets file permissions
/// - **Windows**: No-op (named pipes have different security model)
#[cfg(unix)]
pub fn secure_socket_permissions(socket_path: &PathBuf) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    if socket_path.exists() {
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(socket_path, perms).with_context(|| {
            format!(
                "Failed to set socket permissions: {}",
                socket_path.display()
            )
        })?;
    }

    Ok(())
}

#[cfg(not(unix))]
pub fn secure_socket_permissions(_socket_path: &PathBuf) -> Result<()> {
    // No-op on non-Unix platforms
    Ok(())
}

// ============================================================================
// Simple API (non-Result versions for contexts where errors are fatal)
// ============================================================================

/// Get the IPC socket path, panicking on failure.
///
/// Use this in contexts where failure to get the path is unrecoverable.
pub fn ipc_socket_path() -> PathBuf {
    get_ipc_socket_path().expect("Failed to determine IPC socket path")
}

/// Get the data directory, panicking on failure.
pub fn data_dir() -> PathBuf {
    get_data_dir().expect("Failed to determine data directory")
}

/// Get the models directory, panicking on failure.
pub fn models_dir() -> PathBuf {
    get_models_dir().expect("Failed to determine models directory")
}

/// Get the logs directory, panicking on failure.
pub fn logs_dir() -> PathBuf {
    get_logs_dir().expect("Failed to determine logs directory")
}

/// Get the config directory, panicking on failure.
pub fn config_dir() -> PathBuf {
    get_config_dir().expect("Failed to determine config directory")
}

/// Get the socket directory, panicking on failure.
pub fn socket_dir() -> PathBuf {
    get_socket_dir().expect("Failed to determine socket directory")
}

/// Get the metrics socket path, panicking on failure.
pub fn metrics_socket_path() -> PathBuf {
    get_metrics_socket_path().expect("Failed to determine metrics socket path")
}

/// Get the database directory, panicking on failure.
pub fn db_dir() -> PathBuf {
    get_db_dir().expect("Failed to determine database directory")
}

/// Get the GPU libraries directory, panicking on failure.
pub fn gpu_libs_dir() -> PathBuf {
    get_gpu_libs_dir().expect("Failed to determine GPU libs directory")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_dir_creation() {
        let dir = get_data_dir().expect("Should get data directory");
        assert!(dir.exists(), "Data directory should exist");
        assert!(dir.ends_with("swictation"), "Should end with app name");
    }

    #[test]
    fn test_ipc_socket_path() {
        let path = get_ipc_socket_path().expect("Should get socket path");
        assert!(
            path.to_string_lossy().contains("swictation.sock"),
            "Should contain socket filename"
        );
    }

    #[test]
    fn test_models_dir() {
        let dir = get_models_dir().expect("Should get models directory");
        assert!(dir.exists(), "Models directory should exist");
        assert!(dir.ends_with("models"), "Should end with 'models'");
    }

    #[test]
    fn test_logs_dir() {
        let dir = get_logs_dir().expect("Should get logs directory");
        assert!(dir.exists(), "Logs directory should exist");
    }

    #[test]
    fn test_config_dir() {
        let dir = get_config_dir().expect("Should get config directory");
        assert!(dir.exists(), "Config directory should exist");
    }

    #[test]
    fn test_simple_api() {
        // These should not panic
        let _ = ipc_socket_path();
        let _ = data_dir();
        let _ = models_dir();
        let _ = logs_dir();
        let _ = config_dir();
        let _ = socket_dir();
        let _ = metrics_socket_path();
        let _ = db_dir();
        let _ = gpu_libs_dir();
    }
}
