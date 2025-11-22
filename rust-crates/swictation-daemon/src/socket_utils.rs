//! Unix socket path utilities with security-focused defaults
//!
//! Provides secure socket paths using XDG_RUNTIME_DIR with fallback to user-specific locations.

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Get secure socket directory path
///
/// Priority:
/// 1. XDG_RUNTIME_DIR (user-specific, mode 0700, auto-cleaned)
/// 2. ~/.local/share/swictation (user-specific, manual creation)
pub fn get_socket_dir() -> Result<PathBuf> {
    // Try XDG_RUNTIME_DIR first (best practice for sockets)
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        let path = PathBuf::from(runtime_dir);
        if path.exists() {
            return Ok(path);
        }
    }

    // Fallback to ~/.local/share/swictation
    let home = std::env::var("HOME").context("HOME environment variable not set")?;

    let socket_dir = PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("swictation");

    // Create directory if it doesn't exist
    if !socket_dir.exists() {
        std::fs::create_dir_all(&socket_dir).context("Failed to create socket directory")?;
    }

    // Ensure directory has secure permissions (0700 = owner-only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = std::fs::Permissions::from_mode(0o700);
        std::fs::set_permissions(&socket_dir, permissions)
            .context("Failed to set socket directory permissions")?;
    }

    Ok(socket_dir)
}

/// Get path for main IPC socket (toggle commands)
pub fn get_ipc_socket_path() -> Result<PathBuf> {
    Ok(get_socket_dir()?.join("swictation.sock"))
}

/// Get path for metrics broadcast socket (UI clients)
pub fn get_metrics_socket_path() -> Result<PathBuf> {
    Ok(get_socket_dir()?.join("swictation_metrics.sock"))
}

/// Set secure permissions on a socket file (0600 = owner read/write only)
#[cfg(unix)]
pub fn secure_socket_permissions(socket_path: &std::path::Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    if socket_path.exists() {
        let permissions = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(socket_path, permissions)
            .context("Failed to set socket permissions")?;
    }

    Ok(())
}

#[cfg(not(unix))]
pub fn secure_socket_permissions(_socket_path: &std::path::Path) -> Result<()> {
    // Non-Unix platforms don't use Unix sockets
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_dir_exists() {
        let dir = get_socket_dir().unwrap();
        assert!(dir.is_absolute());
    }

    #[test]
    fn test_ipc_socket_path() {
        let path = get_ipc_socket_path().unwrap();
        assert!(path.ends_with("swictation.sock"));
        assert!(path.is_absolute());
    }

    #[test]
    fn test_metrics_socket_path() {
        let path = get_metrics_socket_path().unwrap();
        assert!(path.ends_with("swictation_metrics.sock"));
        assert!(path.is_absolute());
    }
}
