//! Upgrade-safe configuration repair. Parseable user text is preserved.
use crate::paths::{secure_dir, Paths};
use anyhow::{bail, Context, Result};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn socket(paths: &Paths) -> PathBuf {
    if cfg!(target_os = "linux") {
        if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute() && p.is_dir())
        {
            return runtime.join("swictation.sock");
        }
    }
    paths.data.join("swictation.sock")
}

pub fn configured_socket(paths: &Paths) -> Result<PathBuf> {
    let raw = match fs::read_to_string(paths.config_file()) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(socket(paths)),
        Err(error) => return Err(error).context("cannot read daemon configuration"),
    };
    let config: toml::Value = toml::from_str(&raw).context("config.toml is invalid TOML")?;
    match config.get("socket_path") {
        None => Ok(socket(paths)),
        Some(value) => {
            let value = value
                .as_str()
                .context("config socket_path must be a string")?;
            Ok(if value == "~" {
                paths.home.clone()
            } else if let Some(tail) = value.strip_prefix("~/") {
                paths.home.join(tail)
            } else {
                PathBuf::from(value)
            })
        }
    }
}

pub fn model_defaults(paths: &Paths) -> Vec<(&'static str, PathBuf)> {
    vec![
        (
            "vad_model_path",
            paths.models().join("silero-vad/silero_vad.onnx"),
        ),
        (
            "stt_0_6b_model_path",
            paths.models().join("parakeet-tdt-0.6b-v3-onnx"),
        ),
        (
            "stt_1_1b_model_path",
            paths.models().join("parakeet-tdt-1.1b-onnx"),
        ),
        (
            "stt_coreml_model_path",
            paths.models().join("parakeet-tdt-1.1b-coreml"),
        ),
    ]
}

fn defaults(paths: &Paths) -> Result<String> {
    let mut table = toml::map::Map::new();
    table.insert(
        "socket_path".into(),
        toml::Value::String(socket(paths).to_string_lossy().into()),
    );
    table.insert(
        "stt_model_override".into(),
        toml::Value::String("auto".into()),
    );
    for (key, value) in model_defaults(paths) {
        table.insert(
            key.into(),
            toml::Value::String(value.to_string_lossy().into()),
        );
    }
    let hotkey = if cfg!(target_os = "macos") {
        "Ctrl+Shift+D"
    } else {
        "Super+Shift+D"
    };
    let mut hotkeys = toml::map::Map::new();
    hotkeys.insert("toggle".into(), toml::Value::String(hotkey.into()));
    table.insert("hotkeys".into(), toml::Value::Table(hotkeys));
    Ok(format!(
        "# Swictation configuration. Unspecified settings use daemon defaults.\n{}",
        toml::to_string_pretty(&table)?
    ))
}

pub fn check(paths: &Paths) -> Result<()> {
    let raw = fs::read_to_string(paths.config_file()).context("config.toml missing")?;
    let parsed: toml::Value = toml::from_str(&raw).context("config.toml is invalid TOML")?;
    validate(&parsed)?;
    for (key, default) in model_defaults(paths) {
        if let Some(value) = parsed.get(key).and_then(|v| v.as_str()) {
            let configured = value
                .strip_prefix("~/")
                .map(|tail| paths.home.join(tail))
                .unwrap_or_else(|| PathBuf::from(value));
            if !configured.exists() && default.exists() {
                bail!("config {key} points to a missing path while {} exists; run swictation setup --config", default.display());
            }
        }
    }
    Ok(())
}

/// Mirror DaemonConfig field types without invoking filesystem defaults.
/// Persisted overrides deliberately exclude model-download aliases.
pub fn validate(config: &toml::Value) -> Result<()> {
    let table = config
        .as_table()
        .context("configuration must be a TOML table")?;
    for key in [
        "socket_path",
        "vad_model_path",
        "stt_model_override",
        "stt_0_6b_model_path",
        "stt_1_1b_model_path",
        "stt_coreml_model_path",
    ] {
        if table.get(key).is_some_and(|value| !value.is_str()) {
            bail!("config {key} must be a string; existing file preserved");
        }
    }
    for key in [
        "vad_min_silence",
        "vad_min_speech",
        "vad_max_speech",
        "vad_threshold",
        "phonetic_threshold",
    ] {
        if table
            .get(key)
            .is_some_and(|value| !value.is_float() && !value.is_integer())
        {
            bail!("config {key} must be numeric; existing file preserved");
        }
    }
    if let Some(value) = table.get("num_threads") {
        let integer = value
            .as_integer()
            .context("config num_threads must be an integer")?;
        i32::try_from(integer).context("config num_threads is outside the daemon's i32 range")?;
    }
    if let Some(value) = table.get("audio_device_index") {
        let integer = value
            .as_integer()
            .context("config audio_device_index must be a nonnegative integer")?;
        usize::try_from(integer)
            .context("config audio_device_index must fit a nonnegative device index")?;
    }
    if let Some(hotkeys) = table.get("hotkeys") {
        let hotkeys = hotkeys
            .as_table()
            .context("config hotkeys must be a table")?;
        for key in ["toggle", "push_to_talk"] {
            if hotkeys.get(key).is_some_and(|value| !value.is_str()) {
                bail!("config hotkeys.{key} must be a string");
            }
        }
    }
    if let Some(selection) = table
        .get("stt_model_override")
        .and_then(|value| value.as_str())
    {
        let common = ["auto", "0.6b-cpu", "0.6b-gpu", "1.1b-gpu"].contains(&selection);
        let coreml =
            cfg!(target_os = "macos") && ["1.1b-coreml", "coreml-native"].contains(&selection);
        if !common && !coreml {
            bail!("invalid persisted stt_model_override {selection:?}; use auto, 0.6b-cpu, 0.6b-gpu, 1.1b-gpu{}", if cfg!(target_os = "macos") { ", or 1.1b-coreml" } else { "" });
        }
    }
    Ok(())
}

