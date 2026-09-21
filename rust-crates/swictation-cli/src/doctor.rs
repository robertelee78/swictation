//! Read-only artifact diagnostics; receipts alone never prove an install works.
use crate::{configuration, gpu, models, operations, package, paths::Paths, services};
use anyhow::Result;
use serde::Serialize;
use std::fs;

#[derive(Debug, Serialize)]
pub struct Check {
    pub id: String,
    pub state: String,
    pub summary: String,
    pub repair: String,
}

fn result(id: &str, outcome: Result<()>, healthy: &str) -> Check {
    match outcome {
        Ok(()) => Check {
            id: id.into(),
            state: "healthy".into(),
            summary: healthy.into(),
            repair: String::new(),
        },
        Err(error) => Check {
            id: id.into(),
            state: "unhealthy".into(),
            summary: format!("{error:#}"),
            repair: format!("swictation setup --{id}"),
        },
    }
}

pub fn inspect(paths: &Paths, deep: bool) -> Vec<Check> {
    let mut checks = vec![result(
        "platform",
        operations::platform_check(),
        "supported operating system and architecture",
    )];
    let binaries = ["swictation-daemon", "swictation-ui"]
        .into_iter()
        .try_for_each(|name| {
            let file = paths.current().join("bin").join(name);
            let metadata = fs::metadata(&file)?;
            use std::os::unix::fs::PermissionsExt;
            anyhow::ensure!(
                metadata.is_file()
                    && metadata.len() > 0
                    && metadata.permissions().mode() & 0o111 != 0,
                "{} is missing or not executable",
                file.display()
            );
            Ok(())
        })
        .and_then(|_| {
            if deep {
                package::verify_files(&paths.current(), &package::read_receipt(&paths.current())?)
            } else {
                Ok(())
            }
        });
    checks.push(result(
        "binaries",
        binaries,
        "daemon and UI executables present",
    ));
    checks.last_mut().unwrap().repair = if checks.last().unwrap().state == "healthy" {
        String::new()
    } else {
        "swictation update --force".into()
    };
    checks.push(result(
        "config",
        configuration::check(paths),
        "valid user configuration",
    ));
    checks.push(match gpu::variant() {
        Ok(Some(variant)) => result(
            "gpu-libs",
            gpu::check(paths, &variant, deep),
            if deep {
                "GPU library content hashes match"
            } else {
                "GPU library inventory and sizes match"
            },
        ),
        Ok(None) => Check {
            id: "gpu-libs".into(),
            state: "not-applicable".into(),
            summary: "no NVIDIA library bundle required".into(),
            repair: String::new(),
        },
        Err(error) => result("gpu-libs", Err(error), ""),
    });
    checks.push(result(
        "models",
        models::selected_model(paths).and_then(|selection| models::check(paths, &selection, deep)),
        if deep {
            "selected model content hashes match"
        } else {
            "selected model inventory and sizes match"
        },
    ));
    checks.push(result(
        "services",
        services::check(paths),
        "native service paths and ONNX Runtime configured",
    ));
    let mut integration = result(
        "integration",
        operations::integration_check(),
        "audio and text injection tools present",
    );
    if cfg!(target_os = "macos") {
        integration.state = "unknown".into();
        integration.repair =
            "System Settings → Privacy & Security → Microphone and Accessibility".into();
    }
    checks.push(integration);
    checks
}

pub fn run(paths: &Paths, deep: bool, json: bool) -> Result<bool> {
    let checks = inspect(paths, deep);
    let healthy = checks
        .iter()
        .all(|check| matches!(check.state.as_str(), "healthy" | "not-applicable"));
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({ "schemaVersion": 1, "tool": "swictation doctor", "deep": deep, "healthy": healthy, "platform": std::env::consts::OS, "architecture": std::env::consts::ARCH, "checks": checks })
            )?
        );
    } else {
        println!(
            "Swictation doctor ({} {}, {})",
            std::env::consts::OS,
            std::env::consts::ARCH,
            if deep {
                "content hashes"
            } else {
                "sizes and inventory"
            }
        );
        for check in checks {
            println!("{} {}: {}", check.state, check.id, check.summary);
            if !check.repair.is_empty() {
                println!("  Repair: {}", check.repair);
            }
        }
    }
    Ok(healthy)
}
