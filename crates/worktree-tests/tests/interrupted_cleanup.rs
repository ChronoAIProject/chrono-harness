use super::automatic::{native_consumer, native_git};
use super::*;
use std::{
    process::Stdio,
    thread,
    time::{Duration, Instant},
};
const AUTO_POLICY: &str = ".chrono-harness/cleanup.json";
impl Host {
    pub(super) fn kernel_cleanup(&self) {
        self.automatic("evidence-retain");
        let path = self.root.join(AUTO_POLICY);
        let mut policy = json(&fs::read(&path).unwrap()).unwrap();
        policy["schema"] = value!("chrono-worktree-automatic-cleanup/v2");
        fs::write(path, serde_json::to_vec_pretty(&policy).unwrap()).unwrap();
        commit(&self.root);
        git(&self.root, &["push", "-q", "warehouse", "dev"]);
    }
}
fn output(target: &Path) {
    fs::create_dir_all(target.join("output λ")).unwrap();
    fs::write(target.join("output λ/cache"), "reproducible").unwrap();
}

#[test]
fn busy_or_damaged_enrollment_metadata_does_not_block_an_independent_worktree() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_consumer(&h, "ordinary");
    let a = h.parent.join("metadata-a");
    let b = h.parent.join("metadata-b");
    assert_eq!(h.invoke("feature", "metadata-a", &a).0, 0);
    assert_eq!(h.invoke("feature", "metadata-b", &b).0, 0);
    let original = fs::read(h.ledger_path(&a)).unwrap();
    let gate = chrono_harness::ownership::Lease::acquire(
        &h.record_directory(&a).join("metadata.lease"),
        None,
        false,
        true,
        None,
    )
    .unwrap()
    .unwrap();
    fs::write(h.ledger_path(&a), b"original damaged enrollment").unwrap();
    let (code, used, error) = h.auto(
        "use",
        &["--path", b.to_str().unwrap(), "--operation", "use.consumer"],
    );
    assert_eq!(code, 0, "{used} {error}");
    assert_eq!(used["managed_process"]["stdout"], "ordinary-work");
    assert_eq!(
        fs::read(h.ledger_path(&a)).unwrap(),
        b"original damaged enrollment"
    );
    assert!(
        !h.root
            .join(".chrono-harness/state/automatic-cleanup/ledger.json")
            .exists()
    );
    assert!(
        !h.root
            .join(".chrono-harness/state/automatic-cleanup/admission.lease")
            .exists()
    );
    gate.stable().unwrap();
    drop(gate);
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_ne!(
        code, 0,
        "damaged owner must remain failed: {maintained} {error}"
    );
    let (code, used, error) = h.auto(
        "use",
        &["--path", b.to_str().unwrap(), "--operation", "use.consumer"],
    );
    assert_eq!(
        code, 0,
        "independent use after owner failure: {used} {error}"
    );
    fs::write(h.ledger_path(&a), original).unwrap();
    assert_eq!(h.auto("maintain", &[]).0, 0);
}

#[test]
fn ordinary_use_does_not_sweep_an_independent_worktrees_outputs() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_consumer(&h, "ordinary");
    let a = h.parent.join("local-cleanup-a");
    let b = h.parent.join("local-cleanup-b");
    assert_eq!(h.invoke("feature", "local-cleanup-a", &a).0, 0);
    assert_eq!(h.invoke("feature", "local-cleanup-b", &b).0, 0);
    output(&a);
    let original = fs::read(h.ledger_path(&a)).unwrap();
    let (code, used, error) = h.auto(
        "use",
        &["--path", b.to_str().unwrap(), "--operation", "use.consumer"],
    );
    assert_eq!(code, 0, "{used} {error}");
    assert!(a.join("output λ/cache").is_file());
    assert_eq!(fs::read(h.ledger_path(&a)).unwrap(), original);
    assert!(
        used["drain"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["path"] != value!(a))
    );
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert!(!a.join("output λ").exists());
    assert!(a.join("payload").is_file());
}

