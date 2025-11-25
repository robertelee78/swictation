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
///
/// Uses platform-appropriate socket directory:
/// - Linux: XDG_RUNTIME_DIR or ~/.local/share/swictation/swictation_metrics.sock
/// - macOS: ~/Library/Application Support/swictation/swictation_metrics.sock
pub fn get_default_socket_path() -> String {
    get_socket_dir()
        .join("swictation_metrics.sock")
        .to_string_lossy()
        .to_string()
}

/// Get secure socket directory path (matches daemon and socket_utils)
fn get_socket_dir() -> PathBuf {
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
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            let path = PathBuf::from(runtime_dir);
            if path.exists() {
                return path;
            }
        }

        // Fallback to ~/.local/share/swictation
        dirs::data_local_dir()
            .expect("Failed to get data directory")
            .join("swictation")
    }
}
