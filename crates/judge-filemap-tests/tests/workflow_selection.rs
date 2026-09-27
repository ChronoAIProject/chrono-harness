#[path = "support/mod.rs"]
mod support;
use chrono_judge_filemap::produce;
use serde_json::{Value, json};
use support::*;

fn fixture() -> Values {
    let mut v = values();
    v.get_mut(FM).unwrap()["project_edges"] = json!([]);
    for f in v.get_mut(FM).unwrap()["files"].as_array_mut().unwrap() {
        f["edges"] = json!([]);
        f["surface"] = match f["path"].as_str().unwrap() {
            CONFIG | JUDGES | WORKFLOW => "judge-policy",
            PROJECTS | FM => "membership",
            "doc.txt" => "documentation",
            _ => "product",
        }
        .into();
    }
    v.get_mut(CONFIG).unwrap()["semantic_fields"] = json!([
        {"path":PROJECTS,"pointers":["/projects/*/actions"],"on":"add-modify-delete"},
        {"path":FM,"pointers":["/files/*/surface"],"on":"modify-delete-existing"}]);
    v.get_mut(WORKFLOW).unwrap()["integration"]["tests"] = json!(["t"]);
    v
}
fn report(a: &Values, b: &Values, paths: &[&str]) -> (Value, Vec<chrono_harness::wire::Finding>) {
    let (impact, findings) = produce(
        &load(a),
        &load(b),
        CONFIG,
        &paths.iter().map(|p| delta(p)).collect::<Vec<_>>(),
    );
    (serde_json::to_value(impact).unwrap(), findings)
}
fn stability(v: &mut Values, tests: Value) {
    v.get_mut(WORKFLOW).unwrap()["stability"] =
        json!([{"id":"engine","paths":["src.bin"],"tests":tests,"reason":"explicit stable API"}]);
}

