//! Configuration management

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

use crate::socket_utils;

/// Get default model directory using platform-appropriate paths via swictation_paths.
/// On macOS: ~/Library/Application Support/swictation/models/
/// On Linux: $XDG_DATA_HOME/swictation/models/ or ~/.local/share/swictation/models/
/// Can be overridden with SWICTATION_MODEL_PATH environment variable
fn get_default_model_dir() -> PathBuf {
    env::var("SWICTATION_MODEL_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| swictation_paths::models_dir())
}

/// Get default path for 0.6B model
fn get_default_0_6b_model_path() -> PathBuf {
    get_default_model_dir().join("parakeet-tdt-0.6b-v3-onnx")
}

/// Get default path for 1.1B model
fn get_default_1_1b_model_path() -> PathBuf {
    get_default_model_dir().join("parakeet-tdt-1.1b-onnx")
}

/// Get default path for CoreML model (macOS native)
fn get_default_coreml_model_path() -> PathBuf {
    get_default_model_dir().join("parakeet-tdt-1.1b-coreml")
}

/// Get default path for VAD model
fn get_default_vad_model_path() -> PathBuf {
    get_default_model_dir()
        .join("silero-vad")
        .join("silero_vad.onnx")
}

/// Hotkey configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HotkeyConfig {
    /// Toggle hotkey (default: "Ctrl+Shift+D" on macOS, "Super+Shift+D" on Linux)
    /// User-configurable via UI settings
    pub toggle: String,

    /// Push-to-talk hotkey (default: "Ctrl+Space" on macOS, "Super+Space" on Linux)
    /// User-configurable via UI settings
    pub push_to_talk: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        // On macOS, "Super" maps to Command key (⌘)
        // Use platform-appropriate naming for user-facing config
        let (toggle, ptt) = if cfg!(target_os = "macos") {
            ("Ctrl+Shift+D".to_string(), "Ctrl+Space".to_string())
        } else {
            ("Super+Shift+D".to_string(), "Super+Space".to_string())
        };

        Self {
            toggle,
            push_to_talk: ptt,
        }
    }
}

/// Default socket path from platform-appropriate directory (NEVER /tmp)
fn default_socket_path() -> String {
    socket_utils::get_ipc_socket_path()
        .expect(
            "Failed to determine IPC socket path - cannot proceed without valid socket directory",
        )
        .to_string_lossy()
        .to_string()
}

// Scalar defaults, shared by serde field defaults and `Default` (ADR-034)
fn default_vad_min_silence() -> f32 {
    0.8
}
fn default_vad_min_speech() -> f32 {
    0.25
}
fn default_vad_max_speech() -> f32 {
    30.0
}
// Optimized for real-time transcription (original 0.003 prevented silence detection)
fn default_vad_threshold() -> f32 {
    0.25
}
// STT adaptive model selection (auto = VRAM-based)
fn default_stt_model_override() -> String {
    "auto".to_string()
}
fn default_num_threads() -> Option<i32> {
    Some(4)
}
// Moderate fuzzy matching
fn default_phonetic_threshold() -> f64 {
    0.3
}

/// Daemon configuration
///
/// Every key in config.toml is optional: each field carries a serde default,
/// so the shipped config.example.toml (which comments out most keys) loads
/// verbatim. Defaults are FIELD-level, not container-level, deliberately —
/// the socket/model-path defaults are fallible and touch the filesystem, and
/// they must only run when the corresponding key is actually absent (ADR-034).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Path to configuration file
    #[serde(skip)]
    pub config_path: PathBuf,

    /// Unix socket path for IPC
    #[serde(default = "default_socket_path")]
    pub socket_path: String,

    /// VAD model path
    #[serde(default = "get_default_vad_model_path")]
    pub vad_model_path: PathBuf,

    /// VAD minimum silence duration (seconds)
    #[serde(default = "default_vad_min_silence")]
    pub vad_min_silence: f32,

    /// VAD minimum speech duration (seconds)
    #[serde(default = "default_vad_min_speech")]
    pub vad_min_speech: f32,

    /// VAD maximum speech duration (seconds)
    #[serde(default = "default_vad_max_speech")]
    pub vad_max_speech: f32,

    /// VAD threshold (ONNX: 0.001-0.005, NOT PyTorch 0.5!)
    /// See swictation-vad/ONNX_THRESHOLD_GUIDE.md for details
    #[serde(default = "default_vad_threshold")]
    pub vad_threshold: f32,

    /// STT model selection override
    /// Options: "auto" (platform-adaptive), "0.6b-cpu", "0.6b-gpu", "1.1b-gpu"
    /// macOS + coreml-native feature: also "1.1b-coreml" (alias: "coreml-native")
    #[serde(default = "default_stt_model_override")]
    pub stt_model_override: String,

    /// Path to 0.6B model directory (OrtRecognizer)
    #[serde(default = "get_default_0_6b_model_path")]
    pub stt_0_6b_model_path: PathBuf,

    /// Path to 1.1B INT8 model directory (ONNX Runtime)
    #[serde(default = "get_default_1_1b_model_path")]
    pub stt_1_1b_model_path: PathBuf,

    /// Path to CoreML model directory (macOS native, optional)
    #[serde(default = "get_default_coreml_model_path")]
    pub stt_coreml_model_path: PathBuf,

    /// Number of threads for ONNX Runtime
    #[serde(default = "default_num_threads")]
    pub num_threads: Option<i32>,

    /// Audio device index (None = default device)
    #[serde(default)]
    pub audio_device_index: Option<usize>,

    /// Hotkey configuration
    #[serde(default)]
    pub hotkeys: HotkeyConfig,

    /// Phonetic matching threshold for learned corrections (0.0 - 1.0)
    /// Lower = more strict, Higher = more fuzzy
    /// Default: 0.3
    #[serde(default = "default_phonetic_threshold")]
    pub phonetic_threshold: f64,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            config_path: Self::default_config_path(),
            socket_path: default_socket_path(),
            vad_model_path: get_default_vad_model_path(),
            vad_min_silence: default_vad_min_silence(),
            vad_min_speech: default_vad_min_speech(),
            vad_max_speech: default_vad_max_speech(),
            vad_threshold: default_vad_threshold(),
            stt_model_override: default_stt_model_override(),
            stt_0_6b_model_path: get_default_0_6b_model_path(),
            stt_1_1b_model_path: get_default_1_1b_model_path(),
            stt_coreml_model_path: get_default_coreml_model_path(),
            num_threads: default_num_threads(),
            audio_device_index: None, // Will be set from env var or auto-detected
            hotkeys: HotkeyConfig::default(),
            phonetic_threshold: default_phonetic_threshold(),
        }
    }
}

