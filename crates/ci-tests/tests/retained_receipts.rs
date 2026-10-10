use super::*;
use std::path::PathBuf;

const FAMILY: &str = ".chrono-harness/state/ci-test-failures/";
const GENERATED: &[&str] = &[
    "original-state/cargo/producer/target/",
    "original-state/cargo/tests/target/",
];

pub(super) struct Owner {
    _dir: chrono_worktree::TemporaryHost,
    pub(super) root: PathBuf,
}

impl Drop for Owner {
    fn drop(&mut self) {
        if std::thread::panicking() {
            let destination = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(FAMILY)
                .join(format!(
                    "owner-fixture-{}",
                    chrono_harness::sha256(self.root.as_os_str().as_encoded_bytes())
                ));
            retain_context_state(&self.root.join(".chrono-harness/state"), &destination);
            retain_context_state(
                &self._dir.path().join("linked/.chrono-harness/state"),
                &destination.join("linked-state"),
            );
            eprintln!(
                "original receipt-owner failure evidence: {}",
                destination.display()
            );
        }
    }
}

impl Owner {
    pub(super) fn new() -> Self {
        let dir = crate::tools::temporary_host("host λ ");
        let root = fs::canonicalize(dir.path()).unwrap().join("main");
        fs::create_dir(&root).unwrap();
        git(&root, &["init", "-q", "-b", "dev"]);
        git(&root, &["config", "user.email", "fixture@example.invalid"]);
        git(&root, &["config", "user.name", "Fixture"]);
        let config = ".chrono-harness/config.json";
        let fm = ".chrono-harness/FILEMAP.json";
        let projects = ".chrono-harness/projects.json";
        let judges = ".chrono-harness/judges.json";
        let workflow = ".chrono-harness/workflow.json";
        let worktree = ".chrono-harness/worktree.json";
        let cleanup = ".chrono-harness/cleanup.json";
        fs::create_dir_all(root.join(".chrono-harness")).unwrap();
        for (path, value) in [
            (
                config,
                json!({"schema_version":1,"status":"proposed","enforcement":"not-implemented",
                "runner":{"path":".chrono-harness/bin/chrono-harness","version":"0.1.0","sha256":null},
                "registries":{"filemap":fm,"projects":projects,"judges":judges,"workflow":workflow},
                "canonical_check":{"operation":"validate.delta","argv":[".chrono-harness/bin/chrono-harness","check","--config",config,"--base","{base}","--candidate","{candidate}","--context",".chrono-harness/state/context.json"]},
                "tools":[],"environment":{"inherit":["PATH"],"values":{},"inputs":[]},
                "input_closure":{"status":"incomplete","unresolved":["fixture only"]},
                "protocol":{"id":"chrono-judge/v1","timeout_seconds":30,"stdout_limit_bytes":8388608,"encoding":"UTF-8"},"semantic_fields":[],
                "artifacts":[{"path":".chrono-harness/bin/","owner":"host","kind":"executable","tracked":false},{"path":".chrono-harness/state/","owner":"host","kind":"evidence","tracked":false}]}),
            ),
            (
                fm,
                json!({"schema_version":1,"status":"proposed","files":([config,fm,projects,judges,workflow,worktree,cleanup,".gitignore"].iter().map(|p|json!({"path":p,"owner":"host","surface":"product","cost":"unknown","edges":[]})).collect::<Vec<_>>()),
                "project_edges":[],"cost_models":{"unknown":{"cpu_ms":null,"wall_ms":null,"peak_rss_bytes":null,"io_bytes":null,"basis":"unmeasured fixture"}},"test_costs":[]}),
            ),
            (
                projects,
                json!({"schema_version":1,"status":"proposed","owners":["host"],"projects":[],"scripts":[]}),
            ),
            (
                judges,
                json!({"schema_version":1,"status":"proposed","migration_validator":"registration","judges":[{"id":"registration","executable":".chrono-harness/bin/chrono-judge-registration","version":"0.1.0","sha256":null,"argv":["--protocol","chrono-judge/v1"],"selector":"every-delta","after":[],"modes":["evaluate"]}]}),
            ),
            (
                workflow,
                json!({"schema_version":1,"status":"proposed","target_branch":"dev","feature_prefix":"feature/","integration_prefix":"integration/",
                "staleness":{"max_behind_commits":3,"max_age_hours":24,"combine":"any-exceeded"},"stability":[],"semantic_changes_require_integration":true,
                "mixed_change":{"level":"warning","code":"W_MIXED","requires_acknowledgement":false},"integration":{"tests":[],"bind":[],"evidence":".chrono-harness/state/integration.json"},"retirements":[],"migrations":[]}),
            ),
            (
                worktree,
                json!({"schema":"chrono-worktree-config/v1","automatic_cleanup":cleanup,"host_config":config,"remote":"origin",
                "git":{"program":fixture_git(),"expected_version":null,"sha256":null},"environment":{"inherit":["PATH"],"values":{}},
                "timeout_seconds":30,"output_limit_bytes":1048576,"report_directory":".chrono-harness/state/worktrees/"}),
            ),
            (
                cleanup,
                json!({"schema":"chrono-worktree-automatic-cleanup/v2","coordinator_root":"git-main-worktree",
                "state_directory":".chrono-harness/state/automatic-cleanup/","retained_ref":"refs/heads/dev","retention":"same-tree","remove_branch":false,"allow_evidence_disposal":false,
                "artifacts":[{"path":".chrono-harness/bin/","disposition":"retain"},{"path":".chrono-harness/state/","disposition":"evidence-retain"}],
                "retained_producers":[{"id":"ci-command-result","directory":FAMILY,"generated_outputs":GENERATED}]}),
            ),
        ] {
            fs::write(root.join(path), serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        }
        fs::write(
            root.join(".gitignore"),
            ".chrono-harness/state/\n.chrono-harness/bin/\n",
        )
        .unwrap();
        git(&root, &["add", "."]);
        git(
            &root,
            &["commit", "-qm", "explicit receipt lifecycle fixture"],
        );
        Self { _dir: dir, root }
    }

    fn drain(&self) -> (i32, Value) {
        let output = chrono_worktree::run(&[
            "maintain".into(),
            "--host-root".into(),
            self.root.to_str().unwrap().into(),
        ]);
        let report: Value = serde_json::from_str(&output.stdout).expect(&output.stderr);
        (output.exit_code.into(), report)
    }

    fn participating(&self) {
        let config = self.root.join(".chrono-harness/config.json");
        let mut value: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
        value["tools"] = json!([{"id":"git","program":fixture_git(),"resolution":"PATH-once","version_argv":["--version"],"expected_version":null}]);
        fs::write(config, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        let projects = self.root.join(".chrono-harness/projects.json");
        let mut value: Value = serde_json::from_slice(&fs::read(&projects).unwrap()).unwrap();
        value["owners"] = json!(["host", "host-tests"]);
        value["scripts"] = json!([
            {"id":"host","path":".gitignore","test_script":"host-tests","actions":{"execute":{"operation":"use.receipt-owner","tool":"git","argv":["status","--short"]}}},
            {"id":"host-tests","path":".chrono-harness/workflow.json","tests_for":"host","actions":{"execute":{"operation":"test.receipt-owner","tool":"git","argv":["status","--short"]}}}
        ]);
        fs::write(projects, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        let cleanup = self.root.join(".chrono-harness/cleanup.json");
        let mut value: Value = serde_json::from_slice(&fs::read(&cleanup).unwrap()).unwrap();
        value["main_cache_only"] = json!(true);
        fs::write(cleanup, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        git(&self.root, &["add", "."]);
        git(&self.root, &["commit", "-qm", "registered owner consumer"]);
    }

    fn reenter(&self, invoking: &Path) -> (i32, Value) {
        let output = chrono_worktree::run(&[
            "use".into(),
            "--host-root".into(),
            invoking.to_str().unwrap().into(),
            "--path".into(),
            self.root.to_str().unwrap().into(),
            "--operation".into(),
            "use.receipt-owner".into(),
        ]);
        let report: Value = serde_json::from_str(&output.stdout).expect(&output.stderr);
        (output.exit_code.into(), report)
    }

    fn object(&self) -> PathBuf {
        let registry: Value =
            serde_json::from_slice(&fs::read(self.root.join(FAMILY).join("objects.json")).unwrap())
                .unwrap();
        self.root
            .join(FAMILY)
            .join(registry["objects"][0]["path"].as_str().unwrap())
    }
}

#[test]
fn linked_default_wrapper_reclaims_released_copies_on_ordinary_owner_reentry() {
    assert!(std::env::var_os("CHRONO_CI_TEST_RECEIPTS").is_none());
    let owner = Owner::new();
    owner.participating();
    let linked = fs::canonicalize(owner._dir.path()).unwrap().join("linked");
    git(
        &owner.root,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "feature/receipt",
            linked.to_str().unwrap(),
        ],
    );
    let attachment = fs::read(linked.join(".git")).unwrap();
    let head = git(&linked, &["rev-parse", "HEAD"]);
    let index = fs::read(owner.root.join(".git/index")).unwrap();
    let (execution, output, argv) = cargo_result(true);
    let receipt = retain_expected_command_result(
        &linked,
        execution.path(),
        &argv,
        &json!({"consumer":"linked default receipt producer"}),
        &output,
        Some(101),
    )
    .unwrap();
    let object = owner.object();
    assert!(!linked.join(FAMILY).exists());
    let binding = fs::read(object.join("binding.json")).unwrap();
    let decoded: Value = serde_json::from_slice(&binding).unwrap();
    assert_eq!(decoded["exit"], 101);
    assert_eq!(decoded["expected_exit"], 101);
    assert!(String::from_utf8_lossy(&output.stdout).contains("original business exception"));
    receipt
        .acquire("diagnostic", "original failure evidence still consumed")
        .unwrap();
    receipt
        .release(
            "command-assertion",
            "expected failure and original streams checked",
        )
        .unwrap();
    let (code, protected) = owner.reenter(&linked);
    assert_eq!(code, 0, "{protected}");
    assert_eq!(protected["producer_objects"][0]["status"], "protected");
    assert!(object.join(GENERATED[1]).exists());
    receipt
        .release("diagnostic", "real diagnostic consumer finished")
        .unwrap();
    drop(receipt);
    let (code, reclaimed) = owner.reenter(&linked);
    assert_eq!(code, 0, "{reclaimed}");
    assert_eq!(reclaimed["producer_objects"][0]["status"], "cleaned");
    for path in GENERATED {
        assert!(!object.join(path).exists());
    }
    assert_eq!(fs::read(object.join("stdout.bin")).unwrap(), output.stdout);
    assert_eq!(fs::read(object.join("stderr.bin")).unwrap(), output.stderr);
    assert_eq!(fs::read(object.join("binding.json")).unwrap(), binding);
    assert!(object.join("original-state/recovery/original").exists());
    assert!(
        object
            .join("original-state/cargo/tests/src/lib.rs")
            .exists()
    );
    assert!(object.join("configuration/config.json").exists());
    assert_eq!(fs::read(linked.join(".git")).unwrap(), attachment);
    assert_eq!(git(&linked, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        git(&linked, &["symbolic-ref", "HEAD"]),
        "refs/heads/feature/receipt"
    );
    assert_eq!(fs::read(owner.root.join(".git/index")).unwrap(), index);
    assert!(owner.root.is_dir());
    assert_eq!(
        owner.reenter(&linked).1["producer_objects"][0]["status"],
        "already-disposed"
    );
}

#[test]
fn producer_store_resolution_rejects_undeclared_ambiguous_drifted_and_symlink_routes() {
    let config = ".chrono-harness/worktree.json";
    for kind in [
        "undeclared",
        "duplicate",
        "overlap",
        "drift",
        "symlink",
        "linked-mismatch",
    ] {
        let owner = Owner::new();
        let unknown = owner._dir.path().join("unknown-store");
        fs::create_dir(&unknown).unwrap();
        fs::write(unknown.join("original"), "unknown original destination").unwrap();
        let path = owner.root.join(".chrono-harness/cleanup.json");
        let mut policy: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let mut invoking = owner.root.clone();
        match kind {
            "undeclared" => policy["retained_producers"] = json!([]),
            "duplicate" => {
                let duplicate = policy["retained_producers"][0].clone();
                policy["retained_producers"]
                    .as_array_mut()
                    .unwrap()
                    .push(duplicate);
            }
            "overlap" => policy["retained_producers"]
                .as_array_mut()
                .unwrap()
                .push(json!({
                    "id":"other","directory":format!("{FAMILY}nested/"),"generated_outputs":[]
                })),
            "drift" => {
                policy["retained_producers"][0]["directory"] =
                    json!(".chrono-harness/state/undeployed/")
            }
            "symlink" => {
                fs::create_dir_all(owner.root.join(".chrono-harness/state")).unwrap();
                std::os::unix::fs::symlink(&unknown, owner.root.join(FAMILY.trim_end_matches('/')))
                    .unwrap();
            }
            "linked-mismatch" => {
                invoking = fs::canonicalize(owner._dir.path()).unwrap().join("linked");
                git(
                    &owner.root,
                    &[
                        "worktree",
                        "add",
                        "-q",
                        "-b",
                        "feature/mismatched",
                        invoking.to_str().unwrap(),
                    ],
                );
                policy["retained_producers"][0]["directory"] =
                    json!(".chrono-harness/state/mismatched/");
                fs::write(
                    invoking.join(".chrono-harness/cleanup.json"),
                    serde_json::to_vec_pretty(&policy).unwrap(),
                )
                .unwrap();
                git(&invoking, &["add", "."]);
                git(&invoking, &["commit", "-qm", "incompatible linked policy"]);
            }
            _ => unreachable!(),
        }
        if !["symlink", "linked-mismatch"].contains(&kind) {
            fs::write(&path, serde_json::to_vec_pretty(&policy).unwrap()).unwrap();
            if kind != "drift" {
                git(&owner.root, &["add", "."]);
                git(
                    &owner.root,
                    &["commit", "-qm", "explicit invalid producer route"],
                );
            }
        }
        let error = chrono_worktree::RetainedArtifact::registered_store(
            &invoking,
            config,
            "ci-command-result",
        )
        .unwrap_err();
        let expected = match kind {
            "undeclared" => "not declared",
            "duplicate" | "overlap" => "repeated or overlapping",
            "drift" => "differs from source commit",
            "linked-mismatch" => "coordinator policy/configuration identity mismatch",
            "symlink" => "symlink",
            _ => unreachable!(),
        };
        assert!(error.contains(expected), "{kind}: {error}");
        assert_eq!(
            fs::read_to_string(unknown.join("original")).unwrap(),
            "unknown original destination"
        );
        assert!(!unknown.join("objects.json").exists());
        assert!(owner.root.join(".git").is_dir());
    }
}

#[test]
fn producer_wrapper_refuses_unknown_destination_before_publication() {
    let owner = Owner::new();
    let (execution, output, argv) = cargo_result(true);
    let unknown = fs::canonicalize(owner._dir.path())
        .unwrap()
        .join("unknown-store");
    fs::create_dir(&unknown).unwrap();
    fs::write(unknown.join("original"), "unknown legacy bytes").unwrap();
    let before = fs::read(
        execution
            .path()
            .join(".chrono-harness/state/recovery/original"),
    )
    .unwrap();
    let rejected = std::panic::catch_unwind(|| {
        retain_command_result_in(
            &owner.root,
            execution.path(),
            &argv,
            &json!({}),
            &output,
            Some(101),
            Some(&unknown),
        )
    });
    assert!(rejected.is_err());
    assert_eq!(
        fs::read_to_string(unknown.join("original")).unwrap(),
        "unknown legacy bytes"
    );
    assert_eq!(fs::read_dir(&unknown).unwrap().count(), 1);
    assert!(!owner.root.join(FAMILY).exists());
    assert_eq!(
        fs::read(
            execution
                .path()
                .join(".chrono-harness/state/recovery/original")
        )
        .unwrap(),
        before
    );
}

fn cargo_result(
    fail: bool,
) -> (
    chrono_worktree::TemporaryHost,
    std::process::Output,
    Vec<String>,
) {
    let dir = crate::tools::temporary_host("host λ ");
    let fixtures =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../worktree-tests/tests/support/cargo_pair");
    for project in ["producer", "tests"] {
        for name in ["Cargo.toml", "Cargo.lock", "src/lib.rs"] {
            let target = dir
                .path()
                .join(format!(".chrono-harness/state/cargo/{project}/{name}"));
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::copy(fixtures.join(project).join(name), target).unwrap();
        }
    }
    fs::write(dir.path().join(".chrono-harness/config.json"), serde_json::to_vec(&json!({
        "artifacts":(["producer","tests"].iter().map(|p| json!({"path":format!(".chrono-harness/state/cargo/{p}/target/"),"tracked":false})).collect::<Vec<_>>())
    })).unwrap()).unwrap();
    let mut build = std::process::Command::new("cargo");
    build
        .env_remove("CARGO_TARGET_DIR")
        .current_dir(dir.path())
        .args([
            "build",
            "--locked",
            "--offline",
            "--manifest-path",
            ".chrono-harness/state/cargo/producer/Cargo.toml",
        ]);
    let build = dir.capture_output(&mut build).unwrap();
    assert!(build.status.success(), "{build:?}");
    if fail {
        let input = dir
            .path()
            .join(".chrono-harness/state/cargo/tests/src/lib.rs");
        let mut bytes = fs::read(&input).unwrap();
        bytes.extend_from_slice(b"\n#[test] fn actual_failure() { assert_eq!(bounded_cache_producer::answer(), 0, \"original business exception\"); }\n");
        fs::write(input, bytes).unwrap();
        fs::create_dir_all(dir.path().join(".chrono-harness/state/recovery")).unwrap();
        fs::write(
            dir.path().join(".chrono-harness/state/recovery/original"),
            "required original recovery input",
        )
        .unwrap();
    }
    let argv = vec![
        "cargo".into(),
        "test".into(),
        "--locked".into(),
        "--offline".into(),
        "--manifest-path".into(),
        ".chrono-harness/state/cargo/tests/Cargo.toml".into(),
    ];
    let mut command = std::process::Command::new(&argv[0]);
    command
        .env_remove("CARGO_TARGET_DIR")
        .current_dir(dir.path())
        .args(&argv[1..]);
    let output = dir.capture_output(&mut command).unwrap();
    assert_eq!(
        output.status.code(),
        Some(if fail { 101 } else { 0 }),
        "{output:?}"
    );
    (dir, output, argv)
}

#[test]
fn real_receipt_producer_expected_failure_diagnostic_release_and_legacy_unknown() {
    for fail in [false, true] {
        let owner = Owner::new();
        let (execution, output, argv) = cargo_result(fail);
        let directory = owner.root.join(FAMILY);
        let receipt = retain_command_result_in(
            &owner.root,
            execution.path(),
            &argv,
            &json!({"consumer":"real Cargo child"}),
            &output,
            Some(if fail { 101 } else { 0 }),
            Some(&directory),
        );
        let object = owner.object();
        let original_stdout = fs::read(object.join("stdout.bin")).unwrap();
        let original_stderr = fs::read(object.join("stderr.bin")).unwrap();
        assert_eq!(original_stdout, output.stdout);
        assert_eq!(original_stderr, output.stderr);
        let legacy = directory.join("legacy-unknown/original-state/cargo/tests/target");
        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join("unknown"), "historical evidence").unwrap();
        receipt
            .acquire(
                "diagnostic",
                "active original error/partial-effect investigation",
            )
            .unwrap();
        receipt
            .release("command-assertion", "original expected exit checked")
            .unwrap();
        let (code, active) = owner.drain();
        assert_eq!(code, 0, "{active}");
        assert_eq!(active["producer_objects"][0]["status"], "protected");
        assert!(object.join(GENERATED[1]).is_dir());
        receipt
            .release(
                "diagnostic",
                "diagnostic complete; generated copies no longer consumed",
            )
            .unwrap();
        let (code, released) = owner.drain();
        assert_eq!(code, 0, "{released}");
        assert_eq!(released["producer_objects"][0]["status"], "cleaned");
        for path in GENERATED {
            assert!(!object.join(path).exists());
        }
        assert_eq!(
            fs::read(object.join("stdout.bin")).unwrap(),
            original_stdout
        );
        assert_eq!(
            fs::read(object.join("stderr.bin")).unwrap(),
            original_stderr
        );
        assert!(
            object
                .join("original-state/cargo/tests/src/lib.rs")
                .exists()
        );
        if fail {
            assert!(object.join("original-state/recovery/original").exists());
        }
        assert_eq!(
            fs::read_to_string(legacy.join("unknown")).unwrap(),
            "historical evidence"
        );
        assert_eq!(
            owner.drain().1["producer_objects"][0]["status"],
            "already-disposed"
        );
        assert!(
            receipt
                .acquire("late", "requires already reclaimed generated bytes")
                .is_err()
        );
    }
}

#[test]
fn receipt_interrupted_publication_and_unreleased_failure_remain_protected_on_reentry() {
    let owner = Owner::new();
    let directory = owner.root.join(FAMILY);
    fs::create_dir_all(&directory).unwrap();
    let partial = directory.join("interrupted-exact-object");
    fs::create_dir_all(partial.join(GENERATED[0])).unwrap();
    fs::write(
        partial.join(GENERATED[0]).join("partial"),
        "original partial effect",
    )
    .unwrap();
    let handle = chrono_worktree::RetainedArtifact::begin(
        &directory,
        "ci-command-result",
        &partial,
        &GENERATED.iter().map(|p| p.to_string()).collect::<Vec<_>>(),
        "command-assertion",
        "interrupted publication",
    )
    .unwrap();
    drop(handle);
    let (code, interrupted) = owner.drain();
    assert_eq!(code, 0, "{interrupted}");
    assert_eq!(interrupted["producer_objects"][0]["status"], "protected");
    assert!(partial.join(GENERATED[0]).join("partial").exists());
    let handle =
        chrono_worktree::RetainedArtifact::resume(&directory, "ci-command-result", &partial)
            .unwrap();
    assert!(
        handle
            .release("command-assertion", "no original output yet")
            .is_err()
    );
    for (name, bytes) in [
        ("stdout.bin", b"original stdout".as_slice()),
        ("stderr.bin", b"original error chain".as_slice()),
        ("binding.json", b"{\"exit\":23}".as_slice()),
    ] {
        fs::write(partial.join(name), bytes).unwrap();
    }
    handle.seal().unwrap();
    assert_eq!(
        owner.drain().1["producer_objects"][0]["status"],
        "protected"
    );
    handle
        .release("command-assertion", "explicit recovered producer release")
        .unwrap();
    assert_eq!(owner.drain().0, 0);
    assert!(!partial.join(GENERATED[0]).exists());
    assert_eq!(
        fs::read(partial.join("stderr.bin")).unwrap(),
        b"original error chain"
    );
}

#[test]
fn receipt_partial_disposal_retry_preserves_real_failure_and_remaining_objects() {
    use std::os::unix::fs::PermissionsExt;
    let owner = Owner::new();
    let (execution, output, argv) = cargo_result(true);
    let receipt = retain_command_result_in(
        &owner.root,
        execution.path(),
        &argv,
        &json!({}),
        &output,
        Some(0),
        Some(&owner.root.join(FAMILY)),
    );
    receipt
        .release(
            "command-assertion",
            "failure diagnosed; generated copies released",
        )
        .unwrap();
    let object = owner.object();
    let second = object.join(GENERATED[1]);
    fs::set_permissions(&second, fs::Permissions::from_mode(0o500)).unwrap();
    let (code, failed) = owner.drain();
    fs::set_permissions(&second, fs::Permissions::from_mode(0o700)).unwrap();
    assert_ne!(code, 0, "{failed}");
    assert!(!object.join(GENERATED[0]).exists());
    assert!(second.exists());
    assert!(
        failed
            .to_string()
            .contains("failed-partial-effects-possible")
    );
    let original = fs::read(object.join("stdout.bin")).unwrap();
    let (code, retried) = owner.drain();
    assert_eq!(code, 0, "{retried}");
    assert!(retried.to_string().contains("already-absent"));
    assert!(!second.exists());
    assert_eq!(fs::read(object.join("stdout.bin")).unwrap(), original);
    let binding: Value =
        serde_json::from_slice(&fs::read(object.join("binding.json")).unwrap()).unwrap();
    assert_eq!(binding["exit"], 101);
    assert!(
        binding["retention_reason"]
            .as_str()
            .unwrap()
            .contains("unexpected-command-exit")
    );
    assert_eq!(
        owner.drain().1["producer_objects"][0]["status"],
        "already-disposed"
    );
}

#[test]
fn producer_registry_birth_serializes_create_before_lock_and_retains_both_commands() {
    use std::{os::unix::fs::MetadataExt, sync::mpsc, time::Duration};
    let owner = Owner::new();
    let directory = owner.root.join(FAMILY);
    let commands = [cargo_result(true), cargo_result(false)];
    let (created_tx, created_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let (busy_tx, busy_rx) = mpsc::channel();
    let (finished_tx, finished_rx) = mpsc::channel();
    let (first, second) = std::thread::scope(|scope| {
        let root = &owner.root;
        let command = (commands[0].0.path(), &commands[0].1, &commands[0].2);
        let store = &directory;
        let first = scope.spawn(move || {
            let mut observer = |phase: &str| {
                if phase == "lease-created-before-lock" {
                    created_tx.send(()).unwrap();
                    resume_rx.recv_timeout(Duration::from_secs(10)).unwrap();
                }
            };
            retain_command_result_observed(
                root,
                command.0,
                &command.2,
                &json!({"publisher":0}),
                &command.1,
                Some(101),
                Some(store),
                Some(&mut observer),
            )
        });
        created_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(!directory.join("objects.json").exists());
        let lease_path = directory.join("registry.lease");
        let lease_identity = fs::metadata(&lease_path).unwrap();
        // Prove A is paused after creating the visible lease and before flock.
        // Release this probe before B starts, allowing B to take the old gap.
        let lease = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lease_path)
            .unwrap();
        lease.try_lock().unwrap();
        drop(lease);
        let first_copy = fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| path.is_dir())
            .unwrap();
        assert_eq!(
            fs::read(first_copy.join("stdout.bin")).unwrap(),
            commands[0].1.stdout
        );
        assert!(first_copy.join("original-state/recovery/original").exists());
        let command = (commands[1].0.path(), &commands[1].1, &commands[1].2);
        let root = &owner.root;
        let store = &directory;
        let second = scope.spawn(move || {
            let mut reported_busy = false;
            let mut observer = |phase: &str| {
                if phase == "birth-gate-busy" && !reported_busy {
                    reported_busy = true;
                    busy_tx.send(()).unwrap();
                }
            };
            let receipt = retain_command_result_observed(
                root,
                command.0,
                command.2,
                &json!({"publisher":1}),
                command.1,
                Some(0),
                Some(store),
                Some(&mut observer),
            );
            finished_tx.send(()).unwrap();
            receipt
        });
        let busy = busy_rx.recv_timeout(Duration::from_secs(5));
        let waiting = matches!(finished_rx.try_recv(), Err(mpsc::TryRecvError::Empty));
        let registry_absent = !directory.join("objects.json").exists();
        // Always unblock A and join both publishers before evaluating the result.
        resume_tx.send(()).unwrap();
        let first = first.join();
        let second = second.join();
        assert!(
            busy.is_ok(),
            "B did not observe the serialized birth gate: {busy:?}"
        );
        assert!(
            waiting && registry_absent,
            "B overtook the paused lease creator"
        );
        let first = first.unwrap();
        let second = second.unwrap();
        let metadata = fs::metadata(&lease_path).unwrap();
        assert_eq!(
            (metadata.dev(), metadata.ino()),
            (lease_identity.dev(), lease_identity.ino())
        );
        (first, second)
    });
    let registry: Value =
        serde_json::from_slice(&fs::read(directory.join("objects.json")).unwrap()).unwrap();
    let objects = registry["objects"].as_array().unwrap();
    assert_eq!(objects.len(), 2);
    let verify_originals = || {
        for object in objects {
            let root = directory.join(object["path"].as_str().unwrap());
            let binding: Value =
                serde_json::from_slice(&fs::read(root.join("binding.json")).unwrap()).unwrap();
            let publisher = binding["context"]["publisher"].as_u64().unwrap() as usize;
            let command = &commands[publisher];
            assert_eq!(fs::read(root.join("stdout.bin")).unwrap(), command.1.stdout);
            assert_eq!(fs::read(root.join("stderr.bin")).unwrap(), command.1.stderr);
            assert_eq!(binding["exit"], command.1.status.code().unwrap());
            assert_eq!(binding["expected_exit"], binding["exit"]);
            assert_eq!(binding["status"], command.1.status.to_string());
            assert_eq!(binding["joined"], true);
            assert!(object["sealed"].as_bool().unwrap());
            assert_eq!(object["consumers"]["command-assertion"]["released"], false);
            assert!(root.join("configuration/config.json").exists());
            assert!(root.join("original-state/cargo/tests/src/lib.rs").exists());
            if publisher == 0 {
                assert_eq!(
                    fs::read(root.join("original-state/recovery/original")).unwrap(),
                    b"required original recovery input"
                );
            }
        }
    };
    verify_originals();
    let (code, report) = owner.drain();
    assert_eq!(code, 0, "{report}");
    assert!(
        report["producer_objects"]
            .as_array()
            .unwrap()
            .iter()
            .all(|o| o["status"] == "protected")
    );
    for receipt in [first, second] {
        receipt
            .release("command-assertion", "original exit and streams consumed")
            .unwrap();
    }
    let (code, report) = owner.drain();
    assert_eq!(code, 0, "{report}");
    assert!(
        report["producer_objects"]
            .as_array()
            .unwrap()
            .iter()
            .all(|o| o["status"] == "cleaned")
    );
    for object in objects {
        for path in GENERATED {
            assert!(
                !directory
                    .join(object["path"].as_str().unwrap())
                    .join(path)
                    .exists()
            );
        }
    }
    verify_originals();
    let (code, report) = owner.drain();
    assert_eq!(code, 0, "{report}");
    assert!(
        report["producer_objects"]
            .as_array()
            .unwrap()
            .iter()
            .all(|o| o["status"] == "already-disposed")
    );
    verify_originals();
}

#[test]
fn producer_registry_birth_preserves_unknown_preexisting_lease_and_registry() {
    use std::os::unix::fs::MetadataExt;
    for registry_kind in ["absent", "malformed", "foreign"] {
        let owner = Owner::new();
        let directory = owner.root.join(FAMILY);
        fs::create_dir_all(&directory).unwrap();
        let lease = directory.join("registry.lease");
        fs::write(&lease, b"unknown original ownership").unwrap();
        let identity = fs::metadata(&lease).unwrap();
        let registry = match registry_kind {
            "absent" => None,
            "malformed" => Some(b"{original partial registry".to_vec()),
            "foreign" => {
                let store = fs::metadata(&directory).unwrap();
                Some(serde_json::to_vec(&json!({
                    "schema":"chrono-retained-producer/v1", "producer":"unknown-original-producer",
                    "directory_id":format!("{}:{}", store.dev(), store.ino()),
                    "lease_id":format!("{}:{}", identity.dev(), identity.ino()), "objects":[]
                })).unwrap())
            }
            _ => unreachable!(),
        };
        if let Some(bytes) = &registry {
            fs::write(directory.join("objects.json"), bytes).unwrap();
        }
        let (execution, output, argv) = cargo_result(true);
        let rejected = std::panic::catch_unwind(|| {
            retain_command_result_in(
                &owner.root,
                execution.path(),
                &argv,
                &json!({"unknown_store":registry_kind}),
                &output,
                Some(101),
                Some(&directory),
            )
        });
        assert!(rejected.is_err(), "adopted {registry_kind} registry");
        let copy = fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| path.is_dir())
            .unwrap();
        let binding = fs::read(copy.join("binding.json")).unwrap();
        let decoded: Value = serde_json::from_slice(&binding).unwrap();
        assert_eq!(decoded["exit"], 101);
        assert_eq!(decoded["status"], output.status.to_string());
        assert_eq!(fs::read(copy.join("stdout.bin")).unwrap(), output.stdout);
        assert_eq!(fs::read(copy.join("stderr.bin")).unwrap(), output.stderr);
        assert_eq!(
            fs::read(copy.join("original-state/recovery/original")).unwrap(),
            b"required original recovery input"
        );
        let (code, report) = owner.drain();
        if registry_kind == "absent" {
            assert_eq!(code, 0, "{}", report["producer_drains"]);
            assert_eq!(report["producer_objects"][0]["status"], "protected-unknown");
        } else {
            assert_ne!(code, 0, "{}", report["producer_drains"]);
            assert!(!report["cleanup_failures"].as_array().unwrap().is_empty());
        }
        assert_eq!(fs::read(&lease).unwrap(), b"unknown original ownership");
        let after = fs::metadata(&lease).unwrap();
        assert_eq!((after.dev(), after.ino()), (identity.dev(), identity.ino()));
        assert_eq!(fs::read(directory.join("objects.json")).ok(), registry);
        assert_eq!(fs::read(copy.join("binding.json")).unwrap(), binding);
        assert_eq!(fs::read(copy.join("stdout.bin")).unwrap(), output.stdout);
        assert_eq!(fs::read(copy.join("stderr.bin")).unwrap(), output.stderr);
        assert_eq!(
            fs::read(copy.join("original-state/recovery/original")).unwrap(),
            b"required original recovery input"
        );
        for path in GENERATED {
            assert!(copy.join(path).exists());
        }
        assert_eq!(
            fs::read_dir(&directory).unwrap().count(),
            if registry_kind == "absent" { 2 } else { 3 }
        );
    }
}

#[test]
fn producer_does_not_dispose_copied_outputs_without_exact_source_declarations() {
    let owner = Owner::new();
    let (execution, output, argv) = cargo_result(false);
    fs::write(
        execution.path().join(".chrono-harness/config.json"),
        b"{\"artifacts\":[]}",
    )
    .unwrap();
    let receipt = retain_command_result_in(
        &owner.root,
        execution.path(),
        &argv,
        &json!({}),
        &output,
        Some(0),
        Some(&owner.root.join(FAMILY)),
    );
    receipt
        .release("command-assertion", "original output consumed")
        .unwrap();
    let (code, report) = owner.drain();
    assert_eq!(code, 0, "{report}");
    assert_eq!(
        report["producer_objects"][0]["registered_generated_objects"],
        0
    );
    for path in GENERATED {
        assert!(owner.object().join(path).exists());
    }
}
