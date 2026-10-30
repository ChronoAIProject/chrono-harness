use super::*;
use std::io::{BufRead, BufReader, Write};
use std::process::Stdio;
const PROFILE: &str = ".chrono-harness/ci/owner-stage.json";
const LEDGER: &str = ".chrono-harness/state/automatic-cleanup/ledger.json";

fn fixture(scope: &str) -> (Host, PathBuf, String, String) {
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
        "environment":{"PATH":std::env::join_paths(std::iter::once(tools).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap()))).unwrap()},
        "entrypoint":{"tool":"owner-probe","script":".chrono-harness/ci/bootstrap.py"},
        "participation":{"coordinator_root":"git-main-worktree","program":".chrono-harness/bin/chrono-worktree","git":fixture_git(),"config":POLICY},
        "owner_stage":{"source":".chrono-harness/bin/built-owner","selected":".chrono-harness/bin/chrono-worktree","consumer_scope":scope}});
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
    let mut c = h.auto_command(
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
    let (h, exe, path, sha) = fixture("registered-enrollment-only");
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
#[test]
fn owner_deployment_refuses_unqualified_consumer_handoff_and_live_inherited_child() {
    let (h, exe, path, sha) = fixture("unqualified");
    let before = fs::read(h.root.join(".chrono-harness/bin/chrono-worktree")).unwrap();
    let (code, report, error) = deploy(&h, &exe, &path, &sha);
    assert_ne!(code, 0, "{report} {error}");
    assert!(
        report
            .to_string()
            .contains("unmanaged consumer handoff is unqualified"),
        "{report}"
    );
    assert_eq!(
        fs::read(h.root.join(".chrono-harness/bin/chrono-worktree")).unwrap(),
        before
    );
    let entry = &h.ledger()["entries"][0];
    let lease_path = h.root.join(entry["ownership"]["path"].as_str().unwrap());
    let lease = chrono_harness::ownership::Lease::acquire(
        &lease_path,
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
        .arg("--hold")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
    let mut child = command.spawn().unwrap();
    let mut ready = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut ready)
        .unwrap();
    assert_eq!(ready, "ready\n");
    drop(lease);
    let (code, report, error) = deploy(&h, &exe, &path, &sha);
    assert_ne!(code, 0, "{report} {error}");
    assert!(
        report.to_string().contains("live inherited consumers"),
        "{report}"
    );
    child.stdin.take().unwrap().write_all(b"x").unwrap();
    assert!(child.wait().unwrap().success());
    assert_eq!(
        fs::read(h.root.join(".chrono-harness/bin/chrono-worktree")).unwrap(),
        before
    );
    assert!(
        !h.root
            .join(".chrono-harness/state/bootstrap-owner-current.json")
            .exists()
    );
    assert!(h.root.join(LEDGER).exists());
}
#[test]
fn owner_deployment_keeps_publication_evidence_when_original_admission_drifts() {
    let (h, exe, path, sha) = fixture("registered-enrollment-only");
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