#[test]
fn slow_cache_disposal_allows_independent_use_and_merges_its_original_enrollment() {
    use std::{
        io::{Read, Write},
        os::fd::AsRawFd,
    };
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_consumer(&h, "ordinary");
    let target = h.parent.join("slow-cache");
    let sibling = h.parent.join("independent-use");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    native_git(
        &h,
        "cleanup-io",
        value!({
            "target":target,
            "ledger":h.ledger_path(&target),
            "claim":h.parent.join("cleanup-io-claimed"),
            "synchronize":listener.local_addr().unwrap().to_string()
        }),
    );
    assert_eq!(h.invoke("feature", "independent-use", &sibling).0, 0);
    assert_eq!(h.invoke("feature", "slow-cache", &target).0, 0);
    output(&target);
    let ledger = h.ledger();
    let original = ledger["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["path"] == value!(target))
        .unwrap()
        .clone();
    let cleanup = CapturedChild::spawn(&mut h.auto_command("maintain", &[]), &h.root);
    let mut ready = libc::pollfd {
        fd: listener.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    assert_eq!(
        unsafe { libc::poll(&mut ready, 1, 30_000) },
        1,
        "cleanup never reached the actual effect boundary"
    );
    let (mut release, _) = listener.accept().unwrap();
    release
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let mut message = [0; 5];
    release.read_exact(&mut message).unwrap();
    assert_eq!(&message, b"ready");
    let gate = chrono_harness::ownership::Lease::acquire(
        &h.record_directory(&sibling).join("metadata.lease"),
        None,
        false,
        true,
        None,
    )
    .unwrap();
    assert!(
        gate.is_some(),
        "physical cache disposal retained global enrollment exclusion"
    );
    drop(gate);
    let (code, used, error) = h.auto(
        "use",
        &[
            "--path",
            sibling.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "independent use: {used} {error}");
    assert!(target.join("output λ/cache").is_file());
    let concurrent = h.ledger();
    let sibling_entry = concurrent["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["path"] == value!(sibling))
        .unwrap()
        .clone();
    release.write_all(b"x").unwrap();
    let (code, report, error) = h.received(cleanup.wait_with_output().unwrap());
    assert_eq!(code, 0, "cleanup: {report} {error}");
    assert!(!target.join("output λ").exists());
    let after = h.ledger();
    let entries = after["entries"].as_array().unwrap();
    let cleaned = entries
        .iter()
        .find(|entry| entry["path"] == value!(target))
        .unwrap();
    assert_eq!(cleaned["attachment"], original["attachment"]);
    assert_eq!(cleaned["enrollment"], original["enrollment"]);
    assert_eq!(cleaned["ownership"]["cache_pending"], false);
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry["path"] == value!(sibling))
            .unwrap(),
        &sibling_entry,
        "cleanup overwrote the independently completed use"
    );
}

fn reclaim_reports(root: &Path, paths: &[String], mut round: Value) {
    let inventory = chrono_harness::retained_artifacts::retention::Inventory::adopted(root)
        .unwrap()
        .unwrap();
    let mut rounds = Vec::new();
    // Real maintenance has a wall deadline, including under concurrent tests.
    // Exercise its bounded continuation instead of demanding one-round disposal.
    for attempt in 0..8 {
        assert!(
            round["accounting_errors"].as_array().unwrap().is_empty(),
            "{round}"
        );
        assert!(round["nodes"].as_u64().unwrap() <= 256, "{round}");
        assert!(
            round["verified_bytes"].as_u64().unwrap() <= 1048576,
            "{round}"
        );
        rounds.push(round);
        if paths.iter().all(|path| !root.join(path).exists()) {
            return;
        }
        if attempt < 7 {
            round = inventory.maintain().unwrap();
        } else {
            break;
        }
    }
    panic!("bounded maintenance did not reclaim unrooted reports {paths:?}: {rounds:?}");
}

#[test]
fn caller_report_consumer_protects_primary_and_coordinator_publication() {
    if !super::isolated_capability::isolated_capability_test(
        "interrupted_cleanup::caller_report_consumer_protects_primary_and_coordinator_publication",
    ) {
        return;
    }
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_consumer(&h, "noop");
    let cleanup = h.root.join(AUTO_POLICY);
    let mut policy = json(&fs::read(&cleanup).unwrap()).unwrap();
    policy["retained_producers"] = value!([{"id":"caller-fixture",
        "directory":".chrono-harness/state/caller-fixtures/",
        "temporary":true,"generated_outputs":[]}]);
    fs::write(cleanup, serde_json::to_vec(&policy).unwrap()).unwrap();
    let retention = chrono_harness::retained_artifacts::retention::POLICY_PATH;
    fs::write(h.root.join(retention), serde_json::to_vec(&value!({
        "schema":"chrono-output-retention/v2","producers":{"lifecycle-original":[".chrono-harness/state/worktrees/"]},
        "keep_seconds":0,"max_related_outputs":32,"max_summaries":16,"max_nodes_per_round":256,
        "max_bytes_per_round":1048576,"max_millis_per_round":1000
    })).unwrap()).unwrap();
    let mut fm = json(&fs::read(h.root.join(FM)).unwrap()).unwrap();
    fm["files"]
        .as_array_mut()
        .unwrap()
        .push(file(retention, value!([])));
    fs::write(h.root.join(FM), serde_json::to_vec(&fm).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "caller-fixture")
            .unwrap();
    let fixture = chrono_worktree::TemporaryHost::allocate(
        &store,
        "caller-fixture",
        &outputs,
        "actual caller report consumer ",
    )
    .unwrap();
    let target = h.parent.join("caller-report");
    assert_eq!(h.invoke("feature", "caller-report", &target).0, 0);
    output(&target);
    for (root, consumer, operation) in [
        (&h.root, "primary-reader", "maintain"),
        (&target, "linked-reader", "maintain"),
        (&target, "linked-use-reader", "use"),
    ] {
        let mut command =
            Command::new(source().join("crates/worktree/target/debug/chrono-worktree"));
        command
            .current_dir(root)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap())
            .args([
                operation,
                "--host-root",
                root.to_str().unwrap(),
                "--config",
                POLICY,
                "--report-consumer",
                consumer,
            ]);
        if operation == "use" {
            command.args([
                "--path",
                target.to_str().unwrap(),
                "--operation",
                "use.consumer",
            ]);
        }
        h._dir.bind(&mut command);
        let output = h._dir.capture_bound(&mut command);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report = json(&output.stdout).unwrap();
        let path = h.root.join(report["report_path"].as_str().unwrap());
        let raw = fs::read(&path).unwrap();
        assert_eq!(json(&raw).unwrap(), report);
        let mut passes = vec![&report];
        if operation == "use" {
            assert_eq!(report["managed_process"]["exit_code"], 0);
            assert!(report["post_use_cleanup"].is_object());
            assert!(report["post_use_cleanup"].get("producer_objects").is_none());
            passes.push(&report["post_use_cleanup"]);
        }
        let dependencies: Vec<_> = passes
            .iter()
            .flat_map(|pass| pass["producer_drains"].as_array().unwrap())
            .map(|item| {
                let receipt = &item["receipt"];
                let path = h.root.join(receipt["path"].as_str().unwrap());
                let bytes = fs::read(&path).unwrap();
                assert_eq!(sha256(&bytes), receipt["sha256"]);
                let child = json(&bytes).unwrap();
                assert_eq!(child["status"], item["status"]);
                assert_eq!(child["error"], item["error"]);
                assert!(
                    child["producer_objects"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|object| {
                            object["producer"] == "caller-fixture"
                                && object["status"] == "protected"
                                && h.root.join(object["path"].as_str().unwrap()) == fixture.path()
                        })
                );
                (path, bytes)
            })
            .collect();
        assert_eq!(dependencies.len(), if operation == "use" { 0 } else { 1 });
        assert_eq!(
            dependencies
                .iter()
                .map(|(path, _)| path)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            dependencies.len(),
            "admission and post-use cleanup retain distinct original operations"
        );
        let managed_receipt = if operation == "use" {
            let receipt = &report["managed_use_receipt"];
            let path = h.root.join(receipt["path"].as_str().unwrap());
            let bytes = fs::read(&path).unwrap();
            assert_eq!(sha256(&bytes), receipt["sha256"]);
            assert_eq!(json(&bytes).unwrap()["process"]["exit_code"], 0);
            Some((path, bytes))
        } else {
            None
        };
        let i = chrono_harness::retained_artifacts::retention::Inventory::adopted(&h.root)
            .unwrap()
            .unwrap();
        i.maintain().unwrap();
        assert_eq!(fs::read(&path).unwrap(), raw);
        for (path, bytes) in &dependencies {
            assert_eq!(
                fs::read(path).expect("caller must retain the referenced original"),
                *bytes
            );
        }
        if let Some((path, bytes)) = &managed_receipt {
            assert_eq!(fs::read(path).unwrap(), *bytes);
        }
        assert!(report.get("producer_objects").is_none());
        for item in report["drain"].as_array().unwrap() {
            if let Some(receipt) = item.get("receipt") {
                let bytes = fs::read(h.root.join(receipt["path"].as_str().unwrap())).unwrap();
                assert_eq!(sha256(&bytes), receipt["sha256"]);
                let child = json(&bytes).unwrap();
                if let Some(intent) = child.get("cache_intent") {
                    let bytes = fs::read(h.root.join(intent["path"].as_str().unwrap())).unwrap();
                    assert_eq!(sha256(&bytes), intent["sha256"]);
                }
            }
        }
        let originals: Vec<chrono_harness::retained_artifacts::retention::OriginalIdentity> =
            serde_json::from_value(report["report_consumer"]["originals"].clone()).unwrap();
        i.release_consumer(consumer, &originals, &"1".repeat(64), &"2".repeat(64))
            .unwrap();
        let mut released = vec![report["report_path"].as_str().unwrap().to_owned()];
        released.extend(dependencies.iter().map(|(path, _)| {
            path.strip_prefix(&h.root)
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned()
        }));
        reclaim_reports(&h.root, &released, i.maintain().unwrap());
        if let Some((path, bytes)) = managed_receipt {
            assert_eq!(
                fs::read(path).unwrap(),
                bytes,
                "the fixture policy does not enroll automatic-cleanup state"
            );
        }
        assert!(root.join(".git").exists());
        assert!(root.join("payload").exists());
    }
}

#[test]
fn linked_report_keeps_producer_originals_until_publication_without_consumer() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    let cleanup = h.root.join(AUTO_POLICY);
    let mut policy = json(&fs::read(&cleanup).unwrap()).unwrap();
    policy["retained_producers"] = value!([
        {"id":"first-fixture","directory":".chrono-harness/state/first-fixture/",
            "temporary":true,"generated_outputs":[]},
        {"id":"second-fixture","directory":".chrono-harness/state/second-fixture/",
            "temporary":true,"generated_outputs":[]}
    ]);
    fs::write(cleanup, serde_json::to_vec(&policy).unwrap()).unwrap();
    let retention = chrono_harness::retained_artifacts::retention::POLICY_PATH;
    fs::write(
        h.root.join(retention),
        serde_json::to_vec(&value!({
            "schema":"chrono-output-retention/v2",
            "producers":{"lifecycle-original":[".chrono-harness/state/worktrees/"]},
            "keep_seconds":0,"max_related_outputs":32,"max_summaries":16,"max_nodes_per_round":256,
            "max_bytes_per_round":1048576,"max_millis_per_round":1000
        }))
        .unwrap(),
    )
    .unwrap();
    let mut fm = json(&fs::read(h.root.join(FM)).unwrap()).unwrap();
    fm["files"]
        .as_array_mut()
        .unwrap()
        .push(file(retention, value!([])));
    fs::write(h.root.join(FM), serde_json::to_vec(&fm).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("unrooted-linked-report");
    assert_eq!(h.invoke("feature", "unrooted-linked-report", &target).0, 0);
    let mut previous = Vec::new();
    for _ in 0..2 {
        if !previous.is_empty() {
            let inventory =
                chrono_harness::retained_artifacts::retention::Inventory::adopted(&h.root)
                    .unwrap()
                    .unwrap();
            reclaim_reports(&h.root, &previous, inventory.maintain().unwrap());
        }
        let output = Command::new(source().join("crates/worktree/target/debug/chrono-worktree"))
            .current_dir(&target)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap())
            .args(["maintain", "--config", POLICY])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report = json(&output.stdout).unwrap();
        assert_eq!(report["status"], "maintained");
        assert!(report.get("report_consumer").is_none());
        previous.clear();
        let path = report["report_path"].as_str().unwrap();
        assert_eq!(fs::read(h.root.join(path)).unwrap(), output.stdout);
        previous.push(path.to_owned());
        let drains = report["producer_drains"].as_array().unwrap();
        assert_eq!(drains.len(), 2);
        for (entry, producer) in drains.iter().zip(["first-fixture", "second-fixture"]) {
            assert_eq!(entry["producer"], producer);
            let original = drain_report(entry);
            assert_eq!(original["status"], "cleaned");
            assert_eq!(
                original["producer_objects"][0]["status"],
                "protected-unknown"
            );
            previous.push(entry["receipt"]["path"].as_str().unwrap().to_owned());
        }
    }
    let destination = h.parent.join("linked-start-report");
    let output = Command::new(source().join("crates/worktree/target/debug/chrono-worktree"))
        .current_dir(&target)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap())
        .args([
            "start",
            "--host-root",
            target.to_str().unwrap(),
            "--config",
            POLICY,
            "--kind",
            "feature",
            "--name",
            "linked-start-report",
            "--path",
            destination.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = json(&output.stdout).unwrap();
    assert_eq!(report["status"], "created");
    let parent = target.join(report["report_path"].as_str().unwrap());
    assert_eq!(fs::read(&parent).unwrap(), output.stdout);
    for path in &previous {
        assert!(
            h.root.join(path).exists(),
            "linked creation preserves unrelated coordinator reports"
        );
    }
    let drains = report["producer_drains"].as_array().unwrap();
    assert!(
        drains.is_empty(),
        "creation must not sweep unrelated producers"
    );
    let inventory = chrono_harness::retained_artifacts::retention::Inventory::adopted(&h.root)
        .unwrap()
        .unwrap();
    reclaim_reports(&h.root, &previous, inventory.maintain().unwrap());
    let inventory = chrono_harness::retained_artifacts::retention::Inventory::adopted(&target)
        .unwrap()
        .unwrap();
    reclaim_reports(
        &target,
        &[report["report_path"].as_str().unwrap().to_owned()],
        inventory.maintain().unwrap(),
    );
    for root in [&h.root, &target, &destination] {
        assert!(root.join("payload").exists());
    }
}

#[test]
fn cache_disposal_reuses_immutable_tree_and_rechecks_live_identity() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    let target = h.parent.join("tree-reuse");
    assert_eq!(h.invoke("feature", "tree-reuse", &target).0, 0);
    output(&target);
    fs::create_dir_all(target.join("nested/cache")).unwrap();
    fs::write(target.join("nested/cache/other"), "rebuildable").unwrap();
    let head = git(&target, &["rev-parse", "HEAD"]);
    let source = fs::read(target.join("payload")).unwrap();
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(
        !report["processes"].as_array().unwrap().iter().any(|row| {
            row["argv"] == value!(["--no-replace-objects", "rev-parse", "--git-common-dir"])
        }),
        "the same invoking/coordinator checkout shares one live common-directory observation"
    );
    let child = drain_report(&report["drain"][0]);
    let processes = child["processes"].as_array().unwrap();
    let trees: Vec<_> = processes
        .iter()
        .filter(|p| {
            p["root"] == value!(target)
                && p["argv"]
                    == value!([
                        "--no-replace-objects",
                        "ls-tree",
                        "-rz",
                        "--full-tree",
                        head.trim()
                    ])
        })
        .collect();
    let live = processes
        .iter()
        .filter(|p| {
            p["root"] == value!(target)
                && p["argv"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|a| a == "--show-toplevel")
        })
        .count();
    println!(
        "CACHE_DISPOSAL tree_processes={} live_identity_processes={live}",
        trees.len()
    );
    assert_eq!(
        trees.len(),
        1,
        "one immutable tree acquisition in the disposal operation"
    );
    assert!(
        live >= 4,
        "identity must remain live across both removal effects"
    );
    assert!(processes.iter().any(|p| p["root"] == value!(target)
        && p["argv"] == value!(["--no-replace-objects", "ls-files", "-z"])));
    let original: chrono_harness::ProcessResult =
        serde_json::from_value(trees[0]["process"].clone()).unwrap();
    chrono_harness::observation::process_success(&original).unwrap();
    assert!(!target.join("output λ").exists());
    assert!(!target.join("nested/cache").exists());
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), head);
    assert_eq!(fs::read(target.join("payload")).unwrap(), source);
}

