#![cfg(unix)]
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
use super::policy::artifact_disposal as disposal;
#[cfg(verus_keep_ghost)]
use super::policy::maintenance_signal as signal;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::artifact_disposal as disposal;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;
verus! {
#[cfg_attr(not(verus_keep_ghost), test)]
fn observing_a_socket_requires_both_the_observer_and_socket_kind() {
    if !disposal::observable_socket(true, true)
        || disposal::observable_socket(false, true)
        || disposal::observable_socket(true, false)
        || disposal::observable_socket(false, false) { panic!(); }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn hint_length_keeps_the_original_nonempty_64_byte_boundary() {
    if signal::valid_hint_length(0) || !signal::valid_hint_length(1)
        || !signal::valid_hint_length(64) || signal::valid_hint_length(65)
        || signal::valid_hint_length(usize::MAX) { panic!(); }
}

fn expect_hint(actual: bool)
    requires actual,
{
    if !actual { panic!(); }
}

fn expect_hint_text(actual: &str, expected: &str)
    requires actual@ == expected@,
{
    if actual.to_string() != expected.to_string() { panic!(); }
}

fn hint_send_case<'a>(payload: &'a str, bytes: usize, failure: u8, sent: usize) {
    let open = |mut trace: Vec<(u8, &'a str, bool)>|
        -> (out: (Vec<(u8, &'a str, bool)>, Result<&'a str, String>))
        ensures out.0@ == trace@.push((0, payload, false)),
            if failure == 1 { match out.1 {
                Err(error) => error@ == payload@, _ => false,
            } } else { out.1 == Ok::<&'a str, String>(payload) },
    {
        trace.push((0, payload, false));
        (trace, if failure == 1 { Err(payload.to_string()) } else { Ok(payload) })
    };
    let nonblocking = |mut trace: Vec<(u8, &'a str, bool)>, socket: &&'a str, enabled: bool|
        -> (out: (Vec<(u8, &'a str, bool)>, Result<(), String>))
        ensures out.0@ == trace@.push((1, *socket, enabled)),
            if failure == 2 { match out.1 {
                Err(error) => error@ == payload@, _ => false,
            } } else { out.1 == Ok::<(), String>(()) },
    {
        trace.push((1, *socket, enabled));
        (trace, if failure == 2 { Err(payload.to_string()) } else { Ok(()) })
    };
    let send = |mut trace: Vec<(u8, &'a str, bool)>, socket: &'a str|
        -> (out: (Vec<(u8, &'a str, bool)>, Result<usize, String>))
        ensures out.0@ == trace@.push((2, socket, false)),
            if failure == 3 { match out.1 {
                Err(error) => error@ == payload@, _ => false,
            } } else { out.1 == Ok::<usize, String>(sent) },
    {
        trace.push((2, socket, false));
        (trace, if failure == 3 { Err(payload.to_string()) } else { Ok(sent) })
    };
    let (trace, result) = signal::send_hint(Vec::new(), bytes, open, nonblocking, send);
    if bytes == 0 || bytes > 64 {
        expect_hint(trace.len() == 0);
        match result {
            Err(error) => expect_hint(error == "maintenance hint exceeds 64 bytes".to_string()),
            Ok(()) => expect_hint(false),
        }
    } else {
        let count = if failure == 1 { 1 } else if failure == 2 { 2 } else { 3 };
        expect_hint(trace.len() == count);
        expect_hint(trace[0].0 == 0 && !trace[0].2);
        expect_hint_text(trace[0].1, payload);
        if failure != 1 {
            expect_hint(trace[1].0 == 1 && trace[1].2);
            expect_hint_text(trace[1].1, payload);
            if failure != 2 {
                expect_hint(trace[2].0 == 2 && !trace[2].2);
                expect_hint_text(trace[2].1, payload);
            }
        }
        if failure == 1 || failure == 2 || failure == 3 {
            match result {
                Err(error) => expect_hint(error == payload.to_string()),
                Ok(()) => expect_hint(false),
            }
        } else if sent == bytes {
            expect_hint(result.is_ok());
        } else {
            match result {
                Err(error) => expect_hint(error == "incomplete maintenance datagram".to_string()),
                Ok(()) => expect_hint(false),
            }
        }
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn hint_send_keeps_order_nonblocking_and_the_complete_original_error() {
    hint_send_case("", 1, 1, 1);
    hint_send_case("原错/λ\0\n", 64, 1, 64);
    hint_send_case("原错/λ\0\n", 64, 2, 64);
    hint_send_case("原错/λ\0\n", 64, 3, 64);
    hint_send_case("原提示/λ\0\n", 1, 0, 1);
    hint_send_case("原提示/λ\0\n", 64, 0, 64);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn hint_send_rejects_invalid_lengths_without_opening_or_sending() {
    hint_send_case("retained", 0, 0, 0);
    hint_send_case("retained", 65, 1, 65);
    hint_send_case("retained", usize::MAX, 3, usize::MAX);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn hint_send_requires_the_exact_observed_length() {
    hint_send_case("retained", 1, 0, 0);
    hint_send_case("retained", 64, 0, 1);
    hint_send_case("retained", 64, 0, 65);
    hint_send_case("retained", 1, 0, usize::MAX);
}
}

#[cfg(not(verus_keep_ghost))]
use chrono_harness::maintenance_signal::{self as signal, Policy, RoundGuard};
#[cfg(not(verus_keep_ghost))]
use std::{fs, os::unix::net::UnixDatagram, time::Duration};

#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
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
