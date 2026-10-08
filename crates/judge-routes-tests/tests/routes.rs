use chrono_harness::{CommandSpec, run_process_observed, sha256, wire};
use chrono_judge_registration::execution::{Method, Plan};
use chrono_judge_routes::{Receipt, compare, order, prepare};
use serde_json::json;
use std::{collections::BTreeMap, fs};
fn declarations() -> serde_json::Value {
    let path = chrono_harness::resolve_program(std::path::Path::new("/"), "python3", None).unwrap();
    let out = std::process::Command::new(&path)
        .arg("--version")
        .output()
        .unwrap();
    json!([{"id":"python","program":path,"version_argv":["--version"],"expected_version":String::from_utf8(out.stdout).unwrap().trim_end()}])
}
fn method(id: &str, args: Vec<String>) -> Method {
    Method {
        owner: "script:p".into(),
        operation: id.into(),
        tool: "python".into(),
        argv: args,
    }
}
fn p(ops: &[&str]) -> Plan {
    Plan {
        operations: ops.iter().map(|s| s.to_string()).collect(),
        timeout_seconds: 10,
        output_limit_bytes: 4096,
        operation_bounds: BTreeMap::new(),
    }
}
#[test]
fn shared_prerequisites_order_and_dedup() {
    let plans = BTreeMap::from([
        ("test:a".into(), p(&["shared", "a"])),
        ("test:b".into(), p(&["shared", "a", "b"])),
    ]);
    let methods = ["shared", "a", "b"]
        .into_iter()
        .map(|id| (id.into(), vec![method(id, vec![])]))
        .collect();
    let exec = BTreeMap::from([("test:a".into(), "a".into()), ("test:b".into(), "b".into())]);
    let (_, ops) = order(&plans.keys().cloned().collect(), &plans, &methods, &exec).unwrap();
    assert_eq!(
        ops.iter()
            .map(|o| o.method.operation.as_str())
            .collect::<Vec<_>>(),
        ["shared", "a", "b"]
    );
}
#[test]
fn explicit_prerequisite_bounds_preserve_each_consumer_default_and_reject_disagreement() {
    let mut plans: BTreeMap<String, Plan> = serde_json::from_value(json!({
        "test:distribution": {"operations":["build", "distribution"],
            "timeout_seconds":600,"output_limit_bytes":1048576},
        "test:startup": {"operations":["build", "startup"],
            "timeout_seconds":900,"output_limit_bytes":1048576,
            "operation_bounds":{"build":{"timeout_seconds":600,"output_limit_bytes":1048576}}}
    }))
    .unwrap();
    let methods = ["build", "distribution", "startup"]
        .into_iter()
        .map(|id| (id.into(), vec![method(id, vec![])]))
        .collect();
    let exec = BTreeMap::from([
        ("test:distribution".into(), "distribution".into()),
        ("test:startup".into(), "startup".into()),
    ]);
    let selected = plans.keys().cloned().collect();
    let (chosen, ops) = order(&selected, &plans, &methods, &exec).unwrap();
    assert_eq!(chosen["test:startup"].timeout_seconds, 900);
    assert_eq!(
        ops.iter()
            .map(|o| (
                o.method.operation.as_str(),
                o.timeout_seconds,
                o.output_limit_bytes
            ))
            .collect::<Vec<_>>(),
        [
            ("build", 600, 1048576),
            ("distribution", 600, 1048576),
            ("startup", 900, 1048576)
        ]
    );
    assert_eq!(ops[2].predecessors, ["build".into()].into());
    let directory = tempfile::tempdir().unwrap();
    let collection = chrono_judge_routes::prepare_collection(
        directory.path(),
        json!({}),
        &selected,
        &plans,
        &methods,
        &exec,
        BTreeMap::new(),
    )
    .unwrap();
    assert!(collection.tools.is_empty());
    assert_eq!(collection.selected, chosen);
    assert_eq!(
        serde_json::to_value(&collection.operations).unwrap(),
        serde_json::to_value(&ops).unwrap()
    );
    let decoded: chrono_judge_routes::Execution =
        serde_json::from_value(serde_json::to_value(collection).unwrap()).unwrap();
    decoded.validate().unwrap();
    let original = serde_json::to_value(&plans).unwrap();
    for bounds in [
        json!({"timeout_seconds":601,"output_limit_bytes":1048576}),
        json!({"timeout_seconds":600,"output_limit_bytes":1048577}),
    ] {
        let mut conflicting = original.clone();
        conflicting["test:startup"]["operation_bounds"]["build"] = bounds;
        let conflicting = serde_json::from_value(conflicting).unwrap();
        assert!(
            order(&selected, &conflicting, &methods, &exec)
                .unwrap_err()
                .contains("E_PLAN_BOUNDS")
        );
    }
    plans.get_mut("test:startup").unwrap().timeout_seconds = 901;
    let (_, ops) = order(&selected, &plans, &methods, &exec).unwrap();
    assert_eq!(ops[0].timeout_seconds, 600);
    assert_eq!(ops[2].timeout_seconds, 901);
}