#[test]
fn cache_disposal_checks_new_index_paths_after_immutable_tree_acquisition() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_git(&h, "stage-after-tree", value!({}));
    let target = h.parent.join("tree-index");
    assert_eq!(h.invoke("feature", "tree-index", &target).0, 0);
    output(&target);
    fs::write(h.parent.join("stage-after-tree"), "inject").unwrap();
    let (code, report, error) = h.auto("maintain", &[]);
    assert_ne!(code, 0, "{report} {error}");
    assert!(
        report
            .to_string()
            .contains("disposal directory contains tracked paths"),
        "{report}"
    );
    assert_eq!(
        fs::read_to_string(target.join("output λ/staged-source")).unwrap(),
        "source"
    );
    assert!(target.join("output λ/cache").exists());
    assert_eq!(h.ledger()["entries"][0]["ownership"]["cache_pending"], true);
}

#[test]
fn live_checkout_identity_failures_preserve_pending_cache_and_original_git_error() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_git(&h, "identity-fault", value!({}));
    let target = h.parent.join("observed");
    assert_eq!(h.invoke("feature", "observed", &target).0, 0);
    output(&target);
    fs::create_dir(h.parent.join("foreign-common")).unwrap();
    let source = fs::read(target.join("payload")).unwrap();
    for fault in [
        "truncated",
        "malformed-head",
        "foreign-common",
        "foreign-branch",
        "foreign-metadata",
        "failure",
    ] {
        fs::write(h.parent.join("identity-fault"), fault).unwrap();
        let (code, report, error) = h.auto("maintain", &[]);
        assert_ne!(code, 0, "{fault}: {report} {error}");
        assert!(target.join("output λ/cache").exists(), "{fault}");
        assert_eq!(fs::read(target.join("payload")).unwrap(), source);
        assert_eq!(h.ledger()["entries"][0]["ownership"]["cache_pending"], true);
        if fault == "failure" {
            let child = drain_report(&report["drain"][0]);
            let processes = child["processes"].as_array().unwrap();
            let original = &processes.last().unwrap()["process"];
            assert_eq!(original["exit_code"], 71);
            assert_eq!(original["stderr"], "original identity failure\n");
        }
    }
    fs::remove_file(h.parent.join("identity-fault")).unwrap();
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(!target.join("output λ").exists());
    assert_eq!(fs::read(target.join("payload")).unwrap(), source);
}
fn participating_check(h: &Host) {
    super::check_inputs::bind(h);
    let mut cfg = json(&fs::read(h.root.join(CONFIG)).unwrap()).unwrap();
    cfg["runner"]["path"] = value!(".chrono-harness/bin/inner-check");
    cfg["canonical_check"]["argv"] = value!([".chrono-harness/bin/inner-check", "check"]);
    cfg["canonical_check"]["participation"] =
        value!({"operation":"worktree.check","tool":"chrono-worktree","argv":["check"]});
    cfg["tools"].as_array_mut().unwrap().push(value!({"id":"sh","program":"/bin/sh","resolution":"PATH-once","version_argv":["-c","printf fixture"],"expected_version":"fixture"}));
    fs::write(
        h.root.join(CONFIG),
        serde_json::to_vec_pretty(&cfg).unwrap(),
    )
    .unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
}
fn install_inner(root: &Path, stdout: &str, stderr: &str, exit: i32) {
    super::check_inputs::install(root);
    super::check_inputs::install_readonly_file(
        root,
        Path::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer")),
        "inner-check",
        b"fixture\n",
    );
    fs::create_dir_all(root.join(".chrono-harness/state")).unwrap();
    fs::write(
        root.join(".chrono-harness/state/inner-check-response.json"),
        serde_json::to_vec(&value!({"stdout":stdout,"stderr":stderr,"exit":exit})).unwrap(),
    )
    .unwrap();
}
#[test]
fn public_participating_check_preserves_zero_and_nonzero_stderr_results() {
    for (source, exit) in [None, Some("local"), Some("ci")]
        .into_iter()
        .flat_map(|source| [(source, 0), (source, 7)])
    {
        let h = Host::new("payload");
        participating_check(&h);
        let mut config = json(&fs::read(h.root.join(CONFIG)).unwrap()).unwrap();
        config["environment"]["inherit"]
            .as_array_mut()
            .unwrap()
            .extend([
                value!("CHRONO_TEST_AUTHENTICATION"),
                value!("CHRONO_TEST_AUTHENTICATION_ABSENT"),
                value!("CHRONO_TEST_AGENT_ID"),
            ]);
        config["environment"]["credential_environment"] = value!([
            "CHRONO_TEST_AUTHENTICATION",
            "CHRONO_TEST_AUTHENTICATION_ABSENT"
        ]);
        fs::write(h.root.join(CONFIG), serde_json::to_vec(&config).unwrap()).unwrap();
        h.kernel_cleanup();
        h.policy(|policy| {
            policy["agent_identity_env"] = value!("CHRONO_TEST_AGENT_ID");
            policy["environment"]["inherit"]
                .as_array_mut()
                .unwrap()
                .push(value!("CHRONO_TEST_AGENT_ID"));
        });
        let target = h.parent.join("console");
        let created = h.native_output(
            h.auto_command(
                "start",
                &[
                    "--kind",
                    "feature",
                    "--name",
                    "console",
                    "--path",
                    target.to_str().unwrap(),
                ],
            )
            .env("CHRONO_TEST_AGENT_ID", "console-agent"),
        );
        assert_eq!(h.received(created).0, 0);
        install_inner(&target, "original stdout\n", "original diagnostic\n", exit);
        let response = target.join(".chrono-harness/state/inner-check-response.json");
        let mut expected = json(&fs::read(&response).unwrap()).unwrap();
        expected["authentication"] = value!({
            "CHRONO_TEST_AUTHENTICATION":"inert managed authentication λ",
            "CHRONO_TEST_AUTHENTICATION_ABSENT":null,
        });
        fs::write(response, serde_json::to_vec(&expected).unwrap()).unwrap();
        let out = Command::new(target.join(".chrono-harness/bin/chrono-harness"))
            .current_dir(&target)
            .env("CHRONO_TEST_AGENT_ID", "console-agent")
            .env(
                "CHRONO_TEST_AUTHENTICATION",
                "inert managed authentication λ",
            )
            .env_remove("CHRONO_TEST_AUTHENTICATION_ABSENT")
            .env_remove("CHRONO_CHECK_SOURCE")
            .envs(source.map(|value| ("CHRONO_CHECK_SOURCE", value)))
            .arg("check")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(exit), "{out:?}");
        assert_eq!(out.stdout, b"original stdout\n");
        assert!(out.stderr.starts_with(b"original diagnostic\n"), "{out:?}");
        assert!(
            String::from_utf8_lossy(&out.stderr[b"original diagnostic\n".len()..])
                .contains("W_WORKTREE_ATTENTION"),
            "the public check consumer must forward its creator's reminder: {out:?}"
        );
        assert!(String::from_utf8_lossy(&out.stderr).contains(target.to_str().unwrap()));
        assert_eq!(h.ledger()["entries"][0]["uses"], value!([]));
        let (_, owner, _) = h.received(
            Command::new(target.join(".chrono-harness/bin/chrono-worktree"))
                .current_dir(&target)
                .env("CHRONO_TEST_AGENT_ID", "console-agent")
                .env(
                    "CHRONO_TEST_AUTHENTICATION",
                    "inert managed authentication λ",
                )
                .env_remove("CHRONO_TEST_AUTHENTICATION_ABSENT")
                .env_remove("CHRONO_CHECK_SOURCE")
                .envs(source.map(|value| ("CHRONO_CHECK_SOURCE", value)))
                .args(["check", "--config", POLICY])
                .output()
                .unwrap(),
        );
        assert_eq!(owner["managed_process"]["exit_code"], exit);
        assert!(!owner.to_string().contains("inert managed authentication λ"));
        let process = &owner["managed_process"];
        assert_eq!(process["environment_representation"], "sha256");
        assert_eq!(
            process["environment"]["CHRONO_TEST_AUTHENTICATION"],
            sha256("inert managed authentication λ".as_bytes())
        );
        assert!(
            process["environment"]
                .get("CHRONO_TEST_AUTHENTICATION_ABSENT")
                .is_none()
        );
        let mut environment = std::collections::BTreeMap::from([
            (
                "CHRONO_TEST_AGENT_ID".to_owned(),
                "console-agent".to_owned(),
            ),
            ("PATH".to_owned(), std::env::var("PATH").unwrap()),
            (
                "CHRONO_TEST_AUTHENTICATION".to_owned(),
                "inert managed authentication λ".to_owned(),
            ),
            (
                "CHRONO_WORKTREE_CHECK".to_owned(),
                target.to_str().unwrap().to_owned(),
            ),
        ]);
        if let Some(source) = source {
            environment.insert("CHRONO_CHECK_SOURCE".to_owned(), source.to_owned());
            assert_eq!(
                process["environment"]["CHRONO_CHECK_SOURCE"],
                sha256(source.as_bytes())
            );
        } else {
            assert!(process["environment"].get("CHRONO_CHECK_SOURCE").is_none());
        }
        assert_eq!(
            process["environment_digest"],
            chrono_harness::wire::digest(&environment).unwrap()
        );
        let receipt = &owner["managed_use_receipt"];
        let raw = fs::read(h.root.join(receipt["path"].as_str().unwrap())).unwrap();
        assert_eq!(receipt["sha256"], sha256(&raw));
        assert!(!String::from_utf8_lossy(&raw).contains("inert managed authentication λ"));
        assert_eq!(
            chrono_harness::full::expand_process(&json(&raw).unwrap()["process"]).unwrap(),
            *process
        );
        let report = fs::read(h.root.join(owner["report_path"].as_str().unwrap())).unwrap();
        assert!(!String::from_utf8_lossy(&report).contains("inert managed authentication λ"));
        assert_eq!(process["failure"], Value::Null);
        assert!(process["child_pid"].as_u64().is_some());
        assert!(process["launcher_pid"].as_u64().is_some());
        assert_eq!(
            process["timeout_seconds"],
            config["protocol"]["timeout_seconds"]
        );
        assert_eq!(
            process["output_limit_bytes"],
            config["protocol"]["stdout_limit_bytes"]
        );
        assert_eq!(
            process["sha256"],
            sha256(&fs::read(target.join(".chrono-harness/bin/inner-check")).unwrap())
        );
        assert_eq!(process["stdout_sha256"], sha256(b"original stdout\n"));
        assert_eq!(process["stderr_sha256"], sha256(b"original diagnostic\n"));
        assert_eq!(
            owner["managed_process"]["stdout_bytes"],
            value!(b"original stdout\n".to_vec())
        );
        assert_eq!(
            owner["managed_process"]["stderr_bytes"],
            value!(b"original diagnostic\n".to_vec())
        );
        // Target policy failure is a lifecycle refusal, distinct from diagnostics.
        fs::write(target.join(AUTO_POLICY), "ordinary policy edit").unwrap();
        let refused = Command::new(target.join(".chrono-harness/bin/chrono-harness"))
            .current_dir(&target)
            .arg("check")
            .output()
            .unwrap();
        assert_eq!(refused.status.code(), Some(2), "{refused:?}");
        assert!(String::from_utf8_lossy(&refused.stderr).contains("participation refused"));
        assert!(refused.stdout.is_empty());
    }
}

