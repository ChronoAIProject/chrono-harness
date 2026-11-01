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
    let before_registry = json(&fs::read(store.join("objects.json")).unwrap()).unwrap();
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
    let nested_registry = json(&fs::read(nested_store.join("objects.json")).unwrap()).unwrap();
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
    let registry = json(&fs::read(store.join("objects.json")).unwrap()).unwrap();
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
    let after = json(&fs::read(store.join("objects.json")).unwrap()).unwrap();
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
