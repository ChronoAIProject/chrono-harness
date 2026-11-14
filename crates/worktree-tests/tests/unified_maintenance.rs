//! Independent expectations for the unified owner's business/maintenance boundary.
use super::*;
#[path = "support/fixture_coordinator.rs"]
mod chrono_git_boundary;
#[cfg(unix)]
use std::os::unix::net::UnixDatagram;
use std::sync::mpsc;
use std::time::Duration;

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
    let blocked_path = if boundary == "footprint" {
        h.root.join("output λ")
    } else {
        unrelated.clone()
    };
    let kind = boundary.to_owned();
    let root = h.root.clone();
    let (entered, ready) = mpsc::sync_channel(0);
    let (release, resume) = mpsc::sync_channel(0);
    let maintenance = std::thread::spawn(move || {
        let mut once = false;
        chrono_worktree::with_maintenance_observer(
            move |actual, path| {
                if !once && actual == kind && path == blocked_path {
                    once = true;
                    entered.send(()).unwrap();
                    resume.recv().unwrap();
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
fn legitimate_main_and_linked_business_progress_during_unrelated_disposition() {
    blocked_round_keeps_business_progress("disposition");
}
#[test]
fn legitimate_main_and_linked_business_progress_during_full_footprint_io() {
    blocked_round_keeps_business_progress("footprint");
}

#[cfg(unix)]
fn policy(idle: Option<u64>, timer: u64) -> chrono_harness::maintenance_signal::Policy {
    chrono_harness::maintenance_signal::Policy {
        schema: "chrono-unified-maintenance/v1".into(),
        state_directory: ".chrono-harness/state/maintenance/".into(),
        socket: ".chrono-harness/state/maintenance/wake.sock".into(),
        idle_seconds: idle,
        timer_seconds: timer,
    }
}

#[test]
#[cfg(unix)]
fn merged_events_and_new_events_during_a_round_are_observed_without_empty_retry() {
    let (executor, sender) = UnixDatagram::pair().unwrap();
    sender.send(b"first").unwrap();
    sender.send(b"duplicate").unwrap();
    let mut rounds = Vec::new();
    chrono_worktree::serve_maintenance_socket(
        &executor,
        &policy(Some(1), 3600),
        |trigger, missed| {
            rounds.push((trigger.to_owned(), missed));
            if rounds.len() == 1 {
                sender.send(b"new-generation-during-round").unwrap();
            }
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(rounds, vec![("event".into(), 0), ("event".into(), 0)]);
}

#[test]
#[cfg(unix)]
fn resident_service_handles_timer_and_preserves_original_round_failure() {
    let (executor, _sender) = UnixDatagram::pair().unwrap();
    let mut rounds = Vec::new();
    let result = chrono_worktree::serve_maintenance_socket(
        &executor,
        &policy(None, 1),
        |trigger, missed| {
            rounds.push((trigger.to_owned(), missed));
            if trigger == "timer" {
                return Err("original timer IO failure".into());
            }
            Ok(())
        },
    );
    assert_eq!(result.unwrap_err(), "original timer IO failure");
    assert_eq!(
        rounds,
        vec![("os-activation".into(), 0), ("timer".into(), 0)]
    );
}

#[test]
#[cfg(unix)]
fn long_rounds_do_not_hide_missed_resident_timer_opportunities() {
    let (executor, _sender) = UnixDatagram::pair().unwrap();
    let (_io_owner, unfinished_io) = mpsc::channel::<()>();
    let mut timers = Vec::new();
    let result = chrono_worktree::serve_maintenance_socket(
        &executor,
        &policy(None, 1),
        |trigger, missed| {
            if trigger == "timer" {
                timers.push(missed);
                if timers.len() == 1 {
                    assert_eq!(
                        unfinished_io.recv_timeout(Duration::from_millis(2100)),
                        Err(mpsc::RecvTimeoutError::Timeout)
                    );
                } else {
                    return Err("joined observation finished".into());
                }
            }
            Ok(())
        },
    );
    assert_eq!(result.unwrap_err(), "joined observation finished");
    assert_eq!(timers.len(), 2);
    assert!(
        timers[1] >= 1,
        "the original cadence must expose missed opportunities"
    );
}

#[test]
#[cfg(unix)]
fn idle_exit_leaves_os_owned_socket_usable_for_later_activation() {
    let (executor, sender) = UnixDatagram::pair().unwrap();
    let mut first = Vec::new();
    chrono_worktree::serve_maintenance_socket(&executor, &policy(Some(1), 3600), |trigger, _| {
        first.push(trigger.to_owned());
        Ok(())
    })
    .unwrap();
    sender.send(b"event-after-exit").unwrap();
    let mut later = Vec::new();
    chrono_worktree::serve_maintenance_socket(&executor, &policy(Some(1), 3600), |trigger, _| {
        later.push(trigger.to_owned());
        Ok(())
    })
    .unwrap();
    assert_eq!(first, ["os-activation"]);
    assert_eq!(later, ["event"]);
    // Product socket custody only. Actual launchd/systemd activation is a host obligation.
}

// Unix socket path limits are fixed OS inputs. Use a short private fixture
// path instead of changing any production limits or following a root alias.
#[cfg(unix)]
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
fn adopt_activation(h: &Host) -> std::path::PathBuf {
    let policy = chrono_harness::maintenance_signal::Policy {
        schema: "chrono-unified-maintenance/v1".into(),
        state_directory: ".chrono-harness/state/automatic-cleanup/".into(),
        socket: ".chrono-harness/state/s".into(),
        idle_seconds: Some(1),
        timer_seconds: 3600,
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
fn explicit_competitor_requests_the_resident_executor_without_a_parallel_round() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = socket_host("noop");
    let path = adopt_activation(&h);
    let os = UnixDatagram::bind(path).unwrap();
    os.set_nonblocking(true).unwrap();
    let target = h.parent.join("singleton-disposition");
    assert_eq!(h.invoke("feature", "singleton-disposition", &target).0, 0);
    assert_eq!(h.auto("finish", &["--path", target.to_str().unwrap()]).0, 0);
    let mut bytes = [0; 64];
    while os.recv(&mut bytes).is_ok() {} // Drain this finite setup batch before the tested round.
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
    assert_eq!(first.unwrap()["status"], "maintenance-idle-exit");
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
