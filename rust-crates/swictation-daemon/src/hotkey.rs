//! Global hotkey handling for cross-desktop compatibility
//!
//! Supports multiple backends:
//! - X11: Direct hotkey grabbing via global-hotkey crate
//! - Sway/Wayland: IPC-based integration (requires manual config)
//! - Windows/macOS: Via global-hotkey crate

use anyhow::{Context, Result};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState, hotkey::{HotKey, Code, Modifiers}};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::config::HotkeyConfig;
use crate::display_server::{detect_display_server as detect_display_server_base, DisplayServer as BaseDisplayServer};

/// Hotkey events
#[derive(Debug, Clone)]
pub enum HotkeyEvent {
    /// Toggle recording on/off
    Toggle,
    /// Push-to-talk pressed
    PushToTalkPressed,
    /// Push-to-talk released
    PushToTalkReleased,
}

/// Hotkey-specific display server types (extends base detection with Sway)
#[derive(Debug, Clone, Copy, PartialEq)]
enum HotkeyDisplayServer {
    X11,
    Sway,
    Wayland,
    Headless,
}

/// Detect display server for hotkey management
/// Uses shared detection module and adds Sway-specific logic
fn detect_hotkey_server() -> HotkeyDisplayServer {
    // Check for Sway specifically (wlroots-based compositor with IPC)
    if std::env::var("SWAYSOCK").is_ok() {
        return HotkeyDisplayServer::Sway;
    }

    // Use shared display server detection for X11/Wayland
    let base_info = detect_display_server_base();
    match base_info.server_type {
        BaseDisplayServer::X11 => HotkeyDisplayServer::X11,
        BaseDisplayServer::Wayland => HotkeyDisplayServer::Wayland,
        BaseDisplayServer::Unknown => HotkeyDisplayServer::Headless,
    }
}

/// Hotkey manager for global hotkey registration
pub struct HotkeyManager {
    backend: HotkeyBackend,
}

/// Backend-specific hotkey implementation
enum HotkeyBackend {
    /// X11/Windows/macOS using global-hotkey crate
    GlobalHotkey {
        manager: GlobalHotKeyManager,
        toggle_hotkey: HotKey,
        ptt_hotkey: HotKey,
        rx: mpsc::UnboundedReceiver<HotkeyEvent>,
    },
    /// Sway compositor (requires manual config)
    SwayIpc {
        rx: mpsc::UnboundedReceiver<HotkeyEvent>,
    },
}

impl HotkeyManager {
    /// Create new hotkey manager with configured hotkeys
    /// Returns None if hotkeys are not available on this system
    pub fn new(config: HotkeyConfig) -> Result<Option<Self>> {
        let display_server = detect_hotkey_server();
        info!("Detected display server: {:?}", display_server);

        match display_server {
            HotkeyDisplayServer::X11 => {
                info!("Using X11 hotkey backend (direct key grabbing)");
                Self::new_global_hotkey(config)
            }
            HotkeyDisplayServer::Sway => {
                info!("Using Sway IPC backend (requires manual config)");
                Self::new_sway_ipc(config)
            }
            HotkeyDisplayServer::Wayland => {
                warn!("Generic Wayland compositor detected");
                warn!("Global hotkeys not supported - compositor-specific integration required");
                warn!("Please configure hotkeys in your compositor to call:");
                warn!("  - Toggle: echo 'toggle' | nc -U /tmp/swictation.sock");
                warn!("  - PTT press: echo 'ptt_press' | nc -U /tmp/swictation.sock");
                warn!("  - PTT release: echo 'ptt_release' | nc -U /tmp/swictation.sock");
                Ok(None)
            }
            HotkeyDisplayServer::Headless => {
                warn!("No display server detected (headless mode)");
                warn!("Hotkeys disabled - use IPC/CLI for control");
                Ok(None)
            }
        }
    }

