//! Full transport shares the real bound-Git fixture with event acquisition tests.
#[path = "full_scope.rs"]
mod scope_tests;
use super::*;
use chrono_ci::full::{self, Config as FullConfig};

fn config() -> FullConfig {
    serde_json::from_value(json!({
        "schema":"chrono-github-full-ci/v1", "name":"Full context λ",
        "workflow_path":".github/workflows/full.yml", "runs_on":"ubuntu-24.04",
        "checkout_action":"actions/checkout@11d5960a326750d5838078e36cf38b85af677262",
        "upload_artifact_action":"actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02",
        "timeout_minutes":20, "bootstrap":["/bin/sh",".chrono-harness/bootstrap.sh"],
        "runner":".chrono-harness/bin/chrono-harness", "generator":".chrono-harness/bin/chrono-ci",
        "check_config":FACTS, "context_path":".chrono-harness/state/full/context.json",
        "preparation_path":".chrono-harness/state/full/preparation.json",
        "artifact_directory":".chrono-harness/state/full/"
    }))
    .unwrap()
}
fn host(mode: &str) -> Host {
    let mut h = Host::new(mode);
    let c = config();
    write(&h.root, SOURCE, &serde_json::to_value(&c).unwrap());
    fs::write(h.root.join(".gitignore"), ".chrono-harness/state/\n").unwrap();
    generate(&h.root, SOURCE, false).unwrap();
    h.revise();
    h
}
fn context(h: &Host, base: &str) -> Vec<u8> {
    // Noncanonical whitespace, Unicode and shell-looking strings are input bytes.
    format!(" {{\n\t\"schema_version\": 2, \"run_kind\": \"integration\", \"base\": \"{base}\", \"candidate\": \"{}\",\n\"branch_started_at\": \"2001-02-03T04:05:06Z\", \"note\": \"λ $HOME `literal`\" }}\n",h.candidate).into_bytes()
}
fn cli(root: &Path, args: &[&str]) -> std::process::Output {
    Command::new(Path::new(env!("CARGO_MANIFEST_DIR")).join("../ci/target/debug/chrono-ci"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap()
}
fn prepare_cli(h: &Host, bytes: &[u8], event: &str) -> std::process::Output {
    prepare_cli_from(h, SOURCE, bytes, event)
}
fn prepare_cli_from(h: &Host, source: &str, bytes: &[u8], event: &str) -> std::process::Output {
    let payload = h.root.join(".chrono-harness/state/payload.json");
    write(
        &h.root,
        ".chrono-harness/state/payload.json",
        &json!({"inputs":{"context":String::from_utf8(bytes.to_vec()).unwrap()}}),
    );
    let output = h.root.join(".chrono-harness/state/github-output");
    fs::write(&output, "").unwrap();
    cli(
        h.root.parent().unwrap(),
        &[
            "prepare",
            "--host-root",
            h.root.file_name().unwrap().to_str().unwrap(),
            "--config",
            source,
            "--event",
            event,
            "--payload",
            payload.to_str().unwrap(),
            "--workflow-revision",
            &h.candidate,
            "--github-output",
            output.to_str().unwrap(),
        ],
    )
}
fn prepare(h: &Host, bytes: &[u8]) -> Result<Value, String> {
    full::prepare(&h.root, SOURCE, &config(), bytes, &h.candidate)
}
fn failure(error: &str) -> Value {
    serde_json::from_str(error.strip_prefix("E_FULL_CI_GIT: ").expect(error)).unwrap()
}

#[test]
fn cli_preserves_context_bytes_and_separate_observations_from_another_cwd() {
    let h = host("pass");
    let bytes = context(&h, &h.candidate);
    let out = prepare_cli(&h, &bytes, "workflow_dispatch");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(fs::read(h.root.join(config().context_path)).unwrap(), bytes);
    assert_eq!(r, read(&h.root, &config().preparation_path));
    assert_eq!(r["context"]["input_bytes"], json!(bytes));
    assert_eq!(r["context"]["sha256"], sha256(&bytes));
    assert_eq!(
        r["canonical_argv"],
        json!(full::argv(&config(), &h.candidate, &h.candidate))
    );
    assert_eq!(r["governance"], "not-evaluated");
    assert_eq!(r["parity"], "unestablished");
    assert_eq!(
        fs::read_to_string(h.root.join(".chrono-harness/state/github-output")).unwrap(),
        format!("base={}\ncandidate={}\n", h.candidate, h.candidate)
    );
    assert!(r["git_facts"]["processes"].as_array().unwrap().len() > 5);
    assert!(!h.trace().contains(" fetch "));
    let again = prepare(&h, &bytes).unwrap();
    assert_eq!(again, r);
}

#[cfg(unix)]
#[test]
fn repeated_preparation_retains_distinct_ownership_observations_without_rebinding_inputs() {
    use std::os::fd::AsFd;
    let h = host("pass");
    let bytes = context(&h, &h.candidate);
    let first = prepare(&h, &bytes).unwrap();
    let original = fs::read(h.root.join(config().preparation_path)).unwrap();
    let lease = tempfile::tempfile().unwrap();
    let _scope = chrono_harness::process_fds::Scope::new(&[lease.as_fd()]).unwrap();
    assert_eq!(prepare(&h, &bytes).unwrap(), first);
    assert_eq!(
        fs::read(h.root.join(config().preparation_path)).unwrap(),
        original
    );
    let attempts: Vec<_> = fs::read_dir(
        h.root
            .join(format!("{}preparation", config().artifact_directory)),
    )
    .unwrap()
    .map(|entry| entry.unwrap().path())
    .filter(|path| {
        path.file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("native-attempt-")
    })
    .collect();
    assert_eq!(attempts.len(), 1);
    let retained = fs::read(&attempts[0]).unwrap();
    assert!(
        attempts[0]
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains(&sha256(&retained))
    );
    let actual: Value = serde_json::from_slice(&retained).unwrap();
    assert_ne!(
        actual["git_facts"]["processes"][0]["ownership_fds"],
        first["git_facts"]["processes"][0]["ownership_fds"]
    );
    assert_ne!(
        actual["git_facts"]["processes"][0]["child_pid"],
        first["git_facts"]["processes"][0]["child_pid"]
    );
    let mut stable = actual.clone();
    let mut previous = first.clone();
    for report in [&mut stable, &mut previous] {
        for process in report["git_facts"]["processes"].as_array_mut().unwrap() {
            process.as_object_mut().unwrap().remove("ownership_fds");
            process.as_object_mut().unwrap().remove("launcher_pid");
            process.as_object_mut().unwrap().remove("child_pid");
        }
    }
    assert_eq!(stable, previous);
    let mut changed = first;
    changed["git_facts"]["processes"][0]["exit_code"] = json!(47);
    write(&h.root, &config().preparation_path, &changed);
    let changed_bytes = fs::read(h.root.join(config().preparation_path)).unwrap();
    assert!(
        prepare(&h, &bytes)
            .unwrap_err()
            .contains("output collision")
    );
    assert_eq!(
        fs::read(h.root.join(config().preparation_path)).unwrap(),
        changed_bytes
    );
    assert_eq!(fs::read(attempts[0].as_path()).unwrap(), retained);
    fs::write(h.root.join(config().preparation_path), b"42").unwrap();
    assert!(
        prepare(&h, &bytes)
            .unwrap_err()
            .contains("output collision")
    );
    assert_eq!(
        fs::read(h.root.join(config().preparation_path)).unwrap(),
        b"42"
    );
}

#[test]
fn declaration_init_ownership_customization_and_verify_are_explicit() {
    let d = crate::tools::temporary_host("host λ ");
    let root = d.path();
    let c = config();
    write(root, "incoming.json", &serde_json::to_value(&c).unwrap());
    let invoke = |op: &str, input: &str| cli(root, &[op, "--host-root", ".", "--config", input]);
    assert!(invoke("init", "incoming.json").status.success());
    let adopted = ".chrono-harness/ci/full.json";
    let mut customized = read(root, adopted);
    customized["name"] = json!("customized");
    write(root, adopted, &customized);
    assert!(invoke("init", "incoming.json").status.success());
    assert_eq!(read(root, adopted), customized);
    assert!(invoke("verify", adopted).status.success());
    let original = fs::read(root.join(&c.workflow_path)).unwrap();
    fs::write(
        root.join(&c.workflow_path),
        [original.as_slice(), b"# drift\n"].concat(),
    )
    .unwrap();
    assert!(!invoke("verify", adopted).status.success());
    assert!(invoke("generate", adopted).status.success());
    assert_eq!(fs::read(root.join(&c.workflow_path)).unwrap(), original);
    fs::write(root.join(&c.workflow_path), "unowned\n").unwrap();
    assert!(!invoke("generate", adopted).status.success());
    fs::remove_file(root.join(&c.workflow_path)).unwrap();
    std::os::unix::fs::symlink(root.join("incoming.json"), root.join(&c.workflow_path)).unwrap();
    assert!(!invoke("generate", adopted).status.success());
    for (key, value) in [
        ("unknown", json!(true)),
        ("schema", json!("chrono-github-full-ci/v2")),
        ("check_config", json!(".chrono-harness/state/policy.json")),
        ("context_path", json!(c.preparation_path)),
        ("preparation_path", json!(c.context_path)),
        ("bootstrap", json!(["${{ inputs.command }}"])),
        ("runs_on", json!("${{ inputs.runner }}")),
        ("runner", json!("../outside")),
        ("timeout_minutes", json!(0)),
        ("artifact_directory", json!(".chrono-harness/state/*/")),
        ("checkout_action", json!("actions/checkout@main")),
    ] {
        let mut invalid = serde_json::to_value(&c).unwrap();
        invalid[key] = value;
        write(root, "invalid.json", &invalid);
        assert!(!invoke("init", "invalid.json").status.success(), "{key}");
    }
}

fn script(workflow: &str, name: &str) -> String {
    workflow
        .split(&format!("      - name: {name}\n"))
        .nth(1)
        .unwrap()
        .split("        run: |\n")
        .nth(1)
        .unwrap()
        .lines()
        .take_while(|l| l.starts_with("          "))
        .map(|l| &l[10..])
        .collect::<Vec<_>>()
        .join("\n")
}

fn executable_alias(source: impl AsRef<Path>, destination: impl AsRef<Path>) {
    // Execute the completed build through its fixture name without reopening
    // executable bytes for writing while other test threads launch children.
    std::os::unix::fs::symlink(fs::canonicalize(source).unwrap(), destination).unwrap();
}

#[test]
fn generated_bash_preserves_literal_arguments_and_original_exit() {
    let d = crate::tools::temporary_host("host λ ");
    let root = d.path();
    let mut c = config();
    c.runner = "$CHRONO_BASE".into();
    executable_alias(
        env!("CARGO_BIN_EXE_chrono-ci-test-transport"),
        root.join(&c.runner),
    );
    c.check_config = ".chrono-harness/'$HOME`literal`.json".into();
    let yaml = full::render(&c, SOURCE).unwrap();
    let out = Command::new("/bin/bash")
        .current_dir(root)
        .env("PATH", root)
        .env("CHRONO_TEST_RECORDING", "full")
        .env("CHRONO_BASE", "base fixed")
        .env("CHRONO_CANDIDATE", "candidate fixed")
        .args(["-e", "-c", &script(&yaml, "Canonical full harness check")])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(23), "{out:?}");
    assert_eq!(out.stderr, b"original-failure\n");
    assert_eq!(
        String::from_utf8(out.stdout)
            .unwrap()
            .lines()
            .collect::<Vec<_>>(),
        full::argv(&c, "base fixed", "candidate fixed")[1..]
    );
    assert!(yaml.contains("ref: ${{ fromJSON(inputs.context).candidate }}"));
    assert!(yaml.contains("if: ${{ always() }}"));
    assert!(!yaml.contains("pull_request:") && !yaml.contains("  push:"));
}

#[test]
fn generated_preparation_step_keeps_original_failure_in_artifacts() {
    let h = host("pass");
    let c = config();
    fs::create_dir_all(h.root.join(".chrono-harness/bin")).unwrap();
    executable_alias(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../ci/target/debug/chrono-ci"),
        h.root.join(&c.generator),
    );
    let payload = h.root.join(".chrono-harness/state/payload.json");
    write(
        &h.root,
        ".chrono-harness/state/payload.json",
        &json!({"inputs":{"context":"{}"}}),
    );
    let run = || {
        Command::new("/bin/bash")
            .current_dir(&h.root)
            .env("GITHUB_EVENT_NAME", "workflow_dispatch")
            .env("GITHUB_EVENT_PATH", &payload)
            .env("CHRONO_WORKFLOW_REVISION", &h.candidate)
            .env(
                "GITHUB_OUTPUT",
                h.root.join(".chrono-harness/state/github-output"),
            )
            .args([
                "-e",
                "-c",
                &script(
                    &full::render(&c, SOURCE).unwrap(),
                    "Preserve fixed full context",
                ),
            ])
            .output()
            .unwrap()
    };
    let out = run();
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty() && out.stderr.is_empty());
    assert!(
        fs::read(
            h.root
                .join(format!("{}prepare.stdout.json", c.artifact_directory))
        )
        .unwrap()
        .is_empty()
    );
    assert!(
        fs::read_to_string(
            h.root
                .join(format!("{}prepare.stderr", c.artifact_directory))
        )
        .unwrap()
        .contains("explicit context v2")
    );
    assert!(!h.root.join(&c.context_path).exists());
    let failure_path = h
        .root
        .join(format!("{}prepare.stderr", c.artifact_directory));
    let original = fs::read(&failure_path).unwrap();
    let retry = run();
    assert!(!retry.status.success());
    assert_eq!(fs::read(failure_path).unwrap(), original);
    assert!(String::from_utf8_lossy(&retry.stderr).contains("cannot overwrite existing file"));
    let mut conflicting = c;
    conflicting.context_path = format!("{}prepare.stderr", conflicting.artifact_directory);
    assert!(full::render(&conflicting, SOURCE).is_err());
}

