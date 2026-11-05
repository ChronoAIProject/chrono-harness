// fork duplicates every thread's open descriptions until exec, even CLOEXEC
// capabilities that the child never intentionally consumes. Tests asserting an
// immediate last-close boundary need a process without unrelated fixture forks.
pub fn isolated_capability_test(name: &str) -> bool {
    const MARKER: &str = "CHRONO_RETENTION_CAPABILITY_TEST";
    if std::env::var(MARKER).as_deref() == Ok(name) {
        return true;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env(MARKER, name)
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
