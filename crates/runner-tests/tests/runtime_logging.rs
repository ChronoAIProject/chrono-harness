use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

fn binary() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../runner/target/debug/chrono-harness")
}
fn policy(root: &Path, max_bytes: u64) {
    fs::create_dir_all(root.join(".chrono-harness")).unwrap();
    fs::write(root.join(".chrono-harness/logging.json"), serde_json::to_vec(&json!({
        "schema":"chrono-runtime-logging/v1","level":"info",
        "directory":".chrono-harness/state/logs/", "max_bytes":max_bytes,
        "max_files":3,"stderr":false,
    })).unwrap()).unwrap();
}

#[test]
fn actual_cli_records_success_failure_and_rotates_without_stdout_changes() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(fixture.path()).unwrap();
    policy(&root, 1024);
    for arg in ["--version", "unknown-command", "--version", "--version"] {
        let expected = chrono_harness::dispatch(&[arg]);
        let actual = Command::new(binary()).arg(arg).current_dir(&root).output().unwrap();
        assert_eq!(actual.status.code(), Some(i32::from(expected.exit_code)));
        assert_eq!(actual.stdout, expected.stdout.as_bytes());
        assert_eq!(actual.stderr, expected.stderr.as_bytes());
    }
    let directory = root.join(".chrono-harness/state/logs");
    let mut rows = Vec::new();
    for entry in fs::read_dir(&directory).expect("normal CLI execution must create runtime logs") {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e|e=="jsonl") {
            let bytes = fs::read(path).unwrap();
            assert!(bytes.len() <= 1024 && bytes.ends_with(b"\n"));
            rows.extend(std::str::from_utf8(&bytes).unwrap().lines().map(|s|serde_json::from_str::<Value>(s).unwrap()));
        }
    }
    assert!(directory.join("runtime.1.jsonl").exists());
    assert!(!directory.join("runtime.3.jsonl").exists());
    assert!(rows.iter().all(|r|r["schema"]=="chrono-log/v1" && r["producer"]=="chrono-harness"));
    assert!(rows.iter().any(|r|r["event"]=="cli.started"));
    assert!(rows.iter().any(|r|r["event"]=="cli.finished" && r["level"]=="error" && r["fields"]["exit_code"]==2));
    assert!(rows.iter().any(|r|r["event"]=="cli.finished" && r["fields"]["exit_code"]==0));
}

#[test]
fn actual_cli_reports_log_failure_and_preserves_operation_exit() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(fixture.path()).unwrap();
    policy(&root, 1);
    let actual = Command::new(binary()).arg("unknown-command").current_dir(&root).output().unwrap();
    assert_eq!(actual.status.code(), Some(2));
    assert!(actual.stdout.is_empty());
    let stderr = std::str::from_utf8(&actual.stderr).unwrap();
    assert!(stderr.starts_with("E_USAGE: use --help\n"));
    let event: Value = serde_json::from_str(stderr.lines().last().unwrap()).unwrap();
    assert_eq!(event["event"], "runtime.logging_failed");
    assert_eq!(event["error"]["error"]["context"]["operation_exit_code"], 2);
    assert!(!event["error"]["error"]["related"].as_array().unwrap().is_empty());
}

#[test]
fn actual_cli_rejects_invalid_policy_before_business_execution() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(fixture.path()).unwrap();
    policy(&root, 1024);
    fs::write(root.join(".chrono-harness/logging.json"), b"invalid json").unwrap();
    let actual = Command::new(binary()).arg("--version").current_dir(&root).output().unwrap();
    assert_eq!(actual.status.code(), Some(2));
    assert!(actual.stdout.is_empty());
    let event: Value = serde_json::from_slice(&actual.stderr).unwrap();
    assert_eq!(event["event"], "runtime.logging_failed");
    assert!(event["error"]["error"]["source"].is_object());
}
