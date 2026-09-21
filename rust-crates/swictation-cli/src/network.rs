use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use std::{
    io::{Read, Write},
    path::Path,
    time::Duration,
};

pub const REPOSITORY: &str = "robertelee78/swictation";

fn client() -> Result<Client> {
    Ok(Client::builder()
        .user_agent(concat!("swictation/", env!("CARGO_PKG_VERSION")))
        .https_only(true)
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(1800))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()?)
}

pub fn text(url: &str, limit: u64) -> Result<String> {
    let response = client()?.get(url).send()?.error_for_status()?;
    let mut bytes = Vec::new();
    response.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        bail!("Response exceeded {} bytes", limit);
    }
    Ok(String::from_utf8(bytes)?)
}

pub fn download(url: &str, destination: &Path) -> Result<()> {
    download_bounded(url, destination, 16 * 1024 * 1024 * 1024)
}

pub fn download_bounded(url: &str, destination: &Path, limit: u64) -> Result<()> {
    let parent = destination
        .parent()
        .context("Download destination has no parent")?;
    crate::paths::secure_dir(parent)?;
    let mut response = client()?.get(url).send()?.error_for_status()?;
    if response.content_length().is_some_and(|size| size > limit) {
        bail!("Download exceeds size limit");
    }
    let mut partial = tempfile::NamedTempFile::new_in(parent)?;
    let mut total = 0u64;
    let mut buffer = [0u8; 65536];
    loop {
        let n = response.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > limit {
            bail!("Download exceeds size limit");
        }
        partial.write_all(&buffer[..n])?;
    }
    partial.as_file().sync_all()?;
    partial
        .persist(destination)
        .context("Publish complete download")?;
    crate::paths::sync_dir(parent)
}

pub fn stable_version(version: &str) -> Result<semver::Version> {
    let parsed = semver::Version::parse(version)?;
    if !parsed.pre.is_empty() || !parsed.build.is_empty() || parsed.to_string() != version {
        bail!("Expected canonical stable version (for example 0.7.37)");
    }
    Ok(parsed)
}

pub fn latest() -> Result<String> {
    let body = text(
        &format!("https://api.github.com/repos/{REPOSITORY}/releases/latest"),
        128 * 1024,
    )?;
    let value: serde_json::Value = serde_json::from_str(&body)?;
    if value["draft"].as_bool() != Some(false) || value["prerelease"].as_bool() != Some(false) {
        bail!("Latest release is not stable");
    }
    let tag = value["tag_name"].as_str().context("Missing release tag")?;
    let version = tag
        .strip_prefix('v')
        .context("Release tag must start with v")?;
    stable_version(version)?;
    Ok(version.to_string())
}
