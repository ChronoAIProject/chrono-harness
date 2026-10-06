use super::*;
const CLEANUP: &str = ".chrono-harness/cleanup.json";
const LEDGER: &str = ".chrono-harness/state/automatic-cleanup/ledger.json";

fn edit(root: &Path, path: &str, change: impl FnOnce(&mut Value)) {
    let file = root.join(path);
    let mut value = json(&fs::read(&file).unwrap()).unwrap();
    change(&mut value);
    fs::write(file, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}

fn fixture() -> (Host, PathBuf) {
    let h = Host::new("payload");
    h.kernel_cleanup();
    let target = h.parent.join("migrating");
    let (code, report, error) = h.invoke("integration", "migrating", &target);
    assert_eq!(code, 0, "{report} {error}");
    (h, target)
}

fn advance(h: &Host) {
    edit(&h.root, CLEANUP, |v| {
        v["allow_evidence_disposal"] = value!(false)
    });
    commit(&h.root);
}

fn adopt(h: &Host, target: &Path) {
    let head = git(&h.root, &["rev-parse", "HEAD"]);
    git(target, &["merge", "--ff-only", head.trim()]);
}

fn migrate(h: &Host, target: &Path) -> Value {
    let (code, report, error) = h.auto("migrate", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{report} {error}");
    assert_eq!(report["policy_migration"]["disposal"], "not-performed");
    assert_eq!(
        report["policy_migration"]["terminal_handoff"],
        "not-claimed"
    );
    report
}

fn cache(target: &Path) {
    fs::create_dir_all(target.join("output λ")).unwrap();
    fs::write(target.join("output λ/cache"), "rebuildable").unwrap();
}

#[test]
fn policy_migration_adopts_new_registered_outputs_without_disposal_or_rewriting_birth() {
    let (h, target) = fixture();
    let original = h.ledger()["entries"][0].clone();
    let birth_path = h.root.join(
        original["enrollment"]["sealed_receipt"]["path"]
            .as_str()
            .unwrap(),
    );
    let birth_bytes = fs::read(&birth_path).unwrap();
    edit(&h.root, CONFIG, |v| {
        v["artifacts"].as_array_mut().unwrap().push(
        value!({"path":"fresh output/","owner":"host","kind":"declared-output","tracked":false}))
    });
    edit(&h.root, CLEANUP, |v| {
        v["artifacts"]
            .as_array_mut()
            .unwrap()
            .push(value!({"path":"fresh output/","disposition":"dispose"}))
    });
    let ignore = h.root.join(".gitignore");
    fs::write(
        &ignore,
        fs::read_to_string(&ignore).unwrap() + "fresh output/\n",
    )
    .unwrap();
    commit(&h.root);
    adopt(&h, &target);
    cache(&target);
    fs::create_dir(target.join("fresh output")).unwrap();
    fs::write(target.join("fresh output/data"), "new cache").unwrap();
    let head = git(&target, &["rev-parse", "HEAD"]);
    let source_bytes = fs::read(target.join("payload")).unwrap();
    assert_ne!(h.auto("maintain", &[]).0, 0);
    assert!(target.join("fresh output/data").exists());
    let result = migrate(&h, &target);
    assert_eq!(result["policy_migration"]["status"], "adopted");
    let entry = &h.ledger()["entries"][0];
    assert_eq!(entry["enrollment"], original["enrollment"]);
    assert_eq!(entry["policy_sha256"], original["policy_sha256"]);
    assert_eq!(entry["ownership"]["id"], original["ownership"]["id"]);
    assert_eq!(entry["status"], "active");
    assert!(entry["terminal"].is_null());
    assert_eq!(entry["policy_migrations"].as_array().unwrap().len(), 1);
    assert!(target.join("output λ/cache").exists());
    assert!(target.join("fresh output/data").exists());
    assert_eq!(fs::read(&birth_path).unwrap(), birth_bytes);
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(!target.join("fresh output").exists());
    assert!(!target.join("output λ").exists());
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), head);
    assert_eq!(fs::read(target.join("payload")).unwrap(), source_bytes);
    assert_eq!(fs::read(birth_path).unwrap(), birth_bytes);
}

