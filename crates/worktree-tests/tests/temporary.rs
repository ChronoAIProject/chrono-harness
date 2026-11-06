use super::*;

// Capacity/recovery contract: a subsequent allocation is a real ordinary
// consumer of this producer, and must recover an excluded predecessor without
// turning its unknown original outcome into success. Live siblings survive.
#[test]
fn capacity_ordinary_allocation_recovers_excluded_predecessor() {
    let h = host();
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let previous =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "previous-")
            .unwrap();
    let cache = previous.path().join("cache 空白");
    fs::create_dir(&cache).unwrap();
    fs::write(cache.join("generated"), b"rebuildable").unwrap();
    let path = previous.path().to_owned();
    // Simulate an interrupted allocation: Drop must not establish its outcome.
    drop(previous);
    let live =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "next-")
            .unwrap();
    assert!(
        !cache.exists(),
        "ordinary allocation left eligible predecessor output"
    );
    assert!(path.is_dir(), "allocation source/evidence must survive");
    assert!(live.path().is_dir());
    let state = producer_state(&store);
    let old = state["objects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["path"] == path.file_name().unwrap().to_str().unwrap())
        .unwrap();
    let intent =
        json(&fs::read(store.join(old["intent"]["path"].as_str().unwrap())).unwrap()).unwrap();
    assert_eq!(intent["producer_outcome"], "not-established");
}

fn capacity_host(change: impl FnOnce(&mut Value)) -> Host {
    let h = host();
    let path = h.root.join(".chrono-harness/cleanup.json");
    let mut policy = json(&fs::read(&path).unwrap()).unwrap();
    let capacity = &mut policy["retained_producers"][0]["capacity"];
    *capacity = value!({"max_generated_bytes":8,"max_copy_bytes":8,"max_capture_bytes":64,
        "max_captures":2,"min_available_bytes":0,"max_records_per_round":2,
        "max_record_snapshot":128,"max_metadata_bytes_per_round":1048576,"max_millis_per_round":2000,"max_entries_per_checkpoint":128});
    change(capacity);
    fs::write(path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
    h
}

fn adopt_directory_cleanup(h: &Host) {
    let path = h.root.join(".chrono-harness/cleanup.json");
    let mut policy = json(&fs::read(&path).unwrap()).unwrap();
    policy["retained_producers"][0]["disposable_directory"] = value!(true);
    fs::write(path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
}

#[test]
fn directory_cleanup_reclaims_synthetic_repository_after_actual_consumers_release() {
    use std::os::unix::fs::symlink;

    let h = host();
    adopt_directory_cleanup(&h);
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let mut allocation =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "owned-")
            .unwrap();
    let root = allocation.path().to_owned();
    let record = producer_record_path(&store, &root);
    let repo = root.join("synthetic repository");
    fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "--initial-branch=dev", "-q"]);
    fs::write(repo.join("fixture.rs"), b"synthetic fixture input").unwrap();
    commit(&repo);
    // No individual cleanup declarations for the repository or these members.
    for index in 0..32 {
        fs::write(root.join(format!("scratch-{index}")), b"temporary").unwrap();
    }
    let outside = h.parent.join("external-source.rs");
    fs::write(&outside, b"real external source").unwrap();
    symlink(&outside, root.join("external-link")).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    command.arg("capture-once");
    let output = allocation.capture_output(&mut command).unwrap();
    drop(command);
    assert_eq!(output.status.code(), Some(7));
    let capture = capture_directory(&root);
    let result = fs::read(capture.join("result.json")).unwrap();
    let artifact =
        chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", &root).unwrap();
    artifact
        .acquire("diagnostic", "inspect actual failed original")
        .unwrap();
    let report = allocation.finish().unwrap();
    assert!(
        root.is_dir(),
        "released owner lost an active diagnostic: {report}"
    );
    assert_eq!(fs::read(capture.join("result.json")).unwrap(), result);
    artifact
        .release("diagnostic", "original failure inspected")
        .unwrap();
    let recovered = allocation.finish().unwrap();
    assert!(recovered["recovery_error"].is_null(), "{recovered}");
    assert!(
        !root.exists(),
        "released owned fixture directory accumulated"
    );
    assert!(
        !record.exists(),
        "released directory registration accumulated"
    );
    assert_eq!(fs::read(&outside).unwrap(), b"real external source");
    assert_eq!(json(&result).unwrap()["exit_code"], 7);
}

#[test]
fn directory_cleanup_policy_does_not_reclassify_existing_source_directories() {
    let h = host();
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "old-")
            .unwrap();
    let old = allocation.path().to_owned();
    fs::write(old.join("unique-source.rs"), b"unsaved source").unwrap();
    drop(allocation);
    assert_eq!(h.auto("use", &["--operation", "use.consumer"]).0, 0);
    adopt_directory_cleanup(&h);
    let (code, report, error) = h.auto(
        "migrate",
        &[
            "--path",
            h.root.to_str().unwrap(),
            "--partition-producer",
            "test-allocation",
        ],
    );
    assert_eq!(code, 0, "{report} {error}");
    let mut next =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "new-")
            .unwrap();
    let new = next.path().to_owned();
    fs::write(new.join("scratch"), b"temporary").unwrap();
    next.finish().unwrap();
    let mut reentry = capacity_reenter_after_release(&store, &outputs, &new);
    reentry.finish().unwrap();
    assert!(!new.exists(), "new directory ownership was not applied");
    assert_eq!(
        fs::read(old.join("unique-source.rs")).unwrap(),
        b"unsaved source"
    );
}

#[test]
fn directory_cleanup_custodian_adopts_one_old_directory_without_rewriting_its_intent() {
    let h = host();
    assert_eq!(h.auto("use", &["--operation", "use.consumer"]).0, 0);
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "old-owned-")
            .unwrap();
    let root = allocation.path().to_owned();
    let repo = root.join("synthetic repository");
    fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "--initial-branch=dev", "-q"]);
    fs::write(repo.join("fixture.rs"), b"old synthetic fixture").unwrap();
    commit(&repo);
    fs::write(root.join("unregistered-scratch"), b"temporary").unwrap();
    let artifact =
        chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", &root).unwrap();
    artifact
        .acquire("diagnostic", "inspect old fixture")
        .unwrap();
    drop(allocation);
    let record = producer_record_path(&store, &root);
    let object = json(&fs::read(&record).unwrap()).unwrap()["object"].clone();
    let intent = store.join(object["intent"]["path"].as_str().unwrap());
    let original = fs::read(&intent).unwrap();
    // A valid historic sweep can predate this allocation. Custody must schedule
    // its exact directory rather than waiting for that sweep to wrap around.
    fs::create_dir_all(store.join("recovery-state")).unwrap();
    fs::write(
        store.join("recovery-state/cursor.json"),
        serde_json::to_vec(&value!({"bucket":0,"after":"","round_started":1,"pending":[]})).unwrap(),
    ).unwrap();
    adopt_directory_cleanup(&h);
    let (code, report, error) = h.auto(
        "migrate",
        &[
            "--path",
            h.root.to_str().unwrap(),
            "--partition-producer",
            "test-allocation",
        ],
    );
    assert_eq!(code, 0, "{report} {error}");
    let live = chrono_harness::ownership::Lease::acquire(
        &store.join(object["cache_lease"]["path"].as_str().unwrap()),
        Some(object["cache_lease"]["id"].as_str().unwrap()),
        false,
        false,
        None,
    )
    .unwrap()
    .unwrap();
    let mut plan = value!({"schema":"chrono-fixture-body-custody/v1",
        "head":git(&h.root,&["rev-parse","HEAD"]),"current_consumers_released":true,
        "reason":"current fixture custodian inspected originals and releases this synthetic directory",
        "objects":[{"producer":"test-allocation","allocation":object["path"],
            "identity":object["identity"],"intent":object["intent"],"members":[],
            "adopt_directory":true,"release_originals":true}]});
    let path = ".chrono-harness/state/directory-custody.json";
    let adopt = |plan: &Value| {
        fs::write(h.root.join(path), serde_json::to_vec(plan).unwrap()).unwrap();
        h.auto(
            "migrate",
            &["--path", h.root.to_str().unwrap(), "--adopt-fixtures", path],
        )
    };
    assert_eq!(adopt(&plan).0, 2, "live capability must exclude custody");
    drop(live);
    assert_eq!(adopt(&plan).0, 2, "diagnostic must separately release");
    artifact
        .release("diagnostic", "old fixture inspected")
        .unwrap();
    plan["objects"][0]["identity"] = value!("wrong physical identity");
    assert_eq!(adopt(&plan).0, 2);
    plan["objects"][0]["identity"] = object["identity"].clone();
    let (code, report, error) = adopt(&plan);
    assert_eq!(code, 0, "{report} {error}");
    assert!(
        root.is_dir(),
        "custody must not itself dispose the directory"
    );
    assert_eq!(fs::read(&intent).unwrap(), original);
    let adopted = json(&fs::read(&record).unwrap()).unwrap();
    assert!(adopted["object"]["directory_custody"].is_object());
    assert_ne!(adopted["object"]["disposable_directory"], true);
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(
        !root.exists() && !record.exists(),
        "adopted old directory accumulated"
    );
    assert_eq!(
        json(&original).unwrap()["producer_outcome"],
        "not-established"
    );
}

#[test]
fn directory_cleanup_preserves_live_capability_and_staged_source_then_resumes() {
    let h = host();
    adopt_directory_cleanup(&h);
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let mut allocation =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "guarded-")
            .unwrap();
    let root = allocation.path().to_owned();
    fs::write(root.join("source.rs"), b"staged host source").unwrap();
    let capability = allocation.capability().unwrap();
    let report = allocation.finish().unwrap();
    assert!(
        root.is_dir(),
        "live capability lost its directory: {report}"
    );
    drop(capability);

    let source = root.join("source.rs");
    let relative = source.strip_prefix(&h.root).unwrap().to_str().unwrap();
    git(&h.root, &["add", "-f", "--", relative]);
    let rejected = allocation.finish().unwrap();
    let rows = rejected["producer_objects"].as_array().unwrap();
    assert!(
        rows.iter().any(|r| r["original_retirement"]["error"]
            .as_str()
            .is_some_and(|e| e.contains("committed/index source"))),
        "{rejected}"
    );
    assert_eq!(fs::read(&source).unwrap(), b"staged host source");
    let record = producer_record_path(&store, &root);
    let before = fs::read(&record).unwrap();
    let object = json(&before).unwrap();
    assert_eq!(object["object"]["retiring"], true);
    let intent = store.join(object["object"]["intent"]["path"].as_str().unwrap());
    let intent_bytes = fs::read(&intent).unwrap();
    assert_eq!(
        json(&intent_bytes).unwrap()["producer_outcome"],
        "not-established"
    );

    git(&h.root, &["rm", "--cached", "-q", "--", relative]);
    let resumed = allocation.finish().unwrap();
    assert!(resumed["recovery_error"].is_null(), "{resumed}");
    assert!(!root.exists() && !record.exists());
    assert_eq!(
        json(&intent_bytes).unwrap()["producer_outcome"],
        "not-established"
    );
    assert!(
        rows.iter()
            .any(|r| r["original_retirement"]["status"] == "failed")
    );
}

