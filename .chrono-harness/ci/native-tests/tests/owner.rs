#[path = "support/construction.rs"]
mod support;
use chrono_host_bootstrap::{
    bindings::native_platform, lifecycle, owner,
    verified_bindings::owner_publication as publication,
};
use serde_json::{Value, json};
use std::{
    fs,
    os::fd::AsFd,
    path::PathBuf,
    process::{Command, Output},
};
use support::Fixture;

struct UpgradeFixture {
    fixture: Fixture,
}
impl UpgradeFixture {
    fn new() -> Self {
        let mut fixture = Fixture::new();
        let native = native_platform().unwrap();
        let registry_path = fixture.root.join(".chrono-harness/ci/registry.json");
        let mut registry = read_json(&registry_path);
        registry["platforms"][format!("{}/{}", native.system, native.machine)] =
            registry["platforms"]["fixture/native"].clone();
        write_json(&registry_path, &registry);
        fixture.config["participation"]["git"] = json!("/usr/bin/git");
        fixture.config["participation"]["program"] = json!(".chrono-harness/bin/cargo");
        fixture.commit();
        let loaded = chrono_host_bootstrap::profile::Loaded::read(
            &fixture.root,
            ".chrono-harness/ci/build.json",
            &native,
            &fixture.environment,
        )
        .unwrap();
        let mut construction = chrono_host_bootstrap::construction::Construction::begin(
            &fixture.root,
            loaded,
            &fixture.environment,
        )
        .unwrap();
        construction
            .prerequisites(&mut vec![], &mut vec![])
            .unwrap();
        construction.select().unwrap();
        construction.operations(&mut vec![], &mut vec![]).unwrap();
        construction
            .install_tool(&fixture.root.join("out/cargo"), ".chrono-harness/bin/cargo")
            .unwrap();
        assert!(construction.finish(Ok(())).unwrap().outcome.is_ok());
        let mut stage = fixture.config.clone();
        stage["prerequisites"] = json!([]);
        stage["install"] = json!([]);
        stage["report_path"] = json!(".chrono-harness/state/bootstrap-stage.json");
        stage["owner_stage"] = json!({"source":"out/cargo","selected":".chrono-harness/bin/cargo"});
        write_json(&fixture.root.join(".chrono-harness/ci/stage.json"), &stage);
        write_json(
            &fixture.root.join(".chrono-harness/ci/upgrade.json"),
            &json!({
                "schema":"chrono-bootstrap-owner-upgrade/v1", "selected_original":".chrono-harness/state/bootstrap.json",
                "stage_config":".chrono-harness/ci/stage.json",
            }),
        );
        fixture.config["owner_upgrade"] = json!(".chrono-harness/ci/upgrade.json");
        fixture.commit();
        fixture.environment.insert(
            "CHRONO_TEST_OWNER_NATIVE".into(),
            env!("CARGO_BIN_EXE_chrono-test-bootstrap-native").into(),
        );
        fixture.environment.insert(
            "CHRONO_TEST_OWNER_OPERATION".into(),
            env!("CARGO_BIN_EXE_chrono-test-bootstrap-operation").into(),
        );
        Self { fixture }
    }
    fn selected(&self) -> PathBuf {
        self.fixture.root.join(".chrono-harness/bin/cargo")
    }
    fn run(&self) -> Result<(), chrono_host_bootstrap::construction::BuildFailure> {
        let lease = tempfile::tempfile().unwrap();
        let _scope = chrono_harness::process_fds::Scope::new(&[lease.as_fd()]).unwrap();
        lifecycle::upgrade(
            &self.fixture.root,
            ".chrono-harness/ci/build.json",
            &self.fixture.loaded().profile,
            &native_platform().unwrap(),
            &self.fixture.environment,
        )
    }
    fn calls(&self) -> Vec<Value> {
        let path = self
            .fixture
            .root
            .join(".chrono-harness/state/owner-phase-calls.jsonl");
        fs::read_to_string(path)
            .map(|raw| {
                raw.lines()
                    .map(|line| serde_json::from_str(line).unwrap())
                    .collect()
            })
            .unwrap_or_default()
    }
    fn attempt(&self) -> PathBuf {
        let paths: Vec<_> = fs::read_dir(
            self.fixture
                .root
                .join(".chrono-harness/state/bootstrap-owner-upgrade"),
        )
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
        assert_eq!(paths.len(), 1);
        paths.into_iter().next().unwrap()
    }
}

