use super::*;
use std::io::{BufRead, BufReader, Write};
use std::process::Stdio;
const PROFILE: &str = ".chrono-harness/ci/owner-stage.json";

fn fixture() -> (Host, PathBuf, String, String) {
    fixture_with_publication_probe(None)
}
fn fixture_with_publication_probe(drift: Option<bool>) -> (Host, PathBuf, String, String) {
    fixture_with_bindings(drift, false)
}
fn fixture_with_bindings(drift: Option<bool>, registry: bool) -> (Host, PathBuf, String, String) {
    let h = Host::new("payload");
    h.kernel_cleanup();
    let cleanup = h.root.join(".chrono-harness/cleanup.json");
    let mut p = json(&fs::read(&cleanup).unwrap()).unwrap();
    p["main_cache_only"] = value!(true);
    fs::write(cleanup, serde_json::to_vec_pretty(&p).unwrap()).unwrap();
    let ci = h.root.join(".chrono-harness/ci");
    fs::create_dir_all(&ci).unwrap();
    let mut files = vec![PROFILE.to_owned(), ".chrono-harness/process_fds.py".into()];
    for name in ["bootstrap.py", "bootstrap_owner.py", "bootstrap_shared.py"] {
        fs::copy(
            source().join(".chrono-harness/ci").join(name),
            ci.join(name),
        )
        .unwrap();
        files.push(format!(".chrono-harness/ci/{name}"));
    }
    fs::copy(
        source().join(".chrono-harness/process_fds.py"),
        h.root.join(".chrono-harness/process_fds.py"),
    )
    .unwrap();
    let bin = h.root.join(".chrono-harness/bin");
    fs::create_dir_all(&bin).unwrap();
    fs::write(bin.join("chrono-worktree"), "ordinary original bytes").unwrap();
    fs::copy(
        source().join("crates/worktree/target/debug/chrono-worktree"),
        bin.join("built-owner"),
    )
    .unwrap();
    let tools = h.parent.join("owner-tools");
    fs::create_dir(&tools).unwrap();
    for name in ["rustup", "cargo", "rustc"] {
        fs::copy(
            env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"),
            tools.join(name),
        )
        .unwrap();
    }
    let probe = env!("CARGO_BIN_EXE_chrono-worktree-owner-publication-probe");
    let mut config = json(&fs::read(h.root.join(CONFIG)).unwrap()).unwrap();
    config["tools"].as_array_mut().unwrap().push(value!({"id":"owner-probe","program":probe,"resolution":"PATH-once","version_argv":["--version"],"expected_version":"owner probe"}));
    fs::write(
        h.root.join(CONFIG),
        serde_json::to_vec_pretty(&config).unwrap(),
    )
    .unwrap();
    // One real operation supplies original stdout/stderr for deployment checks.
    let projects_path = h.root.join(PROJECTS);
    let mut projects = json(&fs::read(&projects_path).unwrap()).unwrap();
    projects["scripts"] = value!([
        {"id":"construction","path":".chrono-harness/ci/bootstrap.py","test_script":"construction-tests",
         "actions":{"execute":{"operation":"owner.fixture-version","tool":"cargo","argv":["--version"]}}},
        {"id":"construction-tests","path":".chrono-harness/ci/bootstrap_shared.py","tests_for":"construction",
         "actions":{"execute":{"operation":"owner.fixture-test","tool":"cargo","argv":["--version"]}}}
    ]);
    fs::write(projects_path, serde_json::to_vec_pretty(&projects).unwrap()).unwrap();
    let mut profile = value!({"schema":"chrono-bootstrap/v1","projects":PROJECTS,"rust_toolchain":"1.95.0",
        "tools":{"cargo":tools.join("cargo"),"rustc":tools.join("rustc")},"operations":["owner.fixture-version"],"install":[],
        "require_tool_bindings":true,"rust_incremental":false,
        "native_platform":{"system":if cfg!(target_os="macos") {"Darwin"} else {"Linux"},
            "machine":if cfg!(all(target_os="macos",target_arch="aarch64")) {"arm64"} else {std::env::consts::ARCH}},
        "tool_bindings":{"cargo":{"path":tools.join("cargo"),"sha256":sha256(&fs::read(tools.join("cargo")).unwrap()),"version":"cargo 1.95.0 (fixture)"},
            "rustc":{"path":tools.join("rustc"),"sha256":sha256(&fs::read(tools.join("rustc")).unwrap()),"version":"rustc 1.95.0 (fixture)"}},
        "environment":{"PATH":std::env::join_paths(std::iter::once(tools).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap()))).unwrap().to_str().unwrap()},
        "entrypoint":{"tool":"owner-probe","script":".chrono-harness/ci/bootstrap.py"},
        "participation":{"coordinator_root":"git-main-worktree","program":".chrono-harness/bin/chrono-worktree","git":fixture_git(),"config":POLICY},
        "owner_stage":{"source":".chrono-harness/bin/built-owner","selected":".chrono-harness/bin/chrono-worktree"}});
    if registry {
        let path = ".chrono-harness/ci/owner-toolchains.json";
        let native = &profile["native_platform"];
        let platform = format!(
            "{}/{}",
            native["system"].as_str().unwrap(),
            native["machine"].as_str().unwrap()
        );
        let mut tools = profile["tool_bindings"].clone();
        for item in tools.as_object_mut().unwrap().values_mut() {
            item.as_object_mut().unwrap().remove("path");
        }
        let registration = value!({"schema":"chrono-bootstrap-toolchains/v1",
            "rust_toolchain":"1.95.0","platforms":{platform:{"tools":tools}}});
        fs::write(
            h.root.join(path),
            serde_json::to_vec_pretty(&registration).unwrap(),
        )
        .unwrap();
        files.push(path.into());
        let declaration = profile.as_object_mut().unwrap();
        declaration.remove("tool_bindings");
        declaration.remove("native_platform");
        declaration.insert("tool_bindings_registry".into(), value!(path));
        declaration.insert("tools".into(), value!({}));
    }
    fs::write(
        h.root.join(PROFILE),
        serde_json::to_vec_pretty(&profile).unwrap(),
    )
    .unwrap();
    let mut fm = json(&fs::read(h.root.join(FM)).unwrap()).unwrap();
    for path in files {
        fm["files"]
            .as_array_mut()
            .unwrap()
            .push(file(&path, value!([])));
    }
    fs::write(h.root.join(FM), serde_json::to_vec_pretty(&fm).unwrap()).unwrap();
    let probe_input = drift.map(|drift| {
        automatic::native_git(
            &h,
            "owner-publication",
            value!({"root":h.root,"drift":drift}),
        )
    });
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let (code, report, error) = h.auto("bootstrap", &["--bootstrap-config", PROFILE]);
    assert_eq!(code, 0, "{report} {error}");
    let stdout = String::from_utf8(
        serde_json::from_value(report["managed_process"]["stdout_bytes"].clone()).unwrap(),
    )
    .unwrap();
    let descriptor = json(
        stdout
            .lines()
            .last()
            .unwrap()
            .strip_prefix("chrono-owner-stage-result ")
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
    let path = descriptor["path"].as_str().unwrap().to_owned();
    let sha = descriptor["sha256"].as_str().unwrap().to_owned();
    let stage = json(&fs::read(h.root.join(&path)).unwrap()).unwrap();
    let executable = h
        .root
        .join(stage["owner_stage"]["staged"]["path"].as_str().unwrap());
    if let Some(input) = probe_input {
        let mut config = json(&fs::read(&input).unwrap()).unwrap();
        config["publication"] =
            value!(executable.parent().unwrap().join("publication-result.json"));
        fs::write(input, serde_json::to_vec(&config).unwrap()).unwrap();
    }
    (h, executable, path, sha)
}
fn deploy(h: &Host, executable: &Path, path: &str, sha: &str) -> (i32, Value, String) {
    let c = h.auto_command(
        "deploy-owner",
        &[
            "--bootstrap-config",
            PROFILE,
            "--stage-result",
            path,
            "--stage-sha256",
            sha,
        ],
    );
    let mut actual = Command::new(executable);
    actual
        .args(c.get_args())
        .current_dir(&h.root)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap());
    h.received(h.native_output(&mut actual))
}
#[test]
fn owner_deployment_publishes_in_the_rust_lease_owner() {
    let (h, exe, path, sha) = fixture();
    let (code, report, error) = deploy(&h, &exe, &path, &sha);
    assert_eq!(code, 0, "{report} {error}");
    let stage = json(&fs::read(h.root.join(path)).unwrap()).unwrap();
    let original = exe.parent().unwrap().join("previous-owner");
    assert_eq!(
        chrono_harness::ownership::identity(&original).unwrap(),
        stage["owner_stage"]["previous"]["identity"]
    );
    assert!(report.get("managed_process").is_none(), "{report}");
    let publication = &report["owner_deployment"]["publication"];
    assert_eq!(
        publication["installed"]["identity"],
        chrono_harness::ownership::identity(&h.root.join(".chrono-harness/bin/chrono-worktree"))
            .unwrap()
    );
    assert_eq!(
        *publication,
        json(&fs::read(exe.parent().unwrap().join("publication-result.json")).unwrap()).unwrap()
    );
}

