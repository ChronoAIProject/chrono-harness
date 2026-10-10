use super::*;

fn host() -> Host {
    let h = super::bounded_cleanup::main_host_mode("temporary-producer");
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
    let (code, protected, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{protected} {error}");
    assert!(allocated.join("cache 空白/output").exists());
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
            git(&root, &["ls-files", "cache 空白/source.rs"]),
            "cache 空白/source.rs"
        );
    }
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
