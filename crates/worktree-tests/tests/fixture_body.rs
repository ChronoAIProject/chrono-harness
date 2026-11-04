use super::*;

fn host() -> Host {
    let h = super::bounded_cleanup::main_host_mode("noop");
    let path = h.root.join(".chrono-harness/cleanup.json");
    let mut policy = json(&fs::read(&path).unwrap()).unwrap();
    policy["retained_producers"] = value!([{"id":"test-allocation",
        "directory":".chrono-harness/state/body fixtures/", "temporary":true,
        "fixture_bodies":true,"generated_outputs":[]}]);
    fs::write(path, serde_json::to_vec(&policy).unwrap()).unwrap();
    // Real committed recipe input for the current-custodian transition.
    fs::write(
        h.root.join("recipe.rs"),
        "fn producer() { /* copied fixture input */ }\n",
    )
    .unwrap();
    commit(&h.root);
    h
}

fn allocation(h: &Host) -> (PathBuf, chrono_worktree::TemporaryHost) {
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let host =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "copied λ ")
            .unwrap();
    (store, host)
}

fn copy(store: &Path, allocation: &Path, name: &str) -> PathBuf {
    let source = allocation.join("real-source");
    fs::write(&source, [0, 255, 7, 8]).unwrap();
    let destination = allocation.join(name);
    let original = chrono_worktree::TemporaryHost::register_copy(
        store,
        "test-allocation",
        &source,
        &destination,
        Some(allocation),
        "actual Rust fixture copy",
    )
    .unwrap();
    fs::copy(&source, &destination).unwrap();
    original
}

#[test]
fn preparing_a_copy_original_does_not_block_an_independent_allocation() {
    use std::{sync::mpsc, thread, time::Duration};

    let h = host();
    let (store, allocation) = allocation(&h);
    let source = h.root.join("actual-input.bin");
    fs::write(&source, [0, 255, 7, 8]).unwrap();
    let destination = allocation.path().join("copied.bin");
    let copy_store = store.clone();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    thread::scope(|scope| {
        let copy = scope.spawn(move || {
            chrono_worktree::TemporaryHost::register_copy_with_observer(
                &copy_store,
                "test-allocation",
                &source,
                &destination,
                None,
                "actual copy with a controlled original transfer",
                |stage| {
                    if stage == "copy-original-entered" {
                        entered_tx.send(()).unwrap();
                        release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
                    }
                },
            )
        });
        entered_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        let allocate_store = store.clone();
        let referenced_allocation = allocation.path().to_owned();
        let (done_tx, done_rx) = mpsc::channel();
        let independent = scope.spawn(move || {
            let result = (|| -> Result<PathBuf, String> {
                let reference = chrono_worktree::RetainedArtifact::resume(
                    &allocate_store,
                    "test-allocation",
                    &referenced_allocation,
                )?;
                reference.acquire("overlapping-copy", "actual concurrent diagnostic consumer")?;
                chrono_worktree::TemporaryHost::allocate(
                    &allocate_store,
                    "test-allocation",
                    &[],
                    "independent admission ",
                )
                .map(|host| host.path().to_owned())
            })();
            done_tx.send(result).unwrap();
        });
        let before_release = done_rx.recv_timeout(Duration::from_secs(2));
        release_tx.send(()).unwrap();
        let original = copy.join().unwrap().unwrap();
        independent.join().unwrap();
        let admitted = before_release
            .expect("copy IO must not hold the shared registry lease")
            .unwrap();
        assert!(admitted.is_dir());
        assert_eq!(fs::read(original).unwrap(), [0, 255, 7, 8]);
        let registry = producer_state(&store);
        assert_eq!(registry["objects"].as_array().unwrap().len(), 2);
        let object = registry["objects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|object| {
                object["path"] == allocation.path().file_name().unwrap().to_str().unwrap()
            })
            .unwrap();
        assert_eq!(object["bodies"]["copied.bin"]["length"], 4);
        assert_eq!(object["consumers"]["overlapping-copy"]["released"], false);
        chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", allocation.path())
            .unwrap()
            .release(
                "overlapping-copy",
                "concurrent diagnostic assertion consumed",
            )
            .unwrap();
    });
}

