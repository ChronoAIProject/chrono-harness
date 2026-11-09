#[path = "../../runner-tests/tests/support/retention_state.rs"]
mod retention_state;
use super::*;
const AUTO_POLICY: &str = ".chrono-harness/cleanup.json";

// A metadata fixture with a fully mocked Git boundary. The absent checkout's
// adopted policy cannot be read, while its ledger still consumes an original.
struct EnrollmentOriginalsFixture {
    _directory: tempfile::TempDir,
    root: PathBuf,
    target: PathBuf,
    record: PathBuf,
    original: String,
    sibling: String,
}
impl EnrollmentOriginalsFixture {
    fn new() -> Self {
        use chrono_harness::retained_artifacts::retention::{Inventory, POLICY_PATH};
        let directory = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(directory.path()).unwrap();
        fs::create_dir(root.join(".git")).unwrap();
        let fixture = root.join("git-fixture.json");
        let mut registered = values();
        registered.get_mut(PROJECTS).unwrap()["owners"] = value!(["host"]);
        registered.get_mut(PROJECTS).unwrap()["projects"] = value!([]);
        registered.get_mut(FM).unwrap()["project_edges"] = value!([]);
        registered.get_mut(FM).unwrap()["test_costs"] = value!([]);
        for path in [POLICY, AUTO_POLICY, POLICY_PATH] {
            registered.get_mut(FM).unwrap()["files"]
                .as_array_mut()
                .unwrap()
                .push(file(path, value!([])));
        }
        write_values(&root, &registered);
        let write = |path: &str, value: Value| {
            fs::write(root.join(path), serde_json::to_vec(&value).unwrap()).unwrap();
        };
        write(
            "git-fixture.json",
            value!({"mode":"enrollment-originals","root":root}),
        );
        write(
            POLICY,
            value!({
                "schema":"chrono-worktree-config/v1","host_config":CONFIG,"remote":"unused",
                "git":{"program":env!("CARGO_BIN_EXE_chrono-worktree-test-git"),"expected_version":null,"sha256":null},
                "environment":{"inherit":[],"values":{"HOME":root,"CHRONO_TEST_GIT_FIXTURE":fixture}},
                "timeout_seconds":30,"output_limit_bytes":1048576,
                "report_directory":".chrono-harness/state/worktrees/","automatic_cleanup":AUTO_POLICY
            }),
        );
        write(
            AUTO_POLICY,
            value!({
                "schema":"chrono-worktree-automatic-cleanup/v1","coordinator_root":root,
                "state_directory":".chrono-harness/state/automatic-cleanup/",
                "retained_ref":"refs/heads/dev","retention":"ancestor","remove_branch":false,
                "allow_evidence_disposal":true,"artifacts":[]
            }),
        );
        write(
            POLICY_PATH,
            value!({
                "schema":"chrono-output-retention/v2",
                "producers":{"lifecycle-original":[".chrono-harness/state/"]},
                "keep_seconds":0,"max_related_outputs":512,"max_summaries":128,
                "max_nodes_per_round":256,"max_bytes_per_round":67108864,
                "max_millis_per_round":25,"max_output_bytes":1073741824,
                "max_manifest_nodes":16384,"max_metadata_bytes_per_round":67108864
            }),
        );
        let target = root.join("absent-checkout");
        let record = root
            .join(".chrono-harness/state/automatic-cleanup/enrollments")
            .join(sha256(target.to_str().unwrap().as_bytes()));
        fs::create_dir_all(&record).unwrap();
        let original = ".chrono-harness/state/birth-original.json".to_owned();
        let sibling = ".chrono-harness/state/released-sibling.json".to_owned();
        fs::write(
            record.join("owner.json"),
            serde_json::to_vec(&value!({
                "schema":"chrono-worktree-enrollment-owner/v1","coordinator_root":root,
                "common":root.join(".git"),"path":target
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            record.join("ledger.json"),
            serde_json::to_vec(&value!({
                "schema":"chrono-worktree-enrollment-state/v1","coordinator_root":root,
                "common":root.join(".git"),"entries":[{
                    "path":target,"branch":"integration/absent","status":"active",
                    "attachment":{"gitfile_sha256":"unknown","gitfile_id":"unknown",
                        "metadata":root.join(".git/worktrees/absent"),"metadata_id":"unknown"},
                    "policy_sha256":"old-policy",
                    "enrollment":{"kind":"birth","observed_head":"b".repeat(40),
                        "policy_inputs":{},"receipt":{"source_root":root,"report_path":original}},
                    "uses":[],"terminal":null,"attempts":[]
                }]
            }))
            .unwrap(),
        )
        .unwrap();
        for path in [&original, &sibling] {
            fs::write(root.join(path), b"original failure bytes\n").unwrap();
            Inventory::adopted(&root)
                .unwrap()
                .unwrap()
                .begin("lifecycle-original", path, value!({"status":"unknown"}))
                .unwrap()
                .complete(value!({"status":"failed","error":"original failure"}))
                .unwrap();
        }
        Self {
            _directory: directory,
            root,
            target,
            record,
            original,
            sibling,
        }
    }
    fn invoke(&self, operation: &str, select: bool) -> (i32, Value) {
        let mut args = vec![
            operation.into(),
            "--host-root".into(),
            self.root.to_str().unwrap().into(),
        ];
        if select {
            args.extend(["--path".into(), self.target.to_str().unwrap().into()]);
        }
        let output = chrono_worktree::run(&args);
        let report = json(output.stdout.as_bytes())
            .unwrap_or_else(|error| panic!("{error}: {} {}", output.stdout, output.stderr));
        (output.exit_code, report)
    }
}

#[test]
fn enrollment_originals_survive_failed_absent_policy_read() {
    let h = EnrollmentOriginalsFixture::new();
    let (code, report) = h.invoke("status", true);
    assert_eq!(code, 2, "{report}");
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("mock retained policy read failed"),
        "{report}"
    );
    let role = format!(
        "worktree-enrollment:{}",
        sha256(h.target.to_str().unwrap().as_bytes())
    );
    let state = retention_state::snapshot(&h.root);
    assert_eq!(
        state["roots"][&role].as_array().map(Vec::len),
        Some(1),
        "a policy failure must not prevent the verified ledger from protecting its original"
    );
}

#[test]
fn enrollment_originals_are_protected_before_global_maintenance() {
    let h = EnrollmentOriginalsFixture::new();
    let ledger = fs::read(h.record.join("ledger.json")).unwrap();
    let (code, report) = h.invoke("maintain", false);
    assert_eq!(code, 2, "{report}");
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("mock retained policy read failed"),
        "{report}"
    );
    assert_eq!(
        fs::read(h.root.join(&h.original)).unwrap(),
        b"original failure bytes\n"
    );
    assert!(
        !h.root.join(&h.sibling).exists(),
        "independent released originals can still be reclaimed: {report}"
    );
    assert_eq!(fs::read(h.record.join("ledger.json")).unwrap(), ledger);
}

#[test]
fn enrollment_originals_unknown_metadata_defers_only_global_original_maintenance() {
    let h = EnrollmentOriginalsFixture::new();
    fs::create_dir(h.record.join("gate.lock")).unwrap();
    let (code, report) = h.invoke("maintain", false);
    assert_eq!(code, 0, "{report}");
    assert!(
        h.root.join(&h.original).is_file(),
        "busy metadata has unverified consumers: {report}"
    );
    assert!(h.root.join(&h.sibling).is_file());
    assert_eq!(report["output_retention"]["status"], "preserved");
    assert!(
        report["drain"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["path"] == value!(h.target) && row["status"] == "preserved")
    );
}

impl Host {
    pub(super) fn automatic(&self, disposition: &str) {
        let mut config = json(&fs::read(self.root.join(CONFIG)).unwrap()).unwrap();
        for artifact in [
            value!({"path":"output λ/","owner":"host","kind":"arbitrary-output","tracked":false}),
            value!({"path":"nested/cache/","owner":"host","kind":"another-layout","tracked":false}),
        ] {
            if !config["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a["path"] == artifact["path"])
            {
                config["artifacts"].as_array_mut().unwrap().push(artifact);
            }
        }
        fs::write(
            self.root.join(CONFIG),
            serde_json::to_vec_pretty(&config).unwrap(),
        )
        .unwrap();
        let mut fm = json(&fs::read(self.root.join(FM)).unwrap()).unwrap();
        fm["files"]
            .as_array_mut()
            .unwrap()
            .push(file(AUTO_POLICY, value!([])));
        fs::write(self.root.join(FM), serde_json::to_vec_pretty(&fm).unwrap()).unwrap();
        fs::write(
            self.root.join(AUTO_POLICY),
            serde_json::to_vec_pretty(&value!({
                "schema":"chrono-worktree-automatic-cleanup/v1",
                "coordinator_root":self.root,
                "state_directory":".chrono-harness/state/automatic-cleanup/",
                "retained_ref":"refs/heads/dev", "retention":"ancestor", "remove_branch":false,
                "allow_evidence_disposal":true,
                "artifacts":[
                    {"path":"output λ/","disposition":"dispose"},
                    {"path":"nested/cache/","disposition":"dispose"},
                    {"path":".chrono-harness/state/","disposition":disposition},
                    {"path":".chrono-harness/bin/","disposition":"retain"}
                ]
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            self.root.join(".gitignore"),
            ".chrono-harness/state/\n.chrono-harness/bin/\noutput λ/\nnested/cache/\n",
        )
        .unwrap();
        self.policy(|p| p["automatic_cleanup"] = value!(AUTO_POLICY));
    }
    pub(super) fn auto_command(&self, command: &str, extra: &[&str]) -> Command {
        let mut c = Command::new(source().join("crates/worktree/target/debug/chrono-worktree"));
        c.current_dir(&self.root)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap())
            .args([
                command,
                "--host-root",
                self.root.to_str().unwrap(),
                "--config",
                POLICY,
            ])
            .args(extra);
        self._dir.bind(&mut c);
        c
    }
    pub(super) fn auto(&self, command: &str, extra: &[&str]) -> (i32, Value, String) {
        let mut command = self.auto_command(command, extra);
        self.received(self._dir.capture_bound(&mut command))
    }
    pub(super) fn ledger(&self) -> Value {
        snapshot(&self.root).unwrap()
    }
    pub(super) fn ledger_path(&self, target: &Path) -> PathBuf {
        self.record_directory(target).join("ledger.json")
    }
    pub(super) fn record_directory(&self, target: &Path) -> PathBuf {
        self.root
            .join(".chrono-harness/state/automatic-cleanup/enrollments")
            .join(sha256(target.to_str().unwrap().as_bytes()))
    }
    pub(super) fn set_ledger(&self, ledger: &Value) {
        let directory = self
            .root
            .join(".chrono-harness/state/automatic-cleanup/enrollments");
        for item in fs::read_dir(directory).unwrap() {
            let path = item.unwrap().path().join("ledger.json");
            let mut current = json(&fs::read(&path).unwrap()).unwrap();
            let marker =
                json(&fs::read(path.parent().unwrap().join("owner.json")).unwrap()).unwrap();
            current["entries"] = value!(
                ledger["entries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|entry| entry["path"] == marker["path"])
                    .cloned()
                    .collect::<Vec<_>>()
            );
            fs::write(path, serde_json::to_vec_pretty(&current).unwrap()).unwrap();
        }
    }
}
pub(super) fn snapshot(root: &Path) -> Result<Value, String> {
    let mut ledger = value!({"entries":[]});
    let namespace = root.join(".chrono-harness/state/automatic-cleanup/enrollments");
    for item in fs::read_dir(namespace).map_err(|e| e.to_string())? {
        let bytes = fs::read(item.map_err(|e| e.to_string())?.path().join("ledger.json"))
            .map_err(|e| e.to_string())?;
        let current = json(&bytes)?;
        ledger["entries"]
            .as_array_mut()
            .unwrap()
            .extend(current["entries"].as_array().unwrap().iter().cloned());
    }
    ledger["entries"]
        .as_array_mut()
        .unwrap()
        .sort_by_key(|entry| {
            entry["enrollment"]["receipt"]["report_path"]
                .as_str()
                .unwrap_or("")
                .split('-')
                .rev()
                .nth(1)
                .unwrap_or("")
                .to_owned()
        });
    Ok(ledger)
}

fn output(target: &Path) {
    fs::create_dir_all(target.join("output λ")).unwrap();
    fs::write(target.join("output λ/data"), vec![42u8; 8192]).unwrap();
}

fn capacity_fixture_with_diagnostics(
    host: &Host,
    directory: &Path,
    body: impl FnOnce(),
) -> std::thread::Result<()> {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));
    if result.is_err() {
        host.retain_fixture_diagnostics(directory);
    }
    result
}

#[test]
fn caught_capacity_failure_exports_original_inventory_and_command_streams() {
    let host = Host::new("payload");
    let destination = tempfile::tempdir().unwrap();
    let originals = [
        (
            "output-retention-v1/objects/output-diagnostic/record.json",
            b"original addressed declaration\0\xff".as_slice(),
        ),
        (
            "capacity-excluded.stdout",
            b"partial stdout\0\xfe".as_slice(),
        ),
        (
            "capacity-excluded.stderr",
            b"original capacity refusal\0\xfd".as_slice(),
        ),
        ("capacity-excluded.status", b"exit status: 2".as_slice()),
    ];
    for (name, bytes) in &originals {
        let path = host.root.join(".chrono-harness/state").join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    let source_bytes = fs::read(host.root.join("payload")).unwrap();
    let index = fs::read(host.root.join(".git/index")).unwrap();
    let result = capacity_fixture_with_diagnostics(&host, destination.path(), || {
        panic!("controlled original capacity fixture failure");
    });
    let failure = result.unwrap_err();
    assert_eq!(
        failure.downcast_ref::<&str>(),
        Some(&"controlled original capacity fixture failure")
    );
    let retained = destination.path().join(format!(
        "fixture-{}",
        sha256(host.root.as_os_str().as_encoded_bytes())
    ));
    for (name, bytes) in originals {
        assert_eq!(
            fs::read(retained.join("coordinator-state").join(name)).unwrap(),
            bytes
        );
        assert_eq!(
            fs::read(host.root.join(".chrono-harness/state").join(name)).unwrap(),
            bytes
        );
    }
    assert_eq!(fs::read(host.root.join("payload")).unwrap(), source_bytes);
    assert_eq!(fs::read(host.root.join(".git/index")).unwrap(), index);
}

// Exercise the actual lifecycle CLI with the adopted finite owner, not an
// alternate maintenance implementation. Failed cases retain their original host.
fn ordinary_report_capacity(operation: &str, all_rooted: bool) {
    use chrono_harness::retained_artifacts::retention::{Inventory, POLICY_PATH};
    let h = Host::new("payload");
    let directory = Host::diagnostic_directory(true).unwrap();
    let result = capacity_fixture_with_diagnostics(&h, &directory, || {
        h.kernel_cleanup();
        native_consumer(&h, "ordinary");
        let bootstrap = ".chrono-harness/ci/capacity.json";
        fs::create_dir_all(h.root.join(".chrono-harness/ci")).unwrap();
        fs::write(h.root.join(bootstrap), serde_json::to_vec(&value!({
            "schema":"chrono-bootstrap/v1", "participation":{"coordinator_root":"git-main-worktree"},
            "entrypoint":{"tool":"consumer","script":"noop"}
        })).unwrap()).unwrap();
        // The registered Rust interface fixture returns immediately for noop;
        // the real bootstrap participation dispatch still resolves and joins it.
        fs::write(h.root.join("noop"), include_bytes!("support/consumer.rs")).unwrap();
        fs::write(
            h.root.join(POLICY_PATH),
            serde_json::to_vec(&value!({
                "schema":"chrono-output-retention/v2",
                "producers":{"check-original":[".chrono-harness/state/"],
                    "lifecycle-original":[".chrono-harness/state/worktrees/"]},
                "keep_seconds":0,"max_related_outputs":512,"max_summaries":128,
                "max_nodes_per_round":256,"max_bytes_per_round":67108864,
                "max_millis_per_round":25,"max_output_bytes":1073741824,
                "max_manifest_nodes":16384,
                "max_metadata_bytes_per_round":67108864
            }))
            .unwrap(),
        )
        .unwrap();
        let mut fm = json(&fs::read(h.root.join(FM)).unwrap()).unwrap();
        for path in [POLICY_PATH, bootstrap, "noop"] {
            fm["files"]
                .as_array_mut()
                .unwrap()
                .push(file(path, value!([])));
        }
        fs::write(h.root.join(FM), serde_json::to_vec(&fm).unwrap()).unwrap();
        commit(&h.root);
        git(&h.root, &["push", "-q", "warehouse", "dev"]);
        let source_bytes = fs::read(h.root.join("payload")).unwrap();
        let index = fs::read(h.root.join(".git/index")).unwrap();
        let inventory = || Inventory::adopted(&h.root).unwrap().unwrap();
        let state = || retention_state::snapshot(&h.root);
        fs::create_dir_all(h.root.join(".chrono-harness/state/worktrees")).unwrap();
        let mut ids = Vec::new();
        let mut live = None;
        for n in 0..512 {
            let path = format!(".chrono-harness/state/worktrees/history-{n}.json");
            fs::write(h.root.join(&path), format!("original {n}\n")).unwrap();
            let publication = inventory()
                .begin(
                    "lifecycle-original",
                    &path,
                    value!({"status":"not-established"}),
                )
                .unwrap();
            ids.push(publication.id().to_owned());
            if n == 0 {
                live = Some(publication); // Actual unfinished producer capability.
            } else {
                publication
                    .complete(if n == 1 {
                        value!({"status":"failed","error":"original failure"})
                    } else if n == 2 {
                        value!({"status":"not-established","publication":"interrupted"})
                    } else {
                        value!({"status":"cleaned"})
                    })
                    .unwrap();
            }
        }
        inventory()
            .reference("consumer:lifecycle-recheck:failure", &[ids[1].clone()])
            .unwrap();
        inventory()
            .reference(
                "consumer:publication-recheck:interrupted",
                &[ids[2].clone()],
            )
            .unwrap();
        inventory()
            .reference("native-delivery:pending", &[ids[3].clone()])
            .unwrap();
        inventory()
            .reference("current-report:exact", &[ids[4].clone()])
            .unwrap();
        inventory()
            .dependencies(&ids[4], &[ids[5].clone()])
            .unwrap();
        inventory()
            .dependencies(&ids[6], &[ids[7].clone()])
            .unwrap();
        let reader = inventory().consume(&ids[6]).unwrap();
        if all_rooted {
            inventory()
                .reference("consumer:required-originals", &ids)
                .unwrap();
        }
        let before = state();
        assert_eq!(before["outputs"].as_array().unwrap().len(), 512);
        let invoke = |phase: &str| {
            let extra = if operation == "bootstrap" {
                vec!["--bootstrap-config", bootstrap]
            } else {
                vec![]
            };
            let mut command = h.auto_command(operation, &extra);
            let out = h._dir.capture_bound(&mut command);
            let dir = h.root.join(".chrono-harness/state");
            fs::write(dir.join(format!("capacity-{phase}.stdout")), &out.stdout).unwrap();
            fs::write(dir.join(format!("capacity-{phase}.stderr")), &out.stderr).unwrap();
            fs::write(
                dir.join(format!("capacity-{phase}.status")),
                out.status.to_string(),
            )
            .unwrap();
            h.received(out)
        };
        // This collector declares every original as a required input. Its
        // exact root prevents reclamation; an unrelated activity has no effect.
        inventory()
            .reference("collector-required-inputs", &ids)
            .unwrap();
        let (code, report, error) = invoke("active");
        assert_eq!(code, 0, "{report} {error}");
        for original in before["outputs"].as_array().unwrap() {
            assert!(state()["outputs"].as_array().unwrap().contains(original));
        }
        inventory()
            .reference("collector-required-inputs", &[])
            .unwrap();
        let (code, report, error) = invoke("excluded");
        assert_eq!(code, 0, "{report} {error}");
        assert_eq!(
            report["status"],
            if operation == "maintain" {
                "maintained"
            } else {
                "used"
            }
        );
        if operation == "bootstrap" {
            assert_eq!(report["managed_process"]["exit_code"], 0);
            assert_eq!(
                report["managed_process"]["argv"],
                value!([
                    env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"),
                    "noop",
                    h.root,
                    bootstrap
                ])
            );
        }
        // Only the explicit maintenance case consumes cross-object cleanup.
        // Bootstrap admission must leave all unrelated originals intact even
        // after the collector releases them; it has no global capacity gate.
        if operation == "maintain" {
            for _ in 0..8 {
                let round = inventory().maintain().unwrap();
                assert!(round["nodes"].as_u64().unwrap() <= 256);
                if all_rooted
                    || state()["summaries"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|s| ids.iter().skip(8).any(|id| s["id"] == *id))
                {
                    break;
                }
            }
            if !all_rooted {
                assert!(
                    state()["summaries"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|s| ids.iter().skip(8).any(|id| s["id"] == *id))
                );
            }
        }
        let after = state();
        let retained = if all_rooted || operation == "bootstrap" {
            512
        } else {
            8
        };
        for n in 0..retained {
            assert_eq!(
                fs::read(
                    h.root
                        .join(format!(".chrono-harness/state/worktrees/history-{n}.json"))
                )
                .unwrap(),
                format!("original {n}\n").as_bytes()
            );
            let original = before["outputs"]
                .as_array()
                .unwrap()
                .iter()
                .find(|o| o["id"] == ids[n])
                .unwrap();
            assert_eq!(
                after["outputs"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|o| o["id"] == ids[n])
                    .unwrap(),
                original
            );
        }
        assert_eq!(fs::read(h.root.join("payload")).unwrap(), source_bytes);
        assert_eq!(fs::read(h.root.join(".git/index")).unwrap(), index);
        drop(reader);
        drop(live);
    });
    if let Err(failure) = result {
        eprintln!(
            "original capacity fixture retained at {}",
            h.parent.display()
        );
        std::mem::forget(h);
        std::panic::resume_unwind(failure);
    }
}

#[test]
fn ordinary_report_capacity_maintenance_reclaims_only_eligible_originals() {
    ordinary_report_capacity("maintain", false);
}

#[test]
fn ordinary_report_capacity_bootstrap_admits_without_unrelated_maintenance() {
    ordinary_report_capacity("bootstrap", false);
}

#[test]
fn ordinary_report_capacity_maintenance_preserves_required_originals() {
    ordinary_report_capacity("maintain", true);
}

pub(super) fn inventory_row<'a>(report: &'a Value, target: &Path) -> &'a Value {
    report["worktree_inventory"]
        .as_array()
        .expect("shared drain must publish its Git inventory")
        .iter()
        .find(|row| row["path"] == value!(target))
        .expect("Git-registered worktree must be reported")
}

#[test]
fn automatic_inventory_command_failure_is_fatal_for_maintenance_and_admission() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_consumer(&h, "ordinary");
    native_git(&h, "inventory-failure", value!({}));
    let target = h.parent.join("inventory-failure");
    assert_eq!(h.invoke("feature", "inventory-failure", &target).0, 0);
    output(&target);
    let ledger = h.ledger();
    fs::write(h.parent.join("fail-inventory"), "fail").unwrap();
    for command in ["maintain", "use"] {
        let extra = if command == "use" {
            vec![
                "--path",
                target.to_str().unwrap(),
                "--operation",
                "use.consumer",
            ]
        } else {
            vec![]
        };
        let (code, report, error) = h.auto(command, &extra);
        assert_eq!(code, 2, "{report} {error}");
        assert!(
            report["error"]
                .as_str()
                .unwrap()
                .contains("original inventory failure")
        );
        assert_eq!(report["cleanup_failures"], value!([]));
        assert_eq!(report["worktree_inventory"], value!([]));
        assert!(report["managed_process"].is_null());
        assert_eq!(h.ledger(), ledger);
        assert!(target.join("output λ/data").exists());
    }
}

#[test]
fn automatic_inventory_preserves_ledgerless_dirty_locked_and_prunable_worktrees() {
    for fault in ["dirty", "locked", "prunable"] {
        let h = Host::new("payload");
        h.kernel_cleanup();
        let target = h.parent.join("unowned");
        git(
            &h.root,
            &[
                "worktree",
                "add",
                "-b",
                "unowned",
                target.to_str().unwrap(),
                "HEAD",
            ],
        );
        output(&target);
        fs::write(target.join("payload"), "unretained commit").unwrap();
        commit(&target);
        let head = git(&target, &["rev-parse", "HEAD"]);
        let gitfile = fs::read(target.join(".git")).unwrap();
        match fault {
            "dirty" => fs::write(target.join("payload"), "real dirty work").unwrap(),
            "locked" => {
                git(
                    &h.root,
                    &[
                        "worktree",
                        "lock",
                        "--reason",
                        "live external validation",
                        target.to_str().unwrap(),
                    ],
                );
            }
            _ => {
                fs::rename(&target, h.parent.join("preserved-missing-checkout")).unwrap();
            }
        }
        let (code, report, error) = h.auto("maintain", &[]);
        assert_eq!(code, 0, "{fault}: {report} {error}");
        let row = inventory_row(&report, &target);
        assert_eq!(row["head"], head);
        assert_eq!(row["branch"], "refs/heads/unowned");
        assert_eq!(row["enrollment_match"], false);
        assert!(
            row["preserved_reason"]
                .as_str()
                .unwrap()
                .contains("no current enrollment")
        );
        assert_eq!(row["locked"], fault == "locked");
        assert_eq!(row["prunable"], fault == "prunable");
        assert_eq!(
            row["status"],
            if fault == "prunable" {
                "blocked"
            } else {
                "preserved"
            }
        );
        assert_eq!(h.ledger()["entries"], value!([]));
        assert_eq!(git(&h.root, &["rev-parse", "refs/heads/unowned"]), head);
        let retained = if fault == "prunable" {
            h.parent.join("preserved-missing-checkout")
        } else {
            target.clone()
        };
        assert_eq!(fs::read(retained.join(".git")).unwrap(), gitfile);
        assert!(retained.join("output λ/data").exists());
        if fault == "dirty" {
            assert_eq!(
                fs::read_to_string(retained.join("payload")).unwrap(),
                "real dirty work"
            );
        }
        let main = inventory_row(&report, &h.root);
        assert_eq!(main["status"], "preserved");
        assert_eq!(main["preserved_reason"], "coordinator worktree");
    }
}

#[test]
fn automatic_inventory_requires_current_attachment_but_accepts_advanced_head() {
    for fault in ["advanced", "gitfile", "metadata", "unreadable"] {
        let h = Host::new("payload");
        h.kernel_cleanup();
        let target = h.parent.join("enrolled");
        assert_eq!(h.invoke("feature", "enrolled", &target).0, 0);
        output(&target);
        let metadata = PathBuf::from(git(&target, &["rev-parse", "--absolute-git-dir"]));
        match fault {
            "advanced" => {
                fs::write(target.join("payload"), "new committed source").unwrap();
                commit(&target);
            }
            "gitfile" => {
                fs::rename(target.join(".git"), h.parent.join("original-gitfile")).unwrap();
                fs::write(
                    target.join(".git"),
                    fs::read(h.parent.join("original-gitfile")).unwrap(),
                )
                .unwrap();
            }
            "metadata" => {
                let original = metadata.with_extension("original");
                fs::rename(&metadata, &original).unwrap();
                retain_fixture_state(&original, &metadata).unwrap();
            }
            _ => {
                fs::write(target.join(".git"), "unreadable attachment").unwrap();
            }
        }
        let ledger = h.ledger();
        let (code, report, error) = h.auto("maintain", &[]);
        let row = inventory_row(&report, &target);
        if fault == "advanced" {
            assert_eq!(code, 0, "{report} {error}");
            assert_eq!(row["status"], "enrolled");
            assert_eq!(row["enrollment_match"], true);
            assert_eq!(row["head"], git(&target, &["rev-parse", "HEAD"]));
            assert!(!target.join("output λ").exists());
            assert_eq!(h.ledger()["entries"][0]["status"], "active");
            assert!(h.ledger()["entries"][0]["terminal"].is_null());
        } else {
            assert_ne!(code, 0, "{fault}: {report} {error}");
            assert_eq!(row["status"], "blocked");
            assert_eq!(row["enrollment_match"], false);
            assert!(
                row["preserved_reason"]
                    .as_str()
                    .unwrap()
                    .contains("attachment")
            );
            assert!(target.join("output λ/data").exists());
            assert_eq!(
                h.ledger()["entries"][0]["attachment"],
                ledger["entries"][0]["attachment"]
            );
        }
    }
}

#[test]
fn independent_birth_has_no_dependency_on_another_worktrees_maintenance_original() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_consumer(&h, "ordinary");
    let first = h.parent.join("drained-first");
    let next = h.parent.join("consumer-next");
    assert_eq!(h.invoke("feature", "drained-first", &first).0, 0);
    output(&first);
    let (code, maintenance, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintenance} {error}");
    let input = &maintenance["drain"][0]["input"];
    let original_path = h.root.join(input["path"].as_str().unwrap());
    let original = fs::read(&original_path).unwrap();
    assert_eq!(sha256(&original), input["sha256"]);
    let (code, birth, error) = h.invoke("feature", "consumer-next", &next);
    assert_eq!(code, 0, "{birth} {error}");
    assert!(birth["drain"].as_array().unwrap().is_empty());
    let args = [
        "--path",
        next.to_str().unwrap(),
        "--operation",
        "use.consumer",
    ];
    for missing in [false, true] {
        if missing {
            fs::remove_file(&original_path).unwrap();
        } else {
            fs::write(&original_path, b"changed unrelated original").unwrap();
        }
        let (code, used, error) = h.auto("use", &args);
        assert_eq!(code, 0, "{used} {error}");
        assert_eq!(used["managed_process"]["stdout"], "ordinary-work");
        assert!(next.join("payload").exists());
        fs::write(&original_path, &original).unwrap();
    }
    assert_eq!(fs::read(&original_path).unwrap(), original);
}

