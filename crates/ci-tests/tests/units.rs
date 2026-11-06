#[path = "short.rs"]
mod short;
use super::*;

const UNIT_TOOL: &str = env!("CARGO_BIN_EXE_chrono-ci-test-transport");

fn config_value() -> Value {
    let mut common = serde_json::to_value(config()).unwrap();
    common["workflow_path"] = json!(".github/workflows/collection.yml");
    json!({"schema":"chrono-github-units/v1","collection":common,
        "units":{
            "alpha":{"workflow_path":".github/workflows/alpha.yml","name":"alpha check","runs_on":"ubuntu-24.04","timeout_minutes":10,"bootstrap":["sh","explicit-a.sh"],"context_path":".chrono-harness/state/alpha/context.json","artifact_directory":".chrono-harness/state/"},
            "beta":{"workflow_path":".github/workflows/beta.yml","name":"beta check","runs_on":"macos-14","timeout_minutes":12,"bootstrap":["sh","explicit-b.sh"],"context_path":".chrono-harness/state/beta/context.json","artifact_directory":".chrono-harness/state/"}},
        "gather":{"program":"gh","inherit_environment":["PATH","HOME","GH_TOKEN"],"credential_environment":["GH_TOKEN"],"environment":{"GH_PROMPT_DISABLED":"1"},"timeout_seconds":30,"output_limit_bytes":1048576,"wait_seconds":600,"poll_seconds":15,"manifest_path":".chrono-harness/state/collection/manifest.json","download_directory":".chrono-harness/state/collection/downloads/","report_path":".chrono-harness/state/collection/gather.json"}})
}

fn fixture() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join(".chrono-harness/ci/units.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec_pretty(&config_value()).unwrap()).unwrap();
    let check = json!({"schema":"chrono-ci-check/v3","judge":{"program":".chrono-harness/bin/chrono-judge-ci","args":[],"timeout_seconds":20,"output_limit_bytes":1048576},"report_path":".chrono-harness/state/check.json","policy":{"units":{"alpha":{"tests":["test:a"],"report_path":".chrono-harness/state/alpha/check.json"},"beta":{"tests":["test:b"],"report_path":".chrono-harness/state/beta/check.json"}}}});
    fs::write(
        d.path().join(".chrono-harness/ci/check.json"),
        serde_json::to_vec(&check).unwrap(),
    )
    .unwrap();
    d
}

#[test]
fn full_units_require_exact_separate_context_mappings_and_keep_v1_strict() {
    let mut valid = config_value();
    valid["schema"] = json!("chrono-github-units/v2");
    valid["collection"]["schema"] = json!("chrono-github-ci/v4");
    valid["collection"]["check_config"] = json!(".chrono-harness/config.json");
    valid["collection"]["facts_config"] = json!(".chrono-harness/config.json");
    valid["collection"]["artifact_directory"] = json!(".chrono-harness/state/collection/");
    valid["collection"]["context_path"] = json!(".chrono-harness/state/collection/event.json");
    for id in ["alpha", "beta"] {
        valid["units"][id]["artifact_directory"] = json!(format!(".chrono-harness/state/{id}/"));
    }
    valid["full_contexts"] = json!({"collection":".chrono-harness/state/collection/full.json","units":{"alpha":".chrono-harness/state/alpha/full.json","beta":".chrono-harness/state/beta/full.json"}});
    let config: chrono_ci::units::Config = serde_json::from_value(valid.clone()).unwrap();
    chrono_ci::units::validate(&config).unwrap();
    for case in [
        "legacy",
        "missing",
        "null",
        "extra",
        "wrong-upload",
        "event-alias",
        "manifest-alias",
        "overlap-upload",
    ] {
        let mut value = valid.clone();
        match case {
            "legacy" => value["schema"] = json!("chrono-github-units/v1"),
            "missing" => {
                value.as_object_mut().unwrap().remove("full_contexts");
            }
            "null" => value["full_contexts"] = Value::Null,
            "extra" => {
                value["full_contexts"]["units"]["undeclared"] =
                    json!(".chrono-harness/state/extra/full.json")
            }
            "wrong-upload" => {
                value["full_contexts"]["units"]["alpha"] =
                    json!(".chrono-harness/state/beta/full.json")
            }
            "event-alias" => {
                value["full_contexts"]["units"]["alpha"] =
                    json!(".chrono-harness/state/alpha//context.json")
            }
            "manifest-alias" => {
                value["full_contexts"]["collection"] =
                    json!(".chrono-harness/state/collection//manifest.json")
            }
            _ => {
                value["units"]["beta"]["artifact_directory"] =
                    json!(".chrono-harness/state/alpha//nested/")
            }
        }
        let d = tempfile::tempdir().unwrap();
        json_file(d.path(), ".chrono-harness/ci/units.json", &value);
        assert!(
            generate(d.path(), ".chrono-harness/ci/units.json", false).is_err(),
            "accepted {case}"
        );
        assert!(
            !d.path().join(".github").exists(),
            "wrote output for {case}"
        );
        assert!(
            !d.path().join(".chrono-harness/state").exists(),
            "wrote state for {case}"
        );
    }
}

