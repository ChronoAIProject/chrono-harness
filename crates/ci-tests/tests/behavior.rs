#[path = "../../worktree-tests/tests/support/fixture_coordinator.rs"]
mod chrono_git_boundary;
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

#[test]
#[cfg(unix)]
fn native_cli_rejects_invalid_utf8_classes_without_losing_position_or_bytes() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    let root = tempfile::tempdir().unwrap();
    for bytes in [
        vec![255],
        vec![128],
        vec![240, 159, 146],
        vec![192, 175],
        vec![237, 160, 128],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_chrono-ci-test-cli"))
            .arg("valid-before")
            .arg(OsString::from_vec(bytes.clone()))
            .arg(OsString::from_vec(vec![254]))
            .current_dir(root.path())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let record: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(record["schema"], "chrono-log/v1");
        assert_eq!(record["producer"], "chrono-ci");
        assert_eq!(record["event"], "cli.invalid_argument");
        assert_eq!(record["error"]["error"]["code"], "E_USAGE");
        assert_eq!(record["error"]["error"]["source"], Value::Null);
        assert_eq!(record["error"]["error"]["context"]["argv_index"], 2);
        assert_eq!(record["error"]["error"]["context"]["argument_index"], 1);
        assert_eq!(
            record["error"]["error"]["context"]["argument_bytes"],
            json!(bytes)
        );
    }
}

#[test]
#[cfg(unix)]
fn native_cli_accepts_literal_replacement_and_unicode_as_text() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let root = tempfile::tempdir().unwrap();
    for argument in ["", "�", "目录/é"] {
        let output = Command::new(env!("CARGO_BIN_EXE_chrono-ci-test-cli"))
            .arg(argument)
            .current_dir(root.path())
            .output()
            .unwrap();
        // 合法文本进入原 CI 命令解析，沿用其退出 1 合同。
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(
            std::str::from_utf8(&output.stderr)
                .unwrap()
                .starts_with("E_CI:")
        );
    }
}

fn config() -> Config {
    serde_json::from_value(json!({"schema":"chrono-github-ci/v4","facts_config":".chrono-harness/event-facts.json","workflow_path":".github/workflows/check.yml","name":"Host checks","runs_on":"macos-14","push_branches":["dev","integration/**"],"pull_request_branches":["dev"],"branch_creation_base_ref":"refs/heads/dev","checkout_action":"actions/checkout@11d5960a326750d5838078e36cf38b85af677262","upload_artifact_action":"actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02","timeout_minutes":20,"bootstrap":["/bin/bash",".chrono-harness/ci/bootstrap-native.sh","."],"runner":".chrono-harness/bin/chrono-harness","generator":".chrono-harness/bin/chrono-ci","check_config":".chrono-harness/ci/check.json","context_path":".chrono-harness/state/context.json","artifact_directory":".chrono-harness/state/"})).unwrap()
}
#[test]
fn retired_scoped_providers_are_rejected_before_publishing_outputs() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    for schema in [
        "chrono-github-ci/v1",
        "chrono-github-ci/v2",
        "chrono-github-ci/v3",
    ] {
        let mut source = serde_json::to_value(config()).unwrap();
        source["schema"] = json!(schema);
        source.as_object_mut().unwrap().remove("facts_config");
        source.as_object_mut().unwrap().remove("initial_inventory");
        if schema == "chrono-github-ci/v2" {
            source["initial_inventory"] = json!({
                "path":".chrono-harness/ci/initial.json",
                "profile":{
                    "schema":"chrono-initial-check/v1",
                    "host_config":".chrono-harness/registry.json",
                    "timeout_seconds":30,"stdout_limit_bytes":1048576,
                    "judges":[{"id":"inventory","executable":".chrono-harness/bin/custom-judge",
                        "version":"0.1.0","sha256":"a".repeat(64),
                        "argv":["--protocol","chrono-initial-judge/v1"],
                        "selector":"every-initial","modes":["inventory"],"after":[]}]
                }
            });
        } else if schema == "chrono-github-ci/v3" {
            source["facts_config"] = json!(".chrono-harness/event-facts.json");
        }
        let host = tempfile::tempdir().unwrap();
        let inputs = tempfile::tempdir().unwrap();
        let input = inputs.path().join("provider.json");
        let bytes = serde_json::to_vec_pretty(&source).unwrap();
        fs::write(&input, &bytes).unwrap();
        let result = init(host.path(), &input);
        if let Ok(changed) = result {
            let original_host = host.keep();
            let original_inputs = inputs.keep();
            panic!(
                "retired provider {schema} was accepted; changed={changed}; original host: {}; original inputs: {}",
                original_host.display(),
                original_inputs.display()
            );
        }
        assert!(
            result
                .unwrap_err()
                .contains("unsupported CI provider/schema")
        );
        assert!(!host.path().join(".github").exists());
        assert!(!host.path().join(".chrono-harness/ci/github.json").exists());
        assert_eq!(fs::read(&input).unwrap(), bytes);
    }
}
fn write(p: &Path, c: &Config) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, serde_json::to_vec_pretty(c).unwrap()).unwrap();
}

