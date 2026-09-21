use crate::{
    package::{self, Receipt},
    paths::{self, Paths},
    services,
};
use anyhow::{bail, Context, Result};
use fs2::FileExt;
use std::{
    fs,
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

const OWNER: &[u8] = b"swictation-standalone-v1\n";
pub struct Lock {
    _file: fs::File,
}

impl Drop for Lock {
    fn drop(&mut self) {
        // A concurrently spawned process can inherit this open file description
        // until exec. Closing only our descriptor would leave its lock behind.
        let _ = FileExt::unlock(&self._file);
    }
}

pub fn lock(paths: &Paths) -> Result<Lock> {
    paths::secure_dir(&paths.root)?;
    let lock_path = paths.root.join(".lock");
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&lock_path)?;
    paths::regular_owned(&lock_path)?;
    file.try_lock_exclusive()
        .context("Another swictation install/setup/update is running")?;
    let guard = Lock { _file: file };
    let owner = paths.root.join(".owner");
    if fs::symlink_metadata(&owner).is_ok() {
        paths::regular_owned(&owner)?;
        if fs::read(owner)? != OWNER {
            bail!("Unrecognized installation owner");
        }
    } else {
        if fs::read_dir(&paths.root)?.any(|e| e.map_or(true, |e| e.file_name() != ".lock")) {
            bail!("Refusing to adopt nonempty unowned installation directory");
        }
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(owner)?;
        file.write_all(OWNER)?;
        file.sync_all()?;
    }
    Ok(guard)
}

fn pointer(paths: &Paths, name: &str) -> Result<Option<PathBuf>> {
    let link = paths.root.join(name);
    match fs::symlink_metadata(&link) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
        Ok(m) if !m.file_type().is_symlink() => bail!("Unsafe installation pointer: {name}"),
        _ => (),
    }
    let target = fs::read_link(link)?;
    paths::safe_relative(&target)?;
    if target.components().count() != 2 || !target.starts_with("releases") {
        bail!("Invalid installation pointer");
    }
    paths::secure_dir(&paths.root.join(&target))?;
    package::read_receipt(&paths.root.join(&target))?;
    Ok(Some(target))
}

fn set_pointer(paths: &Paths, name: &str, value: &Path) -> Result<()> {
    let temp = tempfile::Builder::new()
        .prefix(".link-")
        .tempdir_in(&paths.root)?;
    let link = temp.path().join("link");
    std::os::unix::fs::symlink(value, &link)?;
    fs::rename(link, paths.root.join(name))?;
    paths::sync_dir(&paths.root)
        .context("Pointer changed, but durability sync failed; inspect current before retrying")
}

fn check_launcher(paths: &Paths) -> Result<bool> {
    let launcher = paths.launcher();
    match fs::symlink_metadata(&launcher) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
        Ok(m) if m.file_type().is_symlink() && fs::read_link(&launcher)? == paths.current().join("bin/swictation") => Ok(true),
        _ => bail!("Refusing to replace foreign launcher {}. Remove the old npm installation with `npm uninstall -g --ignore-scripts swictation` before installing here; keep your data.", launcher.display()),
    }
}

pub fn current(paths: &Paths) -> Result<Receipt> {
    let target = pointer(paths, "current")?
        .context("No native installation; run the standalone installer")?;
    package::read_receipt(&paths.root.join(target))
}

pub fn require_current_executable(paths: &Paths) -> Result<()> {
    let current = fs::canonicalize(paths.current().join("bin/swictation"))
        .context("No managed installation")?;
    if fs::canonicalize(std::env::current_exe()?)? != current {
        bail!(
            "Run the managed executable at {}",
            paths.launcher().display()
        );
    }
    Ok(())
}

pub fn install(paths: &Paths, archive: &Path, expected: &str, unsigned_local: bool) -> Result<()> {
    let _guard = lock(paths)?;
    install_locked(paths, archive, expected, unsigned_local, None)
}