#[test]
fn generator_projects_independent_workflows_with_exact_scoped_commands() {
    let d = fixture();
    assert!(generate(d.path(), ".chrono-harness/ci/units.json", false).unwrap());
    assert!(!generate(d.path(), ".chrono-harness/ci/units.json", true).unwrap());
    let a = fs::read_to_string(d.path().join(".github/workflows/alpha.yml")).unwrap();
    let b = fs::read_to_string(d.path().join(".github/workflows/beta.yml")).unwrap();
    let summary = fs::read_to_string(d.path().join(".github/workflows/collection.yml")).unwrap();
    assert!(a.contains("'--unit' 'alpha'"));
    assert!(b.contains("'--unit' 'beta'"));
    assert!(a.contains("'explicit-a.sh'"));
    assert!(!a.contains("explicit-b.sh"));
    assert!(b.contains("'explicit-b.sh'"));
    assert!(!b.contains("explicit-a.sh"));
    assert!(!a.contains("needs:"));
    assert!(!b.contains("needs:"));
    assert!(summary.contains("'check' '--collect'\n"));
    assert!(!summary.contains(" gather "));
    assert!(!a.contains("--base"));
    assert!(summary.contains("actions: read"));
}

#[test]
fn parent_diagnostic_artifacts_cannot_alias_another_units_report() {
    let mut raw = config_value();
    raw["collection"]["schema"] = json!("chrono-github-ci/v4");
    raw["collection"]["facts_config"] = json!(".chrono-harness/config.json");
    raw["collection"]["failure_artifact_directory"] = json!(".chrono-harness/state/");
    let mut extra = raw["units"]["alpha"].clone();
    extra["workflow_path"] = json!(".github/workflows/alpha-diagnostics.yml");
    extra["name"] = json!("alpha diagnostics check");
    raw["units"]["alpha-diagnostics"] = extra;
    for id in ["alpha", "beta", "alpha-diagnostics"] {
        raw["units"][id]["artifact_directory"] = json!(format!(".chrono-harness/state/{id}/"));
        raw["units"][id]["context_path"] =
            json!(format!(".chrono-harness/state/{id}/context.json"));
    }
    raw["job_gating"] = json!({"schema":"chrono-job-gating/v1","detector":{
        "runs_on":"ubuntu-24.04","timeout_minutes":10,"bootstrap":["host-tools-only"],
        "sparse_checkout":[".chrono-harness/", ".github/workflows/collection.yml"]}});
    let configured: chrono_ci::units::Config = serde_json::from_value(raw).unwrap();
    chrono_ci::units::validate(&configured).unwrap();
    let outputs = chrono_ci::units::render(&configured, ".chrono-harness/ci/units.json").unwrap();
    let parent: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&outputs[".github/workflows/collection.yml"]).unwrap();
    let mut names = std::collections::BTreeSet::new();
    let mut diagnostics = 0;
    for job in parent["jobs"].as_mapping().unwrap().values() {
        for step in job["steps"].as_sequence().unwrap() {
            if step["name"] == "Preserve failed job diagnostics" {
                diagnostics += 1;
            }
            if step["uses"] == configured.collection.upload_artifact_action.as_str() {
                let name = step["with"]["name"].as_str().unwrap();
                assert!(
                    names.insert(name),
                    "two jobs upload the same artifact: {name}"
                );
            }
        }
    }
    assert_eq!(diagnostics, configured.units.len());
}

#[test]
fn generator_preflights_all_outputs_before_any_unit_write() {
    let d = fixture();
    fs::create_dir_all(d.path().join(".github/workflows")).unwrap();
    fs::write(d.path().join(".github/workflows/beta.yml"), "host-owned\n").unwrap();
    assert!(
        generate(d.path(), ".chrono-harness/ci/units.json", false)
            .unwrap_err()
            .contains("collision")
    );
    assert!(!d.path().join(".github/workflows/alpha.yml").exists());
    assert!(!d.path().join(".github/workflows/collection.yml").exists());
}

fn json_file(root: &Path, path: &str, value: &Value) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

fn install(root: &Path) {
    install_tools(
        root,
        &[
            ("runner", "chrono-harness", ToolProbe::Version),
            ("judge-ci", "chrono-judge-ci", ToolProbe::JudgeEof),
            ("ci", "chrono-ci", ToolProbe::Version),
        ],
    );
}

enum ToolProbe {
    Version,
    JudgeEof,
}