#[test]
fn nonempty_large_cleanup_participation_preserves_reports_and_console_under_bound() {
    for exit in [0, 7] {
        let h = Host::new("payload");
        participating_check(&h);
        h.kernel_cleanup();
        h.policy(|p| p["immutable_input_limits"] = value!({CONFIG:2_097_152, FM:2_097_152}));
        commit(&h.root);
        git(&h.root, &["push", "-q", "warehouse", "dev"]);
        let target = h.parent.join("large-cleanup");
        assert_eq!(h.invoke("feature", "large-cleanup", &target).0, 0);
        // The real observed gap has a small coordinator and a larger, separately
        // adopted host configuration. Keep that same ownership relationship.
        let mut bytes = fs::read(target.join(CONFIG)).unwrap();
        bytes.resize(2_022_211, b' ');
        fs::write(target.join(CONFIG), &bytes).unwrap();
        let mut filemap = fs::read(target.join(FM)).unwrap();
        filemap.resize(1_791_500, b' ');
        fs::write(target.join(FM), &filemap).unwrap();
        commit(&target);
        output(&target);
        let target_head = git(&target, &["rev-parse", "HEAD"]);
        install_inner(
            &h.root,
            "original collection stdout\n",
            "original warning\n",
            exit,
        );
        let out = Command::new(h.root.join(".chrono-harness/bin/chrono-harness"))
            .current_dir(&h.root)
            .args(["check", "--collect"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(exit), "{out:?}");
        assert_eq!(out.stdout, b"original collection stdout\n");
        assert!(out.stderr.starts_with(b"original warning\n"), "{out:?}");
        assert!(String::from_utf8_lossy(&out.stderr).contains("W_WORKTREE_AGENT_SCOPE"));
        assert!(target.join("output λ/cache").exists());
        assert_eq!(git(&target, &["rev-parse", "HEAD"]), target_head);
        assert_eq!(fs::read(target.join(CONFIG)).unwrap(), bytes);
        let report_path = fs::read_dir(h.root.join(".chrono-harness/state/worktrees"))
            .unwrap()
            .map(|r| r.unwrap().path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with("check-")
            })
            .unwrap();
        let parent = fs::read(&report_path).unwrap();
        assert!(parent.len() < 67_108_864);
        let report = json(&parent).unwrap();
        assert_eq!(report["managed_process"]["exit_code"], exit);
        assert_eq!(
            report["managed_process"]["stdout_bytes"],
            value!(b"original collection stdout\n".to_vec())
        );
        assert_eq!(
            report["managed_process"]["stderr_bytes"],
            value!(b"original warning\n".to_vec())
        );
        assert_eq!(
            report["managed_command_failed"].as_bool().unwrap_or(false),
            exit != 0
        );
        assert!(report["drain"].as_array().unwrap().is_empty());
        let (code, maintenance, error) = h.auto("maintain", &[]);
        assert_eq!(code, 0, "{maintenance} {error}");
        assert!(!target.join("output λ").exists());
        let row = &maintenance["drain"][0];
        assert!(row.get("report").is_none());
        assert_eq!(row["status"], "cleaned");
        let child = drain_report(row);
        assert_eq!(child["worktree_removal"], "preserved");
        assert_eq!(child["artifact_disposals"][0]["bytes_before"], 12);
        assert_eq!(child["artifact_disposals"][0]["bytes_after"], 0);
        println!(
            "LIFECYCLE_TRANSPORT managed_exit={exit} parent_bytes={} child_bytes={} stdout_bound=67108864",
            parent.len(),
            row["input"]["byte_length"]
        );
    }
}

#[test]
fn unrelated_policy_drift_retains_failure_and_allows_independent_admission() {
    let h = Host::new("payload");
    participating_check(&h);
    h.kernel_cleanup();
    native_consumer(&h, "ordinary");
    let a = h.parent.join("drifted-a");
    let b = h.parent.join("valid-b");
    assert_eq!(h.invoke("feature", "drifted-a", &a).0, 0);
    output(&a);
    fs::write(a.join(AUTO_POLICY), "ordinary uncommitted policy edit").unwrap();
    let (code, start, error) = h.invoke("feature", "valid-b", &b);
    assert_eq!(code, 0, "{start} {error}");
    assert!(start["cleanup_failures"].as_array().unwrap().is_empty());
    assert!(start["drain"].as_array().unwrap().is_empty());
    let (code, maintenance, error) = h.auto("maintain", &[]);
    assert_ne!(code, 0, "{maintenance} {error}");
    let receipt = maintenance["drain"][0]["receipt"].clone();
    let receipt_path = h.root.join(receipt["path"].as_str().unwrap());
    let original = fs::read(&receipt_path).unwrap();
    assert_eq!(sha256(&original), receipt["sha256"]);
    install_inner(&b, "checked", "check warning\n", 0);
    let (code, used, error) = h.auto(
        "use",
        &["--path", b.to_str().unwrap(), "--operation", "use.consumer"],
    );
    assert_eq!(code, 0, "{used} {error}");
    assert_eq!(used["managed_process"]["stdout"], "ordinary-work");
    let checked = Command::new(b.join(".chrono-harness/bin/chrono-harness"))
        .current_dir(&b)
        .arg("check")
        .output()
        .unwrap();
    assert_eq!(checked.status.code(), Some(0), "{checked:?}");
    assert_eq!(checked.stdout, b"checked");
    assert!(
        checked.stderr.starts_with(b"check warning\n"),
        "{checked:?}"
    );
    assert!(String::from_utf8_lossy(&checked.stderr).contains("W_WORKTREE_AGENT_SCOPE"));
    let head = git(&h.root, &["rev-parse", "HEAD"]);
    let (code, rebuilt, error) = h.reconstruct(
        reconstruction(&head, &head, value!([])),
        "independent-reconstruct",
    );
    assert_eq!(code, 0, "{} {error}", rebuilt["error"]);
    assert!(rebuilt["cleanup_failures"].as_array().unwrap().is_empty());
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_ne!(code, 0, "{maintained} {error}");
    assert!(a.join("output λ/cache").exists());
    assert_eq!(fs::read(receipt_path).unwrap(), original);
    assert!(h.ledger()["entries"][0]["terminal"].is_null());
    let blocked = h.auto(
        "use",
        &["--path", a.to_str().unwrap(), "--operation", "use.consumer"],
    );
    assert_ne!(blocked.0, 0, "{blocked:?}");
    assert!(blocked.1["managed_process"].is_null());
}
fn await_file(path: &Path) {
    let began = Instant::now();
    while !path.exists() && began.elapsed() < Duration::from_secs(10) {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(path.exists(), "missing handshake: {}", path.display());
}
#[test]
fn successful_use_without_finish_reclaims_caches_and_preserves_active_work() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_consumer(&h, "rebuild-evidence");
    let target = h.parent.join("resumable");
    assert_eq!(h.invoke("feature", "resumable", &target).0, 0);
    let (code, report, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "{report} {error}");
    fs::write(target.join("payload"), "unsaved source").unwrap();
    let refs = git(&h.root, &["show-ref"]);
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(
        !target.join("output λ").exists(),
        "no-token session loss must reclaim quiescent caches"
    );
    assert_eq!(
        fs::read_to_string(target.join("payload")).unwrap(),
        "unsaved source"
    );
    assert_eq!(
        fs::read_to_string(target.join(".chrono-harness/state/evidence")).unwrap(),
        "evidence"
    );
    assert_eq!(git(&h.root, &["show-ref"]), refs);
    let ledger = h.ledger();
    assert_eq!(ledger["entries"][0]["status"], "active");
    assert!(ledger["entries"][0]["terminal"].is_null());
    assert_eq!(ledger["entries"][0]["uses"], value!([]));
    assert_eq!(
        h.auto("maintain", &[]).1["drain"],
        value!([]),
        "unchanged generation must not be disposed again"
    );
}

