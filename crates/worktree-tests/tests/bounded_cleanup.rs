#[path = "../../worktree-tests/tests/support/fixture_coordinator.rs"]
mod chrono_git_boundary;
use super::*;

fn main_host() -> Host {
    main_host_mode("rebuild-evidence")
}

#[test]
fn reusable_cache_survives_separate_operations_until_explicit_release() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = main_host();
    let path = h.root.join(".chrono-harness/cleanup.json");
    let mut policy = json(&fs::read(&path).unwrap()).unwrap();
    policy["artifacts"][0]["disposition"] = value!("finish-dispose");
    fs::write(path, serde_json::to_vec_pretty(&policy).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    for _ in 0..2 {
        let (code, used, error) = h.auto("use", &["--operation", "use.consumer"]);
        assert_eq!(code, 0, "{used} {error}");
        assert_eq!(fs::read(h.root.join("output λ/cache")).unwrap(), b"rebuilt");
        assert!(!h.root.join("nested/cache").exists());
        fs::create_dir_all(h.root.join("nested/cache")).unwrap();
        fs::write(h.root.join("nested/cache/temporary"), b"disposable").unwrap();
    }
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert!(h.root.join("output λ/cache").is_file());
    let e = h.ledger()["entries"][0].clone();
    let lease = chrono_harness::ownership::Lease::acquire(
        &h.root.join(e["ownership"]["path"].as_str().unwrap()),
        Some(e["ownership"]["id"].as_str().unwrap()),
        false,
        false,
        None,
    )
    .unwrap()
    .unwrap();
    assert_ne!(h.auto("finish", &[]).0, 0);
    assert!(h.root.join("output λ/cache").is_file());
    drop(lease);
    let outside = h.parent.join("outside-cache");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("keep"), b"external").unwrap();
    if let Err(error) = fs::remove_dir_all(h.root.join("nested/cache")) {
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    }
    fs::create_dir_all(h.root.join("nested")).unwrap();
    std::os::unix::fs::symlink(&outside, h.root.join("nested/cache")).unwrap();
    let (code, failed, _) = h.auto("finish", &[]);
    assert_ne!(code, 0, "{failed}");
    assert!(h.root.join("output λ/cache").is_file());
    assert_eq!(
        h.ledger()["entries"][0]["enrollment"]["cache_release"],
        failed["main_cache_release"]
    );
    let release = h
        .root
        .join(failed["main_cache_release"]["path"].as_str().unwrap());
    let original = fs::read(&release).unwrap();
    fs::remove_file(h.root.join("nested/cache")).unwrap();
    let (code, released, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{released} {error}");
    assert!(h.ledger()["entries"][0]["enrollment"]["cache_release"].is_null());
    assert_eq!(fs::read(release).unwrap(), original);
    assert_eq!(fs::read(outside.join("keep")).unwrap(), b"external");
    assert!(!h.root.join("output λ").exists());
    assert!(!h.root.join("nested/cache").exists());
    assert!(h.root.join("payload").is_file());
    assert!(h.root.join(".chrono-harness/state/evidence").is_file());
    assert!(h.ledger()["entries"][0]["terminal"].is_null());
    let (code, reused, error) = h.auto("use", &["--operation", "use.consumer"]);
    assert_eq!(code, 0, "{reused} {error}");
    assert!(
        h.root.join("output λ/cache").is_file(),
        "an old finish cannot release a new generation"
    );
    assert_eq!(h.auto("finish", &[]).0, 0);
    assert!(!h.root.join("output λ").exists());
}

#[test]
fn linked_finish_reclaims_reusable_outputs_while_preserving_needed_evidence() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.kernel_cleanup();
    super::automatic::native_consumer(&h, "rebuild-evidence");
    let path = h.root.join(".chrono-harness/cleanup.json");
    let mut policy = json(&fs::read(&path).unwrap()).unwrap();
    policy["artifacts"][0]["disposition"] = value!("finish-dispose");
    fs::write(path, serde_json::to_vec_pretty(&policy).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("reusable-linked");
    assert_eq!(h.invoke("integration", "reusable-linked", &target).0, 0);
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
    assert!(target.join("output λ/cache").is_file());
    let (code, finished, error) = h.auto(
        "finish",
        &["--path", target.to_str().unwrap(), "--artifacts-only"],
    );
    assert_eq!(code, 0, "{finished} {error}");
    assert!(!target.join("output λ").exists());
    assert!(target.join("payload").is_file());
    assert!(target.join(".chrono-harness/state/evidence").is_file());
}

