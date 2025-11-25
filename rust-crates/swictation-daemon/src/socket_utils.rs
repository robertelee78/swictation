//! Unix socket path utilities with security-focused defaults
//!
//! Provides secure socket paths using platform-appropriate locations:
//! - Linux: XDG_RUNTIME_DIR or ~/.local/share/swictation
//! - macOS: ~/Library/Application Support/swictation

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Get secure socket directory path
///
/// Platform-specific behavior:
/// - Linux: XDG_RUNTIME_DIR (preferred) or ~/.local/share/swictation (fallback)
/// - macOS: ~/Library/Application Support/swictation
pub fn get_socket_dir() -> Result<PathBuf> {
    // macOS: Use Application Support directory
    #[cfg(target_os = "macos")]
    {
        let socket_dir = dirs::data_local_dir()
            .context("Failed to get Application Support directory")?
            .join("swictation");

        // Create directory if it doesn't exist
        if !socket_dir.exists() {
            std::fs::create_dir_all(&socket_dir).context("Failed to create socket directory")?;
        }

        // Ensure directory has secure permissions (0700 = owner-only)
        use std::os::unix::fs::PermissionsExt;
        let permissions = std::fs::Permissions::from_mode(0o700);
        std::fs::set_permissions(&socket_dir, permissions)
            .context("Failed to set socket directory permissions")?;

        return Ok(socket_dir);
    }

    // Linux: Try XDG_RUNTIME_DIR first (best practice for sockets)
    #[cfg(not(target_os = "macos"))]
    {
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            let path = PathBuf::from(runtime_dir);
            if path.exists() {
                return Ok(path);
            }
        }

        // Fallback to ~/.local/share/swictation
        let socket_dir = dirs::data_local_dir()
            .context("Failed to get data directory")?
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
#[allow(dead_code)]
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

#[allow(dead_code)]
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