struct OwnedGroup(String);
impl Drop for OwnedGroup {
    fn drop(&mut self) {
        let _ = Command::new("/bin/kill")
            .args(["-KILL", "--", &format!("-{}", self.0)])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}
pub(super) struct CapturedChild {
    child: std::process::Child,
    stdout: PathBuf,
    stderr: PathBuf,
    joined: bool,
    release_before_drop: Option<PathBuf>,
}
impl CapturedChild {
    pub(super) fn spawn(command: &mut Command, root: &Path) -> Self {
        static NUMBER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let id = NUMBER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let state = root.join(".chrono-harness/state/child-captures");
        fs::create_dir_all(&state).unwrap();
        let stdout = state.join(format!("{id}.stdout"));
        let stderr = state.join(format!("{id}.stderr"));
        let child = command
            .stdout(fs::File::create(&stdout).unwrap())
            .stderr(fs::File::create(&stderr).unwrap())
            .spawn()
            .unwrap();
        fs::write(
            state.join(format!("{id}.binding.json")),
            serde_json::to_vec_pretty(&value!({"pid":child.id(),"command":format!("{command:?}")}))
                .unwrap(),
        )
        .unwrap();
        Self {
            child,
            stdout,
            stderr,
            joined: false,
            release_before_drop: None,
        }
    }
    fn release_on_drop(&mut self, path: PathBuf) {
        self.release_before_drop = Some(path);
    }
    pub(super) fn kill(&mut self) -> std::io::Result<()> {
        self.child.kill()
    }
    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.child.try_wait()
    }
    fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        let status = self.child.wait()?;
        self.joined = true;
        fs::write(
            self.stdout.with_extension("joined.json"),
            serde_json::to_vec(
                &value!({"pid":self.child.id(),"status":format!("{status:?}"),"joined":true}),
            )
            .unwrap(),
        )?;
        Ok(status)
    }
    pub(super) fn wait_with_output(mut self) -> std::io::Result<std::process::Output> {
        let status = self.wait()?;
        Ok(std::process::Output {
            status,
            stdout: fs::read(&self.stdout)?,
            stderr: fs::read(&self.stderr)?,
        })
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn record_wait_state(&self, watched: &Path) {
        // Observe this fixture's declared child before teardown. These reads are
        // failure diagnostics, never a cleanup or completion decision.
        let mut pending = vec![self.child.id()];
        let mut seen = std::collections::BTreeSet::new();
        let mut processes = Vec::new();
        while let Some(pid) = pending.pop() {
            if seen.len() >= 128 || !seen.insert(pid) {
                continue;
            }
            #[cfg(target_os = "linux")]
            {
                let root = PathBuf::from(format!("/proc/{pid}"));
                let read = |name: &str| match fs::read(root.join(name)) {
                    Ok(mut bytes) => {
                        bytes.truncate(4096);
                        value!({"bytes": String::from_utf8_lossy(&bytes).replace('\0', " ")})
                    }
                    Err(error) => value!({"error": error.to_string()}),
                };
                let before = read("stat");
                let mut children = Vec::new();
                if let Ok(tasks) = fs::read_dir(root.join("task")) {
                    for task in tasks.flatten() {
                        if let Ok(ids) = fs::read_to_string(task.path().join("children")) {
                            children.extend(
                                ids.split_whitespace()
                                    .filter_map(|id| id.parse::<u32>().ok()),
                            );
                        }
                    }
                }
                pending.extend(children.iter().copied());
                processes.push(value!({"pid":pid,"stat_before":before,"command":read("cmdline"),"wait_channel":read("wchan"),"children":children,"stat_after":read("stat")}));
            }
            #[cfg(target_os = "macos")]
            {
                let observe = |program: &str, args: &[&str]| match Command::new(program)
                    .args(args)
                    .output()
                {
                    Ok(out) => {
                        value!({"exit":out.status.code(),"stdout":String::from_utf8_lossy(&out.stdout),"stderr":String::from_utf8_lossy(&out.stderr)})
                    }
                    Err(error) => value!({"error":error.to_string()}),
                };
                let pid_text = pid.to_string();
                let state = observe(
                    "/bin/ps",
                    &[
                        "-p",
                        &pid_text,
                        "-o",
                        "pid=,ppid=,state=,etime=,time=,command=",
                    ],
                );
                let children = observe("/usr/bin/pgrep", &["-P", &pid_text]);
                pending.extend(
                    children["stdout"]
                        .as_str()
                        .unwrap_or("")
                        .split_whitespace()
                        .filter_map(|pid| pid.parse::<u32>().ok()),
                );
                processes.push(value!({"pid":pid,"state":state,"children":children}));
            }
        }
        let parent = watched.parent().unwrap();
        let markers: Vec<_> = [
            "cargo-wrapper",
            "native-pid",
            "native-holder",
            "check-holder",
            "bootstrap-holder",
        ]
        .into_iter()
        .map(|name| {
            let result = fs::read_to_string(parent.join(name));
            value!({"name":name,"result":result.map_err(|e| e.to_string())})
        })
        .collect();
        eprintln!(
            "CAPTURED_CHILD_WAIT_STATE {}",
            value!({"watched":watched,"processes":processes,"markers":markers})
        );
        #[cfg(target_os = "macos")]
        if let Some(trace) = std::env::var_os("CHRONO_CHECK_HANDOFF_TRACE").map(PathBuf::from) {
            // Sampling starts only after the existing deadline has failed. It
            // cannot turn that failure into success or supply a cleanup decision.
            if trace.is_absolute() {
                for row in processes
                    .iter()
                    .rev()
                    .filter(|row| {
                        row["state"]["stdout"]
                            .as_str()
                            .is_some_and(|s| s.contains("/.chrono-harness/bin/chrono-"))
                    })
                    .take(2)
                {
                    let pid = row["pid"].as_u64().unwrap().to_string();
                    let path = trace.with_extension(format!("{pid}.sample.txt"));
                    if path.exists() {
                        continue;
                    }
                    let sampled = Command::new("/usr/bin/sample")
                        .args([&pid, "1", "10", "-file"])
                        .arg(&path)
                        .output();
                    let observation = match sampled {
                        Ok(out) => {
                            value!({"exit":out.status.code(),"stdout":String::from_utf8_lossy(&out.stdout),"stderr":String::from_utf8_lossy(&out.stderr)})
                        }
                        Err(error) => value!({"error":error.to_string()}),
                    };
                    eprintln!(
                        "CAPTURED_CHILD_STACK {}",
                        value!({"pid":pid,"path":path,"observation":observation})
                    );
                }
            }
        }
    }
    pub(super) fn await_file(&mut self, path: &Path) {
        self.await_file_bounded(path, Some(Duration::from_secs(10)));
    }
    pub(super) fn await_managed_file(&mut self, path: &Path) {
        // Cold Cargo builds and canonical input preparation already have the
        // fixture's registered process bounds. Consume their actual result
        // instead of killing them at an unrelated ten-second readiness deadline.
        self.await_file_bounded(path, None);
    }
    fn await_file_bounded(&mut self, path: &Path, readiness: Option<Duration>) {
        let began = Instant::now();
        while !path.exists() && readiness.is_none_or(|limit| began.elapsed() < limit) {
            if self.try_wait().unwrap().is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        if !path.exists() {
            let before_kill = self.try_wait().unwrap();
            if before_kill.is_none() {
                #[cfg(any(target_os = "linux", target_os = "macos"))]
                self.record_wait_state(path);
                let _ = self.kill();
            }
            let status = self.wait().unwrap();
            panic!(
                "missing handshake {}; before test kill {before_kill:?}; joined {status:?}; stdout {}; stderr {}; captures {} {}",
                path.display(),
                String::from_utf8_lossy(&fs::read(&self.stdout).unwrap()),
                String::from_utf8_lossy(&fs::read(&self.stderr).unwrap()),
                self.stdout.display(),
                self.stderr.display()
            );
        }
    }
}
impl Drop for CapturedChild {
    fn drop(&mut self) {
        if !self.joined {
            if let Some(path) = &self.release_before_drop {
                match fs::write(path, "fixture teardown") {
                    Ok(()) => {
                        // The consumer keeps its lease after a wrapper dies.
                        // Release its declared protocol before destroying the
                        // temporary host, including setup assertions that fail
                        // before the consumer's process-group guard exists.
                        let began = Instant::now();
                        loop {
                            match self.try_wait() {
                                Ok(Some(_)) => break,
                                Ok(None) if began.elapsed() < Duration::from_secs(10) => {
                                    thread::sleep(Duration::from_millis(10));
                                }
                                Ok(None) => {
                                    eprintln!(
                                        "fixture release did not join its wrapper within 10 seconds"
                                    );
                                    break;
                                }
                                Err(error) => {
                                    eprintln!("fixture release observation failed: {error}");
                                    break;
                                }
                            }
                        }
                    }
                    Err(error) => eprintln!("fixture release {} failed: {error}", path.display()),
                }
            }
            let _ = self.kill();
            let status = self.wait();
            eprintln!(
                "joined fixture child after test exit: {status:?}; stdout {}; stderr {}",
                String::from_utf8_lossy(&fs::read(&self.stdout).unwrap()),
                String::from_utf8_lossy(&fs::read(&self.stderr).unwrap())
            );
        }
    }
}
fn wrapper(h: &Host, target: &Path) -> CapturedChild {
    CapturedChild::spawn(
        &mut h.auto_command(
            "use",
            &[
                "--path",
                target.to_str().unwrap(),
                "--operation",
                "use.consumer",
            ],
        ),
        &h.root,
    )
}
fn reclaim(h: &Host, target: &Path) -> Value {
    let began = Instant::now();
    loop {
        let (code, report, error) = h.auto("maintain", &[]);
        assert_eq!(code, 0, "{report} {error}");
        if !target.join("output λ").exists() {
            return report;
        }
        assert!(
            began.elapsed() < Duration::from_secs(10),
            "final holder did not release: {report}"
        );
        thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn killed_wrapper_and_parent_preserve_live_grandchild_until_final_close() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_consumer(&h, "parent");
    let target = h.parent.join("descendants");
    assert_eq!(h.invoke("feature", "descendants", &target).0, 0);
    output(&target);
    let mut child = wrapper(&h, &target);
    child.await_managed_file(&target.join(".chrono-harness/state/parent"));
    await_file(&target.join(".chrono-harness/state/grandchild"));
    let _group =
        OwnedGroup(fs::read_to_string(target.join(".chrono-harness/state/parent")).unwrap());
    child.kill().unwrap();
    assert!(!child.wait().unwrap().success());
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(target.join("output λ/cache").exists());
    assert!(
        report["drain"][0]["preserved_reason"]
            .as_str()
            .unwrap()
            .contains("live registered")
    );
    fs::write(target.join(".chrono-harness/state/release-parent"), "exit").unwrap();
    await_file(&target.join(".chrono-harness/state/parent-done"));
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(
        target.join("output λ/cache").exists(),
        "grandchild must retain the inherited description"
    );
    assert_ne!(h.auto("finish", &["--path", target.to_str().unwrap()]).0, 0);
    fs::write(
        target.join(".chrono-harness/state/release-grandchild"),
        "exit",
    )
    .unwrap();
    await_file(&target.join(".chrono-harness/state/grandchild-done"));
    reclaim(&h, &target);
    assert_eq!(h.ledger()["entries"][0]["uses"], value!([]));
    assert_eq!(h.ledger()["entries"][0]["status"], "active");
    assert!(h.ledger()["entries"][0]["terminal"].is_null());
    let receipt = &h.ledger()["entries"][0]["enrollment"]["interrupted_uses"][0];
    let interrupted =
        json(&fs::read(h.root.join(receipt["path"].as_str().unwrap())).unwrap()).unwrap();
    assert_eq!(interrupted["original_result"]["presence"], "absent");
    assert_eq!(interrupted["task_completion"], "not-claimed");
}

#[test]
fn one_dead_wrapper_and_one_live_use_keep_shared_protection() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_consumer(&h, "shared-holder");
    let target = h.parent.join("concurrent");
    assert_eq!(h.invoke("feature", "concurrent", &target).0, 0);
    output(&target);
    let state = target.join(".chrono-harness/state");
    let mut first = wrapper(&h, &target);
    first.release_on_drop(state.join("release"));
    let mut second = CapturedChild::spawn(
        &mut h.auto_command(
            "use",
            &[
                "--path",
                target.to_str().unwrap(),
                "--operation",
                "use.consumer",
            ],
        ),
        &h.root,
    );
    second.release_on_drop(state.join("release"));
    // The same registered operation has two live callers. Each child names
    // its ready marker with its actual wrapper PID; no ledger/directory polling.
    let ready = [
        format!("ready-{}", first.child.id()),
        format!("ready-{}", second.child.id()),
    ];
    first.await_managed_file(&state.join(&ready[0]));
    second.await_managed_file(&state.join(&ready[1]));
    assert_eq!(
        h.ledger()["entries"][0]["uses"].as_array().unwrap().len(),
        2
    );
    let _groups: Vec<_> = ready
        .iter()
        .map(|name| OwnedGroup(fs::read_to_string(state.join(name)).unwrap()))
        .collect();
    assert_eq!(_groups.len(), 2);
    first.kill().unwrap();
    first.wait().unwrap();
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(target.join("output λ/cache").exists());
    fs::write(state.join("release"), "joined").unwrap();
    let (code, report, error) = h.received(second.wait_with_output().unwrap());
    assert_eq!(code, 0, "{report} {error}");
    assert_eq!(report["managed_process"]["stdout"], "completed");
    reclaim(&h, &target);
    assert_eq!(h.ledger()["entries"][0]["uses"], value!([]));
}

#[test]
fn fixture_setup_panic_releases_and_joins_a_live_managed_consumer() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_consumer(&h, "managed-child");
    let target = h.parent.join("setup-panic");
    assert_eq!(h.invoke("feature", "setup-panic", &target).0, 0);
    output(&target);
    let state = target.join(".chrono-harness/state");
    let mut child = wrapper(&h, &target);
    child.release_on_drop(state.join("release"));
    child.await_managed_file(&state.join("ready"));
    assert_eq!(
        h.ledger()["entries"][0]["uses"].as_array().unwrap().len(),
        1
    );
    let stdout = child.stdout.clone();
    let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let _child = child;
        panic!("intentional fixture setup interruption");
    }));
    assert!(interrupted.is_err());
    let joined = json(&fs::read(stdout.with_extension("joined.json")).unwrap()).unwrap();
    assert_eq!(joined["joined"], true);
    assert_eq!(joined["status"], "ExitStatus(unix_wait_status(0))");
    let report = json(&fs::read(&stdout).unwrap()).unwrap();
    assert_eq!(report["managed_process"]["stdout"], "joined-child");
    assert_eq!(report["managed_process"]["exit_code"], 0);
    assert!(report["managed_process"]["failure"].is_null());
    assert_eq!(h.ledger()["entries"][0]["uses"], value!([]));
    reclaim(&h, &target);
    assert!(target.join("payload").exists());
}

