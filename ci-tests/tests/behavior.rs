use chrono_ci::{Config, generate, init, prepare};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::process::Command;
fn config() -> Config {
    serde_json::from_value(json!({"schema":"chrono-github-ci/v1","workflow_path":".github/workflows/check.yml","name":"Host checks","runs_on":"macos-14","push_branches":["dev","integration/**"],"pull_request_branches":["dev"],"branch_creation_base_ref":"refs/heads/dev","checkout_action":"actions/checkout@11d5960a326750d5838078e36cf38b85af677262","upload_artifact_action":"actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02","timeout_minutes":20,"bootstrap":["python3",".chrono-harness/ci/bootstrap.py","."],"runner":".chrono-harness/bin/chrono-harness","generator":".chrono-harness/bin/chrono-ci","check_config":".chrono-harness/ci/check.json","context_path":".chrono-harness/state/context.json","artifact_directory":".chrono-harness/state/"})).unwrap()
}
fn write(p: &Path, c: &Config) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, serde_json::to_vec_pretty(c).unwrap()).unwrap();
}
fn git(p: &Path, args: &[&str]) -> String {
    let o = Command::new("git")
        .args(args)
        .current_dir(p)
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8(o.stdout).unwrap().trim().to_string()
}
fn repo() -> (tempfile::TempDir, String, String) {
    let d = tempfile::tempdir().unwrap();
    git(d.path(), &["init", "-q", "-b", "dev"]);
    git(
        d.path(),
        &["config", "user.email", "fixture@example.invalid"],
    );
    git(d.path(), &["config", "user.name", "Fixture"]);
    fs::write(d.path().join("a"), "a").unwrap();
    git(d.path(), &["add", "a"]);
    git(d.path(), &["commit", "-qm", "base"]);
    let b = git(d.path(), &["rev-parse", "HEAD"]);
    fs::write(d.path().join("a"), "b").unwrap();
    git(d.path(), &["commit", "-qam", "candidate"]);
    let c = git(d.path(), &["rev-parse", "HEAD"]);
    (d, b, c)
}
#[test]
fn init_from_external_config_and_spaced_host_is_idempotent() {
    let d = tempfile::Builder::new()
        .prefix("spaced host ")
        .tempdir()
        .unwrap();
    let input = tempfile::tempdir().unwrap();
    let p = input.path().join("settings.json");
    write(&p, &config());
    assert!(init(d.path(), &p).unwrap());
    let out = d.path().join(".github/workflows/check.yml");
    let before = fs::read(&out).unwrap();
    let mtime = fs::metadata(&out).unwrap().modified().unwrap();
    assert!(!init(d.path(), &p).unwrap());
    assert_eq!(fs::read(&out).unwrap(), before);
    assert_eq!(fs::metadata(&out).unwrap().modified().unwrap(), mtime);
    assert!(!generate(d.path(), ".chrono-harness/ci/github.json", true).unwrap());
}
#[test]
fn init_preserves_adopted_customization() {
    let d = tempfile::tempdir().unwrap();
    let input = d.path().join("source.json");
    write(&input, &config());
    init(d.path(), &input).unwrap();
    let mut custom = config();
    custom.name = "Custom name".into();
    custom.bootstrap.push("extra".into());
    write(&d.path().join(".chrono-harness/ci/github.json"), &custom);
    init(d.path(), &input).unwrap();
    assert_eq!(
        chrono_ci::load(&d.path().join(".chrono-harness/ci/github.json"))
            .unwrap()
            .name,
        "Custom name"
    );
}
#[test]
fn drift_verification_never_repairs() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("source.json");
    write(&p, &config());
    init(d.path(), &p).unwrap();
    let out = d.path().join(".github/workflows/check.yml");
    let changed = fs::read_to_string(&out).unwrap() + "# drift\n";
    fs::write(&out, &changed).unwrap();
    assert!(
        generate(d.path(), ".chrono-harness/ci/github.json", true)
            .unwrap_err()
            .contains("drift")
    );
    assert_eq!(fs::read_to_string(out).unwrap(), changed);
}
#[test]
fn unowned_collision_preflight_preserves_all_inputs() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("source.json");
    write(&p, &config());
    let out = d.path().join(".github/workflows/check.yml");
    fs::create_dir_all(out.parent().unwrap()).unwrap();
    fs::write(&out, "host-owned\n").unwrap();
    assert!(init(d.path(), &p).is_err());
    assert_eq!(fs::read_to_string(out).unwrap(), "host-owned\n");
    assert!(!d.path().join(".chrono-harness/ci/github.json").exists());
}
#[test]
fn unrelated_workflows_and_symlink_collisions() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("source.json");
    write(&p, &config());
    let wf = d.path().join(".github/workflows");
    fs::create_dir_all(&wf).unwrap();
    fs::write(wf.join("unrelated.yml"), "keep").unwrap();
    init(d.path(), &p).unwrap();
    assert_eq!(
        fs::read_to_string(wf.join("unrelated.yml")).unwrap(),
        "keep"
    );
    fs::remove_file(wf.join("check.yml")).unwrap();
    std::os::unix::fs::symlink("unrelated.yml", wf.join("check.yml")).unwrap();
    assert!(generate(d.path(), ".chrono-harness/ci/github.json", false).is_err());
    assert_eq!(
        fs::read_to_string(wf.join("unrelated.yml")).unwrap(),
        "keep"
    );
}
#[test]
fn bootstrap_extension_regenerates_owned_projection() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("source.json");
    let c = config();
    write(&p, &c);
    init(d.path(), &p).unwrap();
    let old = fs::read(d.path().join(&c.workflow_path)).unwrap();
    let mut changed = c.clone();
    changed.bootstrap = vec![
        "python3".into(),
        ".chrono-harness/ci/new bootstrap.py".into(),
        "literal $HOME `cmd`".into(),
    ];
    write(&d.path().join(".chrono-harness/ci/github.json"), &changed);
    assert!(generate(d.path(), ".chrono-harness/ci/github.json", true).is_err());
    assert!(generate(d.path(), ".chrono-harness/ci/github.json", false).unwrap());
    assert_ne!(fs::read(d.path().join(&c.workflow_path)).unwrap(), old);
    assert!(!generate(d.path(), ".chrono-harness/ci/github.json", true).unwrap());
}
#[test]
fn invalid_provider_unknown_member_and_unpinned_action_fail() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("source.json");
    for mutation in ["provider", "member", "action"] {
        let mut v = serde_json::to_value(config()).unwrap();
        match mutation {
            "provider" => v["schema"] = "other/v1".into(),
            "member" => v["guess_tests"] = true.into(),
            _ => v["checkout_action"] = "actions/checkout@main".into(),
        };
        fs::write(&p, serde_json::to_vec(&v).unwrap()).unwrap();
        assert!(init(d.path(), &p).is_err());
    }
}
#[test]
fn push_uses_actual_before_after() {
    let (d, b, c) = repo();
    let r = prepare(
        d.path(),
        &config(),
        "push",
        &json!({"before":b,"after":c,"created":false,"deleted":false}),
        &c,
    )
    .unwrap();
    assert_eq!(r["base"], b);
    assert_eq!(r["candidate"], c);
    assert_eq!(r["source"], "push-before-after");
}
#[test]
fn pr_uses_head_not_merge_ref() {
    let (d, b, c) = repo();
    let p = json!({"pull_request":{"base":{"sha":b},"head":{"sha":c}},"after":"ignored-merge"});
    let r = prepare(d.path(), &config(), "pull_request", &p, &b).unwrap();
    assert_eq!(r["candidate"], c);
    assert_eq!(r["workflow_source_revision"], b);
    assert_eq!(r["base"], b);
}
#[test]
fn manual_full_range_and_explicit_initial() {
    let (d, b, c) = repo();
    let p = json!({"inputs":{"base":b,"candidate":c,"initial":false}});
    assert_eq!(
        prepare(d.path(), &config(), "workflow_dispatch", &p, &c).unwrap()["initial"],
        false
    );
    let p = json!({"inputs":{"candidate":c,"initial":true}});
    assert!(
        prepare(d.path(), &config(), "workflow_dispatch", &p, &c)
            .unwrap_err()
            .contains("parents")
    );
    git(d.path(), &["checkout", "--detach", &b]);
    let p = json!({"inputs":{"candidate":b,"initial":"true"}});
    assert_eq!(
        prepare(d.path(), &config(), "workflow_dispatch", &p, &c).unwrap()["initial"],
        true
    );
}
#[test]
fn deletion_zero_missing_and_wrong_checkout_fail() {
    let (d, b, c) = repo();
    for p in [
        json!({"before":b,"after":c,"deleted":true}),
        json!({"before":"0".repeat(40),"after":c,"created":false}),
        json!({"before":b,"after":b}),
        json!({"before":"HEAD","after":c}),
    ] {
        assert!(prepare(d.path(), &config(), "push", &p, &c).is_err());
    }
}
#[test]
fn branch_creation_resolves_configured_baseline_once() {
    let (d, b, c) = repo();
    let remote = tempfile::tempdir().unwrap();
    git(remote.path(), &["init", "--bare", "-q"]);
    git(
        d.path(),
        &["remote", "add", "origin", remote.path().to_str().unwrap()],
    );
    git(
        d.path(),
        &["push", "-q", "origin", &format!("{b}:refs/heads/dev")],
    );
    let r = prepare(
        d.path(),
        &config(),
        "push",
        &json!({"before":"0".repeat(40),"after":c,"created":true}),
        &c,
    )
    .unwrap();
    assert_eq!(r["base"], b);
    assert_eq!(r["source"], "branch-creation-baseline-ref");
    git(
        d.path(),
        &["push", "-q", "origin", &format!("{c}:refs/heads/dev")],
    );
    assert_eq!(r["base"], b);
}
#[test]
fn event_context_contains_same_canonical_entry() {
    let (d, b, c) = repo();
    let r = prepare(
        d.path(),
        &config(),
        "push",
        &json!({"before":b,"after":c}),
        &c,
    )
    .unwrap();
    let argv: Vec<String> = serde_json::from_value(r["canonical_argv"].clone()).unwrap();
    assert_eq!(
        argv,
        chrono_harness::canonical_argv(
            ".chrono-harness/bin/chrono-harness",
            ".chrono-harness/ci/check.json",
            Some(&b),
            &c,
            false
        )
    );
}
#[test]
fn cli_preserves_context_and_outputs_full_identity() {
    let (d, b, c) = repo();
    let cfg = d.path().join(".chrono-harness/ci/github.json");
    write(&cfg, &config());
    let payload = d.path().join("payload.json");
    fs::write(
        &payload,
        serde_json::to_vec(&json!({"before":b,"after":c})).unwrap(),
    )
    .unwrap();
    let output = d.path().join("github-output");
    fs::write(&output, "").unwrap();
    let a = vec![
        "prepare",
        "--host-root",
        d.path().to_str().unwrap(),
        "--config",
        ".chrono-harness/ci/github.json",
        "--event",
        "push",
        "--payload",
        payload.to_str().unwrap(),
        "--workflow-revision",
        &b,
        "--github-output",
        output.to_str().unwrap(),
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<Vec<_>>();
    let r: Value = serde_json::from_str(&chrono_ci::dispatch(&a).unwrap()).unwrap();
    assert_eq!(r["workflow_source_revision"], b);
    assert!(
        fs::read_to_string(output)
            .unwrap()
            .contains(&format!("candidate={c}"))
    );
    assert!(d.path().join(".chrono-harness/state/context.json").exists());
}
#[test]
fn example_bundle_adopts_without_source_checkout_dependency() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/ci-host");
    let root = tempfile::Builder::new()
        .prefix("copied example host ")
        .tempdir()
        .unwrap();
    let map: Value =
        serde_json::from_slice(&fs::read(source.join(".chrono-harness/FILEMAP.json")).unwrap())
            .unwrap();
    for row in map["files"].as_array().unwrap() {
        let path = row["path"].as_str().unwrap();
        if path == ".github/workflows/chrono-ci.yml" {
            continue;
        }
        let out = root.path().join(path);
        fs::create_dir_all(out.parent().unwrap()).unwrap();
        fs::copy(source.join(path), out).unwrap();
    }
    init(
        root.path(),
        &root.path().join(".chrono-harness/ci/github.json"),
    )
    .unwrap();
    assert!(!generate(root.path(), ".chrono-harness/ci/github.json", true).unwrap());
    let config = chrono_ci::load(&root.path().join(".chrono-harness/ci/github.json")).unwrap();
    assert_eq!(config.runs_on, "self-hosted");
}
#[test]
fn host_bootstrap_consumes_registered_operations_and_propagates_failure() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.chrono-harness/ci/bootstrap.py");
    let root = tempfile::Builder::new()
        .prefix("bootstrap host ")
        .tempdir()
        .unwrap();
    let dir = root.path().join(".chrono-harness/ci");
    fs::create_dir_all(&dir).unwrap();
    fs::copy(source, dir.join("bootstrap.py")).unwrap();
    let config = json!({"schema":"chrono-bootstrap/v1","projects":".chrono-harness/projects.json","rust_toolchain":"1.95.0","tools":{"sh":"/bin/sh"},"operations":["make.tool"],"install":[{"source":"built-tool","destination":".chrono-harness/bin/tool"}]});
    fs::write(
        dir.join("bootstrap.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    let registry = |command: &str| json!({"projects":[{"actions":{"build":{"operation":"make.tool","tool":"sh","argv":["-c",command]}}}],"scripts":[]});
    let pr = root.path().join(".chrono-harness/projects.json");
    fs::write(
        &pr,
        serde_json::to_vec(&registry("printf binary > built-tool")).unwrap(),
    )
    .unwrap();
    let invoke = || {
        Command::new("python3")
            .arg(dir.join("bootstrap.py"))
            .arg(root.path())
            .current_dir(std::env::temp_dir())
            .output()
            .unwrap()
    };
    let success = invoke();
    assert!(
        success.status.success(),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    assert_eq!(
        fs::read(root.path().join(".chrono-harness/bin/tool")).unwrap(),
        b"binary"
    );
    let state: Value = serde_json::from_slice(
        &fs::read(root.path().join(".chrono-harness/state/bootstrap.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        state["installed"][0]["sha256"],
        chrono_harness::sha256(b"binary")
    );
    fs::write(&pr, serde_json::to_vec(&registry("exit 9")).unwrap()).unwrap();
    assert!(!invoke().status.success());
}
