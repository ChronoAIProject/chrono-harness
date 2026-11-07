//! Full transport shares the real bound-Git fixture with event acquisition tests.
#[path = "full_scope.rs"]
mod scope_tests;
use super::*;
use chrono_ci::full::{self, Config as FullConfig};

fn config() -> FullConfig {
    serde_json::from_value(json!({
        "schema":"chrono-github-full-ci/v2", "name":"Full context λ",
        "workflow_path":".github/workflows/full.yml", "runs_on":"ubuntu-24.04",
        "checkout_action":"actions/checkout@11d5960a326750d5838078e36cf38b85af677262",
        "upload_artifact_action":"actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02",
        "timeout_minutes":20, "bootstrap":["/bin/sh",".chrono-harness/bootstrap.sh"],
        "runner":".chrono-harness/bin/chrono-harness",
        "check_config":FACTS, "context_path":".chrono-harness/state/full/context.json",
        "preparation_path":".chrono-harness/state/full/preparation.json",
        "artifact_directory":".chrono-harness/state/full/"
    }))
    .unwrap()
}

#[test]
fn full_render_consumes_literal_argument_policy() {
    for value in ["", "普通文本", "$", "${", "$HOME `literal`", "${x{", "{{"] {
        let mut c = config();
        c.bootstrap.push(value.into());
        assert!(full::render(&c, SOURCE).is_ok(), "{value:?}");
    }
    for value in ["\0", "\n", "\r", "${{", "x${{y", "${${{"] {
        let mut c = config();
        c.bootstrap.push(value.into());
        assert_eq!(
            full::render(&c, SOURCE).unwrap_err(),
            "full CI settings must be literal single-line values",
            "{value:?}"
        );
    }
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
fn prepare(h: &Host, bytes: &[u8]) -> Result<Value, String> {
    full::prepare(&h.root, SOURCE, &config(), bytes, &h.candidate)
}
fn failure(error: &str) -> Value {
    serde_json::from_str(error.strip_prefix("E_FULL_CI_GIT: ").expect(error)).unwrap()
}

#[test]
fn preparation_preserves_context_bytes_and_separate_observations() {
    let h = host("pass");
    let bytes = context(&h, &h.candidate);
    let r = prepare(&h, &bytes).unwrap();
    assert_eq!(fs::read(h.root.join(config().context_path)).unwrap(), bytes);
    assert_eq!(r, read(&h.root, &config().preparation_path));
    assert_eq!(r["context"]["sha256"], sha256(&bytes));
    assert_eq!(r["canonical_argv"], json!(full::argv(&config())));
    assert_eq!(r["governance"], "not-evaluated");
    assert_eq!(r["parity"], "unestablished");
    assert!(r["git_facts"]["processes"].as_array().unwrap().len() > 5);
    assert!(!h.trace().contains(" fetch "));
    let again = prepare(&h, &bytes).unwrap();
    assert_eq!(again["context"], r["context"]);
    assert_eq!(again, read(&h.root, &config().preparation_path));
    assert_eq!(
        fs::read(h.root.join(r["context"]["path"].as_str().unwrap())).unwrap(),
        bytes
    );
}

#[cfg(unix)]
#[test]
fn repeated_preparation_replaces_current_views_and_preserves_original_contexts() {
    use std::os::fd::AsFd;
    let h = host("pass");
    let bytes = context(&h, &h.candidate);
    let first = prepare(&h, &bytes).unwrap();
    let original = first["context"]["path"].as_str().unwrap();
    let lease = tempfile::tempfile().unwrap();
    let _scope = chrono_harness::process_fds::Scope::new(&[lease.as_fd()]).unwrap();
    let second = prepare(&h, &bytes).unwrap();
    assert_eq!(first["context"], second["context"]);
    assert_ne!(
        first["git_facts"]["processes"][0]["ownership_fds"],
        second["git_facts"]["processes"][0]["ownership_fds"]
    );
    assert_ne!(
        first["git_facts"]["processes"][0]["child_pid"],
        second["git_facts"]["processes"][0]["child_pid"]
    );
    assert_eq!(second, read(&h.root, &config().preparation_path));
    let changed = [bytes.as_slice(), b" "].concat();
    let third = prepare(&h, &changed).unwrap();
    assert_ne!(third["context"]["path"], first["context"]["path"]);
    assert_eq!(fs::read(h.root.join(original)).unwrap(), bytes);
    assert_eq!(
        fs::read(h.root.join(third["context"]["path"].as_str().unwrap())).unwrap(),
        changed
    );
    assert_eq!(
        fs::read(h.root.join(config().context_path)).unwrap(),
        changed
    );
    assert_eq!(third, read(&h.root, &config().preparation_path));
}

#[test]
fn declaration_init_ownership_customization_and_verify_are_explicit() {
    let d = tempfile::tempdir().unwrap();
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
        ("generator", json!(".chrono-harness/bin/chrono-ci")),
        ("schema", json!("chrono-github-full-ci/v1")),
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
        let invalid_host = tempfile::tempdir().unwrap();
        write(invalid_host.path(), "invalid.json", &invalid);
        let rejected = cli(
            invalid_host.path(),
            &["init", "--host-root", ".", "--config", "invalid.json"],
        );
        assert!(!rejected.status.success(), "{key}");
        assert!(!invalid_host.path().join(adopted).exists());
        assert!(!invalid_host.path().join(&c.workflow_path).exists());
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
    let d = tempfile::tempdir().unwrap();
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
        full::argv(&c)[1..]
    );
    assert!(yaml.contains("ref: ${{ fromJSON(inputs.context).candidate }}"));
    assert!(yaml.contains("if: ${{ always() }}"));
    assert!(!yaml.contains("pull_request:") && !yaml.contains("  push:"));
}

