#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
use super::policy::owner;
#[cfg(not(verus_keep_ghost))]
use chrono_diagnostics::{rolling as owner, verus};
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {
fn expect_generation(condition: bool)
    requires condition,
{
    if !condition { panic!(); }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn a_log_generation_expires_at_its_original_deadline() {
    let deadline = owner::generation_deadline(100, 24);
    expect_generation(deadline == Some(124));
    expect_generation(!owner::generation_expired(124, 123));
    expect_generation(owner::generation_expired(124, 124));
    expect_generation(owner::generation_expired(124, 125));
    expect_generation(owner::generation_deadline(100, 0).is_none());
    expect_generation(owner::generation_deadline(u64::MAX - 1, 2).is_none());
    expect_generation(owner::generation_deadline(u64::MAX - 1, 1) == Some(u64::MAX));
}

}

#[cfg(not(verus_keep_ghost))]
use chrono_diagnostics::{rolling::RollingFile, runtime};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(not(verus_keep_ghost))]
static NEXT: AtomicU64 = AtomicU64::new(0);
#[cfg(not(verus_keep_ghost))]
struct Fixture(PathBuf);
#[cfg(not(verus_keep_ghost))]
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "chrono-rolling-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(fs::canonicalize(path).unwrap())
    }
    fn policy(&self, max_bytes: u64, max_files: usize, level: &str) {
        fs::create_dir_all(self.0.join(".chrono-harness")).unwrap();
        fs::write(
            self.0.join(runtime::CONFIG_PATH),
            serde_json::to_vec(&json!({
                "schema":"chrono-runtime-logging/v1", "level":level,
                "directory":".chrono-harness/state/logs/", "max_bytes":max_bytes,
                "max_files":max_files,"stderr":false,
            }))
            .unwrap(),
        )
        .unwrap();
    }
}
#[cfg(not(verus_keep_ghost))]
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
#[cfg(not(verus_keep_ghost))]
fn records(directory: &Path) -> Vec<Value> {
    let mut records = Vec::new();
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|ext| ext == "jsonl") {
            let bytes = fs::read(path).unwrap();
            assert!(bytes.ends_with(b"\n"));
            records.extend(
                std::str::from_utf8(&bytes)
                    .unwrap()
                    .lines()
                    .map(|line| serde_json::from_str::<Value>(line).unwrap()),
            );
        }
    }
    records
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn exact_boundary_rotation_restart_and_reduced_retention() {
    let fixture = Fixture::new();
    let row = b"{\"message\":\"hello\"}\n";
    let mut writer = RollingFile::open(&fixture.0, row.len() as u64 * 2, 3).unwrap();
    for _ in 0..2 {
        writer.write_all(row).unwrap();
    }
    assert!(!fixture.0.join("runtime.1.jsonl").exists());
    for _ in 0..7 {
        writer.write_all(row).unwrap();
    }
    assert_eq!(fs::read(fixture.0.join("runtime.jsonl")).unwrap(), row);
    assert_eq!(records(&fixture.0).len(), 5);
    drop(writer);
    let mut reopened = RollingFile::open(&fixture.0, row.len() as u64 * 2, 3).unwrap();
    reopened.write_all(row).unwrap();
    assert_eq!(records(&fixture.0).len(), 6);
    fs::write(fixture.0.join("unrelated.txt"), b"keep").unwrap();
    let mut single = RollingFile::open(&fixture.0, row.len() as u64, 1).unwrap();
    single.write_all(row).unwrap();
    assert_eq!(records(&fixture.0).len(), 1);
    assert_eq!(fs::read(fixture.0.join("unrelated.txt")).unwrap(), b"keep");
    assert!(!fixture.0.join("runtime.1.jsonl").exists());
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn oversized_record_and_invalid_limits_preserve_existing_bytes() {
    let fixture = Fixture::new();
    assert!(RollingFile::open(&fixture.0, 0, 2).is_err());
    assert!(RollingFile::open(&fixture.0, 100, 0).is_err());
    assert!(RollingFile::open(&fixture.0, 100, 65).is_err());
    let mut writer = RollingFile::open(&fixture.0, 10, 2).unwrap();
    writer.write_all(b"{}\n").unwrap();
    let error = writer.write_all(b"record too large\n").unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    assert_eq!(fs::read(fixture.0.join("runtime.jsonl")).unwrap(), b"{}\n");
    assert!(!fixture.0.join("runtime.1.jsonl").exists());
}

#[cfg(unix)]
#[cfg(not(verus_keep_ghost))]
#[test]
fn aliases_and_unknown_member_types_are_preserved() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let real = fixture.0.join("real");
    fs::create_dir(&real).unwrap();
    symlink(&real, fixture.0.join("alias")).unwrap();
    assert!(RollingFile::open(&fixture.0.join("alias/logs"), 100, 2).is_err());
    fs::write(fixture.0.join("original"), b"keep").unwrap();
    symlink(fixture.0.join("original"), real.join("runtime.1.jsonl")).unwrap();
    assert!(RollingFile::open(&real, 100, 1).is_err());
    assert_eq!(fs::read(fixture.0.join("original")).unwrap(), b"keep");
    fs::remove_file(real.join("runtime.1.jsonl")).unwrap();
    let mut writer = RollingFile::open(&real, 3, 2).unwrap();
    writer.write_all(b"{}\n").unwrap();
    fs::create_dir(real.join("runtime.1.jsonl")).unwrap();
    assert!(writer.write_all(b"{}\n").is_err());
    assert_eq!(fs::read(real.join("runtime.jsonl")).unwrap(), b"{}\n");
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn configured_logger_preserves_json_fields_filtering_and_failure_records() {
    let fixture = Fixture::new();
    assert!(
        runtime::configured_logger(&fixture.0, "fixture", io::sink())
            .unwrap()
            .is_none()
    );
    fixture.policy(4096, 3, "info");
    let logger = runtime::configured_logger(&fixture.0, "fixture", io::sink())
        .unwrap()
        .unwrap();
    logger.run(|| {
        let span = tracing::info_span!("operation", request_id = "r-1");
        let _entered = span.enter();
        tracing::debug!(event_id = "hidden", "debug detail");
        tracing::info!(event_id = "operation.started", "正常运行");
        chrono_diagnostics::logging::failure(
            "operation.failed",
            "failed",
            &chrono_diagnostics::error::Failure::new("E_FIXTURE", "original").record(),
        )
        .unwrap();
    });
    logger.finish().unwrap();
    let rows = records(&fixture.0.join(".chrono-harness/state/logs"));
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["producer"], "fixture");
    assert_eq!(rows[0]["context"]["request_id"], "r-1");
    assert_eq!(rows[1]["error"]["error"]["code"], "E_FIXTURE");
    fixture.policy(1, 1, "info");
    let logger = runtime::configured_logger(&fixture.0, "fixture", io::sink())
        .unwrap()
        .unwrap();
    logger.run(|| tracing::info!("too large"));
    let failures = logger.finish().unwrap_err();
    assert!(!failures.is_empty());
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn idle_runtime_logs_expire_at_their_creation_deadline() {
    let fixture = Fixture::new();
    fixture.policy(4096, 3, "info");
    let config = fixture.0.join(runtime::CONFIG_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    policy["keep_seconds"] = json!(86400);
    fs::write(&config, serde_json::to_vec(&policy).unwrap()).unwrap();
    let directory = fixture.0.join(".chrono-harness/state/logs");
    let current = directory.join("runtime.jsonl");
    let logger = runtime::configured_logger(&fixture.0, "fixture", io::sink())
        .unwrap()
        .unwrap();
    logger.run(|| tracing::info!(event_id = "old.entry", "first entry"));
    logger.finish().unwrap();
    let rows = records(&directory);
    let deadline = rows[0]["deadline"].as_u64().unwrap();
    assert_eq!(rows[0]["schema"], "chrono-runtime-log-generation/v1");
    assert_eq!(rows[1]["event"], "old.entry");

    policy["keep_seconds"] = json!(172800);
    fs::write(&config, serde_json::to_vec(&policy).unwrap()).unwrap();
    let logger = runtime::configured_logger(&fixture.0, "fixture", io::sink())
        .unwrap()
        .unwrap();
    logger.run(|| tracing::info!(event_id = "new.entry", "second entry"));
    logger.finish().unwrap();
    assert_eq!(records(&directory)[0]["deadline"], deadline);
    let legacy = directory.join("runtime.2.jsonl");
    fs::write(&legacy, b"{\"schema\":\"chrono-log/v1\"}\n").unwrap();
    let staged = directory.join("runtime.pending");
    fs::write(&staged, b"{\"schema\":\"chrono-runtime-log-gener").unwrap();

    let held = fs::OpenOptions::new()
        .write(true)
        .open(directory.join("runtime.lock"))
        .unwrap();
    held.lock().unwrap();
    let report = runtime::maintain_at(&fixture.0, deadline).unwrap().unwrap();
    assert!(report.busy);
    assert!(current.exists());
    drop(held);

    let report = runtime::maintain_at(&fixture.0, deadline - 1)
        .unwrap()
        .unwrap();
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert_eq!(report.removed, 0);
    assert!(current.exists());
    assert!(!staged.exists());
    let report = runtime::maintain_at(&fixture.0, deadline).unwrap().unwrap();
    assert_eq!(report.removed, 1);
    assert_eq!(report.legacy, 1);
    assert!(!current.exists());
    assert!(legacy.exists());
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn maintenance_retires_only_complete_legacy_log_records_after_the_window() {
    let fixture = Fixture::new();
    fixture.policy(4096, 4, "info");
    let logger = runtime::configured_logger(&fixture.0, "fixture", io::sink())
        .unwrap()
        .unwrap();
    logger.run(|| tracing::info!(event_id = "old.entry", "legacy diagnostic"));
    logger.finish().unwrap();
    let directory = fixture.0.join(".chrono-harness/state/logs");
    let current = directory.join("runtime.jsonl");
    let original = fs::read(&current).unwrap();
    let malformed = directory.join("runtime.1.jsonl");
    let mut damaged = original.clone();
    damaged.extend_from_slice(b"{\"schema\":\"chrono-log/v1\"");
    fs::write(&malformed, &damaged).unwrap();
    let unknown = directory.join("runtime.2.jsonl");
    fs::write(&unknown, b"{\"unrelated\":true}\n").unwrap();
    let config = fixture.0.join(runtime::CONFIG_PATH);
    let mut policy: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    policy["keep_seconds"] = json!(86400);
    fs::write(&config, serde_json::to_vec(&policy).unwrap()).unwrap();

    let birth = current.metadata().unwrap().created().unwrap()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let report = runtime::maintain_at(&fixture.0, birth + 86399).unwrap().unwrap();
    assert_eq!(report.removed, 0);
    assert_eq!(fs::read(&current).unwrap(), original);
    let lock = fs::File::open(directory.join("runtime.lock")).unwrap();
    lock.lock().unwrap();
    assert!(runtime::maintain_at(&fixture.0, birth + 86400).unwrap().unwrap().busy);
    assert!(current.exists());
    drop(lock);

    let report = runtime::maintain_at(&fixture.0, birth + 86400).unwrap().unwrap();
    assert_eq!(report.removed, 1);
    assert!(!current.exists());
    assert_eq!(fs::read(&malformed).unwrap(), damaged);
    assert_eq!(fs::read(&unknown).unwrap(), b"{\"unrelated\":true}\n");
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn explicit_library_sink_survives_file_adoption_and_dual_failure_keeps_both_causes() {
    use std::sync::{Arc, Mutex};
    #[derive(Clone)]
    struct Captured(Arc<Mutex<Vec<u8>>>);
    impl Write for Captured {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let fixture = Fixture::new();
    fixture.policy(4096, 2, "info");
    let captured = Captured(Arc::new(Mutex::new(Vec::new())));
    let logger = runtime::logger(
        &fixture.0,
        "fixture",
        tracing::Level::INFO,
        captured.clone(),
    )
    .unwrap();
    logger.run(|| tracing::info!(event_id = "original.sink", "retained evidence"));
    logger.finish().unwrap();
    let original: Value = serde_json::from_slice(&captured.0.lock().unwrap()).unwrap();
    assert_eq!(
        original,
        records(&fixture.0.join(".chrono-harness/state/logs"))[0]
    );
    struct Failed;
    impl Write for Failed {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from_raw_os_error(5))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    fixture.policy(1, 2, "info");
    let logger = runtime::logger(&fixture.0, "fixture", tracing::Level::INFO, Failed).unwrap();
    logger.run(|| tracing::error!("both sinks fail"));
    let errors = logger.finish().unwrap_err();
    use chrono_diagnostics::error::RecordedError;
    let node = errors[0].error_node();
    let file = &node.source.as_ref().unwrap().source.as_ref().unwrap();
    assert_eq!(file.code, "E_RUNTIME_LOG");
    assert_eq!(file.related[0].role, "stderr");
    assert_eq!(
        file.related[0].error.source.as_ref().unwrap().native_code,
        Some(5)
    );
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn malformed_and_escaping_policy_is_rejected_before_creating_logs() {
    let fixture = Fixture::new();
    fixture.policy(100, 2, "verbose");
    assert!(runtime::configured_logger(&fixture.0, "fixture", io::sink()).is_err());
    let path = fixture.0.join(runtime::CONFIG_PATH);
    for directory in [
        "../outside/",
        ".chrono-harness/state/",
        ".chrono-harness/state/../outside/",
        ".chrono-harness/state/./logs/",
        ".chrono-harness/state//logs/",
    ] {
        fixture.policy(100, 2, "info");
        let mut policy: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        policy["directory"] = json!(directory);
        fs::write(&path, serde_json::to_vec(&policy).unwrap()).unwrap();
        assert!(
            runtime::configured_logger(&fixture.0, "fixture", io::sink()).is_err(),
            "{directory}"
        );
    }
    fs::write(&path, b"{\"schema\":").unwrap();
    assert!(runtime::configured_logger(&fixture.0, "fixture", io::sink()).is_err());
    assert!(!fixture.0.join(".chrono-harness/state/logs").exists());
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn concurrent_processes_reopen_after_rotation_and_keep_whole_records() {
    let fixture = Fixture::new();
    let helper = env!("CARGO_BIN_EXE_chrono-test-log-writer");
    let mut children = Vec::new();
    for id in 0..4 {
        children.push(
            Command::new(helper)
                .arg(&fixture.0)
                .arg(id.to_string())
                .spawn()
                .unwrap(),
        );
    }
    for mut child in children {
        assert!(child.wait().unwrap().success());
    }
    let rows = records(&fixture.0);
    assert_eq!(rows.len(), 160);
    let mut ids = rows
        .iter()
        .map(|r| {
            (
                r["producer"].as_str().unwrap().to_owned(),
                r["fields"]["sequence"].as_u64().unwrap(),
            )
        })
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), rows.len());
    let logs = fs::read_dir(&fixture.0)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "jsonl"))
        .collect::<Vec<_>>();
    assert!(logs.len() > 1 && logs.len() <= 64);
    for path in logs {
        assert!(fs::metadata(path).unwrap().len() <= 2048);
    }
}
