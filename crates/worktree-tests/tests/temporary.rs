use super::*;

fn host() -> Host {
    host_mode("noop")
}

fn host_mode(mode: &str) -> Host {
    let h = super::bounded_cleanup::main_host_mode(mode);
    let path = h.root.join(".chrono-harness/cleanup.json");
    let mut policy = json(&fs::read(&path).unwrap()).unwrap();
    policy["retained_producers"] = value!([{"id":"test-allocation",
        "directory":".chrono-harness/state/custom temporary λ/", "temporary":true,
        "fixture_bodies":true,
        "generated_outputs":["cache 空白/"]}]);
    fs::write(path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
    h
}

fn capture_directory(root: &Path) -> PathBuf {
    let captures: Vec<_> = fs::read_dir(root.join(".chrono-harness/state/command-captures"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(captures.len(), 1);
    captures[0].clone()
}

fn command_carrier_count(command: &Command) -> usize {
    command
        .get_envs()
        .find_map(|(key, value)| {
            (key == chrono_harness::process_fds::ENV).then(|| {
                value
                    .map(|value| value.to_str().unwrap().split(',').count())
                    .unwrap_or(0)
            })
        })
        .unwrap_or(0)
}

#[test]
fn absent_temporary_outputs_do_not_rewrite_producer_state_or_publish_disposal() {
    let h = host();
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation =
        chrono_worktree::TemporaryHost::allocate(&store, "test-allocation", &outputs, "absent λ ")
            .unwrap();
    let name = allocation
        .path()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    drop(allocation);
    let before = fs::read(store.join("objects.json")).unwrap();
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    let row = report["producer_objects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"].as_str().unwrap().ends_with(&name))
        .unwrap();
    assert_eq!(row["status"], "already-absent");
    assert_eq!(row["absent_outputs"], value!(outputs));
    assert_eq!(
        fs::read(store.join("objects.json")).unwrap(),
        before,
        "no disposal result or producer success may be manufactured for absent output"
    );
    assert!(store.join(&name).is_dir());
}

#[test]
fn unavailable_allocation_preserves_originals_and_does_not_block_eligible_sibling() {
    let h = host();
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let missing = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "unavailable λ ",
    )
    .unwrap();
    let original = missing.path().to_owned();
    let name = original.file_name().unwrap().to_str().unwrap().to_owned();
    fs::write(original.join("source.rs"), "original source").unwrap();
    drop(missing);
    let retained = store.join("unknown-retained-original");
    fs::rename(&original, &retained).unwrap();
    let sibling = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "eligible λ ",
    )
    .unwrap();
    let target = sibling.path().join("cache 空白");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("generated"), "rebuildable").unwrap();
    drop(sibling);
    let read = || json(&fs::read(store.join("objects.json")).unwrap()).unwrap();
    let find = |registry: &Value| {
        registry["objects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["path"] == name)
            .unwrap()
            .clone()
    };
    let before = find(&read());
    let (code, report, _) = h.auto("maintain", &[]);
    assert_eq!(code, 2);
    assert!(
        !target.exists(),
        "eligible sibling must be reclaimed despite the missing row"
    );
    assert_eq!(find(&read()), before);
    assert_eq!(
        fs::read(retained.join("source.rs")).unwrap(),
        b"original source"
    );
    let row = report["producer_objects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"].as_str().unwrap().ends_with(&name))
        .unwrap();
    assert_eq!(row["status"], "failed");
    assert!(row["error"].as_str().unwrap().contains("No such file"));
}

