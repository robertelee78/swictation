//! Display server detection tests
//!
//! Comprehensive tests for all environment variable combinations to ensure correct detection of:
//! - X11 (pure)
//! - Wayland (KDE, GNOME, Sway)
//! - XWayland (X11 apps on Wayland)
//! - Unknown/Headless
//!
//! Critical: GNOME Wayland detection must be accurate since it determines
//! whether to use wtype (won't work) or ydotool (required).

mod test_helpers;

use std::collections::HashMap;
use test_helpers::*;

// Import internal module for testing
// We need to use the internal detection function that accepts EnvProvider
use swictation_daemon::display_server::{
    DisplayServer, ConfidenceLevel,
};

// Mock environment provider for testing
struct MockEnv {
    vars: HashMap<String, String>,
}

impl MockEnv {
    fn new(vars: HashMap<String, String>) -> Self {
        Self { vars }
    }
}

impl swictation_daemon::display_server::EnvProvider for MockEnv {
    fn get(&self, key: &str) -> Option<String> {
        self.vars.get(key).cloned()
    }
}

// Helper to call internal detection function
fn detect_with_env(env: HashMap<String, String>) -> swictation_daemon::display_server::DisplayServerInfo {
    let mock_env = MockEnv::new(env);
    // Call the internal pub(crate) function
    // We'll need to make it pub for tests
    swictation_daemon::display_server::detect_display_server_with_env(&mock_env)
}

#[test]
fn test_x11_pure_detection() {
    // Pure X11: DISPLAY set, no WAYLAND_DISPLAY, XDG_SESSION_TYPE=x11
    // Expected: DisplayServer::X11, confidence High (4 + 1 = 5 points)
    let env = x11_env();
    let info = detect_with_env(env);

    assert_eq!(info.server_type, DisplayServer::X11);
    assert_eq!(info.confidence, ConfidenceLevel::High);
    assert_eq!(info.is_gnome_wayland, false);
}

#[test]
fn test_wayland_kde_detection() {
    // KDE Plasma Wayland: WAYLAND_DISPLAY set, XDG_SESSION_TYPE=wayland, XDG_CURRENT_DESKTOP=KDE
    // Expected: DisplayServer::Wayland, is_gnome_wayland=false, confidence High (4 + 2 = 6 points)
    let env = wayland_kde_env();
    let info = detect_with_env(env);

    assert_eq!(info.server_type, DisplayServer::Wayland);
    assert_eq!(info.confidence, ConfidenceLevel::High);
    assert_eq!(info.is_gnome_wayland, false);
    assert_eq!(info.desktop_environment, Some("KDE".to_string()));
}

#[test]
fn test_wayland_gnome_detection() {
    // GNOME Wayland: WAYLAND_DISPLAY set, XDG_SESSION_TYPE=wayland, XDG_CURRENT_DESKTOP=GNOME
    // Expected: DisplayServer::Wayland, is_gnome_wayland=TRUE, confidence High
    //
    // This is CRITICAL: is_gnome_wayland MUST be true so tool selection uses ydotool
    let env = wayland_gnome_env();
    let info = detect_with_env(env);

    assert_eq!(info.server_type, DisplayServer::Wayland);
    assert_eq!(info.confidence, ConfidenceLevel::High);
    assert_eq!(info.is_gnome_wayland, true); // CRITICAL TEST
    assert_eq!(info.desktop_environment, Some("GNOME".to_string()));
}

#[test]
fn test_xwayland_detection() {
    // XWayland: Both DISPLAY and WAYLAND_DISPLAY set, XDG_SESSION_TYPE=wayland
    // Expected: DisplayServer::Wayland (NOT X11)
    //
    // XDG_SESSION_TYPE takes priority over DISPLAY presence
    // Wayland score (4+2=6) > X11 score (1)
    let env = xwayland_env();
    let info = detect_with_env(env);

    assert_eq!(info.server_type, DisplayServer::Wayland);
    assert_eq!(info.confidence, ConfidenceLevel::High);
}

#[test]
fn test_sway_detection() {
    // Sway: SWAYSOCK set, WAYLAND_DISPLAY set, XDG_SESSION_TYPE=wayland
    // Expected: DisplayServer::Wayland, is_gnome_wayland=false
    //
    // Note: SWAYSOCK is not used for display server detection, only in hotkey.rs
    let env = sway_env();
    let info = detect_with_env(env);

    assert_eq!(info.server_type, DisplayServer::Wayland);
    assert_eq!(info.confidence, ConfidenceLevel::High);
    assert_eq!(info.is_gnome_wayland, false);
}

#[test]
fn test_headless_detection() {
    // Headless: No display server environment variables
    // Expected: DisplayServer::Unknown, confidence Low
    let env = headless_env();
    let info = detect_with_env(env);

    assert_eq!(info.server_type, DisplayServer::Unknown);
    assert_eq!(info.confidence, ConfidenceLevel::Low);
    assert_eq!(info.is_gnome_wayland, false);
}

#[test]
fn test_ambiguous_old_system() {
    // Old system: DISPLAY set but no XDG_SESSION_TYPE
    // Expected: DisplayServer::X11, confidence Low (only 1 point from DISPLAY)
    let env = ambiguous_env();
    let info = detect_with_env(env);

    assert_eq!(info.server_type, DisplayServer::X11);
    assert_eq!(info.confidence, ConfidenceLevel::Low);
}

