#[path = "../../runner-tests/tests/support/isolated_capability.rs"]
mod chrono_git_boundary;
use super::*;
const CLEANUP: &str = ".chrono-harness/cleanup.json";

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
    if !chrono_git_boundary::isolated_git() { return; }
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
fn policy_migration_requires_each_endpoints_own_committed_inputs() {
    if !chrono_git_boundary::isolated_git() { return; }
    let (h, target) = fixture();
    advance(&h);
    cache(&target);
    let ledger = fs::read(h.ledger_path(&target)).unwrap();
    let (code, report, error) = h.auto("migrate", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{report} {error}");
    assert_eq!(report["policy_migration"]["status"], "already-current");
    assert_eq!(fs::read(h.ledger_path(&target)).unwrap(), ledger);
    adopt(&h, &target);
    let adopted = fs::read(target.join(CLEANUP)).unwrap();
    fs::write(target.join(CLEANUP), [adopted.as_slice(), b"\n"].concat()).unwrap();
    assert_ne!(
        h.auto("migrate", &["--path", target.to_str().unwrap()]).0,
        0
    );
    assert_eq!(fs::read(h.ledger_path(&target)).unwrap(), ledger);
    fs::write(target.join(CLEANUP), &adopted).unwrap();
    fs::write(h.root.join(CLEANUP), [adopted.as_slice(), b"\n"].concat()).unwrap();
    assert_ne!(
        h.auto("migrate", &["--path", target.to_str().unwrap()]).0,
        0
    );
    assert_eq!(fs::read(h.ledger_path(&target)).unwrap(), ledger);
    fs::write(h.root.join(CLEANUP), adopted).unwrap();
    migrate(&h, &target);
    assert!(target.join("output λ/cache").exists());
}

#[test]
fn independent_worktree_policy_migrates_without_changing_coordinator_choices() {
    if !chrono_git_boundary::isolated_git() { return; }
    let (h, target) = fixture();
    let coordinator_policy = fs::read(h.root.join(CLEANUP)).unwrap();
    let original = h.ledger()["entries"][0].clone();
    edit(&target, CLEANUP, |v| {
        v["artifacts"]
            .as_array_mut()
            .unwrap()
            .retain(|a| a["path"] != "nested/cache/");
    });
    commit(&target);
    cache(&target);
    fs::create_dir_all(target.join("nested/cache")).unwrap();
    fs::write(
        target.join("nested/cache/keep"),
        "another owner's selection",
    )
    .unwrap();
    assert_ne!(h.auto("maintain", &[]).0, 0);
    assert!(target.join("output λ/cache").exists());
    let result = migrate(&h, &target);
    assert_eq!(result["policy_migration"]["status"], "adopted");
    assert_eq!(
        h.ledger()["entries"][0]["enrollment"],
        original["enrollment"]
    );
    assert_eq!(
        h.ledger()["entries"][0]["ownership"]["id"],
        original["ownership"]["id"]
    );
    assert_eq!(fs::read(h.root.join(CLEANUP)).unwrap(), coordinator_policy);
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(!target.join("output λ").exists());
    assert!(target.join("nested/cache/keep").exists());
    assert_eq!(fs::read(h.root.join(CLEANUP)).unwrap(), coordinator_policy);
}

#[test]
fn policy_migration_obeys_original_live_kernel_lease() {
    if !chrono_git_boundary::isolated_git() { return; }
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
    if !chrono_git_boundary::isolated_git() { return; }
    let (h, target) = fixture();
    advance(&h);
    adopt(&h, &target);
    let prior = fs::read(h.ledger_path(&target)).unwrap();
    let first = migrate(&h, &target);
    let receipt = &first["policy_migration"]["receipt"];
    let path = h.root.join(receipt["path"].as_str().unwrap());
    let bytes = fs::read(&path).unwrap();
    // Reproduce the durable state between receipt publication and ledger save.
    fs::write(h.ledger_path(&target), prior).unwrap();
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
    if !chrono_git_boundary::isolated_git() { return; }
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
fn policy_migration_requires_artifact_registration_by_target_owner() {
    if !chrono_git_boundary::isolated_git() { return; }
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
fn target_policy_can_add_registered_outputs_without_coordinator_registration() {
    if !chrono_git_boundary::isolated_git() { return; }
    let (h, target) = fixture();
    let coordinator_policy = fs::read(h.root.join(CLEANUP)).unwrap();
    edit(&target, CONFIG, |v| {
        v["artifacts"].as_array_mut().unwrap().push(
            value!({"path":"local/output/","owner":"host","kind":"declared-output","tracked":false}));
    });
    edit(&target, CLEANUP, |v| {
        v["artifacts"]
            .as_array_mut()
            .unwrap()
            .push(value!({"path":"local/output/","disposition":"dispose"}));
    });
    let ignore = target.join(".gitignore");
    fs::write(
        &ignore,
        fs::read_to_string(&ignore).unwrap() + "local/output/\n",
    )
    .unwrap();
    commit(&target);
    fs::create_dir_all(target.join("local/output")).unwrap();
    fs::write(target.join("local/output/cache"), "rebuildable").unwrap();
    migrate(&h, &target);
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(!target.join("local/output").exists());
    assert_eq!(fs::read(h.root.join(CLEANUP)).unwrap(), coordinator_policy);
}

#[test]
fn linked_policy_cannot_redefine_shared_producer_storage_or_outputs() {
    if !chrono_git_boundary::isolated_git() { return; }
    let (h, target) = fixture();
    edit(&h.root, CLEANUP, |v| {
        v["retained_producers"] = value!([{"id":"actual-owner",
            "directory":".chrono-harness/state/actual-producer/", "temporary":true,
            "generated_outputs":["cache/"]}]);
    });
    commit(&h.root);
    edit(&target, CLEANUP, |v| {
        v["retained_producers"] = value!([{"id":"actual-owner",
            "directory":".chrono-harness/state/another-producer/", "temporary":true,
            "generated_outputs":["other-output/"]}]);
    });
    commit(&target);
    let (directory, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&target, POLICY, "actual-owner").unwrap();
    assert_eq!(
        directory,
        h.root.join(".chrono-harness/state/actual-producer")
    );
    assert_eq!(outputs, vec!["cache/".to_string()]);
    assert!(!directory.exists());
    assert!(
        !h.root
            .join(".chrono-harness/state/another-producer")
            .exists()
    );
}

#[test]
fn interrupted_removal_restores_target_config_even_when_cleanup_policy_is_identical() {
    if !chrono_git_boundary::isolated_git() { return; }
    let h = Host::new("payload");
    super::automatic::native_git(&h, "automatic-remove", value!({"after_effect":true}));
    h.kernel_cleanup();
    let target = h.parent.join("independent-config");
    assert_eq!(h.invoke("integration", "independent-config", &target).0, 0);
    edit(&target, POLICY, |v| {
        v["timeout_seconds"] = value!(v["timeout_seconds"].as_u64().unwrap() + 1);
    });
    commit(&target);
    migrate(&h, &target);
    let target_head = git(&target, &["rev-parse", "HEAD"]);
    git(&h.root, &["merge", "--ff-only", target_head.trim()]);
    edit(&h.root, POLICY, |v| {
        v["timeout_seconds"] = value!(v["timeout_seconds"].as_u64().unwrap() + 1);
    });
    commit(&h.root);
    assert_eq!(
        fs::read(target.join(CLEANUP)).unwrap(),
        fs::read(h.root.join(CLEANUP)).unwrap()
    );
    cache(&target);
    fs::write(h.parent.join("fail-remove"), "one failure").unwrap();
    let (code, failed, error) = h.auto(
        "finish",
        &["--path", target.to_str().unwrap(), "--dispose-evidence"],
    );
    assert_ne!(code, 0, "{failed} {error}");
    assert!(!target.exists());
    let original = drain_report(&failed["drain"][0]);
    let receipt = h.root.join(original["report_path"].as_str().unwrap());
    let bytes = fs::read(&receipt).unwrap();
    fs::remove_file(h.parent.join("fail-remove")).unwrap();
    let (code, retried, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{retried} {error}");
    let recovered = drain_report(&retried["drain"][0]);
    assert_eq!(recovered["policy_source_commit"], target_head.trim());
    assert_eq!(fs::read(receipt).unwrap(), bytes);
    assert_eq!(h.ledger()["entries"][0]["status"], "disposed");
}

#[test]
fn policy_migration_retains_terminal_evidence_and_requires_a_new_finish() {
    if !chrono_git_boundary::isolated_git() { return; }
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
    if !chrono_git_boundary::isolated_git() { return; }
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
    let complete = fs::read(h.ledger_path(&target)).unwrap();
    edit(
        &h.root,
        h.ledger_path(&target)
            .strip_prefix(&h.root)
            .unwrap()
            .to_str()
            .unwrap(),
        |v| {
            v["entries"][0]["policy_migrations"]
                .as_array_mut()
                .unwrap()
                .remove(0);
        },
    );
    let (code, report, error) = h.auto("maintain", &[]);
    assert_ne!(code, 0, "{report} {error}");
    assert!(report.to_string().contains("migration chain"));
    assert!(target.join("output λ/cache").exists());
    fs::write(h.ledger_path(&target), complete).unwrap();
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(!target.join("output λ").exists());
}

#[test]
fn immutable_input_limits_acquire_large_registered_configuration_and_preserve_drift_checks() {
    if !chrono_git_boundary::isolated_git() { return; }
    let h = Host::new("payload");
    let mut bytes = fs::read(h.root.join(CONFIG)).unwrap();
    bytes.resize(2_022_211, b' ');
    fs::write(h.root.join(CONFIG), &bytes).unwrap();
    h.policy(|p| p["immutable_input_limits"] = value!({CONFIG:2_097_152}));
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("large-input");
    let (code, report, error) = h.invoke("integration", "large-input", &target);
    assert_eq!(code, 0, "{} {error}", report["error"]);
    assert_eq!(fs::read(target.join(CONFIG)).unwrap(), bytes);
    let processes = report["processes"].as_array().unwrap();
    let data = processes
        .iter()
        .filter(|p| !p["immutable_input_transport"].is_null())
        .collect::<Vec<_>>();
    assert!(!data.is_empty());
    for row in data {
        assert_eq!(
            row["immutable_input_transport"]["stdout_limit_bytes"],
            2_022_211
        );
        assert_eq!(
            row["immutable_input_transport"]["stderr_limit_bytes"],
            1_048_576
        );
        assert_eq!(row["process"]["stdout_sha256"], sha256(&bytes));
        assert_eq!(row["process"]["exit_code"], 0);
    }
    // Creation is allowed to leave unrelated dirty source files alone. Its
    // governing worktree policy must still match the fixed source commit.
    let policy = fs::read(h.root.join(POLICY)).unwrap();
    fs::write(h.root.join(POLICY), [&policy[..], b"\n"].concat()).unwrap();
    let second = h.parent.join("changed-input");
    let (code, report, error) = h.invoke("integration", "changed-input", &second);
    assert_ne!(code, 0, "{} {error}", report["error"]);
    assert!(!second.exists());
}

#[test]
fn immutable_input_limits_refuse_undeclared_or_over_bound_data_before_worktree_creation() {
    if !chrono_git_boundary::isolated_git() { return; }
    for (path, declared) in [
        (CONFIG, None),
        (CONFIG, Some(2_022_210)),
        (".chrono-harness/", Some(2_097_152)),
    ] {
        let h = Host::new("payload");
        let mut bytes = fs::read(h.root.join(CONFIG)).unwrap();
        bytes.resize(2_022_211, b' ');
        fs::write(h.root.join(CONFIG), &bytes).unwrap();
        if let Some(limit) = declared {
            h.policy(|p| p["immutable_input_limits"] = value!({path:limit}));
        }
        commit(&h.root);
        git(&h.root, &["push", "-q", "warehouse", "dev"]);
        let target = h.parent.join("refused-input");
        let (code, report, error) = h.invoke("integration", "refused-input", &target);
        assert_ne!(code, 0, "{} {error}", report["error"]);
        assert!(!target.exists());
        assert!(report["error"].as_str().unwrap().contains(
            if declared.is_some() && path == CONFIG {
                "declared limit"
            } else {
                "output limit"
            }
        ));
        if declared.is_some() {
            assert!(
                report["processes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|p| p["immutable_input_transport"].is_null())
            );
        }
    }
}

#[test]
fn immutable_input_limits_reject_invalid_paths_and_capacity_before_git_launch() {
    if !chrono_git_boundary::isolated_git() { return; }
    for declaration in [
        value!({CONFIG:0}),
        value!({CONFIG:67_108_865}),
        value!({CONFIG:-1}),
        value!({CONFIG:1.5}),
        value!({"../config.json":2_097_152}),
        value!({"/config.json":2_097_152}),
        Value::Null,
    ] {
        let h = Host::new("payload");
        h.policy(|p| p["immutable_input_limits"] = declaration.clone());
        let target = h.parent.join("invalid-input-contract");
        let (code, report, error) = h.invoke("integration", "invalid-input", &target);
        assert_eq!(code, 2, "{report} {error}");
        assert!(error.contains("E_WORKTREE"), "{error}");
        assert!(
            report.is_null(),
            "invalid config must not produce process receipts"
        );
        assert!(!target.exists());
    }
}

#[test]
fn coordinator_input_capacity_preserves_existing_target_binding_and_fixed_sources() {
    if !chrono_git_boundary::isolated_git() { return; }
    let h = Host::new("payload");
    h.kernel_cleanup();
    let mut bytes = fs::read(h.root.join(CONFIG)).unwrap();
    bytes.resize(2_022_211, b' ');
    fs::write(h.root.join(CONFIG), &bytes).unwrap();
    // An already enrolled target keeps its historical diagnostic policy.
    h.policy(|p| p["output_limit_bytes"] = value!(2_097_152));
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("protected-input");
    let (code, report, error) = h.invoke("integration", "protected-input", &target);
    assert_eq!(code, 0, "{} {error}", report["error"]);
    // Maintenance must validate the current explicit migration binding,
    // retaining the original enrollment and each immutable transition.
    advance(&h);
    adopt(&h, &target);
    migrate(&h, &target);
    let original = h.ledger()["entries"][0].clone();
    assert_eq!(original["policy_migrations"].as_array().unwrap().len(), 1);
    let head = git(&target, &["rev-parse", "HEAD"]);
    let policy = fs::read(target.join(POLICY)).unwrap();
    h.policy(|p| {
        p["output_limit_bytes"] = value!(1_048_576);
        p["immutable_input_limits"] = value!({CONFIG:2_097_152});
    });
    commit(&h.root);
    cache(&target);
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{} {error}", report["error"]);
    assert!(!target.join("output λ/cache").exists());
    assert_eq!(fs::read(target.join(CONFIG)).unwrap(), bytes);
    assert_eq!(fs::read(target.join(POLICY)).unwrap(), policy);
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), head);
    let entry = h.ledger()["entries"][0].clone();
    assert_eq!(entry["enrollment"], original["enrollment"]);
    assert_eq!(entry["policy_migrations"], original["policy_migrations"]);
    assert_eq!(entry["status"], "active");
    assert!(entry["terminal"].is_null());
}

#[test]
fn checkout_migration_preserves_birth_old_commits_and_uncommitted_work() {
    if !chrono_git_boundary::isolated_git() { return; }
    let h = Host::new("payload");
    h.kernel_cleanup();
    super::automatic::native_consumer(&h, "ordinary");
    let target = h.parent.join("reused checkout");
    assert_eq!(h.invoke("integration", "first-task", &target).0, 0);
    fs::write(target.join("payload"), "first task's unique commit\n").unwrap();
    let old_head = commit(&target);
    let original = h.ledger()["entries"][0].clone();
    let birth = h.root.join(
        original["enrollment"]["sealed_receipt"]["path"]
            .as_str()
            .unwrap(),
    );
    let birth_bytes = fs::read(&birth).unwrap();
    git(&target, &["checkout", "-b", "integration/next-task", "dev"]);
    fs::write(target.join("payload"), "staged second task\n").unwrap();
    git(&target, &["add", "payload"]);
    fs::write(target.join("payload"), "unstaged second task\n").unwrap();
    fs::write(target.join("untracked"), "keep this too\n").unwrap();
    cache(&target);
    let metadata = PathBuf::from(git(&target, &["rev-parse", "--absolute-git-dir"]));
    let originals = [
        metadata.join("HEAD"),
        metadata.join("index"),
        target.join("payload"),
        target.join("untracked"),
    ]
    .map(|path| {
        let bytes = fs::read(&path).unwrap();
        (path, bytes)
    });
    let branches = git(&h.root, &["for-each-ref", "refs/heads/"]);
    let ledger = fs::read(h.ledger_path(&target)).unwrap();
    let (code, refused, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_ne!(code, 0, "{refused} {error}");
    assert!(
        refused["error"]
            .as_str()
            .unwrap()
            .contains("identity or ownership lock mismatch")
    );
    assert_eq!(fs::read(h.ledger_path(&target)).unwrap(), ledger);

    let report = migrate(&h, &target);
    assert_eq!(report["checkout_migration"]["status"], "adopted");
    assert_eq!(
        report["checkout_migration"]["previous_branch"],
        "integration/first-task"
    );
    assert_eq!(
        report["checkout_migration"]["branch"],
        "integration/next-task"
    );
    assert_eq!(report["policy_migration"]["status"], "already-current");
    let adopted = h.ledger()["entries"][0].clone();
    assert_eq!(adopted["branch"], original["branch"]);
    assert_eq!(adopted["checkout_branch"], "integration/next-task");
    assert_eq!(adopted["enrollment"], original["enrollment"]);
    assert_eq!(
        adopted["ownership"]["generation"],
        original["ownership"]["generation"].as_u64().unwrap() + 1
    );
    assert_eq!(adopted["policy_migrations"], original["policy_migrations"]);
    assert_eq!(adopted["status"], "active");
    assert!(adopted["terminal"].is_null());
    let adopted_bytes = fs::read(h.ledger_path(&target)).unwrap();
    let repeated = migrate(&h, &target);
    assert_eq!(repeated["checkout_migration"]["status"], "already-current");
    assert_eq!(fs::read(h.ledger_path(&target)).unwrap(), adopted_bytes);
    for (path, bytes) in originals {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    assert_eq!(fs::read(&birth).unwrap(), birth_bytes);
    assert_eq!(git(&h.root, &["for-each-ref", "refs/heads/"]), branches);
    assert_eq!(
        git(&h.root, &["rev-parse", "integration/first-task"]),
        old_head
    );
    assert!(target.join("output λ/cache").exists());

    let (code, used, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "{used} {error}");
    assert_eq!(used["managed_process"]["stdout"], "ordinary-work");
    let (code, status, error) = h.auto("status", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{status} {error}");
    assert_eq!(
        status["worktree_status"]["worktrees"][0]["branch"],
        "integration/next-task"
    );
    assert_eq!(fs::read(birth).unwrap(), birth_bytes);
}

#[test]
fn checkout_migration_preserves_quiescent_cleanup_observations_and_cache_history() {
    if !chrono_git_boundary::isolated_git() { return; }
    for busy in [true, false] {
        let (h, target) = fixture();
        cache(&target);
        let entry = h.ledger()["entries"][0].clone();
        let held = busy.then(|| {
            let held =
                fs::File::open(h.root.join(entry["ownership"]["path"].as_str().unwrap())).unwrap();
            held.lock_shared().unwrap();
            held
        });
        let (code, maintained, error) = h.auto("maintain", &[]);
        assert_eq!(code, 0, "{maintained} {error}");
        let original = h.ledger()["entries"][0].clone();
        assert_eq!(
            original["cleanup"]["stage"],
            if busy {
                "waiting-for-consumers"
            } else {
                "caches-released"
            }
        );
        assert_eq!(original["status"], "active");
        let report = h
            .root
            .join(original["cleanup"]["report_path"].as_str().unwrap());
        let report_bytes = fs::read(&report).unwrap();
        let cache_exists = target.join("output λ/cache").exists();
        assert_eq!(cache_exists, busy);
        drop(held);

        git(&target, &["checkout", "-b", "integration/next-task"]);
        let migrated = migrate(&h, &target);
        assert_eq!(migrated["checkout_migration"]["status"], "adopted");
        let adopted = h.ledger()["entries"][0].clone();
        assert_eq!(adopted["checkout_branch"], "integration/next-task");
        for field in [
            "enrollment",
            "cleanup",
            "cache_attempts",
            "terminal",
            "uses",
        ] {
            assert_eq!(adopted[field], original[field], "{field}");
        }
        assert_eq!(target.join("output λ/cache").exists(), cache_exists);
        assert_eq!(fs::read(report).unwrap(), report_bytes);
        assert_eq!(adopted["status"], "active");
    }
}

#[test]
fn checkout_migration_hands_off_a_sealed_result_without_maintaining_other_worktrees() {
    if !chrono_git_boundary::isolated_git() { return; }
    let (h, target, used) = checkout_result_fixture();
    let other = h.parent.join("unrelated-handoff");
    let (code, created, error) = h.invoke("integration", "unrelated-handoff", &other);
    assert_eq!(code, 0, "{created} {error}");
    let other_ledger = fs::read(h.ledger_path(&other)).unwrap();
    let entry = json(&other_ledger).unwrap()["entries"][0].clone();
    let held = fs::File::open(h.root.join(entry["ownership"]["path"].as_str().unwrap())).unwrap();
    held.lock_shared().unwrap();
    let before = json(&fs::read(h.ledger_path(&target)).unwrap()).unwrap()["entries"][0].clone();
    let original_path = h.root.join(used["report_path"].as_str().unwrap());
    let original = fs::read(&original_path).unwrap();
    let receipt_path = h
        .root
        .join(used["managed_use_receipt"]["path"].as_str().unwrap());
    let receipt = fs::read(&receipt_path).unwrap();
    cache(&target);
    git(&target, &["checkout", "-b", "integration/next-handoff"]);

    let report = migrate(&h, &target);
    assert_eq!(report["checkout_migration"]["status"], "adopted");
    let after = json(&fs::read(h.ledger_path(&target)).unwrap()).unwrap()["entries"][0].clone();
    let mut expected = before["enrollment"].clone();
    expected.as_object_mut().unwrap().remove("completed_uses");
    assert_eq!(after["enrollment"], expected);
    assert_eq!(after["branch"], before["branch"]);
    assert_eq!(after["checkout_branch"], "integration/next-handoff");
    assert_eq!(after["status"], "active");
    assert!(after["terminal"].is_null());
    assert_eq!(after["policy_migrations"], before["policy_migrations"]);
    assert_eq!(fs::read(&original_path).unwrap(), original);
    assert_eq!(fs::read(&receipt_path).unwrap(), receipt);
    assert!(target.join("output λ/cache").is_file());
    assert_eq!(fs::read(h.ledger_path(&other)).unwrap(), other_ledger);
    assert!(
        report["processes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["root"] != value!(other))
    );
    drop(held);
}

fn checkout_result_fixture() -> (Host, PathBuf, Value) {
    use chrono_harness::retained_artifacts::retention::{Inventory, POLICY_PATH};
    let h = Host::new("payload");
    h.kernel_cleanup();
    automatic::native_consumer(&h, "ordinary");
    adopt_lifecycle_retention(&h);
    edit(&h.root, POLICY_PATH, |policy| {
        policy["producers"]["lifecycle-original"] = value!([
            ".chrono-harness/state/worktrees/",
            ".chrono-harness/state/automatic-cleanup/"
        ]);
        policy["max_related_outputs"] = value!(512);
    });
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("completed-use-handoff");
    let (code, created, error) = h.invoke("integration", "completed-use-handoff", &target);
    assert_eq!(code, 0, "{created} {error}");
    let (code, used, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "{used} {error}");
    let entry = json(&fs::read(h.ledger_path(&target)).unwrap()).unwrap()["entries"][0].clone();
    assert_eq!(
        entry["enrollment"]["completed_uses"],
        value!([used["managed_use_receipt"]])
    );
    Inventory::adopted(&h.root)
        .unwrap()
        .unwrap()
        .reference_paths(
            "fixture-use-report",
            &[used["report_path"].as_str().unwrap().into()],
        )
        .unwrap();
    (h, target, used)
}

#[test]
fn checkout_migration_keeps_unqualified_result_handoffs_pending() {
    if !chrono_git_boundary::isolated_git() { return; }
    use std::os::unix::fs::MetadataExt;

    let (h, target, used) = checkout_result_fixture();
    let path = h.root.join(used["report_path"].as_str().unwrap());
    let saved_original = path.with_extension("saved-original");
    let bytes = fs::read(&path).unwrap();
    let metadata = fs::metadata(&path).unwrap();
    let original_identity = (metadata.dev(), metadata.ino());
    git(&target, &["checkout", "-b", "integration/preserve-pending"]);
    let before = fs::read(h.ledger_path(&target)).unwrap();
    let receipt_path = h
        .root
        .join(used["managed_use_receipt"]["path"].as_str().unwrap());
    let receipt = fs::read(&receipt_path).unwrap();
    for fault in [
        "missing", "partial", "binding", "changed", "replaced", "unsealed",
    ] {
        let metadata = fs::metadata(&path).unwrap();
        assert_eq!(
            (metadata.dev(), metadata.ino()),
            original_identity,
            "{fault}"
        );
        assert_eq!(fs::read(&path).unwrap(), bytes, "{fault}");
        match fault {
            "missing" => fs::rename(&path, &saved_original).unwrap(),
            "partial" => fs::write(&path, b"{\"status\":").unwrap(),
            "binding" | "changed" => {
                let mut report = json(&bytes).unwrap();
                if fault == "binding" {
                    report["managed_use_receipt"]["sha256"] = value!("0".repeat(64));
                } else {
                    report["untrusted_edit"] = value!(true);
                }
                fs::write(&path, serde_json::to_vec(&report).unwrap()).unwrap();
            }
            "replaced" => {
                fs::rename(&path, &saved_original).unwrap();
                fs::write(&path, &bytes).unwrap();
            }
            "unsealed" => {
                let mut output = retention_state::snapshot(&h.root)["outputs"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|output| output["path"] == used["report_path"])
                    .unwrap()
                    .clone();
                output["phase"] = value!("publishing");
                retention_state::replace_output(&h.root, &output);
            }
            _ => unreachable!(),
        }
        let (code, report, error) = h.auto("migrate", &["--path", target.to_str().unwrap()]);
        assert_ne!(code, 0, "{fault}: {report} {error}");
        assert_eq!(fs::read(h.ledger_path(&target)).unwrap(), before, "{fault}");
        assert_eq!(fs::read(&receipt_path).unwrap(), receipt, "{fault}");
        assert!(target.is_dir(), "{fault}");
        match fault {
            "missing" => fs::rename(&saved_original, &path).unwrap(),
            "replaced" => {
                fs::remove_file(&path).unwrap();
                fs::rename(&saved_original, &path).unwrap();
            }
            "partial" | "binding" | "changed" => fs::write(&path, &bytes).unwrap(),
            "unsealed" => {}
            _ => unreachable!(),
        }
    }
}

#[test]
fn checkout_migration_rejects_candidate_policy_before_releasing_a_completed_result() {
    if !chrono_git_boundary::isolated_git() { return; }
    let (h, target, _) = checkout_result_fixture();
    git(&target, &["checkout", "-b", "integration/wrong-policy"]);
    edit(&target, CLEANUP, |policy| {
        policy["allow_evidence_disposal"] = value!(false)
    });
    let before = fs::read(h.ledger_path(&target)).unwrap();
    let (code, report, error) = h.auto("migrate", &["--path", target.to_str().unwrap()]);
    assert_ne!(code, 0, "{report} {error}");
    assert_eq!(fs::read(h.ledger_path(&target)).unwrap(), before);
}

#[test]
fn checkout_migration_keeps_live_unjoined_and_terminal_work_protected() {
    if !chrono_git_boundary::isolated_git() { return; }
    let (h, target) = fixture();
    git(&target, &["checkout", "-b", "integration/reuse"]);
    let original = fs::read(h.ledger_path(&target)).unwrap();
    let entry = h.ledger()["entries"][0].clone();
    let held = fs::File::open(h.root.join(entry["ownership"]["path"].as_str().unwrap())).unwrap();
    held.lock_shared().unwrap();
    let (code, refused, error) = h.auto("migrate", &["--path", target.to_str().unwrap()]);
    assert_ne!(code, 0, "{refused} {error}");
    assert!(
        refused["error"]
            .as_str()
            .unwrap()
            .contains("live registered consumers")
    );
    assert_eq!(fs::read(h.ledger_path(&target)).unwrap(), original);
    drop(held);
    for field in ["uses", "completed_uses"] {
        let mut ledger = json(&original).unwrap();
        if field == "uses" {
            ledger["entries"][0][field] = value!(["unknown-unjoined-use"]);
        } else {
            ledger["entries"][0]["enrollment"][field] = value!([{"unjoined":"original-result"}]);
        }
        let pending = serde_json::to_vec_pretty(&ledger).unwrap();
        fs::write(h.ledger_path(&target), &pending).unwrap();
        let (code, refused, error) = h.auto("migrate", &["--path", target.to_str().unwrap()]);
        assert_ne!(code, 0, "{refused} {error}");
        assert!(
            refused["error"].as_str().unwrap().contains("pending work"),
            "{refused}"
        );
        assert_eq!(fs::read(h.ledger_path(&target)).unwrap(), pending);
    }
    fs::write(h.ledger_path(&target), &original).unwrap();
    git(&target, &["checkout", "integration/migrating"]);
    assert_eq!(
        h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--artifacts-only"]
        )
        .0,
        0
    );
    git(&target, &["checkout", "integration/reuse"]);
    let terminal = fs::read(h.ledger_path(&target)).unwrap();
    let (code, refused, error) = h.auto("migrate", &["--path", target.to_str().unwrap()]);
    assert_ne!(code, 0, "{refused} {error}");
    assert!(refused["error"].as_str().unwrap().contains("pending work"));
    assert_eq!(fs::read(h.ledger_path(&target)).unwrap(), terminal);
}

#[test]
fn checkout_migration_checks_current_ref_locks_and_keeps_branch_contract() {
    if !chrono_git_boundary::isolated_git() { return; }
    let (h, target) = fixture();
    let original = fs::read(h.ledger_path(&target)).unwrap();
    for args in [
        vec!["checkout", "--detach", "dev"],
        vec!["checkout", "-b", "client/unregistered"],
    ] {
        git(&target, &args);
        let (code, refused, error) = h.auto("migrate", &["--path", target.to_str().unwrap()]);
        assert_ne!(code, 0, "{refused} {error}");
        assert!(
            refused["error"]
                .as_str()
                .unwrap()
                .contains("registered local work branch"),
            "{refused}"
        );
        assert_eq!(fs::read(h.ledger_path(&target)).unwrap(), original);
    }
    git(&target, &["checkout", "-b", "integration/reuse"]);
    let lock = h.root.join(".git/refs/heads/integration/reuse.lock");
    fs::write(&lock, "unknown owner\n").unwrap();
    let (code, refused, error) = h.auto("migrate", &["--path", target.to_str().unwrap()]);
    assert_ne!(code, 0, "{refused} {error}");
    assert!(
        refused["error"]
            .as_str()
            .unwrap()
            .contains("Git lock remains owned or unknown")
    );
    assert_eq!(fs::read(&lock).unwrap(), b"unknown owner\n");
    assert_eq!(fs::read(h.ledger_path(&target)).unwrap(), original);
    fs::remove_file(lock).unwrap();
    migrate(&h, &target);
    git(&target, &["checkout", "integration/migrating"]);
    let report = migrate(&h, &target);
    assert_eq!(report["checkout_migration"]["status"], "adopted");
    assert!(h.ledger()["entries"][0]["checkout_branch"].is_null());
}

#[test]
fn checkout_and_policy_adoption_share_one_generation_and_cleanup_uses_current_branch() {
    if !chrono_git_boundary::isolated_git() { return; }
    let (h, target) = fixture();
    fs::write(target.join("payload"), "old task unique source\n").unwrap();
    let old_head = commit(&target);
    let birth = h.ledger()["entries"][0]["enrollment"].clone();
    edit(&h.root, CLEANUP, |v| v["remove_branch"] = value!(true));
    let head = commit(&h.root);
    git(&target, &["checkout", "-b", "integration/reuse", &head]);
    let original_generation = h.ledger()["entries"][0]["ownership"]["generation"]
        .as_u64()
        .unwrap();
    let report = migrate(&h, &target);
    assert_eq!(report["checkout_migration"]["status"], "adopted");
    assert_eq!(report["policy_migration"]["status"], "adopted");
    assert_eq!(
        h.ledger()["entries"][0]["ownership"]["generation"],
        original_generation + 1
    );
    assert_eq!(
        h.ledger()["entries"][0]["policy_migrations"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(h.ledger()["entries"][0]["enrollment"], birth);
    let (code, finished, error) = h.auto(
        "finish",
        &["--path", target.to_str().unwrap(), "--dispose-evidence"],
    );
    assert_eq!(code, 0, "{finished} {error}");
    assert!(!target.exists(), "{finished}");
    assert_eq!(
        git(&h.root, &["rev-parse", "integration/migrating"]),
        old_head
    );
    assert!(
        !git(&h.root, &["for-each-ref", "refs/heads/integration/reuse"])
            .contains("refs/heads/integration/reuse")
    );
    let terminal = &h.ledger()["entries"][0]["terminal"]["receipt"];
    let terminal =
        json(&fs::read(h.root.join(terminal["path"].as_str().unwrap())).unwrap()).unwrap();
    assert_eq!(terminal["branch"], "integration/reuse");
}

#[test]
fn imported_checkout_can_adopt_local_or_detached_binding_without_claiming_new_birth() {
    if !chrono_git_boundary::isolated_git() { return; }
    let h = Host::new("payload");
    h.kernel_cleanup();
    let target = h.parent.join("imported reuse");
    git(
        &h.root,
        &[
            "worktree",
            "add",
            "--detach",
            target.to_str().unwrap(),
            "dev",
        ],
    );
    let (code, imported, error) = h.auto("import", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{imported} {error}");
    let original = h.ledger()["entries"][0].clone();
    git(&target, &["checkout", "-b", "client/imported"]);
    migrate(&h, &target);
    assert_eq!(
        h.ledger()["entries"][0]["checkout_branch"],
        "client/imported"
    );
    git(&target, &["checkout", "--detach"]);
    migrate(&h, &target);
    let current = h.ledger()["entries"][0].clone();
    assert_eq!(current["branch"], "HEAD");
    assert!(current["checkout_branch"].is_null());
    assert_eq!(current["enrollment"], original["enrollment"]);
    assert!(current["terminal"].is_null());
}