#[test]
fn malformed_context_fails_without_invented_defaults() {
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
    // The next observation uses the fetched base and replaces the current view.
    let next = prepare(&h, &bytes).unwrap();
    assert!(
        !next["git_facts"]["processes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["argv"].as_array().unwrap().contains(&json!("fetch")))
    );
    assert_eq!(next["context"], r["context"]);
    assert_eq!(read(&h.root, &config().preparation_path), next);
    assert_eq!(
        fs::read(h.root.join(r["context"]["path"].as_str().unwrap())).unwrap(),
        bytes
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
fn context_and_report_symlinks_preserve_the_original_target() {
    for target in [config().context_path, config().preparation_path] {
        let h = host("pass");
        let path = h.root.join(&target);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = fs::read(&h.program).unwrap();
        std::os::unix::fs::symlink(&h.program, &path).unwrap();
        assert!(
            prepare(&h, &context(&h, &h.candidate))
                .unwrap_err()
                .contains("symlink")
        );
        assert_eq!(fs::read(&h.program).unwrap(), original);
        assert!(fs::symlink_metadata(path).unwrap().file_type().is_symlink());
    }
}

#[test]
fn committed_preparation_rejects_component_output_collisions_before_publication() {
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
        let error = full::prepare(&h.root, SOURCE, &c, &bytes, &h.candidate).unwrap_err();
        assert!(error.contains(expected), "{field}={path}: {error}");
        assert!(!h.root.join(&c.context_path).exists());
        assert!(!h.root.join(&c.preparation_path).exists());
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
fn current_source_rejects_component_ownership_before_publication() {
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
            "check_config",
            ".chrono-harness/bin//chrono-harness",
            "distinct ownership",
        ),
        (
            SOURCE,
            "check_config",
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
        let c: FullConfig = serde_json::from_value(source).unwrap();
        let error = full::prepare(
            &h.root,
            source_path,
            &c,
            &context(&h, &h.candidate),
            &h.candidate,
        )
        .unwrap_err();
        assert!(
            error.contains(expected),
            "{source_path}: {field}={value}: {error}"
        );
        assert!(!h.root.join(&c.context_path).exists());
        assert!(!h.root.join(&c.preparation_path).exists());
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
fn committed_preparation_preserves_separate_component_spellings_and_customization() {
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
        let report = full::prepare(&h.root, SOURCE, &c, &bytes, &h.candidate).unwrap();
        assert_eq!(report["status"], "prepared");
        assert_eq!(report["canonical_argv"], json!(full::argv(&c)));
        assert_eq!(report["context"]["current_path"], c.context_path);
        assert_eq!(
            fs::read(h.root.join(report["context"]["path"].as_str().unwrap())).unwrap(),
            bytes
        );
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

#[test]
fn standalone_full_prepare_is_rejected_without_publishing_or_git() {
    for scope in [
        None,
        Some(json!({"kind":"unit","unit":"compiler"})),
        Some(json!({"kind":"collect","manifest":".chrono-harness/state/manifest.json"})),
    ] {
        let mut h = host("pass");
        let mut c = config();
        c.scope = scope.map(|value| serde_json::from_value(value).unwrap());
        write(&h.root, SOURCE, &serde_json::to_value(&c).unwrap());
        generate(&h.root, SOURCE, false).unwrap();
        h.revise();
        let payload = h.root.join(".chrono-harness/state/payload.json");
        write(
            &h.root,
            ".chrono-harness/state/payload.json",
            &json!({"inputs":{"context":String::from_utf8(context(&h, &h.candidate)).unwrap()}}),
        );
        let output = h.root.join(".chrono-harness/state/github-output");
        fs::write(&output, b"caller-owned output\n").unwrap();
        for event in ["workflow_dispatch", "push"] {
            let out = cli(
                h.root.parent().unwrap(),
                &[
                    "prepare",
                    "--host-root",
                    h.root.file_name().unwrap().to_str().unwrap(),
                    "--config",
                    SOURCE,
                    "--event",
                    event,
                    "--payload",
                    payload.to_str().unwrap(),
                    "--workflow-revision",
                    &h.candidate,
                    "--github-output",
                    output.to_str().unwrap(),
                ],
            );
            assert_eq!(out.status.code(), Some(1), "{out:?}");
            assert!(!out.stderr.is_empty(), "{out:?}");
            assert!(out.stdout.is_empty());
            assert!(!h.root.join(&c.context_path).exists());
            assert!(!h.root.join(&c.preparation_path).exists());
            assert_eq!(fs::read(&output).unwrap(), b"caller-owned output\n");
            assert!(h.trace().is_empty());
        }
    }
}
