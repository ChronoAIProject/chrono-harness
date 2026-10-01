use super::*;
use std::{os::unix::fs::PermissionsExt, process::Command};

pub(super) fn bound_host() -> Host {
    bind_host(host())
}
pub(super) fn bind_host(mut h: Host) -> Host {
    let root = h.root();
    let state = root.join(".chrono-harness/state");
    fs::create_dir_all(&state).unwrap();
    let selected = state.join("selected Git λ");
    let trace = state.join("git-trace");
    let real = chrono_harness::resolve_program(&root, "git", None).unwrap();
    let quote = |p: &std::path::Path| format!("'{}'", p.to_str().unwrap().replace('\'', "'\\''"));
    fs::write(&selected, format!("#!/bin/sh\n[ \"$FACTS_SENTINEL\" = declared ] || exit 81\nprintf '%s\\n' \"$*\" >> {}\nexec {} \"$@\"\n",quote(&trace),quote(&real))).unwrap();
    fs::set_permissions(&selected, fs::Permissions::from_mode(0o755)).unwrap();
    let shadow = state.join("shadow");
    fs::create_dir(&shadow).unwrap();
    fs::write(
        shadow.join("git"),
        "#!/bin/sh\necho ambient-git-must-not-run >&2\nexit 83\n",
    )
    .unwrap();
    fs::set_permissions(shadow.join("git"), fs::Permissions::from_mode(0o755)).unwrap();
    let version = Command::new(&real).arg("--version").output().unwrap();
    assert!(version.status.success());
    h.tool = fs::canonicalize(&h.tool).unwrap();
    let cfg = h.values.get_mut(CONFIG).unwrap();
    cfg["tools"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|tool| tool["id"] == "python")
        .unwrap()["program"] = json!(h.tool);
    cfg["schema_version"] = json!(3);
    cfg["facts_git"] = json!({"tool":"facts-git","input":"git-bytes"});
    cfg["tools"].as_array_mut().unwrap().push(json!({"id":"facts-git","program":selected,"resolution":"PATH-once","version_argv":["--version"],"expected_version":String::from_utf8(version.stdout).unwrap().trim_end()}));
    for input in cfg["environment"]["inputs"].as_array_mut().unwrap() {
        input["presence"] = json!("present");
        input["location"] = json!(fs::canonicalize(input["location"].as_str().unwrap()).unwrap());
    }
    cfg["environment"]["inputs"].as_array_mut().unwrap().push(json!({"id":"git-bytes","location":selected,"presence":"present","sha256":sha256(&fs::read(&selected).unwrap())}));
    cfg["environment"]["values"]["PATH"] = json!(shadow);
    cfg["environment"]["values"]["FACTS_SENTINEL"] = json!("declared");
    // Schema v3's declared-complete status is an explicit inventory claim.
    // Keep this fixture's complete whitelist and FILEMAP edges visible here;
    // registration must not infer any of these relationships from the
    // commands, paths, or host layout.
    cfg["input_closure"] = json!({
        "status": "declared-complete",
        "unresolved": [],
        "bindings": [
            {
                "id": "registration-git-facts",
                "consumer": "judge:registration",
                "kind": "git-facts",
                "inputs": [
                    "tool:facts-git",
                    "input:git-bytes",
                    "environment:PATH",
                    "environment:FACTS_SENTINEL"
                ]
            },
            {
                "id": "projects-execution",
                "consumer": "judge:projects",
                "kind": "project-execution",
                "inputs": [
                    "tool:python",
                    "input:interpreter",
                    "input:data",
                    "environment:DECLARED_EMPTY",
                    "environment:DECLARED_ABSENT",
                    "environment:EMPTY",
                    "environment:PATH",
                    "environment:FACTS_SENTINEL"
                ]
            }
        ]
    });
    cfg["semantic_fields"]
        .as_array_mut()
        .unwrap()
        .push(json!({"path":"policy.json","pointers":["/limit"],"on":"add-modify-delete"}));
    h.values.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .extend([
            edge("tool:facts-git", "runtime-input", "judge:registration"),
            edge("input:git-bytes", "judge-trigger", "judge:registration"),
            edge("environment:PATH", "runtime-input", "judge:registration"),
            edge(
                "environment:FACTS_SENTINEL",
                "runtime-input",
                "judge:registration",
            ),
            edge("tool:python", "runtime-input", "judge:projects"),
            edge("input:interpreter", "runtime-input", "judge:projects"),
            edge("input:data", "runtime-input", "judge:projects"),
            edge(
                "environment:DECLARED_EMPTY",
                "runtime-input",
                "judge:projects",
            ),
            edge(
                "environment:DECLARED_ABSENT",
                "runtime-input",
                "judge:projects",
            ),
            edge("environment:EMPTY", "runtime-input", "judge:projects"),
            edge("environment:PATH", "runtime-input", "judge:projects"),
            edge(
                "environment:FACTS_SENTINEL",
                "runtime-input",
                "judge:projects",
            ),
        ]);
    let mut policy = file("policy.json", json!([]));
    policy["surface"] = json!("judge-policy");
    h.values.get_mut(FM).unwrap()["files"]
        .as_array_mut()
        .unwrap()
        .push(policy);
    fs::write(root.join("policy.json"), "{\"limit\":1}").unwrap();
    h.save();
    h.base = h.candidate.clone();
    fs::write(root.join("policy.json"), "{\"limit\":2}").unwrap();
    fs::write(root.join("p/product.py"), "def double(n): return n+n\n").unwrap();
    h.save();
    h
}
pub(super) fn run_bound(h: &Host, kind: &str, digest: Option<&str>) -> (i32, Value) {
    bound_context(h, kind, digest, true)
}
pub(super) fn prepare_bound(h: &Host, kind: &str, digest: Option<&str>) {
    bound_context(h, kind, digest, false);
}
fn bound_context(h: &Host, kind: &str, digest: Option<&str>, execute: bool) -> (i32, Value) {
    let root = h.root();
    let started = std::time::Instant::now();
    let snapshot = chrono_harness::facts::registry_snapshot(&root, &h.candidate, CONFIG).unwrap();
    let selected = std::path::PathBuf::from(
        snapshot.values[&snapshot.effective_path]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["id"] == "facts-git")
            .unwrap()["program"]
            .as_str()
            .unwrap(),
    );
    let edit = |retained: &mut Value| {
        for (endpoint, oid) in [("base", &h.base), ("candidate", &h.candidate)] {
            let snapshot = chrono_harness::facts::registry_snapshot(&root, oid, CONFIG).unwrap();
            let cfg = &snapshot.values[&snapshot.effective_path];
            retained[endpoint]["schema"] =
                json!(chrono_judge_registration::inputs::snapshot_schema(cfg));
            if let Some(selection) = snapshot.selection {
                retained[endpoint]["schema"] = json!("chrono-input-snapshot/v3");
                retained[endpoint]["effective_config_path"] = json!(snapshot.effective_path);
                retained[endpoint]["selection"] = selection;
            }
            retained[endpoint]["config_path"] = json!(CONFIG);
            retained[endpoint]["config_digest"] =
                json!(chrono_harness::wire::digest(&cfg).unwrap());
            for key in cfg["environment"]["inherit"].as_array().unwrap() {
                if retained[endpoint]["environment"]
                    .get(key.as_str().unwrap())
                    .is_none()
                {
                    retained[endpoint]["environment"][key.as_str().unwrap()] = Value::Null;
                }
            }
            if let Some(input) = cfg["environment"]["inputs"]
                .as_array()
                .unwrap()
                .iter()
                .find(|i| i["id"] == "interpreter")
            {
                let location = root.join(input["location"].as_str().unwrap());
                let bytes = fs::read(location).unwrap();
                let digest = sha256(&bytes);
                let blob = format!(".chrono-harness/state/fixture-{digest}");
                fs::write(root.join(&blob), &bytes).unwrap();
                retained[endpoint]["files"]["interpreter"] =
                    json!({"blob":blob,"sha256":digest,"length":bytes.len()});
            }
            if cfg["environment"]["inputs"]
                .as_array()
                .unwrap()
                .iter()
                .any(|i| i["id"] == "git-bytes")
            {
                retained[endpoint]["files"]["git-bytes"] =
                    json!({"bytes":fs::read(&selected).unwrap()});
            }
        }
    };
    let context_edit = |ctx: &mut Value| {
        ctx["schema_version"] = json!(2);
        ctx["run_kind"] = json!(kind);
        ctx["integration_evidence"] = json!(digest);
        if kind == "delivery" {
            ctx["branch_ref"] = json!("feature/bound-git");
        }
    };
    if !execute {
        h.prepare_with_context(edit, context_edit);
        return (0, Value::Null);
    }
    let outcome = h.run_with_context(edit, context_edit);
    let bytes = if outcome.1["scope"] == "configured-judges" {
        fs::metadata(root.join(".chrono-harness/state/report.json"))
            .map(|m| m.len())
            .unwrap_or(0)
    } else {
        0
    };
    println!(
        "COMMAND {} argv={} exit={} seconds={:.3} report_bytes={}",
        root.join(".chrono-harness/bin/chrono-harness").display(),
        json!([
            "check",
            "--config",
            root.join(CONFIG).to_str().unwrap(),
            "--base",
            h.base,
            "--candidate",
            h.candidate,
            "--context",
            root.join(".chrono-harness/state/context.json")
                .to_str()
                .unwrap()
        ]),
        outcome.0,
        started.elapsed().as_secs_f64(),
        bytes
    );
    outcome
}