#[test]
fn interrupted_partial_cache_disposal_retries_without_owned_git_lock_or_finish() {
    use std::os::unix::process::ExitStatusExt;
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_git(
        &h,
        "cache-interrupt",
        value!({"output":"partial/output λ","observed":"cache-observed"}),
    );
    let target = h.parent.join("partial");
    assert_eq!(h.invoke("feature", "partial", &target).0, 0);
    output(&target);
    fs::create_dir_all(target.join("nested/cache")).unwrap();
    fs::write(target.join("nested/cache/data"), "second output").unwrap();
    fs::write(
        h.parent.join("interrupt-cache"),
        "interrupt after first effect",
    )
    .unwrap();
    let out = h.auto_command("maintain", &[]).output().unwrap();
    assert_eq!(out.status.signal(), Some(9), "{out:?}");
    assert!(!target.join("output λ").exists());
    assert!(target.join("nested/cache/data").exists());
    assert!(!git(&h.root, &["worktree", "list", "--porcelain"]).contains("locked"));
    let entry = h.ledger()["entries"][0].clone();
    let attempt = &entry["cache_attempts"][0];
    assert!(attempt["receipt"].is_null());
    let intent_path = h.root.join(attempt["intent"]["path"].as_str().unwrap());
    let intent_bytes = fs::read(&intent_path).unwrap();
    let result_path = h.root.join(attempt["path"].as_str().unwrap());
    let original = fs::read(&result_path).ok();
    fs::remove_file(h.parent.join("interrupt-cache")).unwrap();
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(!target.join("nested/cache").exists());
    assert_eq!(fs::read(intent_path).unwrap(), intent_bytes);
    assert_eq!(
        fs::read(result_path).ok(),
        original,
        "original missing/partial result must be unchanged"
    );
    assert_eq!(
        drain_report(&report["drain"][0])["artifact_disposals"][0]["status"],
        "already-absent"
    );
    assert_eq!(h.ledger()["entries"][0]["status"], "active");
    assert!(h.ledger()["entries"][0]["terminal"].is_null());
    assert_eq!(
        h.ledger()["entries"][0]["cache_attempts"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn cache_retries_reference_failed_and_partial_originals_without_growth() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_git(&h, "cache-fail", value!({"output":"cache-retry/output λ"}));
    let target = h.parent.join("cache-retry");
    assert_eq!(h.invoke("feature", "cache-retry", &target).0, 0);
    fs::write(h.parent.join("fail-cache"), "fail after disposal").unwrap();
    let mut originals = Vec::new();
    for retry in 0..4 {
        output(&target);
        let (code, report, error) = h.auto("maintain", &[]);
        assert_ne!(code, 0, "{report} {error}");
        let report = &drain_report(&report["drain"][0]);
        assert_eq!(report["status"], "failed");
        assert!(report["processes"].as_array().unwrap().iter().any(|p| {
            p["process"]["exit_code"] == 71 && p["process"]["stderr"] == "original cache failure\n"
        }));
        let path = h.root.join(report["report_path"].as_str().unwrap());
        let bytes = fs::read(&path).unwrap();
        if retry > 0 {
            let input = &report["prior_cache_attempt"]["result"]["input"];
            assert_eq!(input["schema"], "chrono-worktree-retained-input/v1");
            let retained = fs::read(h.root.join(input["path"].as_str().unwrap())).unwrap();
            let (_, previous): &(PathBuf, Vec<u8>) = originals.last().unwrap();
            assert_eq!(&retained, previous);
            assert_eq!(input["sha256"], sha256(&retained));
            assert_eq!(input["byte_length"], retained.len());
            assert!(
                bytes.len() <= originals[0].1.len() * 3,
                "recursive cache report growth"
            );
        }
        originals.push((path, bytes));
    }
    // Model a torn, unpublished result of the explicitly registered last attempt.
    let mut ledger = h.ledger();
    let latest = ledger["entries"][0]["cache_attempts"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap();
    latest["receipt"] = Value::Null;
    let path = originals.last().unwrap().0.clone();
    let partial = b"{\"status\":\"failed\",\"processes\":";
    fs::write(&path, partial).unwrap();
    h.set_ledger(&ledger);
    output(&target);
    let (code, failed, error) = h.auto("maintain", &[]);
    assert_ne!(code, 0, "{failed} {error}");
    let original = &drain_report(&failed["drain"][0])["prior_cache_attempt"]["result"];
    assert_eq!(original["original_outcome"], "unknown");
    let input = &original["input"];
    assert_eq!(input["format"], "bytes");
    let retained_path = h.root.join(input["path"].as_str().unwrap());
    assert_ne!(retained_path, path);
    assert_eq!(fs::read(&retained_path).unwrap(), partial);
    assert_eq!(input["sha256"], sha256(partial));
    assert_eq!(input["byte_length"], partial.len());
    fs::write(&path, "source subsequently changed").unwrap();
    fs::remove_file(h.parent.join("fail-cache")).unwrap();
    output(&target);
    fs::write(&retained_path, "damaged snapshot").unwrap();
    let (code, refused, error) = h.auto("maintain", &[]);
    assert_ne!(code, 0, "{refused} {error}");
    assert!(target.join("output λ/cache").exists());
    fs::write(&retained_path, partial).unwrap();
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(!target.join("output λ").exists());
    assert!(target.exists());
    assert!(h.ledger()["entries"][0]["terminal"].is_null());
    assert_eq!(fs::read(&path).unwrap(), b"source subsequently changed");
    for (path, bytes) in &originals[..originals.len() - 1] {
        assert_eq!(fs::read(path).unwrap(), *bytes);
    }
}

#[test]
fn interrupted_cache_generation_is_superseded_by_admitted_rebuilding_after_source_commit() {
    use std::os::unix::process::ExitStatusExt;
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_consumer(&h, "rebuild");
    native_git(
        &h,
        "cache-interrupt",
        value!({"output":"superseded/output λ"}),
    );
    let target = h.parent.join("superseded");
    assert_eq!(h.invoke("feature", "superseded", &target).0, 0);
    output(&target);
    fs::create_dir_all(target.join("nested/cache")).unwrap();
    fs::write(target.join("nested/cache/data"), "partial cleanup").unwrap();
    fs::write(h.parent.join("interrupt-cache"), "interrupt").unwrap();
    let interrupted = CapturedChild::spawn(&mut h.auto_command("maintain", &[]), &h.root)
        .wait_with_output()
        .unwrap();
    assert_eq!(interrupted.status.signal(), Some(9), "{interrupted:?}");
    h.received(interrupted);
    assert!(!target.join("output λ").exists());
    assert!(target.join("nested/cache/data").exists());
    let old = h.ledger()["entries"][0]["cache_attempts"][0].clone();
    let intent_path = h.root.join(old["intent"]["path"].as_str().unwrap());
    let result_path = h.root.join(old["path"].as_str().unwrap());
    let intent = fs::read(&intent_path).unwrap();
    let result = fs::read(&result_path).ok();
    fs::remove_file(h.parent.join("interrupt-cache")).unwrap();
    fs::write(target.join("payload"), "ordinary source commit").unwrap();
    let head = commit(&target);
    let refs = git(&h.root, &["show-ref"]);
    let (code, report, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "{report} {error}");
    assert!(target.join("output λ/cache").exists());
    reclaim(&h, &target);
    let entry = h.ledger()["entries"][0].clone();
    assert_eq!(entry["ownership"]["generation"], 2);
    assert_eq!(entry["cache_attempts"][0], old);
    assert_eq!(fs::read(intent_path).unwrap(), intent);
    assert_eq!(fs::read(result_path).ok(), result);
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), head);
    assert_eq!(git(&h.root, &["show-ref"]), refs);
    assert_eq!(entry["status"], "active");
    assert!(entry["terminal"].is_null());
}

#[test]
fn persisted_unsealed_birth_recovers_through_normal_use_after_actual_interruption() {
    use std::os::unix::process::ExitStatusExt;
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_consumer(&h, "rebuild");
    let target = h.parent.join("unsealed");
    let ledger = h.ledger_path(&target);
    let fixture = native_git(&h, "birth-interrupt", value!({"ledger":ledger}));
    fs::write(h.parent.join("interrupt-birth"), "after durable enrollment").unwrap();
    let interrupted = CapturedChild::spawn(
        Command::new(source().join("crates/worktree/target/debug/chrono-worktree")).args([
            "start",
            "--host-root",
            h.root.to_str().unwrap(),
            "--config",
            POLICY,
            "--kind",
            "feature",
            "--name",
            "unsealed",
            "--path",
            target.to_str().unwrap(),
        ]),
        &h.root,
    )
    .wait_with_output()
    .unwrap();
    assert_eq!(interrupted.status.signal(), Some(9), "{interrupted:?}");
    h.received(interrupted);
    let original = h.ledger()["entries"][0].clone();
    assert!(original["enrollment"]["sealed_receipt"].is_null());
    fs::remove_file(h.parent.join("interrupt-birth")).unwrap();
    // Refusal still validates the original attachment/policy before recovery.
    fs::write(target.join(AUTO_POLICY), "unsaved policy").unwrap();
    let (code, _, _) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_ne!(code, 0);
    assert!(h.ledger()["entries"][0]["enrollment"]["recovered_receipt"].is_null());
    fs::copy(h.root.join(AUTO_POLICY), target.join(AUTO_POLICY)).unwrap();
    let recovery_path = h.root.join(format!(
        ".chrono-harness/state/automatic-cleanup/interrupted-birth-{}.json",
        sha256(
            original["enrollment"]["receipt"]["report_path"]
                .as_str()
                .unwrap()
                .as_bytes()
        )
    ));
    let mut settings = json(&fs::read(&fixture).unwrap()).unwrap();
    settings["mode"] = value!("recovery-interrupt");
    settings["recovery"] = value!(recovery_path);
    fs::write(&fixture, serde_json::to_vec(&settings).unwrap()).unwrap();
    fs::write(
        h.parent.join("interrupt-recovery"),
        "after durable reconciliation",
    )
    .unwrap();
    let again = CapturedChild::spawn(
        &mut h.auto_command(
            "use",
            &[
                "--path",
                target.to_str().unwrap(),
                "--operation",
                "use.consumer",
            ],
        ),
        &h.root,
    )
    .wait_with_output()
    .unwrap();
    assert_eq!(again.status.signal(), Some(9), "{again:?}");
    h.received(again);
    assert!(h.ledger()["entries"][0]["enrollment"]["recovered_receipt"].is_null());
    let interrupted_recovery = fs::read(&recovery_path).unwrap();
    fs::remove_file(h.parent.join("interrupt-recovery")).unwrap();
    let (code, report, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "{report} {error}");
    let recovered = h.ledger()["entries"][0].clone();
    assert!(recovered["enrollment"]["sealed_receipt"].is_null());
    assert_eq!(
        recovered["enrollment"]["receipt"],
        original["enrollment"]["receipt"]
    );
    assert!(recovered["enrollment"]["recovered_receipt"].is_object());
    let recovery_path = h.root.join(
        recovered["enrollment"]["recovered_receipt"]["path"]
            .as_str()
            .unwrap(),
    );
    let bytes = fs::read(&recovery_path).unwrap();
    assert_eq!(bytes, interrupted_recovery);
    let recovery = json(&bytes).unwrap();
    assert_eq!(recovery["original_outcome"], "not-established");
    assert_eq!(recovery["terminal_handoff"], "not-claimed");
    let (code, report, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "{report} {error}");
    assert_eq!(fs::read(recovery_path).unwrap(), bytes);
    reclaim(&h, &target);
    assert!(h.ledger()["entries"][0]["terminal"].is_null());
}

#[test]
fn kernel_caches_preserve_staged_source_unknown_lease_drift_and_foreign_locks() {
    use std::os::unix::fs::symlink;
    for fault in [
        "staged",
        "missing-lease",
        "replaced-lease",
        "foreign-lock",
        "policy-drift",
        "symlink",
    ] {
        let h = Host::new("payload");
        h.kernel_cleanup();
        let target = h.parent.join("protected");
        assert_eq!(h.invoke("feature", "protected", &target).0, 0);
        output(&target);
        let lease = h.root.join(
            h.ledger()["entries"][0]["ownership"]["path"]
                .as_str()
                .unwrap(),
        );
        match fault {
            "staged" => {
                fs::write(target.join("output λ/source"), "staged source").unwrap();
                git(&target, &["add", "-f", "output λ/source"]);
            }
            "missing-lease" => {
                fs::remove_file(&lease).unwrap();
            }
            "replaced-lease" => {
                fs::rename(&lease, lease.with_extension("original")).unwrap();
                fs::write(&lease, "unknown").unwrap();
            }
            "foreign-lock" => {
                git(
                    &h.root,
                    &[
                        "worktree",
                        "lock",
                        "--reason",
                        "foreign owner",
                        target.to_str().unwrap(),
                    ],
                );
            }
            "policy-drift" => {
                fs::write(target.join(AUTO_POLICY), "uncommitted policy").unwrap();
            }
            "symlink" => {
                fs::remove_dir_all(target.join("output λ")).unwrap();
                symlink(&h.root, target.join("output λ")).unwrap();
            }
            _ => unreachable!(),
        }
        let refs = git(&h.root, &["show-ref"]);
        let (code, report, error) = h.auto("maintain", &[]);
        assert_ne!(code, 0, "{fault}: {report} {error}");
        assert!(target.exists());
        assert!(target.join("output λ").exists());
        assert_eq!(git(&h.root, &["show-ref"]), refs);
        assert!(h.ledger()["entries"][0]["terminal"].is_null());
    }
}

#[test]
fn kernel_generation_rearms_and_normal_finish_retains_original_nonzero_exit() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_consumer(&h, "rebuild-error");
    let target = h.parent.join("rearm");
    assert_eq!(h.invoke("feature", "rearm", &target).0, 0);
    for generation in [1, 2] {
        let (code, report, error) = h.auto(
            "use",
            &[
                "--path",
                target.to_str().unwrap(),
                "--operation",
                "use.consumer",
            ],
        );
        assert_ne!(code, 0, "{report} {error}");
        assert_eq!(report["managed_process"]["exit_code"], 23);
        assert_eq!(report["managed_process"]["stderr"], "original-error");
        reclaim(&h, &target);
        assert_eq!(
            h.ledger()["entries"][0]["ownership"]["generation"],
            generation
        );
    }
    let (code, report, error) = h.auto(
        "finish",
        &["--path", target.to_str().unwrap(), "--dispose-evidence"],
    );
    assert_eq!(code, 0, "{report} {error}");
    assert!(!target.exists());
}