#[test]
fn policy_migration_requires_committed_matching_target_and_coordinator() {
    let (h, target) = fixture();
    advance(&h);
    cache(&target);
    let ledger = fs::read(h.root.join(LEDGER)).unwrap();
    let (code, report, error) = h.auto("migrate", &["--path", target.to_str().unwrap()]);
    assert_ne!(code, 0, "{report} {error}");
    assert!(report.to_string().contains("must adopt the coordinator"));
    assert_eq!(fs::read(h.root.join(LEDGER)).unwrap(), ledger);
    adopt(&h, &target);
    let adopted = fs::read(target.join(CLEANUP)).unwrap();
    fs::write(target.join(CLEANUP), [adopted.as_slice(), b"\n"].concat()).unwrap();
    assert_ne!(
        h.auto("migrate", &["--path", target.to_str().unwrap()]).0,
        0
    );
    assert_eq!(fs::read(h.root.join(LEDGER)).unwrap(), ledger);
    fs::write(target.join(CLEANUP), &adopted).unwrap();
    fs::write(h.root.join(CLEANUP), [adopted.as_slice(), b"\n"].concat()).unwrap();
    assert_ne!(
        h.auto("migrate", &["--path", target.to_str().unwrap()]).0,
        0
    );
    assert_eq!(fs::read(h.root.join(LEDGER)).unwrap(), ledger);
    fs::write(h.root.join(CLEANUP), adopted).unwrap();
    migrate(&h, &target);
    assert!(target.join("output λ/cache").exists());
}

#[test]
fn policy_migration_obeys_original_live_kernel_lease() {
    let (h, target) = fixture();
    advance(&h);
    adopt(&h, &target);
    cache(&target);
    let entry = h.ledger()["entries"][0].clone();
    let held = fs::File::open(h.root.join(entry["ownership"]["path"].as_str().unwrap())).unwrap();
    held.lock_shared().unwrap();
    let (code, report, error) = h.auto("migrate", &["--path", target.to_str().unwrap()]);
    assert_ne!(code, 0, "{report} {error}");
    assert!(report.to_string().contains("live registered consumers"));
    assert_eq!(h.ledger()["entries"][0], entry);
    assert!(target.join("output λ/cache").exists());
    drop(held);
    migrate(&h, &target);
}