fn generation_operation(root: &Path) -> std::path::PathBuf {
    let directory = root.join(".chrono-harness/state/ci-generation-operations");
    assert!(
        directory.is_dir(),
        "CI generation returned without a durable operation: {}",
        directory.display()
    );
    let operations = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(operations.len(), 1, "one invocation must own one operation");
    operations.into_iter().next().unwrap()
}

#[test]
fn generation_success_retains_its_intent_and_original_evaluation() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let host = tempfile::tempdir().unwrap();
    let source = ".chrono-harness/ci/github.json";
    let c = config();
    write(&host.path().join(source), &c);
    assert!(generate(host.path(), source, false).unwrap());
    assert!(host.path().join(&c.workflow_path).is_file());

    let operation = generation_operation(host.path());
    let intent: Value =
        serde_json::from_slice(&fs::read(operation.join("intent.json")).unwrap()).unwrap();
    assert_eq!(intent["schema"], "chrono-ci-generation-intent/v1");
    assert_eq!(intent["operation"], "generate");
    assert_eq!(intent["config"], source);
    let result: Value =
        serde_json::from_slice(&fs::read(operation.join("result.json")).unwrap()).unwrap();
    assert_eq!(result["schema"], "chrono-ci-generation-result/v1");
    assert_eq!(result["producer_evaluation"], json!({"changed": true}));
    // A function return is observable here; the enclosing process has not exited.
    assert_eq!(result["original_process_exit"], Value::Null);
}

#[test]
fn generation_failure_retains_the_actual_error_without_claiming_success() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let host = tempfile::tempdir().unwrap();
    let source = ".chrono-harness/ci/github.json";
    let c = config();
    write(&host.path().join(source), &c);
    let target = host.path().join(&c.workflow_path);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(&target, b"unowned original\n").unwrap();
    let error = generate(host.path(), source, false).unwrap_err();
    assert!(error.contains("unowned workflow collision"), "{error}");
    assert_eq!(fs::read(&target).unwrap(), b"unowned original\n");

    let operation = generation_operation(host.path());
    let result: Value =
        serde_json::from_slice(&fs::read(operation.join("result.json")).unwrap()).unwrap();
    assert!(
        result["producer_evaluation"]["error"]
            .as_str()
            .unwrap()
            .contains("unowned workflow collision")
    );
    assert!(result["producer_evaluation"].get("changed").is_none());
    assert_eq!(result["original_process_exit"], Value::Null);
}

#[test]
fn generation_requires_its_adopted_retention_before_workflow_side_effects() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let host = tempfile::tempdir().unwrap();
    let source = ".chrono-harness/ci/github.json";
    let c = config();
    write(&host.path().join(source), &c);
    let policy = json!({
        "schema":"chrono-output-retention/v2",
        "producers":{"unrelated":[".chrono-harness/state/unrelated/"]},
        "keep_seconds":0,"diagnostic_keep_seconds":0,"metadata_keep_seconds":86400,
        "max_related_outputs":512,"max_summaries":32,"max_nodes_per_round":256,
        "max_bytes_per_round":67108864,"max_millis_per_round":25,
        "max_output_bytes":67108864
    });
    fs::write(
        host.path().join(".chrono-harness/retention.json"),
        serde_json::to_vec_pretty(&policy).unwrap(),
    )
    .unwrap();
    let result = generate(host.path(), source, false);
    assert!(
        result
            .unwrap_err()
            .contains("ci-generation-operation requires explicit finite terminal_limits")
    );
    assert!(!host.path().join(&c.workflow_path).exists());
}