#[test]
fn main_host_source_installer_prerequisite_keeps_original_test_contracts() {
    use chrono_judge_registration::execution::{methods, plans, test_bindings};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let filemap =
        serde_json::from_slice(&fs::read(root.join(".chrono-harness/FILEMAP.json")).unwrap())
            .unwrap();
    let projects =
        serde_json::from_slice(&fs::read(root.join(".chrono-harness/projects.json")).unwrap())
            .unwrap();
    let mut plans = plans(&filemap).unwrap();
    let execute = test_bindings(&projects)
        .unwrap()
        .into_iter()
        .map(|(id, bindings)| {
            assert_eq!(bindings.len(), 1);
            (id, bindings[0].operation.clone())
        })
        .collect();
    let selected = [
        "test:host-bootstrap-tests",
        "test:distribution-tests",
        "test:release-build-integration",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let (_, operations) = order(&selected, &plans, &methods(&projects).unwrap(), &execute).unwrap();
    assert_eq!(
        operations
            .iter()
            .filter(|o| o.method.operation == "build.distribution")
            .count(),
        1
    );
    for (operation, timeout) in [
        ("build.distribution", 600),
        ("host.bootstrap-tests", 900),
        ("test.distribution-tests", 600),
        ("release.tests.integration", 600),
    ] {
        let op = operations
            .iter()
            .find(|o| o.method.operation == operation)
            .unwrap();
        assert_eq!(op.timeout_seconds, timeout, "{operation}");
        assert_eq!(op.output_limit_bytes, 1048576, "{operation}");
    }
    assert_eq!(
        plans["test:host-bootstrap-tests"].operations,
        ["build.distribution", "host.bootstrap-tests"]
    );
    assert_eq!(plans["test:host-bootstrap-tests"].timeout_seconds, 900);
    for test in ["test:distribution-tests", "test:release-build-integration"] {
        assert_eq!(plans[test].timeout_seconds, 600);
        assert!(plans[test].operation_bounds.is_empty());
    }
    plans
        .get_mut("test:host-bootstrap-tests")
        .unwrap()
        .operation_bounds
        .clear();
    assert!(
        order(&selected, &plans, &methods(&projects).unwrap(), &execute)
            .unwrap_err()
            .contains("E_PLAN_BOUNDS: inconsistent shared operation build.distribution")
    );
}
#[test]
fn conflicts_missing_duplicate_methods_and_inconsistent_bounds_preflight() {
    let mut plans = BTreeMap::from([
        ("test:a".into(), p(&["a", "b"])),
        ("test:b".into(), p(&["b", "a"])),
    ]);
    let mut methods: BTreeMap<_, _> = ["a", "b"]
        .into_iter()
        .map(|id| (id.into(), vec![method(id, vec![])]))
        .collect();
    let exec = BTreeMap::from([("test:a".into(), "a".into()), ("test:b".into(), "b".into())]);
    let selected = plans.keys().cloned().collect();
    // A nonexistent tool path demonstrates that cycle rejection precedes tool processes.
    assert!(
        prepare(
            std::path::Path::new("/"),
            json!({}),
            &selected,
            &plans,
            &methods,
            &exec,
            &json!([]),
            BTreeMap::new(),
            &BTreeMap::new()
        )
        .unwrap_err()
        .contains("E_PLAN_CYCLE")
    );
    plans.get_mut("test:b").unwrap().operations = vec!["a".into(), "b".into()];
    plans.get_mut("test:b").unwrap().timeout_seconds = 20;
    assert!(
        order(&selected, &plans, &methods, &exec)
            .unwrap_err()
            .contains("E_PLAN_BOUNDS")
    );
    plans.get_mut("test:b").unwrap().timeout_seconds = 10;
    methods.get_mut("a").unwrap().push(method("a", vec![]));
    assert!(
        order(&selected, &plans, &methods, &exec)
            .unwrap_err()
            .contains("E_ROUTE_AMBIGUOUS")
    );
    methods.remove("a");
    assert!(
        order(&selected, &plans, &methods, &exec)
            .unwrap_err()
            .contains("E_ROUTE_MISSING")
    );
}
#[test]
fn observed_literal_argv_environment_bytes_receipt_and_wrong_run() {
    let dir = tempfile::Builder::new()
        .prefix("routes host space ")
        .tempdir()
        .unwrap();
    let args=vec!["-c".into(),"import sys,os,json; print(json.dumps([sys.argv[1:],os.getcwd(),os.environ.get('EMPTY'),os.environ.get('ABSENT')]))".into(),"$HOME `literal` <tag>".into(),"".into(),"line\nnext".into()];
    let methods = BTreeMap::from([("test".into(), vec![method("test", args.clone())])]);
    let plans = BTreeMap::from([("test:t".into(), p(&["test"]))]);
    let exec = BTreeMap::from([("test:t".into(), "test".into())]);
    let plan = prepare(
        dir.path(),
        json!({"run":"one"}),
        &plans.keys().cloned().collect(),
        &plans,
        &methods,
        &exec,
        &declarations(),
        BTreeMap::from([("EMPTY".into(), "".into())]),
        &BTreeMap::new(),
    )
    .unwrap();
    let op = &plan.operations[0];
    let tool = &plan.tools["python"];
    let process = run_process_observed(
        &plan.root,
        &CommandSpec {
            program: tool.path.to_str().unwrap().into(),
            args,
            env: plan.environment.clone(),
            timeout_seconds: 10,
            output_limit_bytes: 4096,
        },
        &[],
        &tool.sha256,
    )
    .unwrap();
    let data: serde_json::Value = serde_json::from_slice(&process.stdout_bytes).unwrap();
    assert_eq!(data[0], json!(["$HOME `literal` <tag>", "", "line\nnext"]));
    assert_eq!(data[2], "");
    assert!(data[3].is_null());
    let receipt = Receipt::new(&plan, op, process).unwrap();
    compare(&plan, op, Some(&receipt)).unwrap();
    assert!(compare(&plan, op, None).is_err());
    let mut tampered = receipt.clone();
    tampered.process.argv.push("injected".into());
    assert!(compare(&plan, op, Some(&tampered)).is_err());
    let mut tampered = receipt.clone();
    tampered.plan = "other-run".into();
    assert!(compare(&plan, op, Some(&tampered)).is_err());
    let mut tampered = receipt.clone();
    tampered.process.stdout_bytes.push(1);
    assert!(compare(&plan, op, Some(&tampered)).is_err());
    for mode in 0..8 {
        let mut process = receipt.process.clone();
        match mode {
            0 => process.argv.push("different".into()),
            1 => process.cwd = "/".into(),
            2 => {
                process.environment.insert("NEW".into(), "input".into());
                process.environment_digest = wire::digest(&process.environment).unwrap();
            }
            3 => process.sha256 = "0".repeat(64),
            4 => process.stdin_sha256 = sha256(b"different input"),
            5 => process.stdout_sha256 = "0".repeat(64),
            6 => {
                process.stdout_bytes = vec![b'x'; op.output_limit_bytes + 1];
                process.stdout = String::from_utf8(process.stdout_bytes.clone()).unwrap();
                process.stdout_sha256 = sha256(&process.stdout_bytes);
            }
            _ => {
                process.stderr_bytes = vec![b'x'; op.output_limit_bytes + 1];
                process.stderr = String::from_utf8(process.stderr_bytes.clone()).unwrap();
                process.stderr_sha256 = sha256(&process.stderr_bytes);
            }
        }
        let sealed = Receipt::new(&plan, op, process).unwrap();
        assert!(
            compare(&plan, op, Some(&sealed)).is_err(),
            "resealed wrong observation mode {mode}"
        );
    }
    assert_eq!(
        receipt.process.stdout_sha256,
        sha256(&receipt.process.stdout_bytes)
    );
    assert_eq!(
        receipt.process.environment_digest,
        wire::digest(&plan.environment).unwrap()
    );
}
#[test]
fn tool_wrong_version_and_replacement_before_launch_fail() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tool");
    fs::copy(declarations()[0]["program"].as_str().unwrap(), &path).unwrap();
    let mut tools = declarations();
    tools[0]["program"] = json!(path);
    tools[0]["expected_version"] = json!("wrong");
    let methods = BTreeMap::from([(
        "test".into(),
        vec![method("test", vec!["--version".into()])],
    )]);
    let plans = BTreeMap::from([("test:t".into(), p(&["test"]))]);
    let exec = BTreeMap::from([("test:t".into(), "test".into())]);
    assert!(
        prepare(
            dir.path(),
            json!({}),
            &plans.keys().cloned().collect(),
            &plans,
            &methods,
            &exec,
            &tools,
            BTreeMap::new(),
            &BTreeMap::new()
        )
        .is_err()
    );
    // Preserve a real executable identity; replacing its bytes must fail before spawn.
    let hash = sha256(&fs::read(&path).unwrap());
    fs::write(&path, b"replaced").unwrap();
    assert!(
        run_process_observed(
            dir.path(),
            &CommandSpec {
                program: path.to_str().unwrap().into(),
                args: vec![],
                env: BTreeMap::new(),
                timeout_seconds: 10,
                output_limit_bytes: 4096
            },
            &[],
            &hash
        )
        .unwrap_err()
        .contains("digest mismatch")
    );
}

