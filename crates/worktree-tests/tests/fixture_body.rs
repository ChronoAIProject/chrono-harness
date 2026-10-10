use super::*;

fn host() -> Host {
    let h = super::bounded_cleanup::main_host_mode("temporary-producer");
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
fn partial_body_missing_original_and_committed_source_do_not_veto_eligible_siblings() {
    let h = host();
    let (store, first) = allocation(&h);
    let root = first.path().to_owned();
    let original = copy(&store, &root, "partial-body");
    fs::write(root.join("partial-body"), b"partial failed copy").unwrap();
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
    assert_eq!(
        fs::read(root.join("partial-body")).unwrap(),
        b"partial failed copy"
    );
    assert_eq!(git(&root, &["rev-parse", "HEAD"]), head);
    assert!(root.join("tracked-body").exists());
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
    let (store, allocation) = allocation(&h);
    let root = allocation.path().to_owned();
    fs::write(root.join("legacy-copy"), b"real copied input").unwrap();
    drop(allocation);
    let registry = json(&fs::read(store.join("objects.json")).unwrap()).unwrap();
    let object = &registry["objects"][0];
    let old_intent = fs::read(store.join(object["intent"]["path"].as_str().unwrap())).unwrap();
    let plan = value!({"schema":"chrono-fixture-body-custody/v1", "head":git(&h.root,&["rev-parse","HEAD"]),
        "current_consumers_released":true,"reason":"This test has joined its actual legacy copy consumer",
        "objects":[{"producer":"test-allocation","allocation":object["path"],"identity":object["identity"],"intent":object["intent"],
        "members":[{"path":"legacy-copy","sha256":sha256(b"real copied input"),"length":17,"git_root":".","recipe":"recipe.rs"}]}]});
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
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(!root.join("legacy-copy").exists());
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