#[test]
fn public_upgrade_uses_qualified_owner_for_real_construction_and_retains_each_consumer() {
    let upgrade = UpgradeFixture::new();
    let before = owner::file_binding(&upgrade.selected()).unwrap();
    upgrade.run().unwrap();
    let calls = upgrade.calls();
    assert_eq!(
        calls
            .iter()
            .map(|call| call["phase"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["migration", "construction", "deployment"]
    );
    let attempt = upgrade.attempt();
    let qualification = read_json(&attempt.join("selected-qualification.json"));
    assert_eq!(
        qualification["selected"],
        serde_json::to_value(&before).unwrap()
    );
    assert_eq!(
        qualification["sha256"],
        chrono_harness::sha256(&fs::read(attempt.join("selected-producer.json")).unwrap())
    );
    for call in &calls {
        let phase = call["phase"].as_str().unwrap();
        let original = read_json(&attempt.join(format!("{phase}.stdout")));
        assert_eq!(
            read_json(&attempt.join(format!("{phase}-consumer.json"))),
            original["report_consumer"]
        );
        assert_eq!(original["report_consumer"]["consumer"], call["consumer"]);
        assert_eq!(
            fs::read(attempt.join(format!("{phase}.exit"))).unwrap(),
            b"0\n"
        );
        assert_eq!(call["program_sha256"], before.sha256);
    }
    for call in &calls[..2] {
        assert_eq!(call["selected"], serde_json::to_value(&before).unwrap());
        assert_eq!(
            PathBuf::from(call["program"].as_str().unwrap()),
            attempt.join("construction-owner")
        );
    }
    assert_ne!(
        owner::file_binding(&upgrade.selected()).unwrap().identity,
        before.identity
    );
    assert!(upgrade.fixture.root.join(owner::CURRENT).is_file());
}

#[test]
fn public_upgrade_rejects_missing_or_misbound_phase_consumers_even_when_wrapper_succeeds() {
    for phase in ["migration", "construction"] {
        for defect in ["missing-consumer", "wrong-consumer", "wrong-original"] {
            let mut upgrade = UpgradeFixture::new();
            upgrade
                .fixture
                .environment
                .insert("CHRONO_TEST_OWNER_DEFECT_PHASE".into(), phase.into());
            upgrade
                .fixture
                .environment
                .insert("CHRONO_TEST_OWNER_DEFECT".into(), defect.into());
            let before = owner::file_binding(&upgrade.selected()).unwrap();
            let failure = upgrade.run().unwrap_err();
            assert!(
                failure.message.contains("report consumer"),
                "{phase}/{defect}: {failure:?}"
            );
            let attempt = upgrade.attempt();
            let original = read_json(&attempt.join(format!("{phase}.stdout")));
            assert_eq!(
                fs::read(attempt.join(format!("{phase}.exit"))).unwrap(),
                b"0\n"
            );
            assert_eq!(
                original["report_path"],
                format!(".chrono-harness/state/worktrees/{phase}.json")
            );
            assert!(!attempt.join(format!("{phase}-consumer.json")).exists());
            assert_eq!(
                upgrade.calls().len(),
                if phase == "migration" { 1 } else { 2 }
            );
            assert!(publication::binding_equal(
                &owner::file_binding(&upgrade.selected()).unwrap(),
                &before
            ));
            assert!(!upgrade.fixture.root.join(owner::CURRENT).exists());
        }
    }
}

#[test]
fn public_upgrade_preserves_original_child_exit_binary_streams_and_consumer_without_deployment() {
    let mut upgrade = UpgradeFixture::new();
    upgrade.fixture.environment.insert(
        "CHRONO_TEST_OWNER_DEFECT_PHASE".into(),
        "construction".into(),
    );
    upgrade
        .fixture
        .environment
        .insert("CHRONO_TEST_OWNER_DEFECT".into(), "child-exit".into());
    let before = owner::file_binding(&upgrade.selected()).unwrap();
    let failure = upgrade.run().unwrap_err();
    assert_eq!(failure.exit_code, Some(7), "{failure:?}");
    let attempt = upgrade.attempt();
    assert_eq!(fs::read(attempt.join("construction.exit")).unwrap(), b"2\n");
    let original = read_json(&attempt.join("construction.stdout"));
    assert_eq!(
        read_json(&attempt.join("construction-consumer.json")),
        original["report_consumer"]
    );
    let process: chrono_harness::ProcessResult = serde_json::from_value(
        chrono_harness::full::expand_process(&original["managed_process"]).unwrap(),
    )
    .unwrap();
    assert_eq!(process.exit_code, 7);
    assert_eq!(process.stdout_bytes, [0, 255, 10]);
    assert_eq!(process.stderr_bytes, [254, 0, 10]);
    assert_eq!(upgrade.calls().len(), 2);
    assert!(publication::binding_equal(
        &owner::file_binding(&upgrade.selected()).unwrap(),
        &before
    ));
    assert!(!upgrade.fixture.root.join(owner::CURRENT).exists());
}

#[test]
fn public_upgrade_refuses_unqualified_producer_before_owner_process_effects() {
    for defect in ["report", "stream", "selected"] {
        let upgrade = UpgradeFixture::new();
        let root = &upgrade.fixture.root;
        let report_path = root.join(".chrono-harness/state/bootstrap.json");
        match defect {
            "report" => fs::remove_file(report_path).unwrap(),
            "stream" => {
                let report = read_json(&report_path);
                fs::write(
                    root.join(report["operations"][0]["stdout"]["path"].as_str().unwrap()),
                    b"changed stream",
                )
                .unwrap();
            }
            "selected" => fs::write(upgrade.selected(), b"unproduced owner").unwrap(),
            _ => unreachable!(),
        }
        let before = owner::file_binding(&upgrade.selected()).unwrap();
        assert!(upgrade.run().is_err(), "accepted {defect}");
        assert!(upgrade.calls().is_empty());
        assert!(
            !root
                .join(".chrono-harness/state/bootstrap-owner-upgrade")
                .exists()
        );
        assert!(publication::binding_equal(
            &owner::file_binding(&upgrade.selected()).unwrap(),
            &before
        ));
    }
}

#[test]
fn shared_main_install_requires_identical_coordinator_bytes() {
    let mut fixture = Fixture::new();
    fixture.config["participation"]["git"] = json!("/usr/bin/git");
    fixture.config["participation"]["program"] = json!(".chrono-harness/bin/cargo");
    let loaded = fixture.commit();
    fixture.environment.insert(
        "CHRONO_WORKTREE_BOOTSTRAP".into(),
        fixture.root.as_os_str().to_owned(),
    );
    let selected = fixture.root.join(".chrono-harness/bin/cargo");
    fs::create_dir_all(selected.parent().unwrap()).unwrap();
    fs::write(&selected, b"original coordinator").unwrap();
    let staged = fixture.root.join("out/cargo");
    fs::write(&staged, b"different owner").unwrap();
    let before = owner::file_binding(&selected).unwrap();
    let rejected = owner::check_shared_install(
        &fixture.root,
        &fixture.environment,
        &loaded.profile,
        &staged,
        &selected,
    )
    .unwrap_err();
    assert!(
        rejected.contains("exclusive owner deployment"),
        "{rejected}"
    );
    fs::write(&staged, fs::read(&selected).unwrap()).unwrap();
    owner::check_shared_install(
        &fixture.root,
        &fixture.environment,
        &loaded.profile,
        &staged,
        &selected,
    )
    .unwrap();
    assert!(publication::binding_equal(
        &owner::file_binding(&selected).unwrap(),
        &before
    ));
}

#[test]
fn linked_shared_install_keeps_coordinator_inode_and_refuses_hardlink_or_symlink_aliases() {
    use std::os::unix::fs::symlink;
    let mut fixture = Fixture::new();
    fixture.config["participation"]["git"] = json!("/usr/bin/git");
    fixture.config["participation"]["program"] = json!(".chrono-harness/bin/cargo");
    let loaded = fixture.commit();
    let selected = fixture.root.join(".chrono-harness/bin/cargo");
    fs::create_dir_all(selected.parent().unwrap()).unwrap();
    fs::write(&selected, b"original coordinator").unwrap();
    let staged = fixture.root.join("out/cargo");
    fs::write(&staged, b"local owner update").unwrap();
    let before = owner::file_binding(&selected).unwrap();
    let linked = fixture.root.join("linked");
    let mut command = Command::new("/usr/bin/git");
    command
        .current_dir(&fixture.root)
        .args(["worktree", "add", "--detach"])
        .arg(&linked)
        .arg("HEAD");
    chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
    let result = command.output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    fixture.environment.insert(
        "CHRONO_WORKTREE_BOOTSTRAP".into(),
        linked.as_os_str().to_owned(),
    );
    let local = linked.join(".chrono-harness/bin/cargo");
    fs::create_dir_all(local.parent().unwrap()).unwrap();
    owner::check_shared_install(
        &linked,
        &fixture.environment,
        &loaded.profile,
        &staged,
        &local,
    )
    .unwrap();
    fs::copy(&staged, &local).unwrap();
    owner::check_shared_install(
        &linked,
        &fixture.environment,
        &loaded.profile,
        &staged,
        &local,
    )
    .unwrap();
    assert_eq!(fs::read(&local).unwrap(), b"local owner update");
    fs::remove_file(&local).unwrap();
    fs::hard_link(&selected, &local).unwrap();
    let rejected = owner::check_shared_install(
        &linked,
        &fixture.environment,
        &loaded.profile,
        &staged,
        &local,
    )
    .unwrap_err();
    assert!(
        rejected.contains("exclusive owner deployment"),
        "{rejected}"
    );
    fs::write(&staged, fs::read(&selected).unwrap()).unwrap();
    owner::check_shared_install(
        &linked,
        &fixture.environment,
        &loaded.profile,
        &staged,
        &local,
    )
    .unwrap();
    fs::write(&staged, b"local owner update").unwrap();
    fs::remove_file(&local).unwrap();
    symlink(&selected, &local).unwrap();
    assert!(
        owner::check_shared_install(
            &linked,
            &fixture.environment,
            &loaded.profile,
            &staged,
            &local
        )
        .unwrap_err()
        .contains("symlink")
    );
    fs::remove_file(&local).unwrap();
    fs::remove_dir(local.parent().unwrap()).unwrap();
    symlink(selected.parent().unwrap(), local.parent().unwrap()).unwrap();
    assert!(
        owner::check_shared_install(
            &linked,
            &fixture.environment,
            &loaded.profile,
            &staged,
            &local
        )
        .unwrap_err()
        .contains("symlink")
    );
    assert!(publication::binding_equal(
        &owner::file_binding(&selected).unwrap(),
        &before
    ));
}

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
    fn invoke(fixture: &Fixture, arguments: &[&str], marker: bool, capabilities: bool) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-test-bootstrap-native"));
        command
            .current_dir(&fixture.root)
            .args(arguments)
            .env_clear()
            .envs(fixture.environment.iter().filter(|(key, _)| {
                key.as_os_str() != std::ffi::OsStr::new(chrono_harness::process_fds::ENV)
            }));
        command.env_remove("CHRONO_WORKTREE_BOOTSTRAP");
        if marker {
            command.env("CHRONO_WORKTREE_BOOTSTRAP", &fixture.root);
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
        Self::from_fixture(Self::setup())
    }
    fn from_fixture(fixture: Fixture) -> Self {
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
fn native_owner_stage_retains_actual_execution_and_selected_inode() {
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
fn owner_stage_refuses_marker_without_descriptors() {
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
}

#[test]
fn selected_owner_qualifies_its_fixed_ordinary_producer_after_source_changes_and_rejects_substitution()
 {
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
    let original: Value = serde_json::from_slice(&fs::read(&result.original).unwrap()).unwrap();
    fs::write(fixture.root.join("later-source"), b"later candidate\n").unwrap();
    fixture.commit();
    assert_ne!(
        chrono_host_bootstrap::source::observe(&fixture.root, &fixture.environment).commit,
        original["source"]["commit"].as_str().map(str::to_owned)
    );
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
        chrono_harness::sha256(&fs::read(&result.original).unwrap())
    );
    assert_eq!(qualification.source.commit, original["source"]["commit"]);
    assert_eq!(
        qualification.selected.sha256,
        owner::file_binding(&fixture.root.join(".chrono-harness/bin/cargo"))
            .unwrap()
            .sha256
    );
    let report_path = fixture.root.join(".chrono-harness/state/bootstrap.json");
    let original_bytes = fs::read(&report_path).unwrap();
    let mut wrong_source = original.clone();
    wrong_source["source"]["commit"] = json!("0".repeat(40));
    write_json(&report_path, &wrong_source);
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
    fs::write(&report_path, &original_bytes).unwrap();
    let unproduced = fixture.root.join(".chrono-harness/bin/unproduced");
    fs::copy(fixture.root.join(".chrono-harness/bin/cargo"), &unproduced).unwrap();
    assert!(
        owner::qualified_selected_original(
            &fixture.root,
            &fixture.environment,
            &native,
            ".chrono-harness/state/bootstrap.json",
            ".chrono-harness/bin/unproduced"
        )
        .unwrap_err()
        .contains("producer destination differs")
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
fn published_owner_rejects_failed_production_even_when_all_outer_digests_are_recomputed() {
    let staged = Staged::new();
    let mut adoption = staged.publish_external_inputs();
    let root = &staged.fixture.root;
    let mut report = staged.report.clone();
    report["status"] = json!("failed");
    write_json(&root.join(&staged.reference), &report);
    adoption["stage_sha256"] = json!(chrono_harness::sha256(
        &fs::read(root.join(&staged.reference)).unwrap()
    ));
    let receipt = root.join(adoption["receipt"]["path"].as_str().unwrap());
    let mut original = adoption.clone();
    original.as_object_mut().unwrap().remove("receipt");
    write_json(&receipt, &original);
    adoption["receipt"]["sha256"] = json!(chrono_harness::sha256(&fs::read(&receipt).unwrap()));
    write_json(&root.join(owner::CURRENT), &adoption);
    let before = owner::file_binding(&root.join(".chrono-harness/bin/cargo")).unwrap();
    let failure = owner::qualified_selected_original(
        root,
        &staged.fixture.environment,
        &native_platform().unwrap(),
        ".chrono-harness/state/bootstrap.json",
        ".chrono-harness/bin/cargo",
    )
    .unwrap_err();
    assert!(
        failure.contains("original producer header differs"),
        "{failure}"
    );
    assert!(publication::binding_equal(
        &owner::file_binding(&root.join(".chrono-harness/bin/cargo")).unwrap(),
        &before
    ));
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
    let mut unrelated = config.clone();
    unrelated["tools"]["unrelated"] = json!("an-unselected-tool");
    let unrelated =
        chrono_host_bootstrap::profile::Profile::parse(&serde_json::to_vec(&unrelated).unwrap())
            .unwrap();
    assert!(
        owner::reuse_construction(
            root,
            &fixture.environment,
            &native,
            &unrelated,
            &report,
            &observed
        )
        .is_ok()
    );
    for defect in ["tool", "environment", "operation", "candidate"] {
        let mut changed = config.clone();
        let mut source = chrono_host_bootstrap::source::SourceObservation {
            commit: observed.commit.clone(),
            tree: observed.tree.clone(),
            dirty: observed.dirty,
            error: observed.error.clone(),
        };
        match defect {
            "tool" => changed["tools"][&row.tool] = json!("another-reused-tool"),
            "environment" => changed["environment"] = json!({"CARGO_NET_OFFLINE":"true"}),
            "operation" => {
                changed["operations"] = json!(["unproduced"]);
                changed["reuse_owner_operations"] = json!(["unproduced"]);
            }
            "candidate" => source.commit = Some("0".repeat(40)),
            _ => unreachable!(),
        }
        let changed =
            chrono_host_bootstrap::profile::Profile::parse(&serde_json::to_vec(&changed).unwrap())
                .unwrap();
        assert!(
            owner::reuse_construction(
                root,
                &fixture.environment,
                &native,
                &changed,
                &report,
                &source
            )
            .is_err(),
            "accepted {defect}"
        );
    }
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

#[test]
fn public_core_reuses_the_published_stage_without_rebuilding_or_replacing_the_selected_owner() {
    let fixture = Staged::setup();
    let mut core = fixture.config.clone();
    core["owner_stage"] = Value::Null;
    core["owner_upgrade"] = json!(".chrono-harness/ci/upgrade.json");
    core["reuse_owner_operations"] = json!(["build"]);
    core["report_path"] = json!(".chrono-harness/state/bootstrap.json");
    core["install"] = json!([{"source":"out/cargo","destination":".chrono-harness/bin/cargo"}]);
    write_json(&fixture.root.join(".chrono-harness/ci/core.json"), &core);
    write_json(
        &fixture.root.join(".chrono-harness/ci/upgrade.json"),
        &json!({"schema":"chrono-bootstrap-owner-upgrade/v1",
            "stage_config":".chrono-harness/ci/build.json",
            "selected_original":".chrono-harness/state/bootstrap.json"}),
    );
    fixture.commit();
    let staged = Staged::from_fixture(fixture);
    staged.publish_external_inputs();
    let root = &staged.fixture.root;
    let selected = root.join(".chrono-harness/bin/cargo");
    let before = owner::file_binding(&selected).unwrap();
    let build_count = || {
        read_json(&staged.fixture.tools.join("calls.json"))
            .as_array()
            .unwrap()
            .iter()
            .filter(|call| call[0] == "cargo" && call[1] == "build-fixture")
            .count()
    };
    let original_builds = build_count();
    assert_eq!(original_builds, 1);
    fs::write(root.join("out/cargo"), b"unrelated scratch rebuild").unwrap();
    let output = Staged::invoke(
        &staged.fixture,
        &[".", ".chrono-harness/ci/core.json"],
        true,
        true,
    );
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(build_count(), original_builds);
    assert!(publication::binding_equal(
        &owner::file_binding(&selected).unwrap(),
        &before
    ));
    let result = read_json(&root.join(".chrono-harness/state/bootstrap.json"));
    assert_eq!(result["status"], "passed");
    assert_eq!(result["operations"].as_array().unwrap().len(), 2);
    let mut reused = result["operations"][1].clone();
    assert_eq!(
        reused.as_object_mut().unwrap().remove("reused_from"),
        Some(json!({"path":staged.reference,"sha256":staged.digest}))
    );
    assert_eq!(reused, staged.report["operations"][1]);
    let validation = Staged::invoke(
        &staged.fixture,
        &[
            "validate-original-execution",
            ".",
            ".chrono-harness/state/bootstrap.json",
        ],
        false,
        false,
    );
    assert!(
        validation.status.success(),
        "{}",
        String::from_utf8_lossy(&validation.stderr)
    );
}