fn install_tools(root: &Path, tools: &[(&str, &str, ToolProbe)]) {
    install_tools_owned(root, tools, false);
}
fn install_retained_tools(root: &Path, tools: &[(&str, &str, ToolProbe)]) {
    install_tools_owned(root, tools, true);
}
fn install_tools_owned(root: &Path, tools: &[(&str, &str, ToolProbe)], retained: bool) {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bin = root.join(".chrono-harness/bin");
    fs::create_dir_all(&bin).unwrap();
    // Test the current product outputs; deployed host tools have a separate
    // caller-owned rollout and may intentionally retain an older schema.
    // Each temporary host owns its executable inode and pathname. A joined copy
    // child keeps writable descriptors out of concurrently forked test children.
    let mut copy = Command::new("/bin/cp");
    for (project, name, _) in tools {
        let built = source.join(format!("crates/{project}/target/debug/{name}"));
        assert!(matches!(
            fs::symlink_metadata(bin.join(name)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound
        ));
        if retained {
            crate::tools::copied_fixture_input(
                root,
                &built,
                &bin.join(name),
                "crates/ci-tests/tests/units.rs::install_retained_tools",
            );
        }
        copy.arg(built);
    }
    if !tools.is_empty() {
        copy.arg(&bin);
        let copied = if retained {
            chrono_harness::process_fds::forward_command(&mut copy, &[]).unwrap();
            chrono_worktree::TemporaryHost::capture_bound_output(root, &mut copy).unwrap()
        } else {
            copy.output().unwrap()
        };
        assert!(
            copied.status.success(),
            "candidate fixture installation: {}",
            String::from_utf8_lossy(&copied.stderr)
        );
    }
    // Finish fixture setup before fault injection and bounded check execution.
    // Each caller declares the program's real readiness protocol.
    for (_, name, probe) in tools {
        let mut command = Command::new(bin.join(name));
        command
            .current_dir(root)
            .env_clear()
            .stdin(std::process::Stdio::null());
        if matches!(probe, ToolProbe::Version) {
            command.arg("--version");
        }
        let argv: Vec<_> = std::iter::once(command.get_program())
            .chain(command.get_args())
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        let output = if retained {
            chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
            chrono_worktree::TemporaryHost::capture_bound_output(root, &mut command).unwrap()
        } else {
            command.output().unwrap()
        };
        let ready = match probe {
            ToolProbe::Version => output.status.success(),
            ToolProbe::JudgeEof => {
                output.status.code() == Some(2)
                    && output.stdout.is_empty()
                    && output.stderr.starts_with(b"E_REQUEST: ")
            }
        };
        if !ready {
            retain_command_result(root, &argv, &json!({"fixture":"tool-ready"}), &output);
        }
        assert!(ready, "fixture {name} readiness: {output:?}");
    }
}

#[test]
fn fixture_executable_identity_survives_neighbor_teardown() {
    use std::os::unix::fs::MetadataExt;

    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/judge-ci/target/debug/chrono-judge-ci");
    let first = fixture();
    let second = fixture();
    let tools = [("judge-ci", "chrono-judge-ci", ToolProbe::JudgeEof)];
    install_tools(first.path(), &tools);
    install_tools(second.path(), &tools);
    let relative = ".chrono-harness/bin/chrono-judge-ci";
    let original = fs::metadata(&source).unwrap();
    let one = fs::metadata(first.path().join(relative)).unwrap();
    let two = fs::metadata(second.path().join(relative)).unwrap();
    assert_ne!((one.dev(), one.ino()), (original.dev(), original.ino()));
    assert_ne!((two.dev(), two.ino()), (original.dev(), original.ino()));
    assert_ne!((one.dev(), one.ino()), (two.dev(), two.ino()));
    assert_eq!(
        fs::read(first.path().join(relative)).unwrap(),
        fs::read(&source).unwrap()
    );
    drop(first);
    assert_eq!(
        fs::read(second.path().join(relative)).unwrap(),
        fs::read(&source).unwrap()
    );
    let result = Command::new(second.path().join(relative))
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    assert!(result.stderr.starts_with(b"E_REQUEST: "));
}

fn consumer() -> (short::ShortHost, String, String) {
    let host = short::ShortHost::new();
    let (base, candidate) = (host.base.clone(), host.candidate.clone());
    (host, base, candidate)
}

// Clone sources contain repository inputs; only an executing host installs tools.
fn consumer_source() -> (tempfile::TempDir, String, String) {
    let d = fixture();
    let root = d.path();
    tools::write_event_facts(root);
    fs::write(
        root.join(".gitignore"),
        ".chrono-harness/state/\n.chrono-harness/bin/\n",
    )
    .unwrap();
    fs::write(root.join("a.txt"), "one").unwrap();
    fs::write(root.join("b.txt"), "one").unwrap();
    fs::write(root.join("a.case"), "alpha\n").unwrap();
    fs::write(root.join("b.case"), "beta\n").unwrap();
    let path = ".chrono-harness/ci/check.json";
    let mut check: Value = serde_json::from_slice(&fs::read(root.join(path)).unwrap()).unwrap();
    check["policy"]["filemap"] = json!(".chrono-harness/FILEMAP.json");
    check["policy"]["projects"] = json!(".chrono-harness/projects.json");
    check["policy"]["tools"] = json!({"unit-fixture":UNIT_TOOL});
    check["policy"]["bindings"] = json!({"test:a":["test.a"],"test:b":["test.b"]});
    check["policy"]["artifacts"] = json!([".chrono-harness/state/", ".chrono-harness/bin/"]);
    check["policy"]["required_inputs"] = json!([path]);
    check["policy"]["adoption_base"] = Value::Null;
    check["policy"]["operation_timeout_seconds"] = json!(5);
    check["policy"]["operation_output_limit_bytes"] = json!(4096);
    check["policy"]["shared_operations"] = json!({});
    json_file(root, path, &check);
    json_file(
        root,
        ".chrono-harness/projects.json",
        &json!({"schema_version":1,"owners":["host","a","b"],"projects":[
        {"id":"a","actions":{"execute":{"operation":"test.a","tool":"unit-fixture","argv":["unit-operation","a.case"]}}},
        {"id":"b","actions":{"execute":{"operation":"test.b","tool":"unit-fixture","argv":["unit-operation","b.case"]}}}],"scripts":[]}),
    );
    let mut files = vec![];
    for path in [
        ".gitignore",
        ".chrono-harness/event-facts.json",
        "a.txt",
        "b.txt",
        "a.case",
        "b.case",
        ".chrono-harness/projects.json",
        ".chrono-harness/FILEMAP.json",
        path,
        ".chrono-harness/ci/units.json",
        ".github/workflows/alpha.yml",
        ".github/workflows/beta.yml",
        ".github/workflows/collection.yml",
    ] {
        let edges = match path {
            "a.txt" | "a.case" => json!([{"kind":"test-execution","to":"test:a"}]),
            "b.txt" | "b.case" => json!([{"kind":"test-execution","to":"test:b"}]),
            _ => json!([]),
        };
        files.push(json!({"path":path,"owner":"host","surface":"product","cost":"unmeasured","edges":edges}));
    }
    json_file(
        root,
        ".chrono-harness/FILEMAP.json",
        &json!({"schema_version":1,"files":files,"project_edges":[]}),
    );
    generate(root, ".chrono-harness/ci/units.json", false).unwrap();
    git(root, &["init", "-q", "-b", "dev"]);
    git(root, &["config", "user.email", "fixture@example.invalid"]);
    git(root, &["config", "user.name", "Fixture"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "base"]);
    let b = git(root, &["rev-parse", "HEAD"]);
    fs::write(root.join("a.txt"), "two").unwrap();
    fs::write(root.join("b.txt"), "two").unwrap();
    git(root, &["commit", "-qam", "candidate"]);
    let c = git(root, &["rev-parse", "HEAD"]);
    (d, b, c)
}

fn unit_context(root: &Path, b: &str, c: &str, unit: Option<&str>) -> Value {
    let config: chrono_ci::units::Config =
        serde_json::from_slice(&fs::read(root.join(".chrono-harness/ci/units.json")).unwrap())
            .unwrap();
    chrono_ci::units::prepare(
        root,
        ".chrono-harness/ci/units.json",
        &config,
        unit,
        "push",
        &json!({"ref":"refs/heads/dev","before":b,"after":c}),
        c,
    )
    .unwrap()
}

#[test]
fn real_unit_checkouts_execute_concurrently_and_collect_original_reports() {
    let (host, b, c) = consumer();
    let mut copies = vec![];
    for _ in 0..2 {
        let copy = tempfile::Builder::new()
            .prefix("unit host λ ")
            .tempdir()
            .unwrap();
        git(
            copy.path(),
            &["clone", "-q", host.path().to_str().unwrap(), "."],
        );
        install(copy.path());
        install_tools(
            copy.path(),
            &[("worktree", "chrono-worktree", ToolProbe::Version)],
        );
        git(
            copy.path(),
            &["remote", "set-url", "origin", host.remote.to_str().unwrap()],
        );
        copies.push(copy);
    }
    let reports = std::thread::scope(|threads| {
        let workers: Vec<_> = copies
            .iter()
            .zip(["alpha", "beta"])
            .map(|(copy, unit)| {
                let (b, c) = (&b, &c);
                threads.spawn(move || {
                    execute_context(copy.path(), &unit_context(copy.path(), b, c, Some(unit)), 0)
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|w| w.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert!(
        !copies[0]
            .path()
            .join(".chrono-harness/state/calls-b")
            .exists()
    );
    assert!(
        !copies[1]
            .path()
            .join(".chrono-harness/state/calls-a")
            .exists()
    );
    assert!(!host.path().join(".chrono-harness/state/calls-a").exists());
    for ((unit, report), copy) in ["alpha", "beta"].iter().zip(&reports).zip(&copies) {
        assert_eq!(report["response"]["status"], "passed");
        let directory = format!(".chrono-harness/state/{unit}");
        short::copy_artifacts(&copy.path().join(&directory), &host.path().join(&directory));
        short::copy_artifacts(
            &copy.path().join(".chrono-harness/state/preparation"),
            &host.path().join(".chrono-harness/state/preparation"),
        );
    }
    let report = execute_context(host.path(), &unit_context(host.path(), &b, &c, None), 0);
    assert_eq!(
        report["response"]["evidence"]["reports"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(report["response"]["evidence"]["executed"], json!([]));
    assert!(!host.path().join(".chrono-harness/state/calls-a").exists());
    assert!(!host.path().join(".chrono-harness/state/calls-b").exists());
}

fn configure_mock_transport(config: &mut Value) {
    config["gather"]["program"] = json!(env!("CARGO_BIN_EXE_chrono-ci-test-transport"));
    config["gather"]["environment"]["CHRONO_CI_TEST_PROVIDER"] = json!("units");
}

fn install_mock_transport(root: &Path, mode: &str, c: &str) {
    json_file(
        root,
        ".chrono-harness/state/mock.json",
        &json!({"candidate":c,"mode":mode}),
    );
    // Establish the external fixture's readiness before measuring transport
    // deadlines. The probe has no artifact-download or failure-injection effects.
    let args = vec![
        env!("CARGO_BIN_EXE_chrono-ci-test-transport").to_string(),
        "--version".to_string(),
    ];
    let ready = Command::new(&args[0])
        .arg(&args[1])
        .current_dir(root)
        .env_clear()
        .env("CHRONO_CI_TEST_PROVIDER", "units")
        .output()
        .unwrap();
    if !ready.status.success() || ready.stdout != b"chrono-ci-test-provider/v1\n" {
        retain_command_result(root, &args, &json!({"fixture":"provider-ready"}), &ready);
    }
    assert!(ready.status.success(), "{:?}", ready.stderr);
    assert_eq!(ready.stdout, b"chrono-ci-test-provider/v1\n");
}

fn gather_host(mode: &str) -> (short::ShortHost, String, String) {
    gather_host_config(mode, |_| {})
}

fn gather_host_config(
    mode: &str,
    configure: impl FnOnce(&mut Value),
) -> (short::ShortHost, String, String) {
    let (mut host, b, _) = consumer();
    let root = host.path();
    let mut cfg = config_value();
    configure_mock_transport(&mut cfg);
    configure(&mut cfg);
    json_file(root, ".chrono-harness/ci/units.json", &cfg);
    generate(root, ".chrono-harness/ci/units.json", false).unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "transport fixture"]);
    let c = git(root, &["rev-parse", "HEAD"]);
    host.candidate = c.clone();
    let root = host.path();
    install_mock_transport(root, mode, &c);
    for unit in ["alpha", "beta"] {
        let mut context = unit_context(root, &b, &c, Some(unit));
        let report = execute_context(root, &context, 0);
        if mode == "stale-context" {
            context["base"] = json!(c);
        }
        json_file(
            root,
            &format!(".chrono-harness/state/mock-artifacts/{unit}/{unit}/context.json"),
            &context,
        );
        json_file(
            root,
            &format!(".chrono-harness/state/mock-artifacts/{unit}/{unit}/check.json"),
            &report,
        );
    }
    json_file(
        root,
        ".chrono-harness/state/context.json",
        &unit_context(root, &b, &c, None),
    );
    (host, b, c)
}

fn gather_cli(root: &Path) -> std::process::Output {
    Command::new(root.join(".chrono-harness/bin/chrono-ci"))
        .current_dir(root)
        .args([
            "gather",
            "--host-root",
            ".",
            "--config",
            ".chrono-harness/ci/units.json",
            "--repository",
            "owner/host",
        ])
        .env("GH_TOKEN", "fixture-credential-must-not-be-persisted")
        .output()
        .unwrap()
}

fn download_retry() -> Value {
    json!({"max_attempts":3,"delay_seconds":1,"http_statuses":[429,500,502,503,504]})
}

fn failed_downloads(root: &Path, count: u64, status: u64) {
    let path = root.join(".chrono-harness/state/mock.json");
    let mut data: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    data["download_failures"] = json!(count);
    data["http_status"] = json!(status);
    fs::write(path, serde_json::to_vec(&data).unwrap()).unwrap();
}

fn gathered(root: &Path) -> Value {
    serde_json::from_slice(
        &fs::read(root.join(".chrono-harness/state/collection/gather.json")).unwrap(),
    )
    .unwrap()
}

fn alpha_downloads(report: &Value) -> Vec<&Value> {
    report["processes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["argv"][1] == "run" && p["argv"][3] == "101")
        .collect()
}

#[test]
fn download_recovery_retains_failed_attempt_and_selects_only_complete_artifact() {
    let (host, b, c) = gather_host_config("success", |cfg| {
        cfg["gather"]["download_retry"] = download_retry();
    });
    failed_downloads(host.path(), 1, 503);
    let result = gather_cli(host.path());
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report = gathered(host.path());
    let attempts = alpha_downloads(&report);
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0]["exit_code"], 23);
    assert_eq!(attempts[1]["exit_code"], 0);
    let stderr: Vec<u8> = serde_json::from_value(attempts[0]["stderr_bytes"].clone()).unwrap();
    assert!(String::from_utf8_lossy(&stderr).contains("HTTP 503:"));
    let first = Path::new(
        attempts[0]["argv"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()
            .as_str()
            .unwrap(),
    );
    let second = Path::new(
        attempts[1]["argv"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()
            .as_str()
            .unwrap(),
    );
    assert_ne!(first, second);
    assert_eq!(
        fs::read(host.path().join(first).join("partial-only")).unwrap(),
        b"failed original bytes"
    );
    assert!(!host.path().join(second).join("partial-only").exists());
    assert_eq!(
        &attempts[0]["argv"].as_array().unwrap()[1..8],
        &attempts[1]["argv"].as_array().unwrap()[1..8]
    );
    execute_context(host.path(), &unit_context(host.path(), &b, &c, None), 0);
}

#[test]
fn download_recovery_is_opt_in_and_exhaustion_or_unlisted_status_stays_failed() {
    for (enabled, failures, status, expected_attempts) in
        [(false, 1, 503, 1), (true, 1, 401, 1), (true, 9, 503, 3)]
    {
        let (host, _, _) = gather_host_config("success", |cfg| {
            if enabled {
                cfg["gather"]["download_retry"] = download_retry();
            }
        });
        failed_downloads(host.path(), failures, status);
        let result = gather_cli(host.path());
        assert!(!result.status.success());
        let report = gathered(host.path());
        assert_eq!(alpha_downloads(&report).len(), expected_attempts);
        assert!(
            !host
                .path()
                .join(".chrono-harness/state/collection/manifest.json")
                .exists()
        );
    }
}

#[test]
fn download_recovery_does_not_accept_invalid_evidence_after_transport_recovers() {
    let (host, _, _) = gather_host_config("wrong-source", |cfg| {
        cfg["gather"]["download_retry"] = download_retry();
    });
    failed_downloads(host.path(), 1, 503);
    assert!(!gather_cli(host.path()).status.success());
    let report = gathered(host.path());
    assert_eq!(alpha_downloads(&report).len(), 2);
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("different workflow source")
    );
    assert!(
        !host
            .path()
            .join(".chrono-harness/state/collection/manifest.json")
            .exists()
    );
}

#[test]
fn download_recovery_configuration_rejects_ambiguous_or_unbounded_policy() {
    let mut valid = config_value();
    valid["gather"]["download_retry"] = download_retry();
    let c: chrono_ci::units::Config = serde_json::from_value(valid.clone()).unwrap();
    chrono_ci::units::validate(&c).unwrap();
    for (key, value) in [
        ("max_attempts", json!(0)),
        ("max_attempts", json!(11)),
        ("delay_seconds", json!(0)),
        ("delay_seconds", json!(60)),
        ("http_statuses", json!([])),
        ("http_statuses", json!([503, 503])),
        ("http_statuses", json!([200])),
        ("unknown", json!(true)),
    ] {
        let mut invalid = valid.clone();
        invalid["gather"]["download_retry"][key] = value;
        let parsed = serde_json::from_value::<chrono_ci::units::Config>(invalid);
        assert!(
            parsed.is_err() || chrono_ci::units::validate(&parsed.unwrap()).is_err(),
            "{key}"
        );
    }
    valid["gather"]["download_retry"] = Value::Null;
    assert!(serde_json::from_value::<chrono_ci::units::Config>(valid).is_err());
}

#[test]
fn download_recovery_shares_original_time_and_output_budgets() {
    for (mode, expected_attempts) in [("deadline", 1), ("output", 2), ("ambiguous-status", 1)] {
        let (host, _, _) = gather_host_config("success", |cfg| {
            cfg["gather"]["download_retry"] = download_retry();
            if mode == "deadline" {
                cfg["gather"]["timeout_seconds"] = json!(2);
            }
            if mode == "output" {
                cfg["gather"]["output_limit_bytes"] = json!(1024);
            }
        });
        failed_downloads(host.path(), 9, 503);
        let path = host.path().join(".chrono-harness/state/mock.json");
        let mut data: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        match mode {
            "deadline" => data["download_pause_ms"] = json!(1500),
            "output" => data["download_padding"] = json!(900),
            _ => data["second_http_status"] = json!(401),
        }
        fs::write(path, serde_json::to_vec(&data).unwrap()).unwrap();
        let started = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_string();
        let result = gather_cli(host.path());
        let returned = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_string();
        assert!(!result.status.success(), "{mode}");
        let report = gathered(host.path());
        let attempts = alpha_downloads(&report);
        if attempts.len() != expected_attempts {
            retain_command_result(
                host.path(),
                &["gather".into()],
                &json!({"case":mode,"expected_attempts":expected_attempts,
                    "started_unix_ns":started,"returned_unix_ns":returned}),
                &result,
            );
        }
        assert_eq!(
            attempts.len(),
            expected_attempts,
            "{mode}: {}",
            report["error"]
        );
        if mode == "output" {
            assert!(attempts.last().unwrap()["failure"].is_string());
            for stream in ["stdout_bytes", "stderr_bytes"] {
                assert!(
                    attempts
                        .iter()
                        .map(|p| p[stream].as_array().unwrap().len())
                        .sum::<usize>()
                        <= 1024
                );
            }
        }
        assert!(
            !host
                .path()
                .join(".chrono-harness/state/collection/manifest.json")
                .exists()
        );
    }
}

#[test]
fn provider_gathers_pinned_attempts_and_keeps_credentials_out_of_reports() {
    let (host, b, c) = gather_host("success");
    let result = gather_cli(host.path());
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = fs::read(
        host.path()
            .join(".chrono-harness/state/collection/gather.json"),
    )
    .unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("fixture-credential-must-not-be-persisted"));
    let gather: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(gather["result"]["status"], "gathered-unjudged");
    assert_eq!(gather["result"]["units"].as_array().unwrap().len(), 2);
    execute_context(host.path(), &unit_context(host.path(), &b, &c, None), 0);
    assert_eq!(
        fs::read_to_string(host.path().join(".chrono-harness/state/calls-a")).unwrap(),
        "alpha"
    );
    assert_eq!(
        fs::read_to_string(host.path().join(".chrono-harness/state/calls-b")).unwrap(),
        "beta"
    );
    let retained = gather["retained_report"].as_str().unwrap();
    let original = fs::read(host.path().join(retained)).unwrap();
    let again = gather_cli(host.path());
    assert!(again.status.success(), "{again:?}");
    assert_eq!(fs::read(host.path().join(retained)).unwrap(), original);
    assert_ne!(gathered(host.path())["retained_report"], retained);
}

#[test]
fn provider_refuses_ambiguous_stale_workflow_and_changed_attempt_evidence() {
    for (mode, needle) in [
        ("duplicate", "ambiguous"),
        ("stale-context", "context disagrees"),
        ("wrong-source", "different workflow source"),
        ("changed-attempt", "run changed"),
    ] {
        let (host, _, _) = gather_host(mode);
        let result = gather_cli(host.path());
        assert!(!result.status.success(), "{mode}");
        let report: Value = serde_json::from_slice(
            &fs::read(
                host.path()
                    .join(".chrono-harness/state/collection/gather.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(
            report["error"].as_str().unwrap().contains(needle),
            "{mode}: {report}"
        );
        assert!(
            !host
                .path()
                .join(".chrono-harness/state/collection/manifest.json")
                .exists()
        );
    }
}

#[test]
fn unit_init_preserves_host_customization_across_generation() {
    let d = fixture();
    let root = d.path();
    let source = ".chrono-harness/ci/units.json";
    let mut custom = config_value();
    custom["units"]["alpha"]["bootstrap"] = json!(["host-bootstrap", "space value", "$literal"]);
    custom["units"]["alpha"]["runs_on"] = json!("self-hosted-custom");
    custom["units"]["alpha"]["timeout_minutes"] = json!(37);
    json_file(root, source, &custom);
    let original = fs::read(root.join(source)).unwrap();
    let incoming = root.join("defaults.json");
    fs::write(&incoming, serde_json::to_vec(&config_value()).unwrap()).unwrap();
    assert!(init(root, &incoming).unwrap());
    assert!(!init(root, &incoming).unwrap());
    assert_eq!(fs::read(root.join(source)).unwrap(), original);
    assert!(!generate(root, source, true).unwrap());
    let output = fs::read_to_string(root.join(".github/workflows/alpha.yml")).unwrap();
    assert!(output.contains("'host-bootstrap' 'space value' '$literal'"));
    assert!(output.contains("self-hosted-custom"));
    assert!(output.contains("timeout-minutes: 37"));
}

fn migrate(root: &Path, prior: &str, prior_source: Option<&str>) -> Result<String, String> {
    let mut args = vec![
        "migrate".into(),
        "--host-root".into(),
        root.to_str().unwrap().into(),
        "--from".into(),
        prior.into(),
        "--config".into(),
        ".chrono-harness/ci/units.json".into(),
    ];
    if let Some(path) = prior_source {
        args.extend(["--previous-config".into(), path.into()]);
    }
    chrono_ci::dispatch(&args)
}

#[test]
fn migration_adopts_units_preserving_custom_source_and_unrelated_workflow() {
    let d = fixture();
    let root = d.path();
    let prior = ".chrono-harness/ci/github.json";
    let mut old = config();
    old.workflow_path = ".github/workflows/collection.yml".into();
    write(&root.join(prior), &old);
    generate(root, prior, false).unwrap();
    let mut custom = config_value();
    custom["units"]["alpha"]["bootstrap"] = json!(["my-custom-entry", "--literal", "a b"]);
    json_file(root, ".chrono-harness/ci/units.json", &custom);
    let original = fs::read(root.join(".chrono-harness/ci/units.json")).unwrap();
    fs::write(
        root.join(".github/workflows/host.yml"),
        "host workflow stays\n",
    )
    .unwrap();
    let report: Value = serde_json::from_str(&migrate(root, prior, None).unwrap()).unwrap();
    assert_eq!(report["status"], "migrated");
    assert_eq!(report["retired_source"], prior);
    assert!(!root.join(prior).exists());
    assert_eq!(
        fs::read(root.join(".chrono-harness/ci/units.json")).unwrap(),
        original
    );
    assert_eq!(
        fs::read_to_string(root.join(".github/workflows/host.yml")).unwrap(),
        "host workflow stays\n"
    );
    assert!(!generate(root, ".chrono-harness/ci/units.json", true).unwrap());
    assert!(
        fs::read_to_string(root.join(".github/workflows/alpha.yml"))
            .unwrap()
            .contains("'my-custom-entry' '--literal' 'a b'")
    );
}

#[test]
fn migration_retires_renamed_units_and_rejects_host_edits_before_writes() {
    for conflict in ["none", "old-edit", "new-collision"] {
        let d = fixture();
        let root = d.path();
        let source = ".chrono-harness/ci/units.json";
        generate(root, source, false).unwrap();
        let prior = ".chrono-harness/state/previous-units.json";
        fs::create_dir_all(root.join(".chrono-harness/state")).unwrap();
        fs::copy(root.join(source), root.join(prior)).unwrap();
        let mut next = config_value();
        next["units"]["alpha"]["workflow_path"] = json!(".github/workflows/renamed.yml");
        next["units"]["beta"]["timeout_minutes"] = json!(23);
        json_file(root, source, &next);
        let source_bytes = fs::read(root.join(source)).unwrap();
        let beta = fs::read(root.join(".github/workflows/beta.yml")).unwrap();
        let alpha = fs::read(root.join(".github/workflows/alpha.yml")).unwrap();
        if conflict == "old-edit" {
            fs::write(
                root.join(".github/workflows/alpha.yml"),
                "host-edited original\n",
            )
            .unwrap();
        }
        if conflict == "new-collision" {
            fs::write(
                root.join(".github/workflows/renamed.yml"),
                "host-owned target\n",
            )
            .unwrap();
        }
        let result = migrate(root, prior, Some(source));
        if conflict != "none" {
            let error = result.unwrap_err();
            if conflict == "old-edit" {
                assert!(error.contains("previous projection differs"));
                assert!(!root.join(".github/workflows/renamed.yml").exists());
                assert_eq!(
                    fs::read_to_string(root.join(".github/workflows/alpha.yml")).unwrap(),
                    "host-edited original\n"
                );
            } else {
                assert!(error.contains("new workflow collision"));
                assert_eq!(
                    fs::read_to_string(root.join(".github/workflows/renamed.yml")).unwrap(),
                    "host-owned target\n"
                );
                assert_eq!(
                    fs::read(root.join(".github/workflows/alpha.yml")).unwrap(),
                    alpha
                );
            }
            assert_eq!(
                fs::read(root.join(".github/workflows/beta.yml")).unwrap(),
                beta
            );
        } else {
            let report: Value = serde_json::from_str(&result.unwrap()).unwrap();
            assert_eq!(
                report["retired_workflows"],
                json!([".github/workflows/alpha.yml"])
            );
            assert!(report["retired_source"].is_null());
            assert!(!root.join(".github/workflows/alpha.yml").exists());
            assert!(!generate(root, source, true).unwrap());
        }
        assert_eq!(fs::read(root.join(source)).unwrap(), source_bytes);
        assert!(root.join(prior).is_file());
    }
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
    let unit = context["scope"]["unit"].as_str();
    let report = short::published_report(root, unit, &output);
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
