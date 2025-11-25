use std::path::PathBuf;

/// Get the default database path
///
/// Uses platform-appropriate data directory:
/// - Linux: ~/.local/share/swictation/metrics.db
/// - macOS: ~/Library/Application Support/swictation/metrics.db
/// - Windows: C:\Users\<user>\AppData\Local\swictation\metrics.db
pub fn get_default_db_path() -> PathBuf {
    dirs::data_local_dir()
        .expect("Failed to get data directory")
        .join("swictation")
        .join("metrics.db")
}

/// Get the default socket path
pub fn get_default_socket_path() -> String {
    "/tmp/swictation_metrics.sock".to_string()
}
