#[path = "../../judge-filemap-tests/tests/support/mod.rs"]
mod support;
use chrono_harness::{facts, wire};
use chrono_judge_registration::{execution, interpret};
use chrono_judge_routes::{execute_actions, prepare_scoped};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    process::Command,
};
use support::*;
fn migration_host() -> (tempfile::TempDir, std::path::PathBuf, String, String) {
    let source = fs::canonicalize(source()).unwrap();
    let original = "4f08aef7ab40d7b0a3fb6ba42af2620600f18d98";
    let dir = tempfile::Builder::new()
        .prefix("migration actual host ")
        .tempdir()
        .unwrap();
    let root = fs::canonicalize(dir.path()).unwrap();
    let tree = facts::tree(&source, original).unwrap();
    facts::export(&source, original, &tree, &root).unwrap();
    git(&root, &["init", "-q"]);
    let base = commit(&root);
    let map: Value = serde_json::from_slice(&fs::read(source.join(FM)).unwrap()).unwrap();
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
    for name in ["chrono-ci", "chrono-harness", "chrono-judge-ci"] {
        fs::copy(
            source.join(format!(".chrono-harness/bin/{name}")),
            root.join(format!(".chrono-harness/bin/{name}")),
        )
        .unwrap();
    }
    // The copied candidate is a test host on the executing native platform.
    // Preserve the historical snapshot; explicitly adopt the current interpreter
    // only in the new fixture. Wrong-version rejection remains a separate test.
    let mut config: Value = serde_json::from_slice(&fs::read(root.join(CONFIG)).unwrap()).unwrap();
    let python = config["tools"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|t| t["id"] == "python3")
        .unwrap();
    let observed = Command::new(python["program"].as_str().unwrap())
        .args(serde_json::from_value::<Vec<String>>(python["version_argv"].clone()).unwrap())
        .output()
        .unwrap();
    assert!(observed.status.success());
    python["expected_version"] = json!(String::from_utf8(observed.stdout).unwrap().trim_end());
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
    assert!(
        interpret(
            &root,
            &base,
            &candidate,
            CONFIG,
            raw.clone(),
            missing_method.clone(),
            &env
        )
        .err()
        .unwrap()
        .contains("historical method changed")
    );
    let old_action = raw[PROJECTS]["scripts"][0]["actions"]["execute"].clone();
    missing_method.get_mut(PROJECTS).unwrap()["projects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|p| p["id"] == "ci")
        .unwrap()["actions"]["execute"] = old_action;
    assert!(
        interpret(
            &root,
            &base,
            &candidate,
            CONFIG,
            raw.clone(),
            missing_method,
            &env
        )
        .is_ok(),
        "omitted replacements must retain the original strict unchanged-method contract"
    );
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
        .unwrap_or_else(|| {
            panic!(
                "ci.verify blocked: {:?}",
                result
                    .executed
                    .iter()
                    .filter(|r| r.status != "passed")
                    .map(|r| (&r.operation, &r.status, &r.error, &r.receipt))
                    .collect::<Vec<_>>()
            )
        });
    assert_eq!(verify.status, "failed");
    assert!(
        verify
            .receipt
            .as_ref()
            .unwrap()
            .process
            .stderr
            .contains("drift")
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
        "{}",
        String::from_utf8_lossy(&output.stderr)
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
    assert!(verified.failure.is_none());
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
    assert!(error.starts_with("E_TOOL_BINDING: migration version mismatch"));
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