#[test]
fn malformed_context_and_events_fail_without_invented_defaults() {
    let h = host("pass");
    let original = context(&h, &h.candidate);
    for (key, value) in [
        ("schema_version", json!(1)),
        ("run_kind", json!(null)),
        ("run_kind", json!("feature")),
        ("base", json!("dev")),
        ("candidate", json!("0".repeat(40))),
    ] {
        let mut bad: Value = serde_json::from_slice(&original).unwrap();
        bad[key] = value;
        assert!(
            prepare(&h, &serde_json::to_vec(&bad).unwrap()).is_err(),
            "{key}"
        );
    }
    assert!(prepare(&h, b"{}").is_err());
    assert!(prepare(&h, b"not json").is_err());
    assert!(!prepare_cli(&h, &original, "push").status.success());
    assert!(h.trace().is_empty());
    assert!(!h.root.join(config().context_path).exists());
}

#[test]
fn candidate_source_policy_checkout_and_workflow_are_bound_before_publication() {
    for case in [
        "policy", "source", "checkout", "workflow", "digest", "version", "legacy",
    ] {
        let mut h = host("pass");
        if case == "policy" {
            let mut v = read(&h.root, FACTS);
            v["protocol"]["timeout_seconds"] = json!(11);
            write(&h.root, FACTS, &v);
        }
        if case == "source" {
            let mut v = read(&h.root, SOURCE);
            v["name"] = json!("drift");
            write(&h.root, SOURCE, &v);
        }
        if case == "checkout" {
            fs::write(h.root.join("later"), "later").unwrap();
            commit(&h.root, false);
        }
        if case == "workflow" {
            fs::write(h.root.join(config().workflow_path), "different bytes").unwrap();
            h.revise();
        }
        if matches!(case, "digest" | "version" | "legacy") {
            let mut v = read(&h.root, FACTS);
            match case {
                "digest" => v["environment"]["inputs"][0]["sha256"] = json!("0".repeat(64)),
                "version" => v["tools"][0]["expected_version"] = json!("impossible"),
                _ => {
                    v["schema_version"] = json!(2);
                    v.as_object_mut().unwrap().remove("facts_git");
                }
            }
            write(&h.root, FACTS, &v);
            h.revise();
        }
        let c: FullConfig = serde_json::from_value(read(&h.root, SOURCE)).unwrap();
        let error = full::prepare(
            &h.root,
            SOURCE,
            &c,
            &context(&h, &h.candidate),
            &h.candidate,
        )
        .unwrap_err();
        let expected = match case {
            "policy" => "config differs from fixed candidate",
            "source" => "source differs from fixed candidate",
            "checkout" => "checkout is not supplied candidate",
            "workflow" => "workflow revision differs",
            "digest" => "declared digest mismatch",
            "version" => "version mismatch",
            _ => "requires candidate full config v3",
        };
        assert!(error.contains(expected), "{case}: {error}");
        assert!(!h.root.join(c.context_path).exists());
        assert!(!h.trace().contains(" fetch "));
    }
}

