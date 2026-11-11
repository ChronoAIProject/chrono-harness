#[path = "../../runner-tests/tests/support/isolated_capability.rs"]
mod chrono_git_boundary;
use super::automatic::{native_consumer, native_git};
use super::*;
use std::os::unix::fs::MetadataExt;
use std::time::{Duration, SystemTime};

fn policy(h: &Host, reuse: u64, delete: u64, evidence: &str) {
    h.automatic(evidence);
    git(&h.root, &["config", "user.name", "Lifecycle fixture"]);
    git(
        &h.root,
        &["config", "user.email", "lifecycle@example.invalid"],
    );
    let path = h.root.join(".chrono-harness/cleanup.json");
    let mut p = json(&fs::read(&path).unwrap()).unwrap();
    p["schema"] = value!("chrono-worktree-automatic-cleanup/v2");
    p["age_policy"] = value!({"reuse_after_seconds":reuse,"delete_after_seconds":delete,
        "preserved_branch_prefix":"preserved/age/"});
    p["artifacts"][0]["disposition"] = value!("finish-dispose");
    fs::write(path, serde_json::to_vec_pretty(&p).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
}

// Wait once against the real filesystem birth time. Tests never change a ledger,
// a timestamp, the product clock or the machine clock to make an object eligible.
fn older_than(target: &Path, seconds: u64) {
    let created = fs::metadata(target).unwrap().created().unwrap();
    let boundary = created + Duration::from_secs(seconds) + Duration::from_millis(50);
    if let Ok(wait) = boundary.duration_since(SystemTime::now()) {
        std::thread::sleep(wait);
    }
}
fn entry(h: &Host, target: &Path) -> Value {
    json(&fs::read(h.ledger_path(target)).unwrap()).unwrap()["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["status"] != "disposed")
        .unwrap()
        .clone()
}
fn output(target: &Path) {
    fs::create_dir_all(target.join("output λ")).unwrap();
    fs::write(target.join("output λ/cache"), vec![11; 4096]).unwrap();
}

fn cache_identity(target: &Path) -> (u64, u64) {
    let cache = target.join("output λ/cache");
    assert_eq!(fs::read(&cache).unwrap(), vec![11; 4096]);
    let metadata = fs::metadata(cache).unwrap();
    (metadata.dev(), metadata.ino())
}

fn temporary_output(target: &Path) {
    fs::create_dir_all(target.join("nested/cache")).unwrap();
    fs::write(target.join("nested/cache/temporary"), b"disposable").unwrap();
}

fn start(h: &Host, name: &str, target: &Path) {
    let (code, report, error) = h.invoke("feature", name, target);
    assert_eq!(code, 0, "{report} {error}");
}

#[test]
fn maintenance_selects_creation_age_from_each_targets_adopted_policy() {
    if !chrono_git_boundary::isolated_git() { return; }
    for (coordinator, target_policy, expired) in [
        (Some((1, 3)), Some((3600, 7200)), false),
        (Some((1, 3)), None, false),
        (Some((3600, 7200)), Some((1, 3)), true),
        (None, Some((1, 3)), true),
    ] {
        let h = Host::new("payload");
        let (reuse, delete) = coordinator.unwrap_or((3600, 7200));
        policy(&h, reuse, delete, "dispose");
        if coordinator.is_none() {
            let path = h.root.join(".chrono-harness/cleanup.json");
            let mut p = json(&fs::read(&path).unwrap()).unwrap();
            p.as_object_mut().unwrap().remove("age_policy");
            fs::write(path, serde_json::to_vec_pretty(&p).unwrap()).unwrap();
            commit(&h.root);
            git(&h.root, &["push", "-q", "warehouse", "dev"]);
        }
        let coordinator_bytes = fs::read(h.root.join(".chrono-harness/cleanup.json")).unwrap();
        let target = h.parent.join("independent-age-policy");
        start(&h, "independent-age-policy", &target);
        let path = target.join(".chrono-harness/cleanup.json");
        let mut p = json(&fs::read(&path).unwrap()).unwrap();
        if let Some((reuse, delete)) = target_policy {
            p["age_policy"]["reuse_after_seconds"] = value!(reuse);
            p["age_policy"]["delete_after_seconds"] = value!(delete);
            p["age_policy"]["preserved_branch_prefix"] = value!("preserved/target-age/");
        } else {
            p.as_object_mut().unwrap().remove("age_policy");
        }
        fs::write(path, serde_json::to_vec_pretty(&p).unwrap()).unwrap();
        let head = commit(&target);
        let (code, report, error) = h.auto("migrate", &["--path", target.to_str().unwrap()]);
        assert_eq!(code, 0, "{report} {error}");
        output(&target);
        temporary_output(&target);
        older_than(&target, 3);
        let (code, report, error) = h.auto("maintain", &[]);
        assert_eq!(code, 0, "{report} {error}");
        assert_eq!(target.exists(), !expired, "{report}");
        assert_eq!(
            fs::read(h.root.join(".chrono-harness/cleanup.json")).unwrap(),
            coordinator_bytes
        );
        if expired {
            let row = report["drain"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["path"] == value!(target))
                .unwrap();
            let original = drain_report(row);
            assert_eq!(
                original["source_preservation"]["original_head"],
                head.trim()
            );
            let reference = original["source_preservation"]["original_ref"]
                .as_str()
                .unwrap();
            assert!(reference.starts_with("refs/heads/preserved/target-age/"));
            assert_eq!(git(&h.root, &["rev-parse", reference]).trim(), head.trim());
        } else {
            assert!(target.join("output λ/cache").exists());
            assert!(!target.join("nested/cache").exists());
            assert_eq!(git(&target, &["rev-parse", "HEAD"]).trim(), head.trim());
        }
    }
}

#[test]
fn real_creation_age_reuse_and_expiry_preserve_source_index_and_branches() {
    if !chrono_git_boundary::isolated_git() { return; }
    let h = Host::new("payload");
    policy(&h, 2, 60, "dispose");
    native_consumer(&h, "ordinary");
    let target = h.parent.join("real-aged-checkout");
    start(&h, "real-aged-checkout", &target);
    let original = git(&target, &["rev-parse", "HEAD"]);
    fs::write(target.join("payload"), "reflog-only-source").unwrap();
    let reflog_only = commit(&target);
    git(&target, &["reset", "--hard", &original]);
    fs::write(target.join("payload"), "staged-original").unwrap();
    git(&target, &["add", "payload"]);
    fs::write(target.join("payload"), "current-source").unwrap();
    fs::write(
        target.join(".gitignore"),
        ".chrono-harness/state/\n.chrono-harness/bin/\noutput λ/\nnested/cache/\nignored-source\n",
    )
    .unwrap();
    fs::write(target.join("ignored-source"), "ignored but unique source").unwrap();
    fs::write(target.join("wildcard*[abc]$literal"), "literal source").unwrap();
    output(&target);
    let cache = cache_identity(&target);
    older_than(&target, 2);
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
    assert!(used["age_reuse"].is_null(), "{used}");
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), original);
    assert_eq!(git(&target, &["show", ":payload"]), "staged-original");
    assert_eq!(
        fs::read_to_string(target.join("payload")).unwrap(),
        "current-source"
    );
    assert!(target.join("output λ/cache").is_file());
    temporary_output(&target);
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    let result = maintained["drain"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == value!(target) && !row["receipt"].is_null())
        .unwrap();
    let original_result = drain_report(result);
    let saved = &original_result["source_preservation"];
    assert_eq!(saved["original_head"], original);
    assert!(
        saved["reflog_refs"]
            .as_object()
            .unwrap()
            .values()
            .any(|v| v == &value!(reflog_only))
    );
    let show = |reference: &Value, path: &str| {
        git(
            &h.root,
            &["show", &format!("{}:{path}", reference.as_str().unwrap())],
        )
    };
    assert_eq!(show(&saved["index_ref"], "payload"), "staged-original");
    assert_eq!(show(&saved["retained_ref"], "payload"), "current-source");
    assert_eq!(
        show(&saved["retained_ref"], "ignored-source"),
        "ignored but unique source"
    );
    assert_eq!(
        show(&saved["retained_ref"], "wildcard*[abc]$literal"),
        "literal source"
    );
    assert!(target.is_dir());
    assert_eq!(cache_identity(&target), cache);
    assert!(!target.join("nested/cache").exists());
    assert_eq!(original_result["age_outcome"], "reusable-source-preserved");
    let reference = saved["retained_ref"].as_str().unwrap().to_owned();
    let head = saved["head"].as_str().unwrap().to_owned();
    temporary_output(&target);
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert_eq!(cache_identity(&target), cache);
    assert!(!target.join("nested/cache").exists());
    let (code, reused, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "{reused} {error}");
    assert!(reused["age_reuse"].is_null(), "{reused}");
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), head);
    assert!(target.join("output λ/cache").is_file());
    assert_eq!(cache_identity(&target), cache);
    // Modification now must not extend checkout creation age.
    fs::write(target.join("payload"), "source at deletion").unwrap();
    older_than(&target, 60);
    let (code, maintained, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert!(!target.exists());
    assert_eq!(git(&h.root, &["rev-parse", &reference]), head);
    let disposed = json(&fs::read(h.ledger_path(&target)).unwrap()).unwrap();
    assert_eq!(disposed["entries"][0]["status"], "disposed");
    let result = maintained["drain"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["path"] == value!(target))
        .unwrap();
    let original_result = drain_report(result);
    assert_eq!(
        original_result["age_outcome"],
        "expired-checkout-verified-absent-source-branches-retained"
    );
    assert_eq!(
        show(
            &original_result["source_preservation"]["retained_ref"],
            "payload"
        ),
        "source at deletion"
    );
}