fn cache_history_host(keep_seconds: u64, max_entries: usize) -> Host {
    let h = main_host();
    adopt_lifecycle_retention(&h);
    let path = h.root.join(".chrono-harness/cleanup.json");
    let mut policy = json(&fs::read(&path).unwrap()).unwrap();
    policy["cache_history"] = value!({
        "keep_seconds":keep_seconds,"max_entries":max_entries,"max_bytes":1048576
    });
    fs::write(path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let path = h.root.join(".chrono-harness/retention.json");
    let mut policy = json(&fs::read(&path).unwrap()).unwrap();
    policy["producers"]["lifecycle-original"] = value!([
        ".chrono-harness/state/worktrees/",
        ".chrono-harness/state/automatic-cleanup/"
    ]);
    policy["diagnostic_keep_seconds"] = value!(0);
    fs::write(path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    h
}

#[test]
fn cache_history_releases_old_generations_and_their_owned_originals() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    if !super::isolated_capability::isolated_capability_test(
        "bounded_cleanup::cache_history_releases_old_generations_and_their_owned_originals",
    ) {
        return;
    }
    use chrono_harness::retained_artifacts::retention::Inventory;
    let h = cache_history_host(0, 2);
    let index = fs::read(h.root.join(".git/index")).unwrap();
    let refs = git(&h.root, &["show-ref"]);
    let inventory = || Inventory::adopted(&h.root).unwrap().unwrap();
    let mut old = None;
    for round in 0..3 {
        let (code, used, error) = h.auto("use", &["--operation", "use.consumer"]);
        assert_eq!(code, 0, "{used} {error}");
        if round == 1 {
            let entry = h.ledger()["entries"][0].clone();
            let attempt = entry["cache_attempts"][0].clone();
            assert!(attempt["completed_at"].as_u64().is_some());
            assert!(
                h.root
                    .join(attempt["intent"]["path"].as_str().unwrap())
                    .is_file()
            );
            inventory()
                .reference_paths(
                    "external-cache-diagnostic",
                    &[attempt["path"].as_str().unwrap().into()],
                )
                .unwrap();
            old = Some(attempt);
        }
    }
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    let entry = h.ledger()["entries"][0].clone();
    let attempts = entry["cache_attempts"].as_array().unwrap();
    assert_eq!(attempts.len(), 1, "{entry}");
    assert_eq!(attempts[0]["generation"], entry["ownership"]["generation"]);
    assert!(attempts[0]["receipt"].is_object());
    assert!(entry["terminal"].is_null());
    let old = old.unwrap();
    let state = retention_state::snapshot(&h.root);
    let ids: Vec<_> = [&old["intent"]["path"], &old["path"]]
        .into_iter()
        .map(|path| {
            state["outputs"]
                .as_array()
                .unwrap()
                .iter()
                .find(|o| o["path"] == *path)
                .unwrap()["id"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    inventory().maintain_originals(&ids).unwrap();
    for path in [&old["intent"]["path"], &old["path"]] {
        assert!(
            h.root.join(path.as_str().unwrap()).is_file(),
            "external report retains its declared closure"
        );
    }
    inventory()
        .reference_paths("external-cache-diagnostic", &[])
        .unwrap();
    // Releasing the last consumer makes these originals eligible; one bounded
    // maintenance round does not promise to finish their physical disposal.
    let mut rounds = Vec::new();
    for _ in 0..8 {
        let round = inventory().maintain_originals(&ids).unwrap();
        for error in round["accounting_errors"].as_array().unwrap() {
            let reason = error["reason"].as_str().unwrap();
            let busy_metadata = reason
                .strip_prefix("retention output metadata is busy: ")
                .is_some_and(|id| ids.iter().any(|expected| expected == id));
            let busy_path = reason
                .strip_prefix("retention path is busy: ")
                .is_some_and(|path| {
                    [&old["intent"]["path"], &old["path"]]
                        .iter()
                        .any(|expected| expected.as_str() == Some(path))
                });
            assert!(busy_metadata || busy_path, "{round}");
        }
        rounds.push(round);
        if [&old["intent"]["path"], &old["path"]]
            .iter()
            .all(|path| !h.root.join(path.as_str().unwrap()).exists())
        {
            break;
        }
    }
    assert!(
        !h.root
            .join(old["intent"]["path"].as_str().unwrap())
            .exists(),
        "old cache intent remains after bounded maintenance: {rounds:?}"
    );
    assert!(
        !h.root.join(old["path"].as_str().unwrap()).exists(),
        "old cache report remains after bounded maintenance: {rounds:?}"
    );
    assert!(
        h.root
            .join(attempts[0]["intent"]["path"].as_str().unwrap())
            .exists()
    );
    assert_eq!(fs::read(h.root.join(".git/index")).unwrap(), index);
    assert_eq!(git(&h.root, &["show-ref"]), refs);
    assert!(h.root.join("payload").is_file());
}

#[test]
fn cache_history_preserves_unknown_time_live_consumers_and_changed_originals() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = cache_history_host(86400, 64);
    for _ in 0..3 {
        let (code, used, error) = h.auto("use", &["--operation", "use.consumer"]);
        assert_eq!(code, 0, "{used} {error}");
    }
    assert_eq!(h.auto("maintain", &[]).0, 0);
    let mut ledger = h.ledger();
    assert_eq!(
        ledger["entries"][0]["cache_attempts"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    // Capacity pressure cannot retire an unknown completion time or bypass a
    // live capability. These are explicit fault inputs, never inferred mtimes.
    ledger["entries"][0]["cache_attempts"][0]["completed_at"] = Value::Null;
    h.set_ledger(&ledger);
    let path = h.root.join(".chrono-harness/cleanup.json");
    let mut policy = json(&fs::read(&path).unwrap()).unwrap();
    policy["cache_history"]["max_entries"] = value!(1);
    fs::write(path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
    let (code, migrated, error) = h.auto("migrate", &["--path", h.root.to_str().unwrap()]);
    assert_eq!(code, 0, "{migrated} {error}");
    let entry = h.ledger()["entries"][0].clone();
    let lease = chrono_harness::ownership::Lease::acquire(
        &h.root.join(entry["ownership"]["path"].as_str().unwrap()),
        Some(entry["ownership"]["id"].as_str().unwrap()),
        false,
        false,
        None,
    )
    .unwrap()
    .unwrap();
    let (_, _, _) = h.auto("maintain", &[]);
    assert_eq!(
        h.ledger()["entries"][0]["cache_attempts"],
        entry["cache_attempts"]
    );
    drop(lease);
    let result_path = h.root.join(
        entry["cache_attempts"][1]["receipt"]["path"]
            .as_str()
            .unwrap(),
    );
    let original = fs::read(&result_path).unwrap();
    fs::write(&result_path, b"changed cache cleanup original").unwrap();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_ne!(code, 0, "{maintained} {error}");
    let after_failure = h.ledger()["entries"][0].clone();
    let protected = after_failure["cache_attempts"].as_array().unwrap();
    assert_eq!(protected.len(), 3, "{maintained}");
    assert_eq!(protected[0], entry["cache_attempts"][0]);
    assert_eq!(protected[1], entry["cache_attempts"][1]);
    // Migration can open a new current generation. Independent completed
    // history may retire while the unknown and changed originals stay rooted.
    assert_eq!(
        protected[2]["generation"],
        after_failure["ownership"]["generation"]
    );
    assert!(protected[2]["receipt"].is_object());
    fs::write(&result_path, original).unwrap();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    let remaining = h.ledger()["entries"][0]["cache_attempts"].clone();
    assert_eq!(remaining.as_array().unwrap().len(), 2);
    assert_eq!(remaining[0], entry["cache_attempts"][0]);
    assert_eq!(remaining[1], protected[2]);
}

pub(super) fn main_host_mode(mode: &str) -> Host {
    configure_main(Host::new("payload"), mode)
}
pub(super) fn configure_main(h: Host, mode: &str) -> Host {
    h.kernel_cleanup();
    super::automatic::native_consumer(&h, mode);
    let path = h.root.join(".chrono-harness/cleanup.json");
    let mut policy = json(&fs::read(&path).unwrap()).unwrap();
    policy["main_cache_only"] = value!(true);
    fs::write(path, serde_json::to_vec_pretty(&policy).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    h
}

const CARGO_FILES: &[(&str, &[u8])] = &[
    (
        "producer/Cargo.toml",
        include_bytes!("support/cargo_pair/producer/Cargo.toml"),
    ),
    (
        "producer/Cargo.lock",
        include_bytes!("support/cargo_pair/producer/Cargo.lock"),
    ),
    (
        "producer/src/lib.rs",
        include_bytes!("support/cargo_pair/producer/src/lib.rs"),
    ),
    (
        "tests/Cargo.toml",
        include_bytes!("support/cargo_pair/tests/Cargo.toml"),
    ),
    (
        "tests/Cargo.lock",
        include_bytes!("support/cargo_pair/tests/Cargo.lock"),
    ),
    (
        "tests/src/lib.rs",
        include_bytes!("support/cargo_pair/tests/src/lib.rs"),
    ),
];

fn cargo_host() -> Host {
    let h = configure_main(Host::retained("payload"), "bounded-cargo");
    let mut cfg = json(&fs::read(h.root.join(CONFIG)).unwrap()).unwrap();
    let mut policy = json(&fs::read(h.root.join(".chrono-harness/cleanup.json")).unwrap()).unwrap();
    let mut map = json(&fs::read(h.root.join(FM)).unwrap()).unwrap();
    for (path, bytes) in CARGO_FILES {
        let path = format!("independent/{path}");
        fs::create_dir_all(h.root.join(&path).parent().unwrap()).unwrap();
        fs::write(h.root.join(&path), bytes).unwrap();
        map["files"]
            .as_array_mut()
            .unwrap()
            .push(file(&path, value!([])));
    }
    for project in ["producer", "tests"] {
        let path = format!("independent/{project}/target/");
        cfg["artifacts"]
            .as_array_mut()
            .unwrap()
            .push(value!({"path":path,"owner":"host","kind":"generated","tracked":false}));
        policy["artifacts"]
            .as_array_mut()
            .unwrap()
            .push(value!({"path":path,"disposition":"dispose"}));
    }
    fs::write(
        h.root.join(CONFIG),
        serde_json::to_vec_pretty(&cfg).unwrap(),
    )
    .unwrap();
    fs::write(h.root.join(FM), serde_json::to_vec_pretty(&map).unwrap()).unwrap();
    fs::write(
        h.root.join(".chrono-harness/cleanup.json"),
        serde_json::to_vec_pretty(&policy).unwrap(),
    )
    .unwrap();
    fs::write(h.root.join(".gitignore"), ".chrono-harness/state/\n.chrono-harness/bin/\noutput λ/\nnested/cache/\nindependent/producer/target/\nindependent/tests/target/\n").unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    h
}

fn cargo_plan(root: &Path, hold: Option<&str>) {
    fs::create_dir_all(root.join(".chrono-harness/state")).unwrap();
    fs::write(root.join(".chrono-harness/state/cargo-plan.json"), serde_json::to_vec(&value!({
        "manifests":["independent/producer/Cargo.toml","independent/tests/Cargo.toml"],"operations":["build","test"],"hold_stage":hold
    })).unwrap()).unwrap();
}

fn cost(root: &Path) -> Value {
    json(&fs::read(root.join(".chrono-harness/state/cargo-cost.json")).unwrap()).unwrap()
}

#[test]
fn independent_cargo_main_and_linked_cleanup_and_rebuild_measurements() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    for linked in [false, true] {
        let h = cargo_host();
        let target = if linked {
            let target = h.parent.join("cargo-linked");
            assert_eq!(h.invoke("feature", "cargo-linked", &target).0, 0);
            target
        } else {
            h.root.clone()
        };
        cargo_plan(&target, Some("retained-rebuild"));
        let plan_path = target.join(".chrono-harness/state/cargo-plan.json");
        let mut plan = json(&fs::read(&plan_path).unwrap()).unwrap();
        plan["passes"] = value!(2);
        fs::write(plan_path, serde_json::to_vec(&plan).unwrap()).unwrap();
        let args = [
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ];
        let mut child = super::interrupted_cleanup::CapturedChild::spawn(
            &mut h.auto_command("use", &args),
            &h.root,
        );
        child.await_managed_file(&target.join(".chrono-harness/state/retained-rebuild-active"));
        let observations = cost(&target);
        let cold = value!(&observations.as_array().unwrap()[..2]);
        let retained = value!(&observations.as_array().unwrap()[2..]);
        let before: Vec<_> = ["producer", "tests"]
            .iter()
            .map(|p| {
                serde_json::to_value(
                    chrono_worktree::artifact_footprint(
                        &target.join(format!("independent/{p}/target")),
                    )
                    .unwrap(),
                )
                .unwrap()
            })
            .collect();
        assert!(
            before
                .iter()
                .all(|m| m["logical_bytes"].as_u64().unwrap() > 0)
        );
        // Retained rebuild uses the actual registered child, while this same use holds its lease.
        assert_eq!(h.auto("maintain", &[]).0, 0);
        assert!(target.join("independent/tests/target").exists());
        fs::write(
            target.join(".chrono-harness/state/retained-rebuild-release"),
            "joined",
        )
        .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "{output:?}");
        let began = std::time::Instant::now();
        let (code, cleaned, error) = h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--artifacts-only"],
        );
        assert_eq!(code, 0, "{} {error}", cleaned["error"]);
        let cleanup_seconds = began.elapsed().as_secs_f64();
        for project in ["producer", "tests"] {
            assert!(
                !target
                    .join(format!("independent/{project}/target"))
                    .exists()
            );
            assert!(
                target
                    .join(format!("independent/{project}/Cargo.toml"))
                    .is_file()
            );
            assert!(
                target
                    .join(format!("independent/{project}/Cargo.lock"))
                    .is_file()
            );
        }
        // Linked finish is a terminal handoff; use a separate active linked fixture for re-entry.
        if !linked {
            cargo_plan(&target, None);
            let (code, rebuilt, error) = h.auto("use", &args);
            assert_eq!(code, 0, "{} {error}", rebuilt["error"]);
            let measurement = value!({"checkout":"main","unit":"seconds and bytes","method":"Rust Cargo command mock; real compiler cost unknown","cold":cold,
                "retained":retained,"cold_after_cleanup":cost(&target),"cleanup_seconds":cleanup_seconds,
                "before":before,"eligible_objects":2,"released_objects":2,"remaining_objects":0,
                "physical_release_bytes":null,"physical_release_reason":"allocation observations do not attribute shared filesystem blocks"});
            println!("BOUNDED_MOCK_COST {measurement}");
        } else {
            println!(
                "BOUNDED_MOCK_COST {}",
                value!({"checkout":"linked","method":"Rust Cargo command mock; real compiler cost unknown","cold":cold,"retained":retained,
                "cleanup_seconds":cleanup_seconds,"before":before,"eligible_objects":2,
                "released_objects":2,"remaining_objects":0,"physical_release_bytes":null})
            );
        }
    }
}

fn await_file(path: &Path) {
    let began = std::time::Instant::now();
    while !path.exists() && began.elapsed() < std::time::Duration::from_secs(10) {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        path.exists(),
        "missing actual consumer handshake: {}",
        path.display()
    );
}

#[test]
fn managed_cargo_handshake_consumes_the_actual_failed_producer_and_joins_it() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = cargo_host();
    cargo_plan(&h.root, Some("retained-rebuild"));
    let path = h.root.join(".chrono-harness/state/cargo-plan.json");
    let mut plan = json(&fs::read(&path).unwrap()).unwrap();
    plan["manifests"][0] = value!("missing/Cargo.toml");
    fs::write(path, serde_json::to_vec(&plan).unwrap()).unwrap();
    let mut child = super::interrupted_cleanup::CapturedChild::spawn(
        &mut h.auto_command("use", &["--operation", "use.consumer"]),
        &h.root,
    );
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        child.await_managed_file(&h.root.join(".chrono-harness/state/retained-rebuild-active"));
    }));
    assert!(failure.is_err());
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    let report = json(&output.stdout).unwrap();
    assert_ne!(report["managed_process"]["exit_code"], 0);
    assert!(
        report["managed_process"]["stderr"]
            .as_str()
            .unwrap()
            .contains("missing/Cargo.toml")
    );
    assert!(
        !h.root
            .join(".chrono-harness/state/retained-rebuild-active")
            .exists()
    );
    assert!(
        h.root
            .join(report["managed_use_receipt"]["path"].as_str().unwrap())
            .is_file()
    );
}