#[test]
fn missing_fixed_base_is_fetched_once_and_original_failures_are_retained() {
    let h = host("pass");
    let (_remote, base) = h.remote();
    let bytes = context(&h, &base);
    let r = prepare(&h, &bytes).unwrap();
    assert!(
        r["git_facts"]["processes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["argv"].as_array().unwrap().contains(&json!("fetch")))
    );
    assert_eq!(git(&h.root, &["rev-parse", &base]), base);
    // Fetch changes the observations. Never overwrite the original preparation report.
    let saved = fs::read(h.root.join(config().preparation_path)).unwrap();
    assert!(
        prepare(&h, &bytes)
            .unwrap_err()
            .contains("output collision")
    );
    assert_eq!(
        fs::read(h.root.join(config().preparation_path)).unwrap(),
        saved
    );
    let mut distinct = host("pass");
    let workflow = distinct.candidate.clone();
    fs::write(
        distinct.root.join("later.txt"),
        "same declared workflow in a new candidate",
    )
    .unwrap();
    distinct.candidate = commit(&distinct.root, false);
    let distinct_report = full::prepare(
        &distinct.root,
        SOURCE,
        &config(),
        &context(&distinct, &workflow),
        &workflow,
    )
    .unwrap();
    assert_eq!(distinct_report["workflow_source_revision"], workflow);
    assert_ne!(
        distinct_report["candidate"],
        distinct_report["workflow_source_revision"]
    );
    for mode in ["full-probe-failure", "full-fetch-failure"] {
        let h = host(mode);
        let (_remote, base) = h.remote();
        let error = prepare(&h, &context(&h, &base)).unwrap_err();
        let e = failure(&error);
        let failed = e["git_facts"]["processes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["exit_code"] == 29 || p["exit_code"] == 31)
            .unwrap();
        let failed = chrono_harness::full::expand_process(failed).unwrap();
        let b: Vec<u8> = serde_json::from_value(failed["stderr_bytes"].clone()).unwrap();
        assert!(b[0] >= 254);
        assert_eq!(failed["stderr_sha256"], sha256(&b));
        assert!(!h.root.join(config().context_path).exists());
        if failed["exit_code"] == 29 {
            assert!(!h.trace().contains(" fetch "));
        }
    }
}

#[test]
fn context_report_and_symlink_collisions_preserve_original_bytes() {
    for target in [config().context_path, config().preparation_path] {
        let h = host("pass");
        let path = h.root.join(&target);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"old evidence").unwrap();
        assert!(
            prepare(&h, &context(&h, &h.candidate))
                .unwrap_err()
                .contains("output collision")
        );
        assert_eq!(fs::read(&path).unwrap(), b"old evidence");
        if target == config().preparation_path {
            assert!(!h.root.join(config().context_path).exists());
        }
        fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(&h.program, &path).unwrap();
        assert!(prepare(&h, &context(&h, &h.candidate)).is_err());
    }
}

#[test]
fn committed_cli_rejects_component_output_collisions_before_publication() {
    for (field, path, expected) in [
        (
            "preparation_path",
            ".chrono-harness/state/full//context.json",
            "distinct ownership",
        ),
        (
            "preparation_path",
            ".chrono-harness/state/full/./context.json/child",
            "distinct ownership",
        ),
        (
            "preparation_path",
            ".chrono-harness/state/full/context.json/",
            "distinct ownership",
        ),
        (
            "preparation_path",
            ".chrono-harness/state/full//prepare.stdout.json",
            "preparation process output",
        ),
        (
            "preparation_path",
            ".chrono-harness/state/full/./prepare.stderr",
            "preparation process output",
        ),
        (
            "context_path",
            ".chrono-harness/state/full/./prepare.stdout.json",
            "preparation process output",
        ),
        (
            "context_path",
            ".chrono-harness/state/full//prepare.stderr/child",
            "preparation process output",
        ),
    ] {
        let mut h = host("pass");
        let original_workflow = fs::read(h.root.join(config().workflow_path)).unwrap();
        let mut source = read(&h.root, SOURCE);
        source[field] = json!(path);
        write(&h.root, SOURCE, &source);
        // Preparation paths are absent from the projection: the A1 fixture retains
        // its real generated workflow and commits the formerly accepted source.
        h.revise();
        let c: FullConfig = serde_json::from_value(source).unwrap();
        let source_bytes = fs::read(h.root.join(SOURCE)).unwrap();
        let unrelated = h.root.join(".chrono-harness/state/full/keep.log");
        fs::create_dir_all(unrelated.parent().unwrap()).unwrap();
        fs::write(&unrelated, b"unrelated retained evidence").unwrap();
        let bytes = context(&h, &h.candidate);
        let out = prepare_cli(&h, &bytes, "workflow_dispatch");
        let published = fs::read(h.root.join(&c.context_path)).ok();
        let report = serde_json::from_slice::<Value>(&out.stdout).ok();
        assert!(
            !out.status.success(),
            "{field}={path}: CLI exit={:?}; reported_status={:?}; context_preserved={}; context_replaced_by_report={}",
            out.status.code(),
            report.as_ref().map(|r| &r["status"]),
            published.as_deref() == Some(bytes.as_slice()),
            published
                .as_ref()
                .and_then(|b| serde_json::from_slice::<Value>(b).ok())
                .is_some_and(|v| v["schema"] == "chrono-full-ci-inputs/v1")
        );
        assert_eq!(out.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&out.stderr).contains(expected));
        assert!(out.stdout.is_empty());
        assert!(!h.root.join(&c.context_path).exists());
        assert!(!h.root.join(&c.preparation_path).exists());
        assert!(
            fs::read(h.root.join(".chrono-harness/state/github-output"))
                .unwrap()
                .is_empty()
        );
        assert_eq!(fs::read(unrelated).unwrap(), b"unrelated retained evidence");
        assert_eq!(fs::read(h.root.join(SOURCE)).unwrap(), source_bytes);
        assert_eq!(
            fs::read(h.root.join(&c.workflow_path)).unwrap(),
            original_workflow
        );
        assert!(h.trace().is_empty());
    }
}

