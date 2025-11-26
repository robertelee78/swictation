//! Socket path utilities matching daemon implementation
//!
//! Provides socket paths using platform-appropriate locations:
//! - Linux: XDG_RUNTIME_DIR or ~/.local/share/swictation
//! - macOS: ~/Library/Application Support/swictation

#[cfg(not(target_os = "macos"))]
use std::env;
use std::path::PathBuf;

/// Get secure socket directory path
///
/// Platform-specific behavior:
/// - Linux: XDG_RUNTIME_DIR (preferred) or ~/.local/share/swictation (fallback)
/// - macOS: ~/Library/Application Support/swictation
pub fn get_socket_dir() -> PathBuf {
    // macOS: Use Application Support directory
    #[cfg(target_os = "macos")]
    {
        return dirs::data_local_dir()
            .expect("Failed to get Application Support directory")
            .join("swictation");
    }

    // Linux: Try XDG_RUNTIME_DIR first (best practice for sockets)
    #[cfg(not(target_os = "macos"))]
    {
        if let Ok(runtime_dir) = env::var("XDG_RUNTIME_DIR") {
            let path = PathBuf::from(runtime_dir);
            if path.exists() {
                return path;
            }
        }

        // Fallback to ~/.local/share/swictation using dirs crate
        dirs::data_local_dir()
            .expect("Failed to get data directory")
            .join("swictation")
    }
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
        let metrics_path = get_metrics_socket_path();
        assert!(metrics_path.ends_with("swictation_metrics.sock"));
    }
}