#[test]
fn main_live_installation_test_and_cache_save_consumers_protect_real_cargo_outputs() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    for stage in ["installation", "test", "cache-save"] {
        let h = cargo_host();
        cargo_plan(&h.root, Some(stage));
        let mut command = h.auto_command("use", &["--operation", "use.consumer"]);
        let mut child = super::interrupted_cleanup::CapturedChild::spawn(&mut command, &h.root);
        child.await_managed_file(&h.root.join(format!(".chrono-harness/state/{stage}-active")));
        let (code, maintained, error) = h.auto("maintain", &[]);
        assert_eq!(code, 0, "{maintained} {error}");
        assert!(maintained.to_string().contains("live registered consumers"));
        assert!(h.root.join("independent/tests/target").is_dir());
        assert_ne!(h.auto("finish", &[]).0, 0);
        fs::write(
            h.root
                .join(format!(".chrono-harness/state/{stage}-release")),
            "joined",
        )
        .unwrap();
        assert!(child.wait_with_output().unwrap().status.success());
        assert_eq!(h.auto("finish", &[]).0, 0);
        assert!(!h.root.join("independent/tests/target").exists());
        if stage == "installation" {
            assert!(
                h.root
                    .join(".chrono-harness/bin/installed-cargo-output")
                    .exists()
            );
        }
    }
}

