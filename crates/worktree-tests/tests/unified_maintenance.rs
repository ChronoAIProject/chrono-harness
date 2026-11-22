//! Independent expectations for the unified owner's business/maintenance boundary.
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
use super::policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(not(verus_keep_ghost))]
use chrono_worktree::verified_lifecycle as policy;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;
verus! {
fn expect(actual: bool) requires actual { if !actual { panic!(); } }
#[cfg_attr(not(verus_keep_ghost), test)]
fn maintenance_keeps_coordinator_and_invoking_roots_in_order() {
    let roots = policy::maintenance_exclusions(17u64, u64::MAX);
    expect(roots.len() == 2);
    expect(roots[0] == 17);
    expect(roots[1] == u64::MAX);
    let same = policy::maintenance_exclusions(0u64, 0u64);
    expect(same.len() == 2);
    expect(same[0] == 0 && same[1] == 0);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn maintenance_keeps_complete_root_text_and_repeated_values() {
    let roots = policy::maintenance_exclusions(
        "/coordinator λ/source\n".to_string(), "/caller λ/source\0\n".to_string());
    expect(roots.len() == 2);
    expect(policy::equal_text(roots[0].as_str(), "/coordinator λ/source\n"));
    expect(policy::equal_text(roots[1].as_str(), "/caller λ/source\0\n"));
    let same = policy::maintenance_exclusions("".to_string(), "".to_string());
    expect(same.len() == 2);
    expect(policy::equal_text(same[0].as_str(), ""));
    expect(policy::equal_text(same[1].as_str(), ""));
}

fn retention_phase_case(complete: bool, unprotected: usize, succeeds: bool,
    payload: Option<u64>, message: &str, expected_stage: u8, expected_code: u64)
    requires if complete && unprotected == 0 {
        if succeeds { expected_stage == 2 && expected_code == 22 }
        else { expected_stage == 3 && expected_code == 23 }
    } else { expected_stage == 4 && expected_code == 24 },
{
    let attempt = |state: (u8, bool)| -> (out: ((u8, bool), Result<Option<u64>, String>))
        ensures out.0 == (1u8, state.1 && state.0 == 0u8),
            match out.1 {
                Ok(value) => succeeds && value == payload,
                Err(error) => !succeeds && error@ == message@,
            },
    {
        ((1, state.1 && state.0 == 0),
            if succeeds { Ok(payload) } else { Err(message.to_string()) })
    };
    let completed = |state: (u8, bool), value: Option<u64>| -> (out: ((u8, bool), u64))
        ensures out == ((2u8, state.1 && state.0 == 1u8 && value == payload), 22u64),
    { ((2, state.1 && state.0 == 1 && value == payload), 22) };
    let failed = |state: (u8, bool), error: String| -> (out: ((u8, bool), u64))
        ensures out == ((3u8, state.1 && state.0 == 1u8 && error@ == message@), 23u64),
    { ((3, state.1 && state.0 == 1 && policy::equal_text(error.as_str(), message)), 23) };
    let deferred = |state: (u8, bool)| -> (out: ((u8, bool), u64))
        ensures out == ((4u8, state.1 && state.0 == 0u8), 24u64),
    { ((4, state.1 && state.0 == 0), 24) };
    let (state, code) = policy::maintenance_retention_phase(
        (0u8, true), complete, unprotected, attempt, completed, failed, deferred);
    expect(state.1);
    expect(state.0 == expected_stage);
    expect(code == expected_code);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn maintenance_retention_defers_before_access_when_original_protection_is_unknown() {
    retention_phase_case(false, 0, true, None, "unused", 4, 24);
    retention_phase_case(false, 0, false, Some(0), "unused error", 4, 24);
    retention_phase_case(false, 1, true, Some(u64::MAX), "unused", 4, 24);
    retention_phase_case(false, usize::MAX, false, None, "unused error", 4, 24);
    retention_phase_case(true, 1, true, None, "unused", 4, 24);
    retention_phase_case(true, usize::MAX, false, Some(u64::MAX), "unused error", 4, 24);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn maintenance_retention_preserves_observed_values_and_complete_errors() {
    retention_phase_case(true, 0, true, None, "unused", 2, 22);
    retention_phase_case(true, 0, true, Some(0), "unused", 2, 22);
    retention_phase_case(true, 0, true, Some(u64::MAX), "unused", 2, 22);
    retention_phase_case(true, 0, false, None, "", 3, 23);
    retention_phase_case(true, 0, false, Some(0), "read λ failed\0\n", 3, 23);
    retention_phase_case(true, 0, false, Some(u64::MAX), "second error", 3, 23);
}

// Each row specifies its own final observation, independent of the production predicates.
// Stages also record that every observation received its exact predecessor and address.
fn activation_case(query_result: (i32, i32), path: (bool, bool, bool, u32, u32),
    binding_result: (bool, bool, u64), message: &str, expected: (u8, bool, &str))
    requires match expected.0 {
        1 => query_result.0 == 0 && query_result.1 != 2 && !expected.1
            && expected.2@ == "OS activation requires a Unix datagram socket"@,
        2 => query_result.0 != 0 && !expected.1 && expected.2@ == message@,
        3 => query_result == (0i32, 2i32) && !path.0 && !expected.1
            && expected.2@ == message@,
        4 => query_result == (0i32, 2i32) && path.0 && !expected.1
            && ((!path.1 && expected.2@ == message@)
                || (path.1 && !path.2
                    && expected.2@ == "adopted activation path is not an owned socket"@)),
        5 => query_result == (0i32, 2i32) && path.0 && path.1 && path.2
            && ((path.3 != path.4 && !expected.1
                    && expected.2@ == "adopted activation path is not an owned socket"@)
                || (path.3 == path.4 && binding_result.0 && expected.1)),
        6 => query_result == (0i32, 2i32) && path.0 && path.1 && path.2 && path.3 == path.4
            && !binding_result.0 && binding_result.1 && expected.1,
        7 => query_result == (0i32, 2i32) && path.0 && path.1 && path.2 && path.3 == path.4
            && !binding_result.0 && !binding_result.1 && !expected.1
            && expected.2@ == message@,
        _ => false,
    },
{
    let query = |state: (u8, bool)| -> (out: ((u8, bool), (i32, i32)))
        ensures out == ((1u8, state.1 && state.0 == 0u8), query_result),
    { ((1, state.1 && state.0 == 0), query_result) };
    let last_error = |state: (u8, bool)| -> (out: ((u8, bool), String))
        ensures out.0 == (2u8, state.1 && state.0 == 1u8), out.1@ == message@,
    { ((2, state.1 && state.0 == 1), message.to_string()) };
    let address = |state: (u8, bool)| -> (out: ((u8, bool), Result<u64, String>))
        ensures out.0 == (3u8, state.1 && state.0 == 1u8),
            match out.1 {
                Ok(value) => path.0 && value == binding_result.2,
                Err(error) => !path.0 && error@ == message@,
            },
    {
        ((3, state.1 && state.0 == 1),
            if path.0 { Ok(binding_result.2) } else { Err(message.to_string()) })
    };
    let metadata = |state: (u8, bool)| -> (out: ((u8, bool), Result<(bool, u32), String>))
        ensures out.0 == (4u8, state.1 && state.0 == 3u8),
            match out.1 {
                Ok(fields) => path.1 && fields == (path.2, path.3),
                Err(error) => !path.1 && error@ == message@,
            },
    {
        ((4, state.1 && state.0 == 3),
            if path.1 { Ok((path.2, path.3)) } else { Err(message.to_string()) })
    };
    let user = |state: (u8, bool)| -> (out: ((u8, bool), u32))
        ensures out == ((5u8, state.1 && state.0 == 4u8), path.4),
    { ((5, state.1 && state.0 == 4), path.4) };
    let binding = |state: (u8, bool), address: &u64| -> (out: ((u8, bool), bool))
        ensures out == ((6u8, state.1 && state.0 == 5u8 && *address == binding_result.2),
            binding_result.1),
    { ((6, state.1 && state.0 == 5 && *address == binding_result.2), binding_result.1) };
    let describe = |state: (u8, bool), address: u64| -> (out: ((u8, bool), String))
        ensures out.0 == (7u8, state.1 && state.0 == 6u8 && address == binding_result.2),
            out.1@ == message@,
    { ((7, state.1 && state.0 == 6 && address == binding_result.2), message.to_string()) };
    let (state, result) = policy::validate_activation_sequence(
        (0u8, true), binding_result.0, 2i32, query, last_error,
        address, metadata, user, binding, describe);
    expect(state.1);
    expect(state.0 == expected.0);
    match result {
        Ok(()) => expect(expected.1),
        Err(error) => {
            expect(!expected.1);
            expect(policy::equal_text(error.as_str(), expected.2));
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn activation_query_errors_keep_errno_text_and_stop() {
    activation_case((-1, 2), (false, false, false, 0, 1), (true, false, 0),
        "errno", (2, false, "errno"));
    activation_case((1, 1), (true, true, true, 0, 0), (false, true, u64::MAX),
        "", (2, false, ""));
    activation_case((i32::MIN, 2), (true, true, true, 0, 0), (true, true, 0),
        "原始\0错误\n", (2, false, "原始\0错误\n"));
    activation_case((i32::MAX, i32::MIN), (true, true, true, 0, 0), (false, true, 0),
        "query\n", (2, false, "query\n"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn activation_requires_the_exact_datagram_kind_before_paths() {
    activation_case((0, 1), (false, false, false, 0, 1), (true, false, 0),
        "later", (1, false, "OS activation requires a Unix datagram socket"));
    activation_case((0, 0), (true, true, true, 0, 0), (false, true, 0),
        "later", (1, false, "OS activation requires a Unix datagram socket"));
    activation_case((0, -1), (true, true, true, 0, 0), (true, true, 0),
        "later", (1, false, "OS activation requires a Unix datagram socket"));
    activation_case((0, i32::MIN), (true, true, true, 0, 0), (false, true, 0),
        "later", (1, false, "OS activation requires a Unix datagram socket"));
    activation_case((0, i32::MAX), (true, true, true, 0, 0), (true, true, 0),
        "later", (1, false, "OS activation requires a Unix datagram socket"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn activation_path_failures_preserve_order_and_complete_errors() {
    activation_case((0, 2), (false, false, false, 0, 1), (true, false, 0),
        "地址\0错误\n", (3, false, "地址\0错误\n"));
    activation_case((0, 2), (false, true, true, 0, 0), (false, true, u64::MAX),
        "", (3, false, ""));
    activation_case((0, 2), (true, false, false, 0, 1), (true, false, 0),
        "元数据\0错误\n", (4, false, "元数据\0错误\n"));
    activation_case((0, 2), (true, false, true, 0, 0), (false, true, u64::MAX),
        "", (4, false, ""));
    activation_case((0, 2), (true, true, false, 0, 0), (true, true, 0),
        "later", (4, false, "adopted activation path is not an owned socket"));
    activation_case((0, 2), (true, true, false, u32::MAX, 0), (false, true, 0),
        "later", (4, false, "adopted activation path is not an owned socket"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn activation_named_sockets_still_require_exact_ownership() {
    activation_case((0, 2), (true, true, true, 0, u32::MAX), (true, true, 0),
        "later", (5, false, "adopted activation path is not an owned socket"));
    activation_case((0, 2), (true, true, true, u32::MAX, 0), (false, true, u64::MAX),
        "later", (5, false, "adopted activation path is not an owned socket"));
    activation_case((0, 2), (true, true, true, 0, 0), (true, false, 0),
        "must not describe", (5, true, ""));
    activation_case((0, 2), (true, true, true, u32::MAX, u32::MAX), (true, false, u64::MAX),
        "must not describe", (5, true, ""));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn activation_direct_binding_keeps_the_original_address_and_error() {
    activation_case((0, 2), (true, true, true, 0, 0), (false, true, 0),
        "must not describe", (6, true, ""));
    activation_case((0, 2), (true, true, true, u32::MAX, u32::MAX), (false, true, u64::MAX),
        "must not describe", (6, true, ""));
    activation_case((0, 2), (true, true, true, 7, 7), (false, false, u64::MAX),
        "原地址\0不匹配\n", (7, false, "原地址\0不匹配\n"));
    activation_case((0, 2), (true, true, true, 0, 0), (false, false, 0),
        "", (7, false, ""));
}

}

#[cfg(not(verus_keep_ghost))]
use super::*;
#[cfg(not(verus_keep_ghost))]
#[path = "support/fixture_coordinator.rs"]
mod chrono_git_boundary;
#[cfg(unix)]
#[cfg(not(verus_keep_ghost))]
use std::os::unix::net::UnixDatagram;
#[cfg(not(verus_keep_ghost))]
use std::sync::mpsc;
#[cfg(not(verus_keep_ghost))]
use std::time::Duration;

#[test]
#[cfg(unix)]
#[cfg(not(verus_keep_ghost))]
fn named_os_activation_accepts_forwarding_without_weakening_direct_binding() {
    let directory = tempfile::tempdir_in("/tmp").unwrap();
    let adopted = directory.path().join("adopted.sock");
    let forwarded = directory.path().join("forwarded.sock");
    let _os_path = UnixDatagram::bind(&adopted).unwrap();
    let descriptor = UnixDatagram::bind(&forwarded).unwrap();
    assert!(chrono_worktree::validate_maintenance_activation(&descriptor, &adopted, true).is_ok());
    assert!(
        chrono_worktree::validate_maintenance_activation(&descriptor, &adopted, false).is_err()
    );
    assert!(
        chrono_worktree::validate_maintenance_activation(&descriptor, &forwarded, false).is_ok()
    );
    let missing = directory.path().join("missing.sock");
    assert!(chrono_worktree::validate_maintenance_activation(&descriptor, &missing, true).is_err());
    let regular = directory.path().join("ordinary");
    fs::write(&regular, b"retain").unwrap();
    assert!(chrono_worktree::validate_maintenance_activation(&descriptor, &regular, true).is_err());
    let link = directory.path().join("alias");
    std::os::unix::fs::symlink(&adopted, &link).unwrap();
    assert!(chrono_worktree::validate_maintenance_activation(&descriptor, &link, true).is_err());
    assert_eq!(fs::read(&regular).unwrap(), b"retain");
    assert!(adopted.exists() && forwarded.exists());
}

#[test]
#[cfg(unix)]
#[cfg(not(verus_keep_ghost))]
fn named_activation_rejects_a_stream_descriptor() {
    use std::os::fd::{FromRawFd, IntoRawFd};
    let directory = tempfile::tempdir_in("/tmp").unwrap();
    let adopted = directory.path().join("adopted.sock");
    let _os_path = UnixDatagram::bind(&adopted).unwrap();
    let (stream, _peer) = std::os::unix::net::UnixStream::pair().unwrap();
    let descriptor = unsafe { UnixDatagram::from_raw_fd(stream.into_raw_fd()) };
    let error =
        chrono_worktree::validate_maintenance_activation(&descriptor, &adopted, true).unwrap_err();
    assert!(error.contains("datagram"), "{error}");
    assert!(adopted.exists());
}

#[cfg(not(verus_keep_ghost))]
fn blocked_round_keeps_business_progress(boundary: &str) {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = super::bounded_cleanup::main_host_mode("noop");
    let unrelated = h.parent.join("background-disposition");
    let linked = h.parent.join("legitimate-business");
    assert_eq!(
        h.invoke("feature", "background-disposition", &unrelated).0,
        0
    );
    assert_eq!(h.invoke("feature", "legitimate-business", &linked).0, 0);
    assert_eq!(h.auto("use", &["--operation", "use.consumer"]).0, 0);
    super::automatic::output(&h.root);
    let (code, handoff, error) = h.auto("finish", &["--path", unrelated.to_str().unwrap()]);
    assert_eq!(code, 0, "{handoff} {error}");
    assert!(unrelated.exists(), "handoff is not physical cleanup");
    assert!(handoff["drain"].as_array().unwrap().is_empty());
    // Hold the first actual full-footprint IO, before unrelated members consume
    // the readiness window. This is the same production measurement boundary.
    let blocked_path = (boundary != "footprint").then(|| unrelated.clone());
    let kind = boundary.to_owned();
    let root = h.root.clone();
    let (entered, ready) = mpsc::sync_channel(0);
    let (release, resume) = mpsc::sync_channel(0);
    let maintenance = std::thread::spawn(move || {
        let mut once = false;
        chrono_worktree::with_maintenance_observer(
            move |actual, path| {
                if !once && actual == kind && blocked_path.as_ref().is_none_or(|p| path == p) {
                    once = true;
                    if entered.send(path.to_owned()).is_ok() {
                        let _ = resume.recv();
                    }
                }
            },
            || {
                chrono_worktree::run(&[
                    "maintain".into(),
                    "--host-root".into(),
                    root.to_str().unwrap().into(),
                    "--config".into(),
                    POLICY.into(),
                ])
            },
        )
    });
    let reached = ready.recv_timeout(Duration::from_secs(5));
    drop(ready);
    let business = reached.as_ref().ok().map(|_| {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // Both original business endpoints must actually succeed while IO is withheld.
            let main = h.auto("use", &["--operation", "use.consumer"]);
            let other = h.auto(
                "use",
                &[
                    "--path",
                    linked.to_str().unwrap(),
                    "--operation",
                    "use.consumer",
                ],
            );
            (main, other)
        }))
    });
    // Always unblock and join, including failed assertions or late boundary arrival.
    let released = if reached.is_ok() {
        release.send(())
    } else {
        Ok(())
    };
    drop(release);
    let joined = maintenance.join();
    reached.expect("the declared IO boundary must be reached");
    released.expect("the original round must retain its boundary until release");
    let result = joined.unwrap();
    let (main, other) = business
        .unwrap()
        .unwrap_or_else(|error| std::panic::resume_unwind(error));
    for (code, report, error) in [main, other] {
        assert_eq!(
            code, 0,
            "business refused instead of progressing: {report} {error}"
        );
        assert_eq!(report["status"], "used");
        assert_eq!(report["managed_process"]["exit_code"], 0);
        assert!(report["managed_use_receipt"]["sha256"].is_string());
        assert!(report["drain"].as_array().unwrap().is_empty());
        let original = h
            .root
            .join(report["managed_process_original"]["path"].as_str().unwrap());
        let intent = json(&fs::read(original.join("intent.json")).unwrap()).unwrap();
        assert_eq!(intent["argv"], value!(["noop"]));
        assert_eq!(intent["stdin_sha256"], sha256(&[]));
        assert_eq!(fs::read(original.join("stdout")).unwrap(), b"");
        assert_eq!(fs::read(original.join("stderr")).unwrap(), b"");
    }
    let report = json(result.stdout.as_bytes()).unwrap();
    assert_eq!(report["owned_work_joined"], true, "{report}");
    assert!(report["complete_inventory"]["path"].is_string());
    assert!(
        !unrelated.exists(),
        "the original eventual cleanup assertion remains"
    );
    assert!(linked.join("payload").is_file());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn legitimate_main_and_linked_business_progress_during_unrelated_disposition() {
    blocked_round_keeps_business_progress("disposition");
}
#[test]
#[cfg(not(verus_keep_ghost))]
fn legitimate_main_and_linked_business_progress_during_full_footprint_io() {
    blocked_round_keeps_business_progress("footprint");
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn live_consumers_can_complete_before_the_maintenance_parent_is_published() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = super::bounded_cleanup::main_host_mode("noop");
    adopt_lifecycle_retention(&h);
    let policy_path = h.root.join(".chrono-harness/cleanup.json");
    let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
    policy["retained_producers"] = value!([{"id":"progress-fixture",
        "directory":".chrono-harness/state/progress-fixture/", "temporary":true,
        "generated_outputs":[]}]);
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let linked = h.parent.join("live-business");
    assert_eq!(h.invoke("feature", "live-business", &linked).0, 0);
    assert_eq!(h.auto("use", &["--operation", "use.consumer"]).0, 0);
    super::automatic::output(&h.root);
    super::automatic::output(&linked);
    let ledger = h.ledger();
    let holders: Vec<_> = ledger["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            let held =
                fs::File::open(h.root.join(entry["ownership"]["path"].as_str().unwrap())).unwrap();
            held.lock_shared().unwrap();
            held
        })
        .collect();
    assert_eq!(holders.len(), 2);
    let root = h.root.clone();
    let (entered, ready) = mpsc::sync_channel(0);
    let (release, resume) = mpsc::sync_channel(0);
    let maintenance = std::thread::spawn(move || {
        let mut once = false;
        chrono_worktree::with_maintenance_observer(
            move |kind, _| {
                if !once && kind == "producer" {
                    once = true;
                    if entered.send(()).is_ok() {
                        let _ = resume.recv();
                    }
                }
            },
            || {
                chrono_worktree::run(&[
                    "maintain".into(),
                    "--host-root".into(),
                    root.to_str().unwrap().into(),
                    "--config".into(),
                    POLICY.into(),
                ])
            },
        )
    });
    // The sender is dropped if maintenance ends before this boundary. Await
    // that actual notification instead of imposing a second readiness timer.
    let reached = ready.recv();
    drop(ready);
    let business = reached.as_ref().ok().map(|_| {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            for entry in h.ledger()["entries"].as_array().unwrap() {
                assert_eq!(entry["cleanup"]["stage"], "waiting-for-consumers");
            }
            let state = retention_state::snapshot(&h.root);
            let parents: Vec<_> = state["outputs"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|output| output["path"].as_str())
                .filter(|path| {
                    path.starts_with(".chrono-harness/state/worktrees/maintain-")
                        && path.ends_with(".json")
                })
                .collect();
            assert_eq!(
                parents.len(),
                1,
                "the actual unpublished round owns its report"
            );
            let parent = parents[0];
            let mut address = h
                .root
                .join(".chrono-harness/state/output-retention-v1/paths");
            for component in parent.split('/') {
                address.push(sha256(component.as_bytes()));
            }
            // Hold the exact report path at its publication boundary. Business
            // must finish without acquiring this unrelated publisher's gate.
            let publishing =
                chrono_harness::ownership::DirectoryLease::acquire_local(&address, true, None)
                    .unwrap()
                    .unwrap();
            let main = h.auto("use", &["--operation", "use.consumer"]);
            let other = h.auto(
                "use",
                &[
                    "--path",
                    linked.to_str().unwrap(),
                    "--operation",
                    "use.consumer",
                ],
            );
            publishing.stable().unwrap();
            drop(publishing);
            (main, other)
        }))
    });
    let released = if reached.is_ok() {
        release.send(())
    } else {
        Ok(())
    };
    drop(release);
    let joined = maintenance.join();
    reached.expect("the parent must remain unpublished after both live-consumer observations");
    released.expect("the maintenance parent remains at its original completion boundary");
    let (main, other) = business
        .unwrap()
        .unwrap_or_else(|error| std::panic::resume_unwind(error));
    for (code, report, error) in [main, other] {
        assert_eq!(
            code, 0,
            "business depends on an unpublished maintenance parent: {report} {error}"
        );
        assert_eq!(report["status"], "used");
        assert_eq!(report["managed_process"]["exit_code"], 0);
        assert_eq!(report["managed_process"]["owned_work_joined"], true);
        assert!(report["managed_use_receipt"]["sha256"].is_string());
    }
    let output = joined.unwrap();
    let report = json(output.stdout.as_bytes()).unwrap();
    assert_eq!(output.exit_code, 0, "{report}");
    assert_eq!(report["owned_work_joined"], true);
    assert!(report["complete_inventory"]["receipt"]["sha256"].is_string());
    for path in [&h.root, &linked] {
        assert!(path.join("output λ/data").is_file());
        assert!(path.join("payload").is_file());
    }
    drop(holders);
}

#[test]
#[cfg(unix)]
#[cfg(not(verus_keep_ghost))]
fn merged_events_and_new_events_during_a_round_are_observed_without_empty_retry() {
    let (executor, sender) = UnixDatagram::pair().unwrap();
    sender.send(b"first").unwrap();
    sender.send(b"duplicate").unwrap();
    let mut rounds = 0;
    chrono_worktree::serve_maintenance_socket(&executor, || {
        rounds += 1;
        if rounds == 1 {
            sender.send(b"new-generation-during-round").unwrap();
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(rounds, 2);
}

#[test]
#[cfg(unix)]
#[cfg(not(verus_keep_ghost))]
fn empty_activation_exits_and_a_later_event_survives_an_interrupted_round() {
    let (executor, sender) = UnixDatagram::pair().unwrap();
    chrono_worktree::serve_maintenance_socket(&executor, || panic!("empty round")).unwrap();
    sender.send(b"first").unwrap();
    let failure = chrono_worktree::serve_maintenance_socket(&executor, || {
        sender.send(b"new-work").unwrap();
        Err("original IO failure".into())
    });
    assert_eq!(failure.unwrap_err(), "original IO failure");
    let mut recovered = 0;
    chrono_worktree::serve_maintenance_socket(&executor, || {
        recovered += 1;
        Ok(())
    })
    .unwrap();
    assert_eq!(recovered, 1);
}

#[test]
#[cfg(unix)]
#[cfg(not(verus_keep_ghost))]
fn empty_service_activation_never_opens_the_executor_lease() {
    let directory = tempfile::tempdir_in("/tmp").unwrap();
    let root = fs::canonicalize(directory.path()).unwrap();
    let state = root.join(".chrono-harness/state");
    fs::create_dir_all(&state).unwrap();
    let policy = chrono_harness::maintenance_signal::Policy {
        schema: "chrono-unified-maintenance/v2".into(),
        state_directory: ".chrono-harness/state/owner/".into(),
        socket: ".chrono-harness/state/s".into(),
    };
    fs::write(
        root.join(chrono_harness::maintenance_signal::POLICY_PATH),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let os = UnixDatagram::bind(state.join("s")).unwrap();
    let report = chrono_worktree::run_maintenance_service_socket(&root, POLICY, os).unwrap();
    assert_eq!(report["status"], "maintenance-queue-empty");
    assert!(!state.join("owner").exists());
}

// Unix socket path limits are fixed OS inputs. Use a short private fixture
// path instead of changing any production limits or following a root alias.
#[cfg(unix)]
#[cfg(not(verus_keep_ghost))]
fn socket_host(mode: &str) -> Host {
    let directory = tempfile::Builder::new()
        .prefix("um-")
        .tempdir_in("/tmp")
        .unwrap();
    super::bounded_cleanup::configure_main(
        Host::in_directory("payload", HostDirectory::Ephemeral(directory)),
        mode,
    )
}

#[cfg(unix)]
#[cfg(not(verus_keep_ghost))]
fn adopt_activation(h: &Host) -> std::path::PathBuf {
    let policy = chrono_harness::maintenance_signal::Policy {
        schema: "chrono-unified-maintenance/v2".into(),
        state_directory: ".chrono-harness/state/automatic-cleanup/".into(),
        socket: ".chrono-harness/state/s".into(),
    };
    let path = chrono_harness::maintenance_signal::POLICY_PATH;
    fs::write(h.root.join(path), serde_json::to_vec(&policy).unwrap()).unwrap();
    let mut fm = json(&fs::read(h.root.join(FM)).unwrap()).unwrap();
    fm["files"]
        .as_array_mut()
        .unwrap()
        .push(file(path, value!([])));
    fs::write(h.root.join(FM), serde_json::to_vec(&fm).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    fs::create_dir_all(h.root.join(policy.state_directory.trim_end_matches('/'))).unwrap();
    policy.socket_path(&h.root).unwrap()
}

#[test]
#[cfg(unix)]
#[cfg(not(verus_keep_ghost))]
fn linked_business_and_its_nested_runner_send_retention_hints_to_the_coordinator() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = socket_host("nested-maintenance-hint");
    let path = adopt_activation(&h);
    let os = UnixDatagram::bind(&path).unwrap();
    let linked = h.parent.join("linked-business");
    assert_eq!(h.invoke("feature", "linked-business", &linked).0, 0);
    os.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let receiver = std::thread::spawn(move || {
        let mut bytes = [0; 64];
        loop {
            match os.recv(&mut bytes) {
                Ok(count) if &bytes[..count] == b"fixture-release" => return true,
                Ok(_) => (),
                Err(_) => return false,
            }
        }
    });
    let (code, report, error) = h.auto(
        "use",
        &[
            "--path",
            linked.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    let observed = receiver.join().unwrap();
    assert_eq!(code, 0, "{report} {error}");
    let stdout = h
        .root
        .join(report["managed_process_original"]["path"].as_str().unwrap())
        .join("stdout");
    let receipt = json(&fs::read(stdout).unwrap()).unwrap();
    assert_eq!(receipt["accepted"], true, "{receipt}");
    assert_eq!(receipt["socket"], value!(path));
    assert_eq!(receipt["activation_root"], value!(h.root));
    assert_eq!(receipt["round_completion"], "not-established");
    assert!(
        observed,
        "the actual coordinator socket must receive the nested hint"
    );
    assert!(!linked.join(".chrono-harness/state/s").exists());
}

#[test]
#[cfg(unix)]
#[cfg(not(verus_keep_ghost))]
fn explicit_competitor_requests_the_active_executor_without_a_parallel_round() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = socket_host("noop");
    let path = adopt_activation(&h);
    let os = UnixDatagram::bind(&path).unwrap();
    os.set_nonblocking(true).unwrap();
    let target = h.parent.join("singleton-disposition");
    assert_eq!(h.invoke("feature", "singleton-disposition", &target).0, 0);
    assert_eq!(h.auto("finish", &["--path", target.to_str().unwrap()]).0, 0);
    let mut bytes = [0; 64];
    while os.recv(&mut bytes).is_ok() {} // Drain this finite setup batch before the tested round.
    chrono_harness::maintenance_signal::send(&path, b"first-round").unwrap();
    let root = h.root.clone();
    let inherited_socket = os.try_clone().unwrap();
    let (entered, ready) = mpsc::sync_channel(0);
    let (release, resume) = mpsc::sync_channel(0);
    let task = std::thread::spawn(move || {
        let mut blocked = false;
        chrono_worktree::with_maintenance_observer(
            move |kind, path| {
                if !blocked && kind == "disposition" && path == target {
                    blocked = true;
                    entered.send(()).unwrap();
                    resume.recv().unwrap();
                }
            },
            || chrono_worktree::run_maintenance_service_socket(&root, POLICY, inherited_socket),
        )
    });
    let reached = ready.recv_timeout(Duration::from_secs(5));
    drop(ready);
    let competing = reached.as_ref().ok().map(|_| {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| h.auto("maintain", &[])))
    });
    let released = if reached.is_ok() {
        release.send(())
    } else {
        Ok(())
    };
    drop(release);
    let joined = task.join();
    reached.expect("actual executor must reach its disposition");
    released.expect("the original executor must remain at its boundary");
    let first = joined.unwrap();
    let (code, report, error) = competing
        .unwrap()
        .unwrap_or_else(|error| std::panic::resume_unwind(error));
    assert_eq!(code, 0, "{report} {error}");
    assert_eq!(report["status"], "maintenance-requested");
    assert_eq!(report["round_started"], false);
    assert_eq!(report["maintenance_request"]["accepted"], true);
    assert!(report.get("complete_inventory").is_none());
    assert_eq!(first.unwrap()["status"], "maintenance-queue-empty");
    assert_eq!(
        os.recv(&mut bytes).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    let actual_rounds: Vec<Value> = fs::read_dir(h.root.join(".chrono-harness/state/worktrees"))
        .unwrap()
        .filter_map(|entry| fs::read(entry.ok()?.path()).ok())
        .filter_map(|bytes| json(&bytes).ok())
        .filter(|report| report["operation"] == "maintain" && report["round_started"] == true)
        .collect();
    assert_eq!(
        actual_rounds.len(),
        2,
        "explicit competition only submits one next round"
    );
    assert!(
        actual_rounds
            .iter()
            .any(|report| report["maintenance_trigger"] == "event")
    );
    for report in actual_rounds {
        assert_eq!(report["status"], "maintained", "{report}");
        assert_eq!(report["owned_work_joined"], true);
        assert!(report["complete_inventory"]["receipt"]["sha256"].is_string());
    }
}

#[test]
#[cfg(unix)]
#[cfg(not(verus_keep_ghost))]
fn activated_service_observes_work_after_the_explicit_owner_releases_the_lease() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = socket_host("noop");
    let path = adopt_activation(&h);
    let os = UnixDatagram::bind(&path).unwrap();
    os.set_nonblocking(true).unwrap();
    let target = h.parent.join("after-explicit-owner");
    assert_eq!(h.invoke("feature", "after-explicit-owner", &target).0, 0);
    super::automatic::output(&target);
    assert_eq!(h.auto("finish", &["--path", target.to_str().unwrap()]).0, 0);
    let lease = h
        .root
        .join(".chrono-harness/state/automatic-cleanup/maintenance.lease");
    let explicit = chrono_harness::ownership::Lease::acquire(&lease, None, true, true, None)
        .unwrap()
        .expect("controlled explicit executor owns the actual singleton");
    chrono_harness::maintenance_signal::send(&path, b"pending-work").unwrap();
    let report_directory = h.root.join(".chrono-harness/state/worktrees");
    let before: std::collections::BTreeSet<_> = fs::read_dir(&report_directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    let root = h.root.clone();
    let inherited_socket = os.try_clone().unwrap();
    let (entering, ready) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        chrono_worktree::with_maintenance_observer(
            move |kind, _| {
                if kind == "service-before-lease" {
                    entering.send(()).unwrap();
                }
            },
            || chrono_worktree::run_maintenance_service_socket(&root, POLICY, inherited_socket),
        )
    });
    let entered = ready.recv_timeout(Duration::from_secs(5));
    let retained = target.join("output λ/data").exists();
    let still_exclusive = explicit.stable();
    let after: std::collections::BTreeSet<_> = fs::read_dir(&report_directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    drop(explicit);
    let recovered = worker.join().unwrap().unwrap();
    entered.unwrap();
    still_exclusive.unwrap();
    assert_eq!(before, after);
    assert!(retained, "the explicit executor excludes disposal");
    assert_eq!(recovered["status"], "maintenance-queue-empty");
    assert!(
        !target.join("output λ/data").exists(),
        "queued event must recover pending work without another wake"
    );
    let rounds: Vec<Value> = fs::read_dir(&report_directory)
        .unwrap()
        .filter_map(|entry| fs::read(entry.ok()?.path()).ok())
        .filter_map(|bytes| json(&bytes).ok())
        .filter(|report| report["operation"] == "maintain" && report["round_started"] == true)
        .collect();
    assert_eq!(rounds.len(), 1);
    assert_eq!(rounds[0]["maintenance_trigger"], "event");
    assert_eq!(rounds[0]["status"], "maintained", "{}", rounds[0]);
    assert_eq!(rounds[0]["owned_work_joined"], true);
    assert!(rounds[0]["complete_inventory"]["receipt"]["sha256"].is_string());
}

#[test]
#[cfg(unix)]
#[cfg(not(verus_keep_ghost))]
fn wake_failure_preserves_successful_business_and_its_original_streams() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = socket_host("ordinary");
    let _absent_socket = adopt_activation(&h);
    let (code, report, error) = h.auto("use", &["--operation", "use.consumer"]);
    assert_eq!(code, 0, "{report} {error}");
    assert_eq!(report["status"], "used");
    assert_eq!(report["managed_process"]["exit_code"], 0);
    assert!(error.contains("wake-failed"), "{error}");
    let stream = h
        .root
        .join(report["managed_process_original"]["path"].as_str().unwrap())
        .join("stdout");
    assert_eq!(fs::read(stream).unwrap(), b"ordinary-work");
    assert_eq!(
        report["maintenance_request"]["round_completion"],
        "not-established"
    );
    assert!(
        h.ledger()["entries"][0]["ownership"]["cache_pending"]
            .as_bool()
            .unwrap()
    );
}

#[test]
#[cfg(unix)]
#[cfg(not(verus_keep_ghost))]
fn wake_failure_keeps_the_original_business_failure_and_stderr() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = socket_host("rebuild-error");
    let _absent_socket = adopt_activation(&h);
    let (code, report, error) = h.auto("use", &["--operation", "use.consumer"]);
    assert_ne!(code, 0, "{report} {error}");
    assert_eq!(report["managed_process"]["exit_code"], 23);
    assert_eq!(report["managed_command_failed"], true);
    assert!(error.contains("wake-failed"), "{error}");
    let stream = h
        .root
        .join(report["managed_process_original"]["path"].as_str().unwrap())
        .join("stderr");
    assert_eq!(fs::read(stream).unwrap(), b"original-error");
    assert!(h.root.join("output λ/cache").is_file());
}
