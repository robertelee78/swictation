use super::{apps, MARKER};
use crate::paths::Paths;
use anyhow::{bail, Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub fn ort_library(paths: &Paths) -> Result<PathBuf> {
    let name = if cfg!(target_os = "macos") {
        "libonnxruntime.dylib"
    } else {
        "libonnxruntime.so"
    };
    let mut candidates = vec![paths.current().join("lib").join(name)];
    if cfg!(target_os = "linux") {
        candidates.insert(0, paths.data.join("gpu-libs").join(name));
    }
    candidates
        .into_iter()
        .find(|p| {
            fs::metadata(p)
                .map(|m| m.is_file() && m.len() > 0)
                .unwrap_or(false)
        })
        .context("ONNX Runtime missing; reinstall Swictation or run swictation setup --gpu-libs")
}

pub fn library_path(paths: &Paths) -> String {
    let mut dirs = vec![paths.data.join("gpu-libs"), paths.current().join("lib")];
    for directory in [
        "/usr/local/cuda/lib64",
        "/usr/local/cuda/lib",
        "/usr/local/cuda-13/lib64",
        "/usr/local/cuda-12.9/lib64",
        "/usr/local/cuda-12/lib64",
    ] {
        if Path::new(directory).is_dir() {
            dirs.push(PathBuf::from(directory));
        }
    }
    dirs.into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(":")
}

pub(super) fn systemd_quote(value: &str) -> Result<String> {
    Ok(environment_quote(value)?.replace('$', "$$"))
}

// Environment= expands specifiers, but does not expand dollar variables.
fn environment_quote(value: &str) -> Result<String> {
    if value.contains(['\n', '\r', '\0']) {
        bail!("newline or NUL in service path");
    }
    Ok(format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('%', "%%")
    ))
}

pub(super) fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn wlroots() -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_lowercase();
    if std::env::var_os("SWAYSOCK").is_some()
        || std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
        || ["sway", "hyprland", "river"]
            .iter()
            .any(|name| desktop.contains(name))
    {
        return true;
    }
    let uid = unsafe { libc::geteuid() }.to_string();
    ["sway", "Hyprland", "river"].iter().any(|name| {
        Command::new("pgrep")
            .args(["-u", &uid, "-x", name])
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    })
}

pub(super) fn render(paths: &Paths, component: &str) -> Result<String> {
    let ort = ort_library(paths)?;
    let binary = paths
        .current()
        .join("bin")
        .join(format!("swictation-{component}"));
    if !binary.is_file() {
        bail!("{} missing from installed release", binary.display());
    }
    if cfg!(target_os = "macos") {
        let app = apps::binary(paths, component);
        let log = paths.home.join("Library/Logs/swictation");
        return Ok(format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<!-- {MARKER} -->\n<plist version=\"1.0\"><dict>\n<key>Label</key><string>com.swictation.{component}</string>\n<key>ProgramArguments</key><array><string>{}</string></array>\n<key>RunAtLoad</key><true/>\n<key>KeepAlive</key><dict><key>SuccessfulExit</key><false/></dict>\n<key>ThrottleInterval</key><integer>5</integer>\n<key>LimitLoadToSessionType</key><string>Aqua</string>\n<key>EnvironmentVariables</key><dict>\n<key>HOME</key><string>{}</string>\n<key>PATH</key><string>{}:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin</string>\n<key>RUST_LOG</key><string>info</string>\n<key>ORT_DYLIB_PATH</key><string>{}</string>\n</dict>\n<key>StandardOutPath</key><string>{}</string>\n<key>StandardErrorPath</key><string>{}</string>\n</dict></plist>\n", xml(&app.to_string_lossy()), xml(&paths.home.to_string_lossy()), xml(&paths.bin.to_string_lossy()), xml(&ort.to_string_lossy()), xml(&log.join(format!("{component}.log")).to_string_lossy()), xml(&log.join(format!("{component}-error.log")).to_string_lossy())));
    }
    let dependencies = if component == "ui" {
        "After=swictation-daemon.service\nRequires=swictation-daemon.service\nPartOf=swictation-daemon.service\nBindsTo=swictation-daemon.service"
    } else {
        "After=graphical-session.target"
    };
    let tray = component == "ui" && wlroots();
    let exec = if tray {
        let script = paths.current().join("share/swictation_tray.py");
        if !script.is_file() {
            bail!("wlroots tray is missing from installed release");
        }
        format!(
            "/usr/bin/python3 {}",
            systemd_quote(&script.to_string_lossy())?
        )
    } else {
        systemd_quote(&binary.to_string_lossy())?
    };
    let mut unit = format!("# {MARKER}\n[Unit]\nDescription=Swictation {component}\n{dependencies}\n\n[Service]\nType=simple\nExecStart={exec}\nRestart=on-failure\nRestartSec=5\nEnvironment=\"RUST_LOG=info\"\nEnvironment={}\nEnvironment={}\nImportEnvironment=PATH XDG_RUNTIME_DIR DISPLAY WAYLAND_DISPLAY DBUS_SESSION_BUS_ADDRESS\n", environment_quote(&format!("ORT_DYLIB_PATH={}", ort.display()))?, environment_quote(&format!("LD_LIBRARY_PATH={}", library_path(paths)))?);
    if tray {
        unit.push_str(&format!(
            "Environment=\"QT_QPA_PLATFORM=wayland\"\nEnvironment={}\n",
            environment_quote(&format!("SWICTATION_UI_BINARY={}", binary.display()))?
        ));
    }
    for key in [
        "DISPLAY",
        "WAYLAND_DISPLAY",
        "XDG_RUNTIME_DIR",
        "DBUS_SESSION_BUS_ADDRESS",
    ] {
        if let Ok(value) = std::env::var(key) {
            unit.push_str(&format!(
                "Environment={}\n",
                environment_quote(&format!("{key}={value}"))?
            ));
        }
    }
    unit.push_str("\n[Install]\nWantedBy=default.target\n");
    Ok(unit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_paths_preserve_literal_dollars_and_escape_specifiers() {
        assert_eq!(
            systemd_quote("/home/$user/100%/binary").unwrap(),
            "\"/home/$$user/100%%/binary\""
        );
        assert_eq!(
            environment_quote("PATH=/home/$user/100%").unwrap(),
            "\"PATH=/home/$user/100%%\""
        );
        assert!(environment_quote("PATH=/home/user\nExecStart=evil").is_err());
    }
}
