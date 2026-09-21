//! User service ownership, stable native paths, and lifecycle state restoration.
#[path = "service_apps.rs"]
mod apps;
#[path = "service_units.rs"]
mod units;
use crate::{models, paths::Paths};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output},
};
pub use units::{library_path, ort_library, wlroots};
use units::{render, systemd_quote, xml};

const MARKER: &str = "Managed by Swictation native installer";
#[derive(Debug, Default, Clone, Serialize)]
pub struct RunningState {
    pub daemon: bool,
    pub ui: bool,
    pub daemon_loaded: bool,
    pub ui_loaded: bool,
}

pub fn unit_path(paths: &Paths, component: &str) -> PathBuf {
    if cfg!(target_os = "macos") {
        paths
            .home
            .join("Library/LaunchAgents")
            .join(format!("com.swictation.{component}.plist"))
    } else {
        paths
            .home
            .join(".config/systemd/user")
            .join(format!("swictation-{component}.service"))
    }
}

pub fn owned(paths: &Paths, component: &str) -> Result<bool> {
    let path = unit_path(paths, component);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() {
        bail!("refusing to replace symlinked service {}", path.display());
    }
    let text = fs::read_to_string(&path)?;
    let legacy_binary = format!("swictation-{component}");
    let legacy_app = if component == "daemon" {
        "SwictationDaemon.app"
    } else {
        "SwictationUI.app"
    };
    let is_owned = text.contains(MARKER)
        || ((text.contains(&legacy_binary)
            || (component == "ui" && text.contains("swictation_tray.py")))
            && text.contains("node_modules"))
        || (text.contains(legacy_app) && text.contains("swictation"));
    if !is_owned {
        bail!(
            "service {} is not owned by this installer; preserve or move it before setup",
            path.display()
        );
    }
    Ok(true)
}

fn run(program: &str, args: &[&str]) -> Result<Output> {
    Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("cannot execute {program}"))
}

fn checked(program: &str, args: &[&str]) -> Result<()> {
    let output = run(program, args)?;
    if !output.status.success() {
        bail!(
            "{program} {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

fn domain() -> Result<String> {
    let output = run("id", &["-u"])?;
    if !output.status.success() {
        bail!("cannot determine current uid");
    }
    let uid = String::from_utf8(output.stdout)?.trim().to_string();
    if !uid.bytes().all(|b| b.is_ascii_digit()) {
        bail!("invalid user id");
    }
    Ok(format!("gui/{uid}"))
}

fn running(paths: &Paths, component: &str) -> Result<bool> {
    if !owned(paths, component)? {
        return Ok(false);
    }
    if cfg!(target_os = "macos") {
        let output = run(
            "launchctl",
            &[
                "print",
                &format!("{}/com.swictation.{component}", domain()?),
            ],
        )?;
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            if error.contains("Could not find service") || error.contains("could not find service")
            {
                return Ok(false);
            }
            bail!("cannot inspect launchd service: {error}");
        }
        return Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line.trim() == "state = running"));
    }
    let name = format!("swictation-{component}.service");
    let output = run(
        "systemctl",
        &["--user", "show", &name, "--property=ActiveState", "--value"],
    )?;
    if !output.status.success() {
        bail!(
            "cannot inspect user service: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(matches!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "active" | "activating" | "reloading"
    ))
}

pub fn snapshot(paths: &Paths) -> Result<RunningState> {
    let mut state = RunningState {
        daemon: running(paths, "daemon")?,
        ui: running(paths, "ui")?,
        ..RunningState::default()
    };
    if cfg!(target_os = "macos") {
        for (component, loaded) in [
            ("daemon", &mut state.daemon_loaded),
            ("ui", &mut state.ui_loaded),
        ] {
            if owned(paths, component)? {
                *loaded = run(
                    "launchctl",
                    &[
                        "print",
                        &format!("{}/com.swictation.{component}", domain()?),
                    ],
                )?
                .status
                .success();
            }
        }
    }
    Ok(state)
}

fn stop_component(paths: &Paths, component: &str) -> Result<()> {
    if !owned(paths, component)? {
        return Ok(());
    }
    if cfg!(target_os = "macos") {
        let name = format!("{}/com.swictation.{component}", domain()?);
        let loaded = run("launchctl", &["print", &name])?;
        if loaded.status.success() {
            checked("launchctl", &["bootout", &name])?;
        } else {
            let message = String::from_utf8_lossy(&loaded.stderr);
            if !message.contains("Could not find service")
                && !message.contains("could not find service")
            {
                bail!("cannot inspect launchd service: {message}");
            }
        }
    } else {
        checked(
            "systemctl",
            &["--user", "stop", &format!("swictation-{component}.service")],
        )?;
    }
    Ok(())
}

