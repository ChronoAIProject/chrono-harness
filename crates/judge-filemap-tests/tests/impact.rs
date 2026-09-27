mod support;
use chrono_harness::{facts, wire::Finding};
use chrono_judge_filemap::{
    Impact,
    graph::{self, Edge, EdgeKind, Origin, Seed},
    produce,
};
use serde_json::json;
use std::{collections::BTreeSet, fs};
use support::*;
fn run(a: &Values, b: &Values, d: &[chrono_harness::wire::Delta]) -> (Impact, Vec<Finding>) {
    produce(&load(a), &load(b), CONFIG, d)
}
fn tests(i: &Impact) -> Vec<&str> {
    i.required_tests.iter().map(|t| t.node.as_str()).collect()
}
fn oracle_edges(rows: &[(&str, EdgeKind, &str)]) -> BTreeSet<Edge> {
    rows.iter()
        .map(|(a, k, b)| Edge {
            from: (*a).into(),
            kind: *k,
            to: (*b).into(),
        })
        .collect()
}
// Witness oracle takes independently specified endpoint edges; it never parses producer strings.
fn witnesses(i: &Impact, allowed: &BTreeSet<Edge>) {
    for (node, seeds) in &i.closure.reached {
        for id in seeds {
            let s = i.seeds.iter().find(|s| &s.id == id).unwrap();
            let mut at = s.node.clone();
            for e in i.closure.witness(id, node).unwrap() {
                assert!(allowed.contains(&e), "fabricated edge {e:?}");
                assert_eq!(at, e.from);
                at = e.to;
            }
            assert_eq!(&at, node);
        }
    }
    for t in &i.closure.traversed {
        assert!(allowed.contains(&t.edge));
    }
}
#[test]
fn typed_content_mode_binary_add_modify_delete_and_rename() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git(root, &["init", "-q"]);
    fs::write(root.join("src.bin"), [0, 255, 1]).unwrap();
    let b = commit(root);
    let a = values();
    for (bytes, mode) in [(vec![0, 255, 2], 0o644), (vec![0, 255, 1], 0o755)] {
        fs::write(root.join("src.bin"), bytes).unwrap();
        fs::set_permissions(root.join("src.bin"), fs::Permissions::from_mode(mode)).unwrap();
        let c = commit(root);
        let d = git_delta(root, &b, &c);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].kind, "M");
        let (i, f) = run(&a, &a, &d);
        assert!(f.is_empty());
        assert_eq!(tests(&i), ["test:t"]);
    }
    fs::rename(root.join("src.bin"), root.join("new.bin")).unwrap();
    let c = commit(root);
    let d = git_delta(root, &b, &c);
    assert_eq!(
        d.iter().map(|d| (&*d.path, &*d.kind)).collect::<Vec<_>>(),
        [("new.bin", "A"), ("src.bin", "D")]
    );
    assert_eq!(d[0].new_blob, d[1].old_blob);
    let mut n = a.clone();
    n.get_mut(FM).unwrap()["files"][0]["path"] = "new.bin".into();
    let (i, f) = run(&a, &n, &d);
    assert!(f.is_empty());
    assert_eq!(tests(&i), ["test:t"]);
    assert!(i.nodes["file:src.bin"].candidate.is_none());
}
#[test]
fn nonancestor_and_multicommit_facts_use_complete_endpoints() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git(root, &["init", "-q"]);
    fs::write(root.join("src.bin"), "one").unwrap();
    let first = commit(root);
    fs::write(root.join("doc.txt"), "docs").unwrap();
    let left = commit(root);
    git(root, &["checkout", "--detach", &first]);
    fs::write(root.join("src.bin"), "two").unwrap();
    commit(root);
    fs::write(root.join("doc.txt"), "other docs").unwrap();
    let right = commit(root);
    for base in [&first, &left] {
        let d = git_delta(root, base, &right);
        assert_eq!(d.len(), 2);
        let (i, _) = run(&values(), &values(), &d);
        assert_eq!(tests(&i), ["test:t"]);
    }
    assert!(facts::tree(root, &"a".repeat(40)).is_err());
}
#[test]
fn no_delta_and_disconnected_docs_only_select_every_delta_judges() {
    let a = values();
    for d in [vec![], vec![delta("doc.txt")]] {
        let (i, f) = run(&a, &a, &d);
        assert!(f.is_empty());
        assert!(tests(&i).is_empty());
        assert_eq!(i.judges.len(), 2);
        assert!(
            i.judges
                .iter()
                .all(|j| j.every_delta && j.triggers.is_empty())
        );
    }
}
#[test]
fn base_only_test_edge_preserves_removed_requirement_owner_and_cost() {
    let a = values();
    let mut b = a.clone();
    b.get_mut(FM).unwrap()["project_edges"] = json!([]);
    b.get_mut(FM).unwrap()["files"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    b.get_mut(FM).unwrap()["test_costs"] = json!([]);
    b.get_mut(FM).unwrap()["cost_models"] = json!({});
    b.get_mut(PROJECTS).unwrap()["projects"] = json!([]);
    b.get_mut(PROJECTS).unwrap()["owners"] = json!([]);
    let (i, f) = run(&a, &b, &[delta(FM)]);
    assert!(f.is_empty());
    assert_eq!(
        tests(&i),
        ["test:t"],
        "removed test edge must still require old test"
    );
    assert!(!i.required_tests[0].candidate_present);
    assert_eq!(
        i.required_tests[0].base_cost.as_ref().unwrap()["value"]["cpu_ms"],
        json!(null)
    );
    assert!(i.required_tests[0].candidate_cost.is_none());
    assert_eq!(
        i.nodes["file:src.bin"].base.as_ref().unwrap()["owner"],
        "host"
    );
    assert!(i.records["/records/projects/owners/host"].base.is_some());
    assert!(i.edges.iter().all(|e| e.origin == Origin::Base));
    witnesses(
        &i,
        &oracle_edges(&[
            ("file:src.bin", EdgeKind::Compile, "project:p"),
            ("project:p", EdgeKind::TestExecution, "test:t"),
        ]),
    );
}
#[test]
fn file_record_field_and_edge_add_delete_retarget_changes_seed_both_ends() {
    let a = values();
    let mut b = a.clone();
    b.get_mut(FM).unwrap()["files"][0]["owner"] = "p".into();
    b.get_mut(FM).unwrap()["project_edges"] =
        json!([edge("project:t", "test-execution", "test:t")]);
    let (i, f) = run(&a, &b, &[delta(FM)]);
    assert!(f.is_empty());
    assert_eq!(tests(&i), ["test:t"]);
    assert!(i.changes.iter().any(
        |c| c.record == "/records/filemap/files/src.bin" && c.fields.contains(&"/owner".into())
    ));
    for node in ["project:p", "project:t", "test:t"] {
        assert!(
            i.seeds
                .iter()
                .any(|s| s.node == node && s.reason == "edge-change")
        );
    }
    assert_eq!(
        i.edges.iter().filter(|e| e.origin == Origin::Base).count(),
        1
    );
    assert_eq!(
        i.edges
            .iter()
            .filter(|e| e.origin == Origin::Candidate)
            .count(),
        1
    );
    assert_eq!(
        i.edges.iter().filter(|e| e.origin == Origin::Both).count(),
        1
    );
    for s in &i.seeds {
        if s.reason != "delta-path" {
            assert!(i.records.contains_key(&s.reference));
        }
    }
}
#[test]
fn cycles_diamonds_and_multiple_causes_retain_real_edges_and_all_seeds() {
    let mut a = values();
    for id in ["a", "b", "z"] {
        a.get_mut(PROJECTS).unwrap()["projects"]
            .as_array_mut()
            .unwrap()
            .push(project(id, "production", "t"));
    }
    let rows = [
        ("file:src.bin", EdgeKind::Compile, "project:p"),
        ("file:doc.txt", EdgeKind::RuntimeInput, "project:a"),
        ("project:p", EdgeKind::Compile, "project:a"),
        ("project:p", EdgeKind::Compile, "project:b"),
        ("project:a", EdgeKind::Compile, "project:z"),
        ("project:b", EdgeKind::Compile, "project:z"),
        ("project:z", EdgeKind::Compile, "project:p"),
        ("project:z", EdgeKind::TestExecution, "test:t"),
    ];
    a.get_mut(FM).unwrap()["files"][1]["edges"] =
        json!([{"kind":"runtime-input","to":"project:a"}]);
    a.get_mut(FM).unwrap()["project_edges"] = serde_json::to_value(
        oracle_edges(&rows)
            .into_iter()
            .filter(|e| !e.from.starts_with("file:"))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let (i, f) = run(&a, &a, &[delta("src.bin"), delta("doc.txt")]);
    assert!(f.is_empty());
    assert_eq!(tests(&i), ["test:t"]);
    assert_eq!(i.closure.reached["test:t"].len(), 2);
    assert_eq!(i.closure.traversed.len(), rows.len());
    witnesses(&i, &oracle_edges(&rows));
}
#[test]
fn input_declaration_boundary_propagates_build_and_runtime_without_claiming_retention() {
    for kind in ["build-input", "runtime-input"] {
        let mut a = values();
        a.get_mut(CONFIG).unwrap()["environment"]["inputs"] =
            json!([{"id":"data","location":"/external/data","sha256":null}]);
        a.get_mut(FM).unwrap()["project_edges"]
            .as_array_mut()
            .unwrap()
            .push(edge("input:data", kind, "project:p"));
        let mut b = a.clone();
        b.get_mut(CONFIG).unwrap()["environment"]["inputs"][0]["sha256"] = "f".repeat(64).into();
        let (i, f) = run(&a, &b, &[delta(CONFIG)]);
        assert!(f.is_empty());
        assert_eq!(tests(&i), ["test:t"]);
        assert!(i.seeds.iter().any(|s| s.node == "input:data"));
        assert!(i.limits.iter().any(|s| s.contains("not retained")));
    }
}
#[test]
fn pairing_and_test_record_seed_without_execution_edge_do_not_select() {
    let mut a = values();
    a.get_mut(FM).unwrap()["project_edges"] = json!([]);
    let mut b = a.clone();
    b.get_mut(FM).unwrap()["test_costs"][0]["cost"] = "changed".into();
    let (i, _) = run(&a, &b, &[delta("src.bin"), delta(FM)]);
    assert!(i.closure.reached.contains_key("project:p"));
    assert!(i.closure.reached.contains_key("test:t"));
    assert!(
        tests(&i).is_empty(),
        "test seed and ownership pairing cannot select execution"
    );
}
#[test]
fn wrong_edge_to_test_cannot_select_and_has_real_cause() {
    let mut a = values();
    a.get_mut(FM).unwrap()["project_edges"] = json!([edge("project:p", "compile", "test:t")]);
    let (i, f) = run(&a, &a, &[delta("src.bin")]);
    assert!(tests(&i).is_empty());
    assert_eq!(f.len(), 2);
    for f in f {
        assert_eq!(f.code, "E_EDGE_TYPE");
        assert_eq!(f.delta_refs, ["src.bin"]);
        assert_eq!(f.causes, ["file:src.bin", "project:p", "test:t"]);
    }
}
#[test]
fn reached_dangling_and_changed_invalid_types_fail_but_disconnected_history_is_context() {
    let mut a = values();
    a.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .push(edge("project:p", "compile", "project:missing"));
    let (_, f) = run(&a, &a, &[delta("src.bin")]);
    assert_eq!(f.len(), 2);
    for f in f {
        assert_eq!(f.delta_refs, ["src.bin"]);
        assert_eq!(f.causes, ["file:src.bin", "project:p", "project:missing"]);
    }
    let (i, f) = run(&a, &a, &[delta("doc.txt")]);
    assert!(f.is_empty());
    assert_eq!(i.historical_context.len(), 2);
    let mut b = a.clone();
    b.get_mut(FM).unwrap()["project_edges"][1] = edge("judge:registration", "compile", "project:p");
    let (i, f) = run(&a, &b, &[delta(FM)]);
    assert!(f.iter().any(|f| f.code == "E_EDGE_TYPE"));
    for f in f {
        let reference = &f.delta_refs[0];
        assert!(i.records.contains_key(reference) || reference == FM);
        let seed = i
            .seeds
            .iter()
            .find(|s| &s.reference == reference && s.node == f.causes[0]);
        assert!(seed.is_some());
    }
}
#[test]
fn judge_triggers_add_explanations_once() {
    let mut a = values();
    a.get_mut(FM).unwrap()["files"][0]["edges"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"judge-trigger","to":"judge:filemap"}));
    let (i, f) = run(&a, &a, &[delta("src.bin")]);
    assert!(f.is_empty());
    let j = i.judges.iter().find(|j| j.node == "judge:filemap").unwrap();
    assert!(j.every_delta);
    assert_eq!(j.triggers.len(), 1);
    assert_eq!(i.judges.len(), 2);
}
#[test]
fn identity_array_order_is_irrelevant_but_argv_order_changes_target() {
    let a = values();
    let mut b = a.clone();
    for (p, k) in [
        (FM, "files"),
        (PROJECTS, "owners"),
        (PROJECTS, "projects"),
        (JUDGES, "judges"),
    ] {
        b.get_mut(p).unwrap()[k].as_array_mut().unwrap().reverse();
    }
    let (i, f) = run(&a, &b, &[delta(FM), delta(PROJECTS), delta(JUDGES)]);
    assert!(f.is_empty());
    assert!(i.changes.is_empty());
    assert!(tests(&i).is_empty());
    b.get_mut(PROJECTS).unwrap()["projects"][1]["actions"]["execute"]["argv"] =
        json!(["exit 0", "-c"]);
    let (i, _) = run(&a, &b, &[delta(PROJECTS)]);
    assert_eq!(tests(&i), ["test:t"]);
    assert!(
        i.changes
            .iter()
            .any(|c| c.fields.contains(&"/actions/execute/argv".into()))
    );
}
#[test]
fn shared_graph_does_not_need_full_registration_and_only_execution_edges_select() {
    let edge = Edge {
        from: "legacy".into(),
        kind: EdgeKind::Compile,
        to: "test:legacy".into(),
    };
    let g = graph::union(&BTreeSet::new(), &BTreeSet::from([edge]));
    let c = graph::closure(
        &g,
        &[Seed {
            id: "s".into(),
            node: "legacy".into(),
            reference: "legacy".into(),
            reason: "adapter".into(),
        }],
    );
    assert!(c.reached.contains_key("test:legacy"));
    assert!(
        c.selected_tests().is_empty(),
        "nonexecution edges cannot select tests"
    );
}
#[test]
fn loader_missing_malformed_inputs_are_errors_not_empty_impact() {
    let mut a = values();
    a.remove(FM);
    assert!(Registrations::load(&a, CONFIG).is_err());
    let mut a = values();
    a.get_mut(FM).unwrap()["files"] = json!({});
    assert!(Registrations::load(&a, CONFIG).is_err());
    let mut a = values();
    a.get_mut(FM).unwrap()["files"][0]["edges"][0]["kind"] = "guessed".into();
    assert!(Registrations::load(&a, CONFIG).is_err());
}
use chrono_judge_registration::Registrations;