#[test]
fn allocation_native_binding_forwards_existing_scope_once() {
    let h = host();
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "binding cost λ ",
    )
    .unwrap();
    let mut baseline = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    chrono_harness::process_fds::forward_command(&mut baseline, &[]).unwrap();
    let expected = command_carrier_count(&baseline);
    assert!(expected > 0);
    let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    command
        .arg("noop")
        .current_dir(allocation.path())
        .env_clear();
    allocation.bind_command(&mut command).unwrap();
    let bound = command_carrier_count(&command);
    assert_eq!(
        bound, expected,
        "allocation lease is already in the held scope"
    );
    assert!(command.output().unwrap().status.success());
    let mut captured = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    captured
        .arg("capture-once")
        .current_dir(allocation.path())
        .env_clear();
    let output = allocation.capture_output(&mut captured).unwrap();
    assert_eq!(output.status.code(), Some(7));
    assert_eq!(command_carrier_count(&captured), expected);
    println!(
        "native capability cost: baseline={expected}, bound={bound}, captured={}",
        command_carrier_count(&captured)
    );
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
    let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
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
fn native_protocol_capture_preserves_exact_input_bytes_and_eof() {
    let h = host();
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "protocol λ ",
    )
    .unwrap();
    let input = b"\xffrequest\0\n";
    let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    command.arg("capture-input").env_clear();
    allocation.bind_command(&mut command).unwrap();
    let output =
        chrono_worktree::TemporaryHost::capture_bound_input(allocation.path(), &mut command, input)
            .unwrap();
    assert_eq!(output.status.code(), Some(9));
    assert_eq!(output.stdout, input);
    assert_eq!(output.stderr, b"\xfeinput\0\n");
    let capture = capture_directory(allocation.path());
    assert_eq!(fs::read(capture.join("stdin.bytes")).unwrap(), input);
    let intent = json(&fs::read(capture.join("intent.json")).unwrap()).unwrap();
    assert_eq!(intent["stdin"]["sha256"], sha256(input));
    assert_eq!(intent["stdin"]["length"], input.len());
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
fn excluded_source_check_allows_unrelated_publication_and_refuses_late_diagnostic() {
    use std::io::{Read, Write};
    let h = host();
    let path = h.root.join(".chrono-harness/cleanup.json");
    let mut policy = json(&fs::read(&path).unwrap()).unwrap();
    policy["retained_producers"][0]["source_roots"] = value!(["."]);
    fs::write(path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "excluded λ ",
    )
    .unwrap();
    let root = allocation.path().to_owned();
    git(&root, &["init", "-q"]);
    fs::write(root.join("source.rs"), "retained source").unwrap();
    commit(&root);
    fs::create_dir(root.join("cache 空白")).unwrap();
    fs::write(root.join("cache 空白/output"), "generated").unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    super::automatic::native_git(
        &h,
        "temporary-source-check",
        value!({
            "root":root, "synchronize":listener.local_addr().unwrap().to_string(),
            "synchronization_claim":h.parent.join("temporary-source-check-once"),
        }),
    );
    drop(allocation);
    let mut command = h.auto_command("maintain", &[]);
    let maintenance = std::thread::spawn(move || command.output().unwrap());
    let (mut stream, _) = listener.accept().unwrap();
    stream.read_exact(&mut [0; 5]).unwrap();
    let unrelated = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "unrelated λ ",
    )
    .unwrap();
    let reference =
        chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", &root).unwrap();
    assert!(
        reference
            .acquire("late-diagnostic", "must not read excluded output")
            .unwrap_err()
            .contains("reference not acquired")
    );
    stream.write_all(b"r").unwrap();
    let output = maintenance.join().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(!root.join("cache 空白").exists());
    assert_eq!(
        fs::read(root.join("source.rs")).unwrap(),
        b"retained source"
    );
    let registry = json(&fs::read(store.join("objects.json")).unwrap()).unwrap();
    assert!(
        registry["objects"][0]["consumers"]
            .get("late-diagnostic")
            .is_none()
    );
    assert!(unrelated.path().is_dir());
}

