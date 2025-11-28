//! Socket path utilities matching daemon implementation
//!
//! Re-exports from swictation-paths crate for consistent cross-platform paths.
//! Provides backward compatibility wrappers for the original API.
//!
//! Platform-specific behavior:
//! - Linux: XDG_RUNTIME_DIR or ~/.local/share/swictation
//! - macOS: ~/Library/Application Support/swictation

use std::path::PathBuf;

/// Get secure socket directory path
///
/// Platform-specific behavior:
/// - Linux: XDG_RUNTIME_DIR (preferred) or ~/.local/share/swictation (fallback)
/// - macOS: ~/Library/Application Support/swictation
///
/// This is a compatibility wrapper for the swictation-paths crate.
#[allow(dead_code)]
pub fn get_socket_dir() -> PathBuf {
    swictation_paths::socket_dir()
}

/// Get path for metrics broadcast socket (UI clients)
///
/// Returns the path to swictation_metrics.sock in the socket directory.
///
/// This is a compatibility wrapper for the swictation-paths crate.
pub fn get_metrics_socket_path() -> PathBuf {
    swictation_paths::metrics_socket_path()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_dir() {
        let dir = get_socket_dir();
        assert!(dir.is_absolute());
    }

    #[test]
    fn test_metrics_socket_path() {
        let metrics_path = get_metrics_socket_path();
        assert!(metrics_path.ends_with("swictation_metrics.sock"));
        assert!(metrics_path.is_absolute());
    }
}