pub fn stop_running(paths: &Paths, state: &RunningState) -> Result<()> {
    let stop = || -> Result<()> {
        if state.ui || state.ui_loaded {
            stop_component(paths, "ui")?;
        }
        if state.daemon || state.daemon_loaded {
            stop_component(paths, "daemon")?;
        }
        Ok(())
    };
    match stop() {
        Ok(()) => Ok(()),
        Err(error) => {
            restore(paths, state)
                .context("stopping services failed and prior state could not be restored")?;
            Err(error)
        }
    }
}

pub fn stop(paths: &Paths) -> Result<()> {
    stop_component(paths, "ui")?;
    stop_component(paths, "daemon")
}

fn launch(paths: &Paths, component: &str) -> Result<()> {
    if !owned(paths, component)? {
        bail!("{component} service missing; run swictation setup --services");
    }
    if cfg!(target_os = "macos") {
        let domain = domain()?;
        let name = format!("{domain}/com.swictation.{component}");
        let loaded = run("launchctl", &["print", &name])?.status.success();
        if loaded && !running(paths, component)? {
            checked("launchctl", &["bootout", &name])?;
        }
        if !loaded || !running(paths, component)? {
            checked(
                "launchctl",
                &[
                    "bootstrap",
                    &domain,
                    &unit_path(paths, component).to_string_lossy(),
                ],
            )?;
        }
        checked("launchctl", &["kickstart", &name])
    } else {
        checked(
            "systemctl",
            &[
                "--user",
                "start",
                &format!("swictation-{component}.service"),
            ],
        )
    }
}

pub fn start(paths: &Paths, with_ui: bool) -> Result<()> {
    if cfg!(target_os = "macos") {
        apps::refresh(paths)?;
    }
    launch(paths, "daemon")?;
    if with_ui {
        launch(paths, "ui")?;
    }
    Ok(())
}

pub fn restore(paths: &Paths, state: &RunningState) -> Result<()> {
    if cfg!(target_os = "macos")
        && [
            ("daemon", state.daemon || state.daemon_loaded),
            ("ui", state.ui || state.ui_loaded),
        ]
        .into_iter()
        .any(|(component, restore)| {
            restore
                && fs::read_to_string(unit_path(paths, component))
                    .map(|text| text.contains(MARKER))
                    .unwrap_or(false)
        })
    {
        apps::refresh(paths)?;
    }
    if state.daemon {
        launch(paths, "daemon")?;
    } else if state.daemon_loaded {
        restore_idle(paths, "daemon")?;
    }
    if state.ui {
        launch(paths, "ui")?;
    } else if state.ui_loaded {
        restore_idle(paths, "ui")?;
    }
    Ok(())
}

fn restore_idle(paths: &Paths, component: &str) -> Result<()> {
    // Restore idle registration; explicit start reloads the canonical policy.
    let source = fs::read_to_string(unit_path(paths, component))?;
    let idle = source
        .replace(
            "<key>RunAtLoad</key><true/>",
            "<key>RunAtLoad</key><false/>",
        )
        .replace(
            "<key>KeepAlive</key><dict><key>SuccessfulExit</key><false/></dict>",
            "<key>KeepAlive</key><false/>",
        );
    if source == idle {
        bail!("cannot safely restore idle legacy LaunchAgent; run swictation setup --services");
    }
    let mut file = tempfile::NamedTempFile::new_in(&paths.root)?;
    file.write_all(idle.as_bytes())?;
    checked(
        "launchctl",
        &["bootstrap", &domain()?, &file.path().to_string_lossy()],
    )
}

pub fn status(paths: &Paths) -> Result<()> {
    let state = snapshot(paths)?;
    println!(
        "Daemon: {}\nUI: {}",
        if state.daemon { "running" } else { "stopped" },
        if state.ui { "running" } else { "stopped" }
    );
    Ok(())
}

fn atomic_write(destination: &Path, content: &[u8]) -> Result<()> {
    let parent = destination.parent().context("destination has no parent")?;
    crate::paths::secure_dir(parent)?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    staged.write_all(content)?;
    staged.as_file().sync_all()?;
    staged.persist(destination).map_err(|e| e.error)?;
    Ok(())
}