#[test]
fn path_shadow_cannot_replace_bound_invocation_and_bytes_are_rechecked() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    fs::create_dir(&a).unwrap();
    fs::create_dir(&b).unwrap();
    let source = declarations();
    symlink(source[0]["program"].as_str().unwrap(), a.join("python3")).unwrap();
    symlink("/bin/sh", b.join("python3")).unwrap();
    let mut tools = source;
    tools[0]["program"] = json!("python3");
    let methods = BTreeMap::from([(
        "test".into(),
        vec![method(
            "test",
            vec!["-c".into(), "print('bound-python')".into()],
        )],
    )]);
    let plans = BTreeMap::from([("test:t".into(), p(&["test"]))]);
    let exec = BTreeMap::from([("test:t".into(), "test".into())]);
    let env = BTreeMap::from([("PATH".into(), a.to_str().unwrap().into())]);
    let plan = prepare(
        dir.path(),
        json!({}),
        &plans.keys().cloned().collect(),
        &plans,
        &methods,
        &exec,
        &tools,
        env,
        &BTreeMap::new(),
    )
    .unwrap();
    let op = &plan.operations[0];
    let tool = &plan.tools["python"];
    let spec = CommandSpec {
        program: tool.path.to_str().unwrap().into(),
        args: op.method.argv.clone(),
        env: BTreeMap::from([("PATH".into(), b.to_str().unwrap().into())]),
        timeout_seconds: 10,
        output_limit_bytes: 4096,
    };
    let actual = run_process_observed(dir.path(), &spec, &[], &tool.sha256).unwrap();
    assert_eq!(actual.stdout, "bound-python\n");
    let receipt = Receipt::new(&plan, op, actual).unwrap();
    assert!(
        compare(&plan, op, Some(&receipt)).is_err(),
        "changed effective PATH cannot use old receipt binding"
    );
    fs::remove_file(a.join("python3")).unwrap();
    symlink("/bin/sh", a.join("python3")).unwrap();
    assert!(
        run_process_observed(dir.path(), &spec, &[], &tool.sha256)
            .unwrap_err()
            .contains("digest mismatch")
    );
}