#[test]
fn actual_cargo_native_descendant_uses_absolute_allocation_from_varying_working_directories() {
    use std::io::{Read, Write};
    for vary in [false, true] {
        let h = host();
        let policy_path = h.root.join(".chrono-harness/cleanup.json");
        let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
        let outputs = vec!["cache 空白/".to_string(), "cargo 空白/target/".to_string()];
        policy["retained_producers"][0]["generated_outputs"] = value!(outputs);
        fs::write(policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
        commit(&h.root);
        let (store, _) =
            chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
                .unwrap();
        let allocation = chrono_worktree::TemporaryHost::allocate(
            &store,
            "test-allocation",
            &outputs,
            "Cargo λ ",
        )
        .unwrap();
        let root = allocation.path().to_owned();
        let project = root.join("cargo 空白");
        fs::create_dir_all(project.join("src")).unwrap();
        fs::write(
            project.join("Cargo.toml"),
            include_bytes!("support/native_cargo/Cargo.toml"),
        )
        .unwrap();
        fs::write(
            project.join("src/lib.rs"),
            include_bytes!("support/native_cargo/src/lib.rs"),
        )
        .unwrap();
        fs::create_dir(root.join("cache 空白")).unwrap();
        fs::write(root.join("cache 空白/output"), "rebuildable").unwrap();
        let cwd = if vary {
            root.join("different cwd λ")
        } else {
            root.clone()
        };
        fs::create_dir_all(&cwd).unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let mut command = Command::new("cargo");
        command
            .current_dir(&cwd)
            .env_remove("CARGO_TARGET_DIR")
            .env("CHRONO_TEMPORARY_CARGO_ROOT", &root)
            .env(
                "CHRONO_TEMPORARY_HELPER",
                env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"),
            )
            .env(
                "CHRONO_TEMPORARY_SYNC",
                listener.local_addr().unwrap().to_string(),
            )
            .args(["test", "--lib", "--offline", "--manifest-path"])
            .arg(project.join("Cargo.toml"));
        allocation.bind_command(&mut command).unwrap();
        let capture_root = root.clone();
        let cargo = std::thread::spawn(move || {
            chrono_worktree::TemporaryHost::capture_bound_output(&capture_root, &mut command)
                .unwrap()
        });
        let (mut stream, _) = listener.accept().unwrap();
        stream.read_exact(&mut [0; 5]).unwrap();
        let diagnostic =
            chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", &root).unwrap();
        diagnostic
            .acquire(
                "actual-cargo-assertion",
                "reads the completed native Cargo output",
            )
            .unwrap();
        drop(allocation);
        assert_eq!(h.auto("maintain", &[]).0, 0);
        assert!(project.join("target").exists());
        stream.write_all(b"r").unwrap();
        let output = cargo.join().unwrap();
        assert!(output.status.success(), "{output:?}");
        assert!(root.join("cargo-native-joined").exists());
        assert_eq!(h.auto("maintain", &[]).0, 0);
        assert!(project.join("target").exists());
        diagnostic
            .release(
                "actual-cargo-assertion",
                "native child and Cargo joined; assertions consumed originals",
            )
            .unwrap();
        assert_eq!(h.auto("maintain", &[]).0, 0);
        assert!(!project.join("target").exists());
        assert!(!root.join("cache 空白").exists());
        assert!(project.join("src/lib.rs").is_file());
        assert!(project.join("Cargo.toml").is_file());
        assert!(project.join("Cargo.lock").is_file());
    }
}

#[test]
fn capture_interruption_helper() {
    let Some(root) = std::env::var_os("CHRONO_CAPTURE_ROOT") else {
        return;
    };
    let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    command
        .arg("capture-once")
        .env("CHRONO_CAPTURE_INTERRUPT", "1");
    chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
    let _ = chrono_worktree::TemporaryHost::capture_bound_output(Path::new(&root), &mut command);
    panic!("the actual native child must interrupt this capture producer");
}

#[test]
fn interrupted_native_capture_keeps_partial_raw_streams_without_a_join_or_disposal_claim() {
    let h = host();
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "interrupted λ ",
    )
    .unwrap();
    let root = allocation.path().to_owned();
    fs::create_dir(root.join("cache 空白")).unwrap();
    fs::write(root.join("cache 空白/output"), "rebuildable").unwrap();
    let diagnostic =
        chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", &root).unwrap();
    diagnostic
        .acquire(
            "interruption-diagnostic",
            "reads original partial streams; native join remains unknown",
        )
        .unwrap();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "temporary::capture_interruption_helper",
            "--nocapture",
        ])
        .env("CHRONO_CAPTURE_ROOT", &root);
    allocation.bind_command(&mut command).unwrap();
    let output = command.output().unwrap();
    assert!(!output.status.success());
    drop(command);
    drop(allocation);
    let capture = capture_directory(&root);
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
    let registry = json(&fs::read(store.join("objects.json")).unwrap()).unwrap();
    assert!(
        !registry["objects"][0]["consumers"]["interruption-diagnostic"]["released"]
            .as_bool()
            .unwrap()
    );
    assert!(h.ledger()["entries"][0]["terminal"].is_null());
}