#[test]
fn concurrent_original_publications_keep_both_body_registrations() {
    use std::{sync::mpsc, thread, time::Duration};

    let h = host();
    let (store, allocation) = allocation(&h);
    let source = h.root.join("concurrent-input.bin");
    fs::write(&source, [0, 255, 7, 8]).unwrap();
    let (entered_tx, entered_rx) = mpsc::channel();
    thread::scope(|scope| {
        let mut releases = vec![];
        let mut tasks = vec![];
        for member in ["first.bin", "second.bin"] {
            let copy_store = store.clone();
            let source = source.clone();
            let destination = allocation.path().join(member);
            let entered_tx = entered_tx.clone();
            let (release_tx, release_rx) = mpsc::channel();
            releases.push(release_tx);
            tasks.push(scope.spawn(move || {
                chrono_worktree::TemporaryHost::register_copy_with_observer(
                    &copy_store,
                    "test-allocation",
                    &source,
                    &destination,
                    None,
                    "two actual copies with concurrent original publication",
                    |stage| {
                        if stage == "copy-original-publish" {
                            entered_tx.send(()).unwrap();
                            release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
                        }
                    },
                )
            }));
        }
        let both_prepared = entered_rx
            .recv_timeout(Duration::from_secs(10))
            .and_then(|_| entered_rx.recv_timeout(Duration::from_secs(10)));
        for release in releases {
            let _ = release.send(());
        }
        let outcomes: Vec<_> = tasks.into_iter().map(|task| task.join()).collect();
        both_prepared.expect("both original transfers must proceed outside the registry lease");
        let originals: Vec<_> = outcomes
            .into_iter()
            .map(|outcome| outcome.unwrap().unwrap())
            .collect();
        assert_eq!(originals[0], originals[1]);
        assert_eq!(fs::read(&originals[0]).unwrap(), [0, 255, 7, 8]);
    });
    let registry = producer_state(&store);
    let bodies = registry["objects"][0]["bodies"].as_object().unwrap();
    assert_eq!(bodies.len(), 2);
    assert_eq!(
        bodies["first.bin"]["original"],
        bodies["second.bin"]["original"]
    );
}