#[test]
fn file_origin_project_edge_change_points_to_its_actual_record() {
    let a = values();
    let mut b = a.clone();
    b.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .push(edge("file:doc.txt", "compile", "test:t"));
    let (i, f) = run(&a, &b, &[delta(FM)]);
    let finding = f.iter().find(|f| f.code == "E_EDGE_TYPE").unwrap();
    assert!(finding.delta_refs[0].starts_with("/records/filemap/project_edges/"));
    assert!(i.changes.iter().any(|c| c.record == finding.delta_refs[0]));
}
#[test]
fn file_edge_only_removal_and_addition_preserve_targets() {
    let a = values();
    let mut b = a.clone();
    b.get_mut(FM).unwrap()["files"][0]["edges"] = json!([]);
    let (i, f) = run(&a, &b, &[delta(FM)]);
    assert!(f.is_empty());
    assert_eq!(tests(&i), ["test:t"]);
    assert!(
        i.seeds
            .iter()
            .any(|s| s.node == "project:p" && s.reason == "edge-change")
    );
    let (i, f) = run(&b, &a, &[delta(FM)]);
    assert!(f.is_empty());
    assert_eq!(tests(&i), ["test:t"]);
    assert!(
        i.edges
            .iter()
            .any(|e| e.origin == Origin::Candidate && e.edge.from == "file:src.bin")
    );
}
#[test]
fn identity_edge_and_input_array_reorder_is_no_semantic_change() {
    let mut a = values();
    a.get_mut(CONFIG).unwrap()["environment"]["inputs"] =
        json!([{"id":"a","location":"/a","sha256":null},{"id":"b","location":"/b","sha256":null}]);
    a.get_mut(FM).unwrap()["files"][0]["edges"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"build-input","to":"project:t"}));
    a.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .push(edge("project:t", "test-execution", "test:t"));
    let mut b = a.clone();
    b.get_mut(CONFIG).unwrap()["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
        .reverse();
    b.get_mut(FM).unwrap()["files"][0]["edges"]
        .as_array_mut()
        .unwrap()
        .reverse();
    b.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let (i, f) = run(&a, &b, &[delta(CONFIG), delta(FM)]);
    assert!(f.is_empty());
    assert!(i.changes.is_empty());
    assert!(tests(&i).is_empty());
}

