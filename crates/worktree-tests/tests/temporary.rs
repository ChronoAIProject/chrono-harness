use super::*;

struct ReleaseNative {
    allocation: PathBuf,
    coordinator: PathBuf,
}
impl Drop for ReleaseNative {
    fn drop(&mut self) {
        if !self.allocation.is_absolute() {
            return;
        }
        // Teardown uses the actual fixture protocol, including assertion failure.
        // The child's final read and the owner's kernel exclusion establish release.
        let _ = fs::write(self.allocation.join("native-release"), "fixture teardown");
        let _ = fs::write(
            self.coordinator
                .join(".chrono-harness/state/temporary-parent-release"),
            "fixture teardown",
        );
    }
}

fn host() -> Host {
    host_mode("temporary-producer")
}

fn host_mode(mode: &str) -> Host {
    let h = super::bounded_cleanup::main_host_mode(mode);
    let path = h.root.join(".chrono-harness/cleanup.json");
    let mut policy = json(&fs::read(&path).unwrap()).unwrap();
    policy["retained_producers"] = value!([{"id":"test-allocation",
        "directory":".chrono-harness/state/custom temporary λ/", "temporary":true,
        "generated_outputs":["cache 空白/"]}]);
    fs::write(path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
    h
}

fn wait(path: &Path) {
    let began = std::time::Instant::now();
    while !path.exists() && began.elapsed().as_secs() < 10 {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        path.exists(),
        "missing actual native observation {}",
        path.display()
    );
}

struct ReleaseCheck(PathBuf);
impl Drop for ReleaseCheck {
    fn drop(&mut self) {
        let _ = fs::write(
            self.0.join("temporary-check-release"),
            "actual fixture release",
        );
    }
}

#[test]
fn excluded_allocation_source_check_allows_unrelated_publication_and_rejects_diagnostic_race() {
    let h = host();
    let policy_path = h.root.join(".chrono-harness/cleanup.json");
    let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
    policy["retained_producers"][0]["source_roots"] = value!(["."]);
    fs::write(policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "blocked λ ")
            .unwrap();
    let root = allocation.path().to_owned();
    git(&root, &["init", "-q"]);
    fs::write(root.join("source.rs"), "retained source").unwrap();
    commit(&root);
    fs::create_dir(root.join("cache 空白")).unwrap();
    fs::write(root.join("cache 空白/output"), "generated").unwrap();
    super::automatic::native_git(&h, "temporary-source-check", value!({"root":root}));
    drop(allocation);
    let mut maintenance = super::interrupted_cleanup::CapturedChild::spawn(
        &mut h.auto_command("maintain", &[]),
        &h.root,
    );
    let _release = ReleaseCheck(h.parent.clone());
    maintenance.await_file(&h.parent.join("temporary-check-active"));
    let began = std::time::Instant::now();
    let unrelated = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "unrelated λ ",
    )
    .unwrap();
    assert!(
        began.elapsed().as_secs() < 10,
        "unrelated publication waited for the source check"
    );
    let reference =
        chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", &root).unwrap();
    assert!(
        reference
            .acquire("racing-diagnostic", "must not enter an excluded cache")
            .unwrap_err()
            .contains("reference not acquired")
    );
    assert!(root.join("cache 空白/output").exists());
    fs::write(h.parent.join("temporary-check-release"), "actual release").unwrap();
    let output = maintenance.wait_with_output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(!root.join("cache 空白").exists());
    assert_eq!(
        fs::read(root.join("source.rs")).unwrap(),
        b"retained source"
    );
    let registry = json(&fs::read(store.join("objects.json")).unwrap()).unwrap();
    let row = registry["objects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["path"] == root.file_name().unwrap().to_str().unwrap())
        .unwrap();
    assert!(row["consumers"].get("racing-diagnostic").is_none());
    assert!(unrelated.path().is_dir());
}

fn capture_directory(root: &Path) -> PathBuf {
    let captures: Vec<_> = fs::read_dir(root.join(".chrono-harness/state/command-captures"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(captures.len(), 1);
    captures[0].clone()
}

fn command_carrier_count(command: &Command) -> usize {
    command.get_envs().find_map(|(key, value)| {
        (key == chrono_harness::process_fds::ENV).then(||
            value.map(|value| value.to_str().unwrap().split(',').count()).unwrap_or(0))
    }).unwrap_or(0)
}

#[test]
fn allocation_native_binding_forwards_existing_scope_once() {
    let h = host();
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation = chrono_worktree::TemporaryHost::allocate(
        &store, "test-allocation", &outputs, "binding cost λ "
    ).unwrap();
    let baseline = native_command(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    let expected = command_carrier_count(&baseline);
    assert!(expected > 0);
    let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    command.arg("noop").current_dir(allocation.path()).env_clear();
    allocation.bind_command(&mut command).unwrap();
    let bound = command_carrier_count(&command);
    assert_eq!(bound, expected, "allocation lease is already in the held scope");
    assert!(command.output().unwrap().status.success());
    let mut captured = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    captured.arg("capture-once").current_dir(allocation.path()).env_clear();
    let output = allocation.capture_output(&mut captured).unwrap();
    assert_eq!(output.status.code(), Some(7));
    assert_eq!(command_carrier_count(&captured), expected);
    println!("native capability cost: baseline={expected}, bound={bound}, captured={}",
        command_carrier_count(&captured));
}

#[test]
fn native_capture_preserves_actual_nonzero_exit_and_raw_bytes() {
    let h = host();
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "capture λ ")
            .unwrap();
    let mut command = native_command(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    command
        .arg("capture-once")
        .current_dir(allocation.path())
        .env_clear();
    let output = allocation.capture_output(&mut command).unwrap();
    assert_eq!(output.status.code(), Some(7));
    assert_eq!(output.stdout, b"\xffpartial stdout\0\n");
    assert_eq!(output.stderr, b"\xfepartial stderr\0\n");
    let capture = capture_directory(allocation.path());
    assert_eq!(
        fs::read(capture.join("stdout.bytes")).unwrap(),
        output.stdout
    );
    assert_eq!(
        json(&fs::read(capture.join("result.json")).unwrap()).unwrap()["exit_code"],
        7
    );
}

#[test]
fn killed_capture_producer_keeps_partial_streams_without_joined_result_or_finish() {
    let h = host_mode("temporary-capture");
    let mut producer = super::interrupted_cleanup::CapturedChild::spawn(
        &mut h.auto_command("use", &["--operation", "use.consumer"]),
        &h.root,
    );
    producer.await_managed_file(&h.root.join(".chrono-harness/state/temporary-path"));
    let root = PathBuf::from(
        fs::read_to_string(h.root.join(".chrono-harness/state/temporary-path")).unwrap(),
    );
    let _release = ReleaseNative {
        allocation: root.clone(),
        coordinator: h.root.clone(),
    };
    wait(&root.join("capture-active"));
    let capture = capture_directory(&root);
    wait(&capture.join("started.json"));
    let parent = fs::read_to_string(h.root.join(".chrono-harness/state/temporary-parent")).unwrap();
    assert!(
        native_command("/bin/kill")
            .args(["-KILL", &parent])
            .status()
            .unwrap()
            .success()
    );
    producer.kill().unwrap();
    assert!(!producer.wait_with_output().unwrap().status.success());
    assert_eq!(
        fs::read(capture.join("stdout.bytes")).unwrap(),
        b"\xffpartial stdout\0\n"
    );
    assert_eq!(
        fs::read(capture.join("stderr.bytes")).unwrap(),
        b"\xfepartial stderr\0\n"
    );
    assert!(!capture.join("result.json").exists());
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(root.join("cache 空白/output").exists());
    fs::write(root.join("native-release"), "actual final native write").unwrap();
    wait(&root.join("capture-done"));
    let began = std::time::Instant::now();
    while root.join("cache 空白").exists() {
        assert_eq!(h.auto("maintain", &[]).0, 0);
        assert!(began.elapsed().as_secs() < 10);
    }
    assert!(capture.join("intent.json").is_file());
    assert!(!capture.join("result.json").exists());
    assert!(h.ledger()["entries"][0]["terminal"].is_null());
}

#[test]
fn prospective_allocation_recovers_after_kill_without_seal_or_finish_and_keeps_native_holder() {
    for signal in ["-TERM", "-KILL"] {
        interrupted(signal);
    }
}

fn interrupted(signal: &str) {
    let h = host();
    let mut producer = super::interrupted_cleanup::CapturedChild::spawn(
        &mut h.auto_command("use", &["--operation", "use.consumer"]),
        &h.root,
    );
    producer.await_managed_file(&h.root.join(".chrono-harness/state/temporary-path"));
    let allocated = PathBuf::from(
        fs::read_to_string(h.root.join(".chrono-harness/state/temporary-path")).unwrap(),
    );
    let _release = ReleaseNative {
        allocation: allocated.clone(),
        coordinator: h.root.clone(),
    };
    wait(&allocated.join("native-active"));
    let parent = fs::read_to_string(h.root.join(".chrono-harness/state/temporary-parent")).unwrap();
    let child = fs::read_to_string(h.root.join(".chrono-harness/state/temporary-child")).unwrap();
    assert_ne!(parent, child);
    let unknown = allocated.parent().unwrap().join("unclaimed neighbor λ");
    fs::create_dir(&unknown).unwrap();
    fs::write(unknown.join("source"), "unowned").unwrap();
    assert!(
        native_command("/bin/kill")
            .args([signal, &parent])
            .status()
            .unwrap()
            .success()
    );
    producer.kill().unwrap();
    assert!(!producer.wait_with_output().unwrap().status.success());
    assert!(
        native_command("/bin/kill")
            .args(["-0", &child])
            .status()
            .unwrap()
            .success(),
        "the independently surviving consumer must actually be alive"
    );
    let (code, protected, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{protected} {error}");
    assert!(allocated.join("cache 空白/output").exists());
    assert!(
        native_command("/bin/kill")
            .args(["-0", &child])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        protected["producer_objects"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["status"] == "protected")
    );
    fs::write(
        allocated.join("native-release"),
        "actual final read authorized",
    )
    .unwrap();
    wait(&allocated.join("native-done"));
    // The marker is a native read observation; EX, not the marker, decides release.
    let mut reclaimed = false;
    for _ in 0..100 {
        let (code, report, error) = h.auto("maintain", &[]);
        assert_eq!(code, 0, "{report} {error}");
        if !allocated.join("cache 空白").exists() {
            reclaimed = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(reclaimed);
    assert_eq!(
        fs::read(allocated.join("source.txt")).unwrap(),
        b"original source\n"
    );
    assert_eq!(
        fs::read(allocated.join("partial.bin")).unwrap(),
        [255, 0, 10]
    );
    assert_eq!(fs::read(unknown.join("source")).unwrap(), b"unowned");
    let registry =
        json(&fs::read(allocated.parent().unwrap().join("objects.json")).unwrap()).unwrap();
    assert_eq!(registry["objects"][0]["sealed"], false);
    assert!(registry["objects"][0]["publication"].is_null());
    assert_eq!(registry["objects"][0]["consumers"], value!({}));
    assert_eq!(registry["objects"][0]["disposed"], true);
    assert_eq!(h.ledger()["entries"][0]["status"], "active");
    assert!(h.ledger()["entries"][0]["terminal"].is_null());
}

#[test]
fn normal_producer_exit_without_finish_recovers_only_declared_output() {
    let h = host();
    let mut producer = super::interrupted_cleanup::CapturedChild::spawn(
        &mut h.auto_command("use", &["--operation", "use.consumer"]),
        &h.root,
    );
    producer.await_managed_file(&h.root.join(".chrono-harness/state/temporary-path"));
    let root = PathBuf::from(
        fs::read_to_string(h.root.join(".chrono-harness/state/temporary-path")).unwrap(),
    );
    let _release = ReleaseNative {
        allocation: root.clone(),
        coordinator: h.root.clone(),
    };
    wait(&root.join("native-active"));
    fs::write(root.join("native-release"), "actual release").unwrap();
    fs::write(
        h.root
            .join(".chrono-harness/state/temporary-parent-release"),
        "actual join",
    )
    .unwrap();
    assert!(producer.wait_with_output().unwrap().status.success());
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(!root.join("cache 空白").exists());
    assert_eq!(
        fs::read(root.join("source.txt")).unwrap(),
        b"original source\n"
    );
    assert_eq!(fs::read(root.join("partial.bin")).unwrap(), [255, 0, 10]);
    assert!(h.ledger()["entries"][0]["terminal"].is_null());
}

#[test]
fn actual_cargo_native_descendant_uses_absolute_allocation_from_varying_working_directories() {
    for vary in [false, true] {
        let h = super::bounded_cleanup::main_host_mode("temporary-cargo");
        let policy_path = h.root.join(".chrono-harness/cleanup.json");
        let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
        policy["retained_producers"] = value!([{"id":"test-allocation", "directory":".chrono-harness/state/custom temporary λ/", "temporary":true,
            "generated_outputs":["cache 空白/", "cargo 空白/target/"]}]);
        fs::write(policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
        commit(&h.root);
        if vary {
            fs::create_dir_all(h.root.join(".chrono-harness/state")).unwrap();
            fs::write(
                h.root.join(".chrono-harness/state/vary-cargo-cwd"),
                "declared invocation",
            )
            .unwrap();
        }
        let mut producer = super::interrupted_cleanup::CapturedChild::spawn(
            &mut h.auto_command("use", &["--operation", "use.consumer"]),
            &h.root,
        );
        producer.await_managed_file(&h.root.join(".chrono-harness/state/temporary-path"));
        let root = PathBuf::from(
            fs::read_to_string(h.root.join(".chrono-harness/state/temporary-path")).unwrap(),
        );
        let _release = ReleaseNative {
            allocation: root.clone(),
            coordinator: h.root.clone(),
        };
        producer.await_managed_file(&root.join("native-active"));
        assert_eq!(h.auto("maintain", &[]).0, 0);
        assert!(root.join("cargo 空白/target").exists());
        fs::write(root.join("native-release"), "actual join").unwrap();
        assert!(producer.wait_with_output().unwrap().status.success());
        assert!(root.join("cargo-native-joined").exists());
        assert_eq!(h.auto("maintain", &[]).0, 0);
        assert!(!root.join("cargo 空白/target").exists());
        assert!(root.join("cargo 空白/src/lib.rs").exists());
    }
}

#[test]
fn declared_nested_git_root_preserves_committed_and_staged_source_under_outputs() {
    for staged in [false, true] {
        let h = host();
        let policy_path = h.root.join(".chrono-harness/cleanup.json");
        let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
        policy["retained_producers"][0]["source_roots"] = value!(["."]);
        fs::write(policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
        commit(&h.root);
        let (store, outputs) =
            chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
                .unwrap();
        let allocation = chrono_worktree::TemporaryHost::allocate(
            &store,
            "test-allocation",
            &outputs,
            "source λ ",
        )
        .unwrap();
        let root = allocation.path().to_owned();
        git(&root, &["init", "-q", "-b", "dev"]);
        git(&root, &["config", "user.email", "fixture@example.invalid"]);
        git(&root, &["config", "user.name", "Fixture"]);
        fs::write(root.join("source"), "original").unwrap();
        commit(&root);
        fs::create_dir(root.join("cache 空白")).unwrap();
        fs::write(root.join("cache 空白/source.rs"), "irreplaceable source").unwrap();
        git(&root, &["add", "cache 空白/source.rs"]);
        if !staged {
            git(&root, &["commit", "-qm", "unretained source"]);
        }
        drop(allocation);
        let (code, report, error) = h.auto("maintain", &[]);
        assert_ne!(code, 0, "{report} {error}");
        assert!(
            report.to_string().contains("contains tracked paths"),
            "{report}"
        );
        assert_eq!(
            fs::read(root.join("cache 空白/source.rs")).unwrap(),
            b"irreplaceable source"
        );
        assert_eq!(
            git(&root, &["ls-files", "-z", "cache 空白/source.rs"]),
            "cache 空白/source.rs\0"
        );
    }
}

#[test]
fn real_coordinator_adoption_protects_native_use_then_recovers_registered_cargo_output() {
    let h = super::bounded_cleanup::cargo_host();
    let allocation = h.parent.clone();
    let source = h.root.join("independent/producer/src/lib.rs");
    let original = fs::read(&source).unwrap();
    let mut cargo = native_command("cargo");
    cargo
        .current_dir(&h.root)
        .env_remove("CARGO_TARGET_DIR")
        .args([
            "build",
            "--offline",
            "--locked",
            "--manifest-path",
            "independent/producer/Cargo.toml",
        ]);
    h._dir.bind_command(&mut cargo).unwrap();
    let output = cargo.output().unwrap();
    assert!(output.status.success(), "{output:?}");
    drop(cargo);
    let target = h.root.join("independent/producer/target");
    let selected =
        "source with spaces/independent/producer/target/debug/libbounded_cache_producer.rlib";
    assert!(allocation.join(selected).is_file());
    let unknown = allocation.parent().unwrap().join(format!(
        "unclaimed-neighbor-{}",
        allocation.file_name().unwrap().to_str().unwrap()
    ));
    fs::create_dir(&unknown).unwrap();
    fs::write(unknown.join("original"), [255, 0, 10]).unwrap();
    let mut command = native_command(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    command
        .args(["temporary-descendant"])
        .arg(&allocation)
        .arg(selected)
        .env_clear();
    h._dir.bind_command(&mut command).unwrap();
    let mut child = command.spawn().unwrap();
    drop(command);
    let _release = ReleaseNative {
        allocation: allocation.clone(),
        coordinator: h.root.clone(),
    };
    wait(&allocation.join("native-active"));
    // This allocation is the real source coordinator's declared worktree-test-host,
    // not the fixture coordinator's test-allocation producer.
    drop(h);
    let maintain = || {
        let out = native_command(crate::source().join(".chrono-harness/bin/chrono-worktree"))
            .current_dir(crate::source())
            .arg("maintain")
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
        json(&out.stdout).unwrap()
    };
    let protected = maintain();
    assert!(target.exists());
    assert!(
        protected["producer_objects"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["producer"] == "worktree-test-host"
                && r["status"] == "protected"
                && r["path"]
                    .as_str()
                    .unwrap()
                    .ends_with(allocation.file_name().unwrap().to_str().unwrap()))
    );
    fs::write(
        allocation.join("native-release"),
        "actual final consumer read",
    )
    .unwrap();
    assert!(child.wait().unwrap().success());
    let reclaimed = maintain();
    assert!(!target.exists());
    assert_eq!(fs::read(source).unwrap(), original);
    assert_eq!(fs::read(unknown.join("original")).unwrap(), [255, 0, 10]);
    let evidence = value!({"candidate":git(&crate::source(), &["rev-parse","HEAD"]),
        "binary_sha256":sha256(&fs::read(crate::source().join(".chrono-harness/bin/chrono-worktree")).unwrap()),
        "policy_sha256":sha256(&fs::read(crate::source().join(".chrono-harness/cleanup.json")).unwrap()),
        "allocation":allocation,"protected_report":protected["report_path"],"recovery_report":reclaimed["report_path"],
        "source_preserved":true,"unknown_neighbor_preserved":true,"finish":"not-called"});
    let evidence_path = allocation.join("real-coordinator-custody.json");
    fs::write(
        &evidence_path,
        serde_json::to_vec_pretty(&evidence).unwrap(),
    )
    .unwrap();
    println!("REAL_COORDINATOR_CUSTODY {}", evidence_path.display());
}

#[test]
fn overlapping_allocation_commands_and_real_diagnostic_reference_preserve_generated_outputs() {
    let h = host();
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "overlap λ ")
            .unwrap();
    let root = allocation.path().to_owned();
    let _release = ReleaseNative {
        allocation: root.clone(),
        coordinator: h.root.clone(),
    };
    fs::create_dir_all(root.join("cache 空白")).unwrap();
    fs::write(root.join("cache 空白/output"), "rebuildable").unwrap();
    let reference =
        chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", &root).unwrap();
    reference
        .acquire(
            "current-diagnostic",
            "this test still reads the actual output",
        )
        .unwrap();
    let mut commands = vec![];
    for _ in 0..2 {
        let mut c = native_command(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
        c.arg("temporary-descendant").arg(&root).env_clear();
        allocation.bind_command(&mut c).unwrap();
        commands.push(c.spawn().unwrap());
    }
    wait(&root.join("native-active"));
    drop(allocation);
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(root.join("cache 空白/output").exists());
    fs::write(root.join("native-release"), "actual release").unwrap();
    for mut c in commands {
        assert!(c.wait().unwrap().success());
    }
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert_eq!(
        fs::read(root.join("cache 空白/output")).unwrap(),
        b"rebuildable"
    );
    // This diagnostic consumer remains unreleased; no release is fabricated.
}
