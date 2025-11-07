//! Global hotkey handling for cross-desktop compatibility

use anyhow::{Context, Result};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState, hotkey::{HotKey, Code, Modifiers}};
use tokio::sync::mpsc;

use crate::config::HotkeyConfig;

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

/// Hotkey manager for global hotkey registration
pub struct HotkeyManager {
    manager: GlobalHotKeyManager,
    toggle_hotkey: HotKey,
    ptt_hotkey: HotKey,
    rx: mpsc::UnboundedReceiver<HotkeyEvent>,
}

impl HotkeyManager {
    /// Create new hotkey manager with configured hotkeys
    pub fn new(config: HotkeyConfig) -> Result<Self> {
        let manager = GlobalHotKeyManager::new()
            .context("Failed to create global hotkey manager")?;

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

        Ok(Self {
            manager,
            toggle_hotkey: toggle_hotkey_clone,
            ptt_hotkey: ptt_hotkey_clone,
            rx,
        })
    }

    /// Get next hotkey event (async)
    pub async fn next_event(&mut self) -> Option<HotkeyEvent> {
        self.rx.recv().await
    }
}

impl Drop for HotkeyManager {
    fn drop(&mut self) {
        let _ = self.manager.unregister(self.toggle_hotkey.clone());
        let _ = self.manager.unregister(self.ptt_hotkey.clone());
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
