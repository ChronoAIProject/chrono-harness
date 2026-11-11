//! Give each legacy command consumer a private PATH without mutating the suite's
//! environment or serializing independent tests. The original test runs in full.
#[path = "isolated_capability.rs"]
mod process;

pub fn isolated(name: &str) -> bool {
    const MARKER: &str = "CHRONO_RUNNER_GIT_FIXTURE";
    if std::env::var(MARKER).as_deref() == Ok(name) {
        assert_eq!(
            chrono_harness::resolve_program(std::path::Path::new("."), "git", None).unwrap(),
            program(),
        );
        return true;
    }
    let tools = tempfile::tempdir().unwrap();
    super::native_executable::install(
        env!("CARGO_BIN_EXE_chrono-test-git-input"),
        tools.path().join("git"),
    )
    .unwrap();
    let mut paths = vec![tools.path().to_path_buf()];
    paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()));
    process::isolated_test(name, MARKER, &[
        ("CHRONO_RUNNER_GIT_FIXTURE_TOOL", tools.path().join("git").into_os_string()),
        ("PATH", std::env::join_paths(paths).unwrap()),
    ])
}

pub fn program() -> std::path::PathBuf {
    std::env::var_os("CHRONO_RUNNER_GIT_FIXTURE_TOOL")
        .expect("isolated test must bind the command-boundary Git fixture")
        .into()
}
