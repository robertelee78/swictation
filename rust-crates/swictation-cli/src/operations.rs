//! Operations use the same sockets and libraries as the daemon.
use crate::{configuration, paths::Paths, services};
use anyhow::{bail, Context, Result};
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
    time::Duration,
};

pub fn toggle(paths: &Paths) -> Result<()> {
    let mut socket =
        std::os::unix::net::UnixStream::connect(configuration::configured_socket(paths)?)
            .context("cannot connect to daemon; run swictation start")?;
    socket.set_read_timeout(Some(Duration::from_secs(15)))?;
    socket.set_write_timeout(Some(Duration::from_secs(5)))?;
    socket.write_all(b"{\"action\":\"toggle\"}")?;
    let mut bytes = Vec::new();
    let mut chunk = [0; 4096];
    loop {
        let n = socket.read(&mut chunk)?;
        if n == 0 {
            bail!("daemon closed connection before sending a complete response");
        }
        bytes.extend_from_slice(&chunk[..n]);
        if bytes.len() > 1024 * 1024 {
            bail!("daemon response exceeds 1 MiB");
        }
        match serde_json::from_slice::<serde_json::Value>(&bytes) {
            Ok(response) => {
                if response.get("status").and_then(|v| v.as_str()) != Some("success") {
                    bail!(
                        "daemon rejected toggle: {}",
                        response.get("error").unwrap_or(&response)
                    );
                }
                println!(
                    "{}",
                    response
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Recording toggled")
                );
                return Ok(());
            }
            Err(error) if error.is_eof() => continue,
            Err(error) => return Err(error).context("invalid daemon response"),
        }
    }
}

pub fn logs(paths: &Paths, follow: bool) -> Result<()> {
    let mut command = if cfg!(target_os = "macos") {
        let mut command = Command::new("tail");
        command.args(["-n", "100"]);
        if follow {
            command.arg("-f");
        }
        command.arg(paths.home.join("Library/Logs/swictation/daemon.log"));
        command.arg(paths.home.join("Library/Logs/swictation/daemon-error.log"));
        command
    } else {
        let mut command = Command::new("journalctl");
        command.args([
            "--user",
            "-u",
            "swictation-daemon.service",
            "-u",
            "swictation-ui.service",
            "-n",
            "100",
            "--no-pager",
        ]);
        if follow {
            command.arg("--follow");
        }
        command
    };
    if !command.status()?.success() {
        bail!("could not read Swictation logs");
    }
    Ok(())
}

pub fn ui(paths: &Paths) -> Result<()> {
    let mut command = Command::new(paths.current().join("bin/swictation-ui"));
    command.env("ORT_DYLIB_PATH", services::ort_library(paths)?);
    if cfg!(target_os = "linux") {
        command.env("LD_LIBRARY_PATH", services::library_path(paths));
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("cannot launch Swictation UI")?;
    Ok(())
}

pub fn config(paths: &Paths, edit: bool) -> Result<()> {
    if !edit {
        println!("{}", paths.config_file().display());
        return Ok(());
    }
    let editor = std::env::var_os("VISUAL")
        .or_else(|| std::env::var_os("EDITOR"))
        .unwrap_or_else(|| "vi".into());
    // Treat the editor as an executable, never as shell code.
    let status = Command::new(PathBuf::from(editor))
        .arg(paths.config_file())
        .status()
        .context("cannot launch editor; set EDITOR to an executable path")?;
    if !status.success() {
        bail!("editor exited unsuccessfully");
    }
    Ok(())
}

pub fn platform_check() -> Result<()> {
    if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        let output = Command::new("ldd")
            .arg("--version")
            .output()
            .context("cannot inspect glibc")?;
        if !output.status.success() {
            bail!("cannot inspect glibc version");
        }
        let first = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .to_string();
        let version = first
            .split_whitespace()
            .rev()
            .find_map(|part| {
                let mut parts = part.split('.');
                Some((
                    parts.next()?.parse::<u32>().ok()?,
                    parts.next()?.parse::<u32>().ok()?,
                ))
            })
            .context("cannot determine glibc version")?;
        if version < (2, 39) {
            bail!("Swictation requires glibc 2.39 or newer");
        }
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        let version = Command::new("sw_vers").arg("-productVersion").output()?;
        let major: u32 = String::from_utf8(version.stdout)?
            .trim()
            .split('.')
            .next()
            .unwrap_or("")
            .parse()?;
        if major < 14 {
            bail!("Swictation requires macOS 14 or newer");
        }
        let memory = Command::new("sysctl").args(["-n", "hw.memsize"]).output()?;
        let bytes: u64 = String::from_utf8(memory.stdout)?.trim().parse()?;
        if bytes / 1024 / 1024 < 14500 {
            bail!("Swictation requires 16 GB unified memory on macOS");
        }
    } else {
        bail!("supported platforms: Linux x86_64 and macOS Apple Silicon");
    }
    Ok(())
}

