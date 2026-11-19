#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{verified_identifiers as identifiers, verus};
#[cfg(verus_keep_ghost)]
#[path = "../../../identifiers/src/lib.rs"]
mod identifiers;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
verus! {
pub fn same_test(expected:&str, current:Option<&str>) -> (matches:bool)
    ensures matches == match current {Some(actual)=>expected@==actual@,None=>false},
{match current {Some(actual)=>identifiers::equal_text(expected,actual),None=>false}}

pub fn release_custody(verified_child: bool, retired: bool) -> (release: bool)
    ensures release == (verified_child && retired),
{ verified_child && retired }
}
// fork duplicates every thread's open descriptions until exec, even CLOEXEC
// capabilities that the child never intentionally consumes. Tests asserting an
// immediate last-close boundary need a process without unrelated fixture forks.
#[cfg(not(verus_keep_ghost))]
pub fn isolated_capability_test(name: &str) -> bool {
    isolated_test(name, "CHRONO_RETENTION_CAPABILITY_TEST", &[])
}

#[cfg(not(verus_keep_ghost))]
pub const OBSERVATION_PREFIX: &str = "CHRONO_TEST_OBSERVATION ";

/// The Git fixture needs the same native completion and single-test result
/// check, with a private child environment instead of global PATH mutation.
#[cfg(not(verus_keep_ghost))]
pub fn isolated_test(name: &str, marker: &str, environment: &[(&str, std::ffi::OsString)]) -> bool {
    run_isolated_test(name, marker, environment, false)
}

#[cfg(not(verus_keep_ghost))]
fn run_isolated_test(
    name: &str,
    marker: &str,
    environment: &[(&str, std::ffi::OsString)],
    forward_observation: bool,
) -> bool {
    if same_test(name, std::env::var(marker).ok().as_deref()) {
        return true;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env(marker, name)
        .envs(environment.iter().map(|(key, value)| (key, value)))
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let one_passed = stdout
        .lines()
        .any(|line| line.starts_with("test result: ok. 1 passed; 0 failed;"));
    assert!(
        output.status.success() && one_passed,
        "isolated capability test failed: {}; stdout: {}; stderr: {}",
        output.status,
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );
    // Successful libtest capture normally hides nested observations. Forward only
    // explicitly emitted records into the enclosing operation's retained stderr.
    // The failure assertion above already carries both original streams.
    if forward_observation {
        use std::io::Write;
        let mut records = output
            .stderr
            .split_inclusive(|byte| *byte == b'\n')
            .filter(|line| line.starts_with(OBSERVATION_PREFIX.as_bytes()));
        let record = records
            .next()
            .expect("successful child omitted its observation");
        assert!(records.next().is_none(), "ambiguous child observation");
        std::io::stderr().lock().write_all(record).unwrap();
    }
    false
}

/// Bind legacy Git acquisition to a private command mock before libtest runs
/// the original case. The parent verifies the child's actual case result.
#[cfg(not(verus_keep_ghost))]
pub fn isolated_git() -> bool {
    isolated_git_with_retirement(|_| Ok(false))
}

/// The fixture publisher supplies its existing owner's retirement observation.
/// This uses the original child join; there is no second completion channel.
#[cfg(not(verus_keep_ghost))]
pub fn isolated_git_with_retirement(
    retirement: impl FnOnce(&std::path::Path) -> Result<bool, String>,
) -> bool {
    isolated_git_with_observation(retirement, false)
}

#[cfg(not(verus_keep_ghost))]
pub fn isolated_git_with_observation(
    retirement: impl FnOnce(&std::path::Path) -> Result<bool, String>,
    forward_observation: bool,
) -> bool {
    let thread = std::thread::current();
    let name = thread.name().expect("libtest case identity");
    const MARKER: &str = "CHRONO_SELECTED_GIT_BOUNDARY";
    if same_test(name, std::env::var(MARKER).ok().as_deref()) {
        return true;
    }
    let directory = tempfile::tempdir().unwrap();
    let program = directory.path().join("git");
    std::fs::copy(
        option_env!("CARGO_BIN_EXE_chrono-test-git-model").expect("registered Git mock binary"),
        &program,
    )
    .unwrap();
    if let Some(cargo) = option_env!("CARGO_BIN_EXE_chrono-test-cargo-model") {
        std::fs::copy(cargo, directory.path().join("cargo")).unwrap();
    }
    let mut path = vec![directory.path().to_path_buf()];
    path.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    // Own this exact private directory through the final removal decision.
    // An incidental forked copy cannot release this owner's live lock.
    let ownership =
        chrono_harness::ownership::DirectoryLease::acquire_local(directory.path(), true, None)
            .unwrap()
            .expect("new private fixture directory is available");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let value = run_isolated_test(
            name,
            MARKER,
            &[
                ("PATH", std::env::join_paths(path).unwrap()),
                (
                    "CHRONO_TEST_CUSTODY_ROOT",
                    directory.path().join("custody").into_os_string(),
                ),
            ],
            forward_observation,
        );
        let custody = directory.path().join("custody");
        let retired = if custody.try_exists().unwrap() {
            retirement(&custody)
                .unwrap_or_else(|error| panic!("fixture retirement unknown: {error}"))
        } else {
            true
        };
        ownership
            .stable()
            .expect("original private fixture directory identity");
        (value, release_custody(true, retired))
    }));
    match result {
        Ok((value, released)) => {
            if !released {
                eprintln!(
                    "Git-boundary custody handoff (use its registered finish/maintain policy): {}",
                    directory.path().display()
                );
                let _ = directory.keep();
            } else {
                directory
                    .close()
                    .expect("released private Git fixture removal");
            }
            value
        }
        Err(error) => {
            eprintln!(
                "failed Git-boundary fixture originals: {}",
                directory.path().display()
            );
            let _ = directory.keep();
            std::panic::resume_unwind(error)
        }
    }
}
