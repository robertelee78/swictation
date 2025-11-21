//! Configuration management commands

use std::sync::Mutex;
use tauri::State;

/// State for daemon config path
pub struct ConfigState {
    pub config_path: Mutex<std::path::PathBuf>,
}

/// Configuration structure matching daemon config
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DaemonConfig {
    pub socket_path: String,
    pub vad_model_path: std::path::PathBuf,
    pub vad_min_silence: f32,
    pub vad_min_speech: f32,
    pub vad_max_speech: f32,
    pub vad_threshold: f32,
    pub stt_model_override: String,
    pub stt_0_6b_model_path: std::path::PathBuf,
    pub stt_1_1b_model_path: std::path::PathBuf,
    pub num_threads: Option<i32>,
    pub audio_device_index: Option<usize>,
    pub hotkeys: HotkeyConfig,
    pub phonetic_threshold: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HotkeyConfig {
    pub toggle: String,
    pub push_to_talk: String,
}

/// Get daemon configuration
#[tauri::command]
pub async fn get_daemon_config(
    state: State<'_, ConfigState>,
) -> Result<DaemonConfig, String> {
    let config_path = state.config_path.lock().unwrap();

    if !config_path.exists() {
        return Err("Config file not found".to_string());
    }

    let contents = std::fs::read_to_string(config_path.as_path())
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    let config: DaemonConfig = toml::from_str(&contents)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    Ok(config)
}

/// Update daemon configuration
#[tauri::command]
pub async fn update_daemon_config(
    state: State<'_, ConfigState>,
    config: DaemonConfig,
) -> Result<(), String> {
    let config_path = state.config_path.lock().unwrap();

    // Ensure parent directory exists
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let contents = toml::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    std::fs::write(config_path.as_path(), contents)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    Ok(())
}

/// Update only phonetic threshold (convenience method)
#[tauri::command]
pub async fn update_phonetic_threshold(
    state: State<'_, ConfigState>,
    threshold: f64,
) -> Result<(), String> {
    // Validate threshold
    if !(0.0..=1.0).contains(&threshold) {
        return Err("Threshold must be between 0.0 and 1.0".to_string());
    }

    // Load current config
    let mut config = get_daemon_config(state.clone()).await?;

    // Update threshold
    config.phonetic_threshold = threshold;

    // Save back
    update_daemon_config(state, config).await
}
