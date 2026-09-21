use anyhow::{bail, Result};
use std::{path::Path, process::Command};

fn team(unsigned_local: bool) -> Result<Option<&'static str>> {
    if !cfg!(target_os = "macos") {
        return Ok(None);
    }
    let value = option_env!("SWICTATION_APPLE_TEAM_ID");
    if unsigned_local && value.is_none() {
        return Ok(None);
    }
    let value = value.ok_or_else(|| anyhow::anyhow!("This development build cannot authenticate macOS releases; use the published installer"))?;
    if value.len() != 10
        || !value
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
    {
        bail!("Invalid compiled Apple team identity");
    }
    Ok(Some(value))
}

fn checked(args: &[&str], path: &Path) -> Result<()> {
    let output = Command::new("/usr/bin/codesign")
        .args(args)
        .arg(path)
        .output()?;
    if !output.status.success() {
        bail!(
            "Apple trust failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn identity_requirement(identity: Option<&str>, team: &str) -> String {
    // Apple's requirement arguments are filenames unless prefixed with '='.
    let mut requirement = format!("=anchor apple generic and certificate leaf[subject.OU] = \"{team}\" and certificate leaf[field.1.2.840.113635.100.6.1.13] exists");
    if let Some(id) = identity {
        requirement.push_str(&format!(" and identifier \"{id}\""));
    }
    requirement
}

fn code(path: &Path, identity: Option<&str>, team: &str, executable: bool) -> Result<()> {
    let requirement = identity_requirement(identity, team);
    checked(
        &[
            "--verify",
            "--strict",
            "--deep",
            "--test-requirement",
            &requirement,
        ],
        path,
    )?;
    let details = Command::new("/usr/bin/codesign")
        .args(["--display", "--verbose=4"])
        .arg(path)
        .output()?;
    let text = String::from_utf8_lossy(&details.stderr);
    let runtime = text
        .lines()
        .filter(|line| line.starts_with("CodeDirectory "))
        .any(|line| {
            line.split_whitespace()
                .find_map(|part| part.strip_prefix("flags=0x"))
                .and_then(|value| {
                    u64::from_str_radix(value.split('(').next().unwrap_or(""), 16).ok()
                })
                .is_some_and(|flags| flags & 0x10000 != 0)
        });
    let timestamp = text.lines().any(|line| {
        line.strip_prefix("Timestamp=")
            .is_some_and(|v| !v.trim().is_empty() && v != "none")
    });
    if !details.status.success() || !timestamp || (executable && !runtime) {
        bail!(
            "Missing secure timestamp or hardened runtime: {}",
            path.display()
        );
    }
    checked(
        &[
            "--verify",
            "--check-notarization",
            "--test-requirement",
            "=notarized",
        ],
        path,
    )?;
    if path.is_file() {
        let output = Command::new("/usr/bin/lipo")
            .arg("-archs")
            .arg(path)
            .output()?;
        if !output.status.success() || String::from_utf8_lossy(&output.stdout).trim() != "arm64" {
            bail!("Expected a thin Apple Silicon Mach-O: {}", path.display());
        }
    }
    Ok(())
}

pub fn verify_payload(
    root: &Path,
    files: &std::collections::BTreeMap<String, String>,
    unsigned_local: bool,
) -> Result<()> {
    let Some(team) = team(unsigned_local)? else {
        return Ok(());
    };
    for (name, id) in [
        ("swictation", "com.swictation.cli"),
        ("swictation-daemon", "com.swictation.daemon"),
        ("swictation-ui", "com.swictation.ui"),
    ] {
        code(&root.join("bin").join(name), Some(id), team, true)?;
    }
    for name in files.keys().filter(|name| name.ends_with(".dylib")) {
        code(&root.join(name), None, team, false)?;
    }
    for (name, id) in [
        ("SwictationDaemon.app", "com.swictation.daemon"),
        ("Swictation.app", "com.swictation.ui"),
    ] {
        let app = root.join("share").join(name);
        if !app.is_dir() {
            bail!("Signed release is missing app bundle {name}");
        }
        code(&app, Some(id), team, true)?;
    }
    Ok(())
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn apple_accepts_inline_identity_requirement() {
        for identity in [Some("com.swictation.cli"), None] {
            let output = Command::new("/usr/bin/csreq")
                .args(["-r", &identity_requirement(identity, "3T2D2YNTVW"), "-t"])
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}
