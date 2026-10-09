#[path = "support/tools.rs"]
mod tools;
use tools::fixture_git;
#[path = "units.rs"]
mod units;
use chrono_ci::{Config, generate, init, prepare};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::process::Command;
mod retained_receipts;
fn config() -> Config {
    serde_json::from_value(json!({"schema":"chrono-github-ci/v1","workflow_path":".github/workflows/check.yml","name":"Host checks","runs_on":"macos-14","push_branches":["dev","integration/**"],"pull_request_branches":["dev"],"branch_creation_base_ref":"refs/heads/dev","checkout_action":"actions/checkout@11d5960a326750d5838078e36cf38b85af677262","upload_artifact_action":"actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02","timeout_minutes":20,"bootstrap":["python3",".chrono-harness/ci/bootstrap.py","."],"runner":".chrono-harness/bin/chrono-harness","generator":".chrono-harness/bin/chrono-ci","check_config":".chrono-harness/ci/check.json","context_path":".chrono-harness/state/context.json","artifact_directory":".chrono-harness/state/"})).unwrap()
}
fn write(p: &Path, c: &Config) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, serde_json::to_vec_pretty(c).unwrap()).unwrap();
}
fn git(p: &Path, args: &[&str]) -> String {
    let o = Command::new(fixture_git())
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
        !Command::new(fixture_git())
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
    // A copied-host fixture owns its receipt policy too. The source checkout's
    // deployed Git binding is host policy, not a native test-platform input.
    let owner = retained_receipts::Owner::new();
    let argv: Vec<String> = serde_json::from_value(context["canonical_argv"].clone()).unwrap();
    assert_eq!(argv[1], "check");
    let output = Command::new(root.join(&argv[0]))
        .args(&argv[1..])
        .current_dir(root)
        .output()
        .unwrap();
    let receipt = retain_expected_command_result(
        &owner.root,
        root,
        &argv,
        context,
        &output,
        Some(expected_exit),
    );
    assert_eq!(
        output.status.code(),
        Some(expected_exit),
        "root {}; argv {argv:?}; status {}; stdout {}; stderr {}",
        root.display(),
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(report.get("transport_failure").is_none(), "{report}");
    if let Some(receipt) = receipt {
        receipt
            .release(
                "command-assertion",
                "expected exit and original response consumed",
            )
            .unwrap();
    }
    report
}
fn retain_command_result(
    root: &Path,
    argv: &[String],
    context: &Value,
    output: &std::process::Output,
) {
    // Existing callers retain an unreleased diagnostic reference. They do not
    // acquire disposal eligibility from another caller's expected-exit contract.
    let _ = retain_expected_command_result(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        root,
        argv,
        context,
        output,
        None,
    );
}

fn retain_expected_command_result(
    producer_root: &Path,
    root: &Path,
    argv: &[String],
    context: &Value,
    output: &std::process::Output,
    expected_exit: Option<i32>,
) -> Option<chrono_worktree::RetainedArtifact> {
    let directory = std::env::var_os("CHRONO_CI_TEST_RECEIPTS").map(std::path::PathBuf::from);
    if directory.is_none() && output.status.success() {
        return None;
    }
    Some(retain_command_result_in(
        producer_root,
        root,
        argv,
        context,
        output,
        expected_exit,
        directory.as_deref(),
    ))
}

fn retain_command_result_in(
    producer_root: &Path,
    root: &Path,
    argv: &[String],
    context: &Value,
    output: &std::process::Output,
    expected_exit: Option<i32>,
    directory_override: Option<&Path>,
) -> chrono_worktree::RetainedArtifact {
    retain_command_result_observed(
        producer_root,
        root,
        argv,
        context,
        output,
        expected_exit,
        directory_override,
        None,
    )
}
fn retain_command_result_observed(
    producer_root: &Path,
    root: &Path,
    argv: &[String],
    context: &Value,
    output: &std::process::Output,
    expected_exit: Option<i32>,
    directory_override: Option<&Path>,
    birth_observer: Option<&mut dyn FnMut(&str)>,
) -> chrono_worktree::RetainedArtifact {
    let (directory, generated_outputs) = chrono_worktree::RetainedArtifact::registered_store(
        producer_root,
        ".chrono-harness/worktree.json",
        "ci-command-result",
    )
    .unwrap_or_else(|error| {
        panic!("receipt owner refused publication: {error}; original command: {output:?}")
    });
    if let Some(requested) = directory_override {
        assert_eq!(
            requested, directory,
            "receipt destination is not the declared owner store; preserve unknown destination and original command: {output:?}"
        );
    }
    fs::create_dir_all(&directory).unwrap();
    let destination = tempfile::Builder::new()
        .prefix("context-")
        .tempdir_in(&directory)
        .unwrap()
        .keep();
    let config: Value = fs::read(root.join(".chrono-harness/config.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or(Value::Null);
    let generated: Vec<String> = generated_outputs
        .iter()
        .filter_map(|p| {
            let path = p.as_str();
            let original = format!(
                ".chrono-harness/state/{}",
                path.strip_prefix("original-state/")?
            );
            config["artifacts"]
                .as_array()?
                .iter()
                .any(|a| a["path"] == original && a["tracked"] == false)
                .then(|| path.to_owned())
        })
        .collect();
    let reason = if expected_exit.is_none() {
        "unknown-expected-exit; original diagnostic reference has not been released"
    } else if output.status.code() == expected_exit {
        "expected-command-exit; assertion consumer still active"
    } else {
        "unexpected-command-exit; original diagnostics and partial effects required"
    };
    // Keep the command's original evidence even if ownership admission fails.
    // An unregistered copy stays unknown and cannot authorize reclamation.
    fs::write(destination.join("stdout.bin"), &output.stdout).unwrap();
    fs::write(destination.join("stderr.bin"), &output.stderr).unwrap();
    fs::write(destination.join("binding.json"), serde_json::to_vec_pretty(&json!({"root":root,"argv":argv,"context":context,"exit":output.status.code(),"expected_exit":expected_exit,"status":output.status.to_string(),"joined":true,"retention_reason":reason})).unwrap()).unwrap();
    retain_context_state(
        &root.join(".chrono-harness/state"),
        &destination.join("original-state"),
    );
    // Retain each command's actual configuration beside its original state.
    retain_context_state(
        &root.join(".chrono-harness/ci"),
        &destination.join("configuration/ci"),
    );
    fs::create_dir_all(destination.join("configuration")).unwrap();
    for name in [
        "config.json",
        "FILEMAP.json",
        "projects.json",
        "judges.json",
        "workflow.json",
    ] {
        let path = root.join(".chrono-harness").join(name);
        if path.is_file() {
            fs::copy(path, destination.join("configuration").join(name)).unwrap();
        }
    }
    if !output.status.success() {
        eprintln!("original CI fixture evidence: {}", destination.display());
    }
    let receipt = match birth_observer {
        Some(observer) => chrono_worktree::RetainedArtifact::begin_with_birth_observer(
            &directory,
            "ci-command-result",
            &destination,
            &generated,
            "command-assertion",
            reason,
            observer,
        ),
        None => chrono_worktree::RetainedArtifact::begin(
            &directory,
            "ci-command-result",
            &destination,
            &generated,
            "command-assertion",
            reason,
        ),
    }
    .unwrap_or_else(|error| {
        panic!(
            "receipt owner refused admission: {error}; original command evidence: {}",
            destination.display()
        )
    });
    receipt.seal().unwrap();
    receipt
}
fn retain_context_state(root: &Path, destination: &Path) {
    assert!(
        !destination.starts_with(root),
        "retained source/destination overlap; preserve original partial receipt"
    );
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
