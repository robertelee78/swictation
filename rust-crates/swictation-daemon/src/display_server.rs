//! Display server detection and text injection tool selection
//!
//! This module provides comprehensive display server detection for X11 and Wayland,
//! with special handling for GNOME Wayland which requires ydotool instead of wtype.
//!
//! Research findings (2024):
//! - X11 still dominates (80-90% of Linux desktop users)
//! - Wayland adoption slow despite being default since 2023
//! - GNOME is most popular DE (35-45%) but wtype doesn't work on GNOME Wayland
//! - GNOME's Mutter compositor lacks virtual-keyboard protocol
//!
//! Tool compatibility:
//! - xdotool: X11 only (fast, mature)
//! - wtype: Wayland only (KDE, Sway, Hyprland work; GNOME does NOT)
//! - ydotool: Universal (X11, all Wayland compositors, even TTY)

use anyhow::Result;
use std::process::Command;
use tracing::{debug, info, warn};

/// Trait for environment variable access (enables testing with mock environments)
/// This is public so tests can implement mock environments
pub trait EnvProvider {
    fn get(&self, key: &str) -> Option<String>;
}

/// Default environment provider using std::env::var
pub struct SystemEnv;

impl EnvProvider for SystemEnv {
    fn get(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}

/// Display server type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayServer {
    X11,
    Wayland,
    Unknown,
}

/// Text injection tool options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextInjectionTool {
    /// xdotool - X11 text injection (fast, mature)
    Xdotool,
    /// wtype - Wayland text injection (compatible with KDE, Sway, Hyprland)
    Wtype,
    /// ydotool - Universal text injection (works everywhere via kernel uinput)
    Ydotool,
}

impl TextInjectionTool {
    /// Get the command name for this tool
    pub fn command(&self) -> &'static str {
        match self {
            Self::Xdotool => "xdotool",
            Self::Wtype => "wtype",
            Self::Ydotool => "ydotool",
        }
    }

    /// Get human-readable tool name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Xdotool => "xdotool",
            Self::Wtype => "wtype",
            Self::Ydotool => "ydotool",
        }
    }
}

/// Detailed display server information
#[derive(Debug, Clone)]
pub struct DisplayServerInfo {
    /// Detected display server type
    pub server_type: DisplayServer,
    /// Desktop environment (e.g., "GNOME", "KDE", "Sway")
    pub desktop_environment: Option<String>,
    /// Whether this is GNOME running on Wayland (requires ydotool)
    pub is_gnome_wayland: bool,
    /// Confidence level in detection
    pub confidence: ConfidenceLevel,
}

/// Confidence level in display server detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidenceLevel {
    /// High confidence (≥4 evidence points)
    High,
    /// Medium confidence (2-3 evidence points)
    Medium,
    /// Low confidence (<2 evidence points)
    Low,
}

/// Detect the current display server with evidence-based scoring
///
/// Detection priority (based on research):
/// 1. XDG_SESSION_TYPE (most reliable)
/// 2. WAYLAND_DISPLAY (Wayland-specific)
/// 3. DISPLAY (X11 or XWayland)
pub fn detect_display_server() -> DisplayServerInfo {
    detect_display_server_with_env(&SystemEnv)
}

/// Internal detection function that accepts environment provider (for testing)
/// This is public so integration tests can inject mock environments
pub fn detect_display_server_with_env(env: &dyn EnvProvider) -> DisplayServerInfo {
    let session_type = env.get("XDG_SESSION_TYPE");
    let desktop = env.get("XDG_CURRENT_DESKTOP");
    let wayland_display = env.get("WAYLAND_DISPLAY");
    let x11_display = env.get("DISPLAY");

    debug!("Environment variables:");
    debug!("  XDG_SESSION_TYPE: {:?}", session_type);
    debug!("  XDG_CURRENT_DESKTOP: {:?}", desktop);
    debug!("  WAYLAND_DISPLAY: {:?}", wayland_display);
    debug!("  DISPLAY: {:?}", x11_display);

    // Evidence-based scoring
    let mut x11_score = 0;
    let mut wayland_score = 0;

    // XDG_SESSION_TYPE is most reliable (4 points)
    match session_type.as_deref() {
        Some("x11") => x11_score += 4,
        Some("wayland") => wayland_score += 4,
        _ => {}
    }

    // WAYLAND_DISPLAY is Wayland-specific (2 points)
    if wayland_display.is_some() {
        wayland_score += 2;
    }

    // DISPLAY can be X11 or XWayland (1 point)
    if x11_display.is_some() {
        x11_score += 1;
    }

    // Determine server type and confidence
    let (server_type, confidence) = if wayland_score > x11_score {
        let confidence = if wayland_score >= 4 {
            ConfidenceLevel::High
        } else if wayland_score >= 2 {
            ConfidenceLevel::Medium
        } else {
            ConfidenceLevel::Low
        };
        (DisplayServer::Wayland, confidence)
    } else if x11_score > wayland_score {
        let confidence = if x11_score >= 4 {
            ConfidenceLevel::High
        } else if x11_score >= 2 {
            ConfidenceLevel::Medium
        } else {
            ConfidenceLevel::Low
        };
        (DisplayServer::X11, confidence)
    } else {
        (DisplayServer::Unknown, ConfidenceLevel::Low)
    };

    // Check for GNOME Wayland (special case - wtype doesn't work)
    let is_gnome_wayland = server_type == DisplayServer::Wayland
        && desktop
            .as_ref()
            .map(|d| d.to_lowercase().contains("gnome"))
            .unwrap_or(false);

    let info = DisplayServerInfo {
        server_type,
        desktop_environment: desktop,
        is_gnome_wayland,
        confidence,
    };

    info!(
        "Detected display server: {:?} (confidence: {:?})",
        info.server_type, info.confidence
    );
    if info.is_gnome_wayland {
        info!("GNOME Wayland detected - ydotool required (wtype will not work)");
    }

    info
}