#[test]
fn policy_migration_retries_published_transition_without_growing_or_rewriting_history() {
    let (h, target) = fixture();
    advance(&h);
    adopt(&h, &target);
    let prior = fs::read(h.root.join(LEDGER)).unwrap();
    let first = migrate(&h, &target);
    let receipt = &first["policy_migration"]["receipt"];
    let path = h.root.join(receipt["path"].as_str().unwrap());
    let bytes = fs::read(&path).unwrap();
    // Reproduce the durable state between receipt publication and ledger save.
    fs::write(h.root.join(LEDGER), prior).unwrap();
    let retry = migrate(&h, &target);
    assert_eq!(retry["policy_migration"]["receipt"], *receipt);
    assert_eq!(fs::read(path).unwrap(), bytes);
    assert_eq!(
        h.ledger()["entries"][0]["policy_migrations"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    fs::write(target.join("payload"), "later source edit").unwrap();
    commit(&target);
    let next = migrate(&h, &target);
    assert_eq!(next["policy_migration"]["status"], "already-current");
    assert_eq!(
        h.ledger()["entries"][0]["policy_migrations"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn policy_migration_corrupt_receipt_or_retained_policy_blocks_later_disposal() {
    let (h, target) = fixture();
    advance(&h);
    adopt(&h, &target);
    let result = migrate(&h, &target);
    let receipt = h.root.join(
        result["policy_migration"]["receipt"]["path"]
            .as_str()
            .unwrap(),
    );
    let original = fs::read(&receipt).unwrap();
    cache(&target);
    fs::write(&receipt, [original.as_slice(), b"\n"].concat()).unwrap();
    assert_ne!(h.auto("maintain", &[]).0, 0);
    assert!(target.join("output λ/cache").exists());
    fs::write(receipt, &original).unwrap();
    let record = json(&original).unwrap();
    let input = h.root.join(
        record["migration_inputs"][0]["input"]["path"]
            .as_str()
            .unwrap(),
    );
    fs::write(input, "different original input").unwrap();
    let (code, report, error) = h.auto("maintain", &[]);
    assert_ne!(code, 0, "{report} {error}");
    assert!(report.to_string().contains("retained input mismatch"));
    assert!(target.join("output λ/cache").exists());
}

#[test]
fn policy_migration_requires_artifact_registration_at_both_endpoints() {
    let (h, target) = fixture();
    edit(&h.root, CONFIG, |v| {
        v["artifacts"].as_array_mut().unwrap().push(
            value!({"path":"new/output/","owner":"host","kind":"declared-output","tracked":false}),
        )
    });
    edit(&h.root, CLEANUP, |v| {
        v["artifacts"]
            .as_array_mut()
            .unwrap()
            .push(value!({"path":"new/output/","disposition":"dispose"}))
    });
    commit(&h.root);
    fs::copy(h.root.join(CLEANUP), target.join(CLEANUP)).unwrap();
    commit(&target);
    let prior = h.ledger()["entries"][0].clone();
    let (code, report, error) = h.auto("migrate", &["--path", target.to_str().unwrap()]);
    assert_ne!(code, 0, "{report} {error}");
    assert_eq!(h.ledger()["entries"][0], prior);
    fs::copy(h.root.join(CONFIG), target.join(CONFIG)).unwrap();
    commit(&target);
    migrate(&h, &target);
}

#[test]
fn policy_migration_retains_terminal_evidence_and_requires_a_new_finish() {
    let (h, target) = fixture();
    let (code, report, error) = h.auto(
        "finish",
        &["--path", target.to_str().unwrap(), "--artifacts-only"],
    );
    assert_eq!(code, 0, "{report} {error}");
    let prior = h.ledger()["entries"][0].clone();
    assert_eq!(prior["status"], "retained");
    let terminal = h
        .root
        .join(prior["terminal"]["receipt"]["path"].as_str().unwrap());
    let bytes = fs::read(&terminal).unwrap();
    advance(&h);
    adopt(&h, &target);
    cache(&target);
    migrate(&h, &target);
    let entry = &h.ledger()["entries"][0];
    assert_eq!(entry["status"], "retained");
    assert_eq!(entry["terminal"], prior["terminal"]);
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(target.join("output λ/cache").exists());
    let (code, report, error) = h.auto(
        "finish",
        &["--path", target.to_str().unwrap(), "--artifacts-only"],
    );
    assert_eq!(code, 0, "{report} {error}");
    assert!(!target.join("output λ").exists());
    assert_eq!(fs::read(terminal).unwrap(), bytes);
    assert_eq!(
        h.ledger()["entries"][0]["enrollment"]["terminal_history"][0],
        prior["terminal"]["receipt"]
    );
}

#[test]
fn policy_migration_supports_policy_cycles_and_rejects_a_missing_predecessor() {
    let (h, target) = fixture();
    let original_policy = fs::read(h.root.join(CLEANUP)).unwrap();
    advance(&h);
    adopt(&h, &target);
    let first = migrate(&h, &target);
    let receipt = h.root.join(
        first["policy_migration"]["receipt"]["path"]
            .as_str()
            .unwrap(),
    );
    let first_bytes = fs::read(&receipt).unwrap();
    fs::write(h.root.join(CLEANUP), original_policy).unwrap();
    commit(&h.root);
    adopt(&h, &target);
    migrate(&h, &target);
    let entry = &h.ledger()["entries"][0];
    assert_eq!(entry["policy_migrations"].as_array().unwrap().len(), 2);
    assert_eq!(entry["ownership"]["generation"], 3);
    assert_eq!(fs::read(receipt).unwrap(), first_bytes);
    cache(&target);
    let complete = fs::read(h.root.join(LEDGER)).unwrap();
    edit(&h.root, LEDGER, |v| {
        v["entries"][0]["policy_migrations"]
            .as_array_mut()
            .unwrap()
            .remove(0);
    });
    let (code, report, error) = h.auto("maintain", &[]);
    assert_ne!(code, 0, "{report} {error}");
    assert!(report.to_string().contains("migration chain"));
    assert!(target.join("output λ/cache").exists());
    fs::write(h.root.join(LEDGER), complete).unwrap();
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(!target.join("output λ").exists());
}