#[test]
fn main_same_entry_recovers_interruption_and_protects_live_descendant_and_second_consumer() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = main_host_mode("parent");
    let mut first = super::interrupted_cleanup::CapturedChild::spawn(
        &mut h.auto_command("use", &["--operation", "use.consumer"]),
        &h.root,
    );
    // Observe the managed producer under its registered bounds, including Git preparation.
    first.await_managed_file(&h.root.join(".chrono-harness/state/grandchild"));
    fs::create_dir_all(h.root.join("output λ")).unwrap();
    fs::write(
        h.root.join("output λ/cache"),
        "registered producer output under the active lease",
    )
    .unwrap();
    let second = super::interrupted_cleanup::CapturedChild::spawn(
        &mut h.auto_command("use", &["--operation", "use.consumer"]),
        &h.root,
    );
    let began = std::time::Instant::now();
    while h.ledger()["entries"][0]["uses"].as_array().unwrap().len() < 2 {
        assert!(began.elapsed() < std::time::Duration::from_secs(10));
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    first.kill().unwrap();
    first.wait_with_output().unwrap();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert!(h.root.join("output λ/cache").exists());
    fs::write(
        h.root.join(".chrono-harness/state/release-parent"),
        "joined",
    )
    .unwrap();
    assert!(second.wait_with_output().unwrap().status.success());
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(
        h.root.join("output λ/cache").exists(),
        "live grandchildren keep the original lease"
    );
    fs::write(
        h.root.join(".chrono-harness/state/release-grandchild"),
        "joined",
    )
    .unwrap();
    // Ordinary use retries until the actual final descriptor closes; no PID/age decision.
    let began = std::time::Instant::now();
    loop {
        let (code, used, error) = h.auto("use", &["--operation", "test.consumer"]);
        assert_eq!(code, 0, "{used} {error}");
        if !h.root.join("output λ").exists() {
            break;
        }
        assert!(began.elapsed() < std::time::Duration::from_secs(10));
    }
    assert!(h.ledger()["entries"][0]["enrollment"]["interrupted_uses"].is_array());
    assert!(h.ledger()["entries"][0]["terminal"].is_null());
}

