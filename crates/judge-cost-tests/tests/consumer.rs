#[path = "../../judge-filemap-tests/tests/support/mod.rs"]
mod support;
use serde_json::json;
use std::fs;
use support::*;

#[path = "support/host.rs"]
mod host;
use host::Host;

#[test]
fn actual_cost_judge_reports_unknown_warning_with_zero_exit_and_source_fidelity() {
    let h = Host::new(true);
    let (exit, r) = h.run();
    assert_eq!(exit, 0, "status={} findings={}", r["status"], r["findings"]);
    assert_eq!(r["status"], "warn");
    assert!(
        r["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["code"] == "W_COST_UNKNOWN")
    );
    assert_eq!(
        r["sources"]["/costs"],
        json!(["/judges/2/response/outputs/costs"])
    );
    assert_eq!(r["costs"], r["judges"][2]["response"]["outputs"]["costs"]);
    assert_eq!(r["costs"]["binding"]["candidate"], h.candidate);
    assert_eq!(r["costs"]["affected_tests"], json!(["test:t"]));
    assert!(r["costs"]["declared_after"]["test:t"]["costs"][0]["value"]["cpu_ms"].is_null());
    assert_eq!(r["judges"][2]["process"]["exit_code"], 0);
}

#[test]
fn actual_known_costs_pass_and_docs_do_not_report_unrelated_projects() {
    let mut h = Host::new(false);
    let (exit, r) = h.run();
    assert_eq!(exit, 0, "status={} findings={}", r["status"], r["findings"]);
    assert_eq!(r["status"], "pass");
    assert_eq!(
        r["costs"]["declared_after"]["test:t"]["costs"][0]["value"]["peak_rss_bytes"],
        3
    );
    h.base = h.candidate.clone();
    fs::write(h.root().join("doc.txt"), "documentation change").unwrap();
    h.save();
    let (exit, r) = h.run();
    assert_eq!(exit, 0, "status={} findings={}", r["status"], r["findings"]);
    assert_eq!(
        r["costs"]["declared_after"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        ["file:doc.txt"]
    );
    assert_eq!(r["costs"]["affected_tests"], json!([]));
}

#[test]
fn missing_impact_producer_fails_instead_of_emitting_empty_costs() {
    let mut h = Host::new(false);
    let js = &mut h.values.get_mut(JUDGES).unwrap()["judges"];
    js.as_array_mut().unwrap().retain(|j| j["id"] != "filemap");
    js[1]["after"] = json!(["registration"]);
    h.save();
    let (exit, r) = h.run();
    assert_eq!(exit, 2, "status={} findings={}", r["status"], r["findings"]);
    assert!(r["costs"].is_null());
    assert!(
        r["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["code"] == "E_IMPACT")
    );
}

#[test]
fn retained_environment_impact_with_empty_git_delta_can_warn_successfully() {
    let mut h = Host::new(true);
    h.values.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .push(edge("environment:PATH", "runtime-input", "project:p"));
    h.save();
    h.base = h.candidate.clone();
    let inputs = json!({"base":{"commit":h.base,"environment":{"PATH":"old-retained-value"},"files":{}},
        "candidate":{"commit":h.candidate,"environment":{"PATH":std::env::var("PATH").unwrap()},"files":{}}});
    let (exit, r) = h.run_with_inputs(Some(inputs));
    assert_eq!(exit, 0, "status={} findings={}", r["status"], r["findings"]);
    assert_eq!(r["delta"], json!([]));
    assert_eq!(r["status"], "warn");
    assert_eq!(r["costs"]["affected_tests"], json!(["test:t"]));
    assert!(
        r["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["code"] == "W_COST_UNKNOWN")
    );
}
