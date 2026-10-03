#[path = "units.rs"]
mod units;
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
fn configured_push_baseline_covers_repeated_pushes_and_tracks_the_named_remote_ref() {
    let (d, base, first) = repo();
    let remote = tempfile::tempdir().unwrap();
    git(remote.path(), &["init", "--bare", "-q"]);
    git(
        d.path(),
        &["remote", "add", "origin", remote.path().to_str().unwrap()],
    );
    git(
        d.path(),
        &["push", "-q", "origin", &format!("{base}:refs/heads/stable")],
    );
    let mut raw = serde_json::to_value(config()).unwrap();
    raw["push_baselines"] =
        json!([{"ref_prefix":"refs/heads/verify/","base_ref":"refs/heads/stable"}]);
    let configured: Config = serde_json::from_value(raw).unwrap();
    let first_context = prepare(d.path(), &configured, "push", &json!({"ref":"refs/heads/verify/change","before":"0".repeat(40),"after":first,"created":true}), &first).unwrap();
    assert_eq!(first_context["base"], base);
    assert_eq!(first_context["source"], "push-configured-baseline-ref");
    fs::write(d.path().join("a"), "repair").unwrap();
    git(d.path(), &["commit", "-qam", "repair"]);
    let repaired = git(d.path(), &["rev-parse", "HEAD"]);
    let event =
        json!({"ref":"refs/heads/verify/change","before":first,"after":repaired,"created":false});
    let context = prepare(d.path(), &configured, "push", &event, &repaired).unwrap();
    assert_eq!(context["base"], base);
    assert_eq!(context["candidate"], repaired);
    assert_eq!(
        context["canonical_argv"],
        json!(chrono_harness::canonical_argv(
            &configured.runner,
            &configured.check_config,
            Some(&base),
            &repaired,
            false
        ))
    );
    git(
        d.path(),
        &[
            "push",
            "-q",
            "origin",
            &format!("{first}:refs/heads/stable"),
        ],
    );
    let advanced = prepare(d.path(), &configured, "push", &event, &repaired).unwrap();
    assert_eq!(advanced["base"], first);
    assert_eq!(context["base"], base);
    let ordinary = json!({"ref":"refs/heads/dev","before":first,"after":repaired,"created":false});
    let control = prepare(d.path(), &configured, "push", &ordinary, &repaired).unwrap();
    assert_eq!(control["base"], first);
    assert_eq!(control["source"], "push-before-after");
}

#[test]
fn configured_push_baseline_rejects_ambiguity_missing_refs_and_invalid_events() {
    let host = tempfile::tempdir().unwrap();
    for rules in [
        json!([{"ref_prefix":"refs/heads/verify/*","base_ref":"refs/heads/stable"}]),
        json!([{"ref_prefix":"refs/heads/verify/","base_ref":"stable"}]),
        json!([{"ref_prefix":"refs/heads/verify/","base_ref":"refs/heads/verify/self"}]),
        json!([{"ref_prefix":"refs/heads/verify/","base_ref":"refs/heads/stable"},{"ref_prefix":"refs/heads/verify/nested/","base_ref":"refs/heads/stable"}]),
        json!([{"ref_prefix":"refs/heads/verify/","base_ref":"refs/heads/stable"},{"ref_prefix":"refs/heads/verify/","base_ref":"refs/heads/other"}]),
    ] {
        let mut raw = serde_json::to_value(config()).unwrap();
        raw["push_baselines"] = rules;
        let source = host.path().join("input.json");
        fs::write(&source, serde_json::to_vec(&raw).unwrap()).unwrap();
        assert!(init(host.path(), &source).is_err());
        assert!(!host.path().join(".github/workflows/check.yml").exists());
    }
    let (d, base, candidate) = repo();
    let remote = tempfile::tempdir().unwrap();
    git(remote.path(), &["init", "--bare", "-q"]);
    git(
        d.path(),
        &["remote", "add", "origin", remote.path().to_str().unwrap()],
    );
    let mut raw = serde_json::to_value(config()).unwrap();
    raw["push_baselines"] =
        json!([{"ref_prefix":"refs/heads/verify/","base_ref":"refs/heads/missing"}]);
    let configured: Config = serde_json::from_value(raw).unwrap();
    let event = json!({"ref":"refs/heads/verify/change","before":base,"after":candidate});
    assert!(
        prepare(d.path(), &configured, "push", &event, &candidate)
            .unwrap_err()
            .contains("remote baseline ref")
    );
    for event in [
        json!({"before":base,"after":candidate}),
        json!({"ref":"refs/heads/verify/bad..ref","before":base,"after":candidate}),
        json!({"ref":"refs/heads/verify/change","before":"HEAD","after":candidate}),
        json!({"ref":"refs/heads/verify/change","before":"0".repeat(40),"after":candidate,"created":false}),
        json!({"ref":"refs/heads/verify/change","before":base,"after":"HEAD"}),
    ] {
        assert!(prepare(d.path(), &configured, "push", &event, &candidate).is_err());
    }
}