fn on_path(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|v| std::env::split_paths(&v).any(|dir| dir.join(name).is_file()))
        .unwrap_or(false)
}

pub fn integration_check() -> Result<()> {
    if cfg!(target_os = "macos") {
        // Accessibility and microphone consent can only be granted by the user;
        // filesystem presence is not evidence that macOS has granted permission.
        bail!("grant Microphone and Accessibility permissions to SwictationDaemon in System Settings → Privacy & Security; permission state is not probed");
    }
    if services::wlroots() {
        let qt = Command::new("/usr/bin/python3")
            .args(["-c", "import PySide6.QtWidgets"])
            .output();
        if !qt.map(|out| out.status.success()).unwrap_or(false) {
            bail!("the wlroots tray requires Python 3 and PySide6; install them with your distribution package manager");
        }
    }
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland");
    if wayland {
        let gnome = std::env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .to_lowercase()
            .contains("gnome");
        let injector = if gnome { "ydotool" } else { "wtype" };
        if !on_path(injector) {
            bail!("install {injector} using your distribution package manager for Wayland text injection");
        }
        if gnome
            && !std::env::var_os("YDOTOOL_SOCKET")
                .map(PathBuf::from)
                .map(|p| p.exists())
                .unwrap_or_else(|| PathBuf::from("/tmp/.ydotool_socket").exists())
        {
            bail!("ydotool socket is unavailable; configure the distribution's ydotool daemon");
        }
    } else if !on_path("xdotool") {
        bail!("install xdotool using your distribution package manager for X11 text injection");
    }
    if !on_path("pipewire") && !on_path("pulseaudio") {
        bail!("PipeWire or PulseAudio is required for audio capture");
    }
    Ok(())
}

pub fn integration_instructions(paths: &Paths) -> Result<()> {
    if cfg!(target_os = "macos") {
        eprintln!("Grant Microphone and Accessibility permissions to {}/SwictationDaemon.app in System Settings → Privacy & Security. Default hotkey: Ctrl+Shift+D.", paths.root.display());
    } else {
        eprintln!("Default hotkey: Super+Shift+D. On GNOME Wayland, add a custom keyboard shortcut running {} toggle. On Sway: bindsym Mod4+Shift+d exec {} toggle", paths.bin.join("swictation").display(), paths.bin.join("swictation").display());
        if fs::read_dir("/sys/class/power_supply")
            .map(|entries| {
                entries
                    .flatten()
                    .any(|entry| entry.file_name().to_string_lossy().starts_with("BAT"))
            })
            .unwrap_or(false)
            && on_path("nvidia-smi")
        {
            eprintln!("NVIDIA laptop: check that your distribution configures NVreg_PreserveVideoMemoryAllocations=1 and NVIDIA suspend/resume services before using hibernation.");
        }
    }
    Ok(())
}