#[test]
fn generation_status_checks_the_original_and_never_accepts_modified_result_bytes() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let host = tools::temporary_host("generation-original");
    let source = ".chrono-harness/ci/github.json";
    write(&host.path().join(source), &config());
    assert!(generate(host.path(), source, false).unwrap());
    let operation = generation_operation(host.path());
    let path = operation
        .strip_prefix(host.path())
        .unwrap()
        .to_str()
        .unwrap();
    let report = chrono_ci::generation::status(host.path(), path).unwrap();
    assert_eq!(report["completion"], "confirmed-evaluation");
    assert_eq!(
        report["original_result"]["producer_evaluation"],
        json!({"changed":true})
    );
    let original = fs::read(operation.join("result.json")).unwrap();
    let mut altered: Value = serde_json::from_slice(&original).unwrap();
    altered["producer_evaluation"]["changed"] = json!(false);
    fs::write(
        operation.join("result.json"),
        serde_json::to_vec(&altered).unwrap(),
    )
    .unwrap();
    assert!(chrono_ci::generation::status(host.path(), path).is_err());
    assert!(chrono_ci::generation::resume(host.path(), path).is_err());
    assert!(
        fs::read_to_string(operation.join("result.json"))
            .unwrap()
            .contains("false")
    );
}

#[test]
#[cfg(unix)]
fn interrupted_generation_resumes_only_a_returned_evaluation_and_never_replays_effects() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use std::io::{BufRead, BufReader, Read};
    use std::os::unix::process::ExitStatusExt;
    use std::process::Stdio;
    for point in ["intent", "prepared", "effect", "evaluated", "result"] {
        let host = tools::temporary_host("generation-interruption");
        let source = ".chrono-harness/ci/github.json";
        let c = config();
        write(&host.path().join(source), &c);
        let mut command = Command::new(env!("CARGO_BIN_EXE_chrono-ci-test-cli"));
        command
            .args(["generate", "--host-root"])
            .arg(host.path())
            .args(["--config", source])
            .env("CHRONO_CI_GENERATION_TEST_CHECKPOINT", point)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        host.bind_command(&mut command).unwrap();
        let mut child = command.spawn().unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let mut first = String::new();
        stdout.read_line(&mut first).unwrap();
        // Always join the original child before judging its checkpoint.
        let kill = child.kill();
        let status = child.wait().unwrap();
        let mut stderr = String::new();
        child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        assert_eq!(
            first,
            format!("generation-checkpoint:{point}\n"),
            "{status}; {stderr}"
        );
        kill.unwrap();
        assert_eq!(status.signal(), Some(9));
        let operation = generation_operation(host.path());
        let path = operation
            .strip_prefix(host.path())
            .unwrap()
            .to_str()
            .unwrap();
        let before = fs::read(host.path().join(&c.workflow_path)).ok();
        let before_identity =
            chrono_harness::artifact_disposal::identity(&host.path().join(&c.workflow_path)).ok();
        let result = chrono_ci::generation::resume(host.path(), path).unwrap();
        assert_eq!(fs::read(host.path().join(&c.workflow_path)).ok(), before);
        assert_eq!(
            chrono_harness::artifact_disposal::identity(&host.path().join(&c.workflow_path)).ok(),
            before_identity
        );
        assert_eq!(generation_operation(host.path()), operation);
        if matches!(point, "intent" | "prepared" | "effect") {
            assert_eq!(before.is_some(), point == "effect");
            assert_eq!(result["completion"], "unknown");
            assert!(!operation.join("result.json").exists());
        } else {
            assert!(before.is_some());
            assert_eq!(result["completion"], "confirmed-evaluation");
            assert_eq!(
                result["original_result"]["producer_evaluation"],
                json!({"changed":true})
            );
            assert_eq!(
                result["original_result"]["original_process_exit"],
                Value::Null
            );
            assert_eq!(
                chrono_ci::generation::resume(host.path(), path).unwrap()["original_result"],
                result["original_result"]
            );
        }
    }
}