#[test]
fn directory_cleanup_keeps_shared_input_until_its_last_directory_releases() {
    let h = host();
    adopt_directory_cleanup(&h);
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let source = h.parent.join("real-input.rs");
    fs::write(&source, b"real source outside temporary directories").unwrap();
    let mut first =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "first-")
            .unwrap();
    let mut second =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "second-")
            .unwrap();
    let mut originals = vec![];
    for allocation in [&first, &second] {
        let destination = allocation.path().join("copied.rs");
        originals.push(
            chrono_worktree::TemporaryHost::register_copy(
                &store,
                "test-allocation",
                &source,
                &destination,
                None,
                "actual fixture input",
            )
            .unwrap(),
        );
        fs::copy(&source, &destination).unwrap();
    }
    assert_eq!(originals[0], originals[1]);
    fs::write(first.path().join("copied.rs"), b"synthetic fixture mutation").unwrap();
    let report = first.finish().unwrap();
    assert!(
        !first.path().exists(),
        "released first directory accumulated: {report}"
    );
    assert!(
        originals[0].is_file(),
        "second directory lost its shared input"
    );
    let report = second.finish().unwrap();
    assert!(
        !second.path().exists(),
        "released second directory accumulated: {report}"
    );
    assert!(
        !originals[0].exists(),
        "last released shared input accumulated: {report}"
    );
    assert_eq!(
        fs::read(&source).unwrap(),
        b"real source outside temporary directories"
    );
}

#[test]
fn directory_cleanup_preserves_nested_live_and_diagnostic_consumers() {
    super::fixture_body::custody_nested_consumers(true);
}

#[test]
fn directory_cleanup_reports_disposal_even_when_shared_input_custody_is_missing() {
    let h = host();
    adopt_directory_cleanup(&h);
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let source = h.parent.join("original-input.rs");
    fs::write(&source, b"required original bytes").unwrap();
    let mut allocation =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "partial-")
            .unwrap();
    let root = allocation.path().to_owned();
    let destination = root.join("copied.rs");
    let original = chrono_worktree::TemporaryHost::register_copy(
        &store,
        "test-allocation",
        &source,
        &destination,
        None,
        "actual fixture input",
    )
    .unwrap();
    fs::copy(&source, &destination).unwrap();
    let record = producer_record_path(&store, &root);
    // A historical allocation may have original bytes but no claim namespace.
    // It cannot establish a new release, yet its owned directory can be gone.
    fs::remove_dir_all(
        store.join("body-input-claims").join(original.file_stem().unwrap()),
    )
    .unwrap();
    let report = allocation.finish().unwrap();
    assert!(!root.exists(), "owned directory was not disposed: {report}");
    assert!(record.exists(), "incomplete retirement lost its continuation");
    assert_eq!(fs::read(&original).unwrap(), b"required original bytes");
    assert!(!report["recovery_error"].is_null(), "{report}");
    let row = report["producer_objects"].as_array().unwrap().iter()
        .find(|row| row["path"].as_str().is_some_and(|p| p.ends_with(root.file_name().unwrap().to_str().unwrap())))
        .unwrap();
    assert_eq!(row["status"], "failed", "{report}");
    let result = &row["original_retirement"];
    assert!(result["error"].as_str().unwrap().contains("claim namespace unknown"));
    assert_eq!(result["physical_absence"], "verified", "{report}");
    assert!(result["artifact_disposals"].as_array().unwrap().iter()
        .any(|effect| effect["status"] == "verified-absent" && effect["bytes_before"].as_u64().unwrap() > 0),
        "actual disposal was lost when shared input release failed: {report}");
}

// Re-enter the real producer rather than assuming that one bounded retention
// round can remove an entire original/metadata relationship. Preserve the final
// physical-absence assertions and reject every error except honest capacity
// deferral while its released predecessor still has work to retire.
fn capacity_reenter_after_release(
    store: &Path,
    outputs: &[String],
    predecessor: &Path,
) -> chrono_worktree::TemporaryHost {
    let record = producer_record_path(store, predecessor);
    for _ in 0..20 {
        match chrono_worktree::TemporaryHost::allocate(
            store,
            "test-allocation",
            outputs,
            "released-reentry-",
        ) {
            Ok(mut next) => {
                if !record.exists() {
                    return next;
                }
                next.finish().unwrap();
            }
            Err(error) => assert!(error.contains("capacity unmet"), "{error}"),
        }
    }
    panic!("released predecessor did not retire through bounded ordinary reentry");
}

#[test]
fn capacity_generated_boundary_and_explicit_finish_preserve_source_and_diagnostic() {
    let h = capacity_host(|_| {});
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let mut allocation =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "boundary-")
            .unwrap();
    let cache = allocation.path().join("cache 空白");
    let source = allocation.path().join("unique-source.rs");
    fs::write(&source, b"unique source").unwrap();
    fs::create_dir(&cache).unwrap();
    fs::write(cache.join("generated"), b"12345678").unwrap();
    assert_eq!(
        allocation.checkpoint().unwrap()["generated_logical_bytes"],
        8
    );
    fs::write(cache.join("generated"), b"123456789").unwrap();
    let error = allocation.checkpoint().unwrap_err();
    assert!(error.contains("capacity unmet"), "{error}");
    let artifact =
        chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", allocation.path())
            .unwrap();
    artifact
        .acquire("diagnostic", "actual failure originals remain required")
        .unwrap();
    let result = allocation.finish().unwrap();
    assert!(
        result["producer_objects"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["status"] == "protected")
    );
    assert!(cache.is_dir());
    assert_eq!(fs::read(&source).unwrap(), b"unique source");
    artifact
        .release(
            "diagnostic",
            "test consumer inspected the original and needs no further cache input",
        )
        .unwrap();
    let mut next = capacity_reenter_after_release(&store, &outputs, allocation.path());
    assert!(!cache.exists());
    assert_eq!(fs::read(&source).unwrap(), b"unique source");
    assert!(next.path().exists());
    next.finish().unwrap();
}

#[test]
fn capacity_copy_refuses_before_original_or_member_bytes_are_written() {
    let h = capacity_host(|_| {});
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "copy-")
            .unwrap();
    let source = allocation.path().join("source");
    let destination = allocation.path().join("copied");
    fs::write(&source, b"123456789").unwrap();
    let error = chrono_worktree::TemporaryHost::register_copy(
        &store,
        "test-allocation",
        &source,
        &destination,
        None,
        "actual copy",
    )
    .unwrap_err();
    assert!(error.contains("before writing"), "{error}");
    assert!(!destination.exists());
    assert!(!store.join("body-inputs").exists());
    fs::write(&source, b"12345678").unwrap();
    let original = chrono_worktree::TemporaryHost::register_copy(
        &store,
        "test-allocation",
        &source,
        &destination,
        None,
        "actual bounded copy",
    )
    .unwrap();
    assert_eq!(fs::read(original).unwrap(), b"12345678");
}

#[test]
fn capacity_pending_copy_reserves_before_releasing_metadata_for_io() {
    use std::sync::mpsc;
    let h = capacity_host(|_| {});
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "parallel-copy-",
    )
    .unwrap();
    let source = allocation.path().join("first-source");
    let destination = allocation.path().join("first-copy");
    fs::write(&source, b"12345").unwrap();
    let (entered, observed) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let directory = store.clone();
    let transfer = std::thread::spawn(move || {
        chrono_worktree::TemporaryHost::register_copy_with_observer(
            &directory,
            "test-allocation",
            &source,
            &destination,
            None,
            "actual first copy",
            |phase| {
                if phase == "copy-original-entered" {
                    entered.send(()).unwrap();
                    released.recv().unwrap();
                }
            },
        )
    });
    observed.recv().unwrap();
    let second = allocation.path().join("second-source");
    fs::write(&second, b"1234").unwrap();
    let result = chrono_worktree::TemporaryHost::register_copy(
        &store,
        "test-allocation",
        &second,
        &allocation.path().join("second-copy"),
        None,
        "actual second copy",
    );
    release.send(()).unwrap();
    let original = transfer.join().unwrap().unwrap();
    assert!(result.unwrap_err().contains("before writing"));
    assert_eq!(fs::read(original).unwrap(), b"12345");
}

#[test]
fn capacity_capture_count_cannot_be_bypassed_by_new_capture_names() {
    let h = capacity_host(|capacity| capacity["max_captures"] = value!(1));
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "native-")
            .unwrap();
    let make_command = || {
        let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
        command.arg("ordinary");
        command
    };
    let first = allocation.capture_output(&mut make_command()).unwrap();
    assert_eq!(first.status.code(), Some(0));
    assert_eq!(first.stdout, b"ordinary-work");
    let error = allocation.capture_output(&mut make_command()).unwrap_err();
    assert!(error.contains("before spawn"), "{error}");
    let captures = allocation
        .path()
        .join(".chrono-harness/state/command-captures");
    assert_eq!(fs::read_dir(captures).unwrap().count(), 1);
}

