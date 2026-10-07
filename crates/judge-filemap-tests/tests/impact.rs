mod support;
use chrono_harness::{facts, wire::Finding};
use chrono_judge_filemap::{
    Impact,
    graph::{self, Edge, EdgeKind, Origin, Seed},
    produce,
};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};
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
            let s = i.seed_causes.iter().find(|s| &s.id == id).unwrap();
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
            i.seed_causes
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
    for s in &i.seed_causes {
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
    let structure = i.structure.as_ref().unwrap();
    let cyclic: Vec<_> = structure
        .union
        .components
        .iter()
        .filter(|component| component.cyclic)
        .collect();
    assert_eq!(cyclic.len(), 1);
    assert_eq!(
        cyclic[0].nodes,
        ["project:a", "project:b", "project:p", "project:z"]
    );
    assert_eq!(
        cyclic[0].witness.first().unwrap().from,
        cyclic[0].witness.last().unwrap().to
    );
    assert_eq!(cyclic[0].witness.len(), 3);
    assert_eq!(structure.union.max_depth, 2);
    assert!(structure.affected_components.contains(&cyclic[0].id));
}

#[test]
fn structure_analysis_exposes_cross_owner_edges_and_execution_claim_conflicts() {
    let mut a = values();
    script_pair(&mut a, "st");
    a.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .extend([
            edge("project:p", "test-execution", "test:t"),
            edge("script:s", "test-execution", "test:st"),
        ]);
    a.get_mut(FM).unwrap()["schema_version"] = json!(2);
    a.get_mut(FM).unwrap()["execution_plans"] = json!({
        "test:t":{"operations":["execute.t"],"timeout_seconds":30,"output_limit_bytes":4096},
        "test:st":{"operations":["script.st"],"timeout_seconds":30,"output_limit_bytes":4096}
    });
    a.get_mut(FM).unwrap()["execution_scheduling"] = json!({
        "max_running":2,
        "resources":["shared"],
        "claims":{
            "execute.t":{"resources":["shared"],"outputs":[]},
            "script.st":{"resources":["shared"],"outputs":[]}
        }
    });
    let (impact, findings) = run(&a, &a, &[delta("src.bin")]);
    assert!(findings.is_empty(), "{findings:?}");
    assert!(
        impact
            .structure
            .as_ref()
            .unwrap()
            .candidate
            .cross_owner_edges
            .iter()
            .any(|edge| edge.edge.from == "file:src.bin" && edge.edge.to == "project:p")
    );
    assert_eq!(
        impact
            .structure
            .as_ref()
            .unwrap()
            .execution
            .candidate
            .resource_conflicts,
        vec![chrono_judge_filemap::graph::ExecutionConflict {
            left: "execute.t".into(),
            right: "script.st".into(),
            resources: vec!["shared".into()],
            outputs: vec![],
        }]
    );
    let mut historical = serde_json::to_value(&impact).unwrap();
    historical.as_object_mut().unwrap().remove("structure");
    let decoded: Impact = serde_json::from_value(historical).unwrap();
    assert!(decoded.structure.is_none());
    let mut cyclic = a.clone();
    cyclic.get_mut(FM).unwrap()["execution_plans"]["test:t"]["operations"] =
        json!(["execute.t", "script.st"]);
    cyclic.get_mut(FM).unwrap()["execution_plans"]["test:st"]["operations"] =
        json!(["script.st", "execute.t"]);
    let (cyclic_impact, cyclic_findings) = run(&cyclic, &cyclic, &[delta("src.bin")]);
    assert!(cyclic_findings.is_empty(), "{cyclic_findings:?}");
    assert_eq!(
        cyclic_impact
            .structure
            .as_ref()
            .unwrap()
            .execution
            .candidate
            .cycles,
        vec![vec!["execute.t", "script.st", "execute.t"]]
    );
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
        assert!(i.seed_causes.iter().any(|s| s.node == "input:data"));
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
            .seed_causes
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
fn self_diagnostic_rejects_a_mutated_structure_report() {
    let values = values();
    let registrations = load(&values);
    let explicit = chrono_judge_filemap::edges(&registrations);
    let union = graph::union(&explicit, &explicit);
    let nodes = registrations.node_data();
    let node_ids: BTreeSet<_> = nodes.keys().cloned().collect();
    let closure = graph::closure(
        &union,
        &[Seed {
            id: "mutation".into(),
            node: "file:src.bin".into(),
            reference: "src.bin".into(),
            reason: "test".into(),
        }],
    );
    let owners = BTreeMap::new();
    let input = graph::AnalysisInput {
        base: &explicit,
        candidate: &explicit,
        union: &union,
        base_nodes: &node_ids,
        candidate_nodes: &node_ids,
        union_nodes: &node_ids,
        owners: &owners,
        base_filemap: registrations.filemap(),
        candidate_filemap: registrations.filemap(),
        reached: &closure.reached,
    };
    let mut mutated = graph::analyze(input);
    mutated.union.max_depth += 1;
    let issues = graph::validate_structure(input, &closure, &mutated);
    assert!(
        issues
            .iter()
            .any(|issue| issue.message.contains("maximum depth")),
        "mutating a structural verdict must be observable: {issues:?}"
    );
}

#[test]
fn self_diagnostic_rejects_mutated_depth_owner_execution_and_resource_evidence() {
    let mut values = values();
    values.get_mut(FM).unwrap()["schema_version"] = json!(2);
    values.get_mut(FM).unwrap()["execution_plans"] = json!({
        "test:t":{"operations":["execute.p","execute.t"],"timeout_seconds":30,"output_limit_bytes":4096}
    });
    values.get_mut(FM).unwrap()["execution_scheduling"] = json!({
        "max_running":2,
        "resources":["shared"],
        "claims":{
            "execute.p":{"resources":["shared"],"outputs":[]},
            "execute.t":{"resources":["shared"],"outputs":[]}
        }
    });
    let registrations = load(&values);
    let explicit = chrono_judge_filemap::edges(&registrations);
    let union = graph::union(&explicit, &explicit);
    let nodes = registrations.node_data();
    let node_ids: BTreeSet<_> = nodes.keys().cloned().collect();
    let closure = graph::closure(
        &union,
        &[Seed {
            id: "mutation".into(),
            node: "file:src.bin".into(),
            reference: "src.bin".into(),
            reason: "test".into(),
        }],
    );
    let owners = BTreeMap::from([
        ("file:src.bin".into(), BTreeSet::from(["host".into()])),
        ("project:p".into(), BTreeSet::from(["p".into()])),
        ("test:t".into(), BTreeSet::from(["t".into()])),
    ]);
    let input = graph::AnalysisInput {
        base: &explicit,
        candidate: &explicit,
        union: &union,
        base_nodes: &node_ids,
        candidate_nodes: &node_ids,
        union_nodes: &node_ids,
        owners: &owners,
        base_filemap: registrations.filemap(),
        candidate_filemap: registrations.filemap(),
        reached: &closure.reached,
    };
    let mut mutated = graph::analyze(input);
    mutated
        .candidate
        .component_depth
        .entry("scc:file:doc.txt".into())
        .and_modify(|depth| *depth += 1);
    mutated.candidate.cross_owner_edges.clear();
    mutated.execution.candidate.operations.pop();
    mutated.execution.candidate.resource_conflicts.clear();
    let issues = graph::validate_structure(input, &closure, &mutated);
    let messages: Vec<_> = issues.iter().map(|issue| issue.message.as_str()).collect();
    assert!(
        messages
            .iter()
            .any(|message| message.contains("component depths disagree")),
        "depth mutation was not observable: {messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("cross-owner edge diagnostics")),
        "owner mutation was not observable: {messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("execution operations disagree")),
        "operation mutation was not observable: {messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("resource diagnostics disagree")),
        "resource mutation was not observable: {messages:?}"
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
fn shared_inventory_retains_definitions_and_only_resolves_unique_aliases() {
    let mut v = values();
    script_pair(&mut v, "t");
    let r = load(&v);
    let nodes = r.node_data();
    assert!(nodes["test:t"].unique().is_none());
    assert_eq!(
        nodes["test:t"]
            .definitions
            .iter()
            .map(|d| (
                d.identity.as_str(),
                d.value["actions"]["execute"]["operation"].as_str().unwrap()
            ))
            .collect::<Vec<_>>(),
        [("project:t", "execute.t"), ("script:t", "script.st")]
    );
    assert_eq!(nodes["project:t"].unique().unwrap().identity, "project:t");
    assert_eq!(nodes["script:t"].unique().unwrap().identity, "script:t");
    script_pair(&mut v, "st2");
    let r = load(&v);
    let nodes = r.node_data();
    assert_eq!(
        nodes["test:t"].unique().unwrap().value["actions"]["execute"]["operation"],
        "execute.t"
    );
    assert_eq!(
        nodes["test:st2"].unique().unwrap().value["actions"]["execute"]["operation"],
        "script.st"
    );
}

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
        i.seed_causes
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
        i.seed_causes
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

#[test]
fn execution_plan_order_is_semantic_and_selects_only_explicit_edges() {
    let mut a = values();
    a.get_mut(FM).unwrap()["schema_version"] = json!(2);
    a.get_mut(FM).unwrap()["execution_plans"] = json!({"test:t": {
        "operations":["prepare.p", "execute.t"], "timeout_seconds":30,"output_limit_bytes":4096
    }});
    a.get_mut(PROJECTS).unwrap()["projects"][0]["actions"]["build"] =
        json!({"operation":"prepare.p","tool":"sh","argv":["-c","exit 0"]});
    let mut b = a.clone();
    b.get_mut(FM).unwrap()["execution_plans"]["test:t"]["operations"] =
        json!(["execute.t", "prepare.p"]);
    let (i, f) = run(&a, &b, &[]);
    assert!(f.is_empty(), "{f:?}");
    assert_eq!(tests(&i), vec!["test:t"]);
    assert!(
        i.changes
            .iter()
            .any(|c| c.record.ends_with("/execution_plans/test:t")
                && c.fields.contains(&"/operations".into()))
    );
    for v in [&mut a, &mut b] {
        v.get_mut(FM).unwrap()["project_edges"] = json!([]);
    }
    assert!(tests(&run(&a, &b, &[]).0).is_empty());
}

#[test]
fn full_assignment_changes_select_explicit_old_and_new_complete_plans() {
    let mut a = values();
    a.get_mut(CONFIG).unwrap()["schema_version"] = json!(3);
    a.get_mut(CONFIG).unwrap()["facts_git"] = json!({"tool":"sh","input":"data"});
    a.get_mut(CONFIG).unwrap()["environment"]["inputs"] =
        json!([{"id":"data","location":"data","presence":"present","sha256":"a".repeat(64)}]);
    a.get_mut(CONFIG).unwrap()["input_closure"]["bindings"] = json!([]);
    a.get_mut(CONFIG).unwrap()["execution_units"] = json!({"units":{"one":{"tests":["test:t"],"report_path":".chrono-harness/state/one.json"},"two":{"tests":["test:t2"],"report_path":".chrono-harness/state/two.json"}},"shared_operations":{},"collection_limits":{"manifest_bytes":1024,"report_bytes":67108864},"report_path":".chrono-harness/state/collected.json"});
    a.get_mut(PROJECTS).unwrap()["projects"][0]["actions"]["build"] =
        json!({"operation":"prepare.p","tool":"sh","argv":["-c","exit 0"]});
    let mut p2 = project("p2", "production", "t2");
    p2["actions"]["build"] = json!({"operation":"prepare.p2","tool":"sh","argv":["-c","exit 0"]});
    a.get_mut(PROJECTS).unwrap()["projects"]
        .as_array_mut()
        .unwrap()
        .extend([p2, project("t2", "test", "p2")]);
    a.get_mut(PROJECTS).unwrap()["owners"]
        .as_array_mut()
        .unwrap()
        .extend([json!("p2"), json!("t2")]);
    a.get_mut(FM).unwrap()["schema_version"] = json!(2);
    a.get_mut(FM).unwrap()["execution_plans"] = json!({"test:t":{"operations":["prepare.p","execute.t"],"timeout_seconds":30,"output_limit_bytes":4096},"test:t2":{"operations":["prepare.p2","execute.t2"],"timeout_seconds":30,"output_limit_bytes":4096}});
    a.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .push(edge("project:p2", "test-execution", "test:t2"));
    a.get_mut(FM).unwrap()["test_costs"]
        .as_array_mut()
        .unwrap()
        .push(json!({"test":"t2","cost":"unknown"}));
    let mut b = a.clone();
    b.get_mut(CONFIG).unwrap()["execution_units"]["units"]["one"]["tests"] = json!(["test:t2"]);
    b.get_mut(CONFIG).unwrap()["execution_units"]["units"]["two"]["tests"] = json!(["test:t"]);
    let (impact, findings) = run(&a, &b, &[]);
    assert!(findings.is_empty(), "{findings:?}");
    assert_eq!(tests(&impact), ["test:t", "test:t2"]);
}

#[test]
fn declared_judge_inputs_remain_valid_when_migration_selects_them() {
    let mut a = values();
    a.get_mut(CONFIG).unwrap()["environment"]["inputs"] =
        json!([{"id":"data","location":"data","sha256":"a".repeat(64)}]);
    a.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .extend([
            edge("tool:sh", "runtime-input", "judge:registration"),
            edge("environment:PATH", "runtime-input", "judge:registration"),
            edge("input:data", "judge-trigger", "judge:registration"),
        ]);
    for mode in 0..3 {
        let mut b = a.clone();
        match mode {
            0 => b.get_mut(CONFIG).unwrap()["tools"][0]["expected_version"] = json!("new"),
            1 => b.get_mut(CONFIG).unwrap()["environment"]["values"]["PATH"] = json!("different"),
            _ => {
                b.get_mut(CONFIG).unwrap()["environment"]["inputs"][0]["sha256"] =
                    json!("b".repeat(64))
            }
        }
        let (impact, findings) = run(&a, &b, &[]);
        assert!(findings.is_empty(), "{findings:?}");
        assert!(impact.judges.iter().any(|j| j.node == "judge:registration"));
        assert!(
            tests(&impact).is_empty(),
            "judge inputs must not infer business test edges"
        );
    }
}

#[test]
fn tool_environment_and_retained_input_records_use_only_explicit_consumers() {
    let mut a = values();
    a.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .extend([
            edge("tool:sh", "runtime-input", "project:p"),
            edge("environment:PATH", "runtime-input", "project:p"),
            edge("input:data", "runtime-input", "project:p"),
        ]);
    a.get_mut(CONFIG).unwrap()["environment"]["inputs"] =
        json!([{"id":"data","location":"data","sha256":"a".repeat(64)}]);
    for mode in 0..3 {
        let mut b = a.clone();
        match mode {
            0 => b.get_mut(CONFIG).unwrap()["tools"][0]["expected_version"] = json!("new"),
            1 => b.get_mut(CONFIG).unwrap()["environment"]["values"]["PATH"] = json!("different"),
            _ => {
                b.get_mut(CONFIG).unwrap()["environment"]["inputs"][0]["sha256"] =
                    json!("b".repeat(64))
            }
        }
        let (i, f) = run(&a, &b, &[]);
        assert!(f.is_empty(), "{f:?}");
        assert_eq!(tests(&i), vec!["test:t"]);
        let mut disconnected = a.clone();
        disconnected.get_mut(FM).unwrap()["project_edges"] = json!([]);
        b.get_mut(FM).unwrap()["project_edges"] = json!([]);
        assert!(tests(&run(&disconnected, &b, &[]).0).is_empty());
    }
}

#[test]
fn explicit_historical_ambiguity_mapping_preserves_definitions_but_candidate_collision_fails() {
    let mut a = values();
    script_pair(&mut a, "t");
    let mut b = a.clone();
    b.get_mut(PROJECTS).unwrap()["scripts"] = json!([]);
    b.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .retain(|e| e["from"] != "script:s");
    b.get_mut(WORKFLOW).unwrap()["schema_version"] = json!(2);
    b.get_mut(WORKFLOW).unwrap()["historical_profiles"] = json!([{"id":"chrono-ci-check/v1","filemap_version":1,"profile_path":"profile.json","script":"decoder","test":"decoder-tests","mappings":[],"legacy_records":[],"ambiguities":[{"node":"test:t","definitions":["project:t","script:t"],"replacement":"test:t"}]}]);
    let (i, f) = run(&a, &b, &[delta("src.bin")]);
    assert!(f.iter().all(|f| f.code != "E_NODE_AMBIGUOUS"), "{f:?}");
    assert_eq!(i.nodes["test:t"].base_definitions.len(), 2);
    assert_eq!(tests(&i), vec!["test:t"]);
    let mut missing = b.clone();
    missing.get_mut(WORKFLOW).unwrap()["historical_profiles"][0]["ambiguities"] = json!([]);
    assert!(
        run(&a, &missing, &[delta("src.bin")])
            .1
            .iter()
            .any(|f| f.code == "E_NODE_AMBIGUOUS")
    );
    let mut collision = a.clone();
    collision.insert(WORKFLOW.into(), b[WORKFLOW].clone());
    assert!(
        run(&a, &collision, &[delta("src.bin")])
            .1
            .iter()
            .any(|f| f.code == "E_NODE_AMBIGUOUS")
    );
}

#[test]
fn retained_effective_environment_api_preserves_old_edges_and_rejects_unknown_facts() {
    let mut a = values();
    a.get_mut(CONFIG).unwrap()["environment"]["inherit"] = json!(["MODE"]);
    a.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .push(edge("environment:MODE", "runtime-input", "project:p"));
    let mut b = a.clone();
    b.get_mut(CONFIG).unwrap()["environment"]["inherit"] = json!([]);
    b.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .retain(|e| e["from"] != "environment:MODE");
    let endpoints = json!({"base":{"inherited":{"MODE":"old"},"environment":{"MODE":"old"}},"candidate":{"inherited":{},"environment":{}}});
    let mut evidence = json!({"schema":"chrono-effective-inputs/v1","identity":chrono_harness::wire::digest(&endpoints).unwrap(),"endpoints":endpoints});
    let (impact, findings) =
        chrono_judge_filemap::produce_with_inputs(&load(&a), &load(&b), CONFIG, &[], &evidence)
            .unwrap();
    assert!(findings.is_empty());
    assert_eq!(impact.tests, vec!["test:t"]);
    assert!(impact.seeds.contains(&"environment:MODE".into()));
    evidence["endpoints"]["candidate"]["environment"]["UNKNOWN"] = json!("unrepresented");
    evidence["identity"] = json!(chrono_harness::wire::digest(&evidence["endpoints"]).unwrap());
    assert!(
        chrono_judge_filemap::produce_with_inputs(&load(&a), &load(&b), CONFIG, &[], &evidence)
            .is_err()
    );
}

#[test]
fn artifact_owner_changes_and_removal_preserve_explicit_endpoint_consumers() {
    let mut a = values();
    script_pair(&mut a, "st");
    a.get_mut(CONFIG).unwrap()["artifacts"]
        .as_array_mut()
        .unwrap()
        .push(json!({"path":"declared outputs/","owner":"p","kind":"output","tracked":false}));
    let mut b = a.clone();
    b.get_mut(CONFIG).unwrap()["artifacts"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap()["owner"] = json!("s");
    let (impact, findings) = run(&a, &b, &[delta(CONFIG)]);
    assert!(findings.is_empty());
    assert_eq!(tests(&impact), ["test:st", "test:t"]);
    assert!(impact.seeds.contains(&"project:p".into()));
    assert!(impact.seeds.contains(&"script:s".into()));
    let mut removed = a.clone();
    removed.get_mut(CONFIG).unwrap()["artifacts"]
        .as_array_mut()
        .unwrap()
        .pop();
    let (impact, findings) = run(&a, &removed, &[delta(CONFIG)]);
    assert!(findings.is_empty());
    assert_eq!(tests(&impact), ["test:t"]);
    let (impact, findings) = run(&a, &a, &[delta("doc.txt")]);
    assert!(findings.is_empty());
    assert!(tests(&impact).is_empty());
}

#[test]
fn group_registration_delta_and_endpoints_preserve_every_opaque_test_identity() {
    let old = values();
    let mut new = old.clone();
    new.get_mut(PROJECTS).unwrap()["projects"][1]["actions"]["second"] =
        json!({"operation":"execute.second","tool":"sh","argv":["-c","exit 0"]});
    new.get_mut(PROJECTS).unwrap()["projects"][1]["test_groups"] =
        json!({"t":"execute","opaque-group":"second"});
    new.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .push(edge("project:p", "test-execution", "test:opaque-group"));
    new.get_mut(FM).unwrap()["test_costs"]
        .as_array_mut()
        .unwrap()
        .push(json!({"test":"opaque-group","cost":"unknown"}));
    for test in ["test:t", "test:opaque-group"] {
        new.get_mut(FM).unwrap()["project_edges"]
            .as_array_mut()
            .unwrap()
            .push(edge("project:t", "test-execution", test));
    }
    for paths in [vec![], vec![delta("src.bin")]] {
        let (impact, findings) = run(&old, &new, &paths);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(tests(&impact), ["test:opaque-group", "test:t"]);
        assert!(impact.retired_tests.is_empty());
        assert_eq!(
            impact.nodes["test:opaque-group"].candidate_definitions[0].identity,
            "project:t"
        );
    }
    let mut rebound = new.clone();
    rebound.get_mut(PROJECTS).unwrap()["projects"][1]["test_groups"] =
        json!({"t":"second","opaque-group":"execute"});
    let (impact, findings) = run(&new, &rebound, &[]);
    assert!(findings.is_empty(), "{findings:?}");
    assert_eq!(tests(&impact), ["test:opaque-group", "test:t"]);
    let (impact, findings) = run(&new, &old, &[]);
    assert!(
        findings.iter().all(|f| f.code != "E_REFERENCE"),
        "{findings:?}"
    );
    assert_eq!(
        impact
            .retired_tests
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["test:opaque-group"]
    );
}

#[test]
fn scheduling_cap_claim_changes_and_removal_keep_both_endpoint_targets() {
    let mut a = values();
    script_pair(&mut a, "st");
    a.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .extend([
            edge("project:t", "test-execution", "test:t"),
            edge("script:st", "test-execution", "test:st"),
        ]);
    a.get_mut(FM).unwrap()["schema_version"] = json!(2);
    a.get_mut(FM).unwrap()["execution_plans"] = json!({"test:t":{"operations":["execute.t"],"timeout_seconds":30,"output_limit_bytes":4096},"test:st":{"operations":["script.st"],"timeout_seconds":30,"output_limit_bytes":4096}});
    a.get_mut(FM).unwrap()["execution_scheduling"] = json!({"max_running":2,"resources":["one","two"],"claims":{"execute.t":{"resources":["one"],"outputs":[]},"script.st":{"resources":[],"outputs":[]}}});
    for mode in 0..4 {
        let mut b = a.clone();
        match mode {
            0 => b.get_mut(FM).unwrap()["execution_scheduling"]["max_running"] = json!(1),
            1 => {
                b.get_mut(FM).unwrap()["execution_scheduling"]["claims"]["execute.t"]["resources"] =
                    json!(["two"])
            }
            2 => {
                b.get_mut(FM)
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .remove("execution_scheduling");
            }
            _ => {
                b.get_mut(FM).unwrap()["execution_scheduling"]["claims"]["execute.t"]["outputs"] =
                    json!([".chrono-harness/state/"])
            }
        }
        for (before, after) in [(&a, &b), (&b, &a)] {
            let (i, f) = run(before, after, &[]);
            assert!(f.is_empty(), "{f:?}");
            assert_eq!(
                tests(&i),
                if mode == 0 || mode == 2 {
                    vec!["test:st", "test:t"]
                } else {
                    vec!["test:t"]
                }
            );
        }
    }
    assert!(tests(&run(&a, &a, &[]).0).is_empty());
}

#[test]
fn scheduling_priority_changes_reorder_and_removal_reach_both_endpoint_consumers() {
    let mut a = values();
    script_pair(&mut a, "st");
    a.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .extend([
            edge("project:t", "test-execution", "test:t"),
            edge("script:st", "test-execution", "test:st"),
        ]);
    a.get_mut(FM).unwrap()["schema_version"] = json!(2);
    a.get_mut(FM).unwrap()["execution_plans"] = json!({"test:t":{"operations":["execute.t"],"timeout_seconds":30,"output_limit_bytes":4096},"test:st":{"operations":["script.st"],"timeout_seconds":30,"output_limit_bytes":4096}});
    a.get_mut(FM).unwrap()["execution_scheduling"] = json!({"max_running":2,"priority":["script.st","execute.t"],"resources":[],"claims":{"execute.t":{"resources":[],"outputs":[]},"script.st":{"resources":[],"outputs":[]}}});
    for priority in [
        Some(json!(["execute.t", "script.st"])),
        Some(json!(["execute.t"])),
        Some(json!([])),
        None,
    ] {
        let mut b = a.clone();
        if let Some(priority) = priority {
            b.get_mut(FM).unwrap()["execution_scheduling"]["priority"] = priority;
        } else {
            b.get_mut(FM).unwrap()["execution_scheduling"]
                .as_object_mut()
                .unwrap()
                .remove("priority");
        }
        for (before, after) in [(&a, &b), (&b, &a)] {
            let (impact, findings) = run(before, after, &[]);
            assert!(findings.is_empty(), "{findings:?}");
            assert_eq!(tests(&impact), ["test:st", "test:t"]);
        }
    }
    assert!(tests(&run(&a, &a, &[]).0).is_empty());
    let mut empty = a.clone();
    empty.get_mut(FM).unwrap()["execution_scheduling"]["priority"] = json!([]);
    let mut absent = empty.clone();
    absent.get_mut(FM).unwrap()["execution_scheduling"]
        .as_object_mut()
        .unwrap()
        .remove("priority");
    assert!(tests(&run(&empty, &absent, &[]).0).is_empty());
    assert!(tests(&run(&absent, &empty, &[]).0).is_empty());
}

#[test]
fn scheduling_resource_reassignment_and_artifact_changes_reach_declared_consumers() {
    let mut a = values();
    script_pair(&mut a, "st");
    a.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .extend([
            edge("project:t", "test-execution", "test:t"),
            edge("script:st", "test-execution", "test:st"),
        ]);
    a.get_mut(FM).unwrap()["schema_version"] = json!(2);
    a.get_mut(FM).unwrap()["execution_plans"] = json!({"test:t":{"operations":["execute.t"],"timeout_seconds":30,"output_limit_bytes":4096},"test:st":{"operations":["script.st"],"timeout_seconds":30,"output_limit_bytes":4096}});
    a.get_mut(CONFIG).unwrap()["artifacts"]
        .as_array_mut()
        .unwrap()
        .push(json!({"path":"out/","owner":"p","kind":"output","tracked":false}));
    a.get_mut(FM).unwrap()["execution_scheduling"] = json!({"max_running":2,"resources":["one","two"],"claims":{"execute.t":{"resources":["one"],"outputs":["out/"]},"script.st":{"resources":["two"],"outputs":["out/"]}}});
    for mode in 0..2 {
        let mut b = a.clone();
        if mode == 0 {
            b.get_mut(FM).unwrap()["execution_scheduling"]["claims"]["execute.t"]["resources"] =
                json!(["two"]);
        } else {
            b.get_mut(CONFIG).unwrap()["artifacts"]
                .as_array_mut()
                .unwrap()
                .last_mut()
                .unwrap()["owner"] = json!("s");
        }
        for (before, after) in [(&a, &b), (&b, &a)] {
            let (i, f) = run(before, after, &[]);
            assert!(f.is_empty(), "{f:?}");
            assert_eq!(tests(&i), ["test:st", "test:t"]);
        }
    }
}