#[test]
fn native_spawn_failure_keeps_intent_and_empty_original_streams_without_result() {
    let h = host();
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "spawn failure λ ",
    )
    .unwrap();
    let mut command = Command::new(allocation.path().join("missing-native-program"));
    assert!(allocation.capture_output(&mut command).is_err());
    let capture = capture_directory(allocation.path());
    assert!(capture.join("intent.json").is_file());
    assert!(fs::read(capture.join("stdout.bytes")).unwrap().is_empty());
    assert!(fs::read(capture.join("stderr.bytes")).unwrap().is_empty());
    assert!(!capture.join("started.json").exists());
    assert!(!capture.join("result.json").exists());
}

#[test]
fn overlapping_native_allocations_preserve_output_after_kill_until_join_and_diagnostic_release() {
    use std::io::{Read, Write};
    unsafe extern "C" {
        fn kill(pid: i32, signal: i32) -> i32;
    }
    for signal in [15, 9] {
        let h = host();
        let (store, outputs) =
            chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
                .unwrap();
        let allocation = chrono_worktree::TemporaryHost::allocate(
            &store,
            "test-allocation",
            &outputs,
            "overlapping native λ ",
        )
        .unwrap();
        let root = allocation.path().to_owned();
        fs::create_dir(root.join("cache 空白")).unwrap();
        fs::write(root.join("cache 空白/output"), "rebuildable").unwrap();
        fs::write(root.join("source.rs"), "unique source").unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let mut children = vec![];
        let mut streams = vec![];
        for _ in 0..2 {
            let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
            command
                .arg("custody-holder")
                .env_clear()
                .env("CHRONO_TEMPORARY_CARGO_ROOT", &root)
                .env(
                    "CHRONO_TEMPORARY_SYNC",
                    listener.local_addr().unwrap().to_string(),
                );
            allocation.bind_command(&mut command).unwrap();
            children.push(command.spawn().unwrap());
            let (mut stream, _) = listener.accept().unwrap();
            stream.read_exact(&mut [0; 5]).unwrap();
            streams.push(stream);
        }
        let diagnostic =
            chrono_worktree::RetainedArtifact::resume(&store, "test-allocation", &root).unwrap();
        diagnostic
            .acquire(
                "overlap-diagnostic",
                "reads exact output after the actual native children join",
            )
            .unwrap();
        drop(allocation);
        let mut first = children.remove(0);
        assert_eq!(unsafe { kill(first.id() as i32, signal) }, 0);
        assert!(!first.wait().unwrap().success());
        assert_eq!(h.auto("maintain", &[]).0, 0);
        assert!(root.join("cache 空白/output").exists());
        streams[1].write_all(b"r").unwrap();
        assert!(children[0].wait().unwrap().success());
        assert_eq!(h.auto("maintain", &[]).0, 0);
        assert_eq!(
            fs::read(root.join("cache 空白/output")).unwrap(),
            b"rebuildable"
        );
        diagnostic
            .release(
                "overlap-diagnostic",
                "both native children joined and original assertion completed",
            )
            .unwrap();
        assert_eq!(h.auto("maintain", &[]).0, 0);
        assert!(!root.join("cache 空白").exists());
        assert_eq!(fs::read(root.join("source.rs")).unwrap(), b"unique source");
    }
}

#[test]
fn prospective_outputs_cannot_claim_git_identity_or_native_capture_originals() {
    let h = host();
    let (store, _) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    for output in [
        ".git/objects/",
        "nested/.git/",
        ".chrono-harness/state/command-captures/",
        "stdout.bin/",
    ] {
        assert!(
            chrono_worktree::TemporaryHost::allocate(
                &store,
                "test-allocation",
                &[output.into()],
                "invalid λ "
            )
            .is_err()
        );
    }
}