#[test]
fn current_source_cli_rejects_component_ownership_before_publication() {
    for (source_path, field, value, expected) in [
        (
            SOURCE,
            "runner",
            ".chrono-harness/ci//github.json",
            "separate host policy input",
        ),
        (
            SOURCE,
            "runner",
            ".chrono-harness/ci/./github.json/child",
            "separate host policy input",
        ),
        (
            SOURCE,
            "runner",
            ".chrono-harness//ci",
            "separate host policy input",
        ),
        (
            ".chrono-harness//state/provider.json",
            "name",
            "current source",
            "separate host policy input",
        ),
        (
            ".chrono-harness/./state/provider.json",
            "name",
            "current source",
            "separate host policy input",
        ),
        (
            SOURCE,
            "generator",
            ".chrono-harness/bin//chrono-harness",
            "distinct ownership",
        ),
        (
            SOURCE,
            "generator",
            ".chrono-harness/bin/./chrono-harness/child",
            "distinct ownership",
        ),
        (
            SOURCE,
            "check_config",
            ".chrono-harness/./state/policy.json",
            "policy/artifact ownership",
        ),
        (
            SOURCE,
            "check_config",
            ".chrono-harness/state",
            "distinct ownership",
        ),
        (
            SOURCE,
            "runner",
            ".chrono-harness/state/./full/tool",
            "policy/artifact ownership",
        ),
        (
            SOURCE,
            "context_path",
            ".chrono-harness/state/fullish/context.json",
            "policy/artifact ownership",
        ),
        (
            SOURCE,
            "context_path",
            ".chrono-harness/state/full",
            "distinct ownership",
        ),
    ] {
        let h = host("pass");
        let mut source = read(&h.root, SOURCE);
        source[field] = json!(value);
        write(&h.root, source_path, &source);
        let source_bytes = fs::read(h.root.join(source_path)).unwrap();
        let workflow_bytes = fs::read(h.root.join(config().workflow_path)).unwrap();
        let out = prepare_cli_from(
            &h,
            source_path,
            &context(&h, &h.candidate),
            "workflow_dispatch",
        );
        assert_eq!(out.status.code(), Some(1), "{source_path}: {field}={value}");
        assert!(
            String::from_utf8_lossy(&out.stderr).contains(expected),
            "{source_path}: {field}={value}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(out.stdout.is_empty());
        let c: FullConfig = serde_json::from_value(source).unwrap();
        assert!(!h.root.join(&c.context_path).exists());
        assert!(!h.root.join(&c.preparation_path).exists());
        assert!(
            fs::read(h.root.join(".chrono-harness/state/github-output"))
                .unwrap()
                .is_empty()
        );
        assert_eq!(fs::read(h.root.join(source_path)).unwrap(), source_bytes);
        assert_eq!(
            fs::read(h.root.join(&c.workflow_path)).unwrap(),
            workflow_bytes
        );
        assert!(h.trace().is_empty());
        let generated = cli(
            &h.root,
            &["generate", "--host-root", ".", "--config", source_path],
        );
        assert_eq!(generated.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&generated.stderr).contains(expected));
        assert_eq!(
            fs::read(h.root.join(&c.workflow_path)).unwrap(),
            workflow_bytes
        );
    }
}