#[test]
fn configured_push_baseline_cli_fetches_the_pinned_missing_object_and_preserves_customization() {
    let (d, base, candidate) = repo();
    let remote = tempfile::tempdir().unwrap();
    git(remote.path(), &["init", "--bare", "-q"]);
    git(
        d.path(),
        &["remote", "add", "origin", remote.path().to_str().unwrap()],
    );
    git(
        d.path(),
        &["push", "-q", "origin", &format!("{base}:refs/heads/stable")],
    );
    let writer = tempfile::tempdir().unwrap();
    git(
        writer.path(),
        &[
            "clone",
            "-q",
            "--branch",
            "stable",
            remote.path().to_str().unwrap(),
            ".",
        ],
    );
    git(
        writer.path(),
        &["config", "user.email", "fixture@example.invalid"],
    );
    git(writer.path(), &["config", "user.name", "Fixture"]);
    fs::write(writer.path().join("a"), "remote advancement").unwrap();
    git(writer.path(), &["commit", "-qam", "advance baseline"]);
    let advanced = git(writer.path(), &["rev-parse", "HEAD"]);
    git(writer.path(), &["push", "-q", "origin", "stable"]);
    assert!(
        !Command::new("git")
            .current_dir(d.path())
            .args(["cat-file", "-e", &advanced])
            .output()
            .unwrap()
            .status
            .success()
    );
    let mut raw = serde_json::to_value(config()).unwrap();
    raw["push_baselines"] =
        json!([{"ref_prefix":"refs/heads/verify/","base_ref":"refs/heads/stable"}]);
    let configured: Config = serde_json::from_value(raw).unwrap();
    let input = d.path().join("input.json");
    write(&input, &configured);
    init(d.path(), &input).unwrap();
    write(&input, &config());
    assert!(!init(d.path(), &input).unwrap());
    let payload = d.path().join("event.json");
    fs::write(
        &payload,
        serde_json::to_vec(
            &json!({"ref":"refs/heads/verify/change","before":base,"after":candidate}),
        )
        .unwrap(),
    )
    .unwrap();
    let output = d.path().join("outputs");
    fs::write(&output, "").unwrap();
    let args: Vec<String> = [
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
        &candidate,
        "--github-output",
        output.to_str().unwrap(),
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    let context: Value = serde_json::from_str(&chrono_ci::dispatch(&args).unwrap()).unwrap();
    assert_eq!(context["base"], advanced);
    assert_eq!(context["candidate"], candidate);
    assert_eq!(context["source"], "push-configured-baseline-ref");
    assert_eq!(
        git(d.path(), &["rev-parse", &format!("{advanced}^{{commit}}")]),
        advanced
    );
    assert!(
        fs::read_to_string(output)
            .unwrap()
            .contains(&format!("base={advanced}\n"))
    );
    let saved: Value =
        serde_json::from_slice(&fs::read(d.path().join(&configured.context_path)).unwrap())
            .unwrap();
    assert_eq!(saved, context);
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
fn initial_event_reads_original_parents_through_git_replace_overlays() {
    let (d, root, child) = repo();
    git(d.path(), &["replace", &child, &root]);
    let payload = json!({"inputs":{"candidate":child,"initial":true}});
    assert!(
        prepare(d.path(), &config(), "workflow_dispatch", &payload, &child)
            .unwrap_err()
            .contains("parents")
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
        &json!({"ref":"refs/heads/integration/new","before":"0".repeat(40),"after":c,"created":true}),
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
fn copied_example() -> tempfile::TempDir {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/ci-host");
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
    root
}
#[test]
fn example_bundle_adopts_without_source_checkout_dependency() {
    let root = copied_example();
    assert!(!generate(root.path(), ".chrono-harness/ci/github.json", true).unwrap());
    let config = chrono_ci::load(&root.path().join(".chrono-harness/ci/github.json")).unwrap();
    assert_eq!(config.runs_on, "self-hosted");
}

fn committed_example(message: &str) -> (tempfile::TempDir, tempfile::TempDir, String) {
    let root = copied_example();
    let installed = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.chrono-harness/bin");
    let bin = root.path().join(".chrono-harness/bin");
    fs::create_dir_all(&bin).unwrap();
    for name in ["chrono-harness", "chrono-judge-ci", "chrono-ci"] {
        fs::copy(installed.join(name), bin.join(name)).expect(
            "bootstrap the registered candidate tools before running the copied-host tests",
        );
    }
    git(root.path(), &["init", "-q", "-b", "dev"]);
    git(
        root.path(),
        &["config", "user.email", "fixture@example.invalid"],
    );
    git(root.path(), &["config", "user.name", "Fixture"]);
    fs::write(root.path().join("message.txt"), message).unwrap();
    git(root.path(), &["add", "."]);
    git(root.path(), &["commit", "-qm", "initial host"]);
    let candidate = git(root.path(), &["rev-parse", "HEAD"]);
    let remote = tempfile::tempdir().unwrap();
    git(remote.path(), &["init", "--bare", "-q"]);
    git(
        root.path(),
        &["remote", "add", "origin", remote.path().to_str().unwrap()],
    );
    git(root.path(), &["push", "-q", "origin", "dev"]);
    (root, remote, candidate)
}
fn example_push(root: &Path, candidate: &str, branch: &str) -> Result<Value, String> {
    let c = chrono_ci::load(&root.join(".chrono-harness/ci/github.json")).unwrap();
    prepare(
        root,
        &c,
        "push",
        &json!({"ref":branch,"before":"0".repeat(40),"after":candidate,"created":true,"deleted":false}),
        candidate,
    )
}
fn execute_context(root: &Path, context: &Value, expected_exit: i32) -> Value {
    let argv: Vec<String> = serde_json::from_value(context["canonical_argv"].clone()).unwrap();
    assert_eq!(argv[1], "check");
    let output = Command::new(root.join(&argv[0]))
        .args(&argv[1..])
        .current_dir(root)
        .output()
        .unwrap();
    if let Ok(directory) = std::env::var("CHRONO_CI_TEST_RECEIPTS") {
        static NUMBER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let id = NUMBER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let destination = Path::new(&directory).join(format!(
            "context-{}-{}-{id}",
            std::process::id(),
            chrono_harness::sha256(root.as_os_str().as_encoded_bytes())
        ));
        fs::create_dir_all(&destination).unwrap();
        fs::write(destination.join("stdout.bin"), &output.stdout).unwrap();
        fs::write(destination.join("stderr.bin"), &output.stderr).unwrap();
        fs::write(destination.join("binding.json"), serde_json::to_vec_pretty(&json!({"root":root,"argv":argv,"context":context,"exit":output.status.code(),"joined":true})).unwrap()).unwrap();
        retain_context_state(
            &root.join(".chrono-harness/state"),
            &destination.join("original-state"),
        );
    }
    assert_eq!(
        output.status.code(),
        Some(expected_exit),
        "root {}; argv {argv:?}; stdout {}; stderr {}",
        root.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(report.get("transport_failure").is_none(), "{report}");
    report
}
fn retain_context_state(root: &Path, destination: &Path) {
    if !root.is_dir() {
        return;
    }
    fs::create_dir_all(destination).unwrap();
    for row in fs::read_dir(root).unwrap() {
        let row = row.unwrap();
        let kind = row.file_type().unwrap();
        if kind.is_dir() {
            retain_context_state(&row.path(), &destination.join(row.file_name()));
        } else if kind.is_file() {
            fs::copy(row.path(), destination.join(row.file_name())).unwrap();
        }
    }
}
#[test]
fn first_baseline_push_runs_copied_example_inventory_with_failure_and_passing_control() {
    for (message, exit) in [("wrong\n", 1), ("hello\n", 0)] {
        let (root, _remote, candidate) = committed_example(message);
        let context = example_push(root.path(), &candidate, "refs/heads/dev").unwrap();
        let report = execute_context(root.path(), &context, exit);
        assert_eq!(context["source"], "baseline-creation-initial-inventory");
        assert_eq!(context["mode"], "initial-inventory");
        assert_eq!(context["initial"], true);
        assert!(context["base"].is_null());
        assert_eq!(report["response"]["evidence"]["mode"], "initial-inventory");
        let result = report["response"]["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == "example.check")
            .unwrap();
        assert_eq!(result["exit_code"], exit);
        assert_eq!(
            result["status"],
            if exit == 0 { "passed" } else { "failed" }
        );
        assert_eq!(
            report["response"]["evidence"]["executed"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }
}
#[test]
fn integration_creation_at_existing_baseline_is_an_honest_noop() {
    let (root, _remote, candidate) = committed_example("hello\n");
    let context = example_push(root.path(), &candidate, "refs/heads/integration/new").unwrap();
    assert_eq!(context["base"], candidate);
    assert_eq!(context["initial"], false);
    assert_eq!(context["mode"], "delta");
    assert_eq!(context["source"], "push-configured-baseline-ref");
    let report = execute_context(root.path(), &context, 0);
    assert_eq!(report["response"]["evidence"]["selected"], json!([]));
    assert_eq!(report["response"]["evidence"]["executed"], json!([]));
}
#[test]
fn baseline_creation_with_parents_requires_explicit_range() {
    let (d, _b, c) = repo();
    let payload = json!({"ref":"refs/heads/dev","before":"0".repeat(40),"after":c,"created":true});
    let error = prepare(d.path(), &config(), "push", &payload, &c).unwrap_err();
    assert!(error.contains("explicit range"), "{error}");
    // Git traversal hides parents at a shallow boundary; the object still has them.
    fs::write(d.path().join(".git/shallow"), format!("{c}\n")).unwrap();
    assert_eq!(git(d.path(), &["rev-list", "--parents", "-n", "1", &c]), c);
    let error = prepare(d.path(), &config(), "push", &payload, &c).unwrap_err();
    assert!(error.contains("explicit range"), "{error}");
}
#[test]
fn creation_requires_valid_event_branch_and_available_baseline() {
    let (d, _b, c) = repo();
    for branch in [
        Value::Null,
        json!("refs/tags/dev"),
        json!("refs/heads/bad..name"),
    ] {
        let payload = json!({"ref":branch,"before":"0".repeat(40),"after":c,"created":true});
        let error = prepare(d.path(), &config(), "push", &payload, &c).unwrap_err();
        assert!(
            error.contains("branch-creation event requires") || error.contains("check-ref-format"),
            "{error}"
        );
    }
    let payload = json!({"ref":"refs/heads/integration/new","before":"0".repeat(40),"after":c,"created":true});
    let error = prepare(d.path(), &config(), "push", &payload, &c).unwrap_err();
    assert!(error.contains("ls-remote"), "{error}");
}
#[test]
fn host_bootstrap_consumes_registered_operations_and_propagates_failure() {
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.chrono-harness/ci/bootstrap.py");
    let root = tempfile::Builder::new()
        .prefix("bootstrap host ")
        .tempdir()
        .unwrap();
    let dir = root.path().join(".chrono-harness/ci");
    fs::create_dir_all(&dir).unwrap();
    fs::copy(&source, dir.join("bootstrap.py")).unwrap();
    fs::copy(
        source
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("process_fds.py"),
        dir.parent().unwrap().join("process_fds.py"),
    )
    .unwrap();
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
    // Model an already installed minimal toolchain without touching real rustup state.
    let tools = root.path().join("mock-tools");
    fs::create_dir(&tools).unwrap();
    let python = Command::new("python3")
        .args(["-c", "import sys; print(sys.executable)"])
        .output()
        .unwrap();
    assert!(python.status.success());
    let interpreter = String::from_utf8(python.stdout).unwrap();
    let mock = format!(
        "#!{}\n{}",
        interpreter.trim(),
        r#"
import json, os, pathlib, sys
root = pathlib.Path(__file__).parent
name, args = pathlib.Path(sys.argv[0]).name, sys.argv[1:]
record = root / 'calls.json'
calls = json.loads(record.read_text()) if record.exists() else []
calls.append([name, *args])
record.write_text(json.dumps(calls))
if name == 'rustup':
    if args == ['run', '1.95.0', 'cargo', '--version']:
        sys.exit(0 if (root / 'cargo-ready').exists() else 1)
    if args == ['run', '1.95.0', 'rustfmt', '--version']:
        sys.exit(0 if (root / 'fmt-ready').exists() else 1)
    if args == ['toolchain', 'install', '1.95.0', '--profile', 'minimal', '--component', 'rustfmt']:
        (root / 'cargo-ready').touch()
        (root / 'fmt-ready').touch()
    elif args == ['component', 'add', '--toolchain', '1.95.0', 'rustfmt']:
        if (root / 'fail-component').exists():
            sys.exit(8)
        (root / 'fmt-ready').touch()
    else:
        sys.exit(99)
else:
    assert name in ['cargo', 'rustc'] and args == ['--version']
    assert os.environ['RUSTUP_TOOLCHAIN'] == '1.95.0'
    print(name + ' 1.95.0 (fixture)')
"#
    );
    use std::os::unix::fs::PermissionsExt;
    for name in ["rustup", "cargo", "rustc"] {
        let p = tools.join(name);
        fs::write(&p, &mock).unwrap();
        fs::set_permissions(p, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fs::write(tools.join("cargo-ready"), "").unwrap();
    let path = std::env::join_paths(
        std::iter::once(tools.clone())
            .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let invoke_with = |selected: Option<&str>| {
        let mut command = Command::new("python3");
        command.arg(dir.join("bootstrap.py")).arg(root.path());
        if let Some(selected) = selected {
            command.arg(selected);
        }
        command
            .current_dir(std::env::temp_dir())
            .env("PATH", &path)
            .output()
            .unwrap()
    };
    let invoke = || invoke_with(None);
    let success = invoke();
    assert!(
        success.status.success(),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    assert!(
        tools.join("fmt-ready").exists(),
        "minimal installed toolchain still lacks rustfmt"
    );
    let calls = || -> Value {
        serde_json::from_slice(&fs::read(tools.join("calls.json")).unwrap()).unwrap()
    };
    assert!(calls().as_array().unwrap().contains(&json!([
        "rustup",
        "component",
        "add",
        "--toolchain",
        "1.95.0",
        "rustfmt"
    ])));
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
    assert_eq!(state["schema"], "chrono-bootstrap-result/v1");
    assert_eq!(state["versions"]["cargo"], "cargo 1.95.0 (fixture)");
    assert_eq!(state["versions"]["rustc"], "rustc 1.95.0 (fixture)");
    let evidence = || -> Value {
        serde_json::from_slice(
            &fs::read(root.path().join(".chrono-harness/state/bootstrap.json")).unwrap(),
        )
        .unwrap()
    };
    // A host without its own source repository must not inherit an enclosing repo.
    assert_eq!(state["source"]["state"], "unavailable");
    assert!(state["source"]["commit"].is_null());
    assert!(state["source"]["tree"].is_null());
    for phase in ["before", "after"] {
        assert!(state["source"][phase]["commit"].is_null());
        assert!(state["source"][phase]["tree"].is_null());
        assert!(state["source"][phase]["dirty"].is_null());
        assert!(!state["source"][phase]["error"].as_str().unwrap().is_empty());
    }
    // Build/install outputs and mock SDK state are not product source changes.
    fs::write(
        root.path().join(".gitignore"),
        "/built-tool\n/selected-tool\n/.chrono-harness/bin/\n/.chrono-harness/state/\n/mock-tools/\n",
    )
    .unwrap();
    fs::write(root.path().join("product-source"), "source").unwrap();
    git(root.path(), &["init", "-q"]);
    git(
        root.path(),
        &["config", "user.email", "fixture@example.invalid"],
    );
    git(root.path(), &["config", "user.name", "Fixture"]);
    git(root.path(), &["add", "."]);
    git(root.path(), &["commit", "-qm", "bootstrap source"]);
    let commit = git(root.path(), &["rev-parse", "HEAD"]);
    let tree = git(root.path(), &["rev-parse", "HEAD^{tree}"]);
    let clean = invoke();
    assert!(
        clean.status.success(),
        "{}",
        String::from_utf8_lossy(&clean.stderr)
    );
    let observation = json!({"commit":commit,"tree":tree,"dirty":false,"error":null});
    assert_eq!(
        evidence()["source"],
        json!({"state":"clean","commit":commit,"tree":tree,"before":observation,"after":observation})
    );
    // Unstaged, staged and untracked source remain usable without clean attribution.
    for (file, staged) in [
        ("product-source", false),
        ("product-source", true),
        ("untracked-source", false),
    ] {
        fs::write(root.path().join(file), "dirty source").unwrap();
        if staged {
            git(root.path(), &["add", file]);
        }
        let dirty = invoke();
        assert!(
            dirty.status.success(),
            "{}",
            String::from_utf8_lossy(&dirty.stderr)
        );
        let observation = json!({"commit":commit,"tree":tree,"dirty":true,"error":null});
        assert_eq!(
            evidence()["source"],
            json!({"state":"dirty","commit":null,"tree":null,"before":observation,"after":observation})
        );
        if file == "product-source" {
            git(root.path(), &["restore", "--staged", "--worktree", file]);
        } else {
            fs::remove_file(root.path().join(file)).unwrap();
        }
    }
    // Explicit host configuration selects only its own operations and installations.
    // An unrelated registered operation fails if accidentally executed.
    let mut selected = config.clone();
    selected["operations"] = json!(["make.selected"]);
    selected["install"] =
        json!([{"source":"selected-tool","destination":".chrono-harness/bin/selected"}]);
    fs::write(
        dir.join("selected profile.json"),
        serde_json::to_vec(&selected).unwrap(),
    )
    .unwrap();
    let mut declarations = registry("exit 99");
    declarations["projects"][0]["actions"]["selected"] = json!({
        "operation":"make.selected","tool":"sh","argv":["-c","printf selected > selected-tool"]
    });
    fs::write(&pr, serde_json::to_vec(&declarations).unwrap()).unwrap();
    fs::remove_file(root.path().join("built-tool")).unwrap();
    let actual = invoke_with(Some(".chrono-harness/ci/selected profile.json"));
    assert!(
        actual.status.success(),
        "{}",
        String::from_utf8_lossy(&actual.stderr)
    );
    assert!(!root.path().join("built-tool").exists());
    assert_eq!(
        fs::read(root.path().join(".chrono-harness/bin/selected")).unwrap(),
        b"selected"
    );
    assert_eq!(
        fs::read(root.path().join(".chrono-harness/bin/tool")).unwrap(),
        b"binary"
    );
    assert_eq!(state["config"], ".chrono-harness/ci/bootstrap.json");
    let selected_state =
        fs::read(root.path().join(".chrono-harness/state/bootstrap.json")).unwrap();
    let selected_result: Value = serde_json::from_slice(&selected_state).unwrap();
    assert_eq!(
        selected_result["config"],
        ".chrono-harness/ci/selected profile.json"
    );
    assert_eq!(selected_result["installed"].as_array().unwrap().len(), 1);
    assert_eq!(
        selected_result["installed"][0]["sha256"],
        chrono_harness::sha256(b"selected")
    );
    let before_calls = calls();
    for invalid in [
        "/tmp/config.json",
        ".chrono-harness/ci/../outside.json",
        "outside.json",
    ] {
        let rejected = invoke_with(Some(invalid));
        assert!(!rejected.status.success());
        assert!(String::from_utf8_lossy(&rejected.stderr).contains("explicit host CI path"));
        assert_eq!(calls(), before_calls, "invalid config launched a tool");
        assert_eq!(
            fs::read(root.path().join(".chrono-harness/state/bootstrap.json")).unwrap(),
            selected_state
        );
    }
    // A selected operation that moves HEAD cannot bind its result to either commit.
    fs::write(
        &pr,
        serde_json::to_vec(&registry(
            "printf moved > product-source; git add product-source; git commit -qm moved; printf binary > built-tool",
        ))
        .unwrap(),
    )
    .unwrap();
    git(root.path(), &["add", "."]);
    git(root.path(), &["commit", "-qm", "moving operation"]);
    let before_commit = git(root.path(), &["rev-parse", "HEAD"]);
    let before_tree = git(root.path(), &["rev-parse", "HEAD^{tree}"]);
    let moved = invoke();
    assert!(
        moved.status.success(),
        "{}",
        String::from_utf8_lossy(&moved.stderr)
    );
    let after_commit = git(root.path(), &["rev-parse", "HEAD"]);
    let after_tree = git(root.path(), &["rev-parse", "HEAD^{tree}"]);
    assert_ne!(before_commit, after_commit);
    assert_ne!(before_tree, after_tree);
    assert_eq!(
        evidence()["source"],
        json!({"state":"changed","commit":null,"tree":null,
            "before":{"commit":before_commit,"tree":before_tree,"dirty":false,"error":null},
            "after":{"commit":after_commit,"tree":after_tree,"dirty":false,"error":null}})
    );
    // Losing source identity during real work preserves the known initial observation.
    fs::write(
        &pr,
        serde_json::to_vec(&registry("mv .git .saved-git; printf binary > built-tool")).unwrap(),
    )
    .unwrap();
    git(root.path(), &["add", "."]);
    git(root.path(), &["commit", "-qm", "unavailable operation"]);
    let before_commit = git(root.path(), &["rev-parse", "HEAD"]);
    let before_tree = git(root.path(), &["rev-parse", "HEAD^{tree}"]);
    let unavailable = invoke();
    assert!(
        unavailable.status.success(),
        "{}",
        String::from_utf8_lossy(&unavailable.stderr)
    );
    let unavailable_state = evidence();
    assert_eq!(unavailable_state["source"]["state"], "unavailable");
    assert!(unavailable_state["source"]["commit"].is_null());
    assert!(unavailable_state["source"]["tree"].is_null());
    assert_eq!(
        unavailable_state["source"]["before"],
        json!({"commit":before_commit,"tree":before_tree,"dirty":false,"error":null})
    );
    assert!(unavailable_state["source"]["after"]["commit"].is_null());
    assert!(unavailable_state["source"]["after"]["tree"].is_null());
    assert!(unavailable_state["source"]["after"]["dirty"].is_null());
    assert!(unavailable_state["source"]["after"]["error"].is_string());
    fs::rename(root.path().join(".saved-git"), root.path().join(".git")).unwrap();
    let last_success = fs::read(root.path().join(".chrono-harness/state/bootstrap.json")).unwrap();
    fs::write(&pr, serde_json::to_vec(&registry("exit 9")).unwrap()).unwrap();
    let failed = invoke();
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("exit status 9"));
    assert_eq!(
        fs::read(root.path().join(".chrono-harness/state/bootstrap.json")).unwrap(),
        last_success
    );
    fs::write(
        &pr,
        serde_json::to_vec(&registry("printf binary > built-tool")).unwrap(),
    )
    .unwrap();
    fs::remove_file(tools.join("cargo-ready")).unwrap();
    fs::remove_file(tools.join("fmt-ready")).unwrap();
    assert!(invoke().status.success());
    assert!(calls().as_array().unwrap().contains(&json!([
        "rustup",
        "toolchain",
        "install",
        "1.95.0",
        "--profile",
        "minimal",
        "--component",
        "rustfmt"
    ])));
    fs::remove_file(tools.join("fmt-ready")).unwrap();
    fs::write(tools.join("fail-component"), "").unwrap();
    fs::remove_file(root.path().join("built-tool")).unwrap();
    assert!(!invoke().status.success());
    assert!(
        !root.path().join("built-tool").exists(),
        "operations ran despite component failure"
    );
}

#[test]
fn standalone_version_is_actual_producer_and_rejects_extra_arguments() {
    assert_eq!(
        chrono_ci::dispatch(&["--version".into()]).unwrap(),
        "chrono-ci 0.1.0\n"
    );
    assert!(chrono_ci::dispatch(&["--version".into(), "extra".into()]).is_err());
}

#[test]
fn adopted_migration_interpreter_ignores_path_shadow_and_matches_host_version() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let read =
        |p: &str| -> Value { serde_json::from_slice(&fs::read(source.join(p)).unwrap()).unwrap() };
    let config = read(".chrono-harness/config.json");
    let mut tool = config["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == "python3")
        .unwrap()
        .clone();
    // This synthetic host adopts its actual interpreter. The product host's fixed
    // interpreter pin is checked by the separately registered host migration tests.
    let observed = Command::new(tool["program"].as_str().unwrap())
        .args(serde_json::from_value::<Vec<String>>(tool["version_argv"].clone()).unwrap())
        .env_clear()
        .output()
        .unwrap();
    assert!(observed.status.success());
    tool["expected_version"] = json!(String::from_utf8(observed.stdout).unwrap().trim_end());
    let dir = tempfile::tempdir().unwrap();
    let shadow = dir.path().join("python3");
    fs::write(&shadow, "#!/bin/sh\nprintf 'Python shadowed\\n'\nexit 0\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&shadow, fs::Permissions::from_mode(0o755)).unwrap();
    let control = Command::new("python3")
        .env_clear()
        .env("PATH", dir.path())
        .arg("--version")
        .output()
        .unwrap();
    assert!(control.status.success());
    assert_eq!(control.stdout, b"Python shadowed\n");
    let actual = Command::new(tool["program"].as_str().unwrap())
        .args(serde_json::from_value::<Vec<String>>(tool["version_argv"].clone()).unwrap())
        .env_clear()
        .env("PATH", dir.path())
        .output()
        .unwrap();
    assert!(actual.status.success());
    assert_eq!(
        String::from_utf8(actual.stdout).unwrap().trim_end(),
        tool["expected_version"].as_str().unwrap(),
        "registered interpreter must survive unrelated PATH precedence"
    );
    assert_eq!(
        read(".chrono-harness/ci/check.json")["policy"]["tools"]["python3"],
        tool["program"]
    );
    let provider = read(".chrono-harness/ci/units.json");
    assert_eq!(provider["collection"]["bootstrap"][0], tool["program"]);
    for unit in provider["units"].as_object().unwrap().values() {
        assert_eq!(unit["bootstrap"][0], tool["program"]);
    }
}