#[test]
fn main_partial_disposal_retry_keeps_business_and_cleanup_errors_and_remaining_objects() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use std::os::unix::fs::PermissionsExt;
    let h = main_host_mode("rebuild-error");
    let (code, failed, error) = h.auto("use", &["--operation", "use.consumer"]);
    assert_ne!(code, 0, "{failed} {error}");
    assert_eq!(failed["managed_process"]["exit_code"], 23);
    assert_eq!(failed["managed_process"]["stderr"], "original-error");
    let original = fs::read(h.root.join(failed["report_path"].as_str().unwrap())).unwrap();
    let protected = h.root.join("nested/cache");
    fs::create_dir_all(&protected).unwrap();
    fs::write(protected.join("remaining"), "partial").unwrap();
    fs::set_permissions(&protected, fs::Permissions::from_mode(0o500)).unwrap();
    let (code, partial, error) = h.auto("maintain", &[]);
    fs::set_permissions(&protected, fs::Permissions::from_mode(0o700)).unwrap();
    assert_ne!(code, 0, "{partial} {error}");
    assert!(!h.root.join("output λ").exists());
    assert!(protected.join("remaining").exists());
    assert!(
        drain_report(&partial["drain"][0])
            .to_string()
            .contains("failed-partial-effects-possible")
    );
    let (code, retried, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{retried} {error}");
    assert!(
        drain_report(&retried["drain"][0])
            .to_string()
            .contains("already-absent")
    );
    assert!(!protected.exists());
    assert_eq!(
        fs::read(h.root.join(failed["report_path"].as_str().unwrap())).unwrap(),
        original
    );
    assert_eq!(h.auto("maintain", &[]).1["drain"], value!([]));
}