#[test]
fn changed_cost_keeps_distinct_old_new_values_and_references() {
    let mut a = values();
    a.get_mut(FM).unwrap()["cost_models"]["unknown"]["cpu_ms"] = json!(17);
    a.get_mut(FM).unwrap()["cost_models"]["unknown"]["basis"] = json!("old fixture measurement");
    let mut b = a.clone();
    b.get_mut(FM).unwrap()["cost_models"]["new"] = json!({"cpu_ms":null,"wall_ms":null,"peak_rss_bytes":null,"io_bytes":null,"basis":"new estimate unknown"});
    b.get_mut(FM).unwrap()["test_costs"][0]["cost"] = "new".into();
    let (i, f) = run(&a, &b, &[delta("src.bin"), delta(FM)]);
    assert!(f.is_empty());
    assert_eq!(tests(&i), ["test:t"]);
    let t = &i.required_tests[0];
    assert_eq!(t.base_cost.as_ref().unwrap()["reference"], "unknown");
    assert_eq!(t.base_cost.as_ref().unwrap()["value"]["cpu_ms"], 17);
    assert_eq!(t.candidate_cost.as_ref().unwrap()["reference"], "new");
    assert_eq!(
        t.candidate_cost.as_ref().unwrap()["value"]["cpu_ms"],
        json!(null)
    );
}

#[test]
fn removed_test_record_seeds_alias_and_exposes_retained_dangling_edge() {
    let a = values();
    let mut b = a.clone();
    b.get_mut(PROJECTS).unwrap()["projects"]
        .as_array_mut()
        .unwrap()
        .retain(|p| p["id"] != "t");
    let (i, f) = run(&a, &b, &[delta(PROJECTS)]);
    assert!(
        i.seeds
            .iter()
            .any(|s| s.node == "test:t" && s.reference == "/records/projects/projects/t")
    );
    assert!(
        tests(&i).is_empty(),
        "node removal alone does not traverse an incoming execution edge"
    );
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].code, "E_DANGLING_EDGE");
    assert_eq!(f[0].delta_refs, ["/records/projects/projects/t"]);
    assert_eq!(f[0].causes, ["test:t"]);
    let (i, f) = run(&a, &b, &[delta(PROJECTS), delta("src.bin")]);
    assert_eq!(tests(&i), ["test:t"]);
    assert!(!i.required_tests[0].candidate_present);
    assert_eq!(f.len(), 1);
}
