#[path = "full_units.rs"]
mod full_units;
#[path = "git_facts.rs"]
mod git_facts;
#[path = "../../judge-projects-tests/tests/support/host.rs"]
mod host;
#[path = "../../judge-filemap-tests/tests/support/mod.rs"]
mod support;
use chrono_harness::sha256;
use host::Host;
use serde_json::{Value, json};
use std::fs;
use support::*;
fn host() -> Host {
    let mut h = Host::new(true);
    for (id, deps) in [
        ("cost", vec!["registration", "filemap"]),
        ("mixed", vec!["registration", "filemap", "cost"]),
        (
            "workflow",
            vec![
                "registration",
                "filemap",
                "routes",
                "projects",
                "cost",
                "mixed",
            ],
        ),
    ] {
        let bytes =
            fs::read(source().join(format!("crates/judge-{id}/target/debug/chrono-judge-{id}")))
                .expect("build registered judge first");
        let path = format!(".chrono-harness/bin/chrono-judge-{id}");
        fs::write(h.root().join(&path), &bytes).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(h.root().join(&path), fs::Permissions::from_mode(0o755)).unwrap();
        }
        h.values.get_mut(JUDGES).unwrap()["judges"].as_array_mut().unwrap().push(json!({"id":id,"executable":path,"version":"0.1.0","sha256":sha256(&bytes),"argv":["--protocol","chrono-judge/v1"],"selector":"every-delta","after":deps,"modes":["evaluate"]}));
    }
    h.values.get_mut(JUDGES).unwrap()["migration_validator"] = "workflow".into();
    for f in h.values.get_mut(FM).unwrap()["files"]
        .as_array_mut()
        .unwrap()
    {
        f["surface"] = match f["path"].as_str().unwrap() {
            CONFIG | JUDGES | WORKFLOW => "judge-policy",
            FM | PROJECTS => "membership",
            "doc.txt" => "documentation",
            _ => "product",
        }
        .into();
    }
    h.values.get_mut(CONFIG).unwrap()["semantic_fields"] =
        json!([{ "path":PROJECTS,"pointers":["/scripts/*/actions"],"on":"add-modify-delete"}]);
    let w = h.values.get_mut(WORKFLOW).unwrap();
    w["integration"]["bind"] = json!([
        "base",
        "candidate_tree",
        "registry_digest",
        "executables",
        "tools",
        "environment",
        "effective_inputs",
        "required_tests",
        "results_digest"
    ]);
    w["integration"]["tests"] = json!(["t"]);
    w["stability"] = json!([{"id":"product","paths":["p/product.py"],"tests":["t"],"reason":"stability fixture"}]);
    h.save();
    h.base = h.candidate.clone();
    fs::write(h.root().join("p/product.py"), "def double(n): return 2*n\n").unwrap();
    h.save();
    h
}
fn run(h: &Host, kind: &str, digest: Option<&str>, edit: impl FnOnce(&mut Value)) -> (i32, Value) {
    h.run_with_context(
        |_| {},
        |ctx| {
            ctx["schema_version"] = 2.into();
            ctx["run_kind"] = kind.into();
            ctx["integration_evidence"] = digest.map(str::to_string).into();
            edit(ctx)
        },
    )
}
fn workflow(r: &Value) -> &Value {
    &r["judges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|j| j["id"] == "workflow")
        .unwrap()["response"]["outputs"]["workflow"]
}
fn passed(exit: i32, r: &Value) {
    assert_eq!(exit, 0, "findings={} stderr={}", r["findings"], r["stderr"]);
    assert!(
        matches!(r["status"].as_str(), Some("pass" | "warn")),
        "{r:#}"
    )
}
fn certificate(h: &Host) -> (Value, String) {
    let bytes = fs::read(h.root().join(".chrono-harness/state/integration.json")).unwrap();
    (serde_json::from_slice(&bytes).unwrap(), sha256(&bytes))
}
#[test]
fn integration_then_delivery_preserves_actual_evidence_and_accepts_commit_metadata_change() {
    let mut h = host();
    let (e, r) = run(&h, "integration", None, |_| {});
    passed(e, &r);
    assert_eq!(workflow(&r)["mode"], "integration_run");
    let (c, d) = certificate(&h);
    assert_eq!(
        c["results_digest"],
        chrono_harness::wire::digest(&r["tests"]).unwrap()
    );
    assert_eq!(c["executables"].as_object().unwrap().len(), 8);
    let original = h.candidate.clone();
    h.candidate = commit(&h.root());
    assert_ne!(h.candidate, original);
    let (e, r) = run(&h, "delivery", Some(&d), |ctx| {
        ctx["branch_ref"] = "feature/topic".into()
    });
    passed(e, &r);
    assert_eq!(workflow(&r)["mode"], "delivery");
    assert_eq!(workflow(&r)["integration"]["commit_metadata_mapped"], true);
    assert_eq!(
        fs::read_to_string(h.root().join(".chrono-harness/state/order")).unwrap(),
        "ptpt"
    );
    assert_eq!(certificate(&h).1, d);
}
#[test]
fn delivery_from_integration_named_source_still_requires_successful_evidence() {
    let h = host();
    let (e, r) = run(&h, "delivery", None, |_| {});
    assert_eq!(e, 1, "{}", r["findings"]);
    assert!(r["findings"].to_string().contains("E_INTEGRATION_REQUIRED"));
    assert!(
        !h.root()
            .join(".chrono-harness/state/integration.json")
            .exists()
    );
}
#[test]
fn ordinary_doc_delta_runs_no_test_and_needs_no_integration() {
    let mut h = host();
    h.base = h.candidate.clone();
    fs::write(h.root().join("doc.txt"), "only prose").unwrap();
    h.save();
    let (e, r) = run(&h, "delivery", None, |ctx| {
        ctx["branch_ref"] = "feature/docs".into()
    });
    passed(e, &r);
    assert_eq!(workflow(&r)["mode"], "ordinary_delta");
    assert_eq!(r["tests"]["executed"], json!([]));
    assert!(!h.root().join(".chrono-harness/state/order").exists());
}

#[test]
fn no_delta_still_runs_the_full_workflow_gate_without_business_tests() {
    let mut h = host();
    h.base = h.candidate.clone();
    let (e, r) = run(&h, "delivery", None, |ctx| {
        ctx["branch_ref"] = "feature/no-delta".into()
    });
    passed(e, &r);
    assert_eq!(r["delta"], json!([]));
    assert_eq!(workflow(&r)["mode"], "ordinary_delta");
    assert_eq!(r["tests"]["executed"], json!([]));
    assert!(r["judges"].as_array().unwrap().iter().all(|judge| {
        judge["state"] == "executed" || judge["state"] == "completed" || judge["state"] == "pass"
    }));
    assert!(!h.root().join(".chrono-harness/state/order").exists());
}

#[test]
fn changed_tree_and_stale_producer_evidence_are_rejected() {
    let mut h = host();
    let (e, r) = run(&h, "integration", None, |_| {});
    passed(e, &r);
    let (_, d) = certificate(&h);
    let (e, r) = run(&h, "delivery", Some(&d), |ctx| {
        ctx["branch_ref"] = "feature/new".into();
        ctx["branch_started_at"] = "2026-01-02T00:00:00Z".into();
        ctx["observed_at"] = "2026-01-02T01:00:00Z".into();
    });
    assert_eq!(e, 1);
    assert!(r["findings"].to_string().contains("E_INTEGRATION_MISMATCH"));
    fs::write(h.root().join("doc.txt"), "new tree").unwrap();
    h.save();
    let (e, r) = run(&h, "delivery", Some(&d), |_| {});
    assert_eq!(e, 1);
    assert!(
        r["findings"]
            .to_string()
            .contains("different binding candidate_tree")
    );
}
#[test]
fn certificate_requires_retained_completed_runner_report_and_untampered_results() {
    let h = host();
    let (e, r) = run(&h, "integration", None, |_| {});
    passed(e, &r);
    let (c, d) = certificate(&h);
    let p = h
        .root()
        .join(c["producer"]["report_path"].as_str().unwrap());
    let bytes = fs::read(&p).unwrap();
    fs::write(&p, b"{}").unwrap();
    let (e, r) = run(&h, "delivery", Some(&d), |_| {});
    assert_eq!(e, 1);
    assert!(r["findings"].to_string().contains("E_INTEGRATION_MISMATCH"));
    fs::write(p, bytes).unwrap();
    let mut bad = c;
    bad["proof"]["results"]["executed"][0]["receipt"]["process"]["stdout_bytes"] = json!([88]);
    let bytes = chrono_harness::wire::canonical(&bad).unwrap();
    fs::write(
        h.root().join(".chrono-harness/state/integration.json"),
        &bytes,
    )
    .unwrap();
    let (e, r) = run(&h, "delivery", Some(&sha256(&bytes)), |_| {});
    assert_eq!(e, 1);
    assert!(r["findings"].to_string().contains("E_INTEGRATION_MISMATCH"));
}
fn retire(h: &mut Host, producer: bool) {
    h.values.get_mut(PROJECTS).unwrap()["scripts"] = json!([]);
    let fm = h.values.get_mut(FM).unwrap();
    fm["files"].as_array_mut().unwrap().retain(|f| {
        !f["path"].as_str().unwrap().starts_with("p/")
            && !f["path"].as_str().unwrap().starts_with("t/")
    });
    fm["project_edges"] = json!([]);
    fm["execution_plans"] = json!({});
    fm["test_costs"] = json!([]);
    fs::remove_dir_all(h.root().join("p")).unwrap();
    fs::remove_dir_all(h.root().join("t")).unwrap();
    let w = h.values.get_mut(WORKFLOW).unwrap();
    w["integration"]["tests"] = json!([]);
    w["stability"] = json!([]);
    w["retirements"] =
        json!([{"kind":"test","id":"t","replacement":null,"reason":"product removed"}]);
    if producer {
        w["retirements"]
            .as_array_mut()
            .unwrap()
            .push(json!({"kind":"script","id":"p","replacement":null,"reason":"product retired"}));
    }
    h.save();
}
#[test]
fn legal_joint_retirement_preserves_old_cost_without_executing_absent_test() {
    let mut h = host();
    retire(&mut h, true);
    let (e, r) = run(&h, "integration", None, |_| {});
    passed(e, &r);
    assert_eq!(r["impact"]["retired_tests"], json!(["test:t"]));
    assert_eq!(r["tests"]["removed"], json!({"test:t":null}));
    assert_eq!(r["tests"]["executed"], json!([]));
    assert!(r["costs"]["declared_before"].get("test:t").is_some());
    assert_eq!(
        workflow(&r)["transitions"]["retirements"][0]["status"],
        "retired"
    );
}
#[test]
fn retirement_request_without_joint_producer_declaration_fails_at_validator() {
    let mut h = host();
    retire(&mut h, false);
    let (e, r) = run(&h, "integration", None, |_| {});
    assert_eq!(e, 1, "{}", r["findings"]);
    assert!(r["findings"].to_string().contains("E_RETIREMENT"));
    assert!(workflow(&r).is_null());
}
#[test]
fn replacement_can_change_method_and_prerequisite_after_explicit_retirement() {
    let mut h = host();
    let p = h.values.get_mut(PROJECTS).unwrap();
    p["scripts"][0]["test_script"] = "replacement".into();
    p["scripts"][0]["actions"]["execute"]["operation"] = "prepare.next".into();
    p["scripts"][1]["id"] = "replacement".into();
    p["scripts"][1]["actions"]["execute"]["operation"] = "execute.replacement".into();
    p["owners"]
        .as_array_mut()
        .unwrap()
        .push("replacement".into());
    let fm = h.values.get_mut(FM).unwrap();
    for f in fm["files"].as_array_mut().unwrap() {
        if f["owner"] == "t" {
            f["owner"] = "replacement".into()
        }
        for e in f["edges"].as_array_mut().unwrap() {
            if e["to"] == "script:t" {
                e["to"] = "script:replacement".into()
            }
        }
    }
    for e in fm["project_edges"].as_array_mut().unwrap() {
        if e["from"] == "script:t" {
            e["from"] = "script:replacement".into()
        }
        if e["to"] == "test:t" {
            e["to"] = "test:replacement".into()
        }
    }
    fm["execution_plans"] = json!({"test:replacement":{"operations":["prepare.next","execute.replacement"],"timeout_seconds":15,"output_limit_bytes":4096}});
    fm["test_costs"][0]["test"] = "replacement".into();
    let w = h.values.get_mut(WORKFLOW).unwrap();
    w["integration"]["tests"] = json!(["replacement"]);
    w["stability"][0]["tests"] = json!(["replacement"]);
    w["retirements"] = json!([{"kind":"test","id":"t","replacement":"replacement","reason":"new method with same product purpose"}]);
    h.save();
    let (e, r) = run(&h, "integration", None, |_| {});
    passed(e, &r);
    assert_eq!(r["tests"]["removed"]["test:t"], "test:replacement");
    assert_eq!(r["tests"]["executed"][0]["operation"], "prepare.next");
    assert_eq!(
        r["tests"]["executed"][1]["operation"],
        "execute.replacement"
    );
}

#[test]
fn retired_judge_needs_declaration_but_never_needs_its_old_binary() {
    let mut h = host();
    h.values.get_mut(JUDGES).unwrap()["judges"].as_array_mut().unwrap().push(json!({"id":"obsolete","executable":".chrono-harness/bin/missing-old-judge","version":"old","sha256":"a".repeat(64),"argv":[],"selector":"every-delta","after":["registration"],"modes":["evaluate"]}));
    h.save();
    h.base = h.candidate.clone();
    h.values.get_mut(JUDGES).unwrap()["judges"]
        .as_array_mut()
        .unwrap()
        .retain(|j| j["id"] != "obsolete");
    h.save();
    let (e, r) = run(&h, "integration", None, |_| {});
    assert_eq!(e, 1);
    assert!(r["findings"].to_string().contains("E_RETIREMENT"));
    h.values.get_mut(WORKFLOW).unwrap()["retirements"] = json!([{"kind":"judge","id":"obsolete","replacement":null,"reason":"retire unavailable old implementation"}]);
    h.save();
    let (e, r) = run(&h, "integration", None, |_| {});
    passed(e, &r);
    assert!(
        !h.root()
            .join(".chrono-harness/bin/missing-old-judge")
            .exists()
    );
    assert!(
        workflow(&r)["transitions"]["retirements"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["kind"] == "judge" && x["id"] == "obsolete")
    );
}
#[test]
fn configured_but_unexecuted_judge_cannot_be_certified() {
    let mut h = host();
    let mut extra = h.values[JUDGES]["judges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|j| j["id"] == "cost")
        .unwrap()
        .clone();
    extra["id"] = "zz-later".into();
    h.values.get_mut(JUDGES).unwrap()["judges"]
        .as_array_mut()
        .unwrap()
        .push(extra);
    h.save();
    let (e, r) = run(&h, "integration", None, |_| {});
    assert_eq!(e, 2, "{}", r["findings"]);
    assert!(
        r["findings"]
            .to_string()
            .contains("workflow must follow all configured judges")
    );
    assert!(
        !h.root()
            .join(".chrono-harness/state/integration.json")
            .exists()
    );
}
#[test]
fn failed_required_test_never_creates_integration_evidence() {
    let mut h = host();
    fs::write(
        h.root().join("p/product.py"),
        "def double(n): return n+n+1\n",
    )
    .unwrap();
    h.save();
    let (e, r) = run(&h, "integration", None, |_| {});
    assert_ne!(e, 0);
    assert!(
        r["judges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|j| j["id"] == "workflow" && j["state"] == "blocked")
    );
    assert!(
        !h.root()
            .join(".chrono-harness/state/integration.json")
            .exists()
    );
}
#[test]
fn changed_base_rejects_old_integration_binding() {
    let mut h = host();
    let (e, r) = run(&h, "integration", None, |_| {});
    passed(e, &r);
    let (_, d) = certificate(&h);
    h.base = h.candidate.clone();
    fs::write(
        h.root().join("p/product.py"),
        "def double(n): return 2*n+0\n",
    )
    .unwrap();
    h.save();
    let (e, r) = run(&h, "delivery", Some(&d), |_| {});
    assert_eq!(e, 1);
    assert!(r["findings"].to_string().contains("different binding base"));
}
fn migration_host() -> Host {
    let mut h = host();
    let decoder = fs::read(source().join(".chrono-harness/migrations/scoped-v1.py")).unwrap();
    let code = "import runpy,json,copy\nf=runpy.run_path('.chrono-harness/decoder.py')['convert']\nr=json.load(open('.chrono-harness/case.json'))\no=f(r)\nassert o['values']['.chrono-harness/FILEMAP.json']['schema_version']==2\nassert o['values']['.chrono-harness/FILEMAP.json']['execution_plans']['test:t']['operations']==['prepare.p','execute.t']\nbad=copy.deepcopy(r)\nbad['profile_value']['schema']='unsupported'\ntry:\n f(bad)\nexcept ValueError:\n pass\nelse:\n raise AssertionError('bad profile accepted')\n";
    for (path, bytes) in [
        (".chrono-harness/decoder.py", decoder.as_slice()),
        (".chrono-harness/decoder-tests.py", code.as_bytes()),
        (".chrono-harness/case.json", b"{}".as_slice()),
        (".chrono-harness/profile.json", b"{}".as_slice()),
    ] {
        fs::write(h.root().join(path), bytes).unwrap();
        h.values.get_mut(FM).unwrap()["files"]
            .as_array_mut()
            .unwrap()
            .push(file(path, json!([])));
    }
    for (path, owner) in [
        (".chrono-harness/decoder.py", "decoder"),
        (".chrono-harness/decoder-tests.py", "decoder-tests"),
    ] {
        let row = h.values.get_mut(FM).unwrap()["files"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|f| f["path"] == path)
            .unwrap();
        row["owner"] = owner.into();
    }
    let p = h.values.get_mut(PROJECTS).unwrap();
    p["owners"]
        .as_array_mut()
        .unwrap()
        .extend([json!("decoder"), json!("decoder-tests")]);
    p["scripts"].as_array_mut().unwrap().extend([
        json!({"id":"decoder","path":".chrono-harness/decoder.py","test_script":"decoder-tests","actions":{"execute":{"operation":"decode","tool":"python","argv":["-B",".chrono-harness/decoder.py"]}}}),
        json!({"id":"decoder-tests","path":".chrono-harness/decoder-tests.py","tests_for":"decoder","actions":{"execute":{"operation":"decode.test","tool":"python","argv":["-B",".chrono-harness/decoder-tests.py"]}}})]);
    h.values.get_mut(FM).unwrap()["execution_plans"]["test:decoder-tests"] =
        json!({"operations":["decode.test"],"timeout_seconds":15,"output_limit_bytes":4096});
    h.values.get_mut(FM).unwrap()["test_costs"]
        .as_array_mut()
        .unwrap()
        .push(json!({"test":"decoder-tests","cost":"unknown"}));
    let plans = h.values[FM]["execution_plans"].clone();
    h.values.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .push(edge(
            "script:decoder",
            "test-execution",
            "test:decoder-tests",
        ));
    let legacy = json!({"id":"legacy","path":".chrono-harness/legacy.py","actions":{"execute":{"operation":"legacy.old","tool":"python","argv":["-c","raise RuntimeError(\"historical code must not execute\")"]}}});
    let mappings = json!([{ "from":"script:legacy","to":"script:decoder"},{"from":"test:legacy","to":"test:decoder-tests"},{"from":"owner:legacy","to":"owner:decoder"}]);
    let mut bindings = plans
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v["operations"].clone()))
        .collect::<serde_json::Map<_, _>>();
    bindings.insert("test:legacy".into(), json!(["legacy.old"]));
    let profile = json!({"schema":"chrono-ci-check/v1","policy":{"filemap":FM,"projects":PROJECTS,"bindings":bindings,"operation_timeout_seconds":15,"operation_output_limit_bytes":4096}});
    let profile_bytes = serde_json::to_vec(&profile).unwrap();
    fs::write(
        h.root().join(".chrono-harness/profile.json"),
        &profile_bytes,
    )
    .unwrap();
    let descriptor = json!({"id":"chrono-ci-check/v1","filemap_version":1,"profile_path":".chrono-harness/profile.json","script":"decoder","test":"decoder-tests","mappings":mappings,"legacy_records":[{"collection":"scripts","id":"legacy","definition":legacy,"nodes":["script:legacy","test:legacy"]}],"ambiguities":[]});
    let w = h.values.get_mut(WORKFLOW).unwrap();
    w["schema_version"] = 2.into();
    w["historical_profiles"] = json!([descriptor]);
    w["integration"]["tests"] = json!(["t", "decoder-tests"]);
    w["migrations"] = json!([{"from_version":1,"to_version":2,"script":"decoder","test":"decoder-tests","mappings":mappings,"reason":"explicit FILEMAP execution plan migration with retired obsolete method"}]);
    h.values.get_mut(PROJECTS).unwrap()["scripts"]
        .as_array_mut()
        .unwrap()
        .push(legacy.clone());
    h.values.get_mut(PROJECTS).unwrap()["owners"]
        .as_array_mut()
        .unwrap()
        .push("legacy".into());
    h.values.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .push(edge("script:legacy", "test-execution", "test:legacy"));
    h.values.get_mut(FM).unwrap()["test_costs"]
        .as_array_mut()
        .unwrap()
        .push(json!({"test":"legacy","cost":"unknown"}));
    h.values.get_mut(FM).unwrap()["schema_version"] = 1.into();
    h.values
        .get_mut(FM)
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("execution_plans");
    h.save();
    h.base = h.candidate.clone();
    let raw = h.values.clone();
    let raw_bytes = raw
        .iter()
        .map(|(k, v)| (k.clone(), json!(serde_json::to_vec(v).unwrap())))
        .collect::<serde_json::Map<_, _>>();
    let case = json!({"schema":"chrono-historical-decode/v1","config_path":CONFIG,"original":raw,"original_bytes":raw_bytes,"profile":descriptor,"profile_bytes":profile_bytes,"profile_value":profile});
    fs::write(
        h.root().join(".chrono-harness/case.json"),
        serde_json::to_vec(&case).unwrap(),
    )
    .unwrap();
    h.values.get_mut(PROJECTS).unwrap()["scripts"]
        .as_array_mut()
        .unwrap()
        .retain(|v| v["id"] != "legacy");
    h.values.get_mut(PROJECTS).unwrap()["owners"]
        .as_array_mut()
        .unwrap()
        .retain(|v| v != "legacy");
    h.values.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .retain(|v| v["from"] != "script:legacy");
    h.values.get_mut(FM).unwrap()["test_costs"]
        .as_array_mut()
        .unwrap()
        .retain(|v| v["test"] != "legacy");
    h.values.get_mut(WORKFLOW).unwrap()["retirements"] = json!([{"kind":"test","id":"legacy","replacement":"decoder-tests","reason":"current compatibility contract replaces obsolete method"},{"kind":"script","id":"legacy","replacement":"decoder","reason":"candidate decoder preserves old facts"}]);
    h.values.get_mut(FM).unwrap()["schema_version"] = 2.into();
    h.values.get_mut(FM).unwrap()["execution_plans"] = plans;
    h.save();
    h
}

#[test]
fn candidate_decoder_and_real_compatibility_test_certify_the_declared_schema_transition() {
    let mut h = migration_host();
    let (e, r) = run(&h, "integration", None, |_| {});
    passed(e, &r);
    assert_eq!(r["tests"]["tests"]["test:decoder-tests"], "passed");
    assert_eq!(r["tests"]["removed"]["test:legacy"], "test:decoder-tests");
    assert_eq!(
        r["costs"]["declared_before"]["script:legacy"]["kind"],
        "script"
    );
    assert!(
        !r["tests"]["executed"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["operation"] == "legacy.old")
    );
    assert_eq!(
        workflow(&r)["transitions"]["migrations"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    h.values.get_mut(WORKFLOW).unwrap()["migrations"][0]["script"] = "p".into();
    h.values.get_mut(WORKFLOW).unwrap()["migrations"][0]["test"] = "t".into();
    h.save();
    let (e, r) = run(&h, "integration", None, |_| {});
    assert_eq!(
        e, 2,
        "an unrelated successful pair cannot certify conversion: {}",
        r["findings"]
    );
    assert!(r["findings"].to_string().contains("E_MIGRATION_EVIDENCE"));
    h.values.get_mut(WORKFLOW).unwrap()["migrations"] = json!([]);
    h.save();
    let (e, r) = run(&h, "integration", None, |_| {});
    assert_eq!(e, 2);
    assert!(r["findings"].to_string().contains("E_MIGRATION_EVIDENCE"));
}

#[test]
fn migration_artifacts_are_allowed_but_candidate_input_mutations_are_rejected() {
    for path in [
        ".chrono-harness/state/decoder-output",
        "unknown-input",
        "doc.txt",
    ] {
        let mut h = migration_host();
        let decoder_path = h.root().join(".chrono-harness/decoder.py");
        let decoder = fs::read_to_string(&decoder_path).unwrap();
        fs::write(
            &decoder_path,
            format!(
                "from pathlib import Path\nPath({path:?}).write_text('decoder output')\n{decoder}"
            ),
        )
        .unwrap();
        h.save();
        let (exit, report) = run(&h, "integration", None, |_| {});
        if path.starts_with(".chrono-harness/state/") {
            passed(exit, &report);
            assert_eq!(report["tests"]["tests"]["test:decoder-tests"], "passed");
            assert_eq!(
                fs::read_to_string(h.root().join(path)).unwrap(),
                "decoder output"
            );
        } else {
            assert_ne!(exit, 0, "decoder input mutation accepted: {path}");
            assert!(
                report["findings"]
                    .to_string()
                    .contains("migration changed candidate inputs"),
                "{path}: {}",
                report["findings"]
            );
            assert!(
                !h.root().join(".chrono-harness/state/order").exists(),
                "business operation ran after decoder mutation"
            );
        }
    }
}

#[test]
fn explicit_alias_repair_can_keep_the_alias_and_replace_the_obsolete_method() {
    let mut h = host();
    h.values.get_mut(WORKFLOW).unwrap()["schema_version"] = 2.into();
    h.values.get_mut(WORKFLOW).unwrap()["historical_profiles"] = json!([]);
    let mut projects = values()[PROJECTS]["projects"].clone();
    projects[0]["actions"] = json!({"build":{"operation":"old.build","tool":"python","argv":["-c","raise RuntimeError('obsolete build')"]}});
    projects[1]["actions"] = json!({"execute":{"operation":"old.test","tool":"python","argv":["-c","raise RuntimeError('obsolete test')"]}});
    h.values.get_mut(PROJECTS).unwrap()["projects"] = projects;
    h.save();
    h.base = h.candidate.clone();
    h.values.get_mut(PROJECTS).unwrap()["projects"] = json!([]);
    h.values.get_mut(WORKFLOW).unwrap()["historical_profiles"] = json!([{"id":"chrono-ci-check/v1","filemap_version":1,"profile_path":"profile.json","script":"p","test":"t","mappings":[],"legacy_records":[],"ambiguities":[{"node":"test:t","definitions":["project:t","script:t"],"replacement":"test:t"}]}]);
    h.values.get_mut(WORKFLOW).unwrap()["retirements"] = json!([{"kind":"project","id":"p","replacement":null,"reason":"obsolete project pair replaced by existing script pair"}]);
    h.save();
    let (e, r) = run(&h, "integration", None, |_| {});
    passed(e, &r);
    assert_eq!(r["tests"]["tests"]["test:t"], "passed");
    assert_eq!(
        workflow(&r)["transitions"]["ambiguities"][0]["node"],
        "test:t"
    );
    assert!(
        !r["tests"]["executed"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["operation"].as_str().unwrap().starts_with("old."))
    );
    h.values.get_mut(WORKFLOW).unwrap()["historical_profiles"][0]["ambiguities"] = json!([]);
    h.save();
    let (e, r) = run(&h, "integration", None, |_| {});
    assert_ne!(e, 0, "{r:#}");
    assert!(
        r["findings"].to_string().contains("E_NODE_AMBIGUOUS"),
        "{}",
        r["findings"]
    );
}

#[test]
fn certified_mixed_rule_product_delivery_remains_a_warning_without_acknowledgement() {
    let mut h = host();
    h.values.get_mut(WORKFLOW).unwrap()["mixed_change"]["code"] = "W_MIXED_JUDGE_PRODUCT".into();
    h.values.get_mut(WORKFLOW).unwrap()["staleness"]["max_age_hours"] = 25.into();
    h.save();
    let (e, r) = run(&h, "integration", None, |_| {});
    passed(e, &r);
    let (_, digest) = certificate(&h);
    let (e, r) = run(&h, "delivery", Some(&digest), |_| {});
    passed(e, &r);
    assert_eq!(r["status"], "warn");
    assert!(r["findings"].to_string().contains("W_MIXED_JUDGE_PRODUCT"));
    assert_eq!(workflow(&r)["mode"], "delivery");
}