#[test]
fn capacity_pending_capture_input_keeps_its_reservation_before_io_or_join() {
    let h = capacity_host(|_| {});
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "pending-input-",
    )
    .unwrap();
    // A published capture intent can precede its input write or survive an
    // interrupted writer. The next capture must consume that durable byte
    // reservation even when no bytes or joined result are available yet.
    let record_path = producer_record_path(&store, allocation.path());
    let mut record = json(&fs::read(&record_path).unwrap()).unwrap();
    record["object"]["captures"] = value!({
        ".chrono-harness/state/command-captures/pending": {
            "outcome":"not-established", "membership":"actual native capture producer",
            "reserved_input_bytes":40
        }
    });
    fs::write(&record_path, serde_json::to_vec(&record).unwrap()).unwrap();
    let before = fs::read(&record_path).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    command.arg("noop");
    allocation.bind_command(&mut command).unwrap();
    let result = chrono_worktree::TemporaryHost::capture_bound_input(
        allocation.path(),
        &mut command,
        &[0; 25],
    );
    assert!(
        result.is_err(),
        "40 reserved + 25 new bytes must exceed the 64 byte capture budget"
    );
    assert!(result.unwrap_err().contains("before spawn"));
    assert_eq!(fs::read(record_path).unwrap(), before);
    assert!(
        !allocation
            .path()
            .join(".chrono-harness/state/command-captures")
            .exists()
    );
    let mut boundary = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    boundary.arg("noop");
    allocation.bind_command(&mut boundary).unwrap();
    let result = chrono_worktree::TemporaryHost::capture_bound_input(
        allocation.path(),
        &mut boundary,
        &[0; 24],
    )
    .unwrap();
    assert_eq!(result.status.code(), Some(0));
    let usage = allocation.checkpoint().unwrap();
    assert_eq!(usage["capture_reserved_input_bytes"], 64);
    assert_eq!(usage["capture_accounted_bytes"], 64);
    assert_eq!(usage["capture_logical_bytes"], 24);
}

#[test]
fn capacity_filesystem_refusal_survives_fresh_allocation_ids() {
    let h = capacity_host(|capacity| capacity["min_available_bytes"] = value!(u64::MAX - 1));
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    for prefix in ["first-", "new-id-"] {
        let error =
            chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, prefix)
                .err()
                .unwrap();
        assert!(error.contains("production refused"), "{error}");
    }
    assert!(
        !store.join("objects.json").exists(),
        "refusal must precede allocation birth"
    );
}

#[test]
fn capacity_explicit_maintenance_advances_the_existing_bounded_cursor() {
    let h = capacity_host(|capacity| capacity["max_records_per_round"] = value!(1));
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let mut allocations = Vec::new();
    for prefix in ["active-", "eligible-a-", "eligible-b-"] {
        let allocation =
            chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, prefix)
                .unwrap();
        fs::create_dir(allocation.path().join("cache 空白")).unwrap();
        fs::write(allocation.path().join("cache 空白/generated"), b"123").unwrap();
        allocations.push(allocation);
    }
    let active = allocations.remove(0);
    let eligible = allocations
        .iter()
        .map(|allocation| allocation.path().join("cache 空白"))
        .collect::<Vec<_>>();
    drop(allocations);
    let mut rounds = Vec::new();
    for _ in 0..8 {
        let (code, report, error) = h.auto("maintain", &[]);
        assert_eq!(code, 0, "{report} {error}");
        let producer = report["producer_drains"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["producer"] == "test-allocation")
            .unwrap();
        let original = h.root.join(producer["receipt"]["path"].as_str().unwrap());
        let raw = fs::read(original).unwrap();
        assert_eq!(sha256(&raw), producer["receipt"]["sha256"]);
        let observed = report_view(json(&raw).unwrap());
        let scope = &observed["producer_recovery_scope"];
        assert_eq!(scope["bounded"], true, "{observed}");
        assert!(scope["visited"].as_u64().unwrap() <= 1, "{observed}");
        rounds.push(scope.clone());
        assert!(active.path().join("cache 空白/generated").is_file());
        if eligible.iter().all(|path| !path.exists()) {
            break;
        }
    }
    assert!(
        eligible.iter().all(|path| !path.exists()),
        "explicit maintenance did not advance past the active allocation: {rounds:?}"
    );
}

#[test]
fn capacity_bounded_recovery_advances_past_active_and_corrupt_siblings() {
    let h = capacity_host(|capacity| capacity["max_records_per_round"] = value!(1));
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let mut allocations = Vec::new();
    for prefix in ["active-", "eligible-a-", "eligible-b-"] {
        let allocation =
            chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, prefix)
                .unwrap();
        fs::create_dir(allocation.path().join("cache 空白")).unwrap();
        fs::write(allocation.path().join("cache 空白/generated"), b"123").unwrap();
        allocations.push(allocation);
    }
    let active = allocations.remove(0);
    let eligible = allocations
        .iter()
        .map(|a| a.path().join("cache 空白"))
        .collect::<Vec<_>>();
    drop(allocations);
    fs::create_dir_all(store.join("producer-records/0")).unwrap();
    fs::write(
        store.join("producer-records/0/corrupt.json"),
        b"original damaged registration",
    )
    .unwrap();
    let mut live = Vec::new();
    for _ in 0..6 {
        live.push(
            chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "entry-")
                .unwrap(),
        );
    }
    assert!(eligible.iter().all(|p| !p.exists()));
    assert!(active.path().join("cache 空白/generated").is_file());
    assert_eq!(
        fs::read(store.join("producer-records/0/corrupt.json")).unwrap(),
        b"original damaged registration"
    );
}

