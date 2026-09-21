mod configuration;
mod doctor;
mod gpu;
mod lifecycle;
mod models;
mod network;
mod operations;
mod package;
mod paths;
mod services;
mod setup;
mod trust;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "swictation",
    version,
    about = "Native voice dictation and installation management",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Install models, preserve configuration, and configure user services.
    Setup {
        #[arg(long)]
        repair: bool,
        #[arg(long)]
        list: bool,
        #[arg(long)]
        offline: bool,
        #[arg(long)]
        start: bool,
        #[arg(long = "step")]
        steps: Vec<String>,
        #[arg(long)]
        services: bool,
        #[arg(long)]
        models: bool,
        #[arg(long)]
        gpu_libs: bool,
        #[arg(long)]
        config: bool,
        #[arg(long)]
        integration: bool,
        #[arg(long)]
        verify: bool,
        #[arg(long)]
        platform: bool,
        #[arg(long)]
        binaries: bool,
        #[arg(long)]
        config_reset: bool,
        #[arg(long)]
        config_heal: bool,
    },
    /// Inspect installed artifacts without modifying configuration or services.
    Doctor {
        #[arg(long)]
        deep: bool,
        #[arg(long)]
        json: bool,
    },
    /// Fetch a verified release, check availability, or activate the retained version.
    Update {
        #[arg(long, conflicts_with = "rollback")]
        check: bool,
        #[arg(long, conflicts_with = "version")]
        rollback: bool,
        #[arg(long)]
        version: Option<String>,
        #[arg(long, conflicts_with_all = ["check", "rollback"])]
        force: bool,
    },
    /// Remove owned installation and services; preserve user data by default.
    Uninstall {
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        purge_config: bool,
        #[arg(long)]
        purge_cache: bool,
    },
    Start {
        #[arg(long)]
        ui: bool,
    },
    Stop,
    Status,
    Toggle,
    #[command(alias = "download-model")]
    DownloadModels {
        #[arg(conflicts_with = "model")]
        selection: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        force: bool,
    },
    Logs {
        #[arg(long)]
        follow: bool,
    },
    Ui,
    Config,
    Version,
    #[command(name = "__install", hide = true)]
    Install {
        #[arg(long)]
        archive: PathBuf,
        #[arg(long)]
        sha256: String,
        #[arg(long, hide = true)]
        allow_unsigned_local: bool,
    },
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let paths = paths::Paths::discover()?;
    match cli.command {
        Command::Setup {
            repair,
            list,
            offline,
            start,
            mut steps,
            services,
            models,
            gpu_libs,
            config,
            integration,
            verify,
            platform,
            binaries,
            config_reset,
            config_heal,
        } => {
            if list {
                setup::list();
                return Ok(());
            }
            for (selected, step) in [
                (services, "services"),
                (models, "models"),
                (gpu_libs, "gpu-libs"),
                (config, "config"),
                (integration, "integration"),
                (verify, "verify"),
                (platform, "platform"),
                (binaries, "binaries"),
                (config_reset, "config-reset"),
                (config_heal, "config-heal"),
            ] {
                if selected {
                    steps.push(step.into());
                }
            }
            setup::run(
                &paths,
                &setup::Options {
                    repair,
                    only: steps,
                    offline,
                    start,
                },
            )?;
        }
        Command::Doctor { deep, json } => {
            if !doctor::run(&paths, deep, json)? {
                std::process::exit(1);
            }
        }
        Command::Update {
            check,
            rollback,
            version,
            force,
        } => {
            lifecycle::require_current_executable(&paths)?;
            if rollback {
                lifecycle::rollback(&paths)?;
            } else {
                lifecycle::update(&paths, version.as_deref(), check, force)?;
            }
        }
        Command::Uninstall {
            yes,
            purge_config,
            purge_cache,
        } => {
            lifecycle::require_current_executable(&paths)?;
            lifecycle::uninstall(&paths, yes, purge_config, purge_cache)?;
        }
        Command::Start { ui } => services::start(&paths, ui)?,
        Command::Stop => services::stop(&paths)?,
        Command::Status => services::status(&paths)?,
        Command::Toggle => operations::toggle(&paths)?,
        Command::DownloadModels {
            selection,
            model,
            force,
        } => {
            let _guard = lifecycle::lock(&paths)?;
            let model = model
                .or(selection)
                .unwrap_or(models::selected_model(&paths)?);
            models::download_models(&paths, &model, force)?;
        }
        Command::Logs { follow } => operations::logs(&paths, follow)?,
        Command::Ui => operations::ui(&paths)?,
        Command::Config => operations::config(&paths, false)?,
        Command::Version => println!("swictation {}", env!("CARGO_PKG_VERSION")),
        Command::Install {
            archive,
            sha256,
            allow_unsigned_local,
        } => {
            if unsafe { libc::geteuid() } == 0 {
                bail!("Install as your normal user, without sudo");
            }
            if allow_unsigned_local && option_env!("SWICTATION_APPLE_TEAM_ID").is_some() {
                bail!("Signed builds do not support unsigned fixtures");
            }
            lifecycle::install(&paths, &archive, &sha256, allow_unsigned_local)?;
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("swictation: {error:#}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod lifecycle_tests;