#[test]
fn canonical_compares_actual_order_and_paths_instead_of_parsed_reconstruction() {
    let dir = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(dir.path()).unwrap();
    fs::write(root.join("runner"), b"runner").unwrap();
    fs::write(root.join("config"), b"config").unwrap();
    let template = vec![
        "runner".into(),
        "check".into(),
        "--config".into(),
        "config".into(),
        "--base".into(),
        "{base}".into(),
        "--candidate".into(),
        "{candidate}".into(),
    ];
    let argv = vec![
        root.join("runner").to_str().unwrap().to_string(),
        "check".into(),
        "--config".into(),
        root.join("config").to_str().unwrap().to_string(),
        "--base".into(),
        "a".into(),
        "--candidate".into(),
        "b".into(),
    ];
    let mut entry = json!({"argv":argv,"cwd":"/"});
    let binding = json!({"base":"a","candidate":"b"});
    chrono_judge_routes::validate_invocation(&root, &entry, &template, &binding).unwrap();
    entry["argv"] = json!([
        root.join("runner"),
        "check",
        "--base",
        "a",
        "--config",
        root.join("config"),
        "--candidate",
        "b"
    ]);
    assert!(
        chrono_judge_routes::validate_invocation(&root, &entry, &template, &binding).is_err(),
        "same parsed values with unregistered argv order must fail"
    );
}