#[test]
fn automatic_birth_enrollment_and_active_retained_checkout_survives_start() {
    let h = Host::new("some language/input.odd");
    h.automatic("evidence-retain");
    let first = h.parent.join("active");
    let (code, r, e) = h.invoke("feature", "active", &first);
    assert_eq!(code, 0, "{r} {e}");
    output(&first);
    let ledger = h.ledger();
    let entry = ledger["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["path"] == value!(first))
        .unwrap();
    assert_eq!(entry["status"], "active");
    assert_eq!(entry["enrollment"]["kind"], "birth");
    assert_eq!(h.invoke("feature", "next", &h.parent.join("next")).0, 0);
    assert!(first.join("output λ/data").exists());
}

fn attention_row<'a>(report: &'a Value, path: &Path) -> &'a Value {
    report["worktree_status"]["worktrees"]
        .as_array()
        .expect("persistent lifecycle status must be observable")
        .iter()
        .find(|row| row["path"] == value!(path))
        .expect("status must include the addressed worktree")
}

#[test]
fn automatic_attention_persists_across_commands_and_stops_after_verified_removal() {
    let h = Host::new("input.custom");
    h.automatic("evidence-retain");
    native_consumer(&h, "finished");
    let target = h.parent.join("attention active");
    let (code, birth, stderr) = h.invoke("integration", "attention", &target);
    assert_eq!(code, 0, "{birth} {stderr}");
    assert_eq!(attention_row(&birth, &target)["work_state"], "active");
    assert!(stderr.contains("WORKTREE_ATTENTION"), "{stderr}");

    let (code, status, stderr) = h.auto("status", &[]);
    assert_eq!(code, 0, "{status} {stderr}");
    assert_eq!(attention_row(&status, &target)["attention_required"], true);
    assert_eq!(attention_row(&status, &h.root)["attention_required"], false);
    let (code, used, stderr) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "{used} {stderr}");
    assert!(stderr.contains("WORKTREE_ATTENTION"), "{stderr}");
    let (code, finished, stderr) = h.auto(
        "finish",
        &["--path", target.to_str().unwrap(), "--dispose-evidence"],
    );
    assert_eq!(code, 0, "{finished} {stderr}");
    assert!(!target.exists());
    let (code, status, stderr) = h.auto("status", &[]);
    assert_eq!(code, 0, "{status} {stderr}");
    let row = attention_row(&status, &target);
    assert_eq!(row["cleanup_state"], "disposed");
    assert_eq!(row["work_state"], "terminal");
    assert_eq!(row["attention_required"], false);
    assert!(!stderr.contains("WORKTREE_ATTENTION"), "{stderr}");
}