#[test]
fn explicit_main_import_releases_unproduced_outputs_without_claiming_execution() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = main_host();
    let index = fs::read(h.root.join(".git/index")).unwrap();
    let refs = git(&h.root, &["show-ref"]);
    let (code, imported, error) = h.auto("import", &[]);
    assert_eq!(code, 0, "{imported} {error}");
    let entry = h.ledger()["entries"][0].clone();
    assert_eq!(entry["cache_only"], true);
    assert_eq!(entry["enrollment"]["kind"], "main-cache");
    assert_eq!(entry["uses"], value!([]));
    assert_eq!(imported["managed_process"], Value::Null);
    let (code, released, error) = h.auto("finish", &[]);
    assert_eq!(code, 0, "{released} {error}");
    let descriptor = &released["main_cache_release"];
    let raw = fs::read(h.root.join(descriptor["path"].as_str().unwrap())).unwrap();
    assert_eq!(sha256(&raw), descriptor["sha256"]);
    let release = json(&raw).unwrap();
    assert_eq!(release["path"], value!(h.root));
    assert_eq!(release["task_completion"], "not-claimed");
    assert_eq!(released["drain"].as_array().unwrap().len(), 1);
    assert!(!h.root.join("output λ").exists());
    assert_eq!(fs::read(h.root.join(".git/index")).unwrap(), index);
    assert_eq!(git(&h.root, &["show-ref"]), refs);
    assert_eq!(h.ledger()["entries"][0]["terminal"], Value::Null);
    // A policy that did not opt in still admits only linked worktrees.
    let ordinary = Host::new("payload");
    ordinary.kernel_cleanup();
    let (code, report, error) = ordinary.auto("import", &[]);
    assert_ne!(code, 0, "{report} {error}");
    assert!(
        report
            .to_string()
            .contains("only an existing linked worktree can enroll")
    );
}

#[test]
fn main_cache_finish_preserves_source_index_refs_bin_and_evidence() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = main_host();
    let (code, used, error) = h.auto("use", &["--operation", "use.consumer"]);
    assert_eq!(code, 0, "{used} {error}");
    fs::write(h.root.join("payload"), "dirty staged source").unwrap();
    git(&h.root, &["add", "payload"]);
    let index = fs::read(h.root.join(".git/index")).unwrap();
    let refs = git(&h.root, &["show-ref"]);
    let head = git(&h.root, &["rev-parse", "HEAD"]);
    let tree = git(&h.root, &["rev-parse", "HEAD^{tree}"]);
    let unretained = git(
        &h.root,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit-tree",
            tree.trim(),
            "-p",
            head.trim(),
            "-m",
            "unretained fixture commit",
        ],
    );
    fs::create_dir_all(h.root.join(".chrono-harness/state/recovery")).unwrap();
    fs::write(
        h.root.join(".chrono-harness/state/recovery/original"),
        "required recovery input",
    )
    .unwrap();
    let (code, finished, error) = h.auto("finish", &[]);
    assert_eq!(code, 0, "{finished} {error}");
    assert!(!h.root.join("output λ").exists());
    assert_eq!(fs::read(h.root.join(".git/index")).unwrap(), index);
    assert_eq!(git(&h.root, &["show-ref"]), refs);
    git(&h.root, &["cat-file", "-e", unretained.trim()]);
    assert!(
        h.root
            .join(".chrono-harness/state/recovery/original")
            .exists()
    );
    assert_eq!(
        fs::read_to_string(h.root.join("payload")).unwrap(),
        "dirty staged source"
    );
    assert_eq!(
        fs::read_to_string(h.root.join(".chrono-harness/state/evidence")).unwrap(),
        "evidence"
    );
    assert!(h.ledger()["entries"][0]["terminal"].is_null());
    assert_eq!(h.ledger()["entries"][0]["status"], "active");
    assert_eq!(h.auto("maintain", &[]).1["drain"], value!([]));
}