#[test]
fn generation_does_not_take_a_store_wide_lock_or_read_unrelated_history() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let host = tools::temporary_host("generation-independent");
    let source = ".chrono-harness/ci/github.json";
    write(&host.path().join(source), &config());
    assert!(generate(host.path(), source, false).unwrap());
    let store = host
        .path()
        .join(".chrono-harness/state/ci-generation-operations");
    let unrelated = store.join("unrelated-unknown");
    fs::create_dir(&unrelated).unwrap();
    fs::write(unrelated.join("intent.json"), b"incomplete original").unwrap();
    let held = chrono_harness::ownership::DirectoryLease::acquire_local(&store, true, None)
        .unwrap()
        .unwrap();
    assert!(!generate(host.path(), source, true).unwrap());
    held.stable().unwrap();
    assert_eq!(
        fs::read(unrelated.join("intent.json")).unwrap(),
        b"incomplete original"
    );
}

#[test]
fn generation_terminal_records_retire_through_the_adopted_finite_policy() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let host = tools::temporary_host("generation-retirement");
    let source = ".chrono-harness/ci/github.json";
    write(&host.path().join(source), &config());
    let mut policy: Value = serde_json::from_slice(include_bytes!(
        "../../../assets/ci/standalone-retention.json"
    ))
    .unwrap();
    policy["terminal_limits"]["ci-generation-operation"]["max_count"] = json!(1);
    fs::write(
        host.path().join(".chrono-harness/retention.json"),
        serde_json::to_vec_pretty(&policy).unwrap(),
    )
    .unwrap();
    assert!(generate(host.path(), source, false).unwrap());
    let first = generation_operation(host.path());
    let first_path = first.strip_prefix(host.path()).unwrap().to_str().unwrap();
    assert!(!generate(host.path(), source, true).unwrap());
    // Each maintenance call is one bounded mutation, not a synchronous drain.
    // Keep the adopted limits and judge the physical and metadata retirement
    // after a finite sequence of real rounds, retaining every original result.
    let mut rounds = Vec::new();
    for _ in 0..32 {
        let report = chrono_ci::generation::maintain(host.path()).unwrap();
        assert_eq!(report["quota_errors"], json!([]), "{rounds:?}; {report}");
        assert_eq!(
            report["round"]["accounting_errors"],
            json!([]),
            "{rounds:?}; {report}"
        );
        assert!(report["round"]["nodes"].as_u64().unwrap() <= 256);
        rounds.push(report);
        if !first.exists()
            && chrono_ci::generation::status(host.path(), first_path).unwrap()["availability"]
                == "unavailable-or-retired"
        {
            break;
        }
    }
    assert!(
        rounds
            .iter()
            .any(|report| report["quota_scope_complete"] == true)
    );
    assert!(
        !first.exists(),
        "expired original must be physically retired: {rounds:?}"
    );
    let retired = chrono_ci::generation::status(host.path(), first_path).unwrap();
    assert_eq!(retired["availability"], "unavailable-or-retired");
    assert_eq!(retired["success_inferred"], false);
    assert!(chrono_ci::generation::resume(host.path(), first_path).is_err());
    assert!(host.path().join(config().workflow_path).exists());
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
    tools::write_event_facts(d.path());
    git(d.path(), &["add", "."]);
    git(d.path(), &["commit", "-qm", "base"]);
    let b = git(d.path(), &["rev-parse", "HEAD"]);
    fs::write(d.path().join("a"), "b").unwrap();
    git(d.path(), &["commit", "-qam", "candidate"]);
    let c = git(d.path(), &["rev-parse", "HEAD"]);
    (d, b, c)
}
#[test]
fn init_from_external_config_and_spaced_host_is_idempotent() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
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
    if !chrono_git_boundary::isolated_git() {
        return;
    }
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
    if !chrono_git_boundary::isolated_git() {
        return;
    }
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
    if !chrono_git_boundary::isolated_git() {
        return;
    }
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
    if !chrono_git_boundary::isolated_git() {
        return;
    }
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
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("source.json");
    let c = config();
    write(&p, &c);
    init(d.path(), &p).unwrap();
    let old = fs::read(d.path().join(&c.workflow_path)).unwrap();
    let mut changed = c.clone();
    changed.bootstrap = vec![
        "/bin/bash".into(),
        ".chrono-harness/ci/new bootstrap.sh".into(),
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
    if !chrono_git_boundary::isolated_git() {
        return;
    }
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
fn report_upload_and_failed_job_diagnostics_have_separate_consumers() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("source.json");
    let mut raw = serde_json::to_value(config()).unwrap();
    raw["artifact_directory"] = json!(".chrono-harness/state/reports/");
    raw["context_path"] = json!(".chrono-harness/state/reports/context.json");
    raw["failure_artifact_directory"] = json!(".chrono-harness/state/");
    let configured: Config = serde_json::from_value(raw.clone()).unwrap();
    write(&p, &configured);
    init(d.path(), &p).unwrap();
    let workflow: serde_yaml_ng::Value =
        serde_yaml_ng::from_slice(&fs::read(d.path().join(".github/workflows/check.yml")).unwrap())
            .unwrap();
    let steps = workflow["jobs"]["check"]["steps"].as_sequence().unwrap();
    let evidence = steps
        .iter()
        .find(|s| s["name"] == "Preserve actual check evidence")
        .unwrap();
    let diagnostics = steps
        .iter()
        .find(|s| s["name"] == "Preserve failed job diagnostics")
        .unwrap();
    assert_eq!(evidence["with"]["path"], ".chrono-harness/state/reports/");
    assert_eq!(evidence["if"], "${{ always() }}");
    assert_eq!(diagnostics["with"]["path"], ".chrono-harness/state/");
    assert_eq!(diagnostics["if"], "${{ failure() }}");
    assert_ne!(diagnostics["with"]["name"], evidence["with"]["name"]);
    for invalid in [Value::Null, json!("/tmp/"), json!(".chrono-harness/state")] {
        raw["failure_artifact_directory"] = invalid;
        fs::write(&p, serde_json::to_vec(&raw).unwrap()).unwrap();
        assert!(init(d.path(), &p).is_err());
    }
}
#[test]
fn push_uses_actual_before_after() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
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
fn pr_checks_fixed_merge_tree_and_keeps_event_head_identity() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let (d, b, c) = repo();
    let p = json!({"pull_request":{"base":{"sha":b},"head":{"sha":c}},"after":"ignored-merge"});
    let merge = tools::pr_workflow_merge(d.path(), &b, &c);
    git(d.path(), &["checkout", "--detach", &merge]);
    let r = prepare(d.path(), &config(), "pull_request", &p, &merge).unwrap();
    assert_eq!(r["candidate"], merge);
    assert_eq!(r["event_head"], c);
    assert_eq!(r["workflow_source_revision"], merge);
    assert_eq!(r["base"], b);
}

