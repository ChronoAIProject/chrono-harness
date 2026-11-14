#![cfg(unix)]
use chrono_harness::maintenance_signal::{self as signal, Policy, RoundGuard};
use std::{fs, os::unix::net::UnixDatagram, time::Duration};

fn host() -> (tempfile::TempDir, std::path::PathBuf) {
    let host = tempfile::tempdir().unwrap();
    let root = host.path();
    fs::create_dir_all(root.join(".chrono-harness/state/m")).unwrap();
    let policy = Policy {
        schema: "chrono-unified-maintenance/v1".into(),
        state_directory: ".chrono-harness/state/m/".into(),
        socket: ".chrono-harness/state/m/wake.sock".into(),
        idle_seconds: Some(300),
        timer_seconds: 3600,
    };
    fs::write(
        root.join(signal::POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let socket = policy.socket_path(root).unwrap();
    (host, socket)
}

#[test]
fn absent_executor_accepts_an_os_queued_hint_without_a_response() {
    let (host, path) = host();
    let os = UnixDatagram::bind(path).unwrap();
    os.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    let receipt = signal::request_adopted(host.path(), "producer-release");
    assert_eq!(receipt["accepted"], true);
    assert_eq!(receipt["round_completion"], "not-established");
    assert_eq!(receipt["disposal"], "not-attempted");
    let mut bytes = [0; 64];
    let n = os.recv(&mut bytes).unwrap();
    assert_eq!(&bytes[..n], b"producer-release");
}

#[test]
fn missing_or_invalid_activation_reports_failure_without_touching_original_state() {
    let (host, _) = host();
    let original = host.path().join(".chrono-harness/state/original.json");
    fs::write(&original, b"unresolved responsibility").unwrap();
    let receipt = signal::request_adopted(host.path(), "finished");
    assert_eq!(receipt["accepted"], false);
    assert_eq!(receipt["status"], "wake-failed");
    assert!(receipt["error"].is_string());
    assert_eq!(fs::read(original).unwrap(), b"unresolved responsibility");
}

#[test]
fn a_full_os_queue_reports_wake_failure_without_waiting_for_an_executor() {
    let (host, path) = host();
    let _os = UnixDatagram::bind(&path).unwrap();
    // No receiver consumes this finite OS queue. The nonblocking boundary must
    // eventually refuse another hint instead of waiting for maintenance.
    while signal::send(&path, b"duplicate").is_ok() {}
    let receipt = signal::request_adopted(host.path(), "released");
    assert_eq!(receipt["accepted"], false);
    assert_eq!(receipt["status"], "wake-failed");
    assert_eq!(receipt["responsibility"], "existing-owner-state");
    assert_eq!(receipt["disposal"], "not-attempted");
}

#[test]
fn retained_activation_sockets_are_measurable_but_never_disposable_files() {
    let (host, path) = host();
    let _os = UnixDatagram::bind(&path).unwrap();
    fs::write(
        host.path().join(".chrono-harness/state/m/original"),
        b"original bytes",
    )
    .unwrap();
    let strict = chrono_harness::artifact_disposal::artifact_footprint(
        &host.path().join(".chrono-harness/state/m"),
    );
    assert!(strict.err().unwrap().contains("special file"));
    let (observed, sockets) = chrono_harness::artifact_disposal::observed_artifact_footprint(
        &host.path().join(".chrono-harness/state/m"),
    )
    .unwrap();
    assert_eq!(sockets, 1);
    assert_eq!(observed.file_entries, 2);
    assert!(observed.logical_bytes >= b"original bytes".len() as u64);
    let expected = chrono_harness::artifact_disposal::identity(&path).unwrap();
    assert!(
        chrono_harness::artifact_disposal::dispose_entry(
            host.path(),
            path.strip_prefix(host.path()).unwrap().to_str().unwrap(),
            &expected,
            || Ok(())
        )
        .unwrap_err()
        .contains("special file")
    );
    assert!(path.exists());
}

#[test]
fn maintenance_report_publication_does_not_wake_itself_or_suppress_other_threads() {
    let (host, path) = host();
    let os = UnixDatagram::bind(path).unwrap();
    os.set_nonblocking(true).unwrap();
    let round = RoundGuard::enter();
    let internal = signal::request_adopted(host.path(), "report-release");
    assert_eq!(internal["accepted"], false);
    assert_eq!(internal["status"], "maintenance-internal-no-new-event");
    let mut bytes = [0; 64];
    assert_eq!(
        os.recv(&mut bytes).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    let root = host.path().to_owned();
    let external = std::thread::spawn(move || signal::request_adopted(&root, "business-finished"));
    assert_eq!(external.join().unwrap()["accepted"], true);
    assert_eq!(os.recv(&mut bytes).unwrap(), b"business-finished".len());
    drop(round);
    assert_eq!(
        signal::request_adopted(host.path(), "after-round")["accepted"],
        true
    );
}
