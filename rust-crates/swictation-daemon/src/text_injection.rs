//! Cross-platform text injection for Linux (X11/Wayland) and macOS with keyboard shortcut support
//!
//! **Linux** - Supports three text injection tools:
//! - xdotool: X11 (fast, mature)
//! - wtype: Wayland compatible (KDE, Sway, Hyprland - NOT GNOME)
//! - ydotool: Universal (X11, all Wayland compositors including GNOME, even TTY)
//!
//! **macOS** - Uses Core Graphics Accessibility API:
//! - MacOSNative: Core Graphics framework (requires Accessibility permissions)
//!
//! This version properly handles <KEY:...> markers by sending actual key events

use anyhow::{Context, Result};
use std::process::Command;
use tracing::{debug, info};

use crate::display_server::{
    detect_available_tools, detect_display_server, select_best_tool, DisplayServerInfo,
    TextInjectionTool,
};

// macOS text injection module (conditional compilation)
#[cfg(target_os = "macos")]
use crate::macos_text_inject::MacOSTextInjector;

/// Text injector that works across platforms
pub struct TextInjector {
    /// Detected display server information
    display_server_info: DisplayServerInfo,
    /// Selected text injection tool
    selected_tool: TextInjectionTool,
    /// macOS text injector (only on macOS)
    #[cfg(target_os = "macos")]
    macos_injector: MacOSTextInjector,
}

impl TextInjector {
    /// Create a new text injector with auto-detection
    pub fn new() -> Result<Self> {
        // Detect display server
        let display_server_info = detect_display_server();

        // Detect available tools
        let available_tools = detect_available_tools();

        if available_tools.is_empty() {
            anyhow::bail!(
                "No text injection tools found. Please install xdotool, wtype, or ydotool"
            );
        }

        // Select best tool for this environment
        let selected_tool = select_best_tool(&display_server_info, &available_tools)?;

        info!(
            "Using {} for text injection ({:?})",
            selected_tool.name(),
            display_server_info.server_type
        );

        if display_server_info.is_gnome_wayland {
            info!("GNOME Wayland detected - using ydotool (wtype not compatible)");
        }

        // Create macOS injector if on macOS
        #[cfg(target_os = "macos")]
        let macos_injector =
            MacOSTextInjector::new().context("Failed to create macOS text injector")?;

        Ok(Self {
            display_server_info,
            selected_tool,
            #[cfg(target_os = "macos")]
            macos_injector,
        })
    }

    /// Inject text into the current window, handling <KEY:...> markers
    pub fn inject_text(&self, text: &str) -> Result<()> {
        // macOS: Delegate to macOS injector
        #[cfg(target_os = "macos")]
        {
            return self.macos_injector.inject_text(text);
        }

        // Linux: Use command-line tools
        #[cfg(target_os = "linux")]
        {
            // Check if text contains keyboard shortcut markers
            if text.contains("<KEY:") {
                self.inject_with_keys(text)
            } else {
                // Plain text injection
                self.inject_plain_text(text)
            }
        }
    }

    /// Process text with <KEY:...> markers (Linux only)
    #[cfg(target_os = "linux")]
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

    /// Send a key combination (e.g., "super-Right", "ctrl-c") (Linux only)
    #[cfg(target_os = "linux")]
    fn send_key_combination(&self, combo: &str) -> Result<()> {
        match self.selected_tool {
            TextInjectionTool::Xdotool => self.send_xdotool_keys(combo),
            TextInjectionTool::Wtype => self.send_wtype_keys(combo),
            TextInjectionTool::Ydotool => self.send_ydotool_keys(combo),
            TextInjectionTool::MacOSNative => {
                // This should never happen on Linux, but we need the pattern for compilation
                anyhow::bail!("macOS text injection not available on Linux")
            }
        }
    }

    /// Send key combination using xdotool on X11 (Linux only)
    #[cfg(target_os = "linux")]
    fn send_xdotool_keys(&self, combo: &str) -> Result<()> {
        // Convert to xdotool format (e.g., "super-Right" -> "super+Right")
        let xdo_combo = combo.replace('-', "+");

        debug!("xdotool key: {}", xdo_combo);

        let output = Command::new("xdotool")
            .arg("key")
            .arg(&xdo_combo)
            .output()
            .context(format!("Failed to send key combination: {}", combo))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("xdotool key command failed: {}", stderr);
        }

