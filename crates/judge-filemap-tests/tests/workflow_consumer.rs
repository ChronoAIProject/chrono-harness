#[path = "../../judge-projects-tests/tests/support/host.rs"]
mod host;
#[path = "support/mod.rs"]
mod support;
use host::Host;
use serde_json::json;
use std::fs;
use support::*;

fn stable_host(test: &str) -> Host {
    let mut h = Host::new(true);
    h.values.get_mut(WORKFLOW).unwrap()["stability"] = json!([{"id":"api","paths":["src.bin"],"tests":[test],"reason":"explicit independent stable surface"}]);
    h.values.get_mut(WORKFLOW).unwrap()["integration"]["tests"] = json!([test]);
    h.save();
    h.base = h.candidate.clone();
    fs::write(h.root().join("src.bin"), "changed stable surface").unwrap();
    h.save();
    h
}

#[test]
fn actual_stability_selection_executes_shared_methods_exactly_once() {
    let h = stable_host("t");
    let (code, r) = h.run(|_| {});
    assert_eq!(code, 0, "{}", r["findings"]);
    assert_eq!(r["impact"]["schema"], "chrono-filemap-impact/v2");
    assert_eq!(r["impact"]["tests"], json!(["test:t"]));
    assert_eq!(r["impact"]["workflow"]["integration_required"], true);
    assert_eq!(r["tests"]["tests"]["test:t"], "passed");
    let operations = r["tests"]["executed"].as_array().unwrap();
    assert_eq!(
        operations
            .iter()
            .map(|r| r["operation"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["prepare.p", "execute.t"]
    );
    assert_eq!(
        fs::read_to_string(h.root().join(".chrono-harness/state/order")).unwrap(),
        "pt"
    );
    assert_eq!(
        r["sources"]["/impact"],
        json!([
            "/judges/1/response/outputs/impact",
            "/judges/2/response/outputs/impact"
        ])
    );
    for pointer in r["sources"]["/impact"].as_array().unwrap() {
        assert_eq!(r.pointer(pointer.as_str().unwrap()).unwrap(), &r["impact"]);
    }
}

#[test]
fn actual_selected_missing_test_blocks_operations_without_fallback() {
    let h = stable_host("missing");
    let (code, r) = h.run(|_| {});
    assert_ne!(code, 0);
    assert!(
        r["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["code"] == "E_DANGLING_EDGE")
    );
    assert!(!h.root().join(".chrono-harness/state/order").exists());
    assert!(r["tests"].is_null());
}

#[test]
fn actual_docs_do_not_run_an_unrelated_integration_suite() {
    let mut h = stable_host("missing");
    h.base = h.candidate.clone();
    fs::write(h.root().join("doc.txt"), "documentation only").unwrap();
    h.save();
    let (code, r) = h.run(|_| {});
    assert_eq!(code, 0, "{}", r["findings"]);
    assert_eq!(r["impact"]["workflow"]["integration_required"], false);
    assert_eq!(r["tests"]["executed"], json!([]));
    assert!(!h.root().join(".chrono-harness/state/order").exists());
}

fn add_extra_pair(h: &mut Host) {
    let p = h.values.get_mut(PROJECTS).unwrap();
    p["owners"]
        .as_array_mut()
        .unwrap()
        .extend([json!("extra"), json!("extra-tests")]);
    p["scripts"].as_array_mut().unwrap().extend([
        json!({"id":"extra","path":"extra/product.py","test_script":"extra-tests","actions":{"execute":{"operation":"extra.product","tool":"python","argv":["-B","extra/product.py"]}}}),
        json!({"id":"extra-tests","path":"extra/test.py","tests_for":"extra","actions":{"execute":{"operation":"extra.test","tool":"python","argv":["-B","extra/test.py"]}}})]);
    let fm = h.values.get_mut(FM).unwrap();
    for (path, owner) in [
        ("extra/product.py", "extra"),
        ("extra/test.py", "extra-tests"),
    ] {
        let mut f = file(
            path,
            json!([{"kind":"runtime-input","to":format!("script:{owner}")} ]),
        );
        f["owner"] = owner.into();
        fm["files"].as_array_mut().unwrap().push(f);
    }
    fm["project_edges"].as_array_mut().unwrap().extend([
        edge("script:extra", "test-execution", "test:extra-tests"),
        edge("script:extra-tests", "test-execution", "test:extra-tests"),
        edge("input:interpreter", "runtime-input", "script:extra"),
        edge("input:interpreter", "runtime-input", "script:extra-tests"),
    ]);
    fm["test_costs"]
        .as_array_mut()
        .unwrap()
        .push(json!({"test":"extra-tests","cost":"unknown"}));
    fm["execution_plans"]["test:extra-tests"] = json!({"operations":["prepare.p","extra.test"],"timeout_seconds":15,"output_limit_bytes":4096});
    fs::create_dir(h.root().join("extra")).unwrap();
    fs::write(
        h.root().join("extra/product.py"),
        "def answer(): return 42\n",
    )
    .unwrap();
    fs::write(h.root().join("extra/test.py"),"import runpy\nassert runpy.run_path('extra/product.py')['answer']()==42\nassert open('.chrono-harness/state/order').read().count('p')==1\nopen('.chrono-harness/state/extra','w').write('passed')\n").unwrap();
}

#[test]
fn actual_semantic_change_runs_additional_declared_suite_with_one_shared_prerequisite() {
    let mut h = Host::new(true);
    add_extra_pair(&mut h);
    for f in h.values.get_mut(FM).unwrap()["files"]
        .as_array_mut()
        .unwrap()
    {
        if f["path"] == PROJECTS {
            f["surface"] = "membership".into();
        }
    }
    h.values.get_mut(CONFIG).unwrap()["semantic_fields"] =
        json!([{"path":PROJECTS,"pointers":["/scripts/*/actions"],"on":"add-modify-delete"}]);
    h.values.get_mut(WORKFLOW).unwrap()["integration"]["tests"] = json!(["extra-tests"]);
    h.save();
    h.base = h.candidate.clone();
    h.values.get_mut(PROJECTS).unwrap()["scripts"][0]["actions"]["execute"]["argv"] = json!([
        "-c",
        "open('.chrono-harness/state/order','a').write('p') # changed method"
    ]);
    h.save();
    let (code, r) = h.run(|_| {});
    assert_eq!(code, 0, "{}", r["findings"]);
    assert_eq!(
        r["tests"]["tests"],
        json!({"test:t":"passed","test:extra-tests":"passed"})
    );
    assert_eq!(r["tests"]["executed"].as_array().unwrap().len(), 3);
    assert_eq!(
        fs::read_to_string(h.root().join(".chrono-harness/state/order")).unwrap(),
        "pt"
    );
    assert_eq!(
        fs::read_to_string(h.root().join(".chrono-harness/state/extra")).unwrap(),
        "passed"
    );
    let requirements = r["impact"]["workflow"]["requirements"].as_array().unwrap();
    assert!(
        requirements
            .iter()
            .any(|q| q["reason"] == "integration" && q["tests"] == json!(["test:extra-tests"]))
    );
    let plan = &r["judges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|j| j["id"] == "routes")
        .unwrap()["response"]["outputs"]["execution_plan"];
    assert_eq!(
        plan["selected"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
        vec!["test:extra-tests", "test:t"]
    );
}