#[test]
fn owner_deployment_retains_original_ex_through_publication_and_adoption() {
    let (h, exe, path, sha) = fixture_with_publication_probe(Some(false));
    let before = h.ledger()["entries"][0].clone();
    let (code, report, error) = deploy(&h, &exe, &path, &sha);
    assert_eq!(code, 0, "{report} {error}");
    let after = h.ledger()["entries"][0].clone();
    assert_eq!(
        before, after,
        "deployment preserves original enrollment and task status"
    );
    assert_eq!(
        fs::read(h.root.join(".chrono-harness/bin/chrono-worktree")).unwrap(),
        fs::read(&exe).unwrap()
    );
    assert_eq!(report["owner_deployment"]["task_completion"], "not-claimed");
    assert!(
        h.root
            .join(".chrono-harness/state/owner-exclusions-checked")
            .exists()
    );
    assert_eq!(
        deploy(&h, &exe, &path, &sha).0,
        0,
        "same original publication is recoverable"
    );

    let (h, exe, path, sha) = fixture();
    let target = linked(&h);
    let policy_path = target.join(POLICY);
    let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
    policy["timeout_seconds"] = value!(policy["timeout_seconds"].as_u64().unwrap() + 1);
    let policy_bytes = serde_json::to_vec_pretty(&policy).unwrap();
    fs::write(&policy_path, &policy_bytes).unwrap();
    commit(&target);
    let before = h.ledger();
    let (code, report, error) = deploy(&h, &exe, &path, &sha);
    assert_eq!(
        code, 0,
        "excluded linked consumers do not couple deployment to their policy: {report} {error}"
    );
    assert_eq!(
        h.ledger(),
        before,
        "deployment does not adopt linked policy"
    );
    assert_eq!(fs::read(&policy_path).unwrap(), policy_bytes);
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
    assert!(
        report.to_string().contains("policy/configuration"),
        "{report}"
    );
    assert!(report.get("managed_process").is_none());
}
fn inherited_holder(h: &Host, entry: &Value) -> std::process::Child {
    let lease = chrono_harness::ownership::Lease::acquire(
        &h.root.join(entry["ownership"]["path"].as_str().unwrap()),
        Some(entry["ownership"]["id"].as_str().unwrap()),
        false,
        false,
        None,
    )
    .unwrap()
    .unwrap();
    let mut command = Command::new(env!(
        "CARGO_BIN_EXE_chrono-worktree-owner-publication-probe"
    ));
    command
        .args(["--hold-descendants", "3"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
    let mut child = command.spawn().unwrap();
    let mut ready = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut ready)
        .unwrap();
    drop(lease);
    assert_eq!(ready, "ready\n");
    child
}
fn join_holder(mut child: std::process::Child) {
    child.stdin.take().unwrap().write_all(b"x").unwrap();
    assert!(child.wait().unwrap().success());
}
fn linked(h: &Host) -> PathBuf {
    let target = h.parent.join("ordinary-linked");
    let (code, report, error) = h.invoke("feature", "ordinary-linked", &target);
    assert_eq!(code, 0, "{report} {error}");
    target
}

#[test]
fn owner_deployment_excludes_main_and_linked_inherited_descendants_until_join() {
    for linked_holder in [false, true] {
        let (h, exe, path, sha) = fixture();
        let target = linked(&h);
        let entries = h.ledger()["entries"].as_array().unwrap().clone();
        let entry = entries
            .iter()
            .find(|e| e["path"] == value!(if linked_holder { &target } else { &h.root }))
            .unwrap();
        let before = fs::read(h.root.join(".chrono-harness/bin/chrono-worktree")).unwrap();
        let child = inherited_holder(&h, entry);
        let (code, report, error) = deploy(&h, &exe, &path, &sha);
        join_holder(child);
        assert_ne!(code, 0, "{report} {error}");
        assert!(
            report.to_string().contains("live inherited consumers"),
            "{report}"
        );
        assert_eq!(
            fs::read(h.root.join(".chrono-harness/bin/chrono-worktree")).unwrap(),
            before
        );
        assert!(
            !h.root
                .join(".chrono-harness/state/bootstrap-owner-current.json")
                .exists()
        );
        let (code, report, error) = deploy(&h, &exe, &path, &sha);
        assert_eq!(code, 0, "{report} {error}");
        let excluded = report["owner_deployment"]["original_protection"]["enrollments"]
            .as_array()
            .unwrap();
        assert_eq!(excluded.len(), 2);
        for e in &entries {
            assert!(excluded.iter().any(|x| x["path"] == e["path"]
                && x["attachment"] == e["attachment"]
                && x["enrollment"]["id"] == e["ownership"]["id"]));
        }
        assert_eq!(h.ledger()["entries"], value!(entries));
        assert!(h.ledger_path(&h.root).exists());
    }
}

#[test]
fn owner_deployment_does_not_exclude_an_unrelated_physical_coordinator() {
    let (h, exe, path, sha) = fixture();
    let (other, _, _, _) = fixture();
    let child = inherited_holder(&other, &other.ledger()["entries"][0]);
    let (code, report, error) = deploy(&h, &exe, &path, &sha);
    join_holder(child);
    assert_eq!(code, 0, "{report} {error}");
    assert_eq!(
        report["owner_deployment"]["original_protection"]["enrollments"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn owner_deployment_preserves_unknown_or_drifted_affected_enrollment() {
    for fault in ["unknown", "ownership", "lease", "attachment"] {
        let (h, exe, path, sha) = fixture();
        let target = linked(&h);
        let mut ledger = h.ledger();
        let e = ledger["entries"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|e| e["path"] == value!(target))
            .unwrap();
        let expected = match fault {
            "unknown" => {
                ledger["entries"].as_array_mut().unwrap().pop();
                "no current enrollment"
            }
            "ownership" => {
                e.as_object_mut().unwrap().remove("ownership");
                "no kernel ownership"
            }
            "lease" => {
                let lease = h.root.join(e["ownership"]["path"].as_str().unwrap());
                fs::rename(&lease, lease.with_extension("original")).unwrap();
                fs::write(lease, "changed physical identity").unwrap();
                "ownership lease identity changed"
            }
            _ => {
                let gitfile = target.join(".git");
                let bytes = fs::read(&gitfile).unwrap();
                fs::rename(&gitfile, target.join("original-gitfile")).unwrap();
                fs::write(&gitfile, bytes).unwrap();
                "attachment changed"
            }
        };
        h.set_ledger(&ledger);
        let old_ledger = h.ledger();
        let before = fs::read(h.root.join(".chrono-harness/bin/chrono-worktree")).unwrap();
        let (code, report, error) = deploy(&h, &exe, &path, &sha);
        assert_ne!(code, 0, "{fault}: {report} {error}");
        assert!(report.to_string().contains(expected), "{fault}: {report}");
        assert_eq!(h.ledger(), old_ledger);
        assert_eq!(
            fs::read(h.root.join(".chrono-harness/bin/chrono-worktree")).unwrap(),
            before
        );
        assert!(exe.exists());
    }
}

#[test]
fn owner_deployment_recovers_after_rename_when_result_publication_fails() {
    let (h, exe, path, sha) = fixture();
    let stage = json(&fs::read(h.root.join(&path)).unwrap()).unwrap();
    let result = exe.parent().unwrap().join("publication-result.json");
    // A real result IO failure occurs after replacement, without a test-only
    // product entrypoint or hand-written successful intent/publication.
    fs::create_dir(&result).unwrap();
    let (code, report, error) = deploy(&h, &exe, &path, &sha);
    assert_ne!(code, 0, "{report} {error}");
    assert!(
        report.to_string().contains("physical regular file"),
        "{report}"
    );
    let selected = h.root.join(".chrono-harness/bin/chrono-worktree");
    assert_eq!(fs::read(&selected).unwrap(), fs::read(&exe).unwrap());
    let installed = chrono_harness::ownership::identity(&selected).unwrap();
    assert_eq!(
        chrono_harness::ownership::identity(&exe.parent().unwrap().join("previous-owner")).unwrap(),
        stage["owner_stage"]["previous"]["identity"]
    );
    assert!(
        exe.parent()
            .unwrap()
            .join("publication-intent.json")
            .is_file()
    );
    assert!(
        !h.root
            .join(".chrono-harness/state/bootstrap-owner-current.json")
            .exists()
    );
    fs::remove_dir(&result).unwrap();
    let (code, report, error) = deploy(&h, &exe, &path, &sha);
    assert_eq!(code, 0, "{report} {error}");
    assert_eq!(
        report["owner_deployment"]["installed"]["identity"],
        installed
    );
    assert_eq!(
        json(&fs::read(result).unwrap()).unwrap(),
        report["owner_deployment"]["publication"]
    );
}

#[test]
fn owner_deployment_rejects_same_published_bytes_from_another_inode() {
    let (h, exe, path, sha) = fixture();
    let (code, report, error) = deploy(&h, &exe, &path, &sha);
    assert_eq!(code, 0, "{report} {error}");
    let selected = h.root.join(".chrono-harness/bin/chrono-worktree");
    let foreign = selected.with_extension("foreign");
    fs::copy(&selected, &foreign).unwrap();
    fs::rename(foreign, &selected).unwrap();
    let original_adoption = fs::read(
        h.root
            .join(".chrono-harness/state/bootstrap-owner-current.json"),
    )
    .unwrap();
    let (code, report, error) = deploy(&h, &exe, &path, &sha);
    assert_ne!(code, 0, "{report} {error}");
    assert!(
        report
            .to_string()
            .contains("drifted after publication intent"),
        "{report}"
    );
    assert_eq!(
        fs::read(
            h.root
                .join(".chrono-harness/state/bootstrap-owner-current.json")
        )
        .unwrap(),
        original_adoption
    );
}

#[test]
fn owner_deployment_rejects_changed_publication_inputs_without_replacement() {
    for fault in [
        "selected",
        "staged",
        "previous",
        "unqualified-next",
        "symlink-previous",
        "symlink-intent",
        "symlink-next",
        "symlink-result",
    ] {
        let (h, exe, path, sha) = fixture();
        let selected = h.root.join(".chrono-harness/bin/chrono-worktree");
        match fault {
            "selected" => fs::write(&selected, "changed selected").unwrap(),
            "staged" => {
                let foreign = exe.with_extension("foreign");
                fs::copy(&exe, &foreign).unwrap();
                fs::rename(foreign, &exe).unwrap();
            }
            "previous" => fs::write(
                exe.parent().unwrap().join("previous-owner"),
                "foreign owner",
            )
            .unwrap(),
            "unqualified-next" => fs::write(
                exe.parent().unwrap().join("publication-next-owner"),
                "unqualified preparation",
            )
            .unwrap(),
            _ => {
                let name = match fault {
                    "symlink-previous" => "previous-owner",
                    "symlink-intent" => "publication-intent.json",
                    "symlink-next" => "publication-next-owner",
                    _ => "publication-result.json",
                };
                std::os::unix::fs::symlink(&selected, exe.parent().unwrap().join(name)).unwrap();
            }
        }
        let before = fs::read(&selected).unwrap();
        let (code, report, error) = deploy(&h, &exe, &path, &sha);
        assert_ne!(code, 0, "{fault}: {report} {error}");
        assert_eq!(fs::read(&selected).unwrap(), before, "{fault}");
        assert!(
            !h.root
                .join(".chrono-harness/state/bootstrap-owner-current.json")
                .exists(),
            "{fault}"
        );
    }
}

#[test]
fn owner_deployment_requires_complete_stage_validation_before_native_effects() {
    for field in ["registrations", "operations", "installed"] {
        let (h, exe, path, _) = fixture();
        let selected = h.root.join(".chrono-harness/bin/chrono-worktree");
        let before = fs::read(&selected).unwrap();
        let mut stage = json(&fs::read(h.root.join(&path)).unwrap()).unwrap();
        stage[field] = value!("invalid original construction input");
        let raw = serde_json::to_vec_pretty(&stage).unwrap();
        fs::write(h.root.join(&path), &raw).unwrap();
        let (code, report, error) = deploy(&h, &exe, &path, &sha256(&raw));
        assert_ne!(code, 0, "{field}: {report} {error}");
        assert_eq!(fs::read(&selected).unwrap(), before, "{field}");
        assert!(
            !exe.parent()
                .unwrap()
                .join("publication-intent.json")
                .exists()
        );
    }
}

#[test]
fn owner_deployment_publishes_without_the_construction_interpreter() {
    let (h, exe, path, sha) = fixture();
    fs::write(
        h.root.join(".chrono-harness/state/owner-probe-unavailable"),
        "construction interpreter is no longer available",
    )
    .unwrap();
    let (code, report, error) = deploy(&h, &exe, &path, &sha);
    assert_eq!(code, 0, "{report} {error}");
    assert!(report.get("managed_process").is_none(), "{report}");
    assert_eq!(
        fs::read(h.root.join(".chrono-harness/bin/chrono-worktree")).unwrap(),
        fs::read(exe).unwrap()
    );
    assert_eq!(report["owner_deployment"]["task_completion"], "not-claimed");
}

#[test]
fn owner_deployment_qualifies_the_selected_native_toolchain_registry() {
    let (h, exe, path, sha) = fixture_with_bindings(None, true);
    let stage = json(&fs::read(h.root.join(&path)).unwrap()).unwrap();
    assert!(stage["registrations"][".chrono-harness/ci/owner-toolchains.json"].is_string());
    fs::write(
        h.root.join(".chrono-harness/state/owner-probe-unavailable"),
        "construction interpreter is no longer available",
    )
    .unwrap();
    let (code, report, error) = deploy(&h, &exe, &path, &sha);
    assert_eq!(code, 0, "{report} {error}");
    assert!(report.get("managed_process").is_none(), "{report}");
    assert_eq!(
        fs::read(h.root.join(".chrono-harness/bin/chrono-worktree")).unwrap(),
        fs::read(exe).unwrap()
    );
}

#[test]
fn owner_deployment_keeps_publication_evidence_when_original_admission_drifts() {
    let (h, exe, path, sha) = fixture_with_publication_probe(Some(true));
    let (code, report, error) = deploy(&h, &exe, &path, &sha);
    assert_ne!(code, 0, "{report} {error}");
    assert!(
        report
            .to_string()
            .contains("ownership lease identity changed"),
        "{report}"
    );
    assert!(
        !h.root
            .join(".chrono-harness/state/bootstrap-owner-current.json")
            .exists()
    );
    assert!(
        exe.parent()
            .unwrap()
            .join("publication-result.json")
            .exists()
    );
    assert!(exe.parent().unwrap().join("previous-owner").exists());
}

#[test]
fn owner_deployment_preserves_bytes_on_stage_original_or_execution_drift() {
    for fault in [
        "source",
        "registrations",
        "operations",
        "argv",
        "exit",
        "pid",
        "stdout",
        "staged",
        "compiler",
        "environment",
        "dirty-source",
        "duplicate-stream",
        "stream-size",
        "missing-pid",
        "missing-exit",
        "compiler-relative",
        "selection",
        "versions",
        "cwd",
    ] {
        let (h, exe, path, _) = fixture();
        let target = h.root.join(".chrono-harness/bin/chrono-worktree");
        let before = binding(&target);
        let mut stage = json(&fs::read(h.root.join(&path)).unwrap()).unwrap();
        match fault {
            "source" => stage["source"]["before"]["dirty"] = value!(true),
            "registrations" => stage["registrations"][PROFILE] = value!("different"),
            "operations" => stage["operations"] = value!([]),
            "argv" => stage["operations"][0]["argv"][1] = value!("foreign"),
            "exit" => stage["operations"][0]["exit_code"] = value!(1),
            "pid" => stage["operations"][0]["pid"] = value!(0),
            "stdout" => {
                let path = h
                    .root
                    .join(stage["operations"][0]["stdout"]["path"].as_str().unwrap());
                fs::write(path, "changed original output").unwrap();
            }
            "duplicate-stream" => {
                stage["operations"][0]["stderr"] = stage["operations"][0]["stdout"].clone()
            }
            "stream-size" => stage["operations"][0]["stdout"]["size"] = value!(u64::MAX),
            "missing-pid" => {
                stage["operations"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("pid");
            }
            "missing-exit" => {
                stage["operations"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("exit_code");
            }
            "compiler-relative" => stage["executables"][1]["path"] = value!("relative/compiler"),
            "selection" => stage["selection"]["rustc"] = value!("foreign compiler"),
            "versions" => stage["versions"]["rustc"] = value!("foreign version"),
            "cwd" => stage["operations"][0]["cwd"] = value!("/foreign"),
            "staged" => stage["owner_stage"]["staged"]["sha256"] = value!("0".repeat(64)),
            "compiler" => stage["executables"][1]["sha256"] = value!("0".repeat(64)),
            "environment" => stage["environment"]["RUSTC"] = value!("foreign compiler"),
            "dirty-source" => {
                fs::write(h.root.join("payload"), "changed source after construction").unwrap()
            }
            _ => unreachable!(),
        }
        let bytes = serde_json::to_vec_pretty(&stage).unwrap();
        fs::write(h.root.join(&path), &bytes).unwrap();
        let (code, report, error) = deploy(&h, &exe, &path, &sha256(&bytes));
        assert_ne!(code, 0, "{fault}: {report} {error}");
        assert_eq!(binding(&target), before, "{fault}");
        assert!(
            !exe.parent()
                .unwrap()
                .join("publication-intent.json")
                .exists(),
            "{fault}"
        );
        assert!(report.get("managed_process").is_none(), "{fault}: {report}");
    }
}

fn binding(path: &Path) -> Value {
    let bytes = fs::read(path).unwrap();
    value!({"identity":chrono_harness::ownership::identity(path).unwrap(),"sha256":sha256(&bytes),"size":bytes.len()})
}