#[test]
fn capacity_normal_success_and_failure_recover_only_declared_operation_producers() {
    for mode in ["noop", "rebuild-error"] {
        let h = host_mode(mode);
        let path = h.root.join(".chrono-harness/cleanup.json");
        let mut policy = json(&fs::read(&path).unwrap()).unwrap();
        policy["retained_producers"][0]["operations"] = value!(["use.consumer"]);
        policy["retained_producers"].as_array_mut().unwrap().push(value!({"id":"unrelated",
            "directory":".chrono-harness/state/unrelated fixtures/","temporary":true,"generated_outputs":["cache 空白/"]}));
        fs::write(path, serde_json::to_vec(&policy).unwrap()).unwrap();
        commit(&h.root);
        let (code, report, error) = h.auto("maintain", &[]);
        assert_eq!(code, 0, "{report} {error}");
        let mut caches = Vec::new();
        for producer in ["test-allocation", "unrelated"] {
            let (store, outputs) =
                chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, producer)
                    .unwrap();
            let allocation =
                chrono_worktree::TemporaryHost::allocate(&store, producer, &outputs, "pending-")
                    .unwrap();
            let cache = allocation.path().join("cache 空白");
            fs::create_dir(&cache).unwrap();
            fs::write(cache.join("generated"), b"generated").unwrap();
            caches.push(cache);
            drop(allocation);
        }
        let (code, report, error) = h.auto(
            "use",
            &[
                "--path",
                h.root.to_str().unwrap(),
                "--operation",
                "use.consumer",
            ],
        );
        assert_eq!(code, if mode == "noop" { 0 } else { 2 }, "{report} {error}");
        assert!(
            !caches[0].exists(),
            "registered producer was not recovered: {report}"
        );
        assert!(caches[1].exists(), "unrelated producer was swept");
        assert_eq!(
            report["related_producer_recovery"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            report["post_use_cleanup"]["related_producer_recovery"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        if mode == "rebuild-error" {
            assert_eq!(report["managed_process"]["exit_code"], 23);
            assert_eq!(report["managed_process"]["stderr"], "original-error");
            assert_eq!(report["status"], "failed");
        }
    }
}

fn host() -> Host {
    host_mode("noop")
}

fn host_mode(mode: &str) -> Host {
    let h = super::bounded_cleanup::main_host_mode(mode);
    let path = h.root.join(".chrono-harness/cleanup.json");
    let mut policy = json(&fs::read(&path).unwrap()).unwrap();
    policy["retained_producers"] = value!([{"id":"test-allocation",
        "directory":".chrono-harness/state/custom temporary λ/", "temporary":true,
        "fixture_bodies":true,
        "generated_outputs":["cache 空白/"]}]);
    fs::write(path, serde_json::to_vec(&policy).unwrap()).unwrap();
    fs::write(h.root.join(".chrono-harness/retention.json"), serde_json::to_vec(&value!({"schema":"chrono-output-retention/v2",
        "producers":{"fixture-retirement":[".chrono-harness/state/"],"lifecycle-original":[".chrono-harness/state/worktrees/"]},
        "keep_seconds":0,"metadata_keep_seconds":0,"max_related_outputs":512,"max_summaries":16,
        "max_nodes_per_round":256,"max_bytes_per_round":1048576,"max_millis_per_round":1000})).unwrap()).unwrap();
    commit(&h.root);
    h
}

fn capture_directory(root: &Path) -> PathBuf {
    let captures: Vec<_> = fs::read_dir(root.join(".chrono-harness/state/command-captures"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(captures.len(), 1);
    captures[0].clone()
}

fn command_carrier_count(command: &Command) -> usize {
    command
        .get_envs()
        .find_map(|(key, value)| {
            (key == chrono_harness::process_fds::ENV).then(|| {
                value
                    .map(|value| value.to_str().unwrap().split(',').count())
                    .unwrap_or(0)
            })
        })
        .unwrap_or(0)
}

#[test]
fn busy_allocation_metadata_does_not_delay_independent_cleanup() {
    use std::{
        os::fd::AsRawFd,
        time::{Duration, Instant},
    };
    unsafe extern "C" {
        fn flock(fd: i32, operation: i32) -> i32;
    }
    let h = host();
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let mut paths = vec![];
    let mut allocations = vec![];
    for prefix in ["busy metadata ", "independent cleanup "] {
        let allocation =
            chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, prefix)
                .unwrap();
        fs::create_dir(allocation.path().join("cache 空白")).unwrap();
        fs::write(
            allocation.path().join("cache 空白/generated"),
            b"rebuildable",
        )
        .unwrap();
        paths.push(allocation.path().to_owned());
        // Keep the first allocation live while allocating the second: ordinary
        // allocation recovery must not reclaim this test's pending outputs.
        allocations.push(allocation);
    }
    // Hold the real allocation metadata exclusion through the complete drain.
    let held = fs::File::open(&paths[0]).unwrap();
    assert_eq!(unsafe { flock(held.as_raw_fd(), 2 | 4) }, 0);
    drop(allocations);
    let began = Instant::now();
    let (code, report, error) = h.auto("maintain", &[]);
    let elapsed = began.elapsed();
    assert_eq!(code, 0, "{report} {error}");
    assert!(
        elapsed < Duration::from_secs(10),
        "busy metadata spent the producer admission deadline: {elapsed:?}"
    );
    assert!(paths[0].join("cache 空白").is_dir());
    assert!(
        !paths[1].join("cache 空白").exists(),
        "independent allocation must be reclaimed while the first remains locked: {report}"
    );
    assert!(
        producer_objects(&report)
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["path"].as_str().is_some_and(
                |path| path.ends_with(paths[0].file_name().unwrap().to_str().unwrap())
            ) && row["status"] == "protected"
                && row["reason"] == "live allocation metadata transaction")
    );
    drop(held);
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(!paths[0].join("cache 空白").exists());

    // Cache disposal does not release the metadata transaction or original
    // lifetime. A later maintenance round must defer this exact metadata too.
    let held = fs::File::open(&paths[0]).unwrap();
    assert_eq!(unsafe { flock(held.as_raw_fd(), 2 | 4) }, 0);
    let began = Instant::now();
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(began.elapsed() < Duration::from_secs(10));
    assert!(
        producer_objects(&report)
            .as_array()
            .unwrap()
            .iter()
            .any(|row| {
                row["path"].as_str().is_some_and(|path| {
                    path.ends_with(paths[0].file_name().unwrap().to_str().unwrap())
                }) && row["status"] == "protected"
                    && row["reason"] == "live allocation metadata transaction"
            })
    );
    drop(held);
}

#[test]
fn producer_store_resolution_does_not_acquire_maintenance_admission() {
    let h = host();
    h.policy(|p| p["timeout_seconds"] = value!(1));
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    let state = h.record_directory(&h.root);
    let owner = fs::read(state.join("owner.json")).unwrap();
    let ledger = fs::read(state.join("ledger.json")).unwrap();
    let identity = json(&owner).unwrap();
    let admission = chrono_harness::ownership::Lease::acquire(
        &state.join("metadata.lease"),
        Some(identity["admission_id"].as_str().unwrap()),
        false,
        true,
        None,
    )
    .unwrap()
    .expect("fixture owns the maintenance admission exclusively");
    let expected = (
        h.root.join(".chrono-harness/state/custom temporary λ"),
        vec!["cache 空白/".to_owned()],
    );
    assert_eq!(
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .expect("read-only declaration resolution must not wait for maintenance"),
        expected
    );
    assert_eq!(
        chrono_worktree::RetainedArtifact::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap(),
        expected
    );
    admission.stable().unwrap();
    assert_eq!(fs::read(state.join("owner.json")).unwrap(), owner);
    assert_eq!(fs::read(state.join("ledger.json")).unwrap(), ledger);
}

#[test]
fn producer_store_resolution_checks_committed_inputs_without_creating_cleanup_state() {
    for changed in [POLICY, ".chrono-harness/cleanup.json"] {
        let h = host();
        let state = h.record_directory(&h.root);
        assert!(!state.exists());
        assert!(
            chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "unknown")
                .unwrap_err()
                .contains("not declared")
        );
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
        assert!(
            !state.exists(),
            "resolving a declaration must not initialize maintenance ownership"
        );
        let path = h.root.join(changed);
        let mut bytes = fs::read(&path).unwrap();
        bytes.push(b' ');
        fs::write(path, bytes).unwrap();
        assert!(
            chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
                .is_err(),
            "uncommitted input drift must reject resolution: {changed}"
        );
        assert!(!state.exists());
    }
    let h = host();
    let path = h.root.join(FM);
    let mut filemap = json(&fs::read(&path).unwrap()).unwrap();
    filemap["files"]
        .as_array_mut()
        .unwrap()
        .retain(|file| file["path"] != ".chrono-harness/cleanup.json");
    fs::write(path, serde_json::to_vec(&filemap).unwrap()).unwrap();
    commit(&h.root);
    assert!(
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap_err()
            .contains("not explicitly registered")
    );
    assert!(
        !h.root
            .join(".chrono-harness/state/automatic-cleanup")
            .exists()
    );

    let h = host();
    let path = h.root.join(".chrono-harness/cleanup.json");
    let mut policy = json(&fs::read(&path).unwrap()).unwrap();
    policy["retained_producers"][0]["temporary"] = value!(false);
    policy["retained_producers"][0]["fixture_bodies"] = value!(false);
    fs::write(path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
    assert!(
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap_err()
            .contains("not prospectively adopted")
    );
    chrono_worktree::RetainedArtifact::registered_store(&h.root, POLICY, "test-allocation")
        .unwrap();
    assert!(
        !h.root
            .join(".chrono-harness/state/automatic-cleanup")
            .exists()
    );
}

#[test]
fn absent_temporary_outputs_do_not_rewrite_producer_state_or_publish_disposal() {
    let h = host();
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "absent λ ")
            .unwrap();
    let name = allocation
        .path()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    drop(allocation);
    let before = fs::read(store.join("objects.json")).unwrap();
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    let objects = producer_objects(&report);
    let row = objects
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"].as_str().unwrap().ends_with(&name))
        .unwrap();
    assert_eq!(row["status"], "already-absent");
    assert_eq!(row["absent_outputs"], value!(outputs));
    assert_eq!(
        fs::read(store.join("objects.json")).unwrap(),
        before,
        "no disposal result or producer success may be manufactured for absent output"
    );
    assert!(store.join(&name).is_dir());
}

#[test]
fn one_drain_keeps_every_released_artifact_disposal_and_original() {
    let h = host();
    let (store, outputs) =
        chrono_worktree::RetainedArtifact::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    fs::create_dir_all(&store).unwrap();
    let mut paths = vec![];
    for name in ["first", "middle", "last"] {
        let path = store.join(name);
        fs::create_dir(&path).unwrap();
        fs::write(path.join("source.rs"), format!("original source {name}")).unwrap();
        for (file, bytes) in [
            ("stdout.bin", "actual fixture producer stdout"),
            ("stderr.bin", "actual fixture producer stderr"),
            ("binding.json", "{\"fixture_producer\":\"same-drain\"}"),
        ] {
            fs::write(path.join(file), bytes).unwrap();
        }
        fs::create_dir(path.join("cache 空白")).unwrap();
        fs::write(path.join("cache 空白/generated"), "rebuildable output").unwrap();
        let artifact = chrono_worktree::RetainedArtifact::begin(
            &store,
            "test-allocation",
            &path,
            &outputs,
            "actual-assertion",
            "three released independent artifacts in one drain",
        )
        .unwrap();
        artifact.seal().unwrap();
        artifact
            .release("actual-assertion", "output assertions consumed")
            .unwrap();
        paths.push(path);
    }
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    let registry = producer_state(&store);
    assert_eq!(registry["objects"].as_array().unwrap().len(), 3);
    for path in paths {
        let object = registry["objects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|object| object["path"] == path.file_name().unwrap().to_str().unwrap())
            .expect("every exact released allocation remains registered");
        assert_eq!(object["disposed"], true);
        assert_eq!(object["attempts"].as_array().unwrap().len(), 1);
        assert!(!path.join("cache 空白").exists());
        assert_eq!(
            fs::read_to_string(path.join("source.rs")).unwrap(),
            format!(
                "original source {}",
                path.file_name().unwrap().to_str().unwrap()
            )
        );
    }
}

#[test]
fn unavailable_allocation_preserves_originals_and_does_not_block_eligible_sibling() {
    let h = host();
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let missing = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "unavailable λ ",
    )
    .unwrap();
    let original = missing.path().to_owned();
    let name = original.file_name().unwrap().to_str().unwrap().to_owned();
    fs::write(original.join("source.rs"), "original source").unwrap();
    drop(missing);
    let retained = store.join("unknown-retained-original");
    fs::rename(&original, &retained).unwrap();
    let sibling = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "eligible λ ",
    )
    .unwrap();
    let target = sibling.path().join("cache 空白");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("generated"), "rebuildable").unwrap();
    drop(sibling);
    let read = || producer_state(&store);
    let find = |registry: &Value| {
        registry["objects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["path"] == name)
            .unwrap()
            .clone()
    };
    let before = find(&read());
    let (code, report, _) = h.auto("maintain", &[]);
    assert_eq!(code, 2);
    assert!(
        !target.exists(),
        "eligible sibling must be reclaimed despite the missing row"
    );
    assert_eq!(find(&read()), before);
    assert_eq!(
        fs::read(retained.join("source.rs")).unwrap(),
        b"original source"
    );
    let objects = producer_objects(&report);
    let row = objects
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"].as_str().unwrap().ends_with(&name))
        .unwrap();
    assert_eq!(row["status"], "failed");
    assert!(row["error"].as_str().unwrap().contains("No such file"));
}

#[test]
fn allocation_native_binding_forwards_existing_scope_once() {
    let h = host();
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "binding cost λ ",
    )
    .unwrap();
    let mut baseline = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    chrono_harness::process_fds::forward_command(&mut baseline, &[]).unwrap();
    let expected = command_carrier_count(&baseline);
    assert!(expected > 0);
    let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    command
        .arg("noop")
        .current_dir(allocation.path())
        .env_clear();
    allocation.bind_command(&mut command).unwrap();
    let bound = command_carrier_count(&command);
    assert_eq!(
        bound, expected,
        "allocation lease is already in the held scope"
    );
    assert!(command.output().unwrap().status.success());
    let mut captured = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    captured
        .arg("capture-once")
        .current_dir(allocation.path())
        .env_clear();
    let output = allocation.capture_output(&mut captured).unwrap();
    assert_eq!(output.status.code(), Some(7));
    assert_eq!(command_carrier_count(&captured), expected);
    println!(
        "native capability cost: baseline={expected}, bound={bound}, captured={}",
        command_carrier_count(&captured)
    );
}

#[test]
fn native_capture_preserves_actual_nonzero_exit_and_raw_bytes() {
    let h = host();
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "capture λ ")
            .unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    command
        .arg("capture-once")
        .current_dir(allocation.path())
        .env_clear();
    let output = allocation.capture_output(&mut command).unwrap();
    assert_eq!(output.status.code(), Some(7));
    assert_eq!(output.stdout, b"\xffpartial stdout\0\n");
    assert_eq!(output.stderr, b"\xfepartial stderr\0\n");
    let capture = capture_directory(allocation.path());
    assert_eq!(
        fs::read(capture.join("stdout.bytes")).unwrap(),
        output.stdout
    );
    assert_eq!(
        json(&fs::read(capture.join("result.json")).unwrap()).unwrap()["exit_code"],
        7
    );
}

