#[path = "../../judge-filemap-tests/tests/support/mod.rs"]
mod support;
use chrono_judge_mixed::produce;
use serde_json::{Value, json};
use support::*;

fn fixture() -> Values {
    let mut v = values();
    for f in v.get_mut(FM).unwrap()["files"].as_array_mut().unwrap() {
        f["surface"] = match f["path"].as_str().unwrap() {
            CONFIG | JUDGES | WORKFLOW => "judge-policy",
            FM | PROJECTS => "membership",
            "doc.txt" => "documentation",
            _ => "product",
        }
        .into();
    }
    v.get_mut(CONFIG).unwrap()["semantic_fields"] = json!([
        {"path":PROJECTS,"pointers":["/projects/*/actions","/scripts/*/actions"],"on":"add-modify-delete"},
        {"path":FM,"pointers":["/files/*/surface"],"on":"modify-delete-existing"}
    ]);
    v
}
fn report(a: &Values, b: &Values, paths: &[&str]) -> Result<Value, String> {
    let old = load(a);
    let new = load(b);
    let delta = paths.iter().map(|p| delta(p)).collect::<Vec<_>>();
    let (impact, errors) = chrono_judge_filemap::produce_with_documents(
        &old,
        &new,
        CONFIG,
        &delta,
        &[a.clone(), b.clone()],
    );
    if let Some(error) = errors.first() {
        return Err(error.message.clone());
    }
    let costs = chrono_judge_cost::produce(&old, &new, &impact).unwrap();
    produce(&old, &new, &impact.delta, a, b, &costs)
}

#[test]
fn ordinary_code_membership_and_edges_do_not_change_rules() {
    let a = fixture();
    let mut b = a.clone();
    b.get_mut(FM).unwrap()["files"]
        .as_array_mut()
        .unwrap()
        .push(file("new.rs", json!([])));
    b.get_mut(FM).unwrap()["files"][0]["edges"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"runtime-input","to":"project:p"}));
    let r = report(&a, &b, &["src.bin", FM]).unwrap();
    assert_eq!(r["mixed"], false);
    assert_eq!(r["rule_paths"], json!([]));
    assert_eq!(r["product_paths"], json!(["src.bin"]));
}

#[test]
fn true_action_change_is_mixed_and_retains_exact_cost_values() {
    let a = fixture();
    let mut b = a.clone();
    b.get_mut(PROJECTS).unwrap()["projects"][0]["actions"]["execute"]["argv"] =
        json!(["-c", "exit 1"]);
    b.get_mut(FM).unwrap()["cost_models"]["unknown"]["cpu_ms"] = 17.into();
    let r = report(&a, &b, &["src.bin", PROJECTS]).unwrap();
    assert_eq!(r["mixed"], true);
    assert_eq!(r["rule_paths"], json!([PROJECTS]));
    assert_eq!(r["product_paths"], json!(["src.bin"]));
    assert!(r["costs"]["declared_before"]["file:src.bin"]["costs"][0]["value"]["cpu_ms"].is_null());
    assert_eq!(
        r["costs"]["declared_after"]["file:src.bin"]["costs"][0]["value"]["cpu_ms"],
        17
    );
    assert_eq!(
        r["changes"][0]["semantic"][0]["before"]["pointer"],
        "/projects/0/actions"
    );
}

#[test]
fn policy_only_product_only_and_empty_are_not_mixed() {
    let a = fixture();
    for (paths, rules, products) in [
        (vec![CONFIG], vec![CONFIG], vec![]),
        (vec!["src.bin"], vec![], vec!["src.bin"]),
        (vec![], vec![], vec![]),
    ] {
        let r = report(&a, &a, &paths).unwrap();
        assert_eq!(r["mixed"], false);
        assert_eq!(r["rule_paths"], json!(rules));
        assert_eq!(r["product_paths"], json!(products));
    }
}