/// Check if a tool is available on the system
pub fn is_tool_available(tool: TextInjectionTool) -> bool {
    Command::new("which")
        .arg(tool.command())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Detect all available text injection tools
pub fn detect_available_tools() -> Vec<TextInjectionTool> {
    let mut tools = Vec::new();

    if is_tool_available(TextInjectionTool::Xdotool) {
        tools.push(TextInjectionTool::Xdotool);
    }

    if is_tool_available(TextInjectionTool::Wtype) {
        tools.push(TextInjectionTool::Wtype);
    }

    if is_tool_available(TextInjectionTool::Ydotool) {
        tools.push(TextInjectionTool::Ydotool);
    }

    debug!("Available tools: {:?}", tools);
    tools
}

/// Select the best text injection tool for the current environment
///
/// Selection logic (based on research):
/// - X11: Prefer xdotool (fast), fallback to ydotool
/// - Wayland (GNOME): Use ydotool (only option - wtype doesn't work)
/// - Wayland (KDE/Sway/Hyprland): Prefer wtype (fast), fallback to ydotool
/// - Unknown: Try ydotool (universal), then xdotool, then wtype
pub fn select_best_tool(
    server_info: &DisplayServerInfo,
    available_tools: &[TextInjectionTool],
) -> Result<TextInjectionTool> {
    match server_info.server_type {
        DisplayServer::X11 => {
            // X11: Prefer xdotool (primary), fallback to ydotool
            if available_tools.contains(&TextInjectionTool::Xdotool) {
                debug!("Selected xdotool for X11 (optimal)");
                Ok(TextInjectionTool::Xdotool)
            } else if available_tools.contains(&TextInjectionTool::Ydotool) {
                warn!("xdotool not found, falling back to ydotool for X11");
                Ok(TextInjectionTool::Ydotool)
            } else {
                Err(anyhow::anyhow!(format_x11_error(server_info)))
            }
        }

        DisplayServer::Wayland => {
            // GNOME Wayland: Must use ydotool (wtype doesn't work)
            if server_info.is_gnome_wayland {
                if available_tools.contains(&TextInjectionTool::Ydotool) {
                    debug!("Selected ydotool for GNOME Wayland (required)");
                    Ok(TextInjectionTool::Ydotool)
                } else {
                    Err(anyhow::anyhow!(format_gnome_wayland_error(server_info)))
                }
            }
            // Other Wayland: Prefer wtype (fast), fallback to ydotool
            else {
                if available_tools.contains(&TextInjectionTool::Wtype) {
                    debug!("Selected wtype for Wayland (optimal)");
                    Ok(TextInjectionTool::Wtype)
                } else if available_tools.contains(&TextInjectionTool::Ydotool) {
                    warn!("wtype not found, falling back to ydotool for Wayland");
                    Ok(TextInjectionTool::Ydotool)
                } else {
                    Err(anyhow::anyhow!(format_wayland_error(server_info)))
                }
            }
        }

        DisplayServer::Unknown => {
            // Unknown: Try tools in order of compatibility
            if available_tools.contains(&TextInjectionTool::Ydotool) {
                warn!("Unknown display server, using ydotool (universal)");
                Ok(TextInjectionTool::Ydotool)
            } else if available_tools.contains(&TextInjectionTool::Xdotool) {
                warn!("Unknown display server, trying xdotool");
                Ok(TextInjectionTool::Xdotool)
            } else if available_tools.contains(&TextInjectionTool::Wtype) {
                warn!("Unknown display server, trying wtype");
                Ok(TextInjectionTool::Wtype)
            } else {
                Err(anyhow::anyhow!(format_unknown_error(server_info)))
            }
        }
    }
}

/// Format error message for missing X11 tool
fn format_x11_error(info: &DisplayServerInfo) -> String {
    format!(
        r#"Error: Text injection tool not found for X11

Required tool: xdotool
Install command:
  Ubuntu/Debian: sudo apt install xdotool
  Fedora:        sudo dnf install xdotool
  Arch:          sudo pacman -S xdotool

Alternative: Install ydotool (universal tool)
  Ubuntu/Debian: sudo apt install ydotool
  Fedora:        sudo dnf install ydotool
  Arch:          sudo pacman -S ydotool
  Setup:         sudo usermod -aG input $USER
  (Then log out and log back in)

Detected environment:
  Display Server:    X11
  DISPLAY:           {}
  XDG_SESSION_TYPE:  {}
  Desktop:           {}"#,
        std::env::var("DISPLAY").unwrap_or_else(|_| "not set".to_string()),
        std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "not set".to_string()),
        info.desktop_environment
            .as_ref()
            .unwrap_or(&"unknown".to_string())
    )
}