#[test]
fn unavailable_lease_preserves_its_registration_and_allows_an_eligible_allocation() {
    let h = host();
    let (store, outputs) =
        chrono_worktree::TemporaryHost::registered_store(&h.root, POLICY, "test-allocation")
            .unwrap();
    let allocation = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "lost lease λ ",
    )
    .unwrap();
    let root = allocation.path().to_owned();
    fs::create_dir(root.join("cache 空白")).unwrap();
    fs::write(root.join("cache 空白/original"), "preserve").unwrap();
    drop(allocation);
    let before =
        json(&fs::read(store.join("objects.json")).unwrap()).unwrap()["objects"][0].clone();
    let lease = before["cache_lease"]["path"].as_str().unwrap();
    fs::rename(store.join(lease), store.join("retained-unavailable-lease")).unwrap();
    let sibling = chrono_worktree::TemporaryHost::allocate(
        &store,
        "test-allocation",
        &outputs,
        "released sibling λ ",
    )
    .unwrap();
    let generated = sibling.path().join("cache 空白");
    fs::create_dir(&generated).unwrap();
    fs::write(generated.join("rebuildable"), "generated").unwrap();
    drop(sibling);
    let (exit, report, _) = h.auto("maintain", &[]);
    assert_eq!(exit, 2, "{report}");
    assert_eq!(
        json(&fs::read(store.join("objects.json")).unwrap()).unwrap()["objects"][0],
        before
    );
    assert_eq!(
        fs::read(root.join("cache 空白/original")).unwrap(),
        b"preserve"
    );
    assert!(!generated.exists());
}

#[test]
fn real_coordinator_adoption_protects_native_use_then_recovers_registered_cargo_output() {
    use std::io::{Read, Write};
    let coordinator = source();
    let (store, outputs) = chrono_worktree::TemporaryHost::registered_store(
        &coordinator,
        POLICY,
        "worktree-test-host",
    )
    .unwrap();
    let allocation = chrono_worktree::TemporaryHost::allocate(
        &store,
        "worktree-test-host",
        &outputs,
        "real coordinator λ ",
    )
    .unwrap();
    let root = allocation.path().to_owned();
    let checkout = root.join("source with spaces");
    let project = checkout.join("independent/producer");
    fs::create_dir_all(project.join("src")).unwrap();
    fs::write(
        project.join("Cargo.toml"),
        include_bytes!("support/cargo_pair/producer/Cargo.toml"),
    )
    .unwrap();
    fs::write(
        project.join("Cargo.lock"),
        include_bytes!("support/cargo_pair/producer/Cargo.lock"),
    )
    .unwrap();
    fs::write(
        project.join("src/lib.rs"),
        include_bytes!("support/cargo_pair/producer/src/lib.rs"),
    )
    .unwrap();
    fs::write(
        checkout.join(".gitignore"),
        "independent/producer/target/\n",
    )
    .unwrap();
    git(&checkout, &["init", "-q"]);
    let original_head = commit(&checkout);
    let original = fs::read(project.join("src/lib.rs")).unwrap();
    let mut cargo = Command::new("cargo");
    cargo
        .current_dir(&checkout)
        .env_remove("CARGO_TARGET_DIR")
        .args([
            "build",
            "--offline",
            "--locked",
            "--manifest-path",
            "independent/producer/Cargo.toml",
        ]);
    let output = allocation.capture_output(&mut cargo).unwrap();
    assert!(output.status.success(), "{output:?}");
    drop(cargo);
    let selected =
        "source with spaces/independent/producer/target/debug/libbounded_cache_producer.rlib";
    assert!(root.join(selected).is_file());
    let unknown = store.join(format!(
        "unknown-neighbor-{}",
        root.file_name().unwrap().to_str().unwrap()
    ));
    fs::create_dir(&unknown).unwrap();
    fs::write(unknown.join("original"), [255, 0, 10]).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    command
        .arg("custody-holder")
        .env_clear()
        .env("CHRONO_TEMPORARY_CARGO_ROOT", &root)
        .env("CHRONO_TEMPORARY_MEMBER", selected)
        .env(
            "CHRONO_TEMPORARY_SYNC",
            listener.local_addr().unwrap().to_string(),
        );
    allocation.bind_command(&mut command).unwrap();
    let mut child = command.spawn().unwrap();
    drop(command);
    let (mut stream, _) = listener.accept().unwrap();
    stream.read_exact(&mut [0; 5]).unwrap();
    drop(allocation);
    let maintain = || {
        let mut command = Command::new(coordinator.join(".chrono-harness/bin/chrono-worktree"));
        command.current_dir(&coordinator).arg("maintain");
        chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
        let output =
            chrono_worktree::TemporaryHost::capture_bound_output(&root, &mut command).unwrap();
        let report = json(&output.stdout).unwrap();
        let failures = report["cleanup_failures"].as_array().unwrap();
        assert_eq!(
            output.status.code(),
            Some(if failures.is_empty() { 0 } else { 2 }),
            "{report}"
        );
        report
    };
    let protected = maintain();
    assert!(project.join("target").exists());
    assert!(
        protected["producer_objects"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["producer"] == "worktree-test-host"
                && row["status"] == "protected"
                && row["path"]
                    .as_str()
                    .unwrap()
                    .ends_with(root.file_name().unwrap().to_str().unwrap()))
    );
    stream.write_all(b"r").unwrap();
    assert!(child.wait().unwrap().success());
    let reclaimed = maintain();
    assert!(!project.join("target").exists(), "{reclaimed}");
    assert_eq!(fs::read(project.join("src/lib.rs")).unwrap(), original);
    assert_eq!(git(&checkout, &["rev-parse", "HEAD"]), original_head);
    assert_eq!(fs::read(unknown.join("original")).unwrap(), [255, 0, 10]);
    assert!(project.join("Cargo.toml").is_file() && project.join("Cargo.lock").is_file());
}