#[test]
fn registered_stability_selects_tests_without_any_filemap_test_edge() {
    let mut v = fixture();
    stability(&mut v, json!(["t"]));
    let (r, f) = report(&v, &v, &["src.bin"]);
    assert!(f.is_empty(), "{f:?}");
    assert_eq!(r["tests"], json!(["test:t"]));
    assert_eq!(r["workflow"]["integration_required"], true);
    assert_eq!(r["workflow"]["rule_changes"]["rule_paths"], json!([]));
    assert!(
        r["workflow"]["requirements"]
            .as_array()
            .unwrap()
            .iter()
            .any(|q| q["pointer"] == "/stability/0"
                && q["registry"] == WORKFLOW
                && q["trigger_paths"] == json!(["src.bin"]))
    );
    assert!(
        r["required_tests"][0]["execution_edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["from"] == "file:src.bin" && e["kind"] == "test-execution")
    );
}

#[test]
fn semantic_actions_require_the_declared_suite_but_membership_and_docs_do_not() {
    let a = fixture();
    let mut b = a.clone();
    b.get_mut(PROJECTS).unwrap()["projects"][0]["actions"]["execute"]["argv"] = json!(["changed"]);
    let (r, f) = report(&a, &b, &[PROJECTS]);
    assert!(f.is_empty(), "{f:?}");
    assert_eq!(r["tests"], json!(["test:t"]));
    assert_eq!(
        r["workflow"]["rule_changes"]["rule_paths"],
        json!([PROJECTS])
    );
    let mut b = a.clone();
    b.get_mut(FM).unwrap()["files"]
        .as_array_mut()
        .unwrap()
        .push(file("new.rs", json!([])));
    for paths in [vec!["src.bin", FM], vec!["doc.txt"], vec![]] {
        let (r, f) = report(&a, &b, &paths);
        assert!(f.is_empty(), "{f:?}");
        assert_eq!(r["tests"], json!([]));
        assert_eq!(r["workflow"]["integration_required"], false);
    }
}

#[test]
fn removed_stability_and_suite_requirements_keep_base_edges_and_costs() {
    let mut a = fixture();
    stability(&mut a, json!(["t"]));
    let mut b = a.clone();
    b.get_mut(WORKFLOW).unwrap()["stability"] = json!([]);
    b.get_mut(WORKFLOW).unwrap()["integration"]["tests"] = json!([]);
    b.get_mut(PROJECTS).unwrap()["projects"]
        .as_array_mut()
        .unwrap()
        .retain(|p| p["id"] != "t");
    b.get_mut(FM).unwrap()["test_costs"] = json!([]);
    let (r, f) = report(&a, &b, &[WORKFLOW]);
    assert!(f.is_empty(), "{f:?}");
    assert_eq!(r["retired_tests"], json!(["test:t"]));
    assert_eq!(r["required_tests"][0]["base_cost"]["reference"], "unknown");
    assert!(r["required_tests"][0]["candidate_cost"].is_null());
    assert!(
        r["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["from"] == format!("file:{WORKFLOW}")
                && e["to"] == "test:t"
                && e["origin"] == "base")
    );
}

#[test]
fn selected_invalid_stability_is_an_error_but_disconnected_historical_rule_is_not() {
    let mut a = fixture();
    stability(&mut a, json!(["missing"]));
    let (_, f) = report(&a, &a, &["src.bin"]);
    assert!(f.iter().any(|f| f.code == "E_DANGLING_EDGE"));
    let (r, f) = report(&a, &a, &["doc.txt"]);
    assert!(f.is_empty(), "{f:?}");
    assert_eq!(r["tests"], json!([]));
}

#[test]
fn reordering_identity_rows_does_not_activate_the_integration_suite() {
    let mut a = fixture();
    stability(&mut a, json!(["t"]));
    let mut b = a.clone();
    b.get_mut(PROJECTS).unwrap()["projects"]
        .as_array_mut()
        .unwrap()
        .reverse();
    b.get_mut(FM).unwrap()["files"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let (r, f) = report(&a, &b, &[PROJECTS, FM]);
    assert!(f.is_empty(), "{f:?}");
    assert_eq!(r["tests"], json!([]));
    assert_eq!(r["workflow"]["integration_required"], false);
}

#[test]
fn disabling_semantic_requirement_does_not_erase_the_old_policy() {
    let a = fixture();
    let mut b = a.clone();
    b.get_mut(WORKFLOW).unwrap()["semantic_changes_require_integration"] = false.into();
    b.get_mut(WORKFLOW).unwrap()["integration"]["tests"] = json!([]);
    let (r, f) = report(&a, &b, &[WORKFLOW]);
    assert!(f.is_empty(), "{f:?}");
    assert_eq!(r["tests"], json!(["test:t"]));
    assert_eq!(r["workflow"]["integration_required"], true);
}

#[test]
fn new_and_deleted_rule_paths_can_trigger_suites_at_the_other_endpoint() {
    for add in [true, false] {
        let a = fixture();
        let mut b = a.clone();
        let mut f = file("new-rule", json!([]));
        f["surface"] = "judge-implementation".into();
        b.get_mut(FM).unwrap()["files"]
            .as_array_mut()
            .unwrap()
            .push(f);
        let (a, b) = if add { (a, b) } else { (b, a) };
        let mut d = delta("new-rule");
        if add {
            d.kind = "A".into();
            d.old_blob = None;
            d.old_mode = None;
        } else {
            d.kind = "D".into();
            d.new_blob = None;
            d.new_mode = None;
        }
        let (impact, errors) = produce(&load(&a), &load(&b), CONFIG, &[d]);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(impact.tests, vec!["test:t"]);
        assert_eq!(impact.required_tests[0].execution_edges.len(), 1);
    }
}

#[test]
fn workflow_edges_do_not_hide_a_declared_dangling_filemap_edge() {
    let a = fixture();
    let mut b = a.clone();
    let mut f = file("new-rule", json!([]));
    f["surface"] = "judge-implementation".into();
    b.get_mut(FM).unwrap()["files"]
        .as_array_mut()
        .unwrap()
        .push(f);
    let mut a = a;
    a.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .push(edge("file:new-rule", "test-execution", "test:t"));
    let (_, errors) = report(&a, &b, &["new-rule"]);
    assert!(errors.iter().any(|f| {
        f.code == "E_DANGLING_EDGE"
            && f.message
                .contains("base: missing source node file:new-rule")
    }));
}