#[test]
fn native_protocol_capture_preserves_exact_input_bytes_and_eof() {
    let h = host();
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "protocol λ ",
    )
    .unwrap();
    let input = b"\xffrequest\0\n";
    let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    command.arg("capture-input").env_clear();
    allocation.bind_command(&mut command).unwrap();
    let output =
        chrono_worktree::TemporaryHost::capture_bound_input(allocation.path(), &mut command, input)
            .unwrap();
    assert_eq!(output.status.code(), Some(9));
    assert_eq!(output.stdout, input);
    assert_eq!(output.stderr, b"\xfeinput\0\n");
    let capture = capture_directory(allocation.path());
    assert_eq!(fs::read(capture.join("stdin.bytes")).unwrap(), input);
    let intent = json(&fs::read(capture.join("intent.json")).unwrap()).unwrap();
    assert_eq!(intent["stdin"]["sha256"], sha256(input));
    assert_eq!(intent["stdin"]["length"], input.len());
}

#[test]
fn declared_nested_git_root_preserves_committed_and_staged_source_under_outputs() {
    for staged in [false, true] {
        let h = host();
        let policy_path = h.root.join(".chrono-harness/cleanup.json");
        let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
        policy["retained_producers"][0]["source_roots"] = value!(["."]);
        fs::write(policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
        commit(&h.root);
        let (store, outputs) =
            chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
                .unwrap();
        let allocation = chrono_worktree::TemporaryHost::allocate(
            &store,
            "test-allocation",
            &outputs,
            "source λ ",
        )
        .unwrap();
        let root = allocation.path().to_owned();
        git(&root, &["init", "-q", "-b", "dev"]);
        git(&root, &["config", "user.email", "fixture@example.invalid"]);
        git(&root, &["config", "user.name", "Fixture"]);
        fs::write(root.join("source"), "original").unwrap();
        commit(&root);
        fs::create_dir(root.join("cache 空白")).unwrap();
        fs::write(root.join("cache 空白/source.rs"), "irreplaceable source").unwrap();
        git(&root, &["add", "cache 空白/source.rs"]);
        if !staged {
            git(&root, &["commit", "-qm", "unretained source"]);
        }
        drop(allocation);
        let (code, report, error) = h.auto("maintain", &[]);
        assert_ne!(code, 0, "{report} {error}");
        assert!(
            report.to_string().contains("contains tracked paths"),
            "{report}"
        );
        assert_eq!(
            fs::read(root.join("cache 空白/source.rs")).unwrap(),
            b"irreplaceable source"
        );
        assert_eq!(
            git(&root, &["ls-files", "-z", "cache 空白/source.rs"]),
            "cache 空白/source.rs\0"
        );
    }
}

#[test]
fn excluded_source_check_allows_unrelated_publication_and_refuses_late_diagnostic() {
    use std::io::{Read, Write};
    let h = host();
    let path = h.root.join(".chrono-harness/cleanup.json");
    let mut policy = json(&fs::read(&path).unwrap()).unwrap();
    policy["retained_producers"][0]["source_roots"] = value!(["."]);
    fs::write(path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "excluded λ ",
    )
    .unwrap();
    let root = allocation.path().to_owned();
    git(&root, &["init", "-q"]);
    fs::write(root.join("source.rs"), "retained source").unwrap();
    commit(&root);
    fs::create_dir(root.join("cache 空白")).unwrap();
    fs::write(root.join("cache 空白/output"), "generated").unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    super::automatic::native_git(
        &h,
        "temporary-source-check",
        value!({
            "root":root, "synchronize":listener.local_addr().unwrap().to_string(),
            "synchronization_claim":h.parent.join("temporary-source-check-once"),
        }),
    );
    drop(allocation);
    let mut command = h.auto_command("maintain", &[]);
    let maintenance = std::thread::spawn(move || command.output().unwrap());
    let (mut stream, _) = listener.accept().unwrap();
    stream.read_exact(&mut [0; 5]).unwrap();
    let unrelated = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "unrelated λ ",
    )
    .unwrap();
    let reference =
        chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", &root).unwrap();
    assert!(
        reference
            .acquire("late-diagnostic", "must not read excluded output")
            .unwrap_err()
            .contains("reference not acquired")
    );
    stream.write_all(b"r").unwrap();
    let output = maintenance.join().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(!root.join("cache 空白").exists());
    assert_eq!(
        fs::read(root.join("source.rs")).unwrap(),
        b"retained source"
    );
    let registry = producer_state(&store);
    assert!(
        registry["objects"][0]["consumers"]
            .get("late-diagnostic")
            .is_none()
    );
    assert!(unrelated.path().is_dir());
}

#[test]
fn actual_cargo_native_descendant_uses_absolute_allocation_from_varying_working_directories() {
    use std::io::{Read, Write};
    for vary in [false, true] {
        let h = host();
        let policy_path = h.root.join(".chrono-harness/cleanup.json");
        let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
        let outputs = vec!["cache 空白/".to_string(), "cargo 空白/target/".to_string()];
        policy["retained_producers"][0]["generated_outputs"] = value!(outputs);
        fs::write(policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
        commit(&h.root);
        let (store, _) =
            chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
                .unwrap();
        let allocation = chrono_worktree::TemporaryHost::allocate(
            &store,
            "test-allocation",
            &outputs,
            "Cargo λ ",
        )
        .unwrap();
        let root = allocation.path().to_owned();
        let project = root.join("cargo 空白");
        fs::create_dir_all(project.join("src")).unwrap();
        fs::write(
            project.join("Cargo.toml"),
            include_bytes!("support/native_cargo/Cargo.toml"),
        )
        .unwrap();
        fs::write(
            project.join("src/lib.rs"),
            include_bytes!("support/native_cargo/src/lib.rs"),
        )
        .unwrap();
        fs::create_dir(root.join("cache 空白")).unwrap();
        fs::write(root.join("cache 空白/output"), "rebuildable").unwrap();
        let cwd = if vary {
            root.join("different cwd λ")
        } else {
            root.clone()
        };
        fs::create_dir_all(&cwd).unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let mut command = Command::new("cargo");
        command
            .current_dir(&cwd)
            .env_remove("CARGO_TARGET_DIR")
            .env("CHRONO_TEMPORARY_CARGO_ROOT", &root)
            .env(
                "CHRONO_TEMPORARY_HELPER",
                env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"),
            )
            .env(
                "CHRONO_TEMPORARY_SYNC",
                listener.local_addr().unwrap().to_string(),
            )
            .args(["test", "--lib", "--offline", "--manifest-path"])
            .arg(project.join("Cargo.toml"));
        allocation.bind_command(&mut command).unwrap();
        let capture_root = root.clone();
        let cargo = std::thread::spawn(move || {
            chrono_worktree::TemporaryHost::capture_bound_output(&capture_root, &mut command)
                .unwrap()
        });
        let (mut stream, _) = listener.accept().unwrap();
        stream.read_exact(&mut [0; 5]).unwrap();
        let diagnostic =
            chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", &root).unwrap();
        diagnostic
            .acquire(
                "actual-cargo-assertion",
                "reads the completed native Cargo output",
            )
            .unwrap();
        drop(allocation);
        assert_eq!(h.auto("maintain", &[]).0, 0);
        assert!(project.join("target").exists());
        stream.write_all(b"r").unwrap();
        let output = cargo.join().unwrap();
        assert!(output.status.success(), "{output:?}");
        assert!(root.join("cargo-native-joined").exists());
        assert_eq!(h.auto("maintain", &[]).0, 0);
        assert!(project.join("target").exists());
        diagnostic
            .release(
                "actual-cargo-assertion",
                "native child and Cargo joined; assertions consumed originals",
            )
            .unwrap();
        assert_eq!(h.auto("maintain", &[]).0, 0);
        assert!(!project.join("target").exists());
        assert!(!root.join("cache 空白").exists());
        assert!(project.join("src/lib.rs").is_file());
        assert!(project.join("Cargo.toml").is_file());
        assert!(project.join("Cargo.lock").is_file());
    }
}

#[test]
fn capture_interruption_helper() {
    let Some(root) = std::env::var_os("CHRONO_CAPTURE_ROOT") else {
        return;
    };
    let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    command
        .arg("capture-once")
        .env("CHRONO_CAPTURE_INTERRUPT", "1");
    chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
    let _ = chrono_worktree::TemporaryHost::capture_bound_output(Path::new(&root), &mut command);
    panic!("the actual native child must interrupt this capture producer");
}

#[test]
fn interrupted_native_capture_keeps_partial_raw_streams_without_a_join_or_disposal_claim() {
    let h = host();
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "interrupted λ ",
    )
    .unwrap();
    let root = allocation.path().to_owned();
    fs::create_dir(root.join("cache 空白")).unwrap();
    fs::write(root.join("cache 空白/output"), "rebuildable").unwrap();
    let diagnostic =
        chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", &root).unwrap();
    diagnostic
        .acquire(
            "interruption-diagnostic",
            "reads original partial streams; native join remains unknown",
        )
        .unwrap();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "temporary::capture_interruption_helper",
            "--nocapture",
        ])
        .env("CHRONO_CAPTURE_ROOT", &root);
    allocation.bind_command(&mut command).unwrap();
    let output = command.output().unwrap();
    assert!(!output.status.success());
    drop(command);
    drop(allocation);
    let capture = capture_directory(&root);
    assert_eq!(
        fs::read(capture.join("stdout.bytes")).unwrap(),
        b"\xffpartial stdout\0\n"
    );
    assert_eq!(
        fs::read(capture.join("stderr.bytes")).unwrap(),
        b"\xfepartial stderr\0\n"
    );
    assert!(!capture.join("result.json").exists());
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(root.join("cache 空白/output").exists());
    let registry = producer_state(&store);
    assert!(
        !registry["objects"][0]["consumers"]["interruption-diagnostic"]["released"]
            .as_bool()
            .unwrap()
    );
    assert!(h.ledger()["entries"][0]["terminal"].is_null());
}

#[test]
fn native_spawn_failure_keeps_intent_and_empty_original_streams_without_result() {
    let h = host();
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "spawn failure λ ",
    )
    .unwrap();
    let mut command = Command::new(allocation.path().join("missing-native-program"));
    assert!(allocation.capture_output(&mut command).is_err());
    let capture = capture_directory(allocation.path());
    assert!(capture.join("intent.json").is_file());
    assert!(fs::read(capture.join("stdout.bytes")).unwrap().is_empty());
    assert!(fs::read(capture.join("stderr.bytes")).unwrap().is_empty());
    assert!(!capture.join("started.json").exists());
    assert!(!capture.join("result.json").exists());
}