fn install_locked(
    paths: &Paths,
    archive: &Path,
    expected: &str,
    unsigned_local: bool,
    version: Option<&str>,
) -> Result<()> {
    paths::secure_dir(&paths.bin)?;
    let had_launcher = check_launcher(paths)?;
    let previous = pointer(paths, "current")?;
    let retained = pointer(paths, "previous")?;
    let releases = paths.root.join("releases");
    paths::secure_dir(&releases)?;
    let staging = tempfile::Builder::new()
        .prefix(".stage-")
        .tempdir_in(&releases)?;
    let receipt = package::unpack(archive, expected, staging.path(), unsigned_local)?;
    if version.is_some_and(|v| v != receipt.release.version) {
        bail!("Downloaded release version differs from requested version");
    }
    let name = format!("{}-{}", receipt.release.version, expected);
    let mut relative = PathBuf::from("releases").join(name);
    let mut dest = paths.root.join(&relative);
    if let Some(old) = &previous {
        let old_receipt = package::read_receipt(&paths.root.join(old))?;
        if crate::network::stable_version(&receipt.release.version)?
            < crate::network::stable_version(&old_receipt.release.version)?
        {
            bail!("Refusing downgrade; use update --rollback for the retained release");
        }
    }
    if dest.exists() {
        let existing = package::read_receipt(&dest)?;
        if existing.archive_sha256 != expected {
            bail!("Existing release identity mismatch");
        }
        if package::verify_files(&dest, &existing).is_err() {
            // Repair into a distinct immutable directory; never overwrite live files.
            relative = PathBuf::from("releases").join(format!(
                "{}-{}-repair-{}",
                receipt.release.version,
                expected,
                staging
                    .path()
                    .file_name()
                    .context("Missing staging name")?
                    .to_string_lossy()
            ));
            dest = paths.root.join(&relative);
        }
    }
    if !dest.exists() {
        let receipt_path = staging.path().join(".receipt.json");
        fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt)?)?;
        fs::set_permissions(&receipt_path, fs::Permissions::from_mode(0o600))?;
        fs::File::open(&receipt_path)?.sync_all()?;
        fs::rename(staging.path(), &dest)?;
        paths::sync_dir(&releases)?;
    }
    if previous.as_ref() == Some(&relative) && had_launcher {
        println!(
            "swictation {} is already installed (archive verified)",
            receipt.release.version
        );
        return Ok(());
    }
    // First install only places binaries. Setup owns legacy service migration.
    let running = if previous.is_some() {
        services::snapshot(paths)?
    } else {
        services::RunningState::default()
    };
    if let Err(error) = services::stop_running(paths, &running) {
        let restore = services::restore(paths, &running);
        bail!("Failed to stop services: {error:#}; restoration: {restore:?}");
    }
    let result = (|| -> Result<()> {
        if let Some(old) = &previous {
            set_pointer(paths, "previous", old)?;
        }
        set_pointer(paths, "current", &relative)?;
        if !had_launcher {
            std::os::unix::fs::symlink(paths.current().join("bin/swictation"), paths.launcher())?;
            paths::sync_dir(&paths.bin)?;
        }
        services::restore(paths, &running)?;
        Ok(())
    })();
    if let Err(error) = result {
        let stopping = services::stop_running(paths, &running);
        let recovery = (|| -> Result<()> {
            if let Some(old) = &previous {
                set_pointer(paths, "current", old)?;
            } else if fs::symlink_metadata(paths.current()).is_ok() {
                fs::remove_file(paths.current())?;
            }
            if !had_launcher && check_launcher(paths)? {
                fs::remove_file(paths.launcher())?;
            }
            if let Some(retained) = &retained {
                set_pointer(paths, "previous", retained)?;
            } else if fs::symlink_metadata(paths.root.join("previous")).is_ok() {
                fs::remove_file(paths.root.join("previous"))?;
            }
            services::restore(paths, &running)
        })();
        bail!("Activation failed: {error:#}; recovery stop: {stopping:?}; previous-release recovery: {recovery:?}");
    }
    println!(
        "Installed swictation {} at {}",
        receipt.release.version,
        paths.launcher().display()
    );
    println!("Run `swictation setup`, then `swictation doctor`. Existing configuration and models were preserved.");
    if !std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).any(|p| p == paths.bin)
    {
        println!("Add {} to PATH for this shell.", paths.bin.display());
    }
    Ok(())
}

pub fn update(paths: &Paths, version: Option<&str>, check: bool, force: bool) -> Result<()> {
    let _guard = lock(paths)?;
    require_current_executable(paths)?;
    let installed = current(paths)?;
    let version = match version {
        Some(v) => v.to_string(),
        None if force => installed.release.version.clone(),
        None => crate::network::latest()?,
    };
    let candidate = crate::network::stable_version(&version)?;
    let active = crate::network::stable_version(&installed.release.version)?;
    if candidate < active {
        bail!("Refusing downgrade from {active} to {candidate}");
    }
    if candidate == active && !force {
        println!("swictation {active} is current");
        return Ok(());
    }
    println!("Available: {active} -> {candidate}");
    if check {
        return Ok(());
    }
    let name = format!("swictation-{version}-{}.tar.gz", package::target()?);
    let base = format!(
        "https://github.com/{}/releases/download/v{version}",
        crate::network::REPOSITORY
    );
    let checksum = crate::network::text(&format!("{base}/{name}.sha256"), 1024)?;
    let words: Vec<_> = checksum.split_whitespace().collect();
    if words.len() != 2 || words[1] != name {
        bail!("Invalid release checksum record");
    }
    package::valid_hash(words[0])?;
    let temp = tempfile::Builder::new()
        .prefix(".download-")
        .tempdir_in(&paths.root)?;
    let archive = temp.path().join(&name);
    crate::network::download_bounded(&format!("{base}/{name}"), &archive, package::ARCHIVE_LIMIT)?;
    install_locked(paths, &archive, words[0], false, Some(&version))
}

