//! One explicit setup path for first install, migration, and targeted repair.
use crate::{configuration, doctor, gpu, models, operations, paths::Paths, services};
use anyhow::{bail, Context, Result};
use std::process::Command;

#[derive(Debug, Default)]
pub struct Options {
    pub repair: bool,
    pub only: Vec<String>,
    pub offline: bool,
    pub start: bool,
}

pub const STEPS: &[&str] = &[
    "platform",
    "binaries",
    "config",
    "gpu-libs",
    "models",
    "services",
    "integration",
    "verify",
];

pub fn list() {
    for step in STEPS {
        println!("{step}\tswictation setup --{step}");
    }
}

fn verify(paths: &Paths) -> Result<()> {
    configuration::check(paths)?;
    let selection = models::selected_model(paths)?;
    models::check(paths, &selection, false)?;
    let mut command = Command::new(paths.current().join("bin/swictation-daemon"));
    command
        .arg("--dry-run")
        .env("ORT_DYLIB_PATH", services::ort_library(paths)?);
    if cfg!(target_os = "linux") {
        command.env("LD_LIBRARY_PATH", services::library_path(paths));
    }
    let output = command
        .output()
        .context("could not run daemon verification")?;
    if !output.status.success() {
        bail!(
            "daemon dry-run failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

pub fn run(paths: &Paths, options: &Options) -> Result<()> {
    // Every direct setup command owns the same installation lock as update.
    let _lock = crate::lifecycle::lock(paths)?;
    run_locked(paths, options)
}

pub fn run_locked(paths: &Paths, options: &Options) -> Result<()> {
    if std::env::var_os("SUDO_USER").is_some() {
        bail!("run setup as the desktop user, without sudo");
    }
    operations::platform_check()?;
    for id in &options.only {
        if !STEPS.contains(&id.as_str()) && !["config-reset", "config-heal"].contains(&id.as_str())
        {
            bail!("unknown setup step {id}; use swictation setup --list");
        }
    }
    crate::paths::secure_dir(&paths.data)?;
    crate::paths::secure_dir(&paths.config)?;
    let health = if options.repair {
        doctor::inspect(paths, true)
    } else {
        Vec::new()
    };
    let mut selected: Vec<_> = STEPS
        .iter()
        .copied()
        .filter(|id| {
            let requested = options.only.is_empty()
                || options.only.iter().any(|value| {
                    value == id
                        || (*id == "config"
                            && matches!(value.as_str(), "config-reset" | "config-heal"))
                });
            requested
                && (!options.repair
                    || health.iter().any(|check| {
                        check.id == *id
                            && check.state != "healthy"
                            && check.state != "not-applicable"
                    }))
        })
        .collect();
    if options.repair && !selected.is_empty() && !selected.contains(&"verify") {
        selected.push("verify");
    }
    let affects_running = selected
        .iter()
        .any(|id| ["gpu-libs", "models", "services", "config"].contains(id));
    let state = if affects_running {
        services::snapshot(paths)?
    } else {
        services::RunningState::default()
    };
    if affects_running {
        services::stop_running(paths, &state)?;
    }
    let offline = options.offline || std::env::var("SWICTATION_OFFLINE").as_deref() == Ok("1");
    let mut failures = Vec::new();
    for id in &selected {
        let outcome = match *id {
            "platform" => operations::platform_check(),
            "binaries" => doctor::inspect(paths, false)
                .into_iter()
                .find(|check| check.id == "binaries")
                .map(|check| {
                    if check.state == "healthy" {
                        Ok(())
                    } else {
                        Err(anyhow::anyhow!(
                            "{}; run swictation update --force",
                            check.summary
                        ))
                    }
                })
                .unwrap_or(Ok(())),
            "config" => configuration::repair(paths, true),
            "gpu-libs" => match gpu::variant() {
                Ok(Some(variant)) if offline => gpu::check(paths, &variant, false)
                    .context("offline: GPU libraries require download"),
                Ok(_) => gpu::install(paths),
                Err(error) => Err(error),
            },
            "models" => models::selected_model(paths)
                .and_then(|selection| {
                    if offline {
                        models::check(paths, &selection, false)
                            .context("offline: selected models require download")
                    } else {
                        models::download_models(paths, &selection, false)
                    }
                })
                .and_then(|_| configuration::repair(paths, false)),
            "services" => services::configure(paths),
            "integration" => operations::integration_instructions(paths).and_then(|_| {
                if cfg!(target_os = "macos") {
                    Ok(())
                } else {
                    operations::integration_check()
                }
            }),
            "verify" => verify(paths),
            _ => unreachable!(),
        };
        match outcome {
            Ok(()) => println!("ok {id}"),
            Err(error) => {
                eprintln!("failed {id}: {error:#}");
                failures.push(format!("{id}: {error:#}"));
            }
        }
    }
    if affects_running {
        if let Err(error) = services::restore(paths, &state) {
            failures.push(format!("restore previously running services: {error:#}"));
        }
    }
    if options.start && failures.is_empty() {
        services::enable(paths)?;
    }
    if !failures.is_empty() {
        bail!("setup needs attention:\n{}", failures.join("\n"));
    }
    Ok(())
}
