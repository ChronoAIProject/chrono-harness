use super::*;
use std::{os::unix::fs::PermissionsExt, process::Command};

fn bound_host() -> Host {
    let mut h = host();
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
    cfg["semantic_fields"]
        .as_array_mut()
        .unwrap()
        .push(json!({"path":"policy.json","pointers":["/limit"],"on":"add-modify-delete"}));
    h.values.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .push(edge(
            "input:git-bytes",
            "judge-trigger",
            "judge:registration",
        ));
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
fn run_bound(h: &Host, kind: &str, digest: Option<&str>) -> (i32, Value) {
    let root = h.root();
    let selected = root.join(".chrono-harness/state/selected Git λ");
    h.run_with_context(
        |retained| {
            for (endpoint, oid) in [("base", &h.base), ("candidate", &h.candidate)] {
                let cfg: Value = serde_json::from_slice(
                    &chrono_harness::facts::blob(&root, oid, CONFIG).unwrap(),
                )
                .unwrap();
                retained[endpoint]["schema"] = json!("chrono-input-snapshot/v2");
                retained[endpoint]["config_path"] = json!(CONFIG);
                retained[endpoint]["config_digest"] =
                    json!(chrono_harness::wire::digest(&cfg).unwrap());
                retained[endpoint]["files"]["git-bytes"] =
                    json!({"bytes":fs::read(&selected).unwrap()});
            }
        },
        |ctx| {
            ctx["schema_version"] = json!(2);
            ctx["run_kind"] = json!(kind);
            ctx["integration_evidence"] = json!(digest);
            if kind == "delivery" {
                ctx["branch_ref"] = json!("feature/bound-git");
            }
        },
    )
}

#[test]
fn all_full_judges_and_integration_delivery_use_candidate_bound_git() {
    let h = bound_host();
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
    let mut h = bound_host();
    let root = h.root();
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
