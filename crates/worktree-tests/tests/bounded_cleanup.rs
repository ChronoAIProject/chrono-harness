use super::*;

fn main_host() -> Host {
    main_host_mode("rebuild-evidence")
}

fn main_host_mode(mode: &str) -> Host {
    let h = Host::new("payload");
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
    let h = main_host_mode("bounded-cargo");
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
        let child = super::interrupted_cleanup::CapturedChild::spawn(
            &mut h.auto_command("use", &args),
            &h.root,
        );
        await_file(&target.join(".chrono-harness/state/retained-rebuild-active"));
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
            let measurement = value!({"checkout":"main","unit":"seconds and bytes","cold":cold,
                "retained":retained,"cold_after_cleanup":cost(&target),"cleanup_seconds":cleanup_seconds,
                "before":before,"eligible_objects":2,"released_objects":2,"remaining_objects":0,
                "physical_release_bytes":null,"physical_release_reason":"allocation observations do not attribute shared filesystem blocks"});
            println!("BOUNDED_COST {measurement}");
        } else {
            println!(
                "BOUNDED_COST {}",
                value!({"checkout":"linked","cold":cold,"retained":retained,
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
fn main_live_installation_test_and_cache_save_consumers_protect_real_cargo_outputs() {
    for stage in ["installation", "test", "cache-save"] {
        let h = cargo_host();
        cargo_plan(&h.root, Some(stage));
        let mut command = h.auto_command("use", &["--operation", "use.consumer"]);
        let child = super::interrupted_cleanup::CapturedChild::spawn(&mut command, &h.root);
        await_file(&h.root.join(format!(".chrono-harness/state/{stage}-active")));
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
    let h = main_host_mode("parent");
    let mut first = super::interrupted_cleanup::CapturedChild::spawn(
        &mut h.auto_command("use", &["--operation", "use.consumer"]),
        &h.root,
    );
    await_file(&h.root.join(".chrono-harness/state/grandchild"));
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
        partial
            .to_string()
            .contains("failed-partial-effects-possible")
    );
    let (code, retried, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{retried} {error}");
    assert!(retried.to_string().contains("already-absent"));
    assert!(!protected.exists());
    assert_eq!(
        fs::read(h.root.join(failed["report_path"].as_str().unwrap())).unwrap(),
        original
    );
    assert_eq!(h.auto("maintain", &[]).1["drain"], value!([]));
}

#[test]
fn main_cache_finish_preserves_source_index_refs_bin_and_evidence() {
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
        finished
            .to_string()
            .contains("pre-existing output has no adopted consumer release")
    );
}

#[test]
fn missing_main_artifact_ownership_fails_closed_instead_of_reporting_empty_cleanup() {
    let h = main_host();
    assert_eq!(h.auto("use", &["--operation", "use.consumer"]).0, 0);
    let ledger_path = h
        .root
        .join(".chrono-harness/state/automatic-cleanup/ledger.json");
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