#[test]
fn committed_cli_preserves_separate_component_spellings_and_customization() {
    for scope in [
        None,
        Some(json!({"kind":"unit","unit":"compiler.arm64-1"})),
        Some(
            json!({"kind":"collect","manifest":".chrono-harness/state/full/./context.json.distinct"}),
        ),
    ] {
        let mut h = host("pass");
        let mut c = config();
        c.scope = scope.map(|value| serde_json::from_value(value).unwrap());
        c.name = "host custom λ".into();
        c.runs_on = "macos-14".into();
        c.bootstrap.push("host custom argument".into());
        c.artifact_directory = ".chrono-harness/state/./full//".into();
        c.context_path = ".chrono-harness/state/full/./context.json".into();
        c.preparation_path = ".chrono-harness/state/full//preparation.json".into();
        let source_bytes = [
            b" \n".as_slice(),
            &serde_json::to_vec_pretty(&c).unwrap(),
            b"\n\t",
        ]
        .concat();
        fs::write(h.root.join(SOURCE), &source_bytes).unwrap();
        generate(&h.root, SOURCE, false).unwrap();
        h.revise();
        let unrelated = h.root.join(".github/workflows/unrelated.yml");
        fs::write(&unrelated, b"host-owned independent workflow").unwrap();
        let manifest = match &c.scope {
            Some(chrono_harness::units::Scope::Collect { manifest }) => Some(manifest),
            _ => None,
        };
        if let Some(manifest) = manifest {
            fs::create_dir_all(h.root.join(manifest).parent().unwrap()).unwrap();
            fs::write(h.root.join(manifest), b"caller-owned collection input").unwrap();
        }
        let bytes = context(&h, &h.candidate);
        let out = prepare_cli(&h, &bytes, "workflow_dispatch");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let report: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(report["status"], "prepared");
        assert_eq!(
            report["canonical_argv"],
            json!(full::argv(&c, &h.candidate, &h.candidate))
        );
        assert_eq!(report["context"]["path"], c.context_path);
        assert_eq!(report["context"]["input_bytes"], json!(bytes));
        assert_eq!(report["config_sha256"], sha256(&source_bytes));
        assert_eq!(fs::read(h.root.join(&c.context_path)).unwrap(), bytes);
        assert_eq!(report, read(&h.root, &c.preparation_path));
        assert_eq!(fs::read(h.root.join(SOURCE)).unwrap(), source_bytes);
        assert_eq!(
            fs::read(unrelated).unwrap(),
            b"host-owned independent workflow"
        );
        if let Some(manifest) = manifest {
            assert_eq!(
                fs::read(h.root.join(manifest)).unwrap(),
                b"caller-owned collection input"
            );
        }
        assert!(!generate(&h.root, SOURCE, true).unwrap());
        assert_eq!(
            fs::read_to_string(h.root.join(&c.workflow_path)).unwrap(),
            full::render(&c, SOURCE).unwrap()
        );
    }
}