#[test]
fn overlapping_native_allocations_preserve_output_after_kill_until_join_and_diagnostic_release() {
    use std::io::{Read, Write};
    unsafe extern "C" {
        fn kill(pid: i32, signal: i32) -> i32;
    }
    for signal in [15, 9] {
        let h = host();
        let (store, outputs) =
            chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
                .unwrap();
        let allocation = chrono_worktree::TemporaryHost::allocate(
            &store,
            "test-allocation",
            &outputs,
            "overlapping native λ ",
        )
        .unwrap();
        let root = allocation.path().to_owned();
        fs::create_dir(root.join("cache 空白")).unwrap();
        fs::write(root.join("cache 空白/output"), "rebuildable").unwrap();
        fs::write(root.join("source.rs"), "unique source").unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let mut children = vec![];
        let mut streams = vec![];
        for _ in 0..2 {
            let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
            command
                .arg("custody-holder")
                .env_clear()
                .env("CHRONO_TEMPORARY_CARGO_ROOT", &root)
                .env(
                    "CHRONO_TEMPORARY_SYNC",
                    listener.local_addr().unwrap().to_string(),
                );
            allocation.bind_command(&mut command).unwrap();
            children.push(command.spawn().unwrap());
            let (mut stream, _) = listener.accept().unwrap();
            stream.read_exact(&mut [0; 5]).unwrap();
            streams.push(stream);
        }
        let diagnostic =
            chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", &root).unwrap();
        diagnostic
            .acquire(
                "overlap-diagnostic",
                "reads exact output after the actual native children join",
            )
            .unwrap();
        drop(allocation);
        let mut first = children.remove(0);
        assert_eq!(unsafe { kill(first.id() as i32, signal) }, 0);
        assert!(!first.wait().unwrap().success());
        assert_eq!(h.auto("maintain", &[]).0, 0);
        assert!(root.join("cache 空白/output").exists());
        streams[1].write_all(b"r").unwrap();
        assert!(children[0].wait().unwrap().success());
        assert_eq!(h.auto("maintain", &[]).0, 0);
        assert_eq!(
            fs::read(root.join("cache 空白/output")).unwrap(),
            b"rebuildable"
        );
        diagnostic
            .release(
                "overlap-diagnostic",
                "both native children joined and original assertion completed",
            )
            .unwrap();
        assert_eq!(h.auto("maintain", &[]).0, 0);
        assert!(!root.join("cache 空白").exists());
        assert_eq!(fs::read(root.join("source.rs")).unwrap(), b"unique source");
    }
}

#[test]
fn prospective_outputs_cannot_claim_git_identity_or_native_capture_originals() {
    let h = host();
    let (store, _) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    for output in [
        ".git/objects/",
        "nested/.git/",
        ".chrono-harness/state/command-captures/",
        "stdout.bin/",
    ] {
        assert!(
            chrono_worktree::TemporaryHost::allocate(
                &store,
                "test-allocation",
                &[output.into()],
                "invalid λ "
            )
            .is_err()
        );
    }
}

#[test]
fn unavailable_lease_preserves_its_registration_and_allows_an_eligible_allocation() {
    let h = host();
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "lost lease λ ",
    )
    .unwrap();
    let root = allocation.path().to_owned();
    fs::create_dir(root.join("cache 空白")).unwrap();
    fs::write(root.join("cache 空白/original"), "preserve").unwrap();
    drop(allocation);
    let before = producer_state(&store)["objects"][0].clone();
    let lease = before["cache_lease"]["path"].as_str().unwrap();
    fs::rename(store.join(lease), store.join("retained-unavailable-lease")).unwrap();
    let sibling = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "released sibling λ ",
    )
    .unwrap();
    let generated = sibling.path().join("cache 空白");
    fs::create_dir(&generated).unwrap();
    fs::write(generated.join("rebuildable"), "generated").unwrap();
    drop(sibling);
    let (exit, report, _) = h.auto("maintain", &[]);
    assert_eq!(exit, 2, "{report}");
    assert_eq!(producer_state(&store)["objects"][0], before);
    assert_eq!(
        fs::read(root.join("cache 空白/original")).unwrap(),
        b"preserve"
    );
    assert!(!generated.exists());
}

#[test]
fn real_coordinator_adoption_protects_native_use_then_recovers_registered_cargo_output() {
    use std::io::{Read, Write};
    let coordinator = source();
    let (store, outputs) = chrono_worktree::TemporaryHost::registered_store(
        &coordinator,
        POLICY,
        "worktree-test-host",
    )
    .unwrap();
    let allocation = chrono_worktree::TemporaryHost::allocate(
        &store,
        "worktree-test-host",
        &outputs,
        "real coordinator λ ",
    )
    .unwrap();
    let root = allocation.path().to_owned();
    let checkout = root.join("source with spaces");
    let project = checkout.join("independent/producer");
    fs::create_dir_all(project.join("src")).unwrap();
    fs::write(
        project.join("Cargo.toml"),
        include_bytes!("support/cargo_pair/producer/Cargo.toml"),
    )
    .unwrap();
    fs::write(
        project.join("Cargo.lock"),
        include_bytes!("support/cargo_pair/producer/Cargo.lock"),
    )
    .unwrap();
    fs::write(
        project.join("src/lib.rs"),
        include_bytes!("support/cargo_pair/producer/src/lib.rs"),
    )
    .unwrap();
    fs::write(
        checkout.join(".gitignore"),
        "independent/producer/target/\n",
    )
    .unwrap();
    git(&checkout, &["init", "-q"]);
    let original_head = commit(&checkout);
    let original = fs::read(project.join("src/lib.rs")).unwrap();
    let mut cargo = Command::new("cargo");
    cargo
        .current_dir(&checkout)
        .env_remove("CARGO_TARGET_DIR")
        .args([
            "build",
            "--offline",
            "--locked",
            "--manifest-path",
            "independent/producer/Cargo.toml",
        ]);
    let output = allocation.capture_output(&mut cargo).unwrap();
    assert!(output.status.success(), "{output:?}");
    drop(cargo);
    let selected =
        "source with spaces/independent/producer/target/debug/libbounded_cache_producer.rlib";
    assert!(root.join(selected).is_file());
    let unknown = store.join(format!(
        "unknown-neighbor-{}",
        root.file_name().unwrap().to_str().unwrap()
    ));
    fs::create_dir(&unknown).unwrap();
    fs::write(unknown.join("original"), [255, 0, 10]).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    command
        .arg("custody-holder")
        .env_clear()
        .env("CHRONO_TEMPORARY_CARGO_ROOT", &root)
        .env("CHRONO_TEMPORARY_MEMBER", selected)
        .env(
            "CHRONO_TEMPORARY_SYNC",
            listener.local_addr().unwrap().to_string(),
        );
    allocation.bind_command(&mut command).unwrap();
    let mut child = command.spawn().unwrap();
    drop(command);
    let (mut stream, _) = listener.accept().unwrap();
    stream.read_exact(&mut [0; 5]).unwrap();
    drop(allocation);
    let maintain = || {
        let mut command = Command::new(coordinator.join(".chrono-harness/bin/chrono-worktree"));
        command.current_dir(&coordinator).arg("maintain");
        chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
        let output =
            chrono_worktree::TemporaryHost::capture_bound_output(&root, &mut command).unwrap();
        let report = json(&output.stdout).unwrap();
        let failures = report["cleanup_failures"].as_array().unwrap();
        assert_eq!(
            output.status.code(),
            Some(if failures.is_empty() { 0 } else { 2 }),
            "{report}"
        );
        report
    };
    let protected = maintain();
    assert!(project.join("target").exists());
    assert!(
        producer_objects(&protected)
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["producer"] == "worktree-test-host"
                && row["status"] == "protected"
                && row["path"]
                    .as_str()
                    .unwrap()
                    .ends_with(root.file_name().unwrap().to_str().unwrap()))
    );
    stream.write_all(b"r").unwrap();
    assert!(child.wait().unwrap().success());
    let reclaimed = maintain();
    assert!(!project.join("target").exists(), "{reclaimed}");
    assert_eq!(fs::read(project.join("src/lib.rs")).unwrap(), original);
    assert_eq!(git(&checkout, &["rev-parse", "HEAD"]), original_head);
    assert_eq!(fs::read(unknown.join("original")).unwrap(), [255, 0, 10]);
    assert!(project.join("Cargo.toml").is_file() && project.join("Cargo.lock").is_file());
}

