#[path = "support/construction.rs"]
mod support;
use chrono_host_bootstrap::{
    bindings::native_platform, owner, verified_bindings::owner_publication as publication,
};
use serde_json::{Value, json};
use std::{
    fs,
    os::fd::AsFd,
    path::PathBuf,
    process::{Command, Output},
};
use support::Fixture;

fn read_json(path: &std::path::Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write_json(path: &std::path::Path, value: &Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}

struct Staged {
    fixture: Fixture,
    reference: String,
    digest: String,
    report: Value,
}
impl Staged {
    fn setup() -> Fixture {
        let mut fixture = Fixture::new();
        let native = native_platform().unwrap();
        let registry_path = fixture.root.join(".chrono-harness/ci/registry.json");
        let mut registry = read_json(&registry_path);
        registry["platforms"][format!("{}/{}", native.system, native.machine)] =
            registry["platforms"]["fixture/native"].clone();
        write_json(&registry_path, &registry);
        fixture.config["install"] = json!([]);
        fixture.config["participation"]["git"] = json!("/usr/bin/git");
        fixture.config["participation"]["program"] = json!(".chrono-harness/bin/cargo");
        fixture.config["report_path"] = json!(".chrono-harness/state/bootstrap-owner-stage.json");
        fixture.config["owner_stage"] =
            json!({"source":"out/cargo", "selected":".chrono-harness/bin/cargo"});
        fs::create_dir_all(fixture.root.join(".chrono-harness/bin")).unwrap();
        fs::write(
            fixture.root.join(".chrono-harness/bin/cargo"),
            b"original installed owner bytes",
        )
        .unwrap();
        fixture.commit();
        fixture
    }
    fn invoke(
        fixture: &Fixture,
        arguments: &[&str],
        marker: bool,
        capabilities: bool,
        deployment: Option<&str>,
    ) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-test-bootstrap-native"));
        command
            .current_dir(&fixture.root)
            .args(arguments)
            .env_clear()
            .envs(fixture.environment.iter().filter(|(key, _)| {
                key.as_os_str() != std::ffi::OsStr::new(chrono_harness::process_fds::ENV)
            }));
        command
            .env_remove("CHRONO_WORKTREE_BOOTSTRAP")
            .env_remove("CHRONO_WORKTREE_OWNER_DEPLOYMENT");
        if marker {
            command.env("CHRONO_WORKTREE_BOOTSTRAP", &fixture.root);
        }
        if let Some(value) = deployment {
            command.env("CHRONO_WORKTREE_OWNER_DEPLOYMENT", value);
        }
        if capabilities {
            let lease = tempfile::tempfile().unwrap();
            let scope = chrono_harness::process_fds::Scope::new(&[lease.as_fd()]).unwrap();
            chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
            drop(scope);
            // Command owns the duplicate until its real child has completed.
            command.output().unwrap()
        } else {
            command.output().unwrap()
        }
    }
    fn new() -> Self {
        let fixture = Self::setup();
        let selected = fixture.root.join(".chrono-harness/bin/cargo");
        let previous = owner::file_binding(&selected).unwrap();
        let output = Self::invoke(
            &fixture,
            &[
                "construct-owner-stage",
                ".",
                ".chrono-harness/ci/build.json",
            ],
            true,
            true,
            None,
        );
        assert!(
            output.status.success(),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let last = output
            .stdout
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .last()
            .unwrap();
        let descriptor: Value = serde_json::from_slice(
            last.strip_prefix(b"chrono-owner-stage-result ")
                .expect("missing stage handoff"),
        )
        .unwrap();
        let reference = descriptor["path"].as_str().unwrap().to_owned();
        let digest = descriptor["sha256"].as_str().unwrap().to_owned();
        let raw = fs::read(fixture.root.join(&reference)).unwrap();
        assert_eq!(chrono_harness::sha256(&raw), digest);
        assert!(publication::binding_equal(
            &owner::file_binding(&selected).unwrap(),
            &previous
        ));
        let report = serde_json::from_slice(&raw).unwrap();
        Self {
            fixture,
            reference,
            digest,
            report,
        }
    }
    fn stage_path(&self) -> PathBuf {
        self.fixture.root.join(
            self.report["owner_stage"]["staged"]["path"]
                .as_str()
                .unwrap(),
        )
    }
    fn publish_external_inputs(&self) -> Value {
        // 合成外部 publication/adoption 输入，只测试候选读取器。
        // 实际独占发布、保全旧 inode 和恢复由 worktree 专属 Rust 测试验证。
        let root = &self.fixture.root;
        let staged = self.stage_path();
        let parent = staged.parent().unwrap();
        let selected = root.join(".chrono-harness/bin/cargo");
        fs::rename(&selected, parent.join("previous-owner")).unwrap();
        fs::copy(&staged, &selected).unwrap();
        let mut layout: Value =
            serde_json::from_slice(include_bytes!("../../fixtures/owner-publication.json"))
                .unwrap();
        let installed = serde_json::to_value(owner::file_binding(&selected).unwrap()).unwrap();
        layout["intent"]["stage"] = self.report["owner_stage"].clone();
        layout["intent"]["replacement"] = installed.clone();
        layout["result"]["stage"] = self.report["owner_stage"].clone();
        layout["result"]["installed"] = installed.clone();
        write_json(&parent.join("publication-intent.json"), &layout["intent"]);
        write_json(&parent.join("publication-result.json"), &layout["result"]);
        let mut adoption = json!({"schema":"chrono-worktree-owner-adoption/v1", "source_commit":self.report["source"]["commit"],
            "stage_result":self.reference, "stage_sha256":self.digest, "installed":installed,
            "original_owner":self.report["owner_stage"]["previous"], "original_protection":{"fixture":"external original"},
            "publication":layout["result"], "terminal_handoff":"not-claimed", "task_completion":"not-claimed"});
        let receipt = ".chrono-harness/state/automatic-cleanup-v2/owner-adoption-fixture.json";
        write_json(&root.join(receipt), &adoption);
        adoption["receipt"] = json!({"path":receipt,"sha256":chrono_harness::sha256(&fs::read(root.join(receipt)).unwrap())});
        write_json(&root.join(owner::CURRENT), &adoption);
        adoption
    }
}

#[test]
fn native_owner_stage_retains_actual_execution_and_selected_inode_then_validates_the_exclusive_callback()
 {
    let staged = Staged::new();
    let root = &staged.fixture.root;
    let qualified = owner::stage_report(
        root,
        &staged.fixture.environment,
        &native_platform().unwrap(),
        &staged.reference,
        Some(&staged.digest),
        None,
    )
    .unwrap();
    assert_eq!(qualified.report.operations.len(), 2);
    assert!(qualified.report.installed.is_empty());
    assert_eq!(
        fs::read(staged.stage_path()).unwrap(),
        fs::read(root.join("out/cargo")).unwrap()
    );
    let output = Staged::invoke(
        &staged.fixture,
        &[
            ".",
            ".chrono-harness/ci/build.json",
            "--owner-stage-validation",
            &staged.reference,
            &staged.digest,
        ],
        true,
        true,
        Some(&staged.digest),
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let validation: publication::Validation = serde_json::from_slice(&output.stdout).unwrap();
    assert!(publication::valid_validation(
        &validation,
        &staged.digest,
        &qualified.stage
    ));
}

#[test]
fn owner_stage_rejects_an_original_from_another_physical_host() {
    let staged = Staged::new();
    let foreign = tempfile::tempdir().unwrap();
    let foreign_root = fs::canonicalize(foreign.path()).unwrap();
    let mut report = staged.report.clone();
    report["host_root"] = json!(foreign_root);
    for operation in report["operations"].as_array_mut().unwrap() {
        operation["cwd"] = json!(foreign_root);
    }
    let raw = serde_json::to_vec(&report).unwrap();
    fs::write(staged.fixture.root.join(&staged.reference), &raw).unwrap();
    let result = owner::stage_report(
        &staged.fixture.root,
        &staged.fixture.environment,
        &native_platform().unwrap(),
        &staged.reference,
        Some(&chrono_harness::sha256(&raw)),
        None,
    );
    assert!(
        result.is_err(),
        "a foreign host's stage original was accepted"
    );
}

#[test]
fn owner_stage_refuses_marker_without_descriptors_and_callbacks_without_exact_exclusive_binding() {
    let fixture = Staged::setup();
    for (marker, capabilities) in [(true, false), (false, true)] {
        let output = Staged::invoke(
            &fixture,
            &[
                "construct-owner-stage",
                ".",
                ".chrono-harness/ci/build.json",
            ],
            marker,
            capabilities,
            None,
        );
        assert!(!output.status.success());
        assert!(
            !fixture
                .root
                .join(".chrono-harness/bin/owner-stages")
                .exists()
        );
        assert!(!fixture.root.join("out/cargo").exists());
    }
    let staged = Staged::new();
    let originals = fs::read(staged.fixture.root.join(&staged.reference)).unwrap();
    for (marker, capabilities, deployment) in [
        (true, true, None),
        (true, true, Some("wrong")),
        (false, true, Some(staged.digest.as_str())),
        (true, false, Some(staged.digest.as_str())),
    ] {
        let output = Staged::invoke(
            &staged.fixture,
            &[
                ".",
                ".chrono-harness/ci/build.json",
                "--owner-stage-validation",
                &staged.reference,
                &staged.digest,
            ],
            marker,
            capabilities,
            deployment,
        );
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
    assert_eq!(
        fs::read(staged.fixture.root.join(&staged.reference)).unwrap(),
        originals
    );
}

#[test]
fn selected_owner_qualifies_its_real_ordinary_producer_and_rejects_changed_installation() {
    let fixture = Fixture::new();
    let mut construction = fixture.committed();
    construction
        .prerequisites(&mut vec![], &mut vec![])
        .unwrap();
    construction.select().unwrap();
    construction.operations(&mut vec![], &mut vec![]).unwrap();
    construction
        .install_tool(&fixture.root.join("out/cargo"), ".chrono-harness/bin/cargo")
        .unwrap();
    let result = construction.finish(Ok(())).unwrap();
    assert!(result.outcome.is_ok());
    let native = chrono_host_bootstrap::bindings::NativePlatform {
        system: "fixture".into(),
        machine: "native".into(),
    };
    let qualification = owner::qualified_selected_original(
        &fixture.root,
        &fixture.environment,
        &native,
        ".chrono-harness/state/bootstrap.json",
        ".chrono-harness/bin/cargo",
    )
    .unwrap();
    assert_eq!(
        qualification.sha256,
        chrono_harness::sha256(&fs::read(result.original).unwrap())
    );
    assert_eq!(
        qualification.selected.sha256,
        owner::file_binding(&fixture.root.join(".chrono-harness/bin/cargo"))
            .unwrap()
            .sha256
    );
    fs::write(
        fixture.root.join(".chrono-harness/bin/cargo"),
        b"unproduced bytes",
    )
    .unwrap();
    assert!(
        owner::qualified_selected_original(
            &fixture.root,
            &fixture.environment,
            &native,
            ".chrono-harness/state/bootstrap.json",
            ".chrono-harness/bin/cargo"
        )
        .is_err()
    );
}

#[test]
fn published_owner_qualifies_fixed_producer_after_candidate_changes_and_retains_actual_origins() {
    let staged = Staged::new();
    let adoption = staged.publish_external_inputs();
    fs::write(
        staged.fixture.root.join("next-candidate"),
        b"later candidate",
    )
    .unwrap();
    staged.fixture.commit();
    let qualification = owner::qualified_selected_original(
        &staged.fixture.root,
        &staged.fixture.environment,
        &native_platform().unwrap(),
        ".chrono-harness/state/bootstrap.json",
        ".chrono-harness/bin/cargo",
    )
    .unwrap();
    assert_eq!(qualification.source.commit, adoption["source_commit"]);
    assert_eq!(qualification.path, staged.reference);
    assert_eq!(qualification.sha256, staged.digest);
}

#[test]
fn published_owner_rejects_corrupted_missing_or_misbound_originals() {
    let staged = Staged::new();
    let adoption = staged.publish_external_inputs();
    let root = &staged.fixture.root;
    let stage = staged.stage_path();
    let parent = stage.parent().unwrap();
    let paths = vec![
        root.join(&staged.reference),
        root.join(adoption["receipt"]["path"].as_str().unwrap()),
        root.join(
            staged.report["operations"][0]["stdout"]["path"]
                .as_str()
                .unwrap(),
        ),
        stage.clone(),
        parent.join("previous-owner"),
        parent.join("publication-intent.json"),
        parent.join("publication-result.json"),
    ];
    for path in paths {
        let original = fs::read(&path).unwrap();
        fs::write(&path, b"corrupted original bytes").unwrap();
        assert!(
            owner::qualified_publication(
                root,
                &staged.fixture.environment,
                &native_platform().unwrap(),
                &adoption,
                ".chrono-harness/bin/cargo"
            )
            .is_err(),
            "accepted {}",
            path.display()
        );
        fs::write(&path, &original).unwrap();
        fs::rename(&path, path.with_extension("retained")).unwrap();
        assert!(
            owner::qualified_publication(
                root,
                &staged.fixture.environment,
                &native_platform().unwrap(),
                &adoption,
                ".chrono-harness/bin/cargo"
            )
            .is_err(),
            "accepted missing {}",
            path.display()
        );
        fs::rename(path.with_extension("retained"), &path).unwrap();
    }
    let mut changed = adoption.clone();
    changed["installed"]["identity"] = json!("other inode");
    assert!(
        owner::qualified_publication(
            root,
            &staged.fixture.environment,
            &native_platform().unwrap(),
            &changed,
            ".chrono-harness/bin/cargo"
        )
        .is_err()
    );
    let native = native_platform().unwrap();
    owner::qualified_publication(
        root,
        &staged.fixture.environment,
        &native,
        &adoption,
        ".chrono-harness/bin/cargo",
    )
    .unwrap();
}

#[test]
fn public_bootstrap_seeds_a_real_git_main_and_keeps_the_actual_failed_child_exit() {
    for failure in [None, Some(73)] {
        let mut fixture = Fixture::new();
        let native = native_platform().unwrap();
        let registry_path = fixture.root.join(".chrono-harness/ci/registry.json");
        let mut registry = read_json(&registry_path);
        registry["platforms"][format!("{}/{}", native.system, native.machine)] =
            registry["platforms"]["fixture/native"].clone();
        write_json(&registry_path, &registry);
        fixture.config["participation"]["git"] = json!("/usr/bin/git");
        if let Some(exit) = failure {
            fixture.failed_operation("build", exit);
        }
        fixture.commit();
        let output = Staged::invoke(
            &fixture,
            &[".", ".chrono-harness/ci/build.json"],
            false,
            false,
            None,
        );
        if let Some(exit) = failure {
            assert_eq!(
                output.status.code(),
                Some(exit),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(output.stdout.ends_with(&[0, 255, 10]));
            assert!(output.stderr.windows(3).any(|bytes| bytes == [254, 0, 10]));
            assert!(!fixture.root.join(".chrono-harness/bin/cargo").exists());
            assert!(
                !fixture
                    .root
                    .join(".chrono-harness/state/bootstrap.json")
                    .exists()
            );
        } else {
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let report = read_json(&fixture.root.join(".chrono-harness/state/bootstrap.json"));
            assert_eq!(report["status"], "passed");
            assert_eq!(report["operations"].as_array().unwrap().len(), 2);
            assert_eq!(report["source"]["state"], "clean");
            assert_eq!(
                fs::read(fixture.root.join(".chrono-harness/bin/cargo")).unwrap(),
                fs::read(fixture.root.join("out/cargo")).unwrap()
            );
        }
        let entries: Vec<_> = fs::read_dir(fixture.root.join(".chrono-harness/state"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.is_dir())
            .collect();
        assert_eq!(entries.len(), 1);
        let original = read_json(&entries[0].join("report.json"));
        assert_eq!(
            original["status"],
            if failure.is_some() {
                "failed"
            } else {
                "passed"
            }
        );
        assert_eq!(original["operations"][1]["exit_code"], failure.unwrap_or(0));
    }
}

#[test]
fn owner_reuse_preserves_actual_original_operations_and_rejects_changed_selection_without_reexecution()
 {
    let staged = Staged::new();
    staged.publish_external_inputs();
    let fixture = &staged.fixture;
    let root = &fixture.root;
    let mut config = fixture.config.clone();
    config["owner_stage"] = Value::Null;
    config["owner_upgrade"] = json!(".chrono-harness/ci/upgrade.json");
    config["reuse_owner_operations"] = json!(["build"]);
    let profile =
        chrono_host_bootstrap::profile::Profile::parse(&serde_json::to_vec(&config).unwrap())
            .unwrap();
    let report: chrono_host_bootstrap::construction::Report =
        serde_json::from_value(staged.report.clone()).unwrap();
    let observed = chrono_host_bootstrap::source::observe(root, &fixture.environment);
    let native = native_platform().unwrap();
    let calls = fs::read(fixture.tools.join("calls.json")).unwrap();
    let reused = owner::reuse_construction(
        root,
        &fixture.environment,
        &native,
        &profile,
        &report,
        &observed,
    )
    .unwrap();
    assert_eq!(reused.operations.len(), 1);
    let row = &reused.operations["build"];
    let original = &staged.report["operations"][1];
    assert_eq!(serde_json::to_value(&row.argv).unwrap(), original["argv"]);
    assert_eq!(row.pid as u64, original["pid"].as_u64().unwrap());
    assert_eq!(
        serde_json::to_value(&row.stdout).unwrap(),
        original["stdout"]
    );
    assert_eq!(
        serde_json::to_value(&row.stderr).unwrap(),
        original["stderr"]
    );
    assert_eq!(row.reused_from.as_ref().unwrap().path, staged.reference);
    assert_eq!(row.reused_from.as_ref().unwrap().sha256, staged.digest);
    assert_eq!(reused.source.unwrap(), staged.stage_path());
    for changed in [
        "environment",
        "host_environment",
        "versions",
        "selection",
        "executables",
    ] {
        let mut value = staged.report.clone();
        value[changed] = json!({"changed":"observed value"});
        if let Ok(report) = serde_json::from_value(value) {
            assert!(
                owner::reuse_construction(
                    root,
                    &fixture.environment,
                    &native,
                    &profile,
                    &report,
                    &observed
                )
                .is_err(),
                "accepted {changed}"
            );
        }
    }
    assert_eq!(fs::read(fixture.tools.join("calls.json")).unwrap(), calls);
}
