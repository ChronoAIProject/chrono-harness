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
}
// fork duplicates every thread's open descriptions until exec, even CLOEXEC
// capabilities that the child never intentionally consumes. Tests asserting an
// immediate last-close boundary need a process without unrelated fixture forks.
#[cfg(not(verus_keep_ghost))]
pub fn isolated_capability_test(name: &str) -> bool {
    isolated_test(name, "CHRONO_RETENTION_CAPABILITY_TEST", &[])
}

/// The Git fixture needs the same native completion and single-test result
/// check, with a private child environment instead of global PATH mutation.
#[cfg(not(verus_keep_ghost))]
pub fn isolated_test(
    name: &str,
    marker: &str,
    environment: &[(&str, std::ffi::OsString)],
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
    false
}

/// Bind legacy Git acquisition to a private command mock before libtest runs
/// the original case. The parent verifies the child's actual case result.
#[cfg(not(verus_keep_ghost))]
pub fn isolated_git() -> bool {
    let thread = std::thread::current();
    let name = thread.name().expect("libtest case identity");
    const MARKER: &str = "CHRONO_SELECTED_GIT_BOUNDARY";
    if same_test(name, std::env::var(MARKER).ok().as_deref()) {
        return true;
    }
    let directory = tempfile::tempdir().unwrap();
    let program = directory.path().join("git");
    std::fs::copy(option_env!("CARGO_BIN_EXE_chrono-test-git-model").expect("registered Git mock binary"), &program).unwrap();
    if let Some(cargo) = option_env!("CARGO_BIN_EXE_chrono-test-cargo-model") {
        std::fs::copy(cargo, directory.path().join("cargo")).unwrap();
    }
    let mut path = vec![directory.path().to_path_buf()];
    path.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| isolated_test(name, MARKER, &[
        ("PATH", std::env::join_paths(path).unwrap()),
        ("CHRONO_TEST_CUSTODY_ROOT", directory.path().join("custody").into_os_string()),
    ])));
    match result {
        Ok(value) => {
            if directory.path().join("custody").exists() {
                // Only the existing custody owner can establish retirement.
                // A libtest exit does not release retained diagnostic consumers.
                eprintln!("Git-boundary custody handoff (use its registered finish/maintain policy): {}", directory.path().display());
                std::mem::forget(directory);
            }
            value
        },
        Err(error) => {
            eprintln!("failed Git-boundary fixture originals: {}", directory.path().display());
            std::mem::forget(directory);
            std::panic::resume_unwind(error)
        }
    }
}