pub fn rollback(paths: &Paths) -> Result<()> {
    let _guard = lock(paths)?;
    let old = pointer(paths, "current")?.context("No active release")?;
    let target = pointer(paths, "previous")?.context("No retained release to roll back to")?;
    let receipt = package::read_receipt(&paths.root.join(&target))?;
    package::verify_files(&paths.root.join(&target), &receipt)?;
    let running = services::snapshot(paths)?;
    if let Err(error) = services::stop_running(paths, &running) {
        let restore = services::restore(paths, &running);
        bail!("Rollback could not stop services: {error:#}; restoration: {restore:?}");
    }
    let result = (|| -> Result<()> {
        set_pointer(paths, "current", &target)?;
        services::restore(paths, &running)?;
        set_pointer(paths, "previous", &old)
    })();
    if let Err(error) = result {
        let stopping = services::stop_running(paths, &running);
        let recovery = (|| -> Result<()> {
            set_pointer(paths, "current", &old)?;
            set_pointer(paths, "previous", &target)?;
            services::restore(paths, &running)
        })();
        bail!("Rollback failed: {error:#}; recovery stop: {stopping:?}; recovery: {recovery:?}");
    }
    println!("Rolled back to swictation {}", receipt.release.version);
    Ok(())
}

pub fn uninstall(paths: &Paths, yes: bool, purge_config: bool, purge_cache: bool) -> Result<()> {
    println!("Remove command: {}", paths.launcher().display());
    println!(
        "Remove release payloads: {}",
        paths.root.join("releases").display()
    );
    for component in ["daemon", "ui"] {
        println!(
            "Remove owned service: {}",
            services::unit_path(paths, component).display()
        );
    }
    if purge_config {
        println!("Purge configuration: {}", paths.config_file().display());
    }
    if purge_cache {
        for name in ["models", "gpu-libs"] {
            println!("Purge: {}", paths.data.join(name).display());
        }
    }
    if !yes {
        bail!("Uninstall requires --yes. Configuration, models and learned data are preserved unless explicit purge flags are supplied.");
    }
    let _guard = lock(paths)?;
    if !check_launcher(paths)? {
        bail!("Owned launcher missing");
    }
    current(paths)?;
    let pointers = [pointer(paths, "current")?, pointer(paths, "previous")?];
    // Preflight every removable release before touching services or the launcher.
    let releases = paths.root.join("releases");
    for entry in fs::read_dir(&releases)? {
        let entry = entry?;
        paths::secure_dir(&entry.path())?;
        package::read_receipt(&entry.path())?;
    }
    if purge_config && fs::symlink_metadata(paths.config_file()).is_ok() {
        paths::regular_owned(&paths.config_file())?;
    }
    if purge_cache {
        for name in ["models", "gpu-libs"] {
            let dir = paths.data.join(name);
            if fs::symlink_metadata(&dir).is_ok() {
                paths::secure_dir(&dir)?;
                reject_tree_links(&dir)?;
            }
        }
    }
    services::uninstall(paths)?;
    fs::remove_file(paths.launcher())?;
    for (name, target) in ["current", "previous"].into_iter().zip(pointers) {
        if target.is_some() {
            fs::remove_file(paths.root.join(name))?;
        }
    }
    fs::remove_dir_all(releases)?;
    // Keep the lock inode/owner directory so waiting processes cannot acquire a stale lock.
    if purge_config {
        let file = paths.config_file();
        if file.exists() {
            paths::regular_owned(&file)?;
            fs::remove_file(file)?;
        }
    }
    if purge_cache {
        for name in ["models", "gpu-libs"] {
            let dir = paths.data.join(name);
            if dir.exists() {
                paths::secure_dir(&dir)?;
                reject_tree_links(&dir)?;
                fs::remove_dir_all(dir)?;
            }
        }
    }
    println!("Swictation uninstalled. Learned data and logs preserved; config/models preserved unless explicitly purged.");
    Ok(())
}

fn reject_tree_links(dir: &Path) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            bail!(
                "Refusing to purge tree containing symlink: {}",
                entry.path().display()
            );
        }
        if kind.is_dir() {
            reject_tree_links(&entry.path())?;
        } else {
            paths::regular_owned(&entry.path())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod lock_tests {
    use super::*;

    #[test]
    fn releasing_guard_unlocks_inherited_file_descriptions() {
        let temporary = tempfile::tempdir().unwrap();
        let home = fs::canonicalize(temporary.path()).unwrap();
        let paths = Paths {
            root: home.join("install"),
            data: home.join("data"),
            config: home.join("config"),
            bin: home.join("bin"),
            home,
        };
        let guard = lock(&paths).unwrap();
        // dup and fork share the same open file description. A child between
        // fork and exec must not extend the completed operation's lock lifetime.
        let inherited = guard._file.try_clone().unwrap();
        assert!(lock(&paths).is_err());
        drop(guard);
        let next = lock(&paths).expect("completed operation must release its lock");
        drop(inherited);
        assert!(
            lock(&paths).is_err(),
            "the next operation still owns its lock"
        );
        drop(next);
    }
}
