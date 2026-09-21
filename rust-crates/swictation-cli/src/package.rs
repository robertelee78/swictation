use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

pub const ARCHIVE_LIMIT: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Release {
    pub schema_version: u32,
    pub version: String,
    pub target: String,
    pub source_sha: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub schema_version: u32,
    pub archive_sha256: String,
    pub release: Release,
    pub files: BTreeMap<String, String>,
}

pub fn target() -> Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        _ => bail!("Supported platforms: Linux x86_64 and macOS Apple Silicon"),
    }
}

pub fn valid_hash(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        bail!("Expected lowercase SHA-256");
    }
    Ok(())
}

pub fn hash(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

pub fn unpack(
    archive: &Path,
    expected: &str,
    dest: &Path,
    unsigned_local: bool,
) -> Result<Receipt> {
    valid_hash(expected)?;
    if crate::paths::regular_owned(archive)?.len() > ARCHIVE_LIMIT {
        bail!("Archive too large");
    }
    // Copy to a private file while hashing: verification and extraction use the same bytes.
    let mut input = fs::File::open(archive)?;
    let mut verified = tempfile::tempfile()?;
    let size = std::io::copy(
        &mut Read::by_ref(&mut input).take(ARCHIVE_LIMIT + 1),
        &mut verified,
    )?;
    if size > ARCHIVE_LIMIT {
        bail!("Archive too large");
    }
    use std::io::{Seek, SeekFrom};
    verified.seek(SeekFrom::Start(0))?;
    let mut digest = Sha256::new();
    std::io::copy(&mut verified, &mut digest)?;
    if format!("{:x}", digest.finalize()) != expected {
        bail!("Archive checksum mismatch; installation unchanged");
    }
    verified.seek(SeekFrom::Start(0))?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(verified));
    let mut files = BTreeMap::new();
    let mut total = 0u64;
    for (count, entry) in tar.entries()?.enumerate() {
        if count > 20_000 {
            bail!("Archive has too many entries");
        }
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        crate::paths::safe_relative(&path)?;
        let first = path
            .components()
            .next()
            .context("Empty archive path")?
            .as_os_str();
        if first != "bin" && first != "lib" && first != "share" && path != Path::new("release.json")
        {
            bail!("Unexpected archive member: {}", path.display());
        }
        let out = dest.join(&path);
        let kind = entry.header().entry_type();
        if kind.is_dir() {
            fs::create_dir_all(&out)?;
            continue;
        }
        if !kind.is_file() {
            bail!("Archive links and special files are forbidden");
        }
        total = total
            .checked_add(entry.size())
            .context("Archive size overflow")?;
        if total > 4 * 1024 * 1024 * 1024 {
            bail!("Expanded archive exceeds limit");
        }
        fs::create_dir_all(out.parent().context("Missing parent")?)?;
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&out)?;
        std::io::copy(&mut entry, &mut output)?;
        output.flush()?;
        output.sync_all()?;
        let mode = if entry.header().mode()? & 0o111 != 0 {
            0o700
        } else {
            0o600
        };
        fs::set_permissions(&out, fs::Permissions::from_mode(mode))?;
        files.insert(
            path.to_str().context("Non-UTF8 archive path")?.to_string(),
            hash(&out)?,
        );
    }
    for name in [
        "bin/swictation",
        "bin/swictation-daemon",
        "bin/swictation-ui",
        "release.json",
        "share/models.manifest.json",
        "share/config.example.toml",
    ] {
        if !files.contains_key(name) {
            bail!("Incomplete release: missing {name}");
        }
    }
    let manifest = dest.join("release.json");
    if fs::metadata(&manifest)?.len() > 16 * 1024 {
        bail!("Release manifest too large");
    }
    let release: Release = serde_json::from_slice(&fs::read(manifest)?)?;
    validate_release(&release)?;
    for name in ["swictation", "swictation-daemon", "swictation-ui"] {
        if fs::metadata(dest.join("bin").join(name))?
            .permissions()
            .mode()
            & 0o100
            == 0
        {
            bail!("Nonexecutable {name}");
        }
    }
    let library = if cfg!(target_os = "macos") {
        "lib/libonnxruntime.dylib"
    } else {
        "lib/libonnxruntime.so"
    };
    if !files.contains_key(library) {
        bail!("Incomplete release: missing {library}");
    }
    crate::trust::verify_payload(dest, &files, unsigned_local)?;
    let output = std::process::Command::new(dest.join("bin/swictation"))
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .output()
        .context("Probe candidate CLI")?;
    if !output.status.success()
        || String::from_utf8_lossy(&output.stdout).trim()
            != format!("swictation {}", release.version)
    {
        bail!("Candidate CLI version disagrees with release manifest");
    }
    Ok(Receipt {
        schema_version: 1,
        archive_sha256: expected.into(),
        release,
        files,
    })
}

pub fn validate_release(release: &Release) -> Result<()> {
    if release.schema_version != 1 || release.target != target()? {
        bail!("Incompatible release schema or platform");
    }
    crate::network::stable_version(&release.version)?;
    if release.source_sha.len() != 40 || !release.source_sha.bytes().all(|c| c.is_ascii_hexdigit())
    {
        bail!("Invalid release source identity");
    }
    Ok(())
}

pub fn read_receipt(dir: &Path) -> Result<Receipt> {
    let file = dir.join(".receipt.json");
    if crate::paths::regular_owned(&file)?.len() > 4 * 1024 * 1024 {
        bail!("Receipt too large");
    }
    let receipt: Receipt = serde_json::from_slice(&fs::read(file)?)?;
    if receipt.schema_version != 1 {
        bail!("Unsupported receipt");
    }
    valid_hash(&receipt.archive_sha256)?;
    validate_release(&receipt.release)?;
    let manifest: Release = serde_json::from_slice(&fs::read(dir.join("release.json"))?)?;
    if manifest != receipt.release {
        bail!("Receipt does not match release metadata");
    }
    Ok(receipt)
}

pub fn verify_files(dir: &Path, receipt: &Receipt) -> Result<()> {
    for (name, expected) in &receipt.files {
        let path = PathBuf::from(name);
        crate::paths::safe_relative(&path)?;
        crate::paths::regular_owned(&dir.join(&path))?;
        valid_hash(expected)?;
        if hash(&dir.join(&path))? != *expected {
            bail!("Installed file was modified: {name}");
        }
    }
    Ok(())
}
