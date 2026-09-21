//! Persistent NVIDIA libraries; archive and extracted-file integrity are separate.
use crate::{
    models::{self, ModelFile},
    network,
    paths::Paths,
};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use std::{fs, io::Read, path::Path, process::Command};

const VERSION: &str = "1.2.0";
#[derive(Serialize, Deserialize)]
struct Inventory {
    variant: String,
    version: String,
    files: Vec<Entry>,
}
#[derive(Serialize, Deserialize)]
struct Entry {
    path: String,
    size: u64,
    sha256: String,
}

pub fn variant() -> Result<Option<String>> {
    if !cfg!(target_os = "linux") {
        return Ok(None);
    }
    let output = match Command::new("nvidia-smi")
        .args(["--query-gpu=compute_cap", "--format=csv,noheader"])
        .output()
    {
        Ok(v) => v,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !output.status.success() {
        // A present but broken driver must not be reported as a CPU-only machine.
        bail!(
            "nvidia-smi could not query compute capability: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let text = String::from_utf8(output.stdout)?;
    let capability = text
        .lines()
        .next()
        .context("NVIDIA returned no GPU")?
        .trim();
    let sm: u32 = capability
        .replace('.', "")
        .parse()
        .context("unrecognized NVIDIA compute capability")?;
    let name = match sm {
        50..=70 => "legacy",
        75..=86 => "modern",
        89..=121 => "latest",
        _ => bail!("unsupported NVIDIA compute capability {capability}"),
    };
    Ok(Some(name.into()))
}

pub fn check(paths: &Paths, expected_variant: &str, deep: bool) -> Result<()> {
    let dir = paths.data.join("gpu-libs");
    let inventory: Inventory =
        serde_json::from_slice(&fs::read(dir.join("gpu-libs.manifest.json"))?)
            .context("GPU inventory missing or invalid")?;
    if inventory.variant != expected_variant
        || inventory.version != VERSION
        || inventory.files.is_empty()
    {
        bail!("GPU library inventory does not match this hardware");
    }
    for entry in inventory.files {
        let path = dir.join(models::relative(&entry.path)?);
        models::verify_file(
            &path,
            &ModelFile {
                path: entry.path,
                size: entry.size,
                sha256: entry.sha256,
            },
            deep,
        )?;
    }
    if !dir.join("libonnxruntime.so").is_file() {
        bail!("GPU ONNX Runtime is missing");
    }
    Ok(())
}

fn verify_archive(path: &Path, variant: &str) -> Result<()> {
    let name = format!("cuda-libs-{variant}.tar.gz");
    let expected = include_str!("../../../config/gpu-checksums.txt")
        .lines()
        .find_map(|line| {
            let mut parts = line.split_whitespace();
            let digest = parts.next()?;
            (parts.next()? == name).then_some(digest)
        })
        .context("GPU archive checksum missing")?;
    let mut digest = Sha512::new();
    let mut reader = fs::File::open(path)?;
    let mut buffer = [0; 65536];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    if format!("{:x}", digest.finalize()) != expected {
        bail!("GPU archive SHA-512 mismatch");
    }
    Ok(())
}

pub fn install(paths: &Paths) -> Result<()> {
    let Some(variant) = variant()? else {
        return Ok(());
    };
    if check(paths, &variant, true).is_ok() {
        return Ok(());
    }
    crate::paths::secure_dir(&paths.data)?;
    let stage = tempfile::tempdir_in(&paths.data)?;
    let archive = stage.path().join("gpu.tar.gz");
    let url = format!("https://github.com/robertelee78/swictation/releases/download/gpu-libs-v{VERSION}/cuda-libs-{variant}.tar.gz");
    network::download(&url, &archive)?;
    verify_archive(&archive, &variant)?;
    let unpacked = stage.path().join("unpacked");
    fs::create_dir(&unpacked)?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(fs::File::open(&archive)?));
    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        models::relative(path.to_str().context("non-UTF8 GPU archive path")?)?;
        let kind = entry.header().entry_type();
        if !(kind.is_file() || kind.is_dir() || kind.is_symlink()) {
            bail!("unsupported GPU archive entry {}", path.display());
        }
        if kind.is_symlink() {
            let target = entry.link_name()?.context("GPU symlink has no target")?;
            models::relative(target.to_str().context("non-UTF8 GPU symlink")?)?;
        }
        if !entry.unpack_in(&unpacked)? {
            bail!("GPU archive path escaped staging directory");
        }
    }
    let source = unpacked.join(&variant).join("libs");
    let candidate = stage.path().join("gpu-libs");
    fs::create_dir(&candidate)?;
    let mut files = Vec::new();
    for item in fs::read_dir(&source).context("GPU archive does not contain expected libraries")? {
        let item = item?;
        if !item.path().is_file() {
            continue;
        }
        let resolved = fs::canonicalize(item.path())?;
        if !resolved.starts_with(fs::canonicalize(&source)?) {
            bail!("GPU symlink escapes library directory");
        }
        let filename = item.file_name();
        let destination = candidate.join(&filename);
        fs::copy(resolved, &destination)?;
        files.push(Entry {
            path: filename.to_string_lossy().into(),
            size: fs::metadata(&destination)?.len(),
            sha256: models::checksum(&destination)?,
        });
    }
    if files.is_empty() || !candidate.join("libonnxruntime.so").is_file() {
        bail!("GPU archive has no ONNX Runtime");
    }
    let inventory = Inventory {
        variant: variant.clone(),
        version: VERSION.into(),
        files,
    };
    fs::write(
        candidate.join("gpu-libs.manifest.json"),
        serde_json::to_vec_pretty(&inventory)?,
    )?;
    let destination = paths.data.join("gpu-libs");
    let previous = stage.path().join("previous");
    if destination.exists() {
        fs::rename(&destination, &previous)?;
    }
    if let Err(error) = fs::rename(&candidate, &destination) {
        if previous.exists() {
            fs::rename(&previous, &destination)
                .context("GPU installation failed and rollback failed")?;
        }
        return Err(error.into());
    }
    check(paths, &variant, false)
}
