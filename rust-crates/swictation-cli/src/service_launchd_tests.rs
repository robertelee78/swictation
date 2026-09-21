use super::*;
use std::{collections::VecDeque, os::unix::process::ExitStatusExt, process::ExitStatus};

const NAME: &str = "gui/501/com.swictation.daemon";
const DOMAIN: &str = "gui/501";
const UNIT: &str = "/isolated/daemon.plist";
type Step = (Vec<String>, Output);

fn step(args: &[&str], code: i32, stdout: &str, stderr: &str) -> Step {
    (
        args.iter().map(|arg| arg.to_string()).collect(),
        Output {
            status: ExitStatus::from_raw(code << 8),
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        },
    )
}

fn present(state: &str) -> Step {
    step(&["print", NAME], 0, state, "")
}

fn missing() -> Step {
    step(
        &["print", NAME],
        113,
        "",
        "Could not find service com.swictation.daemon in domain for user gui: 501",
    )
}

fn startup(steps: &mut Vec<Step>) {
    steps.push(step(&["bootstrap", DOMAIN, UNIT], 0, "", ""));
    steps.push(step(
        &["kickstart", "-p", NAME],
        0,
        "service spawned with pid: 42\n",
        "",
    ));
}

fn stable(steps: &mut Vec<Step>) {
    for _ in 0..STABLE_POLLS {
        steps.push(present("\tstate = running\n\tpid = 42\n"));
    }
}

fn exercise(
    steps: Vec<Step>,
    action: impl FnOnce(&mut Manager<'_>) -> Result<()>,
) -> (Result<()>, usize) {
    let mut steps: VecDeque<_> = steps.into();
    let mut pauses = 0;
    let result = {
        let mut run = |args: &[&str]| {
            let (expected, result) = steps.pop_front().expect("unexpected launchctl command");
            assert_eq!(args, expected, "launchctl command ordering changed");
            Ok(result)
        };
        let mut pause = || pauses += 1;
        action(&mut Manager {
            run: &mut run,
            pause: &mut pause,
        })
    };
    assert!(steps.is_empty(), "expected launchctl commands were skipped");
    (result, pauses)
}

#[test]
fn bootstrap_waits_for_asynchronous_bootout_removal() {
    let mut steps = vec![
        present("state = waiting"),
        present("state = waiting"),
        step(&["bootout", NAME], 0, "", ""),
        present("state = exiting"),
        present("state = exiting"),
        missing(),
    ];
    startup(&mut steps);
    stable(&mut steps);
    let (result, pauses) = exercise(steps, |manager| manager.launch(DOMAIN, NAME, UNIT));
    result.unwrap();
    assert_eq!(pauses, 2 + STABLE_POLLS - 1);
}

#[test]
fn stop_waits_for_removal_before_returning() {
    let steps = vec![
        present("state = running\npid = 42"),
        step(&["bootout", NAME], 0, "", ""),
        present("state = exiting"),
        missing(),
    ];
    let (result, pauses) = exercise(steps, |manager| manager.stop(NAME));
    result.unwrap();
    assert_eq!(pauses, 1);
}

#[test]
fn stop_then_start_does_not_bootstrap_while_old_job_is_registered() {
    let mut steps = vec![
        present("state = running\npid = 41"),
        step(&["bootout", NAME], 0, "", ""),
        present("state = exiting"),
        missing(),
        missing(),
    ];
    startup(&mut steps);
    stable(&mut steps);
    exercise(steps, |manager| {
        manager.stop(NAME)?;
        manager.launch(DOMAIN, NAME, UNIT)
    })
    .0
    .unwrap();
}

#[test]
fn stop_of_absent_service_does_not_bootout() {
    exercise(vec![missing()], |manager| manager.stop(NAME))
        .0
        .unwrap();
}

#[test]
fn removal_timeout_is_bounded_and_never_bootstraps() {
    let mut steps = vec![
        present("state = waiting"),
        present("state = waiting"),
        step(&["bootout", NAME], 0, "", ""),
    ];
    steps.extend((0..MAX_POLLS).map(|_| present("state = exiting")));
    let (result, pauses) = exercise(steps, |manager| manager.launch(DOMAIN, NAME, UNIT));
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("not removed within 10 seconds"));
    assert_eq!(pauses, MAX_POLLS - 1);
}

#[test]
fn inspection_failure_is_not_treated_as_absence() {
    let steps = vec![step(&["print", NAME], 1, "", "permission denied")];
    let error = exercise(steps, |manager| manager.launch(DOMAIN, NAME, UNIT))
        .0
        .unwrap_err();
    assert!(error.to_string().contains("permission denied"));
}

#[test]
fn bootstrap_failure_is_reported_without_kickstart() {
    let steps = vec![
        missing(),
        step(&["bootstrap", DOMAIN, UNIT], 5, "", "Input/output error"),
    ];
    let error = exercise(steps, |manager| manager.launch(DOMAIN, NAME, UNIT))
        .0
        .unwrap_err();
    assert!(error.to_string().contains("bootstrap"));
    assert!(error.to_string().contains("Input/output error"));
}

#[test]
fn existing_running_service_is_not_reloaded() {
    let mut steps = vec![
        present("state = running\npid = 42"),
        step(&["kickstart", "-p", NAME], 0, "42\n", ""),
    ];
    stable(&mut steps);
    exercise(steps, |manager| manager.launch(DOMAIN, NAME, UNIT))
        .0
        .unwrap();
}

#[test]
fn pending_first_spawn_is_not_an_exit_failure() {
    let mut steps = vec![missing()];
    startup(&mut steps);
    steps.push(present(
        "state = spawn scheduled\nlast exit code = (never exited)",
    ));
    stable(&mut steps);
    exercise(steps, |manager| manager.launch(DOMAIN, NAME, UNIT))
        .0
        .unwrap();
}

#[test]
fn spawned_process_that_exits_is_reported_as_failure() {
    let mut steps = vec![missing()];
    startup(&mut steps);
    steps.push(present("state = running\npid = 42"));
    steps.push(present("state = waiting\nlast exit code = 1"));
    let error = exercise(steps, |manager| manager.launch(DOMAIN, NAME, UNIT))
        .0
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("exited during startup (exit code 1)"));
}

#[test]
fn respawn_does_not_hide_a_failed_initial_process() {
    let mut steps = vec![missing()];
    startup(&mut steps);
    steps.push(present("state = running\npid = 43"));
    let error = exercise(steps, |manager| manager.launch(DOMAIN, NAME, UNIT))
        .0
        .unwrap_err();
    assert!(error.to_string().contains("restarted during startup"));
}

#[test]
fn startup_timeout_is_bounded() {
    let mut steps = vec![missing()];
    startup(&mut steps);
    steps.extend((0..MAX_POLLS).map(|_| present("state = waiting")));
    let (result, pauses) = exercise(steps, |manager| manager.launch(DOMAIN, NAME, UNIT));
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("did not remain running"));
    assert_eq!(pauses, MAX_POLLS - 1);
}

#[test]
fn kickstart_must_report_a_real_pid() {
    let steps = vec![
        missing(),
        step(&["bootstrap", DOMAIN, UNIT], 0, "", ""),
        step(&["kickstart", "-p", NAME], 0, "", ""),
    ];
    let error = exercise(steps, |manager| manager.launch(DOMAIN, NAME, UNIT))
        .0
        .unwrap_err();
    assert!(error.to_string().contains("did not report a process ID"));
}