#[test]
fn old_surfaces_survive_reclassification_and_deleted_files() {
    let mut a = fixture();
    a.get_mut(FM).unwrap()["files"][0]["surface"] = "judge-implementation".into();
    let mut b = a.clone();
    b.get_mut(FM).unwrap()["files"][0]["surface"] = "product".into();
    let r = report(&a, &b, &["src.bin", FM]).unwrap();
    assert!(
        r["rule_paths"]
            .as_array()
            .unwrap()
            .contains(&json!("src.bin"))
    );
    assert_eq!(r["mixed"], true);
    b.get_mut(FM).unwrap()["files"]
        .as_array_mut()
        .unwrap()
        .retain(|f| f["path"] != "src.bin");
    let r = report(&a, &b, &["src.bin", FM]).unwrap();
    assert!(r["costs"]["declared_after"]["file:src.bin"].is_null());
    assert!(
        r["rule_paths"]
            .as_array()
            .unwrap()
            .contains(&json!("src.bin"))
    );
}

#[test]
fn row_reorder_is_neutral_but_ordered_arguments_change_rules() {
    let a = fixture();
    let mut b = a.clone();
    b.get_mut(PROJECTS).unwrap()["projects"]
        .as_array_mut()
        .unwrap()
        .reverse();
    b.get_mut(FM).unwrap()["files"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let r = report(&a, &b, &["src.bin", FM, PROJECTS]).unwrap();
    assert_eq!(r["rule_paths"], json!([]));
    b.get_mut(PROJECTS).unwrap()["projects"][0]["actions"]["execute"]["argv"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let r = report(&a, &b, &["src.bin", PROJECTS]).unwrap();
    assert_eq!(r["mixed"], true);
    assert_eq!(
        r["changes"][0]["semantic"][0]["before"]["pointer"],
        "/projects/1/actions"
    );
    assert_eq!(
        r["changes"][0]["semantic"][0]["after"]["pointer"],
        "/projects/0/actions"
    );
}

#[test]
fn script_identity_precedes_path_and_actions_added_are_semantic() {
    let mut a = fixture();
    script_pair(&mut a, "st");
    let mut b = a.clone();
    b.get_mut(PROJECTS).unwrap()["scripts"][0]["path"] = "other.sh".into();
    b.get_mut(PROJECTS).unwrap()["scripts"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let r = report(&a, &b, &["src.bin", PROJECTS]).unwrap();
    assert_eq!(r["rule_paths"], json!([]));
    b.get_mut(PROJECTS).unwrap()["scripts"][0]["actions"]["execute"]["argv"] =
        json!(["changed.sh"]);
    assert_eq!(
        report(&a, &b, &["src.bin", PROJECTS]).unwrap()["mixed"],
        true
    );
    let a = fixture();
    let mut b = a.clone();
    script_pair(&mut b, "st");
    assert_eq!(
        report(&a, &b, &["src.bin", PROJECTS]).unwrap()["mixed"],
        true
    );
}

#[test]
fn explicit_wildcards_escaped_keys_null_removal_and_named_documents() {
    let mut a = fixture();
    a.get_mut(CONFIG).unwrap()["semantic_fields"] = json!([{"path":"rules.json","pointers":["/a~1b/~0key/*/value"],"on":"modify-delete-existing"}]);
    let mut f = file("rules.json", json!([]));
    f["surface"] = "membership".into();
    a.get_mut(FM).unwrap()["files"]
        .as_array_mut()
        .unwrap()
        .push(f);
    a.insert(
        "rules.json".into(),
        json!({"a/b":{"~key":{"one":{"value":null},"two":{"nested":{"value":1}}}}}),
    );
    let mut b = a.clone();
    b.get_mut("rules.json").unwrap()["a/b"]["~key"]["two"]["nested"]["value"] = 2.into();
    b.get_mut("rules.json").unwrap()["a/b"]["~key"]["new"] = json!({"value":2});
    assert_eq!(
        report(&a, &b, &["src.bin", "rules.json"]).unwrap()["mixed"],
        false
    );
    b.get_mut("rules.json").unwrap()["a/b"]["~key"]["one"]
        .as_object_mut()
        .unwrap()
        .remove("value");
    let r = report(&a, &b, &["src.bin", "rules.json"]).unwrap();
    assert_eq!(r["mixed"], true);
    assert_eq!(
        r["changes"][0]["semantic"][0]["before"]["pointer"],
        "/a~1b/~0key/one/value"
    );
    assert!(r["changes"][0]["semantic"][0]["before"]["value"].is_null());
    assert!(r["changes"][0]["semantic"][0]["after"].is_null());
}

#[test]
fn old_patterns_survive_removal_and_invalid_pointers_or_ambiguous_rows_fail() {
    let a = fixture();
    let mut b = a.clone();
    b.get_mut(CONFIG).unwrap()["semantic_fields"] = json!([]);
    b.get_mut(PROJECTS).unwrap()["projects"][0]["actions"]["execute"]["argv"] =
        json!(["different"]);
    assert_eq!(
        report(&a, &b, &["src.bin", PROJECTS]).unwrap()["mixed"],
        true
    );
    let mut a = fixture();
    a.get_mut(CONFIG).unwrap()["semantic_fields"][0]["pointers"] = json!(["/projects/~2bad"]);
    assert!(
        report(&a, &a, &[PROJECTS])
            .unwrap_err()
            .contains("E_SEMANTIC_POINTER")
    );
    let mut a = fixture();
    a.get_mut(CONFIG).unwrap()["semantic_fields"] =
        json!([{"path":"doc.txt","pointers":["/rows"],"on":"add-modify-delete"}]);
    a.insert(
        "doc.txt".into(),
        json!({"rows":[{"id":"same","x":1},{"id":"same","x":2}]}),
    );
    assert!(
        report(&a, &a, &["doc.txt"])
            .unwrap_err()
            .contains("E_SEMANTIC_IDENTITY")
    );
}

#[test]
fn selected_row_subtrees_ignore_reordering_but_operation_sequences_keep_order() {
    let mut a = fixture();
    a.get_mut(CONFIG).unwrap()["semantic_fields"] = json!([{
        "path":"doc.txt","pointers":["/rows","/operations"],"on":"add-modify-delete"}]);
    a.insert("doc.txt".into(), json!({"rows":[{"path":"one","mode":1},{"path":"two","mode":2}],"operations":["build","test"]}));
    let mut b = a.clone();
    b.get_mut("doc.txt").unwrap()["rows"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(
        report(&a, &b, &["src.bin", "doc.txt"]).unwrap()["mixed"],
        false
    );
    b.get_mut("doc.txt").unwrap()["operations"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let r = report(&a, &b, &["src.bin", "doc.txt"]).unwrap();
    assert_eq!(r["mixed"], true);
    assert_eq!(r["changes"][0]["semantic"][0]["pattern"], "/operations");
}

#[test]
fn missing_named_documents_and_incomplete_costs_are_errors_without_fallback() {
    let a = fixture();
    let old = load(&a);
    let (impact, _) = chrono_judge_filemap::produce(&old, &old, CONFIG, &[delta(PROJECTS)]);
    let mut costs = chrono_judge_cost::produce(&old, &old, &impact).unwrap();
    let mut missing = a.clone();
    missing.remove(PROJECTS);
    assert!(
        produce(&old, &old, &impact.delta, &a, &missing, &costs)
            .unwrap_err()
            .contains("E_SEMANTIC_INPUT")
    );
    costs["declared_after"]
        .as_object_mut()
        .unwrap()
        .remove(&format!("file:{PROJECTS}"));
    assert!(
        produce(&old, &old, &impact.delta, &a, &a, &costs)
            .unwrap_err()
            .contains("E_COST_INPUT")
    );
}