#[test]
fn pr_merge_candidate_contains_concurrent_target_changes_without_updating_head() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let (d, old_base, head) = repo();
    git(d.path(), &["checkout", "--detach", &old_base]);
    fs::write(d.path().join("concurrent"), "target-only input").unwrap();
    git(d.path(), &["add", "concurrent"]);
    git(d.path(), &["commit", "-qm", "concurrent target change"]);
    let base = git(d.path(), &["rev-parse", "HEAD"]);
    git(d.path(), &["merge", "--no-ff", "--no-edit", &head]);
    let merge = git(d.path(), &["rev-parse", "HEAD"]);
    let payload = json!({"pull_request":{"base":{"sha":old_base},"head":{"sha":head}}});
    let context = prepare(d.path(), &config(), "pull_request", &payload, &merge).unwrap();
    assert_eq!(context["base"], base);
    assert_eq!(context["candidate"], merge);
    assert_eq!(context["event_head"], head);
    assert_eq!(
        git(d.path(), &["show", &format!("{merge}:concurrent")]),
        "target-only input"
    );
    assert_eq!(git(d.path(), &["show", &format!("{merge}:a")]), "b");
    // A head-only checkout must not obtain a merge-tree verdict.
    git(d.path(), &["checkout", "--detach", &head]);
    assert!(
        prepare(d.path(), &config(), "pull_request", &payload, &merge)
            .unwrap_err()
            .contains("checkout is not exact event candidate")
    );
}

