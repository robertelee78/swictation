//! Pinned, streamed speech-model downloads, shared by setup and repair.
use crate::{
    network,
    paths::{secure_dir, Paths},
};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
    process::Command,
};

#[derive(Deserialize)]
pub struct Manifest {
    pub models: BTreeMap<String, Model>,
}
#[derive(Deserialize)]
pub struct Model {
    #[serde(rename = "targetDir")]
    pub target_dir: String,
    pub source: Source,
    pub files: Vec<ModelFile>,
}
#[derive(Deserialize)]
pub struct Source {
    pub url: Option<String>,
    pub repo: Option<String>,
    pub revision: Option<String>,
}
#[derive(Deserialize)]
pub struct ModelFile {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

pub fn manifest() -> Result<Manifest> {
    serde_json::from_str(include_str!("../../../config/models.manifest.json"))
        .context("invalid embedded model manifest")
}

pub fn relative(path: &str) -> Result<&Path> {
    let p = Path::new(path);
    if p.as_os_str().is_empty() || p.components().any(|c| !matches!(c, Component::Normal(_))) {
        bail!("unsafe manifest path: {path}");
    }
    Ok(p)
}

pub fn checksum(path: &Path) -> Result<String> {
    let mut reader = fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

pub fn verify_file(path: &Path, expected: &ModelFile, deep: bool) -> Result<()> {
    let metadata = fs::metadata(path).with_context(|| format!("missing {}", path.display()))?;
    if !metadata.is_file() || metadata.len() != expected.size {
        bail!(
            "{}: expected {} bytes, found {}",
            path.display(),
            expected.size,
            metadata.len()
        );
    }
    if deep && checksum(path)? != expected.sha256 {
        bail!("SHA-256 mismatch: {}", path.display());
    }
    Ok(())
}

pub fn normalize(selection: &str) -> &str {
    match selection {
        "cpu-only" | "0.6b-cpu" | "0.6b-gpu" => "0.6b",
        "1.1b-gpu" => "1.1b",
        "coreml-native" => "1.1b-coreml",
        other => other,
    }
}

pub fn selected_model(paths: &Paths) -> Result<String> {
    if let Ok(raw) = fs::read_to_string(paths.config_file()) {
        let config: toml::Value =
            toml::from_str(&raw).context("invalid config.toml; run swictation setup --config")?;
        crate::configuration::validate(&config)?;
        if let Some(value) = config.get("stt_model_override").and_then(|v| v.as_str()) {
            if value != "auto" {
                return validate_selection(value);
            }
        }
    }
    if cfg!(target_os = "macos") {
        return Ok("1.1b-coreml".into());
    }
    let output = Command::new("nvidia-smi")
        .args(["--query-gpu=memory.total", "--format=csv,noheader,nounits"])
        .output();
    let memory = output
        .ok()
        .filter(|v| v.status.success())
        .and_then(|v| String::from_utf8(v.stdout).ok())
        .and_then(|v| v.lines().next().unwrap_or("").trim().parse::<u64>().ok())
        .unwrap_or(0);
    Ok(if memory >= 6000 { "1.1b" } else { "0.6b" }.into())
}

fn validate_selection(selection: &str) -> Result<String> {
    let key = normalize(selection);
    if key == "vad" || !manifest()?.models.contains_key(key) {
        bail!("unknown speech model: {selection}");
    }
    if cfg!(target_os = "linux") && key.contains("coreml") {
        bail!("CoreML models require macOS Apple Silicon");
    }
    Ok(key.into())
}

pub fn model_dir(paths: &Paths, key: &str, model: &Model) -> Result<PathBuf> {
    let config_key = match key {
        "vad" => "vad_model_path",
        "0.6b" => "stt_0_6b_model_path",
        "1.1b" => "stt_1_1b_model_path",
        "1.1b-coreml" => "stt_coreml_model_path",
        _ => "",
    };
    if let Ok(raw) = fs::read_to_string(paths.config_file()) {
        if let Ok(config) = toml::from_str::<toml::Value>(&raw) {
            if let Some(value) = config.get(config_key).and_then(|v| v.as_str()) {
                let p = if let Some(tail) = value.strip_prefix("~/") {
                    paths.home.join(tail)
                } else {
                    PathBuf::from(value)
                };
                // Custom, absolute model paths remain user-owned. Missing stale paths
                // are healed only after the default model has been downloaded.
                if p.is_absolute() && p.exists() {
                    return if key == "vad" {
                        Ok(p.parent().context("invalid VAD path")?.into())
                    } else {
                        Ok(p)
                    };
                }
            }
        }
    }
    Ok(paths.models().join(relative(&model.target_dir)?))
}

pub fn check(paths: &Paths, selection: &str, deep: bool) -> Result<()> {
    let manifest = manifest()?;
    for key in ["vad", normalize(selection)] {
        let model = manifest
            .models
            .get(key)
            .with_context(|| format!("unknown model {key}"))?;
        let dir = model_dir(paths, key, model)?;
        for file in &model.files {
            verify_file(&dir.join(relative(&file.path)?), file, deep)?;
        }
    }
    Ok(())
}

fn source_url(model: &Model, file: &ModelFile) -> Result<String> {
    if let Some(url) = &model.source.url {
        if !url.starts_with("https://") {
            bail!("model source requires HTTPS");
        }
        return Ok(url.clone());
    }
    let repo = model
        .source
        .repo
        .as_deref()
        .context("model repository missing")?;
    let revision = model
        .source
        .revision
        .as_deref()
        .context("model revision missing")?;
    if revision.len() != 40 || !revision.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("model revision is not pinned");
    }
    relative(repo)?;
    relative(&file.path)?;
    Ok(format!(
        "https://huggingface.co/{repo}/resolve/{revision}/{}",
        file.path
    ))
}

pub fn download_models(paths: &Paths, selection: &str, force: bool) -> Result<()> {
    let selection = if selection == "auto" {
        selected_model(paths)?
    } else {
        selection.to_string()
    };
    let keys = if selection == "both" {
        if cfg!(target_os = "macos") {
            vec!["1.1b-coreml".into()]
        } else {
            vec!["0.6b".into(), "1.1b".into()]
        }
    } else {
        vec![validate_selection(&selection)?]
    };
    let manifest = manifest()?;
    for key in std::iter::once("vad".to_string()).chain(keys) {
        let model = &manifest.models[&key];
        let dir = model_dir(paths, &key, model)?;
        secure_dir(&dir)?;
        for file in &model.files {
            let destination = dir.join(relative(&file.path)?);
            if fs::symlink_metadata(&destination).is_ok() {
                crate::paths::regular_owned(&destination)?;
            }
            let valid = verify_file(&destination, file, true).is_ok();
            if !force && valid {
                continue;
            }
            let parent = destination.parent().context("model file has no parent")?;
            secure_dir(parent)?;
            let stage = tempfile::tempdir_in(parent)?;
            let staged_file = stage.path().join("download");
            eprintln!("Downloading {key}/{} ({} bytes)", file.path, file.size);
            network::download_bounded(&source_url(model, file)?, &staged_file, file.size)?;
            verify_file(&staged_file, file, true)?;
            let mut previous = None;
            if destination.exists() {
                let quarantine = destination.with_file_name(format!(
                    "{}.{}.{}",
                    destination.file_name().unwrap().to_string_lossy(),
                    if valid { "replaced" } else { "corrupt" },
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_nanos()
                ));
                fs::rename(&destination, &quarantine)?;
                previous = Some(quarantine);
            }
            if let Err(error) = fs::rename(&staged_file, &destination) {
                if let Some(previous) = previous {
                    fs::rename(previous, &destination).context(
                        "model publication failed and previous file could not be restored",
                    )?;
                }
                return Err(error.into());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn same_size_tamper_fails_deep_verification() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("weights");
        fs::write(&file, b"good").unwrap();
        let expected = ModelFile {
            path: "weights".into(),
            size: 4,
            sha256: checksum(&file).unwrap(),
        };
        fs::write(&file, b"evil").unwrap();
        assert!(verify_file(&file, &expected, false).is_ok());
        assert!(verify_file(&file, &expected, true).is_err());
    }
    #[test]
    fn manifest_paths_cannot_escape_root() {
        for path in ["../outside", "/absolute", "weights/../../outside", ""] {
            assert!(relative(path).is_err());
        }
        assert!(relative("Encoder.mlmodelc/weights/weight.bin").is_ok());
    }
}