#[test]
fn aged_busy_consumer_defers_and_independent_expired_checkout_advances() {
    if !chrono_git_boundary::isolated_git() { return; }
    let h = Host::new("payload");
    policy(&h, 2, 5, "dispose");
    let busy = h.parent.join("aged-busy");
    let free = h.parent.join("aged-free");
    start(&h, "aged-busy", &busy);
    start(&h, "aged-free", &free);
    output(&busy);
    output(&free);
    fs::write(busy.join("payload"), "live-source").unwrap();
    let e = entry(&h, &busy);
    let lease = chrono_harness::ownership::Lease::acquire(
        &h.root.join(e["ownership"]["path"].as_str().unwrap()),
        Some(e["ownership"]["id"].as_str().unwrap()),
        false,
        false,
        None,
    )
    .unwrap()
    .unwrap();
    older_than(&free, 5);
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(busy.join("output λ/cache").exists());
    assert_eq!(
        fs::read_to_string(busy.join("payload")).unwrap(),
        "live-source"
    );
    assert!(!free.exists());
    assert!(report["drain"].as_array().unwrap().iter().any(|r| {
        r["path"] == value!(busy)
            && r["preserved_reason"]
                .as_str()
                .is_some_and(|s| s.contains("live registered"))
    }));
    drop(lease);
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(!busy.exists());
}

