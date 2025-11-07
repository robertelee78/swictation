use std::path::PathBuf;

/// Get the default database path
pub fn get_default_db_path() -> PathBuf {
    dirs::home_dir()
        .expect("Failed to get home directory")
        .join(".local")
        .join("share")
        .join("swictation")
        .join("metrics.db")
}

/// Get the default socket path
pub fn get_default_socket_path() -> String {
    "/tmp/swictation_metrics.sock".to_string()
}
