//! Cross-platform text injection for Linux (X11/Wayland)

use anyhow::{Context, Result};
use std::process::Command;

/// Display server type
#[derive(Debug, Clone)]
pub enum DisplayServer {
    X11,
    Wayland,
    Unknown,
}

/// Text injector that works across X11 and Wayland
pub struct TextInjector {
    display_server: DisplayServer,
}

impl TextInjector {
    /// Create a new text injector with auto-detection
    pub fn new() -> Result<Self> {
        let display_server = Self::detect_display_server();

        // Verify required tools are installed
        match &display_server {
            DisplayServer::X11 => {
                Command::new("which")
                    .arg("xdotool")
                    .output()
                    .context("xdotool not found. Install with: sudo apt install xdotool")?;
            }
            DisplayServer::Wayland => {
                Command::new("which")
                    .arg("wtype")
                    .output()
                    .context("wtype not found. Install with: sudo apt install wtype")?;
            }
            DisplayServer::Unknown => {
                eprintln!("Warning: Could not detect display server");
            }
        }

        Ok(Self { display_server })
    }

    /// Detect the current display server
    fn detect_display_server() -> DisplayServer {
        // Check for Wayland
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            return DisplayServer::Wayland;
        }

        // Check for X11
        if std::env::var("DISPLAY").is_ok() {
            // Double-check it's not XWayland
            if std::env::var("XDG_SESSION_TYPE")
                .map(|t| t == "x11")
                .unwrap_or(false)
            {
                return DisplayServer::X11;
            }
        }

        // Check XDG_SESSION_TYPE as fallback
        match std::env::var("XDG_SESSION_TYPE").as_deref() {
            Ok("wayland") => DisplayServer::Wayland,
            Ok("x11") => DisplayServer::X11,
            _ => DisplayServer::Unknown,
        }
    }

    /// Inject text into the current window
    pub fn inject_text(&self, text: &str) -> Result<()> {
        match self.display_server {
            DisplayServer::X11 => self.inject_x11(text),
            DisplayServer::Wayland => self.inject_wayland(text),
            DisplayServer::Unknown => {
                // Try both as fallback
                if let Ok(_) = self.inject_wayland(text) {
                    return Ok(());
                }
                self.inject_x11(text)
            }
        }
    }

    /// Inject text using X11 tools
    fn inject_x11(&self, text: &str) -> Result<()> {
        // xdotool type escapes special characters properly
        Command::new("xdotool")
            .arg("type")
            .arg("--clearmodifiers")
            .arg("--")
            .arg(text)
            .output()
            .context("Failed to inject text with xdotool")?;

        Ok(())
    }

    /// Inject text using Wayland tools
    fn inject_wayland(&self, text: &str) -> Result<()> {
        // wtype handles text input on Wayland
        Command::new("wtype")
            .arg(text)
            .output()
            .context("Failed to inject text with wtype")?;

        Ok(())
    }

    /// Get the detected display server type
    pub fn display_server(&self) -> &DisplayServer {
        &self.display_server
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_server_detection() {
        let injector = TextInjector::new();
        if injector.is_ok() {
            let injector = injector.unwrap();
            println!("Detected display server: {:?}", injector.display_server());
        }
    }
}