use super::automatic::{native_consumer, native_git};
use super::*;
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

#[test]
fn real_creation_age_reuse_and_expiry_preserve_source_index_and_branches() {
    let h = Host::new("payload");
    policy(&h, 2, 60, "dispose");
    native_consumer(&h, "ordinary");
    let target = h.parent.join("real-aged-checkout");
    assert_eq!(h.invoke("feature", "real-aged-checkout", &target).0, 0);
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
    let saved = &used["age_reuse"]["source_preservation"];
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
    assert!(!target.join("output λ").exists());
    let reference = saved["retained_ref"].as_str().unwrap().to_owned();
    let head = saved["head"].as_str().unwrap().to_owned();
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
    let h = Host::new("payload");
    policy(&h, 2, 5, "dispose");
    let busy = h.parent.join("aged-busy");
    let free = h.parent.join("aged-free");
    assert_eq!(h.invoke("feature", "aged-busy", &busy).0, 0);
    assert_eq!(h.invoke("feature", "aged-free", &free).0, 0);
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
fn fresh_checkout_and_needed_evidence_are_preserved_by_ordinary_use() {
    let h = Host::new("payload");
    policy(&h, 30, 40, "evidence-retain");
    native_consumer(&h, "ordinary");
    let target = h.parent.join("age-protected");
    assert_eq!(h.invoke("feature", "age-protected", &target).0, 0);
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
    assert!(target.exists());
    assert_eq!(
        fs::read(target.join(".chrono-harness/state/needed-original")).unwrap(),
        b"needed original\0\xff"
    );
    let observations = used["post_use_cleanup"]["drain"].as_array().unwrap();
    assert!(
        observations
            .iter()
            .map(drain_report)
            .any(|r| !r["age_deletion_deferred"].is_null())
    );
}

#[test]
fn external_absence_adoption_preserves_failed_and_missing_original_outcomes() {
    external_absence_adoption(false);
}

#[test]
fn singleton_external_absence_adoption_preserves_original_observations() {
    external_absence_adoption(true);
}

fn external_absence_adoption(singleton: bool) {
    let h = Host::new("payload");
    h.kernel_cleanup();
    let target = h.parent.join("external-disposal");
    assert_eq!(h.invoke("feature", "external-disposal", &target).0, 0);
    let e = entry(&h, &target);
    let head = git(&target, &["rev-parse", "HEAD"]);
    let reference = "refs/heads/retained/external-disposal";
    git(&h.root, &["update-ref", reference, &head]);
    git(
        &h.root,
        &["worktree", "remove", "--force", target.to_str().unwrap()],
    );
    let (code, failed, _) = h.auto("maintain", &[]);
    assert_eq!(
        code, 2,
        "missing physical checkout must retain the original failure: {failed}"
    );
    let before = fs::read(h.ledger_path(&target)).unwrap();
    let dir = h.root.join(".chrono-harness/state/absence");
    fs::create_dir_all(&dir).unwrap();
    let row =
        value!({"path":target,"status":"verified-absent","head":head,"retained_ref":reference});
    let (results, registration) = if singleton {
        let mut fixture = json(include_bytes!("fixtures/external-expiry-singleton.json")).unwrap();
        fixture["result"] = row;
        fixture["registration"]["target"]["path"] = value!(target);
        fixture["registration"]["target"]["head"]["stdout"] = value!(format!("{head}\n"));
        fixture["registration"]["target"]["common"]["stdout"] =
            value!(format!("{}\n", h.root.join(".git").display()));
        (fixture["result"].clone(), fixture["registration"].clone())
    } else {
        (
            value!([row]),
            value!({"targets":[{"path":target,"head":head,"owner_git_common":h.root.join(".git")}]}),
        )
    };
    let write = |name: &str, v: &Value| {
        let bytes = serde_json::to_vec(v).unwrap();
        fs::write(dir.join(name), &bytes).unwrap();
        value!({"path":format!(".chrono-harness/state/absence/{name}"),"sha256":sha256(&bytes)})
    };
    let original = write("original.json", &results);
    let registered = write("registration.json", &registration);
    let binding = value!({"path":e["path"],"branch":e["branch"],"attachment":e["attachment"],"policy_sha256":e["policy_sha256"],"observed_head":e["enrollment"]["observed_head"],"policy_inputs":e["enrollment"]["policy_inputs"],"original_receipt":e["enrollment"]["receipt"],"ownership":{"path":e["ownership"]["path"],"id":e["ownership"]["id"]}});
    let plan = value!({"schema":"chrono-worktree-absent-adoption/v1","binding":binding,"original_result":original,"result_index":0,"original_registration":registered});
    write("plan.json", &plan);
    let invoke_adoption = || {
        h.auto(
            "migrate",
            &[
                "--path",
                target.to_str().unwrap(),
                "--adopt-absence",
                ".chrono-harness/state/absence/plan.json",
            ],
        )
    };
    let original_bytes = fs::read(dir.join("original.json")).unwrap();
    fs::write(dir.join("original.json"), b"changed original").unwrap();
    assert_eq!(
        invoke_adoption().0,
        2,
        "changed original must refuse adoption"
    );
    fs::write(dir.join("original.json"), &original_bytes).unwrap();
    fs::create_dir(&target).unwrap();
    assert_eq!(
        invoke_adoption().0,
        2,
        "recreated path must remain protected"
    );
    fs::remove_dir(&target).unwrap();
    let lease = chrono_harness::ownership::Lease::acquire(
        &h.root.join(e["ownership"]["path"].as_str().unwrap()),
        Some(e["ownership"]["id"].as_str().unwrap()),
        false,
        false,
        None,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        invoke_adoption().0,
        2,
        "live consumer of missing path must remain protected"
    );
    drop(lease);
    assert_eq!(fs::read(h.ledger_path(&target)).unwrap(), before);
    if singleton {
        let mut changed_plan = plan.clone();
        changed_plan["result_index"] = value!(1);
        write("plan.json", &changed_plan);
        assert_eq!(invoke_adoption().0, 2, "singleton has exactly one result");
        write("plan.json", &plan);
        let mut changed_registration = registration.clone();
        changed_registration["target"]["head"]["exit"] = value!(1);
        let changed_original = write("registration.json", &changed_registration);
        changed_plan = plan.clone();
        changed_plan["original_registration"] = changed_original;
        write("plan.json", &changed_plan);
        assert_eq!(
            invoke_adoption().0,
            2,
            "failed original observation cannot bind source custody"
        );
        write("registration.json", &registration);
        write("plan.json", &plan);
        assert_eq!(fs::read(h.ledger_path(&target)).unwrap(), before);
    }
    let (code, adopted, error) = h.auto(
        "migrate",
        &[
            "--path",
            target.to_str().unwrap(),
            "--adopt-absence",
            ".chrono-harness/state/absence/plan.json",
        ],
    );
    assert_eq!(code, 0, "{adopted} {error}");
    assert_eq!(adopted["worktree_removal"], "already-absent");
    assert_eq!(adopted["worktree_removed"], false);
    let after = json(&fs::read(h.ledger_path(&target)).unwrap()).unwrap();
    assert_eq!(after["entries"][0]["status"], "disposed");
    assert_eq!(
        after["entries"][0]["attempts"],
        json(&before).unwrap()["entries"][0]["attempts"]
    );
    assert_eq!(
        after["entries"][0]["cache_attempts"],
        json(&before).unwrap()["entries"][0]["cache_attempts"]
            .as_array()
            .map(|a| value!(a))
            .unwrap_or(value!(null))
    );
    assert_eq!(git(&h.root, &["rev-parse", reference]), head);
    assert_eq!(h.auto("maintain", &[]).0, 0);
}

#[test]
fn interrupted_age_disposal_reentry_preserves_original_unknown_result() {
    use std::os::unix::process::ExitStatusExt;
    let h = Host::new("payload");
    policy(&h, 1, 3, "dispose");
    native_git(&h, "age-interrupt", value!({}));
    let target = h.parent.join("interrupted-age");
    assert_eq!(h.invoke("feature", "interrupted-age", &target).0, 0);
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