#[test]
fn retained_lifecycle_command_preserves_real_allocation_exclusion_after_host_drop() {
    use std::os::fd::AsRawFd;
    unsafe extern "C" {
        fn flock(fd: i32, operation: i32) -> i32;
    }
    let h = super::bounded_cleanup::configure_main(Host::retained("payload"), "allocation-lease");
    let allocation = h._dir.path().to_owned();
    let directory = allocation.parent().unwrap();
    let registry = producer_state(&directory);
    let record = registry["objects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|object| object["path"] == allocation.file_name().unwrap().to_str().unwrap())
        .unwrap();
    let lease_path = directory.join(record["cache_lease"]["path"].as_str().unwrap());
    let path = h.root.join(PROJECTS);
    let mut projects = json(&fs::read(&path).unwrap()).unwrap();
    projects["scripts"][0]["actions"]["execute"]["argv"]
        .as_array_mut()
        .unwrap()
        .push(value!(lease_path));
    fs::write(path, serde_json::to_vec(&projects).unwrap()).unwrap();
    commit(&h.root);
    let mut command = h.auto_command("use", &["--operation", "use.consumer"]);
    let root = h.root.clone();
    drop(h);
    // This capture adapter does not bind capabilities. Retire the command's
    // parent copies immediately after spawn: only the actual child can hold it.
    let child = super::interrupted_cleanup::CapturedChild::spawn(&mut command, &root);
    drop(command);
    let output = child.wait_with_output().unwrap();
    let report = json(&output.stdout).unwrap();
    assert!(output.status.success(), "{}", report["error"]);
    assert_eq!(report["managed_process"]["exit_code"], 0);
    assert_eq!(
        report["managed_process"]["stdout_bytes"],
        value!(b"allocation-live\xff\0".as_slice())
    );
    assert_eq!(
        report["managed_process"]["stderr_bytes"],
        value!(b"original-native-stderr\xfe".as_slice())
    );
    let exclusion = fs::File::open(lease_path).unwrap();
    assert_eq!(unsafe { flock(exclusion.as_raw_fd(), 2 | 4) }, 0);
}

#[test]
fn retained_lifecycle_adapter_captures_actual_failed_result_and_original_streams() {
    let h = super::bounded_cleanup::configure_main(Host::retained("payload"), "capture-once");
    let (code, report, error) = h.auto("use", &["--operation", "use.consumer"]);
    assert_eq!(code, 2, "{error}");
    assert_eq!(report["managed_process"]["exit_code"], 7);
    assert_eq!(
        report["managed_process"]["stdout_bytes"],
        value!(b"\xffpartial stdout\0\n".as_slice())
    );
    assert_eq!(
        report["managed_process"]["stderr_bytes"],
        value!(b"\xfepartial stderr\0\n".as_slice())
    );
    let capture = capture_directory(h._dir.path());
    assert_eq!(
        super::report_view(json(&fs::read(capture.join("stdout.bytes")).unwrap()).unwrap()),
        report
    );
    assert!(fs::read(capture.join("stderr.bytes")).unwrap().is_empty());
    let joined = json(&fs::read(capture.join("result.json")).unwrap()).unwrap();
    assert_eq!(joined["exit_code"], 2);
    assert_eq!(joined["joined"], true);
}

#[test]
fn capacity_fresh_allocation_cannot_escape_protected_excess() {
    let h = capacity_host(|_| {});
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let mut first =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "overflow-")
            .unwrap();
    let cache = first.path().join("cache 空白");
    fs::create_dir(&cache).unwrap();
    fs::write(cache.join("generated"), b"123456789").unwrap();
    let diagnostic =
        chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", first.path()).unwrap();
    diagnostic
        .acquire("reader", "reads actual overflow original")
        .unwrap();
    assert!(first.checkpoint().unwrap_err().contains("capacity unmet"));
    first.finish().unwrap();
    let result =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "fresh-");
    assert!(
        result.is_err(),
        "a fresh allocation evaded unresolved excess"
    );
    assert!(cache.exists());
    diagnostic.release("reader", "original consumed").unwrap();
    let mut next = capacity_reenter_after_release(&store, &outputs, first.path());
    assert!(!cache.exists());
    assert!(next.path().exists());
    next.finish().unwrap();
}

#[test]
fn capacity_recovery_progresses_beyond_one_snapshot() {
    let h = capacity_host(|c| {
        c["max_record_snapshot"] = value!(4);
    });
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let mut live = vec![];
    for _ in 0..9 {
        live.push(
            chrono_worktree::TemporaryHost::allocate(
                &store,
                "test-allocation",
                &outputs,
                "snapshot-",
            )
            .expect("total history must not be a snapshot admission ceiling"),
        );
    }
    let mut caches = vec![];
    for allocation in &live {
        let cache = allocation.path().join("cache 空白");
        fs::create_dir(&cache).unwrap();
        fs::write(cache.join("generated"), b"123").unwrap();
        caches.push(cache);
    }
    drop(live);
    for _ in 0..20 {
        let mut next = chrono_worktree::TemporaryHost::allocate(
            &store,
            "test-allocation",
            &outputs,
            "reentry-",
        )
        .unwrap();
        next.finish().unwrap();
        if caches.iter().all(|p| !p.exists()) {
            break;
        }
    }
    assert!(caches.iter().all(|p| !p.exists()));
}

#[test]
fn capacity_shared_original_retires_only_after_both_actual_consumers_release() {
    let h = capacity_host(|_| {});
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let source = h.root.join("actual-copy-input");
    fs::write(&source, b"shared").unwrap();
    let mut first =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "first-")
            .unwrap();
    let mut second =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "second-")
            .unwrap();
    let mut originals = vec![];
    for host in [&first, &second] {
        let destination = host.path().join("copied.bin");
        originals.push(
            chrono_worktree::TemporaryHost::register_copy(
                &store,
                "test-allocation",
                &source,
                &destination,
                None,
                "actual shared original consumer",
            )
            .unwrap(),
        );
        fs::copy(&source, &destination).unwrap();
    }
    assert_eq!(originals[0], originals[1]);
    let unique = first.path().join("unique-source.rs");
    fs::write(&unique, b"unique source").unwrap();
    let report = first.finish().unwrap();
    assert!(
        originals[0].is_file(),
        "first release destroyed second consumer input: {report}"
    );
    assert_eq!(fs::read(&unique).unwrap(), b"unique source");
    let report = second.finish().unwrap();
    assert!(
        !originals[0].exists(),
        "released shared original was not retired: {report}"
    );
    assert_eq!(fs::read(&source).unwrap(), b"shared");
    assert_eq!(fs::read(&unique).unwrap(), b"unique source");
}

#[test]
fn capacity_native_failure_originals_and_metadata_retire_after_diagnostic_release() {
    let h = capacity_host(|c| c["max_capture_bytes"] = value!(4096));
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let mut host = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "failed-original-",
    )
    .unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    command.arg("capture-once");
    let output = host.capture_output(&mut command).unwrap();
    drop(command);
    assert_eq!(output.status.code(), Some(7));
    let capture = capture_directory(host.path());
    let result = json(&fs::read(capture.join("result.json")).unwrap()).unwrap();
    assert_eq!(result["exit_code"], 7);
    let diagnostic =
        chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", host.path()).unwrap();
    diagnostic
        .acquire("reader", "reads failure original")
        .unwrap();
    let record = producer_record_path(&store, host.path());
    let metadata = store.join(format!(
        "producer-metadata/{}",
        sha256(host.path().file_name().unwrap().as_encoded_bytes())
    ));
    host.finish().unwrap();
    assert_eq!(
        fs::read(capture.join("stdout.bytes")).unwrap(),
        output.stdout
    );
    assert_eq!(
        fs::read(capture.join("stderr.bytes")).unwrap(),
        output.stderr
    );
    diagnostic
        .release(
            "reader",
            "actual consumer inspected original failed result and streams",
        )
        .unwrap();
    let mut next = capacity_reenter_after_release(&store, &outputs, host.path());
    next.finish().unwrap();
    assert!(!capture.exists(), "released native originals accumulated");
    assert!(!metadata.exists(), "released producer receipts accumulated");
    assert!(
        !record.exists(),
        "released allocation registration accumulated"
    );
    assert_eq!(
        result["exit_code"], 7,
        "retirement changed original outcome"
    );
    assert!(
        !host.path().exists(),
        "source-free native container skeleton accumulated"
    );
}

#[test]
fn capacity_metadata_refuses_before_publication_and_keeps_original_references() {
    let h = capacity_host(|c| c["max_metadata_bytes_per_round"] = value!(8192));
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let mut host =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "metadata-")
            .unwrap();
    let artifact =
        chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", host.path()).unwrap();
    artifact.acquire("actual-reader", "reads original").unwrap();
    let before = fs::read(producer_record_path(&store, host.path())).unwrap();
    let metadata = store.join(format!(
        "producer-metadata/{}",
        sha256(host.path().file_name().unwrap().as_encoded_bytes())
    ));
    let before_count = fs::read_dir(&metadata).unwrap().count();
    let error = artifact
        .acquire("oversize-reader", &"x".repeat(8192))
        .unwrap_err();
    assert!(error.contains("metadata byte capacity"), "{error}");
    assert_eq!(
        fs::read(producer_record_path(&store, host.path())).unwrap(),
        before
    );
    assert_eq!(fs::read_dir(&metadata).unwrap().count(), before_count);
    artifact
        .release("actual-reader", "original consumed")
        .unwrap();
    host.finish().unwrap();
    assert!(
        !metadata.exists(),
        "ordinary finish did not retire released metadata"
    );
}

#[test]
fn capacity_interrupted_copy_claim_retires_without_inventing_original_bytes() {
    let h = capacity_host(|_| {});
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let mut host = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "pending-original-",
    )
    .unwrap();
    let source = h.root.join("actual-copy-source");
    fs::write(&source, b"pending").unwrap();
    let digest = sha256(b"pending");
    let original = store.join(format!("body-inputs/{digest}.bin"));
    let claims = store.join(format!("body-input-claims/{digest}"));
    let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        chrono_worktree::TemporaryHost::register_copy_with_observer(
            &store,
            "test-allocation",
            &source,
            &host.path().join("copy"),
            None,
            "actual pending copy",
            |stage| {
                if stage == "copy-original-entered" {
                    panic!("controlled original publication interruption");
                }
            },
        )
    }));
    assert!(interrupted.is_err());
    assert!(!original.exists());
    assert_eq!(fs::read_dir(&claims).unwrap().count(), 1);
    let record = producer_record_path(&store, host.path());
    let state = json(&fs::read(&record).unwrap()).unwrap();
    assert_eq!(state["object"]["copy_intents"]["copy"], 7);
    assert!(state["object"]["original_release"].is_null());
    host.finish().unwrap();
    let mut next = capacity_reenter_after_release(&store, &outputs, host.path());
    next.finish().unwrap();
    assert!(!original.exists(), "recovery invented original bytes");
    assert!(!claims.exists(), "released pending-copy claims accumulated");
    assert!(
        !record.exists(),
        "released pending registration accumulated"
    );
    assert_eq!(fs::read(source).unwrap(), b"pending");
}

