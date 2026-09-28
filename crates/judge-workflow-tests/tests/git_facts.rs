use super::*;
use std::{os::unix::fs::PermissionsExt, process::Command};

#[test]
fn all_full_judges_and_integration_delivery_use_candidate_bound_git() {
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
    let cfg = h.values.get_mut(CONFIG).unwrap();
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
    let run_bound = |kind: &str, digest: Option<&str>| {
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
    };
    let (exit, report) = run_bound("integration", None);
    passed(exit, &report);
    assert_eq!(workflow(&report)["mode"], "integration_run");
    let (_, digest) = certificate(&h);
    let (exit, report) = run_bound("delivery", Some(&digest));
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