#[test]
fn all_full_judges_and_integration_delivery_use_candidate_bound_git() {
    all_full_judges_consumer(false);
}

#[test]
fn selected_full_judges_and_integration_delivery_use_entry_bound_policy() {
    all_full_judges_consumer(true);
}

fn all_full_judges_consumer(selected_mode: bool) {
    let mut h = bound_host();
    if selected_mode {
        select(&mut h, NATIVE_CONFIG);
        h.save();
        h.base = h.candidate.clone();
        fs::write(h.root().join("policy.json"), "{\"limit\":3}").unwrap();
        fs::write(h.root().join("p/product.py"), "def double(n): return n+n\n").unwrap();
        h.save();
    }
    let state = h.root().join(".chrono-harness/state");
    let selected = state.join("selected Git λ");
    let trace = state.join("git-trace");
    let (exit, report) = run_bound(&h, "integration", None);
    passed(exit, &report);
    assert_eq!(workflow(&report)["mode"], "integration_run");
    let (_, digest) = certificate(&h);
    let (exit, report) = run_bound(&h, "delivery", Some(&digest));
    passed(exit, &report);
    assert_eq!(workflow(&report)["mode"], "delivery");
    assert_eq!(report["judges"].as_array().unwrap().len(), 7);
    assert_eq!(report["git_facts"]["binding"]["path"], json!(selected));
    for judge in report["judges"].as_array().unwrap() {
        assert_eq!(judge["state"], "executed", "{}", judge["id"]);
        assert_eq!(
            judge["response"]["outputs"]["git_facts"]["binding"]["path"],
            json!(selected),
            "{}",
            judge["id"]
        );
        assert!(
            !judge["response"]["outputs"]["git_facts"]["processes"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    let trace = fs::read_to_string(trace).unwrap();
    for command in [
        "ls-tree",
        "rev-list",
        "check-ref-format",
        "rev-parse --show-toplevel",
        ":policy.json",
    ] {
        assert!(
            trace.contains(command),
            "missing actual consumer: {command}"
        );
    }
    assert_eq!(fs::read_to_string(state.join("order")).unwrap(), "ptpt");
}

#[test]
fn full_ci_prepared_context_runs_actual_integration_and_delivery_without_policy_substitution() {
    full_ci_consumer(false);
}

#[test]
fn selected_full_ci_prepares_and_consumes_unchanged_canonical_command() {
    full_ci_consumer(true);
}

const NATIVE_CONFIG: &str = ".chrono-harness/full native.json";
fn select(h: &mut Host, target: &str) {
    let config = h.values.remove(CONFIG).unwrap();
    h.values.insert(target.into(), config);
    h.values.insert(
        CONFIG.into(),
        json!({"schema":"chrono-git-configs/v1",
        "platforms":{format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH):target}}),
    );
    let mut f = file(target, json!([]));
    f["surface"] = json!("judge-policy");
    h.values.get_mut(FM).unwrap()["files"]
        .as_array_mut()
        .unwrap()
        .push(f);
}

fn full_ci_consumer(selected: bool) {
    let mut h = bound_host();
    if selected {
        select(&mut h, NATIVE_CONFIG);
    }
    let root = h.root();
    let linked_entry = ".chrono-harness/linked entry.json";
    let linked_directory = ".chrono-harness/linked directory";
    std::os::unix::fs::symlink("config.json", root.join(linked_entry)).unwrap();
    std::os::unix::fs::symlink(".", root.join(linked_directory)).unwrap();
    for path in [linked_entry, linked_directory] {
        let mut f = file(path, json!([]));
        f["surface"] = json!("documentation");
        h.values.get_mut(FM).unwrap()["files"]
            .as_array_mut()
            .unwrap()
            .push(f);
    }
    let ci = source().join("crates/ci/target/debug/chrono-ci");
    let source_path = ".chrono-harness/ci/full.json";
    let workflow_path = ".github/workflows/full.yml";
    let provider = json!({"schema":"chrono-github-full-ci/v1","name":"Full fixture",
        "workflow_path":workflow_path,"runs_on":"ubuntu-24.04","timeout_minutes":20,
        "checkout_action":"actions/checkout@11d5960a326750d5838078e36cf38b85af677262",
        "upload_artifact_action":"actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02",
        "bootstrap":["/bin/true"],"runner":".chrono-harness/bin/chrono-harness",
        "generator":".chrono-harness/bin/chrono-ci","check_config":CONFIG,
        "context_path":".chrono-harness/state/context.json",
        "preparation_path":".chrono-harness/state/preparation.json",
        "artifact_directory":".chrono-harness/state/"});
    fs::create_dir_all(root.join(".chrono-harness/ci")).unwrap();
    fs::write(
        root.join(source_path),
        serde_json::to_vec(&provider).unwrap(),
    )
    .unwrap();
    let generated = Command::new(&ci)
        .current_dir(&root)
        .args(["generate", "--host-root", ".", "--config", source_path])
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    for path in [source_path, workflow_path] {
        let mut f = file(path, json!([]));
        f["surface"] = json!("documentation");
        h.values.get_mut(FM).unwrap()["files"]
            .as_array_mut()
            .unwrap()
            .push(f);
    }
    h.save();
    h.base = h.candidate.clone();
    fs::write(root.join("p/product.py"), "def double(n): return 2*n\n").unwrap();
    h.save();
    let replay = |expected: i32| {
        let context = fs::read(root.join(".chrono-harness/state/context.json")).unwrap();
        let payload = root.join(".chrono-harness/state/payload.json");
        fs::write(
            &payload,
            serde_json::to_vec(
                &json!({"inputs":{"context":String::from_utf8(context.clone()).unwrap()}}),
            )
            .unwrap(),
        )
        .unwrap();
        let out = Command::new(&ci)
            .current_dir(&root)
            .args([
                "prepare",
                "--host-root",
                ".",
                "--config",
                source_path,
                "--event",
                "workflow_dispatch",
                "--payload",
                payload.to_str().unwrap(),
                "--workflow-revision",
                &h.candidate,
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let prepared: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(prepared["governance"], "not-evaluated");
        assert_eq!(prepared["canonical_argv"][3], CONFIG);
        if selected {
            assert_eq!(prepared["git_facts"]["selection"]["path"], CONFIG);
            assert_eq!(
                prepared["git_facts"]["selection"]["config_path"],
                NATIVE_CONFIG
            );
        }
        assert_eq!(
            fs::read(root.join(".chrono-harness/state/context.json")).unwrap(),
            context
        );
        let argv: Vec<String> = serde_json::from_value(prepared["canonical_argv"].clone()).unwrap();
        let out = Command::new(root.join(&argv[0]))
            .current_dir(&root)
            .env("DECLARED_EMPTY", "")
            .env_remove("DECLARED_ABSENT")
            .args(&argv[1..])
            .output()
            .unwrap();
        let report: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(out.status.code(), Some(expected), "{}", report["findings"]);
        fs::remove_file(root.join(".chrono-harness/state/preparation.json")).unwrap();
        report
    };
    let (exit, local) = run_bound(&h, "integration", None);
    passed(exit, &local);
    let integration = replay(0);
    passed(0, &integration);
    assert_eq!(workflow(&integration)["mode"], "integration_run");
    assert_eq!(integration["judges"].as_array().unwrap().len(), 7);
    let (_, digest) = certificate(&h);
    let (exit, local) = run_bound(&h, "delivery", Some(&digest));
    passed(exit, &local);
    let delivery = replay(0);
    passed(0, &delivery);
    assert_eq!(workflow(&delivery)["mode"], "delivery");
    assert_eq!(delivery["tests"]["tests"], local["tests"]["tests"]);
    assert_eq!(delivery["parity"]["status"], "unestablished");
    // Exercise the real CLI's root/config adaptation, including a caller cwd
    // outside the host and a relative path from a nested cwd. A direct policy
    // keeps its legacy alias behavior; a selector must reach the shared guard
    // with its supplied entry still intact, before Git or business operations.
    let invoke = |cwd: &std::path::Path, config: &str| {
        Command::new(root.join(".chrono-harness/bin/chrono-harness"))
            .current_dir(cwd)
            .env("DECLARED_EMPTY", "")
            .env_remove("DECLARED_ABSENT")
            .args([
                "check",
                "--config",
                config,
                "--base",
                &h.base,
                "--candidate",
                &h.candidate,
                "--context",
                ".chrono-harness/state/context.json",
            ])
            .output()
            .unwrap()
    };
    // Pair direct-policy path kinds with distinct caller contexts to avoid
    // redundant full judge chains. Selectors retain all nine combinations.
    for (path, direct_cwd) in [
        (CONFIG, root.clone()),
        (linked_entry, std::path::PathBuf::from("/")),
        (
            ".chrono-harness/linked directory/config.json",
            root.join("p"),
        ),
    ] {
        for (cwd, argument) in [
            (root.clone(), path.to_owned()),
            (
                std::path::PathBuf::from("/"),
                root.join(path).to_str().unwrap().to_owned(),
            ),
            (root.join("p"), format!("../{path}")),
        ] {
            if !selected && cwd != direct_cwd {
                continue;
            }
            let trace_path = root.join(".chrono-harness/state/git-trace");
            let order_path = root.join(".chrono-harness/state/order");
            let trace = fs::read(&trace_path).unwrap();
            let order = fs::read(&order_path).unwrap();
            let out = invoke(&cwd, &argument);
            if selected && path != CONFIG {
                assert_eq!(
                    out.status.code(),
                    Some(2),
                    "Q1: linked selector must fail before launch; Git launched={}, business launched={}; stderr={}",
                    fs::read(&trace_path).unwrap() != trace,
                    fs::read(&order_path).unwrap() != order,
                    String::from_utf8_lossy(&out.stderr)
                );
                assert!(
                    String::from_utf8_lossy(&out.stderr).contains("symlink path is not allowed:")
                );
                assert!(out.stdout.is_empty());
                assert_eq!(fs::read(&trace_path).unwrap(), trace);
                assert_eq!(fs::read(&order_path).unwrap(), order);
            } else {
                let report: Value = serde_json::from_slice(&out.stdout).unwrap();
                passed(out.status.code().unwrap(), &report);
                assert_eq!(
                    report["git_facts"]["config_path"],
                    if selected { NATIVE_CONFIG } else { CONFIG }
                );
                assert_eq!(report["tests"]["tests"]["test:t"], "passed");
                assert_ne!(fs::read(&trace_path).unwrap(), trace);
                assert_eq!(
                    fs::read(&order_path).unwrap(),
                    [order, b"pt".to_vec()].concat()
                );
            }
        }
    }
    if selected {
        // Full-CI preparation must reject the same literal linked entries.
        let original = fs::read(root.join(source_path)).unwrap();
        for path in [linked_entry, ".chrono-harness/linked directory/config.json"] {
            let mut linked_provider = provider.clone();
            linked_provider["check_config"] = json!(path);
            fs::write(
                root.join(source_path),
                serde_json::to_vec(&linked_provider).unwrap(),
            )
            .unwrap();
            let trace = fs::read(root.join(".chrono-harness/state/git-trace")).unwrap();
            let order = fs::read(root.join(".chrono-harness/state/order")).unwrap();
            let out = Command::new(&ci)
                .current_dir(&root)
                .args([
                    "prepare",
                    "--host-root",
                    ".",
                    "--config",
                    source_path,
                    "--event",
                    "workflow_dispatch",
                    "--payload",
                    ".chrono-harness/state/payload.json",
                    "--workflow-revision",
                    &h.candidate,
                ])
                .output()
                .unwrap();
            assert_eq!(out.status.code(), Some(1));
            assert!(String::from_utf8_lossy(&out.stderr).contains("symlink path is not allowed:"));
            assert_eq!(
                fs::read(root.join(".chrono-harness/state/git-trace")).unwrap(),
                trace
            );
            assert_eq!(
                fs::read(root.join(".chrono-harness/state/order")).unwrap(),
                order
            );
            assert!(!root.join(".chrono-harness/state/preparation.json").exists());
        }
        fs::write(root.join(source_path), original).unwrap();
    }
    // Preparation transports an invalid freshness claim; only workflow judges it.
    let context_path = root.join(".chrono-harness/state/context.json");
    let mut stale: Value = serde_json::from_slice(&fs::read(&context_path).unwrap()).unwrap();
    stale["observed_at"] = json!("2030-01-01T01:00:00Z");
    fs::write(context_path, serde_json::to_vec(&stale).unwrap()).unwrap();
    let rejected = replay(1);
    assert!(
        rejected["findings"].to_string().contains("E_BRANCH_STALE"),
        "{}",
        rejected["findings"]
    );
}

#[test]
fn full_unit_real_seven_judge_contribution() {
    let mut h = bound_host();
    h.values.get_mut(CONFIG).unwrap()["schema_version"] = json!(3);
    h.values.get_mut(CONFIG).unwrap()["protocol"]["timeout_seconds"] = json!(180);
    h.values.get_mut(CONFIG).unwrap()["execution_units"] = json!({
      "units":{"one":{"tests":["test:t"],"report_path":".chrono-harness/state/unit-one.json"}},
      "shared_operations":{},
      "collection_limits":{"manifest_bytes":1048576,"report_bytes":67108864},
      "report_path":".chrono-harness/state/collected.json"
    });
    h.save();
    h.base = h.candidate.clone();
    fs::write(h.root().join("p/product.py"), "def double(n): return 2*n\n").unwrap();
    h.save();
    let (control_exit, control) = run_bound(&h, "integration", None);
    fs::write(
        h.root().join(".chrono-harness/state/control.json"),
        serde_json::to_vec(&control).unwrap(),
    )
    .unwrap();
    println!(
        "DIRECT_CONTROL exit={control_exit} status={} findings={} effects={:?}",
        control["status"],
        control["findings"],
        fs::read_to_string(h.root().join(".chrono-harness/state/order"))
    );
    assert_eq!(control_exit, 0, "direct control failed");
    let certificate = h.root().join(".chrono-harness/state/integration.json");
    fs::remove_file(&certificate).unwrap();
    let out = Command::new(h.root().join(".chrono-harness/bin/chrono-harness"))
        .current_dir("/")
        .env("DECLARED_EMPTY", "")
        .env_remove("DECLARED_ABSENT")
        .args([
            "check",
            "--config",
            h.root().join(CONFIG).to_str().unwrap(),
            "--base",
            &h.base,
            "--candidate",
            &h.candidate,
            "--context",
            h.root()
                .join(".chrono-harness/state/context.json")
                .to_str()
                .unwrap(),
            "--unit",
            "one",
        ])
        .output()
        .unwrap();
    fs::write(
        h.root().join(".chrono-harness/state/unit-probe.json"),
        &out.stdout,
    )
    .unwrap();
    let unit: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|_| json!({"stderr":String::from_utf8_lossy(&out.stderr)}));
    println!(
        "FULL_UNIT exit={:?} status={} findings={} results_scope={} effects={:?} certificate={}",
        out.status.code(),
        unit["status"],
        unit["findings"],
        unit["tests"]["scope"],
        fs::read_to_string(h.root().join(".chrono-harness/state/order")),
        certificate.exists()
    );
    for row in unit["judges"].as_array().unwrap() {
        println!(
            "JUDGE {} state={} status={} findings={}",
            row["id"], row["state"], row["response"]["status"], row["response"]["findings"]
        );
    }

    let manifest_path = ".chrono-harness/state/full-manifest.json";
    let unit_bytes = fs::read(h.root().join(".chrono-harness/state/unit-one.json")).unwrap();
    println!("REPORT_SIZE unit_bytes={}", unit_bytes.len());
    fs::write(h.root().join(manifest_path),serde_json::to_vec(&json!({"schema":"chrono-full-collection/v1","reports":[{"unit":"one","path":".chrono-harness/state/unit-one.json","sha256":sha256(&unit_bytes)}]})).unwrap()).unwrap();
    let before = fs::read(h.root().join(".chrono-harness/state/order")).unwrap();
    let collected = Command::new(h.root().join(".chrono-harness/bin/chrono-harness"))
        .current_dir("/")
        .env("DECLARED_EMPTY", "")
        .env_remove("DECLARED_ABSENT")
        .args([
            "check",
            "--config",
            h.root().join(CONFIG).to_str().unwrap(),
            "--base",
            &h.base,
            "--candidate",
            &h.candidate,
            "--context",
            h.root()
                .join(".chrono-harness/state/context.json")
                .to_str()
                .unwrap(),
            "--collect",
            manifest_path,
        ])
        .output()
        .unwrap();
    fs::write(
        h.root().join(".chrono-harness/state/collect-probe.json"),
        &collected.stdout,
    )
    .unwrap();
    let collection: Value = serde_json::from_slice(&collected.stdout)
        .unwrap_or_else(|_| json!({"stderr":String::from_utf8_lossy(&collected.stderr)}));
    let after = fs::read(h.root().join(".chrono-harness/state/order")).unwrap();
    println!(
        "COLLECT_FAILED_UNIT exit={:?} status={} findings={} additional_business_bytes={}",
        collected.status.code(),
        collection["status"],
        collection["findings"],
        after.len() - before.len()
    );
    assert_eq!(
        before, after,
        "collector must not rerun business operations"
    );
    assert_eq!(
        collected.status.code(),
        Some(0),
        "valid collected evidence must pass: {}",
        collection["findings"]
    );
    assert_eq!(workflow(&collection)["mode"], "integration_run");
    let (_, digest) = super::certificate(&h);
    let (exit, delivery) = run_bound(&h, "delivery", Some(&digest));
    passed(exit, &delivery);
    println!("DELIVERY exit={exit} mode={}", workflow(&delivery)["mode"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "valid unit contribution must pass"
    );
}

#[test]
fn unit_report_must_not_target_canonical_full_report() {
    let profile = json!({"schema_version":3,"execution_units":{
        "units":{"one":{"tests":["test:t"],"report_path":".chrono-harness/state/report.json"}},
        "shared_operations":{},
        "collection_limits":{"manifest_bytes":1048576,"report_bytes":67108864},
        "report_path":".chrono-harness/state/collected.json"
    }});
    let actual = chrono_harness::units::full_execution_units(&profile);
    println!("CANONICAL_UNIT_REPORT_VALIDATION {actual:?}");
    assert!(
        actual.is_err(),
        "unit report must not overwrite canonical full report"
    );
}
