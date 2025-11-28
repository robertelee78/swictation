//! Unix socket path utilities with security-focused defaults
//!
//! Re-exports from swictation-paths crate for consistent cross-platform paths.
//! This module provides backward compatibility for existing daemon code.

// Re-export the functions actually used by the daemon
pub use swictation_paths::{
    get_ipc_socket_path,
    get_metrics_socket_path,
};

// Re-export additional utilities for potential future use and API consistency
// These are currently unused in production code but used in tests
#[allow(unused_imports)]
pub use swictation_paths::{
    get_socket_dir,
    secure_socket_permissions,
};

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