#[test]
fn admission_owner_death_keeps_git_mutation_and_native_hook_protected() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_git(&h, "admission-interrupt", value!({}));
    h.hook(value!({"writes":[{"path":h.parent.join("native-hook"),"contents":"native"}],"release":h.parent.join("release-hook")}));
    fs::write(h.parent.join("interrupt-admission"), "kill owner").unwrap();
    let target = h.parent.join("interrupted-birth");
    let mut start = CapturedChild::spawn(
        Command::new(source().join("crates/worktree/target/debug/chrono-worktree"))
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap())
            .args([
                "start",
                "--host-root",
                h.root.to_str().unwrap(),
                "--config",
                POLICY,
                "--kind",
                "feature",
                "--name",
                "interrupted-birth",
                "--path",
                target.to_str().unwrap(),
            ]),
        &h.root,
    );
    start.await_file(&h.parent.join("admission-holder"));
    let _group = OwnedGroup(fs::read_to_string(h.parent.join("admission-holder")).unwrap());
    assert!(!start.wait().unwrap().success());
    for release in ["release-mutation", "release-hook"] {
        let (code, report, error) = h.auto("maintain", &[]);
        assert_eq!(code, 0, "{report} {error}");
        let preserved = report["drain"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["path"] == value!(target))
            .expect("interrupted target remains explicitly protected");
        assert_eq!(preserved["status"], "preserved");
        assert_eq!(preserved["preserved_reason"], "enrollment metadata is busy");
        assert!(h.ledger()["entries"].as_array().unwrap().is_empty());
        assert!(!h.parent.join(release).exists());
        fs::write(h.parent.join(release), "join original mutation/hook").unwrap();
        if release == "release-mutation" {
            await_file(&h.parent.join("native-hook"));
        }
    }
    {
        use std::os::fd::AsRawFd;
        let metadata = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(h.record_directory(&target).join("metadata.lease"))
            .unwrap();
        assert_eq!(
            unsafe { libc::flock(metadata.as_raw_fd(), libc::LOCK_EX) },
            0
        );
    }
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(target.join("payload").exists());
    assert!(
        h.ledger()["entries"].as_array().unwrap().is_empty(),
        "crash must not fabricate completed birth"
    );
    assert!(
        git(&h.root, &["worktree", "list", "--porcelain"]).contains("locked"),
        "ordinary interrupted birth recovery remains explicit"
    );
}

#[test]
fn returned_use_without_token_still_protects_detached_native_child() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_consumer(&h, "detached-parent");
    let target = h.parent.join("returned");
    assert_eq!(h.invoke("feature", "returned", &target).0, 0);
    output(&target);
    let (code, report, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "{report} {error}");
    await_file(&target.join(".chrono-harness/state/detached"));
    let _group =
        OwnedGroup(fs::read_to_string(target.join(".chrono-harness/state/detached")).unwrap());
    assert_eq!(h.ledger()["entries"][0]["uses"], value!([]));
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(target.join("output λ/cache").exists());
    fs::write(target.join(".chrono-harness/state/release"), "join").unwrap();
    await_file(&target.join(".chrono-harness/state/detached-done"));
    reclaim(&h, &target);
}