#[test]
fn updating_one_allocation_keeps_other_copies_and_diagnostic_consumers() {
    let h = host();
    let (store, first) = allocation(&h);
    let (_, middle) = allocation(&h);
    let (_, last) = allocation(&h);
    for allocation in [&first, &middle, &last] {
        copy(&store, allocation.path(), "first.bin");
    }
    for allocation in [&first, &last] {
        chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", allocation.path())
            .unwrap()
            .acquire(
                "actual-diagnostic",
                "unrelated allocation still has a consumer",
            )
            .unwrap();
    }
    let membership = fs::read(store.join("objects.json")).unwrap();
    let first_record = producer_record_path(&store, first.path());
    let last_record = producer_record_path(&store, last.path());
    let unrelated_bytes = [
        fs::read(&first_record).unwrap(),
        fs::read(&last_record).unwrap(),
    ];
    let unrelated_identities = [
        chrono_harness::ownership::identity(&first_record),
        chrono_harness::ownership::identity(&last_record),
    ];
    let before = producer_state(&store);
    let original = copy(&store, middle.path(), "second.bin");
    let after_membership = fs::read(store.join("objects.json")).unwrap();
    assert_eq!(
        after_membership, membership,
        "copying into one existing allocation must not republish shared membership"
    );
    let after = producer_state(&store);
    assert_eq!(after["objects"].as_array().unwrap().len(), 3);
    assert!(
        json(&membership).unwrap().get("objects").is_none(),
        "the immutable namespace identity must contain no dynamic membership"
    );
    let find = |state: &Value, allocation: &Path| {
        state["objects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|object| object["path"] == allocation.file_name().unwrap().to_str().unwrap())
            .unwrap()
            .clone()
    };
    assert_eq!(find(&after, first.path()), find(&before, first.path()));
    assert_eq!(find(&after, last.path()), find(&before, last.path()));
    assert_eq!(
        find(&after, middle.path())["bodies"]
            .as_object()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(fs::read(&first_record).unwrap(), unrelated_bytes[0]);
    assert_eq!(fs::read(&last_record).unwrap(), unrelated_bytes[1]);
    assert_eq!(
        chrono_harness::ownership::identity(&first_record),
        unrelated_identities[0]
    );
    assert_eq!(
        chrono_harness::ownership::identity(&last_record),
        unrelated_identities[1]
    );
    assert_eq!(fs::read(original).unwrap(), [0, 255, 7, 8]);
    for allocation in [&first, &middle, &last] {
        assert_eq!(
            fs::read(allocation.path().join("first.bin")).unwrap(),
            [0, 255, 7, 8]
        );
    }
    for allocation in [&first, &last] {
        chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", allocation.path())
            .unwrap()
            .release("actual-diagnostic", "copy-preservation assertion consumed")
            .unwrap();
    }
}

#[test]
fn metadata_publication_for_one_allocation_does_not_lock_another() {
    use std::{sync::mpsc, thread, time::Duration};
    let h = host();
    let (store, existing) = allocation(&h);
    let pending = store.join("publication in progress");
    fs::create_dir(&pending).unwrap();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    thread::scope(|scope| {
        let pending_store = store.clone();
        let pending = scope.spawn(move || {
            chrono_worktree::RetainedArtifact::begin_with_birth_observer(
                &pending_store,
                "test-allocation",
                &pending,
                &[],
                "actual pending publication",
                "controlled real metadata publication",
                |stage| {
                    if stage == "object-record-admitted" {
                        entered_tx.send(()).unwrap();
                        release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
                    }
                },
            )
        });
        entered_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        let independent_store = store.clone();
        let existing = existing.path().to_owned();
        let (done_tx, done_rx) = mpsc::channel();
        let independent = scope.spawn(move || {
            let outcome = (|| -> Result<(), String> {
                let reference = chrono_worktree::RetainedArtifact::resume(
                    &independent_store,
                    "test-allocation",
                    &existing,
                )?;
                reference.acquire("independent diagnostic", "actual concurrent reader")?;
                reference.release("independent diagnostic", "reader joined")?;
                let allocation = chrono_worktree::TemporaryHost::allocate(
                    &independent_store,
                    "test-allocation",
                    &[],
                    "independent metadata ",
                )?;
                assert!(allocation.path().is_dir());
                Ok(())
            })();
            done_tx.send(outcome).unwrap();
        });
        let completed_before_release = done_rx.recv_timeout(Duration::from_secs(2));
        release_tx.send(()).unwrap();
        pending.join().unwrap().unwrap();
        independent.join().unwrap();
        completed_before_release
            .expect("unrelated metadata must progress while publication is held")
            .unwrap();
    });
    assert!(!store.join("registry.lease").exists());
}

#[test]
fn explicit_partition_preserves_original_data_and_current_consumers() {
    let h = host();
    assert_eq!(h.auto("use", &["--operation", "use.consumer"]).0, 0);
    let (store, allocation) = allocation(&h);
    let root = allocation.path().to_owned();
    let original = copy(&store, &root, "retained body");
    let reference =
        chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", &root).unwrap();
    reference
        .acquire(
            "actual diagnostic",
            "must survive explicit format conversion",
        )
        .unwrap();
    let before = producer_state(&store);
    let lease = chrono_harness::ownership::Lease::acquire(
        &store.join("registry.lease"),
        None,
        true,
        true,
        None,
    )
    .unwrap()
    .unwrap();
    let previous = value!({"schema":"chrono-retained-producer/v1",
        "producer":"test-allocation","directory_id":before["directory_id"],
        "lease_id":lease.id(),"objects":before["objects"]});
    drop(lease);
    let previous_bytes = serde_json::to_vec(&previous).unwrap();
    fs::rename(
        store.join("producer-records"),
        store.join("retained fixture input"),
    )
    .unwrap();
    fs::write(store.join("objects.json"), &previous_bytes).unwrap();
    assert!(
        chrono_worktree::TemporaryHost::allocate(
            &store,
            "test-allocation",
            &[],
            "refused before migration "
        )
        .is_err()
    );
    assert_eq!(
        fs::read(store.join("objects.json")).unwrap(),
        previous_bytes
    );
    let candidate = git(&h.root, &["rev-parse", "HEAD"]);
    let interrupted = chrono_worktree::RetainedArtifact::partition_with_observer(
        &store,
        "test-allocation",
        candidate.trim(),
        |stage| {
            assert_eq!(stage, "partition-records-published");
            Err("controlled interruption before namespace activation".into())
        },
    )
    .unwrap_err();
    assert!(interrupted.contains("controlled interruption"));
    assert_eq!(
        fs::read(store.join("objects.json")).unwrap(),
        previous_bytes
    );
    let staged_path = producer_record_path(&store, &root);
    let staged_bytes = fs::read(&staged_path).unwrap();
    let mut changed = json(&staged_bytes).unwrap();
    changed["object"]["retention_reason"] = value!("unknown independent retained state");
    let changed_bytes = serde_json::to_vec(&changed).unwrap();
    fs::write(&staged_path, &changed_bytes).unwrap();
    let migrate = || {
        h.auto(
            "migrate",
            &[
                "--path",
                h.root.to_str().unwrap(),
                "--partition-producer",
                "test-allocation",
            ],
        )
    };
    let (code, report, error) = migrate();
    assert_eq!(code, 2, "{report} {error}");
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("unknown registration"),
        "{report}"
    );
    assert_eq!(fs::read(&staged_path).unwrap(), changed_bytes);
    assert_eq!(
        fs::read(store.join("objects.json")).unwrap(),
        previous_bytes
    );
    // Repair this test's deliberately corrupted staging bytes; the real owner
    // must then resume the same bound interruption without releasing readers.
    fs::write(&staged_path, &staged_bytes).unwrap();
    let (code, report, error) = migrate();
    assert_eq!(code, 0, "{report} {error}");
    assert_eq!(report["producer_partition"]["status"], "partitioned");
    assert_eq!(report["producer_partition"]["disposal"], "not-performed");
    let receipt = &report["producer_partition"]["receipt"];
    let bytes = fs::read(store.join(receipt["path"].as_str().unwrap())).unwrap();
    assert_eq!(sha256(&bytes), receipt["sha256"]);
    let migration = json(&bytes).unwrap();
    let retained = fs::read(store.join(migration["source"]["path"].as_str().unwrap())).unwrap();
    assert_eq!(retained, previous_bytes);
    assert_eq!(sha256(&retained), migration["source"]["sha256"]);
    let after = producer_state(&store);
    assert_eq!(after["objects"], before["objects"]);
    assert_eq!(fs::read(&original).unwrap(), [0, 255, 7, 8]);
    assert_eq!(
        fs::read(root.join("retained body")).unwrap(),
        [0, 255, 7, 8]
    );
    let owner_bytes = fs::read(store.join("objects.json")).unwrap();
    let (code, report, error) = migrate();
    assert_eq!(code, 0, "{report} {error}");
    assert_eq!(report["producer_partition"]["status"], "already-current");
    assert_eq!(fs::read(store.join("objects.json")).unwrap(), owner_bytes);
    chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", &root)
        .unwrap()
        .release(
            "actual diagnostic",
            "current reader consumed migrated original",
        )
        .unwrap();
}

#[test]
fn copy_publication_refuses_identity_or_original_drift_without_overwriting_it() {
    for drift in ["allocation", "intent", "cache-lease", "owner", "original"] {
        let h = host();
        let (store, allocation) = allocation(&h);
        let source = h.root.join("drift-input.bin");
        let bytes = [0, 255, 7, 8];
        fs::write(&source, bytes).unwrap();
        let destination = allocation.path().join("copied.bin");
        let registry_path = if drift == "owner" {
            store.join("objects.json")
        } else {
            producer_record_path(&store, allocation.path())
        };
        let before = fs::read(&registry_path).unwrap();
        let original = store.join(format!("body-inputs/{}.bin", sha256(&bytes)));
        let mut changed_registry = None;
        let result = chrono_worktree::TemporaryHost::register_copy_with_observer(
            &store,
            "test-allocation",
            &source,
            &destination,
            None,
            "actual transfer with a changed publication prerequisite",
            |stage| {
                if stage != "copy-original-prepared" {
                    return;
                }
                if drift == "original" {
                    let replacement = h.root.join("replacement-original.bin");
                    fs::write(&replacement, [9, 9, 9, 9]).unwrap();
                    fs::rename(replacement, &original).unwrap();
                } else {
                    let mut registry = json(&before).unwrap();
                    match drift {
                        "allocation" => registry["object"]["identity"] = value!("changed"),
                        "intent" => registry["object"]["intent"]["sha256"] = value!("changed"),
                        "cache-lease" => {
                            registry["object"]["cache_lease"]["id"] = value!("changed")
                        }
                        "owner" => registry["records_id"] = value!("changed"),
                        _ => unreachable!(),
                    }
                    let changed = serde_json::to_vec(&registry).unwrap();
                    fs::write(&registry_path, &changed).unwrap();
                    changed_registry = Some(changed);
                }
            },
        );
        assert!(result.is_err(), "{drift} must prevent body publication");
        assert!(!destination.exists());
        assert_eq!(fs::read(&source).unwrap(), bytes);
        assert_eq!(
            fs::read(&registry_path).unwrap(),
            changed_registry.unwrap_or_else(|| before.clone())
        );
        if drift == "original" {
            assert_eq!(fs::read(&original).unwrap(), [9, 9, 9, 9]);
            fs::write(&original, bytes).unwrap();
        } else {
            assert_eq!(fs::read(&original).unwrap(), bytes);
        }
        fs::write(&registry_path, before).unwrap();
        let admitted = chrono_worktree::TemporaryHost::register_copy(
            &store,
            "test-allocation",
            &source,
            &destination,
            None,
            "restored actual publication prerequisites",
        )
        .unwrap();
        assert_eq!(fs::read(admitted).unwrap(), bytes);
    }
}

#[test]
fn same_length_body_declaration_change_refuses_cached_registration() {
    let h = host();
    let (store, allocation) = allocation(&h);
    let root = allocation.path();
    let original = copy(&store, root, "body.bin");
    let source = root.join("real-source");
    let destination = root.join("body.bin");
    let path = producer_record_path(&store, root);
    let before = fs::read(&path).unwrap();
    let registry = json(&before).unwrap();
    let digest = registry["object"]["bodies"]["body.bin"]["sha256"]
        .as_str()
        .unwrap();
    let replacement = if digest == "a".repeat(64) { "b" } else { "a" }.repeat(64);
    let marker = format!(r#""sha256":"{digest}""#);
    let text = std::str::from_utf8(&before).unwrap();
    assert_eq!(text.matches(&marker).count(), 1);
    let damaged = text.replacen(&marker, &format!(r#""sha256":"{replacement}""#), 1);
    assert_eq!(damaged.len(), before.len());
    fs::write(&path, &damaged).unwrap();
    let error = chrono_worktree::TemporaryHost::register_copy(
        &store,
        "test-allocation",
        &source,
        &destination,
        Some(root),
        "actual Rust fixture copy",
    )
    .unwrap_err();
    assert!(
        error.contains("copied body registration changed"),
        "{error}"
    );
    assert_eq!(fs::read(&path).unwrap(), damaged.as_bytes());
    assert_eq!(fs::read(&destination).unwrap(), [0, 255, 7, 8]);
    assert_eq!(fs::read(&original).unwrap(), [0, 255, 7, 8]);
    fs::write(&path, &before).unwrap();
    let restored = chrono_worktree::TemporaryHost::register_copy(
        &store,
        "test-allocation",
        &source,
        &destination,
        Some(root),
        "actual Rust fixture copy",
    )
    .unwrap();
    assert_eq!(restored, original);
    assert_eq!(fs::read(&path).unwrap(), before);
}

#[test]
fn duplicate_producer_map_members_and_nested_fields_refuse_publication() {
    let h = host();
    let (store, allocation) = allocation(&h);
    let root = allocation.path();
    let original = copy(&store, root, "body.bin");
    let path = producer_record_path(&store, root);
    let before = fs::read(&path).unwrap();
    let registry = json(&before).unwrap();
    let text = registry.to_string();
    let body = registry["object"]["bodies"]["body.bin"].to_string();
    let reference = r#"{"released":false,"reason":"actual diagnostic","receipt":null}"#;
    let bodies = format!(r#""bodies":{{"body.bin":{body}}}"#);
    let duplicate_receipt = body.replacen(r#""sha256":"#, r#""sha256":"duplicate","sha256":"#, 1);
    let changes = [
        (r#""generated":{}"#.to_owned(), r#""generated":{"cache/":"one","cache/":"two"}"#.to_owned()),
        (r#""evidence":{}"#.to_owned(), r#""evidence":{"probe":null,"probe":{}}"#.to_owned()),
        (r#""evidence":{}"#.to_owned(), r#""evidence":{"probe":[{"field":1,"field":2}]}"#.to_owned()),
        (r#""consumers":{}"#.to_owned(), format!(r#""consumers":{{"reader":{reference},"reader":{reference}}}"#)),
        (r#""consumers":{}"#.to_owned(), r#""consumers":{"reader":{"released":false,"reason":"one","reason":"two","receipt":null}}"#.to_owned()),
        (bodies.clone(), format!(r#""bodies":{{"body.bin":{body},"b\u006fdy.bin":{body}}}"#)),
        (bodies, format!(r#""bodies":{{"body.bin":{duplicate_receipt}}}"#)),
    ];
    for (old, replacement) in changes {
        assert_eq!(text.matches(&old).count(), 1, "fixture marker {old}");
        let damaged = text.replacen(&old, &replacement, 1);
        fs::write(&path, &damaged).unwrap();
        let error = chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", root)
            .err()
            .expect("the addressed duplicate declaration must refuse admission");
        assert!(error.contains("duplicate"), "{error}");
        assert_eq!(fs::read(&path).unwrap(), damaged.as_bytes());
        assert_eq!(fs::read(root.join("body.bin")).unwrap(), [0, 255, 7, 8]);
        assert_eq!(fs::read(&original).unwrap(), [0, 255, 7, 8]);
    }
    let independent = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &[],
        "independent of damaged registration ",
    )
    .unwrap();
    assert!(independent.path().is_dir());
    fs::write(&path, &before).unwrap();
    let next = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &[],
        "restored valid registry ",
    )
    .unwrap();
    assert!(next.path().is_dir());
    assert_eq!(fs::read(&original).unwrap(), [0, 255, 7, 8]);
}

fn joined_command_body(outcome: &str) {
    use std::io::{Read, Write};

    let h = host();
    let projects_path = h.root.join(PROJECTS);
    let mut projects = json(&fs::read(&projects_path).unwrap()).unwrap();
    projects["scripts"][0]["actions"]["execute"]["argv"] = value!(["custody-holder"]);
    fs::write(projects_path, serde_json::to_vec(&projects).unwrap()).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = listener.local_addr().unwrap();
    let consumer_root = h.root.join(".chrono-harness/state/consumer-root");
    let config_path = h.root.join(CONFIG);
    let mut config = json(&fs::read(&config_path).unwrap()).unwrap();
    config["environment"]["values"]["TMPDIR"] = value!(consumer_root);
    config["environment"]["values"]["CHRONO_TEMPORARY_CARGO_ROOT"] = value!(consumer_root);
    config["environment"]["values"]["CHRONO_TEMPORARY_SYNC"] =
        value!(listener.local_addr().unwrap().to_string());
    fs::write(config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    commit(&h.root);
    fs::create_dir_all(consumer_root.join("cache 空白")).unwrap();
    fs::write(consumer_root.join("cache 空白/output"), "rebuildable").unwrap();
    let source = fs::read(h.root.join("recipe.rs")).unwrap();
    let mut command = h.auto_command("use", &["--operation", "use.consumer"]);
    let (output, store, root, original, diagnostic, partial) = std::thread::scope(|scope| {
        let wrapper = scope.spawn(|| {
            let output = command.output().unwrap();
            std::net::TcpStream::connect(endpoint)
                .unwrap()
                .write_all(b"ended")
                .unwrap();
            output
        });
        let (mut stream, _) = listener.accept().unwrap();
        let mut readiness = [0; 5];
        stream.read_exact(&mut readiness).unwrap();
        if readiness != *b"ready" {
            panic!(
                "managed child ended before readiness: {:?}",
                wrapper.join().unwrap()
            );
        }
        // This allocation did not exist during admission's drain.
        let (store, allocation) = allocation(&h);
        let root = allocation.path().to_owned();
        let original = copy(&store, &root, "joined-body.bin");
        let partial = [255, 0, 10];
        fs::write(root.join("partial.stderr"), partial).unwrap();
        let diagnostic =
            chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", &root).unwrap();
        if outcome == "diagnostic" {
            diagnostic
                .acquire("assertion", "actual post-command copied-body consumer")
                .unwrap();
        }
        drop(allocation);
        if outcome == "failure" {
            fs::write(
                consumer_root.join("cache 空白/output"),
                "original failed input",
            )
            .unwrap();
        }
        stream.write_all(b"r").unwrap();
        (
            wrapper.join().unwrap(),
            store,
            root,
            original,
            diagnostic,
            partial,
        )
    });
    let (code, report, error) = h.received(output);
    assert_eq!(fs::read(&original).unwrap(), [0, 255, 7, 8]);
    assert_eq!(fs::read(root.join("real-source")).unwrap(), [0, 255, 7, 8]);
    assert_eq!(fs::read(root.join("partial.stderr")).unwrap(), partial);
    assert_eq!(fs::read(h.root.join("recipe.rs")).unwrap(), source);
    assert!(h.ledger()["entries"][0]["terminal"].is_null());
    assert!(report["producer_drains"].is_array(), "{report}");
    assert!(
        report["managed_use_receipt"]["path"].is_string(),
        "{report}"
    );
    if outcome == "failure" {
        assert_ne!(code, 0, "{report} {error}");
        assert_eq!(report["managed_process"]["exit_code"], 101);
        assert_eq!(report["managed_process"]["stdout_bytes"], value!([]));
        let stderr: Vec<u8> =
            serde_json::from_value(report["managed_process"]["stderr_bytes"].clone()).unwrap();
        assert!(String::from_utf8_lossy(&stderr).contains("assertion `left == right` failed"));
        assert_eq!(report["managed_command_failed"], true);
        assert!(report["post_use_cleanup"].is_null(), "{report}");
        assert!(root.join("joined-body.bin").exists());
    } else {
        assert_eq!(code, 0, "{report} {error}");
        assert_eq!(report["managed_process"]["exit_code"], 0);
        assert_eq!(
            root.join("joined-body.bin").exists(),
            outcome == "diagnostic"
        );
        assert!(
            report["post_use_cleanup"]["producer_drains"].is_array(),
            "{report}"
        );
        if outcome == "diagnostic" {
            assert_eq!(
                fs::read(root.join("joined-body.bin")).unwrap(),
                [0, 255, 7, 8]
            );
            diagnostic
                .release("assertion", "actual read completed")
                .unwrap();
            assert_eq!(h.auto("maintain", &[]).0, 0);
            assert!(!root.join("joined-body.bin").exists());
        }
    }
    assert!(store.join("objects.json").is_file());
}

#[test]
fn successful_join_reclaims_copied_bodies_created_during_managed_use() {
    joined_command_body("success");
}

#[test]
fn successful_join_preserves_unreleased_copied_body_consumer() {
    joined_command_body("diagnostic");
}

#[test]
fn failed_join_preserves_copied_bodies_and_original_process_failure() {
    joined_command_body("failure");
}

#[test]
fn body_recovers_without_finish_after_overlapping_holders_and_diagnostic_release() {
    let h = host();
    let (store, allocation) = allocation(&h);
    let root = allocation.path().to_owned();
    let original = copy(&store, &root, "body.bin");
    fs::write(root.join("partial.stderr"), [255, 0, 10]).unwrap();
    let neighbor = store.join("unknown neighbor");
    fs::create_dir(&neighbor).unwrap();
    fs::write(neighbor.join("body.bin"), "unique source").unwrap();
    assert!(
        chrono_worktree::TemporaryHost::register_copy(
            &store,
            "test-allocation",
            &root.join("real-source"),
            &neighbor.join("copy.bin"),
            Some(&neighbor),
            "unknown producer"
        )
        .is_err()
    );
    let diagnostic =
        chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", &root).unwrap();
    diagnostic
        .acquire("assertion", "actual current consumer reads copied input")
        .unwrap();
    let capability = allocation.capability().unwrap();
    drop(allocation);
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(root.join("body.bin").exists());
    drop(capability);
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(root.join("body.bin").exists());
    assert_eq!(fs::read(root.join("body.bin")).unwrap(), [0, 255, 7, 8]);
    diagnostic
        .release(
            "assertion",
            "assertion read completed; original remains in evidence custody",
        )
        .unwrap();
    let before_registry = producer_state(&store);
    let intent_path = store.join(
        before_registry["objects"][0]["intent"]["path"]
            .as_str()
            .unwrap(),
    );
    let old_intent = fs::read(&intent_path).unwrap();
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(!root.join("body.bin").exists());
    assert_eq!(fs::read(&original).unwrap(), [0, 255, 7, 8]);
    assert_eq!(fs::read(root.join("partial.stderr")).unwrap(), [255, 0, 10]);
    assert_eq!(
        fs::read(neighbor.join("body.bin")).unwrap(),
        b"unique source"
    );
    assert_eq!(fs::read(&intent_path).unwrap(), old_intent);
    assert!(h.ledger()["entries"][0]["terminal"].is_null());
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert_eq!(fs::read(&original).unwrap(), [0, 255, 7, 8]);
}

#[test]
fn moved_checkout_carries_body_membership_and_retains_its_original_commit() {
    let h = host();
    let (store, allocation) = allocation(&h);
    let root = allocation.path().to_owned();
    let checkout = root.join("checkout");
    fs::create_dir(&checkout).unwrap();
    git(&checkout, &["init", "-q", "-b", "dev"]);
    fs::write(checkout.join("source.rs"), "unique committed source\n").unwrap();
    fs::write(checkout.join(".gitignore"), "copy.bin\n").unwrap();
    commit(&checkout);
    let head = git(&checkout, &["rev-parse", "HEAD"]);
    let source = root.join("input");
    fs::write(&source, b"original copied input").unwrap();
    let original = chrono_worktree::TemporaryHost::register_copy(
        &store,
        "test-allocation",
        &source,
        &checkout.join("copy.bin"),
        Some(&checkout),
        "actual offline fixture copy",
    )
    .unwrap();
    fs::copy(&source, checkout.join("copy.bin")).unwrap();
    let hook_source = Path::new(env!("CARGO_BIN_EXE_chrono-worktree-test-hook"));
    let hook = checkout.join(".git/hooks/post-checkout");
    let hook_original = chrono_worktree::TemporaryHost::register_copy(
        &store,
        "test-allocation",
        hook_source,
        &hook,
        Some(&checkout),
        "actual copied native Git hook",
    )
    .unwrap();
    fs::copy(hook_source, &hook).unwrap();
    fs::write(
        hook.with_extension("json"),
        serde_json::to_vec(&value!({
            "writes":[{"path":"native-hook-read","contents":"actual Git consumer"}]
        }))
        .unwrap(),
    )
    .unwrap();
    assert!(
        chrono_worktree::TemporaryHost::register_copy(
            &store,
            "test-allocation",
            hook_source,
            &checkout.join(".git/HEAD"),
            Some(&checkout),
            "Git identity must stay protected",
        )
        .is_err()
    );
    assert!(
        chrono_worktree::TemporaryHost::register_copy(
            &store,
            "test-allocation",
            hook_source,
            &checkout.join(".git/HEAD"),
            None,
            "omitted Git root grants no metadata disposal authority",
        )
        .is_err()
    );
    git(&checkout, &["checkout", "--detach", "HEAD"]);
    assert_eq!(
        fs::read(checkout.join("native-hook-read")).unwrap(),
        b"actual Git consumer"
    );
    let retained = root.join("offline checkout");
    chrono_worktree::TemporaryHost::relocate_copies(
        &store,
        "test-allocation",
        &checkout,
        &retained,
    )
    .unwrap();
    fs::rename(&checkout, &retained).unwrap();
    drop(allocation);
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(!retained.join("copy.bin").exists());
    assert!(!retained.join(".git/hooks/post-checkout").exists());
    assert!(retained.join(".git/hooks/post-checkout.json").exists());
    assert_eq!(
        fs::read(hook_original).unwrap(),
        fs::read(hook_source).unwrap()
    );
    assert_eq!(fs::read(original).unwrap(), b"original copied input");
    assert_eq!(
        fs::read(retained.join("source.rs")).unwrap(),
        b"unique committed source\n"
    );
    assert_eq!(git(&retained, &["rev-parse", "HEAD"]), head);
}

#[test]
fn partial_body_missing_original_and_committed_source_do_not_veto_eligible_siblings() {
    let h = host();
    let (store, first) = allocation(&h);
    let root = first.path().to_owned();
    let original = copy(&store, &root, "partial-body");
    fs::write(root.join("partial-body"), [0, 255, 7, 9]).unwrap();
    let good = copy(&store, &root, "good-body");
    assert_eq!(good, original);
    git(&root, &["init", "-q", "-b", "dev"]);
    fs::write(root.join(".gitignore"), "good-body\npartial-body\n").unwrap();
    fs::write(root.join("tracked-body"), [0, 255, 7, 8]).unwrap();
    commit(&root);
    chrono_worktree::TemporaryHost::register_copy(
        &store,
        "test-allocation",
        &root.join("real-source"),
        &root.join("tracked-body"),
        Some(&root),
        "actual copy later committed as source",
    )
    .unwrap();
    let head = git(&root, &["rev-parse", "HEAD"]);
    drop(first);
    let (code, report, _) = h.auto("maintain", &[]);
    assert_eq!(code, 2, "{report}");
    assert!(report.to_string().contains("partial; preserve"));
    assert!(report.to_string().contains("committed or staged source"));
    assert!(!root.join("good-body").exists());
    assert_eq!(fs::read(root.join("partial-body")).unwrap(), [0, 255, 7, 9]);
    assert_eq!(git(&root, &["rev-parse", "HEAD"]), head);
    assert!(root.join("tracked-body").exists());
    let child = report["producer_drains"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["producer"] == "test-allocation")
        .unwrap();
    let bytes = fs::read(h.root.join(child["receipt"]["path"].as_str().unwrap())).unwrap();
    assert_eq!(sha256(&bytes), child["receipt"]["sha256"]);
    let original_report = json(&bytes).unwrap();
    // One live source/index observation per content-matching member suffices at
    // the shared unlink boundary. Partial copies never reach source admission.
    let source_checks = original_report["processes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|process| process["root"] == value!(root) && process["argv"][1] == "ls-files")
        .count();
    assert_eq!(
        source_checks, 2,
        "repeated source/index reads: {source_checks}"
    );
    let (_, second) = allocation(&h);
    let other = second.path().to_owned();
    copy(&store, &other, "body");
    drop(second);
    fs::rename(&original, store.join("retained unknown original")).unwrap();
    let (code, report, _) = h.auto("maintain", &[]);
    assert_eq!(code, 2, "{report}");
    assert!(other.join("body").exists());
    assert!(report.to_string().contains("No such file"));
}

#[test]
fn current_custody_migrates_a_bound_legacy_copy_without_rewriting_unknown_outcome() {
    let h = host();
    let (code, report, error) = h.auto("use", &["--operation", "use.consumer"]);
    assert_eq!(code, 0, "{report} {error}");
    let (store, allocation) = allocation(&h);
    let root = allocation.path().to_owned();
    fs::write(root.join("legacy-copy"), b"real copied input").unwrap();
    let nested_store = root.join("nested producer");
    fs::create_dir(&nested_store).unwrap();
    let nested =
        chrono_worktree::TemporaryHost::allocate(&nested_store, "nested-fixture", &[], "child ")
            .unwrap();
    let nested_root = nested.path().to_owned();
    fs::write(nested_root.join("copy"), b"real copied input").unwrap();
    drop(nested);
    let nested_registry = producer_state(&nested_store);
    let nested_object = &nested_registry["objects"][0];
    let nested_binding = &nested_object["cache_lease"];
    let nested_path = format!(
        "nested producer/{}",
        nested_binding["path"].as_str().unwrap()
    );
    let nested_member = format!(
        "nested producer/{}/copy",
        nested_object["path"].as_str().unwrap()
    );
    let nested_id = nested_binding["id"].as_str().unwrap();
    drop(allocation);
    let registry = producer_state(&store);
    let object = &registry["objects"][0];
    let old_intent = fs::read(store.join(object["intent"]["path"].as_str().unwrap())).unwrap();
    let plan = value!({"schema":"chrono-fixture-body-custody/v1", "head":git(&h.root,&["rev-parse","HEAD"]),
        "current_consumers_released":true,"reason":"This test has joined its actual legacy copy consumer",
        "objects":[{"producer":"test-allocation","allocation":object["path"],"identity":object["identity"],"intent":object["intent"],
        "members":[{"path":"legacy-copy","sha256":sha256(b"real copied input"),"length":17,"git_root":".","recipe":"recipe.rs"},
        {"path":nested_member,"sha256":sha256(b"real copied input"),"length":17,"git_root":null,"recipe":"recipe.rs",
        "leases":[{"path":nested_path,"id":nested_id}],
        "nested_consumers":[{"directory":"nested producer","producer":"nested-fixture",
        "registry_identity":nested_registry["directory_id"],"allocation":nested_object["path"],
        "identity":nested_object["identity"],"intent":nested_object["intent"]}]}]}]});
    let plan_path = ".chrono-harness/state/body-custody.json";
    fs::write(h.root.join(plan_path), serde_json::to_vec(&plan).unwrap()).unwrap();
    let (code, report, error) = h.auto(
        "migrate",
        &[
            "--path",
            h.root.to_str().unwrap(),
            "--adopt-fixtures",
            plan_path,
        ],
    );
    assert_eq!(code, 0, "{report} {error}");
    assert!(
        root.join("legacy-copy").exists(),
        "migration must perform no disposal"
    );
    use std::os::fd::AsRawFd;
    unsafe extern "C" {
        fn flock(fd: i32, operation: i32) -> i32;
    }
    let holder = fs::File::open(root.join(&nested_path)).unwrap();
    assert_eq!(unsafe { flock(holder.as_raw_fd(), 1) }, 0);
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(
        nested_root.join("copy").exists(),
        "nested consumer outlives parent allocation holder"
    );
    drop(holder);
    let nested_diagnostic =
        chrono_worktree::RetainedArtifact::resume(&nested_store, "nested-fixture", &nested_root)
            .unwrap();
    nested_diagnostic
        .acquire(
            "actual late diagnostic",
            "reads nested copied body after migration",
        )
        .unwrap();
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(
        nested_root.join("copy").exists(),
        "persistent nested diagnostic outlives its lease holder"
    );
    nested_diagnostic
        .release("actual late diagnostic", "actual nested read completed")
        .unwrap();
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(!root.join("legacy-copy").exists());
    assert!(!nested_root.join("copy").exists());
    assert_eq!(
        fs::read(store.join(object["intent"]["path"].as_str().unwrap())).unwrap(),
        old_intent
    );
    let after = producer_state(&store);
    let body = &after["objects"][0]["bodies"]["legacy-copy"];
    assert_eq!(
        fs::read(store.join(body["original"].as_str().unwrap())).unwrap(),
        b"real copied input"
    );
    assert_eq!(
        json(&old_intent).unwrap()["producer_outcome"],
        "not-established"
    );
    assert_eq!(h.auto("maintain", &[]).0, 0);
}