#[test]
fn reused_tool_observation_avoids_second_version_launch_and_cycle_launches_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(dir.path()).unwrap();
    let source = declarations();
    let program = source[0]["program"].as_str().unwrap();
    let version_argv = vec![
        "-c".into(),
        "import sys; open('version-count','a').write('v'); print(sys.version)".into(),
    ];
    let environment = BTreeMap::new();
    let observed =
        chrono_harness::observation::tool(&root, program, &version_argv, &environment, 10, 4096)
            .unwrap();
    let declarations = json!([{"id":"python","program":program,"version_argv":version_argv,"expected_version":observed.version.stdout.trim_end()}]);
    let methods = ["a", "b"]
        .into_iter()
        .map(|id| (id.into(), vec![method(id, vec!["--version".into()])]))
        .collect();
    let mut plans = BTreeMap::from([
        ("test:a".into(), p(&["a", "b"])),
        ("test:b".into(), p(&["b"])),
    ]);
    let actions = BTreeMap::from([("test:a".into(), "a".into()), ("test:b".into(), "b".into())]);
    let selected = plans.keys().cloned().collect();
    let reused = BTreeMap::from([("python".into(), observed.clone())]);
    let plan = prepare(
        &root,
        json!({"run":"reuse"}),
        &selected,
        &plans,
        &methods,
        &actions,
        &declarations,
        environment.clone(),
        &reused,
    )
    .unwrap();
    assert_eq!(
        plan.tools["python"].version.stdout_bytes,
        observed.version.stdout_bytes
    );
    assert_eq!(fs::read_to_string(root.join("version-count")).unwrap(), "v");
    let mut wrong = reused;
    wrong
        .get_mut("python")
        .unwrap()
        .version
        .environment
        .insert("UNDECLARED".into(), "value".into());
    assert!(
        prepare(
            &root,
            json!({}),
            &selected,
            &plans,
            &methods,
            &actions,
            &declarations,
            environment.clone(),
            &wrong
        )
        .unwrap_err()
        .contains("E_TOOL_BINDING")
    );
    plans.get_mut("test:b").unwrap().operations = vec!["b".into(), "a".into()];
    assert!(
        prepare(
            &root,
            json!({}),
            &selected,
            &plans,
            &methods,
            &actions,
            &declarations,
            environment,
            &BTreeMap::new()
        )
        .unwrap_err()
        .contains("E_PLAN_CYCLE")
    );
    assert_eq!(
        fs::read_to_string(root.join("version-count")).unwrap(),
        "v",
        "conflicting plan launched no tool version process"
    );
}

#[test]
fn scheduling_binds_identity_and_collection_reconstructs_without_processes() {
    let d = tempfile::tempdir().unwrap();
    let plans = BTreeMap::from([("test:t".into(), p(&["run"]))]);
    let methods = BTreeMap::from([("run".into(), vec![method("run", vec!["literal".into()])])]);
    let actions = BTreeMap::from([("test:t".into(), "run".into())]);
    let legacy = chrono_judge_routes::prepare_collection(
        d.path(),
        json!({}),
        &plans.keys().cloned().collect(),
        &plans,
        &methods,
        &actions,
        BTreeMap::new(),
    )
    .unwrap();
    let value = serde_json::to_value(&legacy).unwrap();
    assert!(value.get("scheduling").is_none());
    let mut unsigned = value.clone();
    unsigned.as_object_mut().unwrap().remove("identity");
    assert_eq!(legacy.identity, wire::digest(&unsigned).unwrap());
    let policy = serde_json::from_value(
        json!({"max_running":2,"resources":[],"claims":{"run":{"resources":[],"outputs":[]}}}),
    )
    .unwrap();
    let concurrent = legacy.clone().with_scheduling(Some(policy)).unwrap();
    assert_ne!(legacy.identity, concurrent.identity);
    assert!(concurrent.tools.is_empty());
    let mut changed = serde_json::to_value(&concurrent).unwrap();
    changed["scheduling"]["max_running"] = json!(1);
    let changed: chrono_judge_routes::Execution = serde_json::from_value(changed).unwrap();
    assert!(changed.validate().is_err());
    let bad = serde_json::from_value(json!({"max_running":2,"resources":[],"claims":{}})).unwrap();
    assert!(legacy.with_scheduling(Some(bad)).is_err());
}