#[test]
fn adopted_short_check_enters_owner_and_nested_native_runner_keeps_lease() {
    let h = Host::new("payload");
    super::check_inputs::bind(&h);
    h.kernel_cleanup();
    let mut cfg = json(&fs::read(h.root.join(CONFIG)).unwrap()).unwrap();
    let trace = std::env::var_os("CHRONO_CHECK_HANDOFF_TRACE").map(PathBuf::from);
    if let Some(path) = &trace {
        assert!(path.is_absolute());
        cfg["environment"]["values"]["GIT_TRACE2_EVENT"] = value!(path);
        h.policy(|p| p["environment"]["values"]["GIT_TRACE2_EVENT"] = value!(path));
    }
    cfg["canonical_check"]["participation"] =
        value!({"operation":"worktree.check","tool":"chrono-worktree","argv":["check"]});
    fs::write(
        h.root.join(CONFIG),
        serde_json::to_vec_pretty(&cfg).unwrap(),
    )
    .unwrap();
    let mut judges = json(&fs::read(h.root.join(JUDGES)).unwrap()).unwrap();
    judges["judges"][0]["argv"] = value!(["hold"]);
    fs::write(
        h.root.join(JUDGES),
        serde_json::to_vec_pretty(&judges).unwrap(),
    )
    .unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("normal-check");
    let other = h.parent.join("interrupted-other");
    assert_eq!(h.invoke("integration", "normal-check", &target).0, 0);
    assert_eq!(h.invoke("feature", "interrupted-other", &other).0, 0);
    super::check_inputs::install(&target);
    output(&target);
    output(&other);
    fs::write(target.join("payload"), "candidate").unwrap();
    commit(&target);
    assert!(
        git(&target, &["ls-files", "--", ".chrono-harness/bin/"]).is_empty(),
        "installed executables must remain untracked host artifacts"
    );
    if let Some(path) = &trace {
        fs::write(
            path.with_extension("handoff.json"),
            serde_json::to_vec(&value!({
                "target":target,
                "start_unix_ns":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos().to_string(),
            })).unwrap(),
        ).unwrap();
    }
    let mut check = CapturedChild::spawn(
        Command::new(target.join(".chrono-harness/bin/chrono-harness"))
            .current_dir(&target)
            .env("CHRONO_CHECK_SOURCE", "local")
            .arg("check"),
        &h.root,
    );
    let handshake = target.join(".chrono-harness/state/check-holder");
    let began = Instant::now();
    check.await_managed_file(&handshake);
    let handoff_seconds = began.elapsed().as_secs_f64();
    assert!(
        other.join("output λ/cache").exists(),
        "ordinary check has no cleanup dependency on another enrollment"
    );
    let _group =
        OwnedGroup(fs::read_to_string(target.join(".chrono-harness/state/check-holder")).unwrap());
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(!other.join("output λ").exists());
    assert!(target.join("output λ/cache").exists());
    fs::write(
        target.join(".chrono-harness/state/check-release"),
        "join native check",
    )
    .unwrap();
    let out = check.wait_with_output().unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("check: pass (exit 0)"),
        "original check console must be forwarded: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let policy = json(&fs::read(h.root.join(POLICY)).unwrap()).unwrap();
    let directory = h.root.join(policy["report_directory"].as_str().unwrap());
    for row in fs::read_dir(directory).unwrap() {
        let path = row.unwrap().path();
        if path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("check-")
        {
            let report = json(&fs::read(&path).unwrap()).unwrap();
            let mut calls = std::collections::BTreeMap::<String, usize>::new();
            for process in report["processes"].as_array().unwrap() {
                *calls.entry(process["argv"].to_string()).or_default() += 1;
            }
            eprintln!(
                "CHECK_ADMISSION_MEASUREMENT {}",
                value!({"handoff_seconds":handoff_seconds,"report_sha256":sha256(&fs::read(&path).unwrap()),"git_calls":calls,"drain":report["drain"].as_array().map(Vec::len)})
            );
        }
    }
    reclaim(&h, &target);
    assert!(h.ledger()["entries"][0]["terminal"].is_null());
    for args in [
        vec!["check", "--unit", "missing"],
        vec!["check", "--collect"],
    ] {
        let out = Command::new(target.join(".chrono-harness/bin/chrono-harness"))
            .current_dir(&target)
            .env("CHRONO_CHECK_SOURCE", "local")
            .args(args)
            .output()
            .unwrap();
        assert_ne!(out.status.code(), Some(0));
        let entry = &h.ledger()["entries"][0];
        assert_eq!(entry["uses"], value!([]));
        assert_eq!(
            entry["ownership"]["cache_pending"], true,
            "selection failure still participates and retains original error"
        );
        assert_eq!(h.auto("maintain", &[]).0, 0);
    }
}

#[test]
fn native_cargo_child_survives_its_cargo_and_managed_wrappers() {
    let h = Host::retained("payload");
    let allocation = h._dir.path().to_owned();
    let coordinator = source();
    let maintain_allocation = || {
        let mut command =
            Command::new(coordinator.join("crates/worktree/target/debug/chrono-worktree"));
        command.current_dir(&coordinator).arg("maintain");
        chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
        // Maintenance already publishes its original at the surviving owner.
        // Capturing here must not recreate the allocation after its handoff.
        let output = command.output().unwrap();
        let report = json(&output.stdout).unwrap();
        let failures = report["cleanup_failures"].as_array().unwrap();
        assert_eq!(
            output.status.code(),
            Some(if failures.is_empty() { 0 } else { 2 }),
            "allocation maintenance failed: {}",
            allocation.display()
        );
        producer_objects(&report)
            .as_array()
            .unwrap()
            .iter()
            .find(|row| {
                row["producer"] == "worktree-test-host"
                    && row["path"]
                        .as_str()
                        .unwrap()
                        .ends_with(allocation.file_name().unwrap().to_str().unwrap())
            })
            .expect("actual fixture allocation maintenance record")
            .clone()
    };
    h.kernel_cleanup();
    super::automatic::native_consumer_actions(&h, "cargo-run", "cargo-prepare");
    // Compile and list the native tests under the registered fixture action, with the same
    // managed environment and process bounds as its later Cargo invocation.
    // The native-holder deadline measures launch/lease handoff after preparation.
    let target = h.parent.join("native-cargo");
    assert_eq!(h.invoke("feature", "native-cargo", &target).0, 0);
    let (code, prepared, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "test.consumer",
        ],
    );
    assert_eq!(code, 0, "{prepared} {error}");
    assert_eq!(prepared["status"], "used");
    assert_eq!(prepared["managed_process"]["exit_code"], 0);
    assert_eq!(prepared["managed_process"]["failure"], Value::Null);
    assert!(
        prepared["managed_process"]["stdout"]
            .as_str()
            .unwrap()
            .contains("native_holder: test"),
        "native test executable was not prepared: {prepared}"
    );
    assert!(!target.join(".chrono-harness/state/native-holder").exists());
    let cargo_output = target.join(".chrono-harness/state/cargo-case/target");
    assert!(cargo_output.is_dir(), "the actual Cargo output must exist");
    output(&target);
    let synchronization = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    fs::write(
        target.join(".chrono-harness/state/native-sync"),
        synchronization.local_addr().unwrap().to_string(),
    )
    .unwrap();
    let mut owner = wrapper(&h, &target);
    owner.await_file(&target.join(".chrono-harness/state/native-holder"));
    use std::{
        io::{Read, Write},
        os::fd::AsRawFd,
    };
    let mut ready = libc::pollfd {
        fd: synchronization.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    assert_eq!(unsafe { libc::poll(&mut ready, 1, 10_000) }, 1);
    let (mut native_completion, _) = synchronization.accept().unwrap();
    native_completion
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let mut connected = [0; 5];
    native_completion.read_exact(&mut connected).unwrap();
    assert_eq!(&connected, b"ready");
    let cargo = fs::read_to_string(target.join(".chrono-harness/state/cargo-wrapper")).unwrap();
    let native = fs::read_to_string(target.join(".chrono-harness/state/native-pid")).unwrap();
    eprintln!("CARGO_HANDOFF launcher_pid={cargo} native_pid={native}");
    assert_ne!(
        cargo, native,
        "the killed launcher must be distinct from its live child"
    );
    let _group = OwnedGroup(cargo.clone());
    owner.kill().unwrap();
    owner.wait().unwrap();
    assert!(
        Command::new("/bin/kill")
            .args(["-KILL", &cargo])
            .status()
            .unwrap()
            .success()
    );
    native_completion.write_all(b"show").unwrap();
    let mut shown = [0; 5];
    native_completion.read_exact(&mut shown).unwrap();
    assert_eq!(
        &shown,
        b"shown",
        "native output after both launchers exited: {}",
        fs::read_to_string(target.join(".chrono-harness/state/native-io-error"))
            .unwrap_or_default()
    );
    assert_eq!(h.auto("maintain", &[]).0, 0);
    let preserved_while_native_live = target.join("output λ/cache").exists();
    let protected = maintain_allocation();
    assert!(cargo_output.is_dir(), "{protected}");
    assert_eq!(protected["status"], "protected", "{protected}");
    fs::write(target.join(".chrono-harness/state/native-release"), "join").unwrap();
    native_completion.write_all(b"join").unwrap();
    let mut done = [0; 4];
    native_completion
        .read_exact(&mut done)
        .expect("original native completion channel");
    assert_eq!(&done, b"done");
    assert!(target.join(".chrono-harness/state/native-done").exists());
    assert!(
        fs::read_to_string(target.join(".chrono-harness/state/native.stdout"))
            .unwrap()
            .contains("original native progress after launcher exit")
    );
    assert!(
        fs::read_to_string(target.join(".chrono-harness/state/native.stderr"))
            .unwrap()
            .contains("original native diagnostic after launcher exit")
    );
    assert!(
        preserved_while_native_live,
        "actual native Cargo child completed after cleanup but lost enrollment lease"
    );
    reclaim(&h, &target);
    let original_head = git(&target, &["rev-parse", "HEAD"]);
    let original_payload = fs::read(target.join("payload")).unwrap();
    let manifest = target.join(".chrono-harness/state/cargo-case/Cargo.toml");
    let library = target.join(".chrono-harness/state/cargo-case/src/lib.rs");
    let original_manifest = fs::read(&manifest).unwrap();
    let original_library = fs::read(&library).unwrap();
    // The target lifecycle must preserve source while this fixture's actual
    // assertions still consume it. The outer allocation owns these synthetic
    // sources and can roll as one directory only after their final assertion.
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), original_head);
    assert_eq!(fs::read(target.join("payload")).unwrap(), original_payload);
    assert_eq!(fs::read(manifest).unwrap(), original_manifest);
    assert_eq!(fs::read(library).unwrap(), original_library);
    drop(h);
    let reclaimed = maintain_allocation();
    assert!(!cargo_output.exists(), "{reclaimed}");
    assert!(
        !allocation.exists(),
        "released fixture rolls as a whole: {reclaimed}"
    );
}