fn backup(paths: &Paths) -> Result<()> {
    let destination = paths.config_file().with_extension(format!(
        "toml.bak.{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    fs::copy(paths.config_file(), &destination)?;
    eprintln!(
        "Preserved previous configuration at {}",
        destination.display()
    );
    Ok(())
}

fn replace_simple(raw: &str, key: &str, expected: &str, replacement: &str) -> String {
    let mut in_table = false;
    raw.split_inclusive('\n')
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with('[') {
                in_table = true;
            }
            if in_table {
                return line.to_string();
            }
            let Some((left, right)) = trimmed.split_once('=') else {
                return line.to_string();
            };
            if left.trim() != key {
                return line.to_string();
            }
            // A single-line parse prevents rewriting quoted keys or multiline values.
            let Ok(probe) = toml::from_str::<toml::Value>(&format!("probe = {right}")) else {
                return line.to_string();
            };
            if probe.get("probe").and_then(|v| v.as_str()) != Some(expected) {
                return line.to_string();
            }
            let prefix = &line[..line.find('=').unwrap() + 1];
            let value = toml::Value::String(replacement.into()).to_string();
            format!(
                "{prefix} {value}{}",
                if line.ends_with('\n') { "\n" } else { "" }
            )
        })
        .collect()
}

pub fn repair(paths: &Paths, reset_managed: bool) -> Result<()> {
    secure_dir(&paths.config)?;
    let file = paths.config_file();
    if fs::symlink_metadata(&file).is_ok() {
        crate::paths::regular_owned(&file)?;
    }
    if !file.exists() {
        atomic_write(&file, defaults(paths)?.as_bytes())?;
        return Ok(());
    }
    let raw = fs::read_to_string(&file)?;
    let parsed = match toml::from_str::<toml::Value>(&raw) {
        Ok(parsed) => parsed,
        Err(error) => {
            backup(paths)?;
            atomic_write(&file, defaults(paths)?.as_bytes())?;
            eprintln!("Replaced invalid configuration after backup: {error}");
            return Ok(());
        }
    };
    let mut updated = raw.clone();
    for (key, default) in model_defaults(paths) {
        if let Some(value) = parsed.get(key).and_then(|v| v.as_str()) {
            let configured = value
                .strip_prefix("~/")
                .map(|tail| paths.home.join(tail))
                .unwrap_or_else(|| PathBuf::from(value));
            if !configured.exists() && default.exists() {
                let healed = replace_simple(&updated, key, value, &default.to_string_lossy());
                if healed == updated {
                    bail!("cannot automatically heal multiline or quoted config key {key}; edit it to {}", default.display());
                }
                updated = healed;
            }
        }
    }
    let state_path = paths.config.join("postinstall-state.json");
    let mut pending_state = None;
    if reset_managed {
        if let Ok(raw_state) = fs::read_to_string(&state_path) {
            if let Ok(mut state) = serde_json::from_str::<serde_json::Value>(&raw_state) {
                if let Some(managed) = state.get("managedOverride").and_then(|v| v.as_str()) {
                    if parsed.get("stt_model_override").and_then(|v| v.as_str()) == Some(managed) {
                        let reset = replace_simple(&updated, "stt_model_override", managed, "auto");
                        if managed != "auto" && reset == updated {
                            bail!("cannot reset multiline or quoted managed stt_model_override; marker preserved for retry");
                        }
                        updated = reset;
                    }
                }
                if let Some(object) = state.as_object_mut() {
                    object.remove("managedOverride");
                    object.remove("managedOverrideAt");
                    crate::paths::regular_owned(&state_path)?;
                    pending_state = Some(serde_json::to_vec_pretty(&state)?);
                }
            }
        }
    }
    validate(&toml::from_str::<toml::Value>(&updated)?)?;
    if raw != updated {
        backup(paths)?;
        atomic_write(&file, updated.as_bytes())?;
    }
    if let Some(state) = pending_state {
        atomic_write(&state_path, &state)?;
    }
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut staged =
        tempfile::NamedTempFile::new_in(path.parent().context("config has no parent")?)?;
    staged.write_all(bytes)?;
    staged.as_file().sync_all()?;
    staged.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rewrite_leaves_unrelated_text_and_tables_intact() {
        let raw = "# custom\nstt_model_override = 'bad'\n\n[other]\nstt_model_override = 'bad'\n";
        assert_eq!(
            replace_simple(raw, "stt_model_override", "bad", "auto"),
            "# custom\nstt_model_override = \"auto\"\n\n[other]\nstt_model_override = 'bad'\n"
        );
    }
    #[test]
    fn refuses_multiline_rewrite() {
        let raw = "stt_model_override = '''\nbad\n'''\n";
        assert_eq!(
            replace_simple(raw, "stt_model_override", "bad", "auto"),
            raw
        );
    }
    #[test]
    fn valid_custom_config_remains_byte_identical() {
        let temp = tempfile::tempdir().unwrap();
        let home = fs::canonicalize(temp.path()).unwrap();
        let paths = Paths {
            data: home.join("data"),
            config: home.join("config"),
            bin: home.join("bin"),
            root: home.join("root"),
            home,
        };
        secure_dir(&paths.config).unwrap();
        let original = "# User comments\nnum_threads=7\nstt_model_override = '0.6b-cpu'\n\n[hotkeys]\ntoggle = 'Alt+D'\n";
        fs::write(paths.config_file(), original).unwrap();
        repair(&paths, true).unwrap();
        assert_eq!(fs::read_to_string(paths.config_file()).unwrap(), original);
    }

    fn fixture() -> (tempfile::TempDir, Paths) {
        let temp = tempfile::tempdir().unwrap();
        let home = fs::canonicalize(temp.path()).unwrap();
        let paths = Paths {
            data: home.join("data"),
            config: home.join("config"),
            bin: home.join("bin"),
            root: home.join("root"),
            home,
        };
        secure_dir(&paths.config).unwrap();
        (temp, paths)
    }

    #[test]
    fn rejects_wrong_daemon_types_and_download_aliases() {
        for raw in [
            "num_threads = 'four'",
            "audio_device_index = -1",
            "vad_threshold = 'quiet'",
            "hotkeys = 1",
            "[hotkeys]\ntoggle = false",
            "stt_model_override = '0.6b'",
            "stt_model_override = 'cpu-only'",
        ] {
            assert!(
                validate(&toml::from_str::<toml::Value>(raw).unwrap()).is_err(),
                "accepted {raw}"
            );
        }
        assert!(validate(
            &toml::from_str::<toml::Value>(
                "num_threads = 4\nstt_model_override = '0.6b-cpu'\n[hotkeys]\ntoggle = 'Alt+D'"
            )
            .unwrap()
        )
        .is_ok());
    }

    #[test]
    fn stale_paths_are_diagnosed_and_healed() {
        let (_temp, paths) = fixture();
        let target = paths.models().join("parakeet-tdt-0.6b-v3-onnx");
        secure_dir(&target).unwrap();
        fs::write(
            paths.config_file(),
            "# keep\nstt_0_6b_model_path = '/missing/old-model'\n",
        )
        .unwrap();
        assert!(check(&paths).is_err());
        repair(&paths, false).unwrap();
        assert!(check(&paths).is_ok());
        assert!(fs::read_to_string(paths.config_file())
            .unwrap()
            .starts_with("# keep\n"));
    }

    #[test]
    fn failed_managed_reset_retains_ownership_marker() {
        let (_temp, paths) = fixture();
        let original = "stt_model_override = '''0.6b-cpu\n'''\n";
        fs::write(paths.config_file(), original).unwrap();
        let state = serde_json::json!({"managedOverride": "0.6b-cpu\n"});
        let marker = paths.config.join("postinstall-state.json");
        let bytes = serde_json::to_vec(&state).unwrap();
        fs::write(&marker, &bytes).unwrap();
        assert!(repair(&paths, true).is_err());
        assert_eq!(fs::read(marker).unwrap(), bytes);
        assert_eq!(fs::read_to_string(paths.config_file()).unwrap(), original);
    }

    #[test]
    fn invalid_user_types_are_preserved_for_editing() {
        let (_temp, paths) = fixture();
        fs::write(paths.config_file(), "num_threads = 'four'\n").unwrap();
        assert!(repair(&paths, true).is_err());
        assert_eq!(
            fs::read_to_string(paths.config_file()).unwrap(),
            "num_threads = 'four'\n"
        );
    }

    #[test]
    fn toggle_uses_configured_socket_with_daemon_tilde_expansion() {
        let (_temp, paths) = fixture();
        fs::write(
            paths.config_file(),
            "socket_path = '~/custom/daemon.sock'\n",
        )
        .unwrap();
        assert_eq!(
            configured_socket(&paths).unwrap(),
            paths.home.join("custom/daemon.sock")
        );
        fs::write(paths.config_file(), "socket_path = 123\n").unwrap();
        assert!(configured_socket(&paths).is_err());
    }
}
