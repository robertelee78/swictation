//! Socket path utilities matching daemon implementation
//! Uses XDG_RUNTIME_DIR with fallback to ~/.local/share/swictation

use std::env;
use std::path::PathBuf;

/// Get secure socket directory path
///
/// Priority:
/// 1. XDG_RUNTIME_DIR (user-specific, mode 0700, auto-cleaned)
/// 2. ~/.local/share/swictation (user-specific, manual creation)
pub fn get_socket_dir() -> PathBuf {
    // Try XDG_RUNTIME_DIR first (best practice for sockets)
    if let Ok(runtime_dir) = env::var("XDG_RUNTIME_DIR") {
        let path = PathBuf::from(runtime_dir);
        if path.exists() {
            return path;
        }
    }

    // Fallback to ~/.local/share/swictation
    let home = env::var("HOME")
        .unwrap_or_else(|_| String::from("/tmp"));

    PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("swictation")
}

/// Get path for main IPC socket (toggle commands)
pub fn get_ipc_socket_path() -> PathBuf {
    get_socket_dir().join("swictation.sock")
}

/// Get path for metrics broadcast socket (UI clients)
pub fn get_metrics_socket_path() -> PathBuf {
    get_socket_dir().join("swictation_metrics.sock")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_paths() {
        let ipc_path = get_ipc_socket_path();
        assert!(ipc_path.ends_with("swictation.sock"));

        let metrics_path = get_metrics_socket_path();
        assert!(metrics_path.ends_with("swictation_metrics.sock"));
    }
}