#[test]
fn pr_rejects_non_merge_and_unrelated_workflow_parents() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let (d, base, head) = repo();
    let payload = json!({"pull_request":{"base":{"sha":base},"head":{"sha":head}}});
    let reversed = tools::pr_workflow_merge(d.path(), &head, &base);
    for revision in [&base, &head, &reversed] {
        git(d.path(), &["checkout", "--detach", revision]);
        let error = prepare(d.path(), &config(), "pull_request", &payload, revision).unwrap_err();
        assert!(
            error.contains("exactly two parents and the event head as second parent"),
            "{error}"
        );
        assert!(error.contains("git_facts"), "{error}");
    }
    git(d.path(), &["checkout", "--detach", &head]);
    assert_eq!(git(d.path(), &["rev-parse", "HEAD"]), head);
}

#[test]
fn configured_push_baseline_covers_repeated_pushes_and_tracks_the_named_remote_ref() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
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
        json!([configured.runner, "check"])
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
    if !chrono_git_boundary::isolated_git() {
        return;
    }
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
fn preparation_fetches_pinned_baseline_and_preserves_customization() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
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
    let saved: Config =
        serde_json::from_slice(&fs::read(d.path().join(".chrono-harness/ci/github.json")).unwrap())
            .unwrap();
    assert_eq!(
        serde_json::to_value(&saved).unwrap(),
        serde_json::to_value(&configured).unwrap()
    );
    let context = prepare(
        d.path(),
        &saved,
        "push",
        &json!({"ref":"refs/heads/verify/change","before":base,"after":candidate}),
        &candidate,
    )
    .unwrap();
    assert_eq!(context["base"], advanced);
    assert_eq!(context["candidate"], candidate);
    assert_eq!(context["source"], "push-configured-baseline-ref");
    assert_eq!(
        git(d.path(), &["rev-parse", &format!("{advanced}^{{commit}}")]),
        advanced
    );
    assert_eq!(context["workflow_source_revision"], candidate);
    assert_eq!(context["canonical_argv"], json!([saved.runner, "check"]));
}
#[test]
fn manual_full_range_and_explicit_initial() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
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
    if !chrono_git_boundary::isolated_git() {
        return;
    }
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
    if !chrono_git_boundary::isolated_git() {
        return;
    }
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
    if !chrono_git_boundary::isolated_git() {
        return;
    }
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
    if !chrono_git_boundary::isolated_git() {
        return;
    }
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
    assert_eq!(argv, vec![".chrono-harness/bin/chrono-harness", "check"]);
}
#[test]
fn standalone_prepare_is_rejected_before_context_and_output_publication() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
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
    fs::write(&output, "untouched\n").unwrap();
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
    let result = chrono_ci::dispatch(&a);
    assert!(result.is_err(), "retired prepare accepted: {result:?}");
    assert_eq!(fs::read_to_string(output).unwrap(), "untouched\n");
    assert!(!d.path().join(".chrono-harness/state/context.json").exists());
}
fn retain_command_result(
    root: &Path,
    argv: &[String],
    context: &Value,
    output: &std::process::Output,
) {
    // Existing callers retain an unreleased diagnostic reference. They do not
    // acquire disposal eligibility from another caller's expected-exit contract.
    if std::env::var_os("CHRONO_CI_TEST_RECEIPTS").is_none() && output.status.success() {
        return;
    }
    // Both diagnostic consumers use the fixture's actual receipt owner. The
    // optional destination receives references, as in judge-ci diagnostics;
    // it is not a request to qualify the exposed source checkout through a mock.
    let mut owner = retained_receipts::Owner::new();
    owner.preserve_diagnostic();
    let _ = retain_expected_command_result(&owner.root, root, argv, context, output, None);
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
        None,
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
    let export = std::env::var_os("CHRONO_CI_TEST_RECEIPTS").map(std::path::PathBuf::from);
    retain_command_result_exported(
        producer_root,
        root,
        argv,
        context,
        output,
        expected_exit,
        directory_override,
        birth_observer,
        export.as_deref(),
    )
}
fn retain_command_result_exported(
    producer_root: &Path,
    root: &Path,
    argv: &[String],
    context: &Value,
    output: &std::process::Output,
    expected_exit: Option<i32>,
    directory_override: Option<&Path>,
    birth_observer: Option<&mut dyn FnMut(&str)>,
    export: Option<&Path>,
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
    if let Some(export) = export {
        receipt
            .acquire(
                "diagnostic",
                "finite caller debugging/acceptance; caller owns release",
            )
            .unwrap();
        let mut originals = serde_json::Map::new();
        for name in ["stdout.bin", "stderr.bin", "binding.json"] {
            let path = destination.join(name);
            let (sha256, size) = chrono_harness::file_identity(&path).unwrap();
            originals.insert(
                name.into(),
                json!({"path":path,"sha256":sha256,"size":size}),
            );
        }
        let identity = chrono_harness::sha256(destination.as_os_str().as_encoded_bytes());
        let mut reference =
            fs::File::create_new(export.join(format!("ci-command-{identity}.json"))).unwrap();
        serde_json::to_writer(&mut reference, &json!({
            "consumer":"finite engineering acceptance",
            "retention":"until caller debugging/acceptance obligations finish; caller owns release",
            "producer":"ci-command-result","producer_root":producer_root,
            "directory":directory,"artifact":destination,"originals":originals,
            "release_consumers":["command-assertion","diagnostic"]
        })).unwrap();
    }
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
fn baseline_creation_with_parents_requires_explicit_range() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
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
    if !chrono_git_boundary::isolated_git() {
        return;
    }
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
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    assert_eq!(
        chrono_ci::dispatch(&["--version".into()]).unwrap(),
        "chrono-ci 0.1.0\n"
    );
    assert!(chrono_ci::dispatch(&["--version".into(), "extra".into()]).is_err());
}

