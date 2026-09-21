use anyhow::{bail, Context, Result};
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct Paths {
    pub home: PathBuf,
    pub data: PathBuf,
    pub config: PathBuf,
    pub bin: PathBuf,
    pub root: PathBuf,
}

impl Paths {
    pub fn discover() -> Result<Self> {
        let home = dirs::home_dir().context("Cannot determine home directory")?;
        let data = dirs::data_dir()
            .context("Cannot determine data directory")?
            .join("swictation");
        let config = if cfg!(target_os = "macos") {
            data.clone()
        } else {
            dirs::config_dir()
                .context("Cannot determine config directory")?
                .join("swictation")
        };
        Ok(Self {
            bin: home.join(".local/bin"),
            root: data.join("install"),
            home,
            data,
            config,
        })
    }
    pub fn current(&self) -> PathBuf {
        self.root.join("current")
    }
    pub fn models(&self) -> PathBuf {
        self.data.join("models")
    }
    pub fn config_file(&self) -> PathBuf {
        self.config.join("config.toml")
    }
    pub fn launcher(&self) -> PathBuf {
        self.bin.join("swictation")
    }
}

pub fn safe_relative(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        bail!("Unsafe relative path: {}", path.display());
    }
    Ok(())
}

/// Reject links and writable-by-other-users directories on the entire path.
/// System-owned ancestors are allowed; newly created directories are owner-only.
pub fn secure_dir(path: &Path) -> Result<()> {
    if !path.is_absolute() {
        bail!("Expected absolute directory: {}", path.display());
    }
    let mut cursor = PathBuf::new();
    for part in path.components() {
        if matches!(part, Component::ParentDir | Component::CurDir) {
            bail!("Noncanonical directory");
        }
        cursor.push(part);
        match fs::symlink_metadata(&cursor) {
            Ok(m) => {
                if !m.is_dir() || m.file_type().is_symlink() {
                    bail!("Unsafe directory: {}", cursor.display());
                }
                // /tmp is permitted as an ancestor for isolated fixture/source builds.
                if m.mode() & 0o022 != 0 && m.mode() & 0o1000 == 0 {
                    bail!("Directory is writable by others: {}", cursor.display());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&cursor)?;
                fs::set_permissions(&cursor, fs::Permissions::from_mode(0o700))?;
            }
            Err(e) => return Err(e.into()),
        }
    }
    let m = fs::metadata(path)?;
    if m.uid() != unsafe { libc::geteuid() } {
        bail!("Directory is not owned by current user: {}", path.display());
    }
    Ok(())
}

pub fn regular_owned(path: &Path) -> Result<fs::Metadata> {
    let m = fs::symlink_metadata(path).with_context(|| format!("Read {}", path.display()))?;
    if !m.is_file()
        || m.file_type().is_symlink()
        || m.nlink() != 1
        || m.uid() != unsafe { libc::geteuid() }
        || m.mode() & 0o022 != 0
    {
        bail!("Not a safe owned regular file: {}", path.display());
    }
    Ok(m)
}

pub fn sync_dir(path: &Path) -> Result<()> {
    fs::File::open(path)?.sync_all().context("Sync directory")
}