    /// Create X11/Windows/macOS backend using global-hotkey
    fn new_global_hotkey(config: HotkeyConfig) -> Result<Option<Self>> {
        // Try to create hotkey manager
        let manager = match GlobalHotKeyManager::new() {
            Ok(m) => m,
            Err(e) => {
                warn!("Failed to initialize global hotkey manager: {}", e);
                warn!("Hotkeys disabled - use IPC/CLI for control");
                return Ok(None);
            }
        };

        // Parse and register toggle hotkey
        let toggle_hotkey = parse_hotkey(&config.toggle)
            .context("Invalid toggle hotkey")?;
        let toggle_hotkey_clone = toggle_hotkey.clone();
        manager.register(toggle_hotkey)
            .context("Failed to register toggle hotkey")?;

        // Parse and register push-to-talk hotkey
        let ptt_hotkey = parse_hotkey(&config.push_to_talk)
            .context("Invalid push-to-talk hotkey")?;
        let ptt_hotkey_clone = ptt_hotkey.clone();
        manager.register(ptt_hotkey)
            .context("Failed to register push-to-talk hotkey")?;

        // Create event channel
        let (tx, rx) = mpsc::unbounded_channel();

        // Spawn hotkey event listener thread
        let toggle_id = toggle_hotkey_clone.id();
        let ptt_id = ptt_hotkey_clone.id();
        std::thread::spawn(move || {
            loop {
                if let Ok(event) = GlobalHotKeyEvent::receiver().recv() {
                    let hotkey_event = if event.id == toggle_id && event.state == HotKeyState::Pressed {
                        Some(HotkeyEvent::Toggle)
                    } else if event.id == ptt_id && event.state == HotKeyState::Pressed {
                        Some(HotkeyEvent::PushToTalkPressed)
                    } else if event.id == ptt_id && event.state == HotKeyState::Released {
                        Some(HotkeyEvent::PushToTalkReleased)
                    } else {
                        None
                    };

                    if let Some(event) = hotkey_event {
                        if tx.send(event).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(Some(Self {
            backend: HotkeyBackend::GlobalHotkey {
                manager,
                toggle_hotkey: toggle_hotkey_clone,
                ptt_hotkey: ptt_hotkey_clone,
                rx,
            },
        }))
    }

    /// Create Sway IPC backend
    ///
    /// Note: Sway does not support dynamic hotkey registration via IPC.
    /// We check if hotkeys exist in ~/.config/sway/config and auto-configure if needed.
    fn new_sway_ipc(_config: HotkeyConfig) -> Result<Option<Self>> {
        #[cfg(feature = "sway-integration")]
        {
            // Check if we can connect to Sway
            match swayipc::Connection::new() {
                Ok(_) => {
                    info!("✓ Connected to Sway compositor");

                    // Don't auto-configure - let users add hotkeys manually
                    info!("");
                    info!("To add hotkeys, edit ~/.config/sway/config:");
                    info!("  bindsym $mod+Shift+d exec sh -c \"echo 'toggle' | nc -U /tmp/swictation.sock\"");
                    info!("  (Choose your own non-conflicting keys)");
                    info!("");

                    // We don't actually listen for events via IPC since Sway will
                    // trigger our Unix socket directly. Return None to indicate
                    // that IPC/CLI is the only control method.
                    Ok(None)
                }
                Err(e) => {
                    warn!("Failed to connect to Sway: {}", e);
                    warn!("Make sure SWAYSOCK environment variable is set");
                    Ok(None)
                }
            }
        }

        #[cfg(not(feature = "sway-integration"))]
        {
            warn!("Sway detected but built with --no-default-features (minimal build)");
            warn!("Hotkeys disabled - use IPC/CLI for control");
            warn!("For full Sway support, rebuild with default features (recommended)");
            Ok(None)
        }
    }

    /// Check if Sway config has our hotkeys, add them if not, and reload Sway
    #[cfg(feature = "sway-integration")]
    fn configure_sway_hotkeys(_config: &HotkeyConfig) -> Result<()> {
        let sway_config_path = std::env::var("HOME")
            .map(|home| format!("{}/.config/sway/config", home))
            .context("HOME environment variable not set")?;

        // Read current config
        let config_content = std::fs::read_to_string(&sway_config_path)
            .context("Failed to read Sway config")?;

        // Check if our hotkeys already exist
        if config_content.contains("# Swictation voice-to-text hotkeys") {
            info!("✓ Swictation hotkeys already configured in Sway");
            return Ok(());
        }

        info!("Adding Swictation hotkeys to Sway config...");

        // Append hotkeys to config
        let hotkey_config = format!(
            r#"
# Swictation voice-to-text hotkeys
bindsym $mod+Shift+d exec sh -c "echo 'toggle' | nc -U /tmp/swictation.sock"
bindsym $mod+Space exec sh -c "echo 'ptt_press' | nc -U /tmp/swictation.sock"
bindsym --release $mod+Space exec sh -c "echo 'ptt_release' | nc -U /tmp/swictation.sock"
"#
        );

        std::fs::write(&sway_config_path, format!("{}{}", config_content, hotkey_config))
            .context("Failed to write Sway config")?;

        info!("✓ Hotkeys added to Sway config");
        info!("Reloading Sway...");

        // Reload Sway
        if let Ok(mut conn) = swayipc::Connection::new() {
            if let Err(e) = conn.run_command("reload") {
                warn!("Failed to reload Sway: {}", e);
                info!("Please run: swaymsg reload");
            } else {
                info!("✓ Sway reloaded successfully");
            }
        }

        Ok(())
    }

    /// Get next hotkey event (async)
    pub async fn next_event(&mut self) -> Option<HotkeyEvent> {
        match &mut self.backend {
            HotkeyBackend::GlobalHotkey { rx, .. } => rx.recv().await,
            HotkeyBackend::SwayIpc { rx } => rx.recv().await,
        }
    }
}

impl Drop for HotkeyManager {
    fn drop(&mut self) {
        if let HotkeyBackend::GlobalHotkey { manager, toggle_hotkey, ptt_hotkey, .. } = &self.backend {
            let _ = manager.unregister(toggle_hotkey.clone());
            let _ = manager.unregister(ptt_hotkey.clone());
        }
    }
}

/// Parse hotkey string like "Ctrl+Shift+R" into HotKey
fn parse_hotkey(s: &str) -> Result<HotKey> {
    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();

    if parts.is_empty() {
        anyhow::bail!("Empty hotkey string");
    }

    let mut modifiers = Modifiers::empty();
    let mut key_code: Option<Code> = None;

    for part in parts {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "shift" => modifiers |= Modifiers::SHIFT,
            "alt" => modifiers |= Modifiers::ALT,
            "super" | "win" | "cmd" | "meta" => modifiers |= Modifiers::SUPER,
            key => {
                key_code = Some(parse_key_code(key)?);
            }
        }
    }

    let key_code = key_code.context("No key code found in hotkey string")?;
    Ok(HotKey::new(Some(modifiers), key_code))
}

/// Parse key code from string
fn parse_key_code(s: &str) -> Result<Code> {
    let code = match s.to_lowercase().as_str() {
        // Letters
        "a" => Code::KeyA,
        "b" => Code::KeyB,
        "c" => Code::KeyC,
        "d" => Code::KeyD,
        "e" => Code::KeyE,
        "f" => Code::KeyF,
        "g" => Code::KeyG,
        "h" => Code::KeyH,
        "i" => Code::KeyI,
        "j" => Code::KeyJ,
        "k" => Code::KeyK,
        "l" => Code::KeyL,
        "m" => Code::KeyM,
        "n" => Code::KeyN,
        "o" => Code::KeyO,
        "p" => Code::KeyP,
        "q" => Code::KeyQ,
        "r" => Code::KeyR,
        "s" => Code::KeyS,
        "t" => Code::KeyT,
        "u" => Code::KeyU,
        "v" => Code::KeyV,
        "w" => Code::KeyW,
        "x" => Code::KeyX,
        "y" => Code::KeyY,
        "z" => Code::KeyZ,

        // Numbers
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,

        // Special keys
        "space" => Code::Space,
        "enter" | "return" => Code::Enter,
        "tab" => Code::Tab,
        "backspace" => Code::Backspace,
        "escape" | "esc" => Code::Escape,

        // Function keys
        "f1" => Code::F1,
        "f2" => Code::F2,
        "f3" => Code::F3,
        "f4" => Code::F4,
        "f5" => Code::F5,
        "f6" => Code::F6,
        "f7" => Code::F7,
        "f8" => Code::F8,
        "f9" => Code::F9,
        "f10" => Code::F10,
        "f11" => Code::F11,
        "f12" => Code::F12,

        _ => anyhow::bail!("Unknown key code: {}", s),
    };

    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hotkey() {
        let hotkey = parse_hotkey("Ctrl+Shift+R").unwrap();
        assert!(hotkey.mods.contains(Modifiers::CONTROL));
        assert!(hotkey.mods.contains(Modifiers::SHIFT));
        assert_eq!(hotkey.key, Code::KeyR);

        let hotkey = parse_hotkey("Alt+F4").unwrap();
        assert!(hotkey.mods.contains(Modifiers::ALT));
        assert_eq!(hotkey.key, Code::F4);

        let hotkey = parse_hotkey("Ctrl+Space").unwrap();
        assert!(hotkey.mods.contains(Modifiers::CONTROL));
        assert_eq!(hotkey.key, Code::Space);
    }

    #[test]
    fn test_parse_key_code() {
        assert_eq!(parse_key_code("r").unwrap(), Code::KeyR);
        assert_eq!(parse_key_code("space").unwrap(), Code::Space);
        assert_eq!(parse_key_code("f4").unwrap(), Code::F4);
        assert!(parse_key_code("invalid").is_err());
    }
}