pub fn check(paths: &Paths) -> Result<()> {
    for component in ["daemon", "ui"] {
        if !owned(paths, component)? {
            bail!("{component} service missing");
        }
        let text = fs::read_to_string(unit_path(paths, component))?;
        let expected = render(paths, component)?;
        // Session environment is allowed to change between setup and doctor.
        let binary = if cfg!(target_os = "macos") {
            apps::binary(paths, component)
        } else {
            paths
                .current()
                .join("bin")
                .join(format!("swictation-{component}"))
        };
        let encoded_binary = if cfg!(target_os = "macos") {
            xml(&binary.to_string_lossy())
        } else {
            systemd_quote(&binary.to_string_lossy())?
        };
        let current_tray = systemd_quote(
            &paths
                .current()
                .join("share/swictation_tray.py")
                .to_string_lossy(),
        )?;
        let has_binary =
            text.contains(&encoded_binary) || (component == "ui" && text.contains(&current_tray));
        if !text.contains(MARKER) || !has_binary {
            bail!("{component} service uses a legacy or stale binary path; run swictation setup --services");
        }
        let ort = ort_library(paths)?;
        if !text.contains(&ort.to_string_lossy().replace('&', "&amp;"))
            || !expected.contains("ORT_DYLIB_PATH")
        {
            bail!("{component} service has stale ONNX Runtime path");
        }
    }
    Ok(())
}

pub fn configure(paths: &Paths) -> Result<()> {
    // Validate every target before mutating any service.
    for component in ["daemon", "ui"] {
        owned(paths, component)?;
        render(paths, component)?;
    }
    if cfg!(target_os = "macos") {
        crate::paths::secure_dir(&paths.home.join("Library/Logs/swictation"))?;
        apps::refresh(paths)?;
    }
    let mut receipt = serde_json::Map::new();
    for component in ["daemon", "ui"] {
        let path = unit_path(paths, component);
        let text = render(paths, component)?;
        if fs::read_to_string(&path).ok().as_deref() != Some(&text) {
            if path.exists() {
                let backup = path.with_extension(format!(
                    "backup.{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_nanos()
                ));
                fs::copy(&path, backup)?;
            }
            atomic_write(&path, text.as_bytes())?;
        }
        receipt.insert(
            component.into(),
            serde_json::Value::String(models::checksum(&path)?),
        );
    }
    atomic_write(
        &paths.root.join("service-manifest.json"),
        &serde_json::to_vec_pretty(&receipt)?,
    )?;
    if cfg!(target_os = "linux") {
        checked("systemctl", &["--user", "daemon-reload"])?;
    }
    check(paths)
}

pub fn enable(paths: &Paths) -> Result<()> {
    if cfg!(target_os = "linux") {
        checked(
            "systemctl",
            &[
                "--user",
                "enable",
                "swictation-daemon.service",
                "swictation-ui.service",
            ],
        )?;
    }
    start(paths, true)
}

pub fn uninstall(paths: &Paths) -> Result<()> {
    apps::validate_remove(paths)?;
    let receipt: serde_json::Value = fs::read(paths.root.join("service-manifest.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or(serde_json::Value::Null);
    let mut found = false;
    // Preflight content hashes before stopping or removing any service.
    for component in ["daemon", "ui"] {
        if !owned(paths, component)? {
            continue;
        }
        found = true;
        let digest = models::checksum(&unit_path(paths, component))?;
        if receipt.get(component).and_then(|value| value.as_str()) != Some(&digest) {
            bail!(
                "preserved modified or unrecorded service {}; move it aside before uninstalling",
                unit_path(paths, component).display()
            );
        }
    }
    if !found {
        return apps::uninstall(paths);
    }
    let state = snapshot(paths)?;
    stop_running(paths, &state)?;
    for component in ["daemon", "ui"] {
        let file = unit_path(paths, component);
        if file.exists() {
            if cfg!(target_os = "linux") {
                checked(
                    "systemctl",
                    &[
                        "--user",
                        "disable",
                        &format!("swictation-{component}.service"),
                    ],
                )?;
            }
            fs::remove_file(file)?;
        }
    }
    if cfg!(target_os = "linux") {
        checked("systemctl", &["--user", "daemon-reload"])?;
    }
    apps::uninstall(paths)
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
