use super::*;
use std::io::{BufRead, BufReader, Write};
use std::process::Stdio;
const PROFILE: &str = ".chrono-harness/ci/owner-stage.json";
const LEDGER: &str = ".chrono-harness/state/automatic-cleanup/ledger.json";

fn fixture() -> (Host, PathBuf, String, String) {
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
        std::os::unix::fs::symlink(
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
    let profile = value!({"schema":"chrono-bootstrap/v1","projects":PROJECTS,"rust_toolchain":"1.95.0",
        "tools":{"cargo":tools.join("cargo"),"rustc":tools.join("rustc")},"operations":[],"install":[],
        "environment":{"PATH":std::env::join_paths(std::iter::once(tools).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap()))).unwrap().to_str().unwrap()},
        "entrypoint":{"tool":"owner-probe","script":".chrono-harness/ci/bootstrap.py"},
        "participation":{"coordinator_root":"git-main-worktree","program":".chrono-harness/bin/chrono-worktree","git":fixture_git(),"config":POLICY},
        "owner_stage":{"source":".chrono-harness/bin/built-owner","selected":".chrono-harness/bin/chrono-worktree"}});
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
fn owner_deployment_retains_original_ex_through_publication_and_adoption() {
    let (h, exe, path, sha) = fixture();
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
    assert_eq!(
        deploy(&h, &exe, &path, &sha).0,
        0,
        "same original publication is recoverable"
    );
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
        assert!(h.root.join(LEDGER).exists());
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
        fs::write(
            h.root.join(LEDGER),
            serde_json::to_vec_pretty(&ledger).unwrap(),
        )
        .unwrap();
        let old_ledger = fs::read(h.root.join(LEDGER)).unwrap();
        let before = fs::read(h.root.join(".chrono-harness/bin/chrono-worktree")).unwrap();
        let (code, report, error) = deploy(&h, &exe, &path, &sha);
        assert_ne!(code, 0, "{fault}: {report} {error}");
        assert!(report.to_string().contains(expected), "{fault}: {report}");
        assert_eq!(fs::read(h.root.join(LEDGER)).unwrap(), old_ledger);
        assert_eq!(
            fs::read(h.root.join(".chrono-harness/bin/chrono-worktree")).unwrap(),
            before
        );
        assert!(exe.exists());
    }
}
#[test]
fn owner_deployment_keeps_publication_evidence_when_original_admission_drifts() {
    let (h, exe, path, sha) = fixture();
    fs::write(
        h.root.join(".chrono-harness/state/owner-probe-drift"),
        "inject fixture drift",
    )
    .unwrap();
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
