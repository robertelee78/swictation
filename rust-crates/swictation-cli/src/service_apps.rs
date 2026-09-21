//! Stable aliases retain complete signed app bundles through atomic updates.
use crate::paths::Paths;
use anyhow::{bail, Result};
use std::{fs, path::PathBuf};

const APPS: &[&str] = &["SwictationDaemon.app", "Swictation.app"];

pub(super) fn binary(paths: &Paths, component: &str) -> PathBuf {
    let name = if component == "daemon" {
        "SwictationDaemon.app"
    } else {
        "Swictation.app"
    };
    paths
        .root
        .join(name)
        .join("Contents/MacOS")
        .join(format!("swictation-{component}"))
}

pub(super) fn refresh(paths: &Paths) -> Result<()> {
    crate::paths::secure_dir(&paths.root)?;
    validate_remove(paths)?;
    for name in APPS {
        if !paths.current().join("share").join(name).is_dir() {
            bail!("complete signed app {name} missing from active release");
        }
    }
    for name in APPS {
        let destination = paths.root.join(name);
        if fs::symlink_metadata(&destination).is_ok() {
            continue;
        }
        std::os::unix::fs::symlink(PathBuf::from("current/share").join(name), destination)?;
    }
    Ok(())
}

pub(super) fn validate_remove(paths: &Paths) -> Result<()> {
    for name in APPS {
        let destination = paths.root.join(name);
        match fs::symlink_metadata(&destination) {
            Ok(metadata) => {
                if !metadata.file_type().is_symlink()
                    || fs::read_link(&destination)? != PathBuf::from("current/share").join(name)
                {
                    bail!(
                        "preserved foreign or modified application {}",
                        destination.display()
                    );
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

pub(super) fn uninstall(paths: &Paths) -> Result<()> {
    validate_remove(paths)?;
    for name in APPS {
        let destination = paths.root.join(name);
        if fs::symlink_metadata(&destination).is_ok() {
            fs::remove_file(destination)?;
        }
    }
    Ok(())
}