#[test]
fn main_first_enrollment_preserves_unknown_historical_registered_outputs() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = main_host();
    fs::create_dir_all(h.root.join("nested/cache")).unwrap();
    fs::write(
        h.root.join("nested/cache/unknown-history"),
        "no consumer has published release",
    )
    .unwrap();
    assert_eq!(h.auto("use", &["--operation", "use.consumer"]).0, 0);
    let (code, finished, error) = h.auto("finish", &[]);
    assert_eq!(code, 0, "{finished} {error}");
    assert!(!h.root.join("output λ").exists());
    assert!(h.root.join("nested/cache/unknown-history").exists());
    assert!(
        drain_report(&finished["drain"][0])
            .to_string()
            .contains("pre-existing output has no adopted consumer release")
    );
}

#[test]
fn missing_main_artifact_ownership_fails_closed_instead_of_reporting_empty_cleanup() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = main_host();
    assert_eq!(h.auto("use", &["--operation", "use.consumer"]).0, 0);
    let ledger_path = h.ledger_path(&h.root);
    let original = fs::read(&ledger_path).unwrap();
    let mut ledger = json(&original).unwrap();
    ledger["entries"][0]["enrollment"]
        .as_object_mut()
        .unwrap()
        .remove("main_cache_artifacts");
    fs::write(&ledger_path, serde_json::to_vec(&ledger).unwrap()).unwrap();
    let (code, failed, error) = h.auto("maintain", &[]);
    assert_ne!(code, 0, "{failed} {error}");
    assert!(
        failed
            .to_string()
            .contains("main cache artifact declarations missing")
    );
    assert!(h.root.join("output λ/cache").exists());
    fs::write(ledger_path, original).unwrap();
    assert_eq!(h.auto("finish", &[]).0, 0);
}

#[test]
fn main_cache_ownership_protects_refs_across_branch_change_and_detached_head() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = main_host();
    assert_eq!(h.auto("use", &["--operation", "use.consumer"]).0, 0);
    git(
        &h.root,
        &["checkout", "-qb", "integration/main-cache-switch"],
    );
    let (code, used, error) = h.auto("use", &["--operation", "use.consumer"]);
    assert_eq!(code, 0, "{used} {error}");
    git(&h.root, &["checkout", "-q", "--detach"]);
    let refs = git(&h.root, &["show-ref"]);
    let (code, finished, error) = h.auto("finish", &[]);
    assert_eq!(code, 0, "{finished} {error}");
    assert_eq!(git(&h.root, &["show-ref"]), refs);
    assert!(!h.root.join("output λ").exists());
    assert!(h.ledger()["entries"][0]["terminal"].is_null());
    let (code, used, error) = h.auto("use", &["--operation", "use.consumer"]);
    assert_eq!(code, 0, "{used} {error}");
}

fn historical_host() -> Host {
    let h = main_host();
    for path in ["output λ", "nested/cache"] {
        fs::create_dir_all(h.root.join(path)).unwrap();
        fs::write(h.root.join(path).join("old-output"), "rebuildable history").unwrap();
    }
    assert_eq!(h.auto("use", &["--operation", "use.consumer"]).0, 0);
    h
}

fn custody_plan(h: &Host, artifacts: &[&str]) -> String {
    let path = ".chrono-harness/state/current-custody.json";
    let plan = value!({"schema":"chrono-main-cache-custody/v1", "path":h.root,
        "head":git(&h.root, &["rev-parse", "HEAD"]).trim(), "artifacts":artifacts,
        "current_consumers_released":true,
        "reason":"Current fixture operator joined its consumers and assumes custody; past producer outcome is unknown"});
    fs::write(h.root.join(path), serde_json::to_vec_pretty(&plan).unwrap()).unwrap();
    path.into()
}

fn custody(h: &Host, plan: &str) -> (i32, Value, String) {
    h.auto(
        "migrate",
        &["--path", h.root.to_str().unwrap(), "--adopt-cache", plan],
    )
}