#[test]
fn retained_lifecycle_command_preserves_real_allocation_exclusion_after_host_drop() {
    use std::os::fd::AsRawFd;
    unsafe extern "C" {
        fn flock(fd: i32, operation: i32) -> i32;
    }
    let h = super::bounded_cleanup::configure_main(Host::retained("payload"), "allocation-lease");
    let allocation = h._dir.path().to_owned();
    let directory = allocation.parent().unwrap();
    let registry = json(&fs::read(directory.join("objects.json")).unwrap()).unwrap();
    let record = registry["objects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|object| object["path"] == allocation.file_name().unwrap().to_str().unwrap())
        .unwrap();
    let lease_path = directory.join(record["cache_lease"]["path"].as_str().unwrap());
    let path = h.root.join(PROJECTS);
    let mut projects = json(&fs::read(&path).unwrap()).unwrap();
    projects["scripts"][0]["actions"]["execute"]["argv"]
        .as_array_mut()
        .unwrap()
        .push(value!(lease_path));
    fs::write(path, serde_json::to_vec(&projects).unwrap()).unwrap();
    commit(&h.root);
    let mut command = h.auto_command("use", &["--operation", "use.consumer"]);
    let root = h.root.clone();
    drop(h);
    // This capture adapter does not bind capabilities. Retire the command's
    // parent copies immediately after spawn: only the actual child can hold it.
    let mut child = super::interrupted_cleanup::CapturedChild::spawn(&mut command, &root);
    drop(command);
    let output = child.wait_with_output().unwrap();
    let report = json(&output.stdout).unwrap();
    assert!(output.status.success(), "{}", report["error"]);
    assert_eq!(report["managed_process"]["exit_code"], 0);
    assert_eq!(report["managed_process"]["stdout_bytes"], value!(b"allocation-live\xff\0".as_slice()));
    assert_eq!(report["managed_process"]["stderr_bytes"], value!(b"original-native-stderr\xfe".as_slice()));
    let exclusion = fs::File::open(lease_path).unwrap();
    assert_eq!(unsafe { flock(exclusion.as_raw_fd(), 2 | 4) }, 0);
}

#[test]
fn retained_lifecycle_adapter_captures_actual_failed_result_and_original_streams() {
    let h = super::bounded_cleanup::configure_main(Host::retained("payload"), "capture-once");
    let (code, report, error) = h.auto("use", &["--operation", "use.consumer"]);
    assert_eq!(code, 2, "{error}");
    assert_eq!(report["managed_process"]["exit_code"], 7);
    assert_eq!(report["managed_process"]["stdout_bytes"], value!(b"\xffpartial stdout\0\n".as_slice()));
    assert_eq!(report["managed_process"]["stderr_bytes"], value!(b"\xfepartial stderr\0\n".as_slice()));
    let capture = capture_directory(h._dir.path());
    assert_eq!(json(&fs::read(capture.join("stdout.bytes")).unwrap()).unwrap(), report);
    assert!(fs::read(capture.join("stderr.bytes")).unwrap().is_empty());
    let joined = json(&fs::read(capture.join("result.json")).unwrap()).unwrap();
    assert_eq!(joined["exit_code"], 2);
    assert_eq!(joined["joined"], true);
}