/// Format error message for GNOME Wayland without ydotool
fn format_gnome_wayland_error(info: &DisplayServerInfo) -> String {
    format!(
        r#"Error: GNOME Wayland requires ydotool

GNOME's Wayland compositor does not support wtype.
You must use ydotool for text injection.

Install ydotool:
  Ubuntu 24.04:  sudo apt install ydotool
  Fedora 40/41:  sudo dnf install ydotool
  Debian 12:     sudo apt install ydotool
  Arch:          sudo pacman -S ydotool

Grant permissions (REQUIRED):
  sudo usermod -aG input $USER

Then log out and log back in.

Detected environment:
  Display Server:        Wayland
  Desktop:               {}
  WAYLAND_DISPLAY:       {}
  XDG_SESSION_TYPE:      {}
  XDG_CURRENT_DESKTOP:   {}

Why? GNOME's Mutter compositor lacks the virtual-keyboard protocol
that wtype requires. ydotool uses kernel uinput instead."#,
        info.desktop_environment
            .as_ref()
            .unwrap_or(&"GNOME".to_string()),
        std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "not set".to_string()),
        std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "not set".to_string()),
        std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "not set".to_string())
    )
}

/// Format error message for missing Wayland tool
fn format_wayland_error(info: &DisplayServerInfo) -> String {
    format!(
        r#"Error: Wayland text injection tool not found

Recommended tool: wtype (fast)
  Ubuntu/Debian: sudo apt install wtype
  Fedora:        sudo dnf install wtype
  Arch:          sudo pacman -S wtype

Alternative: ydotool (universal, works everywhere)
  Ubuntu/Debian: sudo apt install ydotool
  Fedora:        sudo dnf install ydotool
  Arch:          sudo pacman -S ydotool
  Setup:         sudo usermod -aG input $USER
  (Then log out and log back in)

Detected environment:
  Display Server:        Wayland
  Desktop:               {}
  WAYLAND_DISPLAY:       {}
  XDG_SESSION_TYPE:      {}"#,
        info.desktop_environment
            .as_ref()
            .unwrap_or(&"unknown".to_string()),
        std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "not set".to_string()),
        std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "not set".to_string())
    )
}

/// Format error message for unknown display server
fn format_unknown_error(_info: &DisplayServerInfo) -> String {
    format!(
        r#"Error: Could not detect display server and no text injection tools found

Please install one of the following:

Universal option (works everywhere):
  ydotool:
    Ubuntu/Debian: sudo apt install ydotool
    Fedora:        sudo dnf install ydotool
    Arch:          sudo pacman -S ydotool
    Setup:         sudo usermod -aG input $USER

For X11:
  xdotool:
    Ubuntu/Debian: sudo apt install xdotool
    Fedora:        sudo dnf install xdotool
    Arch:          sudo pacman -S xdotool

For Wayland:
  wtype:
    Ubuntu/Debian: sudo apt install wtype
    Fedora:        sudo dnf install wtype
    Arch:          sudo pacman -S wtype

Detected environment:
  DISPLAY:              {}
  WAYLAND_DISPLAY:      {}
  XDG_SESSION_TYPE:     {}
  XDG_CURRENT_DESKTOP:  {}"#,
        std::env::var("DISPLAY").unwrap_or_else(|_| "not set".to_string()),
        std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "not set".to_string()),
        std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "not set".to_string()),
        std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "not set".to_string())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_command_names() {
        assert_eq!(TextInjectionTool::Xdotool.command(), "xdotool");
        assert_eq!(TextInjectionTool::Wtype.command(), "wtype");
        assert_eq!(TextInjectionTool::Ydotool.command(), "ydotool");
    }

    #[test]
    fn test_display_server_detection() {
        // This test just ensures detection doesn't panic
        let info = detect_display_server();
        println!("Detected: {:?}", info);
    }

    #[test]
    fn test_tool_detection() {
        let tools = detect_available_tools();
        println!("Available tools: {:?}", tools);
        // Should have at least one tool on any Linux system
    }
}
