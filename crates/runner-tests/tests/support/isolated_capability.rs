// fork duplicates every thread's open descriptions until exec, even CLOEXEC
// capabilities that the child never intentionally consumes. Tests asserting an
// immediate last-close boundary need a process without unrelated fixture forks.
pub fn isolated_capability_test(name: &str) -> bool {
    isolated_test(name, "CHRONO_RETENTION_CAPABILITY_TEST", &[])
}

/// The Git fixture needs the same native completion and single-test result
/// check, with a private child environment instead of global PATH mutation.
pub fn isolated_test(
    name: &str,
    marker: &str,
    environment: &[(&str, std::ffi::OsString)],
) -> bool {
    if std::env::var(marker).as_deref() == Ok(name) {
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
