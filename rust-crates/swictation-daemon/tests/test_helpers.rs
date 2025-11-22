//! Test helpers for display server detection and text injection testing
//!
//! Provides utilities for:
//! - Environment variable manipulation
//! - Mock setup for tool availability
//! - Test fixture creation
//! - Performance measurement

use std::collections::HashMap;
use std::sync::Mutex;

/// Thread-safe environment variable storage for testing
///
/// We can't actually set environment variables in tests because:
/// 1. Tests run in parallel
/// 2. Environment variables are process-global
/// 3. Would cause race conditions
///
/// Instead, we inject a custom environment getter during tests.
static TEST_ENV: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

/// Set up test environment with specific variables
pub fn setup_test_env(vars: HashMap<String, String>) {
    let mut env = TEST_ENV.lock().unwrap();
    *env = Some(vars);
}

/// Clear test environment
pub fn clear_test_env() {
    let mut env = TEST_ENV.lock().unwrap();
    *env = None;
}

/// Get environment variable for testing
/// Falls back to actual env::var if not in test mode
pub fn test_env_var(key: &str) -> Result<String, std::env::VarError> {
    let env = TEST_ENV.lock().unwrap();
    if let Some(ref vars) = *env {
        vars.get(key)
            .map(|s| s.clone())
            .ok_or(std::env::VarError::NotPresent)
    } else {
        std::env::var(key)
    }
}

/// Test fixture for X11 environment
pub fn x11_env() -> HashMap<String, String> {
    let mut env = HashMap::new();
    env.insert("DISPLAY".to_string(), ":0".to_string());
    env.insert("XDG_SESSION_TYPE".to_string(), "x11".to_string());
    env
}

/// Test fixture for pure Wayland environment (KDE)
pub fn wayland_kde_env() -> HashMap<String, String> {
    let mut env = HashMap::new();
    env.insert("WAYLAND_DISPLAY".to_string(), "wayland-0".to_string());
    env.insert("XDG_SESSION_TYPE".to_string(), "wayland".to_string());
    env.insert("XDG_CURRENT_DESKTOP".to_string(), "KDE".to_string());
    env
}

/// Test fixture for GNOME Wayland environment
pub fn wayland_gnome_env() -> HashMap<String, String> {
    let mut env = HashMap::new();
    env.insert("WAYLAND_DISPLAY".to_string(), "wayland-0".to_string());
    env.insert("XDG_SESSION_TYPE".to_string(), "wayland".to_string());
    env.insert("XDG_CURRENT_DESKTOP".to_string(), "GNOME".to_string());
    env
}

/// Test fixture for Sway environment
pub fn sway_env() -> HashMap<String, String> {
    let mut env = HashMap::new();
    env.insert(
        "SWAYSOCK".to_string(),
        "/run/user/1000/sway-ipc.sock".to_string(),
    );
    env.insert("WAYLAND_DISPLAY".to_string(), "wayland-0".to_string());
    env.insert("XDG_SESSION_TYPE".to_string(), "wayland".to_string());
    env.insert("XDG_CURRENT_DESKTOP".to_string(), "sway".to_string());
    env
}

/// Test fixture for XWayland environment (X11 apps on Wayland)
pub fn xwayland_env() -> HashMap<String, String> {
    let mut env = HashMap::new();
    env.insert("DISPLAY".to_string(), ":0".to_string());
    env.insert("WAYLAND_DISPLAY".to_string(), "wayland-0".to_string());
    env.insert("XDG_SESSION_TYPE".to_string(), "wayland".to_string());
    env.insert("XDG_CURRENT_DESKTOP".to_string(), "GNOME".to_string());
    env
}

/// Test fixture for headless/unknown environment
pub fn headless_env() -> HashMap<String, String> {
    HashMap::new() // No display server variables
}

/// Test fixture for ambiguous environment (old systems)
pub fn ambiguous_env() -> HashMap<String, String> {
    let mut env = HashMap::new();
    env.insert("DISPLAY".to_string(), ":0".to_string());
    // No XDG_SESSION_TYPE (old system)
    env
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_setup_and_retrieval() {
        let mut env = HashMap::new();
        env.insert("TEST_VAR".to_string(), "test_value".to_string());

        setup_test_env(env);

        let value = test_env_var("TEST_VAR");
        assert_eq!(value.unwrap(), "test_value");

        clear_test_env();

        let value = test_env_var("TEST_VAR");
        assert!(value.is_err());
    }

    #[test]
    fn test_x11_fixture() {
        let env = x11_env();
        assert_eq!(env.get("DISPLAY"), Some(&":0".to_string()));
        assert_eq!(env.get("XDG_SESSION_TYPE"), Some(&"x11".to_string()));
        assert!(env.get("WAYLAND_DISPLAY").is_none());
    }

    #[test]
    fn test_gnome_wayland_fixture() {
        let env = wayland_gnome_env();
        assert_eq!(env.get("XDG_CURRENT_DESKTOP"), Some(&"GNOME".to_string()));
        assert_eq!(env.get("XDG_SESSION_TYPE"), Some(&"wayland".to_string()));
    }

    #[test]
    fn test_xwayland_fixture() {
        let env = xwayland_env();
        // XWayland has BOTH DISPLAY and WAYLAND_DISPLAY
        assert!(env.contains_key("DISPLAY"));
        assert!(env.contains_key("WAYLAND_DISPLAY"));
        assert_eq!(env.get("XDG_SESSION_TYPE"), Some(&"wayland".to_string()));
    }
}
