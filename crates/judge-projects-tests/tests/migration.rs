#[path = "../../judge-filemap-tests/tests/support/mod.rs"]
mod support;
use chrono_harness::{facts, observation, wire};
use chrono_judge_registration::{execution, interpret};
use chrono_judge_routes::{execute_actions, prepare_scoped};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    process::Command,
};
use support::*;
fn adopt_fixture_tool(root: &std::path::Path, config: &mut Value, id: &str) -> observation::Tool {
    let tool = config["tools"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|tool| tool["id"] == id)
        .unwrap();
    let env = std::env::vars().collect();
    let observed = observation::tool(
        root,
        tool["program"].as_str().unwrap(),
        &serde_json::from_value::<Vec<String>>(tool["version_argv"].clone()).unwrap(),
        &env,
        30,
        1048576,
    )
    .unwrap_or_else(|error| panic!("observe fixture {id}: {error}"));
    observation::process_success(&observed.version)
        .unwrap_or_else(|error| panic!("observe fixture {id}: {error}; {observed:?}"));
    tool["program"] = json!(observed.path);
    tool["expected_version"] = json!(observed.version.stdout.trim_end());
    observed
}
fn adopt_fixture_bindings(root: &std::path::Path, config: &mut Value) {
    adopt_fixture_tool(root, config, "python3");
    let git_id = config["facts_git"]["tool"].as_str().unwrap().to_owned();
    let git_input = config["facts_git"]["input"].as_str().unwrap().to_owned();
    let git = adopt_fixture_tool(root, config, &git_id);
    let input = config["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|input| input["id"] == git_input)
        .unwrap();
    input["location"] = json!(git.path);
    input["sha256"] = json!(git.sha256);
}
fn migration_host() -> (tempfile::TempDir, std::path::PathBuf, String, String) {
    migration_host_with_alias_collision(false)
}
fn copy_nested_receipts(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    let kind = fs::symlink_metadata(from)?.file_type();
    if kind.is_file() {
        fs::copy(from, to)?;
    } else if kind.is_dir() {
        fs::create_dir(to)?;
        for entry in fs::read_dir(from)? {
            let entry = entry?;
            copy_nested_receipts(&entry.path(), &to.join(entry.file_name()))?;
        }
    } else {
        return Err(std::io::Error::other(format!(
            "unsupported nested receipt entry: {}",
            from.display()
        )));
    }
    Ok(())
}
fn execution_failure(
    root: &std::path::Path,
    reason: &str,
    result: &chrono_judge_projects::Results,
) -> String {
    let directory = source().join(".chrono-harness/state/migration-tests");
    let retained = (|| -> Result<String, Box<dyn std::error::Error>> {
        fs::create_dir_all(&directory)?;
        let (mut file, path) = tempfile::Builder::new()
            .prefix("unexpected-execution-")
            .suffix(".json")
            .tempfile_in(&directory)?
            .keep()?;
        let mut summary = match serde_json::to_writer(&mut file, result) {
            Ok(()) => format!("original execution retained at {}", path.display()),
            Err(error) => format!(
                "original execution retention incomplete at {}: {error}",
                path.display()
            ),
        };
        // The inner CI owner has already recorded its real process result and
        // inputs. Preserve those bytes before this temporary host is dropped.
        let nested = root.join(".chrono-harness/state/ci-test-failures");
        let destination = path.with_extension("ci-test-failures");
        let evidence = match copy_nested_receipts(&nested, &destination) {
            Ok(()) => format!("nested CI evidence retained at {}", destination.display()),
            Err(error) => format!(
                "nested CI evidence retention incomplete at {}: {error}",
                destination.display()
            ),
        };
        summary.push_str(&format!("; {evidence}"));
        Ok(summary)
    })()
    .unwrap_or_else(|error| format!("original execution retention failed: {error}"));
    let mut summary = format!("{reason}; {retained}");
    for row in result.executed.iter().chain(&result.blocked) {
        if row.status == "passed" {
            continue;
        }
        let error: String = row
            .error
            .as_deref()
            .unwrap_or("")
            .chars()
            .take(240)
            .collect();
        summary.push_str(&format!("\n{}: {} {error}", row.operation, row.status));
        if let Some(receipt) = &row.receipt {
            summary.push_str(&format!(
                " (exit {}, stdout {} bytes, stderr {} bytes)",
                receipt.process.exit_code,
                receipt.process.stdout_bytes.len(),
                receipt.process.stderr_bytes.len()
            ));
        }
    }
    summary
}
fn migration_host_with_alias_collision(
    collision: bool,
) -> (tempfile::TempDir, std::path::PathBuf, String, String) {
    let source = fs::canonicalize(source()).unwrap();
    let original = "4f08aef7ab40d7b0a3fb6ba42af2620600f18d98";
    let dir = tempfile::Builder::new()
        .prefix("migration actual host ")
        .tempdir()
        .unwrap();
    let root = fs::canonicalize(dir.path()).unwrap();
    let tree = facts::tree(&source, original).unwrap();
    facts::export(&source, original, &tree, &root).unwrap();
    if collision {
        // A controlled regression input, committed in this temporary host before
        // decoding, preserves both real original definitions under one alias.
        let mut projects: Value =
            serde_json::from_slice(&fs::read(root.join(PROJECTS)).unwrap()).unwrap();
        let mut project = projects["projects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["kind"] == "test")
            .unwrap()
            .clone();
        project["id"] = json!("ci-verify");
        project["actions"] = json!({"execute":{"operation":"collision.execute","tool":"cargo","argv":["--version"]}});
        projects["projects"].as_array_mut().unwrap().push(project);
        let mut permissions = fs::metadata(root.join(PROJECTS)).unwrap().permissions();
        permissions.set_readonly(false);
        fs::set_permissions(root.join(PROJECTS), permissions).unwrap();
        fs::write(
            root.join(PROJECTS),
            serde_json::to_vec_pretty(&projects).unwrap(),
        )
        .unwrap();
    }
    git(&root, &["init", "-q"]);
    let base = commit(&root);
    let map: Value = serde_json::from_slice(&fs::read(source.join(FM)).unwrap()).unwrap();
    let historical: Value = serde_json::from_slice(&fs::read(root.join(FM)).unwrap()).unwrap();
    let current_paths: BTreeSet<_> = map["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    // Overlaying the current candidate must include its declared retirements.
    // The fixed historical snapshot remains untouched at base.
    for file in historical["files"].as_array().unwrap() {
        let path = file["path"].as_str().unwrap();
        if !current_paths.contains(path) && path != "AGENTS.md" {
            let target = root.join(path);
            if target.exists() {
                fs::remove_file(target).unwrap();
            }
        }
    }
    for file in map["files"].as_array().unwrap() {
        let path = file["path"].as_str().unwrap();
        if path == "AGENTS.md" {
            continue;
        }
        let dest = root.join(path);
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        if dest.exists() {
            let mut permissions = fs::metadata(&dest).unwrap().permissions();
            permissions.set_readonly(false);
            fs::set_permissions(&dest, permissions).unwrap();
        }
        fs::copy(source.join(path), &dest)
            .unwrap_or_else(|e| panic!("copy declared fixture {path}: {e}"));
    }
    fs::create_dir_all(root.join(".chrono-harness/bin")).unwrap();
    for (project, name) in [
        ("ci", "chrono-ci"),
        ("runner", "chrono-harness"),
        ("judge-ci", "chrono-judge-ci"),
    ] {
        fs::copy(
            source.join(format!("crates/{project}/target/debug/{name}")),
            root.join(format!(".chrono-harness/bin/{name}")),
        )
        .unwrap();
    }
    // The copied candidate is a test host on the executing native platform.
    // Preserve the historical snapshot; adopt observed bindings only in the new
    // fixture. The copied host's Git digest/version can reject at Reader::for_config
    // before either historical-method or wrong-Python-version diagnostics.
    let mut config: Value = serde_json::from_slice(&fs::read(root.join(CONFIG)).unwrap()).unwrap();
    adopt_fixture_bindings(&root, &mut config);
    fs::write(
        root.join(CONFIG),
        serde_json::to_vec_pretty(&config).unwrap(),
    )
    .unwrap();
    let workflow = ".github/workflows/chrono-ci.yml";
    let mut bytes = fs::read(root.join(workflow)).unwrap();
    bytes.extend_from_slice(b"\n# real workflow drift\n");
    fs::write(root.join(workflow), bytes).unwrap();
    let candidate = commit(&root);
    (dir, root, base, candidate)
}

#[test]
fn decoded_historical_bindings_preserve_original_aliases_and_ambiguity() {
    for collision in [false, true] {
        let (_dir, root, base, candidate) = migration_host_with_alias_collision(collision);
        let raw = facts::registry_values(&root, &base, CONFIG).unwrap();
        let new = facts::registry_values(&root, &candidate, CONFIG).unwrap();
        let (old, current, view) = interpret(
            &root,
            &base,
            &candidate,
            CONFIG,
            raw.clone(),
            new,
            &std::env::vars().collect(),
        )
        .unwrap();
        assert_eq!(view["conversion"]["input"]["original"], json!(raw));
        let nodes = old.node_data();
        let node = &nodes["test:ci-verify"];
        assert_eq!(node.definitions.len(), if collision { 2 } else { 1 });
        let bindings = old.test_bindings();
        let historical = &bindings["test:ci-verify"];
        assert_eq!(historical.len(), node.definitions.len());
        assert!(historical.iter().any(|b| b.owner == "script:ci-verify"
            && b.action == "execute"
            && b.operation == "ci.verify"));
        if collision {
            assert!(
                historical
                    .iter()
                    .any(|b| b.owner == "project:ci-verify" && b.operation == "collision.execute")
            );
            assert!(node.unique().is_none());
            assert!(!execute_actions(&old).contains_key("test:ci-verify"));
            let error = chrono_judge_routes::order(
                &BTreeSet::from(["test:ci-verify".into()]),
                &execution::plans(old.filemap()).unwrap(),
                &execution::methods(current.projects()).unwrap(),
                &execute_actions(&old),
            )
            .unwrap_err();
            assert!(error.contains("unique execute action"), "{error}");
        } else {
            assert_eq!(execute_actions(&old)["test:ci-verify"], "ci.verify");
        }
        assert!(!execute_actions(&current).contains_key("test:ci-verify"));
        assert_eq!(execute_actions(&current)["test:ci-tests"], "test.ci-tests");
    }
}
#[test]
fn real_historical_profile_repair_preserves_obligations_and_verify_detects_drift() {
    let (_dir, root, base, candidate) = migration_host();
    let raw = facts::registry_values(&root, &base, CONFIG).unwrap();
    let new = facts::registry_values(&root, &candidate, CONFIG).unwrap();
    let mut missing_mapping = new.clone();
    missing_mapping.get_mut(WORKFLOW).unwrap()["historical_profiles"][0]["mappings"] = json!([]);
    let mut env: BTreeMap<String, String> = std::env::vars().collect();
    env.insert("RUSTUP_TOOLCHAIN".into(), "1.95.0".into());
    env.insert("CARGO_TERM_COLOR".into(), "never".into());
    // The historical decoder retains the original method. The current host
    // explicitly replaces its provider; standalone consumers may not infer it.
    let mut missing_method = new.clone();
    missing_method.get_mut(WORKFLOW).unwrap()["historical_profiles"][0]
        .as_object_mut()
        .unwrap()
        .remove("method_replacements");
    let error = interpret(
        &root,
        &base,
        &candidate,
        CONFIG,
        raw.clone(),
        missing_method.clone(),
        &env,
    )
    .err()
    .expect("missing method replacement unexpectedly accepted");
    assert!(
        error.contains("historical method changed"),
        "unexpected historical method error: {error}"
    );
    let old_action = raw[PROJECTS]["scripts"][0]["actions"]["execute"].clone();
    missing_method.get_mut(PROJECTS).unwrap()["projects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|p| p["id"] == "ci")
        .unwrap()["actions"]["execute"] = old_action;
    interpret(
        &root,
        &base,
        &candidate,
        CONFIG,
        raw.clone(),
        missing_method,
        &env,
    )
    .unwrap_or_else(|error| {
        panic!("omitted replacements must retain the original strict unchanged-method contract: {error}")
    });
    for (pointer, value) in [
        ("/from/argv", json!(["invented historical method"])),
        ("/from/owner", json!("script:missing")),
        ("/to/argv", json!(["stale candidate method"])),
        ("/to/tool", json!("missing-tool")),
        ("/to/owner", json!("project:runner")),
        ("/to/operation", json!("renamed.operation")),
        ("/to/argv", json!("not-an-array")),
        ("/reason", json!("")),
    ] {
        let mut invalid = new.clone();
        *invalid.get_mut(WORKFLOW).unwrap()["historical_profiles"][0]["method_replacements"][0]
            .pointer_mut(pointer)
            .unwrap() = value;
        assert!(
            interpret(&root, &base, &candidate, CONFIG, raw.clone(), invalid, &env).is_err(),
            "unbound replacement accepted: {pointer}"
        );
    }
    for variant in [
        "duplicate",
        "unknown-field",
        "unchanged",
        "unused",
        "ambiguous-target",
    ] {
        let mut invalid = new.clone();
        let rows =
            invalid.get_mut(WORKFLOW).unwrap()["historical_profiles"][0]["method_replacements"]
                .as_array_mut()
                .unwrap();
        match variant {
            "duplicate" => rows.push(rows[0].clone()),
            "unknown-field" => rows[0]["waiver"] = json!(true),
            "unchanged" => rows[0]["to"] = rows[0]["from"].clone(),
            "unused" => {
                rows[0]["from"]["operation"] = json!("unused.operation");
                rows[0]["to"]["operation"] = json!("unused.operation");
            }
            "ambiguous-target" => {
                let action = invalid.get_mut(PROJECTS).unwrap()["projects"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|p| p["id"] == "ci")
                    .unwrap()["actions"]
                    .as_object_mut()
                    .unwrap();
                action.insert("duplicate".into(), action["execute"].clone());
            }
            _ => unreachable!(),
        }
        assert!(
            interpret(&root, &base, &candidate, CONFIG, raw.clone(), invalid, &env).is_err(),
            "invalid method replacement accepted: {variant}"
        );
    }
    let (old, current, view) =
        interpret(&root, &base, &candidate, CONFIG, raw.clone(), new, &env).unwrap();
    assert!(
        interpret(
            &root,
            &base,
            &candidate,
            CONFIG,
            raw.clone(),
            missing_mapping,
            &env
        )
        .err()
        .unwrap()
        .contains("mapping")
    );
    assert_eq!(
        old.node_data()["test:ci-verify"].definitions[0].value,
        &raw[PROJECTS]["scripts"][0]
    );
    assert_eq!(old.filemap()["project_edges"], raw[FM]["project_edges"]);
    assert_eq!(old.filemap()["test_costs"], raw[FM]["test_costs"]);
    assert_eq!(
        view["conversion"]["input_digest"],
        wire::digest(&view["conversion"]["input"]).unwrap()
    );
    assert_eq!(view["conversion"]["input"]["original"], json!(raw));
    assert_eq!(
        chrono_judge_registration::replacements(&current).unwrap()["test:ci-verify"],
        "test:ci-tests"
    );
    let plans = execution::plans(current.filemap()).unwrap();
    for operation in &execution::plans(old.filemap()).unwrap()["test:ci-verify"].operations {
        assert!(plans["test:ci-tests"].operations.contains(operation));
    }
    let selected = BTreeSet::from(["test:ci-tests".into()]);
    let plan = prepare_scoped(
        &root,
        json!({"base":base,"candidate":candidate,"run":"real-ci-consumer"}),
        &selected,
        &plans,
        &execution::methods(current.projects()).unwrap(),
        &execute_actions(&current),
        &current.config()["tools"],
        env,
        &chrono_judge_registration::reused_tools(&view).unwrap(),
    )
    .unwrap();
    let result = chrono_judge_projects::execute(&plan).unwrap();
    let verify = result
        .executed
        .iter()
        .find(|r| r.operation == "ci.verify")
        .unwrap_or_else(|| panic!("{}", execution_failure(&root, "ci.verify blocked", &result)));
    assert_eq!(
        verify.status,
        "failed",
        "{}",
        execution_failure(&root, "ci.verify must reject drift", &result)
    );
    assert!(
        verify
            .receipt
            .as_ref()
            .unwrap()
            .process
            .stderr
            .contains("drift"),
        "{}",
        execution_failure(&root, "ci.verify did not diagnose drift", &result)
    );
    let output = Command::new(root.join(".chrono-harness/bin/chrono-ci"))
        .args([
            "generate",
            "--host-root",
            root.to_str().unwrap(),
            "--config",
            ".chrono-harness/ci/units.json",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "workflow repair failed: {output:?}"
    );
    // The repair changes only workflow bytes; rerun its single registered method with a new input binding.
    let action = current.projects()["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "ci")
        .unwrap()["actions"]["execute"]
        .clone();
    let tool = &plan.tools[action["tool"].as_str().unwrap()];
    let spec = chrono_harness::CommandSpec {
        program: tool.path.to_str().unwrap().into(),
        args: serde_json::from_value(action["argv"].clone()).unwrap(),
        env: plan.environment.clone(),
        timeout_seconds: 30,
        output_limit_bytes: 4096,
    };
    let verified = chrono_harness::run_process_observed(&root, &spec, &[], &tool.sha256).unwrap();
    assert_eq!(verified.exit_code, 0, "{}", verified.stderr);
    assert!(
        verified.failure.is_none(),
        "workflow verify failed: {verified:?}"
    );
}

#[test]
fn migration_binding_failure_retains_expected_and_observed_version() {
    let (_dir, root, base, candidate) = migration_host();
    let raw = facts::registry_values(&root, &base, CONFIG).unwrap();
    let mut new = facts::registry_values(&root, &candidate, CONFIG).unwrap();
    let tool = new.get_mut(CONFIG).unwrap()["tools"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|t| t["id"] == "python3")
        .unwrap();
    let observed = Command::new(tool["program"].as_str().unwrap())
        .arg("--version")
        .output()
        .unwrap();
    tool["expected_version"] = json!("Python deliberately-wrong");
    let env: BTreeMap<String, String> = std::env::vars().collect();
    let error = interpret(&root, &base, &candidate, CONFIG, raw, new, &env)
        .err()
        .unwrap();
    assert!(
        error.starts_with("E_TOOL_BINDING: migration version mismatch"),
        "unexpected migration binding error: {error}"
    );
    assert!(
        error.contains("Python deliberately-wrong"),
        "missing expected version: {error}"
    );
    let diagnostic: Value = serde_json::from_str(
        error
            .split_once("; observation=")
            .expect("missing observed binding")
            .1,
    )
    .unwrap();
    assert_eq!(
        diagnostic["tool"]["version"]["stdout_bytes"],
        json!(observed.stdout)
    );
    assert_eq!(
        diagnostic["tool"]["version"]["stderr_bytes"],
        json!(observed.stderr)
    );
    assert_eq!(
        diagnostic["tool"]["version"]["exit_code"],
        observed.status.code().unwrap()
    );
    assert!(
        diagnostic["tool"]["path"]
            .as_str()
            .unwrap()
            .starts_with('/')
    );
    assert!(diagnostic["tool"]["sha256"].as_str().unwrap().len() == 64);
}

#[test]
fn migration_copied_git_binding_rejects_mismatch_before_decoder() {
    let (_dir, root, base, candidate) = migration_host();
    let raw = facts::registry_values(&root, &base, CONFIG).unwrap();
    let config = facts::registry_values(&root, &candidate, CONFIG).unwrap()[CONFIG].clone();
    // A real forwarding executable has the same Git version but different bytes.
    // Moving a copied host declaration to it must reject the stale digest, then
    // pass after adopting the fixture's actual observation.
    let original_git = config["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["id"] == "git")
        .unwrap()["program"]
        .as_str()
        .unwrap();
    let wrapper = root.join(".chrono-harness/bin/fixture-git");
    fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\nexec '{}' \"$@\"\n",
            original_git.replace('\'', "'\\''")
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    let env: BTreeMap<String, String> = std::env::vars().collect();
    for field in ["digest", "version"] {
        let mut wrong = config.clone();
        if field == "digest" {
            wrong["tools"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|tool| tool["id"] == "git")
                .unwrap()["program"] = json!(wrapper);
            wrong["environment"]["inputs"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|input| input["id"] == "git-executable")
                .unwrap()["location"] = json!(wrapper);
        } else {
            wrong["tools"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|tool| tool["id"] == "git")
                .unwrap()["expected_version"] = json!("git version deliberately-wrong-platform");
        }
        fs::write(
            root.join(CONFIG),
            serde_json::to_vec_pretty(&wrong).unwrap(),
        )
        .unwrap();
        let wrong_candidate = commit(&root);
        let new = facts::registry_values(&root, &wrong_candidate, CONFIG).unwrap();
        let error = interpret(
            &root,
            &base,
            &wrong_candidate,
            CONFIG,
            raw.clone(),
            new,
            &env,
        )
        .err()
        .expect("copied mismatched Git binding unexpectedly accepted");
        if field == "digest" {
            assert_eq!(error, "E_GIT_FACTS: facts_git declared digest mismatch");
        } else {
            let diagnostic: Value =
                serde_json::from_str(error.strip_prefix("E_GIT_FACTS: ").expect(&error))
                    .unwrap_or_else(|parse| {
                        panic!("invalid Git binding diagnostic: {parse}; {error}")
                    });
            assert_eq!(
                diagnostic["message"], "Git facts version mismatch",
                "{error}"
            );
            assert_eq!(
                diagnostic["observation"]["binding"]["expected_version"],
                "git version deliberately-wrong-platform",
                "{error}"
            );
            assert_eq!(
                chrono_harness::full::expand_process(&diagnostic["observation"]["processes"][0])
                    .unwrap()["stdout"],
                format!(
                    "{}\n",
                    config["tools"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|tool| tool["id"] == "git")
                        .unwrap()["expected_version"]
                        .as_str()
                        .unwrap()
                ),
                "{error}"
            );
        }
        adopt_fixture_bindings(&root, &mut wrong);
        fs::write(
            root.join(CONFIG),
            serde_json::to_vec_pretty(&wrong).unwrap(),
        )
        .unwrap();
        let repaired_candidate = commit(&root);
        let mut new = facts::registry_values(&root, &repaired_candidate, CONFIG).unwrap();
        new.get_mut(CONFIG).unwrap()["tools"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|tool| tool["id"] == "python3")
            .unwrap()["expected_version"] = json!("Python deliberately-wrong");
        let error = interpret(
            &root,
            &base,
            &repaired_candidate,
            CONFIG,
            raw.clone(),
            new,
            &env,
        )
        .err()
        .expect("wrong Python binding unexpectedly accepted after adopting Git");
        assert!(
            error.starts_with("E_TOOL_BINDING: migration version mismatch"),
            "{error}"
        );
    }
}
