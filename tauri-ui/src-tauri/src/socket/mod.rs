// Socket connection module for real-time metrics streaming
//
// This module provides:
// - Async Unix socket connection for metrics streaming (MetricsSocket)
// - Automatic reconnection on disconnect
// - Event parsing and Tauri integration

mod metrics;
mod socket_utils;

// Primary exports
pub use metrics::MetricsSocket;
pub use socket_utils::get_metrics_socket_path;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::socket::socket_utils;

    #[test]
    fn test_socket_path_validation() {
        let socket_path = socket_utils::get_metrics_socket_path();
        let socket_str = socket_path.to_string_lossy();
        assert!(!socket_str.is_empty());
        // Socket should NEVER be in /tmp - must be in platform-appropriate directory
        assert!(!socket_str.starts_with("/tmp/"), "Socket path must not be in /tmp");
        assert!(socket_str.ends_with("swictation_metrics.sock"));
    }
}