#[test]
fn detached_age_reuse_preserves_source_and_index_without_moving_other_branches() {
    if !chrono_git_boundary::isolated_git() { return; }
    let h = Host::new("payload");
    // Keep reuse and expiry as separate real-birth phases, as in the named
    // checkout test; Git/source preservation is allowed to take over 3 seconds.
    policy(&h, 1, 60, "dispose");
    native_consumer(&h, "ordinary");
    let target = h.parent.join("detached aged checkout");
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
    let head = git(&target, &["rev-parse", "HEAD"]);
    let dev = git(&h.root, &["rev-parse", "dev"]);
    let (code, imported, error) = h.auto("import", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{imported} {error}");
    fs::write(target.join("payload"), "original staged source").unwrap();
    git(&target, &["add", "payload"]);
    fs::write(target.join("payload"), "current detached source").unwrap();
    output(&target);
    let cache = cache_identity(&target);
    older_than(&target, 1);
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
    assert!(used["age_reuse"].is_null(), "{used}");
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        git(&target, &["show", ":payload"]),
        "original staged source"
    );
    assert_eq!(
        fs::read_to_string(target.join("payload")).unwrap(),
        "current detached source"
    );
    assert!(target.join("output λ/cache").is_file());
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    let result = maintained["drain"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == value!(target) && !row["receipt"].is_null())
        .unwrap();
    let original_result = drain_report(result);
    let saved = &original_result["source_preservation"];
    assert_eq!(saved["original_head"], head);
    assert_ne!(saved["head"], head);
    let show = |reference: &Value| {
        git(
            &h.root,
            &["show", &format!("{}:payload", reference.as_str().unwrap())],
        )
    };
    assert_eq!(show(&saved["index_ref"]), "original staged source");
    assert_eq!(show(&saved["retained_ref"]), "current detached source");
    assert_eq!(git(&h.root, &["rev-parse", "dev"]), dev);
    assert_eq!(git(&target, &["rev-parse", "--abbrev-ref", "HEAD"]), "HEAD");
    assert_eq!(
        git(&target, &["rev-parse", "HEAD"]),
        saved["head"].as_str().unwrap()
    );
    assert_eq!(
        fs::read_to_string(target.join("payload")).unwrap(),
        "current detached source"
    );
    assert_eq!(cache_identity(&target), cache);
    older_than(&target, 60);
    let (code, finished, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{finished} {error}");
    assert!(!target.exists(), "{finished}");
    assert_eq!(git(&h.root, &["rev-parse", "dev"]), dev);
    assert_eq!(show(&saved["retained_ref"]), "current detached source");
}

#[test]
fn fresh_checkout_and_needed_evidence_are_preserved_by_ordinary_use() {
    if !chrono_git_boundary::isolated_git() { return; }
    let h = Host::new("payload");
    policy(&h, 30, 40, "evidence-retain");
    native_consumer(&h, "ordinary");
    let target = h.parent.join("age-protected");
    start(&h, "age-protected", &target);
    fs::write(target.join("payload"), "fresh work").unwrap();
    let old = git(&target, &["rev-parse", "HEAD"]);
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
    assert!(used["age_reuse"].is_null());
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), old);
    fs::create_dir_all(target.join(".chrono-harness/state")).unwrap();
    fs::write(
        target.join(".chrono-harness/state/needed-original"),
        b"needed original\0\xff",
    )
    .unwrap();
    older_than(&target, 40);
    output(&target);
    let cache = cache_identity(&target);
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
    assert!(used["age_reuse"].is_null(), "{used}");
    assert!(target.join("output λ/cache").is_file());
    assert!(target.exists());
    assert_eq!(
        fs::read(target.join(".chrono-harness/state/needed-original")).unwrap(),
        b"needed original\0\xff"
    );
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert!(target.exists());
    assert_eq!(cache_identity(&target), cache);
    assert_eq!(
        fs::read(target.join(".chrono-harness/state/needed-original")).unwrap(),
        b"needed original\0\xff"
    );
    let observations = maintained["drain"].as_array().unwrap();
    assert!(
        observations
            .iter()
            .map(drain_report)
            .any(|r| !r["age_deletion_deferred"].is_null())
    );
    let (code, finished, error) = h.auto(
        "finish",
        &["--path", target.to_str().unwrap(), "--artifacts-only"],
    );
    assert_eq!(code, 0, "{finished} {error}");
    assert!(target.exists());
    assert!(!target.join("output λ").exists(), "{finished}");
    assert_eq!(
        fs::read(target.join(".chrono-harness/state/needed-original")).unwrap(),
        b"needed original\0\xff"
    );
}