#[test]
fn main_historical_custody_reclaims_selected_outputs_and_preserves_original_enrollment() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = historical_host();
    let original = h.ledger()["entries"][0].clone();
    let plan = custody_plan(&h, &["nested/cache/"]);
    let (code, adopted, error) = custody(&h, &plan);
    assert_eq!(code, 0, "{adopted} {error}");
    assert_eq!(adopted["cache_adoption"]["status"], "adopted");
    assert!(h.root.join("nested/cache/old-output").exists());
    let entry = h.ledger()["entries"][0].clone();
    assert_eq!(entry["enrollment"], original["enrollment"]);
    assert_eq!(entry["status"], "active");
    assert!(entry["terminal"].is_null());
    let (code, cleaned, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{cleaned} {error}");
    assert!(!h.root.join("nested/cache").exists());
    assert!(
        h.root.join("output λ/old-output").exists(),
        "unselected historical output stays protected"
    );
    assert_eq!(
        fs::read_to_string(h.root.join("payload")).unwrap(),
        "literal host input; no dependency inference\n"
    );
    let (code, repeated, error) = custody(&h, &plan);
    assert_eq!(code, 0, "{repeated} {error}");
    assert_eq!(repeated["cache_adoption"]["status"], "already-managed");
    assert_eq!(
        h.ledger()["entries"][0]["main_cache_adoptions"],
        entry["main_cache_adoptions"]
    );
    assert_eq!(h.auto("maintain", &[]).1["drain"], value!([]));
}

#[test]
fn main_historical_custody_requires_exclusion_explicit_scope_and_source_protection() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = historical_host();
    let plan = custody_plan(&h, &["nested/cache/"]);
    let entry = h.ledger()["entries"][0].clone();
    let held = fs::File::open(h.root.join(entry["ownership"]["path"].as_str().unwrap())).unwrap();
    held.lock_shared().unwrap();
    assert_ne!(custody(&h, &plan).0, 0);
    drop(held);
    assert_eq!(h.ledger()["entries"][0], entry);
    for names in [
        vec!["payload/"],
        vec!["unregistered/"],
        vec![".chrono-harness/state/"],
        vec!["nested/cache/", "nested/cache/"],
    ] {
        let invalid = custody_plan(&h, &names);
        assert_ne!(custody(&h, &invalid).0, 0);
        assert!(h.root.join("nested/cache/old-output").exists());
    }
    git(&h.root, &["add", "-f", "nested/cache/old-output"]);
    let plan = custody_plan(&h, &["nested/cache/"]);
    let (code, refused, error) = custody(&h, &plan);
    assert_ne!(code, 0, "{refused} {error}");
    assert!(refused.to_string().contains("tracked paths"));
    assert!(h.root.join("nested/cache/old-output").exists());
    // An unresolved independent historical object does not veto eligible siblings.
    let plan = custody_plan(&h, &["output λ/"]);
    assert_eq!(custody(&h, &plan).0, 0);
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(!h.root.join("output λ").exists());
    assert!(h.root.join("nested/cache/old-output").exists());
}

#[test]
fn main_historical_custody_publication_retry_and_partial_disposal_preserve_originals() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use std::os::unix::fs::PermissionsExt;
    let h = historical_host();
    let plan = custody_plan(&h, &["output λ/", "nested/cache/"]);
    let ledger = h.ledger_path(&h.root);
    let prior = fs::read(&ledger).unwrap();
    let (code, first, error) = custody(&h, &plan);
    assert_eq!(code, 0, "{first} {error}");
    let receipt = &first["cache_adoption"]["receipt"];
    let path = h.root.join(receipt["path"].as_str().unwrap());
    let bytes = fs::read(&path).unwrap();
    // Durable receipt exists, but adoption's ledger publication was interrupted.
    fs::write(&ledger, prior).unwrap();
    let (code, retry, error) = custody(&h, &plan);
    assert_eq!(code, 0, "{retry} {error}");
    assert_eq!(retry["cache_adoption"]["receipt"], *receipt);
    assert_eq!(fs::read(&path).unwrap(), bytes);
    fs::set_permissions(
        h.root.join("nested/cache"),
        fs::Permissions::from_mode(0o500),
    )
    .unwrap();
    let (code, partial, error) = h.auto("maintain", &[]);
    fs::set_permissions(
        h.root.join("nested/cache"),
        fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    assert_ne!(code, 0, "{partial} {error}");
    assert!(!h.root.join("output λ").exists());
    let original_result = h
        .root
        .join(partial["drain"][0]["receipt"]["path"].as_str().unwrap());
    let original_result_bytes = fs::read(&original_result).unwrap();
    let (code, cleaned, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{cleaned} {error}");
    assert!(
        drain_report(&cleaned["drain"][0])
            .to_string()
            .contains("already-absent")
    );
    assert!(!h.root.join("nested/cache").exists());
    assert_eq!(fs::read(original_result).unwrap(), original_result_bytes);
    assert_eq!(fs::read(&path).unwrap(), bytes);
    // The same ordinary entry protects the next producer and later reclaims its outputs.
    assert_eq!(h.auto("use", &["--operation", "use.consumer"]).0, 0);
    assert!(h.root.join("output λ/cache").exists());
    assert_eq!(h.auto("use", &["--operation", "test.consumer"]).0, 0);
    assert!(!h.root.join("output λ").exists());
}
