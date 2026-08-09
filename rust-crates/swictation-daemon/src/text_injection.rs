//! Cross-platform text injection for Linux (X11/Wayland) and macOS
//!
//! **Linux** - Supports three text injection tools:
//! - xdotool: X11 (fast, mature)
//! - wtype: Wayland compatible (KDE, Sway, Hyprland - NOT GNOME)
//! - ydotool: Universal (X11, all Wayland compositors including GNOME, even TTY)
//!
//! **macOS** - Uses Core Graphics Accessibility API:
//! - MacOSNative: Core Graphics framework (requires Accessibility permissions)
//!
//! # Injected text is always literal
//!
//! Everything that reaches [`TextInjector::inject_text`] is typed verbatim.
//! The injector deliberately interprets no escape or marker syntax, because
//! its input is transcribed speech plus the user's hot-reloadable correction
//! table — neither is a trusted source of commands. An earlier version parsed
//! `<KEY:...>` markers out of the text and synthesised the corresponding key
//! combination, which meant a dictated phrase (or a correction whose
//! replacement contained a marker) could press arbitrary key combinations in
//! whatever window had focus. Nothing in the pipeline ever produced such a
//! marker, so the parsing existed only as a way in.

use anyhow::{Context, Result};
use tracing::info;

// Linux-specific imports
#[cfg(target_os = "linux")]
use std::process::Command;
#[cfg(target_os = "linux")]
use tracing::debug;

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

    /// Inject text into the current window, verbatim
    ///
    /// No part of `text` is interpreted: see the module documentation for why
    /// key-combination markers are typed as characters rather than pressed.
    pub fn inject_text(&self, text: &str) -> Result<()> {
        // macOS: Delegate to macOS injector
        #[cfg(target_os = "macos")]
        {
            self.macos_injector.inject_text(text)
        }

        // Linux: Use command-line tools
        #[cfg(target_os = "linux")]
        {
            self.inject_plain_text(text)
        }
    }

    /// Inject text literally (Linux only)
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

    /// Regression: transcribed speech must never become keystrokes.
    ///
    /// The daemon feeds this STT output after user corrections have been
    /// applied, so a dictated "less than KEY colon ctrl dash a greater than"
    /// — or a correction rule whose replacement contains that text — used to
    /// be parsed out and pressed as a real key combination. These inputs must
    /// now travel the same literal path as any other text.
    #[test]
    fn test_key_markers_are_typed_not_pressed() {
        let Ok(injector) = TextInjector::new() else {
            println!("Skipping: no injector available in this environment");
            return;
        };

        for text in [
            "Hello, world!",
            "Press <KEY:ctrl-c> to copy",
            "<KEY:super-Right>",
            "Multiple <KEY:ctrl-a> keys <KEY:ctrl-v>",
            // Previously aborted injection with "Malformed KEY marker"
            "unterminated <KEY:ctrl-a",
        ] {
            assert!(
                injector.inject_text(text).is_ok(),
                "literal injection should succeed for {text:?}"
            );
        }
    }

    #[test]
    fn test_empty_text() {
        if let Ok(injector) = TextInjector::new() {
            // Empty text should not error
            assert!(injector.inject_text("").is_ok());
        }
    }
}
