//! Wait for launchd transitions instead of treating command acceptance as completion.
use anyhow::{bail, Context, Result};
use std::{
    process::Output,
    thread,
    time::{Duration, Instant},
};

const POLL_INTERVAL: Duration = Duration::from_millis(100);
const MAX_POLLS: usize = 100;
const STABLE_POLLS: usize = 10;
const WAIT_LIMIT: Duration = Duration::from_secs(10);

pub(super) struct State {
    pub running: bool,
    pid: Option<u32>,
    exit: Option<String>,
}

type Runner<'a> = dyn FnMut(&[&str]) -> Result<Output> + 'a;
struct Manager<'a> {
    run: &'a mut Runner<'a>,
    pause: &'a mut dyn FnMut(),
}

impl Manager<'_> {
    fn checked(&mut self, args: &[&str]) -> Result<Output> {
        let output = (self.run)(args)?;
        if !output.status.success() {
            bail!(
                "launchctl {} failed ({}): {}",
                args.join(" "),
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(output)
    }

    fn inspect(&mut self, name: &str) -> Result<Option<State>> {
        let output = (self.run)(&["print", name])?;
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            if error.to_lowercase().contains("could not find service") {
                return Ok(None);
            }
            bail!("cannot inspect launchd service {name}: {error}");
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let value = |key: &str| text.lines().find_map(|line| line.trim().strip_prefix(key));
        Ok(Some(State {
            running: value("state = ") == Some("running"),
            pid: value("pid = ").and_then(|pid| pid.parse().ok()),
            exit: value("last exit code = ")
                .and_then(|code| code.parse::<i32>().ok())
                .map(|code| format!("exit code {code}"))
                .or_else(|| {
                    value("last terminating signal = ")
                        .filter(|signal| !matches!(*signal, "0" | "(never exited)" | "none"))
                        .map(|signal| format!("signal {signal}"))
                }),
        }))
    }

    fn stop(&mut self, name: &str) -> Result<()> {
        if self.inspect(name)?.is_none() {
            return Ok(());
        }
        self.checked(&["bootout", name])?;
        // bootout may return while the old registration is still being removed.
        // A subsequent bootstrap must wait for print to confirm it is gone.
        let deadline = Instant::now() + WAIT_LIMIT;
        for attempt in 0..MAX_POLLS {
            if self.inspect(name)?.is_none() {
                return Ok(());
            }
            if Instant::now() >= deadline {
                break;
            }
            if attempt + 1 < MAX_POLLS {
                (self.pause)();
            }
        }
        bail!("launchd service {name} was not removed within 10 seconds after bootout; retry after it finishes stopping")
    }

    fn launch(&mut self, domain: &str, name: &str, unit: &str) -> Result<()> {
        match self.inspect(name)? {
            Some(state) if state.running => (),
            Some(_) => {
                self.stop(name)?;
                self.checked(&["bootstrap", domain, unit])?;
            }
            None => {
                self.checked(&["bootstrap", domain, unit])?;
            }
        }
        // -p is documented to return the new or already-running process ID.
        let output = self.checked(&["kickstart", "-p", name])?;
        // macOS prints "service spawned with pid: N"; accept bare N as well.
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let pid = stdout
            .lines()
            .chain(stderr.lines())
            .find_map(|line| {
                let line = line.trim();
                line.strip_prefix("service spawned with pid:")
                    .unwrap_or(line)
                    .trim()
                    .parse::<u32>()
                    .ok()
            })
            .context("launchctl kickstart did not report a process ID")?;
        if pid == 0 || pid > i32::MAX as u32 {
            bail!("launchctl kickstart reported an invalid process ID");
        }
        let mut stable = 0;
        let deadline = Instant::now() + WAIT_LIMIT;
        for attempt in 0..MAX_POLLS {
            let state = self
                .inspect(name)?
                .context("launchd service disappeared during startup")?;
            if state.running && state.pid == Some(pid) {
                stable += 1;
                if stable >= STABLE_POLLS {
                    return Ok(());
                }
            } else {
                stable = 0;
                if let Some(exit) = state.exit {
                    bail!("launchd service {name} exited during startup ({exit})");
                }
                if state.pid.is_some_and(|current| current != pid) {
                    bail!("launchd service {name} restarted during startup (process {pid} exited)");
                }
            }
            if Instant::now() >= deadline {
                break;
            }
            if attempt + 1 < MAX_POLLS {
                (self.pause)();
            }
        }
        bail!("launchd service {name} did not remain running within 10 seconds")
    }
}

fn with_manager<T>(action: impl FnOnce(&mut Manager<'_>) -> Result<T>) -> Result<T> {
    let mut run = |args: &[&str]| super::run("launchctl", args);
    let mut pause = || thread::sleep(POLL_INTERVAL);
    action(&mut Manager {
        run: &mut run,
        pause: &mut pause,
    })
}

pub(super) fn inspect(name: &str) -> Result<Option<State>> {
    with_manager(|manager| manager.inspect(name))
}

pub(super) fn stop(name: &str) -> Result<()> {
    with_manager(|manager| manager.stop(name))
}

pub(super) fn launch(domain: &str, name: &str, unit: &str) -> Result<()> {
    with_manager(|manager| manager.launch(domain, name, unit))
}

#[cfg(test)]
#[path = "service_launchd_tests.rs"]
mod tests;
