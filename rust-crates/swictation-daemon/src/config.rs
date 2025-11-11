//! Configuration management

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Hotkey configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    /// Toggle hotkey (default: "Super+Shift+D" for Dictation)
    /// User-configurable via UI settings
    pub toggle: String,

    /// Push-to-talk hotkey (default: "Super+Space")
    /// User-configurable via UI settings
    pub push_to_talk: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            toggle: "Super+Shift+D".to_string(),  // Windows/Super key + Shift + D (Dictation)
            push_to_talk: "Super+Space".to_string(),  // Windows/Super key + Space
        }
    }
}

/// Daemon configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Path to configuration file
    #[serde(skip)]
    pub config_path: PathBuf,

    /// Unix socket path for IPC
    pub socket_path: String,

    /// VAD model path
    pub vad_model_path: String,

    /// VAD minimum silence duration (seconds)
    pub vad_min_silence: f32,

    /// VAD minimum speech duration (seconds)
    pub vad_min_speech: f32,

    /// VAD maximum speech duration (seconds)
    pub vad_max_speech: f32,

    /// VAD threshold (ONNX: 0.001-0.005, NOT PyTorch 0.5!)
    /// See swictation-vad/ONNX_THRESHOLD_GUIDE.md for details
    pub vad_threshold: f32,

    /// STT model selection override
    /// Options: "auto" (VRAM-based), "0.6b-cpu", "0.6b-gpu", "1.1b-gpu"
    pub stt_model_override: String,

    /// Path to 0.6B model directory (sherpa-rs)
    pub stt_0_6b_model_path: String,

    /// Path to 1.1B INT8 model directory (ONNX Runtime)
    pub stt_1_1b_model_path: String,

    /// Number of threads for ONNX Runtime
    pub num_threads: Option<i32>,

    /// Audio device index (None = default device)
    pub audio_device_index: Option<usize>,

    /// Hotkey configuration
    pub hotkeys: HotkeyConfig,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            config_path: Self::default_config_path(),
            socket_path: "/tmp/swictation.sock".to_string(),
            vad_model_path: "/opt/swictation/models/silero-vad/silero_vad.onnx".to_string(),
            vad_min_silence: 0.5,
            vad_min_speech: 0.25,
            vad_max_speech: 30.0,
            vad_threshold: 0.003, // ONNX model threshold (100-200x lower than PyTorch 0.5)
            // STT adaptive model selection (auto = VRAM-based)
            stt_model_override: "auto".to_string(),
            stt_0_6b_model_path: "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx".to_string(),
            stt_1_1b_model_path: "/opt/swictation/models/parakeet-tdt-1.1b-onnx".to_string(),
            num_threads: Some(4),
            audio_device_index: None, // Will be set from env var or auto-detected
            hotkeys: HotkeyConfig::default(),
        }
    }
}

impl DaemonConfig {
    /// Load configuration from file, or create default
    pub fn load() -> Result<Self> {
        let config_path = Self::default_config_path();

        if config_path.exists() {
            // Load existing config
            let contents = std::fs::read_to_string(&config_path)
                .context("Failed to read config file")?;

            let mut config: DaemonConfig = toml::from_str(&contents)
                .context("Failed to parse config file")?;

            config.config_path = config_path;
            Ok(config)
        } else {
            // Create default config
            let config = Self::default();
            config.save()
                .context("Failed to save default config")?;
            Ok(config)
        }
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        // Ensure config directory exists
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }

        let contents = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        std::fs::write(&self.config_path, contents)
            .context("Failed to write config file")?;

        Ok(())
    }

    /// Get default config path
    fn default_config_path() -> PathBuf {
        let config_dir = if cfg!(target_os = "windows") {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("Swictation")
        } else if cfg!(target_os = "macos") {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("com.swictation.daemon")
        } else {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("swictation")
        };

        config_dir.join("config.toml")
    }
}