#[test]
fn interrupted_age_disposal_reentry_preserves_original_unknown_result() {
    if !chrono_git_boundary::isolated_git() { return; }
    use std::os::unix::process::ExitStatusExt;
    let h = Host::new("payload");
    policy(&h, 1, 3, "dispose");
    native_git(&h, "age-interrupt", value!({}));
    let target = h.parent.join("interrupted-age");
    start(&h, "interrupted-age", &target);
    fs::write(target.join("payload"), "source survives interruption").unwrap();
    output(&target);
    older_than(&target, 3);
    fs::write(h.parent.join("interrupt-age"), "interrupt original removal").unwrap();
    let original = h._dir.capture_bound(&mut h.auto_command("maintain", &[]));
    assert_eq!(original.status.signal(), Some(9), "{original:?}");
    h.received(original);
    let pending = entry(&h, &target);
    assert_eq!(pending["status"], "terminal");
    assert!(pending["attempts"][0]["receipt"].is_null());
    let intent = pending["enrollment"]["age_retirement"].clone();
    let intent_bytes = fs::read(h.root.join(intent["path"].as_str().unwrap())).unwrap();
    fs::remove_file(h.parent.join("interrupt-age")).unwrap();
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(!target.exists());
    assert_eq!(
        fs::read(h.root.join(intent["path"].as_str().unwrap())).unwrap(),
        intent_bytes
    );
    let after = json(&fs::read(h.ledger_path(&target)).unwrap()).unwrap();
    assert!(after["entries"][0]["attempts"][0]["receipt"].is_null());
    assert_eq!(after["entries"][0]["attempts"][0], pending["attempts"][0]);
    assert_eq!(after["entries"][0]["status"], "disposed");
}