impl DaemonConfig {
    /// Load configuration from file, or create default
    pub fn load() -> Result<Self> {
        let config_path = Self::default_config_path();

        if config_path.exists() {
            // Load existing config
            let contents =
                std::fs::read_to_string(&config_path).context("Failed to read config file")?;

            let mut config: DaemonConfig =
                toml::from_str(&contents).context("Failed to parse config file")?;

            config.config_path = config_path;
            config.expand_tilde_paths();
            Ok(config)
        } else {
            // Create default config (expand tilde first: SWICTATION_MODEL_PATH
            // may itself contain `~`, and the saved config must hold real paths)
            let mut config = Self::default();
            config.expand_tilde_paths();
            config.save().context("Failed to save default config")?;
            Ok(config)
        }
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        // Ensure config directory exists
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        let contents = toml::to_string_pretty(self).context("Failed to serialize config")?;

        std::fs::write(&self.config_path, contents).context("Failed to write config file")?;

        Ok(())
    }

    /// Get default config path using swictation_paths as single source of truth
    fn default_config_path() -> PathBuf {
        swictation_paths::config_dir().join("config.toml")
    }

    /// Expand a leading `~` in every user-suppliable path field (ADR-034).
    /// PathBuf::from("~/...") resolves to a literal `./~` directory otherwise.
    fn expand_tilde_paths(&mut self) {
        self.socket_path = expand_tilde_str(&self.socket_path);
        self.vad_model_path = expand_tilde(&self.vad_model_path);
        self.stt_0_6b_model_path = expand_tilde(&self.stt_0_6b_model_path);
        self.stt_1_1b_model_path = expand_tilde(&self.stt_1_1b_model_path);
        self.stt_coreml_model_path = expand_tilde(&self.stt_coreml_model_path);
    }
}

/// Expand a leading `~` or `~/` to the user's home directory. Paths without a
/// leading tilde (and bare-`~user` forms, which we don't support) pass through.
fn expand_tilde(path: &std::path::Path) -> PathBuf {
    let Some(s) = path.to_str() else {
        return path.to_path_buf();
    };
    PathBuf::from(expand_tilde_str(s))
}

fn expand_tilde_str(s: &str) -> String {
    if s == "~" {
        if let Some(home) = dirs::home_dir() {
            return home.to_string_lossy().into_owned();
        }
    } else if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest).to_string_lossy().into_owned();
        }
    }
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shipped example config must parse verbatim — this is the drift guard
    /// for ADR-034 finding 1 (example config previously failed with
    /// "missing field `socket_path`").
    #[test]
    fn example_config_parses_verbatim() {
        let example = include_str!("../../../config/config.example.toml");
        let config: DaemonConfig =
            toml::from_str(example).expect("config.example.toml must parse with all defaults");
        // Spot-check a value the example sets explicitly.
        assert!(config.vad_min_silence > 0.0);
    }

    /// An empty config file is valid: every field falls back to Default.
    #[test]
    fn empty_config_parses_to_defaults() {
        let config: DaemonConfig = toml::from_str("").expect("empty config must parse");
        let defaults = DaemonConfig::default();
        assert_eq!(config.socket_path, defaults.socket_path);
        assert_eq!(config.stt_model_override, defaults.stt_model_override);
        assert_eq!(config.hotkeys.toggle, defaults.hotkeys.toggle);
    }

    /// A partial config keeps user values and defaults the rest.
    #[test]
    fn partial_config_keeps_user_values() {
        let config: DaemonConfig =
            toml::from_str("vad_threshold = 0.5\n[hotkeys]\ntoggle = \"Ctrl+Alt+Z\"\n")
                .expect("partial config must parse");
        assert_eq!(config.vad_threshold, 0.5);
        assert_eq!(config.hotkeys.toggle, "Ctrl+Alt+Z");
        assert_eq!(
            config.stt_model_override,
            DaemonConfig::default().stt_model_override
        );
    }

    #[test]
    fn tilde_paths_expand_to_home() {
        let mut config = DaemonConfig {
            socket_path: "~/sock/s.sock".to_string(),
            vad_model_path: PathBuf::from("~/models/vad.onnx"),
            ..Default::default()
        };
        config.expand_tilde_paths();
        let home = dirs::home_dir().expect("home dir required for test");
        assert_eq!(
            config.socket_path,
            home.join("sock/s.sock").to_string_lossy()
        );
        assert_eq!(config.vad_model_path, home.join("models/vad.onnx"));
    }

    #[test]
    fn non_tilde_paths_pass_through_unchanged() {
        let mut config = DaemonConfig {
            socket_path: "/run/user/1000/s.sock".to_string(),
            ..Default::default()
        };
        config.expand_tilde_paths();
        assert_eq!(config.socket_path, "/run/user/1000/s.sock");
    }
}
