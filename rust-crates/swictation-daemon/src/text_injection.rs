//! Cross-platform text injection for Linux (X11/Wayland) with keyboard shortcut support
//!
//! This version properly handles <KEY:...> markers by sending actual key events

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

    /// Inject text into the current window, handling <KEY:...> markers
    pub fn inject_text(&self, text: &str) -> Result<()> {
        // Check if text contains keyboard shortcut markers
        if text.contains("<KEY:") {
            self.inject_with_keys(text)
        } else {
            // Plain text injection
            match self.display_server {
                DisplayServer::X11 => self.inject_x11_text(text),
                DisplayServer::Wayland => self.inject_wayland_text(text),
                DisplayServer::Unknown => {
                    // Try both as fallback
                    if let Ok(_) = self.inject_wayland_text(text) {
                        return Ok(());
                    }
                    self.inject_x11_text(text)
                }
            }
        }
    }

    /// Process text with <KEY:...> markers
    fn inject_with_keys(&self, text: &str) -> Result<()> {
        let mut remaining = text;

        while !remaining.is_empty() {
            if let Some(key_start) = remaining.find("<KEY:") {
                // Inject any text before the key marker
                if key_start > 0 {
                    let text_part = &remaining[..key_start];
                    self.inject_plain_text(text_part)?;
                }

                // Find the end of the key marker
                let key_end = remaining[key_start..]
                    .find('>')
                    .context("Malformed KEY marker")?;

                // Extract the key combination
                let key_combo = &remaining[key_start + 5..key_start + key_end];

                // Send the key combination
                self.send_key_combination(key_combo)?;

                // Move past this marker
                remaining = &remaining[key_start + key_end + 1..];
            } else {
                // No more markers, inject remaining text
                self.inject_plain_text(remaining)?;
                break;
            }
        }

        Ok(())
    }

    /// Send a key combination (e.g., "super-Right", "ctrl-c")
    fn send_key_combination(&self, combo: &str) -> Result<()> {
        match self.display_server {
            DisplayServer::Wayland => self.send_wayland_keys(combo),
            DisplayServer::X11 => self.send_x11_keys(combo),
            DisplayServer::Unknown => {
                if let Ok(_) = self.send_wayland_keys(combo) {
                    return Ok(());
                }
                self.send_x11_keys(combo)
            }
        }
    }

    /// Send key combination using wtype on Wayland
    fn send_wayland_keys(&self, combo: &str) -> Result<()> {
        // Parse the key combination
        let parts: Vec<&str> = combo.split('-').collect();

        let mut cmd = Command::new("wtype");

        // Add modifiers
        for part in &parts[..parts.len() - 1] {
            let modifier = match part.to_lowercase().as_str() {
                "super" | "mod4" => "logo",
                "ctrl" | "control" => "ctrl",
                "alt" => "alt",
                "shift" => "shift",
                _ => continue,
            };
            cmd.arg("-M").arg(modifier);
        }

        // Add the key
        if let Some(key) = parts.last() {
            cmd.arg("-k").arg(key);
        }

        // Release modifiers (automatic when wtype exits)
        cmd.output()
            .context(format!("Failed to send key combination: {}", combo))?;

        Ok(())
    }

    /// Send key combination using xdotool on X11
    fn send_x11_keys(&self, combo: &str) -> Result<()> {
        // Convert to xdotool format (e.g., "super-Right" -> "super+Right")
        let xdo_combo = combo.replace('-', "+");

        Command::new("xdotool")
            .arg("key")
            .arg(xdo_combo)
            .output()
            .context(format!("Failed to send key combination: {}", combo))?;

        Ok(())
    }

    /// Inject plain text (no key markers)
    fn inject_plain_text(&self, text: &str) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        match self.display_server {
            DisplayServer::X11 => self.inject_x11_text(text),
            DisplayServer::Wayland => self.inject_wayland_text(text),
            DisplayServer::Unknown => {
                if let Ok(_) = self.inject_wayland_text(text) {
                    return Ok(());
                }
                self.inject_x11_text(text)
            }
        }
    }

    /// Inject text using X11 tools
    fn inject_x11_text(&self, text: &str) -> Result<()> {
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
    fn inject_wayland_text(&self, text: &str) -> Result<()> {
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

    #[test]
    fn test_key_marker_parsing() {
        let injector = TextInjector::new().unwrap();

        // Test that it doesn't panic on various inputs
        let _ = injector.inject_text("Hello, world!");
        let _ = injector.inject_text("Press <KEY:ctrl-c> to copy");
        let _ = injector.inject_text("<KEY:super-Right>");
        let _ = injector.inject_text("Multiple <KEY:ctrl-a> keys <KEY:ctrl-v>");
    }
}