        Ok(())
    }

    /// Send key combination using wtype on Wayland (Linux only)
    #[cfg(target_os = "linux")]
    fn send_wtype_keys(&self, combo: &str) -> Result<()> {
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

        debug!("wtype command: {:?}", cmd);

        // Release modifiers (automatic when wtype exits)
        let output = cmd
            .output()
            .context(format!("Failed to send key combination: {}", combo))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("wtype key command failed: {}", stderr);
        }

        Ok(())
    }

    /// Send key combination using ydotool (universal) (Linux only)
    #[cfg(target_os = "linux")]
    fn send_ydotool_keys(&self, combo: &str) -> Result<()> {
        // ydotool key command uses key codes
        // For simplicity, we'll use the same format as xdotool (modifier+key)
        // and let ydotool parse it
        let yd_combo = combo.replace('-', "+");

        debug!("ydotool key: {}", yd_combo);

        let output = Command::new("ydotool")
            .arg("key")
            .arg(&yd_combo)
            .output()
            .context(format!("Failed to send key combination: {}", combo))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Check for permission errors
            if stderr.contains("Permission denied") || stderr.contains("input group") {
                anyhow::bail!(
                    "ydotool permission denied. Add user to input group:\n  \
                    sudo usermod -aG input $USER\n  \
                    Then log out and back in.\n\n\
                    Error: {}",
                    stderr
                );
            }

            anyhow::bail!("ydotool key command failed: {}", stderr);
        }

        Ok(())
    }

    /// Inject plain text (no key markers) (Linux only)
    #[cfg(target_os = "linux")]
    fn inject_plain_text(&self, text: &str) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        match self.selected_tool {
            TextInjectionTool::Xdotool => self.inject_xdotool_text(text),
            TextInjectionTool::Wtype => self.inject_wtype_text(text),
            TextInjectionTool::Ydotool => self.inject_ydotool_text(text),
            TextInjectionTool::MacOSNative => {
                // This should never happen on Linux, but we need the pattern for compilation
                anyhow::bail!("macOS text injection not available on Linux")
            }
        }
    }

    /// Inject text using xdotool (X11) (Linux only)
    #[cfg(target_os = "linux")]
    fn inject_xdotool_text(&self, text: &str) -> Result<()> {
        debug!("xdotool type: {} chars", text.len());

        let output = Command::new("xdotool")
            .arg("type")
            .arg("--clearmodifiers")
            .arg("--")
            .arg(text)
            .output()
            .context("Failed to inject text with xdotool")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("xdotool type command failed: {}", stderr);
        }

        Ok(())
    }

    /// Inject text using wtype (Wayland) (Linux only)
    #[cfg(target_os = "linux")]
    fn inject_wtype_text(&self, text: &str) -> Result<()> {
        debug!("wtype: {} chars", text.len());

        let output = Command::new("wtype")
            .arg("--")
            .arg(text)
            .output()
            .context("Failed to inject text with wtype")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("wtype command failed: {}", stderr);
        }

        Ok(())
    }

    /// Inject text using ydotool (universal - works on X11, Wayland, TTY) (Linux only)
    #[cfg(target_os = "linux")]
    fn inject_ydotool_text(&self, text: &str) -> Result<()> {
        debug!("ydotool type: {} chars", text.len());

        let output = Command::new("ydotool")
            .arg("type")
            .arg("--")
            .arg(text)
            .output()
            .context("Failed to inject text with ydotool")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Check for permission errors
            if stderr.contains("Permission denied") || stderr.contains("input group") {
                anyhow::bail!(
                    "ydotool permission denied. Add user to input group:\n  \
                    sudo usermod -aG input $USER\n  \
                    Then log out and back in.\n\n\
                    Error: {}",
                    stderr
                );
            }

            anyhow::bail!("ydotool type command failed: {}", stderr);
        }

        Ok(())
    }

    /// Get the detected display server information
    pub fn display_server_info(&self) -> &DisplayServerInfo {
        &self.display_server_info
    }

    /// Get the selected text injection tool
    #[allow(dead_code)]
    pub fn selected_tool(&self) -> TextInjectionTool {
        self.selected_tool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_injector_creation() {
        // Should not panic during detection
        let result = TextInjector::new();

        if let Ok(injector) = result {
            println!(
                "Created text injector: {:?} using {}",
                injector.display_server_info().server_type,
                injector.selected_tool().name()
            );
        } else if let Err(e) = result {
            println!(
                "Text injector creation failed (expected if no tools installed): {}",
                e
            );
        }
    }

    #[test]
    fn test_key_marker_parsing() {
        // Only test if we can create an injector
        if let Ok(injector) = TextInjector::new() {
            // Test that it doesn't panic on various inputs
            let _ = injector.inject_text("Hello, world!");
            let _ = injector.inject_text("Press <KEY:ctrl-c> to copy");
            let _ = injector.inject_text("<KEY:super-Right>");
            let _ = injector.inject_text("Multiple <KEY:ctrl-a> keys <KEY:ctrl-v>");
        }
    }

    #[test]
    fn test_empty_text() {
        if let Ok(injector) = TextInjector::new() {
            // Empty text should not error
            assert!(injector.inject_plain_text("").is_ok());
        }
    }
}