#[test]
fn automatic_attention_linked_query_publishes_its_original_at_the_coordinator() {
    let h = Host::new("input.custom");
    h.automatic("evidence-retain");
    let target = h.parent.join("linked status");
    assert_eq!(h.invoke("feature", "linked-status", &target).0, 0);
    let mut command = Command::new(source().join("crates/worktree/target/debug/chrono-worktree"));
    command
        .current_dir(&target)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap())
        .args([
            "status",
            "--host-root",
            target.to_str().unwrap(),
            "--config",
            POLICY,
        ]);
    // received checks exact stdout against the original at source_root/report_path.
    let output = h.native_output(&mut command);
    let (code, report, stderr) = h.received(output);
    assert_eq!(code, 0, "{report} {stderr}");
    assert_eq!(report["source_root"], value!(h.root));
    assert_eq!(attention_row(&report, &target)["attention_required"], true);
    assert!(stderr.contains("WORKTREE_ATTENTION"), "{stderr}");
}

#[test]
fn automatic_attention_preserves_unregistered_dirty_and_corrupt_worktrees() {
    let h = Host::new("input.custom");
    h.automatic("evidence-retain");
    let healthy = h.parent.join("healthy");
    let corrupt = h.parent.join("corrupt");
    assert_eq!(h.invoke("feature", "healthy", &healthy).0, 0);
    assert_eq!(h.invoke("feature", "corrupt", &corrupt).0, 0);
    let unknown = h.parent.join("unknown");
    git(
        &h.root,
        &[
            "worktree",
            "add",
            "--detach",
            unknown.to_str().unwrap(),
            "HEAD",
        ],
    );
    fs::write(unknown.join("unique-work"), "keep this source").unwrap();
    fs::write(h.ledger_path(&corrupt), b"broken original ledger").unwrap();
    let (code, status, stderr) = h.auto("status", &[]);
    assert_eq!(code, 0, "{status} {stderr}");
    assert_eq!(attention_row(&status, &healthy)["work_state"], "active");
    assert_eq!(attention_row(&status, &corrupt)["work_state"], "unknown");
    assert_eq!(
        attention_row(&status, &unknown)["work_state"],
        "unregistered"
    );
    assert_eq!(
        fs::read(unknown.join("unique-work")).unwrap(),
        b"keep this source"
    );
    assert_eq!(
        fs::read(h.ledger_path(&corrupt)).unwrap(),
        b"broken original ledger"
    );
    assert!(
        !h.ledger_path(&unknown).exists(),
        "observation must not import work"
    );
}

