//! Lower explicit workflow test requirements to typed edges before execution.
use crate::{
    graph::{Edge, EdgeKind},
    records::Record,
};
use chrono_harness::wire::Delta;
use chrono_judge_mixed::Documents;
use chrono_judge_registration::Registrations;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub fn select(
    a: &Registrations,
    b: &Registrations,
    delta: &[Delta],
    records: [&BTreeMap<String, Record>; 2],
    documents: &[Documents; 2],
) -> Result<Value, String> {
    let changes = chrono_judge_mixed::classify(a, b, delta, &documents[0], &documents[1])?;
    let changed_paths: BTreeSet<_> = delta.iter().map(|d| d.path.as_str()).collect();
    let mut requirements = vec![];
    let mut triggers = BTreeSet::<String>::new();
    for (endpoint, r) in [("base", a), ("candidate", b)] {
        let path = r.config()["registries"]["workflow"].as_str().unwrap();
        for (i, row) in r.workflow()["stability"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            let record = format!(
                "/records/workflow/stability/{}",
                crate::records::escape(row["id"].as_str().unwrap())
            );
            let changed = records[0].get(&record).map(|r| (&r.path, &r.value))
                != records[1].get(&record).map(|r| (&r.path, &r.value));
            let mut paths: BTreeSet<String> = row["paths"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p.as_str().unwrap())
                .filter(|p| changed_paths.contains(p))
                .map(str::to_string)
                .collect();
            if changed {
                paths.insert(path.into());
            }
            if paths.is_empty() {
                continue;
            }
            triggers.extend(paths.clone());
            requirements.push(requirement(
                endpoint,
                path,
                &format!("/stability/{i}"),
                "stability",
                &paths,
                &row["tests"],
            ));
        }
    }
    if [a, b]
        .iter()
        .any(|r| r.workflow()["semantic_changes_require_integration"] == true)
    {
        triggers.extend(
            changes["rule_paths"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p.as_str().unwrap().to_string()),
        );
    }
    let required = !triggers.is_empty();
    if required {
        for (endpoint, r) in [("base", a), ("candidate", b)] {
            requirements.push(requirement(
                endpoint,
                r.config()["registries"]["workflow"].as_str().unwrap(),
                "/integration/tests",
                "integration",
                &triggers,
                &r.workflow()["integration"]["tests"],
            ));
        }
    }
    Ok(
        json!({"schema":"chrono-workflow-selection/v1","integration_required":required,
        "rule_changes":changes,"trigger_paths":triggers,"requirements":requirements,
        "certification":"not performed; this output selects declared tests only"}),
    )
}
fn requirement(
    endpoint: &str,
    registry: &str,
    pointer: &str,
    reason: &str,
    paths: &BTreeSet<String>,
    tests: &Value,
) -> Value {
    let tests: BTreeSet<_> = tests
        .as_array()
        .unwrap()
        .iter()
        .map(|t| format!("test:{}", t.as_str().unwrap()))
        .collect();
    let edges: Vec<_> = paths
        .iter()
        .flat_map(|p| {
            tests.iter().map(move |t| Edge {
                from: format!("file:{p}"),
                kind: EdgeKind::TestExecution,
                to: t.clone(),
            })
        })
        .collect();
    json!({"endpoint":endpoint,"registry":registry,"pointer":pointer,"reason":reason,"trigger_paths":paths,"tests":tests,"edges":edges})
}
pub fn edges(selection: &Value, endpoint: &str) -> BTreeSet<Edge> {
    selection["requirements"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|r| r["endpoint"] == endpoint)
        .flat_map(|r| r["edges"].as_array().unwrap())
        .map(|e| serde_json::from_value(e.clone()).unwrap())
        .collect()
}
pub fn reference(selection: &Value, edge: &Edge) -> Option<String> {
    selection["requirements"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .find(|(_, r)| {
            r["edges"]
                .as_array()
                .unwrap()
                .contains(&serde_json::to_value(edge).unwrap())
        })
        .map(|(i, _)| format!("/impact/workflow/requirements/{i}"))
}