#[test]
fn adopted_bootstrap_shell_ignores_path_shadow_and_matches_host_version() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let read =
        |p: &str| -> Value { serde_json::from_slice(&fs::read(source.join(p)).unwrap()).unwrap() };
    let config = read(".chrono-harness/git/macos-27.json");
    let mut tool = config["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == "bash")
        .unwrap()
        .clone();
    let registered_program = tool["program"].clone();
    let selected = tempfile::tempdir().unwrap();
    let executable = selected.path().join("selected-bash");
    fs::copy(env!("CARGO_BIN_EXE_chrono-ci-test-transport"), &executable).unwrap();
    tool["program"] = json!(executable);
    // The dependency's version boundary is explicit fixture data.
    let observed = Command::new(tool["program"].as_str().unwrap())
        .args(serde_json::from_value::<Vec<String>>(tool["version_argv"].clone()).unwrap())
        .env_clear()
        .output()
        .unwrap();
    assert!(observed.status.success());
    tool["expected_version"] = json!(String::from_utf8(observed.stdout).unwrap().trim_end());
    let dir = tempfile::tempdir().unwrap();
    let shadow = dir.path().join("bash");
    std::os::unix::fs::symlink(
        fs::canonicalize(env!("CARGO_BIN_EXE_chrono-ci-test-transport")).unwrap(),
        &shadow,
    )
    .unwrap();
    let control = Command::new("bash")
        .env_clear()
        .env("PATH", dir.path())
        .arg("--version")
        .output()
        .unwrap();
    assert!(control.status.success());
    assert_eq!(control.stdout, b"Bash shadowed\n");
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
        "registered bootstrap shell must survive unrelated PATH precedence"
    );
    let provider = read(".chrono-harness/ci/units.json");
    assert_eq!(provider["collection"]["bootstrap"][0], registered_program);
    for unit in provider["units"].as_object().unwrap().values() {
        assert_eq!(unit["bootstrap"][0], registered_program);
    }
}

#[test]
fn opted_in_delivery_acknowledgement_uses_the_actual_successful_upload_outputs() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let mut declaration = config();
    let old = chrono_ci::render(&declaration, ".chrono-harness/ci/workflow.json").unwrap();
    assert!(!old.contains("Acknowledge delivered"));
    declaration.retention_delivery = true;
    let projected = chrono_ci::render(&declaration, ".chrono-harness/ci/workflow.json").unwrap();
    assert!(projected.contains("id: chrono_evidence_upload"));
    assert!(projected.contains("always() && steps.chrono_evidence_upload.outcome == 'success'"));
    assert!(projected.contains("steps.chrono_evidence_upload.outputs.artifact-id"));
    assert!(projected.contains("steps.chrono_evidence_upload.outputs.artifact-digest"));
    assert!(projected.contains("retention --host-root . --delivered-check"));
}