#[test]
fn test_confidence_scoring_x11() {
    // Test evidence-based scoring for X11:
    // - XDG_SESSION_TYPE=x11 → 4 points
    // - DISPLAY=:0 → 1 point
    // Total: 5 points → High confidence
    let env = x11_env();
    let info = detect_with_env(env);

    assert_eq!(info.confidence, ConfidenceLevel::High);
}

#[test]
fn test_confidence_scoring_wayland() {
    // Test evidence-based scoring for Wayland:
    // - XDG_SESSION_TYPE=wayland → 4 points
    // - WAYLAND_DISPLAY=wayland-0 → 2 points
    // Total: 6 points → High confidence
    let env = wayland_kde_env();
    let info = detect_with_env(env);

    assert_eq!(info.confidence, ConfidenceLevel::High);
}

#[test]
fn test_gnome_variations() {
    // Test all GNOME desktop environment variations
    let test_cases = vec![
        "GNOME",
        "ubuntu:GNOME",
        "GNOME:GNOME",
        "gnome",        // lowercase
        "Gnome",        // mixed case
        "Ubuntu:gnome", // mixed with colon
    ];

    for desktop_var in test_cases {
        let mut env = HashMap::new();
        env.insert("WAYLAND_DISPLAY".to_string(), "wayland-0".to_string());
        env.insert("XDG_SESSION_TYPE".to_string(), "wayland".to_string());
        env.insert("XDG_CURRENT_DESKTOP".to_string(), desktop_var.to_string());

        let info = detect_with_env(env);

        assert_eq!(
            info.is_gnome_wayland, true,
            "Failed to detect GNOME for desktop environment: {}",
            desktop_var
        );
    }
}

#[test]
fn test_desktop_environment_parsing() {
    // Test XDG_CURRENT_DESKTOP parsing for various values
    let test_cases = vec![
        ("KDE", false),
        ("GNOME", true),
        ("ubuntu:GNOME", true),
        ("XFCE", false),
        ("Hyprland", false),
        ("sway", false),
        ("i3", false),
        ("MATE", false),
    ];

    for (desktop, should_be_gnome) in test_cases {
        let mut env = HashMap::new();
        env.insert("WAYLAND_DISPLAY".to_string(), "wayland-0".to_string());
        env.insert("XDG_SESSION_TYPE".to_string(), "wayland".to_string());
        env.insert("XDG_CURRENT_DESKTOP".to_string(), desktop.to_string());

        let info = detect_with_env(env);

        assert_eq!(
            info.is_gnome_wayland, should_be_gnome,
            "Incorrect GNOME detection for desktop environment: {}",
            desktop
        );
    }
}

#[test]
fn test_wayland_without_xdg_session_type() {
    // Edge case: WAYLAND_DISPLAY set but no XDG_SESSION_TYPE
    // Expected: Wayland with Medium confidence (2 points only)
    let mut env = HashMap::new();
    env.insert("WAYLAND_DISPLAY".to_string(), "wayland-0".to_string());

    let info = detect_with_env(env);

    assert_eq!(info.server_type, DisplayServer::Wayland);
    assert_eq!(info.confidence, ConfidenceLevel::Medium);
}

#[test]
fn test_x11_wayland_tie() {
    // Edge case: Both DISPLAY and WAYLAND_DISPLAY but no XDG_SESSION_TYPE
    // X11 score = 1, Wayland score = 2
    // Expected: Wayland wins (higher score)
    let mut env = HashMap::new();
    env.insert("DISPLAY".to_string(), ":0".to_string());
    env.insert("WAYLAND_DISPLAY".to_string(), "wayland-0".to_string());

    let info = detect_with_env(env);

    assert_eq!(info.server_type, DisplayServer::Wayland);
    assert_eq!(info.confidence, ConfidenceLevel::Medium);
}

#[test]
fn test_gnome_on_x11() {
    // GNOME can run on X11 too - should NOT set is_gnome_wayland
    let mut env = HashMap::new();
    env.insert("DISPLAY".to_string(), ":0".to_string());
    env.insert("XDG_SESSION_TYPE".to_string(), "x11".to_string());
    env.insert("XDG_CURRENT_DESKTOP".to_string(), "GNOME".to_string());

    let info = detect_with_env(env);

    assert_eq!(info.server_type, DisplayServer::X11);
    assert_eq!(info.is_gnome_wayland, false); // Should be false (X11, not Wayland)
}

#[test]
fn test_confidence_levels() {
    // Test all confidence level thresholds

    // High: >= 4 points
    let mut env = HashMap::new();
    env.insert("XDG_SESSION_TYPE".to_string(), "x11".to_string());
    let info = detect_with_env(env.clone());
    assert_eq!(info.confidence, ConfidenceLevel::High);

    // Medium: 2-3 points
    env.clear();
    env.insert("WAYLAND_DISPLAY".to_string(), "wayland-0".to_string());
    let info = detect_with_env(env.clone());
    assert_eq!(info.confidence, ConfidenceLevel::Medium);

    // Low: < 2 points
    env.clear();
    env.insert("DISPLAY".to_string(), ":0".to_string());
    let info = detect_with_env(env);
    assert_eq!(info.confidence, ConfidenceLevel::Low);
}