#[test]
fn automatic_attention_keeps_retained_terminal_work_visible_and_scopes_ordinary_commands() {
    let h = Host::new("input.custom");
    h.automatic("evidence-retain");
    let target = h.parent.join("retained");
    let other = h.parent.join("other");
    assert_eq!(h.invoke("feature", "retained", &target).0, 0);
    fs::create_dir_all(target.join(".chrono-harness/state")).unwrap();
    fs::write(
        target.join(".chrono-harness/state/evidence"),
        "required evidence",
    )
    .unwrap();
    let (code, report, stderr) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{report} {stderr}");
    assert!(target.exists());
    let row = attention_row(&report, &target);
    assert_eq!(row["work_state"], "terminal");
    assert_eq!(row["cleanup_state"], "retained");
    assert_eq!(row["attention_required"], true);
    assert!(stderr.contains("WORKTREE_ATTENTION"), "{stderr}");
    let (code, report, stderr) = h.invoke("feature", "other", &other);
    assert_eq!(code, 0, "{report} {stderr}");
    assert_eq!(
        report["worktree_status"]["worktrees"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(attention_row(&report, &other)["work_state"], "active");
    let (code, report, stderr) = h.auto("status", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{report} {stderr}");
    assert_eq!(
        report["worktree_status"]["worktrees"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(attention_row(&report, &target)["attention_required"], true);
}

#[test]
fn automatic_finish_absent_evidence_does_not_retain_the_whole_checkout() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    let target = h.parent.join("no-evidence");
    assert_eq!(h.invoke("integration", "no-evidence", &target).0, 0);
    output(&target);
    assert!(!target.join(".chrono-harness/state").exists());
    let (code, report, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(
        !target.exists(),
        "absent evidence cannot keep an otherwise disposable checkout"
    );
    assert_eq!(
        drain_report(&report["drain"][0])["worktree_removal"],
        "verified-absent"
    );
    assert_eq!(h.ledger()["entries"][0]["status"], "disposed");
    assert_eq!(h.ledger()["entries"][0]["cleanup"]["stage"], "disposed");
    assert!(!git(&h.root, &["worktree", "list", "--porcelain"]).contains(target.to_str().unwrap()));
}

#[test]
fn automatic_maintenance_resumes_retained_checkout_after_evidence_release() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    let target = h.parent.join("released-evidence");
    assert_eq!(h.invoke("integration", "released-evidence", &target).0, 0);
    let evidence = target.join(".chrono-harness/state");
    fs::create_dir_all(&evidence).unwrap();
    fs::write(
        evidence.join("consumer-original"),
        "required until consumed",
    )
    .unwrap();
    output(&target);
    let (code, report, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(target.exists());
    assert_eq!(h.ledger()["entries"][0]["status"], "retained");
    let progress = h.ledger()["entries"][0]["cleanup"].clone();
    assert_eq!(progress["stage"], "retained");
    assert!(progress["detail"].as_str().unwrap().contains("evidence"));
    assert_eq!(
        progress["report_path"],
        report["drain"][0]["receipt"]["path"]
    );
    let original = entry_receipts(&h, &h.ledger()["entries"][0]);
    // The fixture's sole evidence consumer finishes and releases its exact files.
    fs::remove_file(evidence.join("consumer-original")).unwrap();
    fs::remove_dir(&evidence).unwrap();
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(
        !target.exists(),
        "retained checkout must remain discoverable for later cleanup"
    );
    assert_eq!(
        drain_report(&report["drain"][0])["worktree_removal"],
        "verified-absent"
    );
    assert_eq!(h.ledger()["entries"][0]["status"], "disposed");
    for (path, bytes) in original {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn retained_checkout_keeps_its_cleanup_original_until_the_responsibility_is_released() {
    use chrono_harness::retained_artifacts::retention::Inventory;
    let h = Host::new("payload");
    h.kernel_cleanup();
    adopt_lifecycle_retention(&h);
    // Retention itself owns metadata under state/. The fixture consumer owns
    // this separate evidence path and can release all of it without deleting
    // another producer's metadata or weakening the original-receipt assertion.
    // Target-local retention metadata has no retained caller after cleanup; the
    // ordinary disposal owner must verify its release before removing the tree.
    let policy_path = h.root.join(AUTO_POLICY);
    let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
    for artifact in policy["artifacts"].as_array_mut().unwrap() {
        if artifact["path"] == ".chrono-harness/state/" {
            artifact["disposition"] = value!("dispose");
        }
    }
    policy["artifacts"].as_array_mut().unwrap().push(value!({
        "path":"consumer-evidence/", "disposition":"evidence-retain"
    }));
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let mut config = json(&fs::read(h.root.join(CONFIG)).unwrap()).unwrap();
    config["artifacts"].as_array_mut().unwrap().push(value!({
        "path":"consumer-evidence/", "owner":"host", "kind":"evidence", "tracked":false
    }));
    fs::write(h.root.join(CONFIG), serde_json::to_vec(&config).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("retained-original");
    let (code, created, error) = h.invoke("integration", "retained-original", &target);
    assert_eq!(code, 0, "{created} {error}");
    let evidence = target.join("consumer-evidence");
    fs::create_dir_all(&evidence).unwrap();
    fs::write(
        evidence.join("needed"),
        "consumer has not released this original",
    )
    .unwrap();
    output(&target);
    let (code, finished, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{finished} {error}");
    let entry = h.ledger()["entries"][0].clone();
    assert_eq!(entry["status"], "retained");
    assert_eq!(entry["cleanup"]["stage"], "retained");
    let receipt = &entry["attempts"][0]["receipt"];
    let path = h.root.join(receipt["path"].as_str().unwrap());
    let original = fs::read(&path).unwrap();
    assert_eq!(receipt["sha256"], sha256(&original));
    let inventory = Inventory::adopted(&h.root).unwrap().unwrap();
    let maintenance = inventory.maintain().unwrap();
    assert!(
        path.is_file(),
        "an unresolved checkout still consumes its original cleanup result: {maintenance}"
    );
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(target.join("payload").is_file());
    assert_eq!(h.ledger()["entries"][0], entry);
    // The sole fixture consumer now releases the evidence; the next entry must
    // be able to read the preserved result and finish the physical cleanup.
    fs::remove_file(evidence.join("needed")).unwrap();
    fs::remove_dir(&evidence).unwrap();
    let (code, resumed, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{resumed} {error}");
    assert!(!target.exists(), "{resumed} {error}");
    assert_eq!(h.ledger()["entries"][0]["status"], "disposed");
    assert!(!git(&h.root, &["worktree", "list", "--porcelain"]).contains(target.to_str().unwrap()));
}

#[test]
fn creator_agent_and_pending_reminders_follow_the_original_creator() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    h.policy(|policy| {
        policy["agent_identity_env"] = value!("CHRONO_TEST_AGENT_ID");
        policy["environment"]["inherit"]
            .as_array_mut()
            .unwrap()
            .push(value!("CHRONO_TEST_AGENT_ID"));
    });
    let invoke = |operation: &str, extra: &[&str], agent: Option<&str>| {
        let mut command = h.auto_command(operation, extra);
        if let Some(agent) = agent {
            command.env("CHRONO_TEST_AGENT_ID", agent);
        }
        let output = h._dir.capture_bound(&mut command);
        h.received(output)
    };
    let a = h.parent.join("agent-a-work");
    let b = h.parent.join("agent-b-work");
    let unknown = h.parent.join("unknown-agent-work");
    for (name, path, agent) in [
        ("agent-a", &a, Some("agent-A")),
        ("agent-b", &b, Some("agent-B")),
        ("unknown-agent", &unknown, None),
    ] {
        let (code, created, error) = invoke(
            "start",
            &[
                "--kind",
                "integration",
                "--name",
                name,
                "--path",
                path.to_str().unwrap(),
            ],
            agent,
        );
        assert_eq!(code, 0, "{created} {error}");
        if let Some(agent) = agent {
            assert_eq!(created["agent"]["identity"]["id"], agent);
            let intent = json(
                &fs::read(
                    h.root
                        .join(created["recovery_intent"]["path"].as_str().unwrap()),
                )
                .unwrap(),
            )
            .unwrap();
            assert_eq!(intent["creator_agent"]["id"], agent);
        } else {
            assert_eq!(created["agent"]["status"], "unknown");
        }
    }
    let a_entry = h.ledger()["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["path"] == value!(a))
        .unwrap()
        .clone();
    assert_eq!(a_entry["creator_agent"]["id"], "agent-A");
    let imported = h.parent.join("imported-agent-work");
    git(
        &h.root,
        &[
            "worktree",
            "add",
            "-qb",
            "external-agent-work",
            imported.to_str().unwrap(),
            "dev",
        ],
    );
    let (code, imported_report, error) = invoke(
        "import",
        &["--path", imported.to_str().unwrap()],
        Some("agent-B"),
    );
    assert_eq!(code, 0, "{imported_report} {error}");
    let imported_entry = h.ledger()["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["path"] == value!(imported))
        .unwrap()
        .clone();
    assert!(imported_entry["creator_agent"].is_null());
    assert_eq!(imported_entry["custodian_agent"]["id"], "agent-B");
    let (code, maintained, error) = invoke("maintain", &[], Some("agent-A"));
    assert_eq!(code, 0, "{maintained} {error}");
    let reminders: Vec<_> = maintained["worktree_status"]["worktrees"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["agent_attention_required"] == true)
        .collect();
    assert_eq!(reminders.len(), 1, "{maintained}");
    assert_eq!(reminders[0]["path"], value!(a));
    assert_eq!(reminders[0]["work_state"], "active");
    assert!(error.contains(a.to_str().unwrap()), "{error}");
    assert!(!error.contains(b.to_str().unwrap()), "{error}");
    assert!(
        maintained["worktree_status"]["unknown_creator_records"]
            .as_u64()
            .unwrap()
            >= 1
    );
    let unregistered = h.parent.join("agent-unregistered");
    git(
        &h.root,
        &[
            "worktree",
            "add",
            "--detach",
            unregistered.to_str().unwrap(),
            "dev",
        ],
    );
    // A related operation on B must also remind A about its own outstanding A.
    // Git addresses without any enrollment keep the observation incomplete.
    let (code, scoped, error) = invoke("status", &["--path", b.to_str().unwrap()], Some("agent-A"));
    assert_eq!(code, 0, "{scoped} {error}");
    assert_eq!(attention_row(&scoped, &a)["agent_attention_required"], true);
    assert_eq!(
        attention_row(&scoped, &b)["agent_attention_required"],
        false
    );
    assert!(error.contains(a.to_str().unwrap()), "{error}");
    assert!(!error.contains(b.to_str().unwrap()), "{error}");
    assert_eq!(
        scoped["worktree_status"]["agent_observation_complete"],
        false
    );
    assert_eq!(
        scoped["worktree_status"]["unknown_creator_records"]
            .as_u64()
            .unwrap(),
        maintained["worktree_status"]["unknown_creator_records"]
            .as_u64()
            .unwrap()
            + 1
    );
    let (code, unknown_scope, error) = invoke("status", &["--path", b.to_str().unwrap()], None);
    assert_eq!(code, 0, "{unknown_scope} {error}");
    assert_eq!(
        unknown_scope["worktree_status"]["agent"]["status"],
        "unknown"
    );
    assert!(error.contains("W_WORKTREE_AGENT_SCOPE"), "{error}");
    let (code, finished, error) =
        invoke("finish", &["--path", a.to_str().unwrap()], Some("agent-B"));
    assert_eq!(code, 0, "{finished} {error}");
    assert!(!a.exists());
    let a_entry = h.ledger()["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["path"] == value!(a))
        .unwrap()
        .clone();
    assert_eq!(a_entry["creator_agent"]["id"], "agent-A");
    let (code, maintained, error) = invoke("maintain", &[], Some("agent-A"));
    assert_eq!(code, 0, "{maintained} {error}");
    assert!(
        maintained["worktree_status"]["worktrees"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["agent_attention_required"] != true)
    );
    assert!(b.join("payload").is_file());
    assert!(unknown.join("payload").is_file());
}

#[test]
fn terminal_history_rotation_preserves_current_work_and_releases_only_disposed_records() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    let policy_path = h.root.join(AUTO_POLICY);
    let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
    policy["terminal_history"] = value!({"keep_seconds":0,"max_entries":2,"max_bytes":1048576});
    fs::write(&policy_path, serde_json::to_vec_pretty(&policy).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let active = h.parent.join("still-active");
    let target = h.parent.join("rotating-history");
    let (code, created, error) = h.invoke("integration", "still-active", &active);
    assert_eq!(code, 0, "{created} {error}");
    let active_entry = h.ledger()["entries"][0].clone();
    let (code, created, error) = h.invoke("integration", "rotating-history", &target);
    assert_eq!(code, 0, "{created} {error}");
    let (code, finished, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{finished} {error}");
    assert!(!target.exists());
    let disposed = h.ledger()["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["path"] == value!(target))
        .unwrap()
        .clone();
    assert_eq!(disposed["status"], "disposed");
    assert!(disposed["disposed_at"].as_u64().is_some());
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    let remaining = h.ledger();
    assert_eq!(remaining["entries"].as_array().unwrap().len(), 1);
    assert_eq!(remaining["entries"][0]["path"], active_entry["path"]);
    assert_eq!(
        remaining["entries"][0]["attachment"],
        active_entry["attachment"]
    );
    assert_eq!(remaining["entries"][0]["status"], "active");
    assert!(remaining["entries"][0]["terminal"].is_null());
    assert!(active.join("payload").is_file());
    assert!(!target.exists());
    assert!(
        !h.root
            .join(disposed["ownership"]["path"].as_str().unwrap())
            .exists(),
        "retired attachment capabilities must not outlive their final responsibility"
    );
    assert!(
        !h.record_directory(&target).exists(),
        "an empty retired address must release its metadata and directory"
    );
    assert!(!git(&h.root, &["worktree", "list", "--porcelain"]).contains(target.to_str().unwrap()));
    let (code, again, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{again} {error}");
    assert_eq!(h.ledger()["entries"].as_array().unwrap().len(), 1);
}

#[test]
fn retired_address_preserves_directory_consumers_and_unknown_members() {
    use chrono_harness::ownership::DirectoryLease;
    let h = Host::new("payload");
    h.kernel_cleanup();
    let policy_path = h.root.join(AUTO_POLICY);
    let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
    policy["terminal_history"] = value!({"keep_seconds":0,"max_entries":1,"max_bytes":1048576});
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("retirement-boundaries");
    let (code, created, error) = h.invoke("integration", "retirement-boundaries", &target);
    assert_eq!(code, 0, "{created} {error}");
    let (code, finished, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{finished} {error}");
    let directory = h.record_directory(&target);
    let before = fs::read(directory.join("ledger.json")).unwrap();
    let reader = DirectoryLease::acquire(&directory, false, None)
        .unwrap()
        .unwrap();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert_eq!(fs::read(directory.join("ledger.json")).unwrap(), before);
    drop(reader);
    let foreign = directory.join("unknown-work");
    fs::write(&foreign, b"retain this independent work").unwrap();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 2, "{maintained} {error}");
    assert_eq!(fs::read(&foreign).unwrap(), b"retain this independent work");
    assert!(directory.join("owner.json").is_file());
    fs::remove_file(foreign).unwrap();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert!(!directory.exists());
}

#[test]
fn interrupted_address_retirement_preserves_changed_members_then_resumes_original_identity() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    let policy_path = h.root.join(AUTO_POLICY);
    let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
    policy["terminal_history"] = value!({"keep_seconds":0,"max_entries":1,"max_bytes":1048576});
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("interrupted-retirement");
    let (code, created, error) = h.invoke("integration", "interrupted-retirement", &target);
    assert_eq!(code, 0, "{created} {error}");
    let (code, finished, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{finished} {error}");
    let directory = h.record_directory(&target);
    fs::write(directory.join("fixture-consumer"), b"owned by this fixture").unwrap();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 2, "{maintained} {error}");
    assert!(h.ledger()["entries"].as_array().unwrap().is_empty());
    fs::remove_file(directory.join("fixture-consumer")).unwrap();
    let members: Vec<_> = ["ledger.json", "owner.json", "metadata.lease"]
        .iter()
        .map(|name| {
            let path = directory.join(name);
            let bytes = fs::read(&path).unwrap();
            value!({"name":name,"identity":chrono_harness::ownership::identity(&path).unwrap(),
            "sha256":sha256(&bytes),"length":bytes.len()})
        })
        .collect();
    let declaration = value!({"schema":"chrono-worktree-address-retirement/v1",
        "coordinator_root":h.root,"common":h.root.join(".git"),"path":target,
        "directory_identity":chrono_harness::artifact_disposal::identity(&directory).unwrap(),
        "admission_id":members[2]["identity"],"members":members});
    fs::write(
        directory.join("retirement.json"),
        serde_json::to_vec(&declaration).unwrap(),
    )
    .unwrap();
    // A real unlink happened before interruption. Recovery must accept this
    // partial effect without accepting another member's changed bytes.
    fs::remove_file(directory.join("ledger.json")).unwrap();
    let owner = directory.join("owner.json");
    let original = fs::read(&owner).unwrap();
    fs::write(&owner, [original.as_slice(), b" "].concat()).unwrap();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 2, "{maintained} {error}");
    assert!(directory.join("retirement.json").is_file());
    assert!(directory.join("metadata.lease").is_file());
    assert_eq!(
        fs::read(&owner).unwrap(),
        [original.as_slice(), b" "].concat()
    );
    fs::write(&owner, original).unwrap();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert!(!directory.exists());
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
}

#[test]
fn retired_capability_failure_keeps_the_creators_original_responsibility_visible() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    h.policy(|policy| {
        policy["agent_identity_env"] = value!("CHRONO_TEST_AGENT_ID");
        policy["environment"]["inherit"]
            .as_array_mut()
            .unwrap()
            .push(value!("CHRONO_TEST_AGENT_ID"));
    });
    let policy_path = h.root.join(AUTO_POLICY);
    let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
    policy["terminal_history"] = value!({"keep_seconds":0,"max_entries":1,"max_bytes":1048576});
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let invoke = |operation: &str, extra: &[&str]| {
        let mut command = h.auto_command(operation, extra);
        command.env("CHRONO_TEST_AGENT_ID", "retirement-agent-A");
        h.received(h._dir.capture_bound(&mut command))
    };
    let target = h.parent.join("retired-capability");
    let (code, created, error) = invoke(
        "start",
        &[
            "--kind",
            "integration",
            "--name",
            "retired-capability",
            "--path",
            target.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "{created} {error}");
    let lease = h.root.join(
        h.ledger()["entries"][0]["ownership"]["path"]
            .as_str()
            .unwrap(),
    );
    let (code, finished, error) = invoke("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{finished} {error}");
    let identity = chrono_harness::ownership::identity(&lease).unwrap();
    fs::write(&lease, b"unknown capability bytes").unwrap();
    let (code, maintained, error) = invoke("maintain", &[]);
    assert_eq!(code, 2, "{maintained} {error}");
    let (code, status, error) = invoke("status", &["--path", h.root.to_str().unwrap()]);
    assert_eq!(code, 0, "{status} {error}");
    let pending = status["worktree_status"]["worktrees"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["record_kind"] == "retired-lease")
        .unwrap();
    assert_eq!(pending["creator_agent"]["id"], "retirement-agent-A");
    assert_eq!(pending["branch"], "integration/retired-capability");
    assert_eq!(pending["agent_attention_required"], true);
    assert_eq!(pending["next_action"], "maintain-retired-lease");
    assert!(pending["error"].as_str().unwrap().contains("unknown bytes"));
    assert!(error.contains(target.to_str().unwrap()), "{error}");
    assert_eq!(fs::read(&lease).unwrap(), b"unknown capability bytes");
    assert_eq!(
        chrono_harness::ownership::identity(&lease).unwrap(),
        identity
    );
    fs::write(&lease, b"").unwrap();
    let (code, maintained, error) = invoke("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert!(!lease.exists());
    assert!(!h.record_directory(&target).exists());
}

#[test]
fn terminal_history_count_and_byte_limits_release_verified_generations_before_the_ttl() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    let policy_path = h.root.join(AUTO_POLICY);
    let set_history = |bytes| {
        let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
        policy["terminal_history"] =
            value!({"keep_seconds":86400,"max_entries":1,"max_bytes":bytes});
        fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
        commit(&h.root);
        git(&h.root, &["push", "-q", "warehouse", "dev"]);
    };
    set_history(1048576);
    let target = h.parent.join("reused-history-address");
    for name in ["history-first", "history-second"] {
        let (code, created, error) = h.invoke("integration", name, &target);
        assert_eq!(code, 0, "{created} {error}");
        let (code, finished, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
        assert_eq!(code, 0, "{finished} {error}");
        assert!(!target.exists());
    }
    assert_eq!(h.ledger()["entries"].as_array().unwrap().len(), 2);
    let newest = h.ledger()["entries"][1].clone();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert_eq!(h.ledger()["entries"], value!([newest]));
    set_history(1);
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert!(h.ledger()["entries"].as_array().unwrap().is_empty());
    assert!(!target.exists());
    assert!(git(&h.root, &["show-ref"]).contains("refs/heads/integration/history-first"));
    assert!(git(&h.root, &["show-ref"]).contains("refs/heads/integration/history-second"));
}

#[test]
fn automatic_originals_follow_enrollment_and_report_consumers_after_history_rotation() {
    use chrono_harness::retained_artifacts::retention::Inventory;
    let h = Host::new("payload");
    h.kernel_cleanup();
    adopt_lifecycle_retention(&h);
    let retention_path = h
        .root
        .join(chrono_harness::retained_artifacts::retention::POLICY_PATH);
    let mut retention = json(&fs::read(&retention_path).unwrap()).unwrap();
    retention["producers"]["lifecycle-original"] = value!([
        ".chrono-harness/state/worktrees/",
        ".chrono-harness/state/automatic-cleanup/"
    ]);
    retention["max_related_outputs"] = value!(512);
    fs::write(&retention_path, serde_json::to_vec(&retention).unwrap()).unwrap();
    let policy_path = h.root.join(AUTO_POLICY);
    let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
    policy["terminal_history"] = value!({"keep_seconds":0,"max_entries":1,"max_bytes":1048576});
    for artifact in policy["artifacts"].as_array_mut().unwrap() {
        if artifact["path"] == ".chrono-harness/state/" {
            artifact["disposition"] = value!("dispose");
        }
    }
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("finite-originals");
    let (code, created, error) = h.invoke("integration", "finite-originals", &target);
    assert_eq!(code, 0, "{created} {error}");
    let birth = h.ledger()["entries"][0]["enrollment"]["sealed_receipt"]["path"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        Inventory::adopted(&h.root)
            .unwrap()
            .unwrap()
            .enrolled(&birth)
            .unwrap(),
        "the automatic producer must enroll its own original, not leave an unbounded file"
    );
    Inventory::adopted(&h.root)
        .unwrap()
        .unwrap()
        .maintain()
        .unwrap();
    assert!(
        h.root.join(&birth).is_file(),
        "active enrollment still consumes birth"
    );
    let (code, finished, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{finished} {error}");
    assert!(!target.exists());
    let terminal = h.ledger()["entries"][0]["terminal"]["receipt"]["path"]
        .as_str()
        .unwrap()
        .to_owned();
    let finished_path = finished["report_path"].as_str().unwrap();
    let inventory = Inventory::adopted(&h.root).unwrap().unwrap();
    inventory
        .reference_paths("fixture-finish-consumer", &[finished_path.into()])
        .unwrap();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert!(h.ledger()["entries"].as_array().unwrap().is_empty());
    inventory.maintain().unwrap();
    assert!(
        !h.root.join(&birth).exists(),
        "unconsumed birth must leave with history"
    );
    assert!(
        h.root.join(&terminal).is_file(),
        "finish report still consumes terminal input"
    );
    inventory
        .reference_paths("fixture-finish-consumer", &[])
        .unwrap();
    maintain_original_until_absent(&inventory, &h.root.join(&terminal));
    assert!(
        !h.root.join(&terminal).exists(),
        "released terminal original must be reclaimed"
    );
    assert!(git(&h.root, &["show-ref"]).contains("refs/heads/integration/finite-originals"));
}

fn maintain_original_until_absent(
    inventory: &chrono_harness::retained_artifacts::retention::Inventory,
    path: &Path,
) {
    let mut rounds = Vec::new();
    for _ in 0..8 {
        let round = inventory.maintain().unwrap();
        // 只核本用例的原件；无关对象的保留失败仍留在每轮原始诊断中。
        rounds.push(round);
        if !path.exists() {
            return;
        }
    }
    panic!(
        "bounded original disposal did not finish: {} {rounds:?}",
        path.display()
    );
}

#[test]
fn managed_result_handoff_preserves_partial_reports_then_releases_the_interim_reference() {
    use chrono_harness::retained_artifacts::retention::{Inventory, POLICY_PATH};
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_consumer(&h, "ordinary");
    adopt_lifecycle_retention(&h);
    let policy_path = h.root.join(POLICY_PATH);
    let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
    policy["producers"]["lifecycle-original"] = value!([
        ".chrono-harness/state/worktrees/",
        ".chrono-harness/state/automatic-cleanup/"
    ]);
    policy["max_related_outputs"] = value!(512);
    fs::write(policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let cleanup_path = h.root.join(AUTO_POLICY);
    let mut cleanup = json(&fs::read(&cleanup_path).unwrap()).unwrap();
    cleanup["terminal_history"] = value!({"keep_seconds":0,"max_entries":1,"max_bytes":1048576});
    fs::write(cleanup_path, serde_json::to_vec(&cleanup).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("pending-result-handoff");
    let (code, created, error) = h.invoke("integration", "pending-result-handoff", &target);
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
    let receipt = used["managed_use_receipt"].clone();
    let raw_path = h.root.join(receipt["path"].as_str().unwrap());
    let raw = fs::read(&raw_path).unwrap();
    assert_eq!(
        h.ledger()["entries"][0]["enrollment"]["completed_uses"],
        value!([receipt])
    );
    let report_path = h.root.join(used["report_path"].as_str().unwrap());
    let report_bytes = fs::read(&report_path).unwrap();
    let inventory = Inventory::adopted(&h.root).unwrap().unwrap();
    inventory.maintain().unwrap();
    assert_eq!(
        fs::read(&report_path).unwrap(),
        report_bytes,
        "the pending handoff still consumes its final report before the next worktree command"
    );
    inventory
        .reference_paths(
            "fixture-use-report",
            &[used["report_path"].as_str().unwrap().into()],
        )
        .unwrap();
    // Fault injection at the final report boundary: a partial JSON original
    // cannot discharge the pending handoff or establish a command outcome.
    fs::write(&report_path, b"{\"status\":").unwrap();
    let (code, observed, error) = h.auto("status", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{observed} {error}");
    assert_eq!(fs::read(&raw_path).unwrap(), raw);
    assert_eq!(
        h.ledger()["entries"][0]["enrollment"]["completed_uses"],
        value!([receipt])
    );
    let mut changed = json(&report_bytes).unwrap();
    changed["untrusted_edit"] = value!(true);
    fs::write(&report_path, serde_json::to_vec(&changed).unwrap()).unwrap();
    let (code, observed, error) = h.auto("status", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{observed} {error}");
    assert_eq!(
        h.ledger()["entries"][0]["enrollment"]["completed_uses"],
        value!([receipt]),
        "matching JSON fields do not replace the sealed producer bytes"
    );
    let (code, finished, error) = h.auto(
        "finish",
        &["--path", target.to_str().unwrap(), "--dispose-evidence"],
    );
    assert_eq!(code, 0, "{finished} {error}");
    assert!(!target.exists());
    let (_code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(
        h.ledger()["entries"].as_array().unwrap().len(),
        1,
        "an unresolved report handoff must survive disposed-history rotation: {maintained} {error}"
    );
    assert_eq!(
        h.ledger()["entries"][0]["enrollment"]["completed_uses"],
        value!([receipt])
    );
    let row = maintained["worktree_status"]["worktrees"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == value!(target))
        .unwrap();
    assert_eq!(row["work_state"], "terminal");
    assert_eq!(row["report_handoff_pending"], true);
    assert_eq!(row["attention_required"], true);
    assert_eq!(row["next_action"], "recover-result-handoff");
    fs::write(&report_path, &report_bytes).unwrap();
    let (code, observed, error) = h.auto("status", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{observed} {error}");
    assert!(h.ledger()["entries"][0]["enrollment"]["completed_uses"].is_null());
    inventory.maintain().unwrap();
    assert_eq!(
        fs::read(&raw_path).unwrap(),
        raw,
        "report's actual consumer still needs the result"
    );
    inventory
        .reference_paths("fixture-use-report", &[])
        .unwrap();
    maintain_original_until_absent(&inventory, &raw_path);
    assert!(
        !raw_path.exists(),
        "successful report handoff must not create permanent roots"
    );
    assert!(!target.exists());
}

#[test]
fn terminal_history_rotation_preserves_live_consumers_and_changed_originals() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    let policy_path = h.root.join(AUTO_POLICY);
    let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
    policy["terminal_history"] = value!({"keep_seconds":0,"max_entries":1,"max_bytes":1048576});
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("protected-history");
    assert_eq!(h.invoke("integration", "protected-history", &target).0, 0);
    let (code, finished, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{finished} {error}");
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
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_ne!(code, 0, "{maintained} {error}");
    assert_eq!(h.ledger()["entries"], value!([entry.clone()]));
    drop(lease);
    let original_path = h
        .root
        .join(entry["attempts"][0]["receipt"]["path"].as_str().unwrap());
    let original = fs::read(&original_path).unwrap();
    fs::write(&original_path, b"changed original cleanup report").unwrap();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_ne!(code, 0, "{maintained} {error}");
    assert_eq!(h.ledger()["entries"], value!([entry]));
    fs::write(&original_path, original).unwrap();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert!(h.ledger()["entries"].as_array().unwrap().is_empty());
}

#[test]
fn automatic_finish_reclaims_real_checkout_and_reports_measured_outputs() {
    let h = Host::new("input.custom");
    h.automatic("evidence-retain");
    let target = h.parent.join("finished");
    assert_eq!(h.invoke("integration", "finished", &target).0, 0);
    output(&target);
    fs::create_dir_all(target.join(".chrono-harness/state")).unwrap();
    fs::write(
        target.join(".chrono-harness/state/evidence"),
        "explicitly settled",
    )
    .unwrap();
    let (code, r, e) = h.auto(
        "finish",
        &["--path", target.to_str().unwrap(), "--dispose-evidence"],
    );
    assert_eq!(code, 0, "{r} {e}");
    assert!(!target.exists());
    let effect = &drain_report(&r["drain"][0]);
    assert_eq!(effect["worktree_removal"], "verified-absent");
    assert_eq!(effect["artifact_disposals"][0]["bytes_before"], 8192);
    assert_eq!(effect["artifact_disposals"][0]["bytes_after"], 0);
    assert!(git(&h.root, &["show-ref"]).contains("refs/heads/integration/finished"));
    assert_eq!(h.auto("maintain", &[]).0, 0);
}

fn entry_receipts(h: &Host, entry: &Value) -> Vec<(PathBuf, Vec<u8>)> {
    let evidence = std::env::var("CHRONO_WORKTREE_TEST_RECEIPTS")
        .ok()
        .map(|p| PathBuf::from(p).join("original-lifecycle-state"));
    if let Some(directory) = &evidence {
        fs::create_dir_all(directory).unwrap();
        let ledger = serde_json::to_vec_pretty(&h.ledger()).unwrap();
        fs::write(
            directory.join(format!("ledger-{}.json", sha256(&ledger))),
            ledger,
        )
        .unwrap();
    }
    let mut receipts = vec![entry["terminal"]["receipt"].clone()];
    if !entry["enrollment"]["sealed_receipt"].is_null() {
        receipts.push(entry["enrollment"]["sealed_receipt"].clone());
    }
    receipts.extend(
        entry["attempts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["receipt"].clone()),
    );
    receipts
        .iter()
        .map(|r| {
            let path = h.root.join(r["path"].as_str().unwrap());
            let bytes = fs::read(&path).unwrap();
            assert_eq!(r["sha256"], sha256(&bytes));
            if let Some(directory) = &evidence {
                fs::write(
                    directory.join(format!("receipt-{}.json", sha256(&bytes))),
                    &bytes,
                )
                .unwrap();
            }
            (path, bytes)
        })
        .collect()
}

#[test]
fn automatic_disposed_path_reuse_targets_new_birth_and_import_generations() {
    let h = Host::new("input.custom");
    h.automatic("evidence-retain");
    native_consumer(&h, "output-branch");
    let target = h.parent.join("reused");
    let args = ["--path", target.to_str().unwrap(), "--dispose-evidence"];
    let (code, r, e) = h.invoke("feature", "first-task", &target);
    assert_eq!(code, 0, "{r} {e}");
    output(&target);
    let (code, r, e) = h.auto("finish", &args);
    assert_eq!(code, 0, "{r} {e}");
    assert!(!target.exists());
    assert_eq!(
        drain_report(&r["drain"][0])["artifact_disposals"][0]["bytes_before"],
        8192
    );
    assert_eq!(
        drain_report(&r["drain"][0])["artifact_disposals"][0]["bytes_after"],
        0
    );
    let first = h.ledger()["entries"][0].clone();
    assert_eq!(first["status"], "disposed");
    assert!(!first["attempts"].as_array().unwrap().is_empty());
    let mut originals = entry_receipts(&h, &first);

    let (code, r, e) = h.invoke("feature", "second-task", &target);
    assert_eq!(code, 0, "{r} {e}");
    assert_eq!(r["drain"], value!([]));
    let ledger = h.ledger();
    assert_eq!(ledger["entries"].as_array().unwrap().len(), 2);
    assert_eq!(ledger["entries"][0], first);
    assert_eq!(ledger["entries"][1]["branch"], "feature/second-task");
    assert_ne!(
        ledger["entries"][1]["enrollment"]["receipt"],
        first["enrollment"]["receipt"]
    );
    for branch in ["second-task", "third-task"] {
        let (code, r, e) = h.auto(
            "use",
            &[
                "--path",
                target.to_str().unwrap(),
                "--operation",
                "use.consumer",
            ],
        );
        assert_eq!(code, 0, "{r} {e}");
        assert_eq!(
            r["managed_process"]["stdout"],
            format!("feature/{branch}\n")
        );
        assert_eq!(
            fs::read(target.join("output λ/use")).unwrap(),
            b"managed-output"
        );
        originals.push((
            h.root
                .join(r["managed_use_receipt"]["path"].as_str().unwrap()),
            fs::read(
                h.root
                    .join(r["managed_use_receipt"]["path"].as_str().unwrap()),
            )
            .unwrap(),
        ));
        let (code, r, e) = h.auto("finish", &args);
        assert_eq!(code, 0, "{r} {e}");
        assert!(!target.exists());
        assert_eq!(r["drain"].as_array().unwrap().len(), 1);
        assert_eq!(
            drain_report(&r["drain"][0])["branch_ref"],
            format!("feature/{branch}")
        );
        assert_eq!(
            drain_report(&r["drain"][0])["worktree_removal"],
            "verified-absent"
        );
        assert_eq!(h.ledger()["entries"][0], first);
        let entries = h.ledger()["entries"].as_array().unwrap().clone();
        let retired = entries.last().unwrap().clone();
        assert_eq!(retired["status"], "disposed");
        assert_eq!(retired["uses"], value!([]));
        originals.extend(entry_receipts(&h, &retired));
        let (code, r, e) = h.auto("maintain", &[]);
        assert_eq!(code, 0, "{r} {e}");
        assert_eq!(r["drain"], value!([]));
        assert_eq!(h.ledger()["entries"], value!(entries));
        for (path, bytes) in &originals {
            assert_eq!(&fs::read(path).unwrap(), bytes);
        }
        if branch == "second-task" {
            git(
                &h.root,
                &[
                    "worktree",
                    "add",
                    "-b",
                    "feature/third-task",
                    target.to_str().unwrap(),
                    "dev",
                ],
            );
            let before_observation = h.ledger();
            let (code, observed, error) = h.auto("maintain", &[]);
            assert_eq!(code, 0, "{observed} {error}");
            assert_eq!(inventory_row(&observed, &target)["enrollment_match"], false);
            assert_eq!(inventory_row(&observed, &target)["status"], "preserved");
            assert_eq!(h.ledger(), before_observation);
            let (code, r, e) = h.auto("import", &["--path", target.to_str().unwrap()]);
            assert_eq!(code, 0, "{r} {e}");
            assert_eq!(h.ledger()["entries"][2]["enrollment"]["kind"], "import");
            assert_eq!(
                &h.ledger()["entries"].as_array().unwrap()[..2],
                entries.as_slice()
            );
        }
    }
}

#[test]
fn automatic_absent_active_enrollment_conflict_preflights_without_creation() {
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    let target = h.parent.join("unresolved");
    assert_eq!(h.invoke("feature", "original-task", &target).0, 0);
    let original = h.ledger();
    // External removal does not establish a terminal handoff in this owner.
    git(
        &h.root,
        &["worktree", "remove", "--force", target.to_str().unwrap()],
    );
    assert!(!target.exists());
    let refs = git(&h.root, &["show-ref"]);
    let inventory = git(&h.root, &["worktree", "list", "--porcelain"]);
    let (code, r, e) = h.invoke("feature", "conflicting-task", &target);
    assert_ne!(code, 0, "{r} {e}");
    assert!(
        r["error"].as_str().unwrap().contains("already enrolled"),
        "{r} {e}"
    );
    assert!(
        !target.exists(),
        "registration conflict must precede checkout creation: {r}"
    );
    assert_eq!(git(&h.root, &["show-ref"]), refs);
    assert_eq!(
        git(&h.root, &["worktree", "list", "--porcelain"]),
        inventory
    );
    assert_eq!(h.ledger(), original);
}

#[test]
fn automatic_cache_only_preserves_evidence_dirty_and_unretained_work() {
    for reason in ["evidence", "dirty", "unretained"] {
        let h = Host::new("payload");
        h.automatic("evidence-retain");
        let target = h.parent.join("preserved");
        assert_eq!(h.invoke("feature", "preserved", &target).0, 0);
        output(&target);
        match reason {
            "evidence" => {
                fs::create_dir_all(target.join(".chrono-harness/state")).unwrap();
                fs::write(target.join(".chrono-harness/state/receipt"), "needed").unwrap();
            }
            "dirty" => fs::write(target.join("payload"), "unsaved").unwrap(),
            _ => {
                fs::write(target.join("payload"), "unretained commit").unwrap();
                commit(&target);
            }
        }
        let args = if reason == "evidence" {
            vec!["--path", target.to_str().unwrap()]
        } else {
            vec!["--path", target.to_str().unwrap(), "--dispose-evidence"]
        };
        let (code, r, e) = h.auto("finish", &args);
        assert_eq!(code, 0, "{reason}: {r} {e}");
        assert!(target.exists());
        assert!(!target.join("output λ").exists());
        assert!(drain_report(&r["drain"][0])["preserved_reason"].is_string());
        if reason == "evidence" {
            assert!(target.join(".chrono-harness/state/receipt").exists());
        }
    }
}

#[test]
fn explicit_current_custody_accepts_an_existing_client_branch_without_claiming_birth() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    let target = h.parent.join("client-checkout");
    git(
        &h.root,
        &[
            "worktree",
            "add",
            "-b",
            "client/current-work",
            target.to_str().unwrap(),
            "dev",
        ],
    );
    let original_head = git(&target, &["rev-parse", "HEAD"]);
    let original_source = fs::read(target.join("payload")).unwrap();

    let (code, explicit, error) = h.maintain(
        "cleanup",
        value!({"schema":"chrono-worktree-maintenance/v1","operation":"cleanup",
            "path":target,"branch":"client/current-work","head":original_head,
            "retained_ref":"refs/heads/dev","retained_commit":git(&h.root,&["rev-parse","dev"]),
            "retention":"ancestor","discard_artifacts":[],"remove_branch":false,
            "allow_absent_worktree":false}),
    );
    assert_ne!(code, 0, "{explicit} {error}");
    assert!(
        explicit["error"]
            .as_str()
            .unwrap()
            .contains("registered work prefix")
    );
    assert!(target.exists());

    let (code, observation, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{observation} {error}");
    let missing = observation["worktree_inventory"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == target.to_str().unwrap())
        .unwrap();
    assert_eq!(missing["enrollment_match"], false);
    assert!(
        missing["preserved_reason"]
            .as_str()
            .unwrap()
            .contains("explicit import")
    );
    assert!(target.exists());

    // A refused command leaves an address record; an empty ledger is still unowned.
    assert_ne!(h.auto("finish", &["--path", target.to_str().unwrap()]).0, 0);
    let empty = json(&fs::read(h.ledger_path(&target)).unwrap()).unwrap();
    assert!(empty["entries"].as_array().unwrap().is_empty());
    let (code, observation, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{observation} {error}");
    let empty = observation["worktree_inventory"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == target.to_str().unwrap())
        .unwrap();
    assert_eq!(empty["enrollment_match"], false);
    assert!(target.exists());

    let (code, imported, error) = h.auto("import", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{imported} {error}");
    let entry = h.ledger()["entries"][0].clone();
    assert_eq!(entry["branch"], "client/current-work");
    assert_eq!(entry["status"], "active");
    assert_eq!(entry["enrollment"]["kind"], "import");
    assert_eq!(
        entry["enrollment"]["receipt"]["historical_birth"],
        "not-claimed"
    );
    assert!(entry["terminal"].is_null());
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), original_head);
    assert_eq!(fs::read(target.join("payload")).unwrap(), original_source);

    let (code, finished, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{finished} {error}");
    assert!(!target.exists());
    assert_eq!(h.ledger()["entries"][0]["status"], "disposed");
    assert!(!git(&h.root, &["worktree", "list", "--porcelain"]).contains(target.to_str().unwrap()));
}

#[test]
fn automatic_import_is_explicit_and_not_a_historical_birth() {
    let h = Host::new("payload");
    let mut config = json(&fs::read(h.root.join(CONFIG)).unwrap()).unwrap();
    config["artifacts"].as_array_mut().unwrap().extend([
        value!({"path":"output λ/","owner":"host","kind":"arbitrary-output","tracked":false}),
        value!({"path":"nested/cache/","owner":"host","kind":"another-layout","tracked":false}),
    ]);
    fs::write(h.root.join(CONFIG), serde_json::to_vec(&config).unwrap()).unwrap();
    h.policy(|_| ());
    let target = h.parent.join("legacy");
    assert_eq!(h.invoke("feature", "legacy", &target).0, 0);
    h.automatic("evidence-retain");
    // Import the unchanged legacy checkout, without inventing a birth or rewriting its source.
    assert!(!target.join(AUTO_POLICY).exists());
    assert_ne!(h.auto("finish", &["--path", target.to_str().unwrap()]).0, 0);
    let (code, r, e) = h.auto("import", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{r} {e}");
    assert_eq!(h.ledger()["entries"][0]["enrollment"]["kind"], "import");
    assert_eq!(
        h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"]
        )
        .0,
        0
    );
    assert!(!target.exists());
}

#[test]
fn observational_import_binds_original_inputs_without_adopting_format_or_disposal_scopes() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_consumer(&h, "ordinary");
    for (detached, retired) in [(false, false), (false, true), (true, false), (true, true)] {
        let target = h
            .parent
            .join(format!("observational import {detached} {retired}"));
        let branch = format!("client/observational-import-{detached}-{retired}");
        let mut args = vec!["worktree", "add"];
        if detached {
            args.push("--detach");
        } else {
            args.extend(["-b", &branch]);
        }
        args.extend([target.to_str().unwrap(), "dev"]);
        git(&h.root, &args);
        let config_path = target.join(POLICY);
        let mut config = json(&fs::read(&config_path).unwrap()).unwrap();
        if retired {
            config["schema"] = value!("chrono-worktree-config/v3");
            config["git"]["identities"] = value!([]);
        }
        let config_bytes = serde_json::to_vec_pretty(&config).unwrap();
        fs::write(&config_path, &config_bytes).unwrap();
        let mut registry = json(&fs::read(target.join(CONFIG)).unwrap()).unwrap();
        registry["artifacts"]
            .as_array_mut()
            .unwrap()
            .retain(|artifact| artifact["path"] != "nested/cache/");
        fs::write(
            target.join(CONFIG),
            serde_json::to_vec_pretty(&registry).unwrap(),
        )
        .unwrap();
        let head = commit(&target);
        fs::write(target.join("payload"), "unfinished source\n").unwrap();
        output(&target);
        let metadata = PathBuf::from(git(&target, &["rev-parse", "--absolute-git-dir"]));
        let originals = [
            target.join(".git"),
            metadata.join("HEAD"),
            metadata.join("index"),
            target.join("payload"),
            target.join(AUTO_POLICY),
            target.join(CONFIG),
        ]
        .map(|path| {
            let bytes = fs::read(&path).unwrap();
            (path, bytes)
        });

        let (code, report, error) = h.auto("status", &["--path", target.to_str().unwrap()]);
        assert_eq!(code, 0, "{report} {error}");
        fs::write(&config_path, [&config_bytes[..], b"\n"].concat()).unwrap();
        let (code, refused, error) = h.auto("import", &["--path", target.to_str().unwrap()]);
        assert_ne!(code, 0, "{refused} {error}");
        assert!(
            refused["error"]
                .as_str()
                .unwrap()
                .contains("differs from commit")
        );
        fs::write(&config_path, &config_bytes).unwrap();
        let (code, imported, error) = h.auto("import", &["--path", target.to_str().unwrap()]);
        assert_eq!(code, 0, "{imported} {error}");
        assert_eq!(imported["policy_source_root"], value!(h.root));
        let ledger_bytes = fs::read(h.ledger_path(&target)).unwrap();
        let ledger = json(&ledger_bytes).unwrap();
        let entry = &ledger["entries"][0];
        assert_eq!(entry["enrollment"]["observed_head"], head);
        assert_eq!(
            entry["enrollment"]["policy_inputs"][POLICY],
            sha256(&config_bytes)
        );
        assert!(entry["terminal"].is_null());
        let (code, observed, error) = h.auto("status", &["--path", target.to_str().unwrap()]);
        assert_eq!(code, 0, "{observed} {error}");
        let row = observed["worktree_status"]["worktrees"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["path"] == value!(target))
            .unwrap();
        assert_eq!(row["work_state"], "active");
        assert_eq!(row["attention_required"], true);
        for operation in ["use", "migrate", "finish"] {
            let mut args = vec!["--path", target.to_str().unwrap()];
            if operation == "use" {
                args.extend(["--operation", "use.consumer"]);
            }
            let (code, refused, error) = h.auto(operation, &args);
            assert_ne!(code, 0, "{operation}: {refused} {error}");
            assert!(
                refused["error"].as_str().unwrap().contains(if retired {
                    "identities"
                } else {
                    "disposal directory is not an artifact"
                }),
                "{operation}: {refused} {error}"
            );
        }
        assert_eq!(fs::read(h.ledger_path(&target)).unwrap(), ledger_bytes);
        assert_eq!(fs::read(&config_path).unwrap(), config_bytes);
        assert!(target.join("output λ/data").is_file());
        for (path, bytes) in originals {
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
    }
}

#[test]
fn detached_import_preserves_unresolved_index_and_reports_unfinished_work() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    let base = git(&h.root, &["rev-parse", "HEAD"]);
    git(&h.root, &["checkout", "-b", "client/other-side"]);
    fs::write(h.root.join("payload"), "other side\n").unwrap();
    let other = commit(&h.root);
    git(&h.root, &["checkout", "dev"]);
    fs::write(h.root.join("payload"), "current side\n").unwrap();
    let head = commit(&h.root);
    let target = h.parent.join("detached unresolved checkout");
    git(
        &h.root,
        &[
            "worktree",
            "add",
            "--detach",
            target.to_str().unwrap(),
            &head,
        ],
    );
    git(&target, &["read-tree", "-m", &base, &head, &other]);
    assert!(!git(&target, &["ls-files", "--unmerged"]).is_empty());
    let metadata = PathBuf::from(git(&target, &["rev-parse", "--absolute-git-dir"]));
    let original = [
        target.join(".git"),
        metadata.join("HEAD"),
        metadata.join("index"),
        target.join("payload"),
    ]
    .map(|path| {
        let bytes = fs::read(&path).unwrap();
        (path, bytes)
    });

    let (code, report, error) = h.auto("import", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{report} {error}");
    let (code, migrated, error) = h.auto("migrate", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{migrated} {error}");
    let (code, status, error) = h.auto("status", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{status} {error}");
    let row = status["worktree_status"]["worktrees"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == value!(target))
        .unwrap();
    assert_eq!(row["branch"], "HEAD");
    assert_eq!(row["work_state"], "active");
    assert_eq!(row["attention_required"], true);
    assert_eq!(row["next_action"], "complete-work-and-finish");
    let ledger = json(&fs::read(h.ledger_path(&target)).unwrap()).unwrap();
    assert!(ledger["entries"][0]["terminal"].is_null());
    for (path, bytes) in original {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), head);
    assert!(!git(&target, &["ls-files", "--unmerged"]).is_empty());
}

#[test]
fn detached_finish_removes_only_the_enrolled_checkout_and_keeps_all_branches() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    let path = h.root.join(AUTO_POLICY);
    let mut policy = json(&fs::read(&path).unwrap()).unwrap();
    policy["remove_branch"] = value!(true);
    fs::write(path, serde_json::to_vec_pretty(&policy).unwrap()).unwrap();
    commit(&h.root);
    let target = h.parent.join("detached finished checkout");
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
    let branches = git(&h.root, &["for-each-ref", "refs/heads/"]);
    let (code, report, error) = h.auto("import", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{report} {error}");
    output(&target);
    let (code, finished, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{finished} {error}");
    assert!(!target.exists(), "{finished}");
    assert_eq!(git(&h.root, &["for-each-ref", "refs/heads/"]), branches);
    let row = finished["drain"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == value!(target))
        .unwrap();
    let cleanup = drain_report(row);
    assert_eq!(cleanup["branch_removal"], "not-applicable-detached");
    assert_eq!(cleanup["branch_removed"], false);
    assert_eq!(cleanup["worktree_removal"], "verified-absent");
}

#[test]
fn detached_enrollment_rejects_later_branch_attachment_without_changing_work() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    let target = h.parent.join("changed detached checkout");
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
    let (code, report, error) = h.auto("import", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{report} {error}");
    git(&target, &["checkout", "-b", "client/reattached"]);
    let metadata = PathBuf::from(git(&target, &["rev-parse", "--absolute-git-dir"]));
    let index = fs::read(metadata.join("index")).unwrap();
    let branches = git(&h.root, &["for-each-ref", "refs/heads/"]);
    let (code, refused, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_ne!(code, 0, "{refused} {error}");
    assert!(
        refused["error"]
            .as_str()
            .unwrap()
            .contains("identity or ownership lock mismatch")
    );
    assert!(target.is_dir());
    assert_eq!(fs::read(metadata.join("index")).unwrap(), index);
    assert_eq!(git(&h.root, &["for-each-ref", "refs/heads/"]), branches);
    let ledger = json(&fs::read(h.ledger_path(&target)).unwrap()).unwrap();
    assert!(ledger["entries"][0]["terminal"].is_null());
}

fn blocked_remove(h: &Host, after_effect: bool) {
    native_git(h, "automatic-remove", value!({"after_effect":after_effect}));
}

#[test]
fn automatic_partial_failure_preserves_independent_start_and_explicit_retry() {
    for after_effect in [false, true] {
        let h = Host::new("payload");
        blocked_remove(&h, after_effect);
        h.automatic("evidence-retain");
        let target = h.parent.join("pending");
        assert_eq!(h.invoke("feature", "pending", &target).0, 0);
        output(&target);
        fs::write(h.parent.join("fail-remove"), "one failure").unwrap();
        let (code, failed, e) = h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"],
        );
        assert_ne!(code, 0, "{failed} {e}");
        let original = &drain_report(&failed["drain"][0]);
        assert_eq!(original["status"], "failed");
        assert_eq!(original["worktree_removal"], "attempted-unverified");
        assert_eq!(original["worktree_removed"], false);
        assert_eq!(
            original["artifact_disposals"][0]["status"],
            "verified-absent"
        );
        assert_eq!(target.exists(), !after_effect);
        assert!(original["processes"].as_array().unwrap().iter().any(|p| {
            p["process"]["exit_code"] == 71
                && p["process"]["stderr"]
                    .as_str()
                    .unwrap_or("")
                    .contains("original automatic removal failure")
        }));
        let receipt = h.root.join(original["report_path"].as_str().unwrap());
        let bytes = fs::read(&receipt).unwrap();
        let next = h.parent.join("next");
        if !after_effect {
            let independent = h.parent.join("independent");
            let (code, admitted, error) = h.invoke("feature", "independent", &independent);
            assert_eq!(code, 0, "{} {error}", admitted["error"]);
            assert!(independent.exists());
            assert!(admitted["cleanup_failures"].as_array().unwrap().is_empty());
            assert!(admitted["drain"].as_array().unwrap().is_empty());
            assert_eq!(fs::read(&receipt).unwrap(), bytes);
        }
        fs::remove_file(h.parent.join("fail-remove")).unwrap();
        let (code, r, e) = h.invoke("feature", "next", &next);
        assert_eq!(code, 0, "{r} {e}");
        assert_eq!(target.exists(), !after_effect);
        assert!(next.exists());
        assert_eq!(fs::read(receipt).unwrap(), bytes);
        assert_eq!(h.auto("maintain", &[]).0, 0);
        assert!(!target.exists());
    }
}

#[test]
fn automatic_retries_keep_original_evidence_without_recursive_growth() {
    for kernel in [false, true] {
        let h = Host::new("payload");
        blocked_remove(&h, false);
        if kernel {
            h.kernel_cleanup();
        } else {
            h.automatic("evidence-retain");
        }
        let target = h.parent.join("bounded-retry");
        assert_eq!(h.invoke("feature", "bounded-retry", &target).0, 0);
        output(&target);
        fs::write(h.parent.join("fail-remove"), "retain original failure").unwrap();
        let (code, failed, error) = h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"],
        );
        assert_ne!(code, 0, "{failed} {error}");
        let report = &drain_report(&failed["drain"][0]);
        let original_path = h.root.join(report["report_path"].as_str().unwrap());
        let original = fs::read(&original_path).unwrap();
        let bound = original.len() * 3;
        let mut originals = vec![(original_path, original)];
        for retry in 0..3 {
            let (code, failed, error) = h.auto("maintain", &[]);
            assert_ne!(code, 0, "{failed} {error}");
            let report = &drain_report(&failed["drain"][0]);
            assert_eq!(report["status"], "failed");
            assert!(report["processes"].as_array().unwrap().iter().any(|p| {
                p["process"]["exit_code"] == 71
                    && p["process"]["stderr"]
                        .as_str()
                        .unwrap_or("")
                        .contains("original automatic removal failure")
            }));
            let path = h.root.join(report["report_path"].as_str().unwrap());
            let bytes = fs::read(&path).unwrap();
            // The controlled operation is unchanged across retries. Its report
            // may grow with current observations, but cannot multiply history.
            assert!(
                bytes.len() <= bound,
                "retry {retry}, kernel={kernel}: {} bytes exceeds current-operation bound {bound}",
                bytes.len()
            );
            let input = &report["prior_report"]["input"];
            assert_eq!(input["schema"], "chrono-worktree-retained-input/v1");
            assert_eq!(input["source_root"], value!(h.root));
            let retained = fs::read(h.root.join(input["path"].as_str().unwrap())).unwrap();
            assert_eq!(retained, originals.last().unwrap().1);
            assert_eq!(input["sha256"], sha256(&retained));
            assert_eq!(input["byte_length"], retained.len());
            for (path, bytes) in &originals {
                assert_eq!(fs::read(path).unwrap(), *bytes);
            }
            originals.push((path, bytes));
        }
        fs::remove_file(h.parent.join("fail-remove")).unwrap();
        for fault in ["latest", "ancestor", "absent", "symlink"] {
            let (path, bytes) = if fault == "latest" {
                originals.last().unwrap()
            } else {
                &originals[0]
            };
            match fault {
                "absent" => fs::remove_file(path).unwrap(),
                "symlink" => {
                    fs::remove_file(path).unwrap();
                    let outside = h.parent.join("same-original-bytes");
                    fs::write(&outside, bytes).unwrap();
                    std::os::unix::fs::symlink(&outside, path).unwrap();
                }
                _ => {
                    let mut drifted = bytes.clone();
                    drifted.push(b' ');
                    fs::write(path, drifted).unwrap();
                }
            }
            let (code, refused, error) = h.auto("maintain", &[]);
            assert_ne!(code, 0, "{fault}: {refused} {error}");
            if matches!(fault, "latest" | "ancestor") {
                assert!(
                    refused["error"]
                        .as_str()
                        .unwrap()
                        .contains("original lifecycle receipt changed")
                );
            }
            assert!(
                target.exists(),
                "{fault}: invalid evidence cannot authorize deletion"
            );
            if fault == "symlink" {
                fs::remove_file(path).unwrap();
            }
            fs::write(path, bytes).unwrap();
        }
        assert_eq!(h.auto("maintain", &[]).0, 0);
        assert!(!target.exists());
        for (path, bytes) in originals {
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
    }
}

#[test]
fn automatic_symlink_containment_and_internal_external_target_safety() {
    use std::os::unix::fs::symlink;
    for fault in ["internal", "ancestor", "selected-root"] {
        let h = Host::new("payload");
        h.automatic("evidence-retain");
        let target = h.parent.join("links");
        assert_eq!(h.invoke("feature", "links", &target).0, 0);
        output(&target);
        let outside = h.parent.join("external-data");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("keep"), "outside").unwrap();
        match fault {
            "internal" => symlink(&outside, target.join("output λ/link")).unwrap(),
            "ancestor" => symlink(&outside, target.join("nested")).unwrap(),
            _ => {
                fs::remove_dir_all(target.join("output λ")).unwrap();
                symlink(&outside, target.join("output λ")).unwrap();
            }
        }
        let (code, r, e) = h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"],
        );
        assert_eq!(fs::read(outside.join("keep")).unwrap(), b"outside");
        if fault == "internal" {
            assert_eq!(code, 0, "{r} {e}");
            assert!(!target.exists());
        } else {
            assert_ne!(code, 0, "{r} {e}");
            assert!(target.exists());
            if fault == "ancestor" {
                assert!(
                    target.join("output λ/data").exists(),
                    "whole selection must preflight before deletion"
                );
            }
        }
    }
}

pub(super) fn native_git(h: &Host, mode: &str, mut parameters: Value) -> PathBuf {
    let input = h.parent.join("git-fixture.json");
    parameters["mode"] = value!(mode);
    parameters["git"] = value!(fixture_git());
    fs::write(&input, serde_json::to_vec(&parameters).unwrap()).unwrap();
    h.policy(|policy| {
        policy["git"]["program"] = value!(env!("CARGO_BIN_EXE_chrono-worktree-test-git"));
        policy["environment"]["values"]["CHRONO_TEST_GIT_FIXTURE"] = value!(input);
    });
    input
}

pub(super) fn native_consumer(h: &Host, mode: &str) {
    native_consumer_actions(h, mode, "noop");
}

pub(super) fn native_consumer_actions(h: &Host, mode: &str, test_mode: &str) {
    super::check_inputs::ready_version_program(
        &h.root,
        Path::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer")),
        b"fixture\n",
    );
    let mut config = json(&fs::read(h.root.join(CONFIG)).unwrap()).unwrap();
    config["tools"].as_array_mut().unwrap().push(value!({
        "id":"consumer", "program":env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"),
        "resolution":"PATH-once", "version_argv":["--version"], "expected_version":"fixture"
    }));
    if matches!(mode, "bounded-cargo" | "cargo-run") {
        // Capture the actual Cargo selection before auto_command clears its environment.
        for name in ["HOME", "RUSTUP_HOME", "RUSTUP_TOOLCHAIN"] {
            if let Ok(value) = std::env::var(name) {
                config["environment"]["values"][name] = value!(value);
            }
        }
    }
    fs::write(h.root.join(CONFIG), serde_json::to_vec(&config).unwrap()).unwrap();
    consuming_scripts(
        h,
        value!([
            {"id":"consumer","path":"consumer.rs","test_script":"consumer-tests","actions":{"execute":{"operation":"use.consumer","tool":"consumer","argv":[mode]}}},
            {"id":"consumer-tests","path":"consumer-tests.rs","tests_for":"consumer","actions":{"execute":{"operation":"test.consumer","tool":"consumer","argv":[test_mode]}}}
        ]),
        &[
            ("consumer.rs", include_bytes!("support/consumer.rs")),
            ("consumer-tests.rs", include_bytes!("support/consumer.rs")),
            (
                "native_cargo/Cargo.toml",
                include_bytes!("support/native_cargo/Cargo.toml"),
            ),
            (
                "native_cargo/src/lib.rs",
                include_bytes!("support/native_cargo/src/lib.rs"),
            ),
        ],
    );
}

fn consuming_scripts(h: &Host, scripts: Value, sources: &[(&str, &[u8])]) {
    let mut projects = json(&fs::read(h.root.join(PROJECTS)).unwrap()).unwrap();
    projects["owners"]
        .as_array_mut()
        .unwrap()
        .extend([value!("consumer"), value!("consumer-tests")]);
    projects["scripts"] = scripts;
    fs::write(
        h.root.join(PROJECTS),
        serde_json::to_vec(&projects).unwrap(),
    )
    .unwrap();
    let mut fm = json(&fs::read(h.root.join(FM)).unwrap()).unwrap();
    fm["files"]
        .as_array_mut()
        .unwrap()
        .extend(sources.iter().map(|(path, _)| file(path, value!([]))));
    fm["project_edges"] = value!([edge(
        "script:consumer",
        "test-execution",
        "test:consumer-tests"
    )]);
    fm["test_costs"] = value!([{"test":"consumer-tests","cost":"unknown"}]);
    fs::write(h.root.join(FM), serde_json::to_vec(&fm).unwrap()).unwrap();
    for (path, bytes) in sources {
        fs::create_dir_all(h.root.join(path).parent().unwrap()).unwrap();
        fs::write(h.root.join(path), bytes).unwrap();
    }
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
}

#[test]
fn automatic_managed_real_child_blocks_finish_and_exit_releases_use() {
    use std::{
        thread,
        time::{Duration, Instant},
    };
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    native_consumer(&h, "managed-child");
    let target = h.parent.join("in-use");
    assert_eq!(h.invoke("feature", "in-use", &target).0, 0);
    output(&target);
    let mut child = h
        .auto_command(
            "use",
            &[
                "--path",
                target.to_str().unwrap(),
                "--operation",
                "use.consumer",
            ],
        )
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let began = Instant::now();
    while !target.join(".chrono-harness/state/ready").exists()
        && began.elapsed() < Duration::from_secs(10)
    {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    let ready = target.join(".chrono-harness/state/ready").exists();
    // Join the wrapper before asserting, including its failure path. A missing
    // readiness marker must expose the real wrapper result and retain its receipt.
    let finish = ready.then(|| {
        h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"],
        )
    });
    let output_retained = target.join("output λ/data").exists();
    let uses = h.ledger()["entries"][0]["uses"].clone();
    let release = fs::write(target.join(".chrono-harness/state/release"), "join now");
    let (code, r, e) = h.received(child.wait_with_output().unwrap());
    assert!(
        ready,
        "managed child did not become ready: exit {code}; {r} {e}"
    );
    release.unwrap();
    let (finish_code, finish_report, finish_error) = finish.unwrap();
    assert_ne!(finish_code, 0, "{finish_report} {finish_error}");
    assert!(output_retained);
    assert_eq!(uses.as_array().unwrap().len(), 1);
    assert_eq!(code, 0, "{r} {e}");
    assert_eq!(r["managed_process"]["stdout"], "joined-child");
    assert_eq!(h.ledger()["entries"][0]["uses"], value!([]));
    let (code, r, e) = h.auto(
        "finish",
        &["--path", target.to_str().unwrap(), "--dispose-evidence"],
    );
    assert_eq!(code, 0, "{r} {e}");
    assert!(!target.exists());
}

#[test]
fn automatic_terminal_rejects_changed_head_ref_policy_attachment_and_git_locks() {
    for fault in [
        "head",
        "ref",
        "policy",
        "target-policy",
        "target-config",
        "attachment",
        "git-lock",
        "worktree-lock",
    ] {
        let h = Host::new("payload");
        blocked_remove(&h, false);
        h.automatic("evidence-retain");
        let target = h.parent.join("drift");
        assert_eq!(h.invoke("feature", "drift", &target).0, 0);
        output(&target);
        fs::write(h.parent.join("fail-remove"), "fail").unwrap();
        assert_ne!(
            h.auto(
                "finish",
                &["--path", target.to_str().unwrap(), "--dispose-evidence"]
            )
            .0,
            0
        );
        fs::remove_file(h.parent.join("fail-remove")).unwrap();
        output(&target);
        match fault {
            "head" => {
                fs::write(target.join("payload"), "changed head").unwrap();
                commit(&target);
            }
            "ref" => {
                git(
                    &h.root,
                    &[
                        "-c",
                        "user.name=fixture",
                        "-c",
                        "user.email=fixture@example.invalid",
                        "commit",
                        "--allow-empty",
                        "-qm",
                        "advanced reference",
                    ],
                );
            }
            "policy" => {
                let mut p = json(&fs::read(h.root.join(AUTO_POLICY)).unwrap()).unwrap();
                p["remove_branch"] = value!(true);
                fs::write(h.root.join(AUTO_POLICY), serde_json::to_vec(&p).unwrap()).unwrap();
            }
            "target-policy" => {
                let mut p = json(&fs::read(target.join(AUTO_POLICY)).unwrap()).unwrap();
                p["remove_branch"] = value!(true);
                fs::write(target.join(AUTO_POLICY), serde_json::to_vec(&p).unwrap()).unwrap();
            }
            "target-config" => {
                let mut p = json(&fs::read(target.join(POLICY)).unwrap()).unwrap();
                p["output_limit_bytes"] = value!(8192);
                fs::write(target.join(POLICY), serde_json::to_vec(&p).unwrap()).unwrap();
            }
            "attachment" => {
                fs::rename(target.join(".git"), h.parent.join("original-gitfile")).unwrap();
                fs::write(
                    target.join(".git"),
                    fs::read(h.parent.join("original-gitfile")).unwrap(),
                )
                .unwrap();
            }
            "git-lock" => {
                let metadata = git(&target, &["rev-parse", "--absolute-git-dir"]);
                fs::write(Path::new(&metadata).join("index.lock"), "owned").unwrap();
            }
            _ => {
                git(
                    &h.root,
                    &[
                        "worktree",
                        "lock",
                        "--reason",
                        "another-owner",
                        target.to_str().unwrap(),
                    ],
                );
            }
        }
        let (code, r, e) = h.auto("maintain", &[]);
        assert_ne!(code, 0, "{fault}: {r} {e}");
        assert!(target.exists());
        assert!(target.join("output λ/data").exists());
    }
}

#[test]
fn automatic_reconstruct_enrolls_only_success_and_unresolved_evidence_can_later_settle() {
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    let head = git(&h.root, &["rev-parse", "HEAD"]);
    let (code, r, e) = h.reconstruct(reconstruction(&head, &head, value!([])), "reborn");
    assert_eq!(code, 0, "{r} {e}");
    let target = h.parent.join("reborn");
    fs::create_dir_all(target.join(".chrono-harness/state")).unwrap();
    fs::write(
        target.join(".chrono-harness/state/needed-original"),
        "evidence",
    )
    .unwrap();
    output(&target);
    assert_eq!(
        h.ledger()["entries"][0]["enrollment"]["receipt"]["operation"],
        "reconstruct"
    );
    assert_eq!(h.auto("finish", &["--path", target.to_str().unwrap()]).0, 0);
    assert!(target.exists());
    assert_eq!(h.ledger()["entries"][0]["status"], "retained");
    let (code, r, e) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{r} {e}");
    assert_eq!(
        drain_report(&r["drain"][0])["worktree_removal"],
        "preserved"
    );
    assert_eq!(
        h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"]
        )
        .0,
        0
    );
    assert!(!target.exists());
}

#[test]
fn automatic_same_tree_retention_remains_exact_and_branch_removal_is_opt_in() {
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    let mut policy = json(&fs::read(h.root.join(AUTO_POLICY)).unwrap()).unwrap();
    policy["retention"] = value!("same-tree");
    policy["remove_branch"] = value!(true);
    fs::write(
        h.root.join(AUTO_POLICY),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("squash");
    assert_eq!(h.invoke("integration", "squash", &target).0, 0);
    fs::write(target.join("payload"), "landed work").unwrap();
    let candidate = commit(&target);
    fs::write(h.root.join("payload"), "landed work").unwrap();
    git(&h.root, &["add", "payload"]);
    git(
        &h.root,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--no-gpg-sign",
            "-qm",
            "verified squash landing",
        ],
    );
    let landed = git(&h.root, &["rev-parse", "HEAD"]);
    assert_ne!(candidate, landed);
    output(&target);
    let (code, r, e) = h.auto(
        "finish",
        &[
            "--path",
            target.to_str().unwrap(),
            "--dispose-evidence",
            "--retained-commit",
            &landed,
        ],
    );
    assert_eq!(code, 0, "{r} {e}");
    assert!(!target.exists());
    assert!(
        !git(&h.root, &["for-each-ref", "--format=%(refname)"])
            .contains("refs/heads/integration/squash")
    );
    assert_eq!(git(&h.root, &["rev-parse", "dev"]), landed);
}

#[test]
fn automatic_filesystem_partial_failure_retries_under_original_owned_lock() {
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    let target = h.parent.join("filesystem-failure");
    assert_eq!(h.invoke("feature", "filesystem-failure", &target).0, 0);
    output(&target);
    fs::create_dir_all(target.join("nested/cache")).unwrap();
    fs::write(target.join("nested/cache/keep-until-retry"), "cache").unwrap();
    let special = target.join("nested/cache/special");
    assert!(
        Command::new("mkfifo")
            .arg(&special)
            .status()
            .unwrap()
            .success()
    );
    let (code, r, e) = h.auto(
        "finish",
        &["--path", target.to_str().unwrap(), "--dispose-evidence"],
    );
    assert_ne!(code, 0, "{r} {e}");
    let original = &drain_report(&r["drain"][0]);
    assert_eq!(
        original["artifact_disposals"][0]["status"],
        "verified-absent"
    );
    assert_eq!(original["worktree_removal"], "not-attempted");
    assert!(target.exists());
    assert!(!target.join("output λ").exists());
    assert!(special.exists());
    assert!(
        git(&h.root, &["worktree", "list", "--porcelain"])
            .contains(original["lock_reason"].as_str().unwrap())
    );
    let receipt = h.root.join(original["report_path"].as_str().unwrap());
    let bytes = fs::read(&receipt).unwrap();
    fs::remove_file(special).unwrap();
    let (code, r, e) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{r} {e}");
    assert!(!target.exists());
    assert_eq!(fs::read(receipt).unwrap(), bytes);
    assert_eq!(
        drain_report(&r["drain"][0])["artifact_disposals"][0]["status"],
        "already-absent"
    );
}

#[test]
fn automatic_failed_birth_and_missing_state_are_protected() {
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    h.hook(value!({"stderr":"original-birth-failure","exit":17}));
    let failed = h.parent.join("failed");
    assert_ne!(h.invoke("feature", "failed", &failed).0, 0);
    assert!(failed.exists());
    assert_eq!(h.ledger()["entries"], value!([]));
    assert_ne!(h.auto("finish", &["--path", failed.to_str().unwrap()]).0, 0);
    assert!(failed.exists());
    fs::remove_file(h.root.join(".git/hooks/post-checkout")).unwrap();
    let active = h.parent.join("active");
    assert_eq!(h.invoke("feature", "active", &active).0, 0);
    output(&active);
    fs::remove_file(h.ledger_path(&active)).unwrap();
    let next = h.parent.join("next");
    assert_eq!(h.invoke("feature", "next", &next).0, 0);
    assert!(next.exists());
    assert!(active.join("output λ/data").exists());
}

#[test]
fn automatic_explicit_main_anchor_short_commands_and_finish_from_source_then_start() {
    let h = Host::new("layout/source.ext");
    h.automatic("evidence-retain");
    let mut policy = json(&fs::read(h.root.join(AUTO_POLICY)).unwrap()).unwrap();
    policy["coordinator_root"] = value!("git-main-worktree");
    fs::write(
        h.root.join(AUTO_POLICY),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("self-finish");
    assert_eq!(h.invoke("feature", "self-finish", &target).0, 0);
    output(&target);
    let out = Command::new(source().join("crates/worktree/target/debug/chrono-worktree"))
        .current_dir(&target)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap())
        .args(["finish", "--dispose-evidence"])
        .output()
        .unwrap();
    let (code, r, e) = h.received(out);
    assert_eq!(code, 0, "{r} {e}");
    assert_eq!(
        r["drain"][0]["preserved_reason"],
        "invoking source or destination"
    );
    assert!(target.exists());
    let next = h.parent.join("fresh");
    let (code, r, e) = h.invoke("feature", "fresh", &next);
    assert_eq!(code, 0, "{r} {e}");
    assert!(
        target.exists(),
        "another birth has no cleanup dependency on this target"
    );
    let (code, r, e) = h.received(
        Command::new(source().join("crates/worktree/target/debug/chrono-worktree"))
            .current_dir(&h.root)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap())
            .arg("maintain")
            .output()
            .unwrap(),
    );
    assert_eq!(code, 0, "{r} {e}");
    assert!(!target.exists());
    assert!(next.exists());
}

#[test]
fn automatic_staged_source_inside_artifact_and_unauthorized_evidence_remain() {
    for fault in ["staged-source", "evidence-authorization", "gate"] {
        let h = Host::new("payload");
        h.automatic("evidence-retain");
        if fault == "evidence-authorization" {
            let mut p = json(&fs::read(h.root.join(AUTO_POLICY)).unwrap()).unwrap();
            p["allow_evidence_disposal"] = value!(false);
            fs::write(h.root.join(AUTO_POLICY), serde_json::to_vec(&p).unwrap()).unwrap();
            commit(&h.root);
            git(&h.root, &["push", "-q", "warehouse", "dev"]);
        }
        let target = h.parent.join("protected");
        assert_eq!(h.invoke("feature", "protected", &target).0, 0);
        output(&target);
        if fault == "staged-source" {
            git(&target, &["add", "-f", "output λ/data"]);
        }
        if fault == "gate" {
            fs::create_dir(h.record_directory(&target).join("gate.lock")).unwrap();
        }
        let (code, r, e) = h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"],
        );
        assert_ne!(code, 0, "{fault}: {r} {e}");
        assert!(target.join("output λ/data").exists());
    }
}

#[test]
fn automatic_interrupted_managed_wrapper_keeps_unknown_use_protected() {
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    native_consumer(&h, "unknown-use");
    let target = h.parent.join("unknown-use");
    assert_eq!(h.invoke("feature", "unknown-use", &target).0, 0);
    output(&target);
    let mut wrapper = super::interrupted_cleanup::CapturedChild::spawn(
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
    wrapper.await_file(&target.join(".chrono-harness/state/ready"));
    let child_pid = fs::read_to_string(target.join(".chrono-harness/state/ready")).unwrap();
    wrapper.kill().unwrap();
    let out = wrapper.wait_with_output().unwrap();
    assert_ne!(h.received(out).0, 0);
    // Join exactly the test-owned process group; no process scanning or age heuristic.
    assert!(
        Command::new("/bin/kill")
            .args(["-KILL", "--", &format!("-{child_pid}")])
            .status()
            .unwrap()
            .success()
    );
    assert_eq!(
        h.ledger()["entries"][0]["uses"].as_array().unwrap().len(),
        1
    );
    assert_ne!(
        h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"]
        )
        .0,
        0
    );
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(target.join("output λ/data").exists());
}

#[test]
fn automatic_concurrent_managed_completions_release_both_uses() {
    use std::{
        thread,
        time::{Duration, Instant},
    };
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    native_consumer(&h, "waiting-completion");
    let target = h.parent.join("parallel-use");
    assert_eq!(h.invoke("feature", "parallel-use", &target).0, 0);
    let mut children = vec![];
    for _ in 0..2 {
        children.push(
            h.auto_command(
                "use",
                &[
                    "--path",
                    target.to_str().unwrap(),
                    "--operation",
                    "use.consumer",
                ],
            )
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap(),
        );
    }
    let began = Instant::now();
    while h.ledger()["entries"][0]["uses"].as_array().unwrap().len() != 2
        && began.elapsed() < Duration::from_secs(30)
    {
        thread::sleep(Duration::from_millis(20));
    }
    let admitted = h.ledger()["entries"][0]["uses"].as_array().unwrap().len();
    fs::create_dir_all(target.join(".chrono-harness/state")).unwrap();
    fs::write(target.join(".chrono-harness/state/release"), "joined").unwrap();
    for child in children {
        let (code, r, e) = h.received(child.wait_with_output().unwrap());
        assert_eq!(code, 0, "{r} {e}");
    }
    assert_eq!(
        admitted, 2,
        "both live consumers must enroll before release"
    );
    assert_eq!(h.ledger()["entries"][0]["uses"], value!([]));
    assert_eq!(
        h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"]
        )
        .0,
        0
    );
    assert!(!target.exists());
}

#[test]
fn automatic_use_from_checkout_publishes_at_surviving_anchor() {
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    native_consumer(&h, "finished");
    let target = h.parent.join("local-use");
    assert_eq!(h.invoke("feature", "local-use", &target).0, 0);
    let out = Command::new(source().join("crates/worktree/target/debug/chrono-worktree"))
        .current_dir(&target)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap())
        .args(["use", "--operation", "use.consumer"])
        .output()
        .unwrap();
    let (code, r, e) = h.received(out);
    assert_eq!(code, 0, "{r} {e}");
    assert_eq!(r["source_root"], value!(h.root));
    assert_eq!(r["invoking_root"], value!(target));
    let receipt = h.root.join(r["report_path"].as_str().unwrap());
    let bytes = fs::read(&receipt).unwrap();
    assert_eq!(
        h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"]
        )
        .0,
        0
    );
    assert!(!target.exists());
    assert_eq!(fs::read(receipt).unwrap(), bytes);
}