#[test]
fn capacity_interrupted_native_originals_require_exact_custodian_release() {
    let h = capacity_host(|c| c["max_capture_bytes"] = value!(4096));
    assert_eq!(h.auto("use", &["--operation", "use.consumer"]).0, 0);
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "interrupted-original-",
    )
    .unwrap();
    let root = allocation.path().to_owned();
    let artifact =
        chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", &root).unwrap();
    fs::write(root.join("unique-source.rs"), b"preserve unique source").unwrap();
    artifact
        .acquire("diagnostic", "needs original partial streams")
        .unwrap();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "temporary::capture_interruption_helper",
            "--nocapture",
        ])
        .env("CHRONO_CAPTURE_ROOT", &root);
    allocation.bind_command(&mut command).unwrap();
    let output = command.output().unwrap();
    assert!(!output.status.success());
    drop(command);
    drop(allocation);
    let capture = capture_directory(&root);
    let partial = fs::read(capture.join("stdout.bytes")).unwrap();
    assert_eq!(partial, b"\xffpartial stdout\0\n");
    assert!(!capture.join("result.json").exists());
    let mut next = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "ordinary-reentry-",
    )
    .unwrap();
    next.finish().unwrap();
    assert_eq!(fs::read(capture.join("stdout.bytes")).unwrap(), partial);
    let state = producer_state(&store);
    let object = state["objects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["path"] == root.file_name().unwrap().to_str().unwrap())
        .unwrap();
    let plan = value!({"schema":"chrono-fixture-body-custody/v1","head":git(&h.root,&["rev-parse","HEAD"]),
        "current_consumers_released":true,"reason":"actual interruption custodian joined the helper and inspected original streams; outcome remains unknown",
        "objects":[{"producer":"test-allocation","allocation":object["path"],"identity":object["identity"],"intent":object["intent"],"members":[],"release_originals":true}]});
    let path = ".chrono-harness/state/original-custody.json";
    fs::write(h.root.join(path), serde_json::to_vec(&plan).unwrap()).unwrap();
    let migrate = || {
        h.auto(
            "migrate",
            &["--path", h.root.to_str().unwrap(), "--adopt-fixtures", path],
        )
    };
    let (code, report, error) = migrate();
    assert_eq!(code, 2, "{report} {error}");
    assert!(capture.is_dir(), "unreleased diagnostic was bypassed");
    artifact
        .release("diagnostic", "original partial streams inspected")
        .unwrap();
    let (code, report, error) = migrate();
    assert_eq!(code, 0, "{report} {error}");
    assert!(
        !capture.join("result.json").exists(),
        "migration invented a join"
    );
    let mut next = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "released-reentry-",
    )
    .unwrap();
    next.finish().unwrap();
    assert!(
        !capture.exists(),
        "released interrupted originals accumulated"
    );
    assert!(root.is_dir(), "retirement removed unclassified source root");
}

#[test]
fn capacity_layout_context_and_pending_cursor_migrate_without_resetting_outcomes() {
    let h = capacity_host(|_| {});
    assert_eq!(h.auto("use", &["--operation", "use.consumer"]).0, 0);
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "old-layout-",
    )
    .unwrap();
    let root = allocation.path().to_owned();
    let mut command = Command::new(root.join("missing-program"));
    assert!(allocation.capture_output(&mut command).is_err());
    drop(command);
    let original = fs::read(producer_record_path(&store, &root)).unwrap();
    let mut record = json(&original).unwrap();
    let mut owner = json(&fs::read(store.join("objects.json")).unwrap()).unwrap();
    owner.as_object_mut().unwrap().remove("record_layout");
    let old_owner = serde_json::to_vec(&owner).unwrap();
    record["owner_sha256"] = value!(sha256(&old_owner));
    let old_record = serde_json::to_vec(&record).unwrap();
    let flat = store.join(format!(
        "producer-records/{}.json",
        sha256(root.file_name().unwrap().as_encoded_bytes())
    ));
    fs::write(&flat, &old_record).unwrap();
    fs::remove_file(producer_record_path(&store, &root)).unwrap();
    fs::write(store.join("objects.json"), &old_owner).unwrap();
    let candidate = git(&h.root, &["rev-parse", "HEAD"]);
    let error = chrono_worktree::RetainedArtifact::partition_with_observer(
        &store,
        "test-allocation",
        candidate.trim(),
        |_| Ok(()),
    )
    .unwrap_err();
    assert!(error.contains("active/unknown"), "{error}");
    assert_eq!(fs::read(store.join("objects.json")).unwrap(), old_owner);
    drop(allocation);
    let error = chrono_worktree::RetainedArtifact::partition_with_observer(
        &store,
        "test-allocation",
        candidate.trim(),
        |stage| {
            assert_eq!(stage, "layout-owner-activated");
            Err("controlled interruption after layout activation".into())
        },
    )
    .unwrap_err();
    assert!(error.contains("controlled interruption"));
    assert!(flat.exists());
    assert!(store.join("producer-layout-pending.json").exists());
    let context_path = store.join("producer-context.json");
    let mut context = json(&fs::read(&context_path).unwrap()).unwrap();
    context["policy_sha256"] = value!("old-preserved-binding");
    let old_context = serde_json::to_vec(&context).unwrap();
    fs::write(&context_path, &old_context).unwrap();
    let cursor_path = store.join("recovery-state/cursor.json");
    fs::create_dir_all(cursor_path.parent().unwrap()).unwrap();
    let old_cursor =
        serde_json::to_vec(&value!([root.file_name().unwrap().to_str().unwrap()])).unwrap();
    fs::write(&cursor_path, &old_cursor).unwrap();
    let (code, report, error) = h.auto(
        "migrate",
        &[
            "--path",
            h.root.to_str().unwrap(),
            "--partition-producer",
            "test-allocation",
        ],
    );
    assert_eq!(code, 0, "{report} {error}");
    assert!(!flat.exists());
    assert!(!store.join("producer-layout-pending.json").exists());
    assert_eq!(
        fs::read(store.join(format!(
            "producer-context-source-{}.json",
            sha256(&old_context)
        )))
        .unwrap(),
        old_context
    );
    assert_eq!(
        fs::read(store.join(format!(
            "producer-recovery-source-{}.json",
            sha256(&old_cursor)
        )))
        .unwrap(),
        old_cursor
    );
    let migrated = json(&fs::read(producer_record_path(&store, &root)).unwrap()).unwrap();
    assert_eq!(
        migrated["object"], record["object"],
        "migration changed original status or reservations"
    );
    let mut next = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "after-migration-",
    )
    .unwrap();
    next.finish().unwrap();
    let capture = capture_directory(&root);
    assert!(!capture.join("result.json").exists());
    assert!(
        capture.join("intent.json").exists(),
        "unknown original was discarded"
    );
}

#[test]
fn capacity_context_migration_retains_one_gate_per_allocation() {
    use std::os::unix::process::CommandExt;

    let h = capacity_host(|_| {});
    assert_eq!(h.auto("use", &["--operation", "use.consumer"]).0, 0);
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let mut originals = vec![];
    for _ in 0..24 {
        let allocation = chrono_worktree::TemporaryHost::allocate(
            &store,
            "test-allocation",
            &outputs,
            "descriptor-bound-",
        )
        .unwrap();
        let record = producer_record_path(&store, allocation.path());
        originals.push(record);
        drop(allocation);
    }
    let originals: Vec<_> = originals
        .into_iter()
        .map(|path| {
            let bytes = fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect();
    let context_path = store.join("producer-context.json");
    let mut context = json(&fs::read(&context_path).unwrap()).unwrap();
    context["policy_sha256"] = value!("previous-context-binding");
    let previous = serde_json::to_vec(&context).unwrap();
    fs::write(&context_path, &previous).unwrap();
    let mut command = h.auto_command(
        "migrate",
        &[
            "--path",
            h.root.to_str().unwrap(),
            "--partition-producer",
            "test-allocation",
        ],
    );
    // Lower only this native child; do not change the test runner or host limit.
    unsafe {
        command.pre_exec(|| {
            let mut limit = std::mem::zeroed::<libc::rlimit>();
            if libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if limit.rlim_cur < 288 {
                return Err(std::io::Error::other(
                    "fixture requires 288 available descriptors, including reserved native transport slots",
                ));
            }
            limit.rlim_cur = 288;
            if libc::setrlimit(libc::RLIMIT_NOFILE, &limit) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let output = h._dir.capture_bound(&mut command);
    let (code, report, error) = h.received(output);
    assert_eq!(code, 0, "{report} {error}");
    assert_eq!(report["producer_context_migration"]["objects"], 24);
    assert_eq!(
        fs::read(store.join(format!(
            "producer-context-source-{}.json",
            sha256(&previous)
        )))
        .unwrap(),
        previous
    );
    for (path, bytes) in originals {
        assert_eq!(fs::read(path).unwrap(), bytes, "original object changed");
    }
}

#[test]
fn capacity_partial_retirement_keeps_its_record_until_originals_are_absent() {
    let h = capacity_host(|c| c["max_capture_bytes"] = value!(4096));
    let path = h.root.join(".chrono-harness/retention.json");
    let mut policy = json(&fs::read(&path).unwrap()).unwrap();
    policy["max_nodes_per_round"] = value!(8);
    fs::write(path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let mut host = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "partial-retirement-",
    )
    .unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    command.arg("capture-once");
    let original = host.capture_output(&mut command).unwrap();
    drop(command);
    assert_eq!(original.status.code(), Some(7));
    let record = producer_record_path(&store, host.path());
    let original_record = json(&fs::read(&record).unwrap()).unwrap();
    let original_identity = original_record["object"]["identity"].clone();
    host.finish().unwrap();
    assert!(
        host.path().exists(),
        "fixture did not exercise partial disposal"
    );
    assert!(
        record.exists(),
        "partial retirement lost its continuation record"
    );
    let captures = host.path().join(".chrono-harness/state/command-captures");
    let capture_count = || {
        fs::read_dir(&captures)
            .map(|entries| entries.count())
            .unwrap_or(0)
    };
    let before = capture_count();
    let mut refused = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    refused.arg("capture-once");
    let error = chrono_worktree::TemporaryHost::capture_bound_input(
        host.path(),
        &mut refused,
        b"released input must not be written",
    )
    .unwrap_err();
    assert!(error.contains("original lifetime released"), "{error}");
    assert_eq!(capture_count(), before, "retired capture created output");
    let mut observed_retirement = false;
    for _ in 0..16 {
        if !record.exists() {
            break;
        }
        let report = host.finish().unwrap();
        for row in report["producer_objects"].as_array().unwrap() {
            if row["status"] == "retiring" {
                observed_retirement = true;
                assert_eq!(row["producer"], "test-allocation");
                assert_eq!(
                    row["path"],
                    host.path().strip_prefix(&h.root).unwrap().to_str().unwrap()
                );
                assert_eq!(row["identity"], original_identity);
            }
        }
        assert!(
            !report["original_retention"].as_array().unwrap().is_empty(),
            "ordinary reentry did not consume its declared retirement paths"
        );
        assert!(
            !host.path().exists() || record.exists(),
            "continuation disappeared before original container"
        );
    }
    assert!(
        observed_retirement,
        "bounded reentry lost the allocation's retirement identity"
    );
    assert!(
        !host.path().exists(),
        "bounded ordinary recovery did not advance"
    );
    assert!(
        !record.exists(),
        "completed retirement registration accumulated"
    );
    assert_eq!(original.status.code(), Some(7));
}
