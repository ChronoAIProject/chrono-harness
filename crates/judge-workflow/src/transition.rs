use chrono_harness::{facts, wire::Request};
use chrono_judge_projects::Results;
use chrono_judge_registration::Registrations;
use serde_json::{Value, json};
use std::collections::BTreeSet;
fn declaration<'a>(r: &'a Registrations, kind: &str, id: &str) -> Result<&'a Value, String> {
    let rows: Vec<_> = r.workflow()["retirements"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|x| x["kind"] == kind && x["id"] == id)
        .collect();
    if rows.len() != 1 {
        return Err(format!("E_RETIREMENT: missing/ambiguous {kind}:{id}"));
    }
    Ok(rows[0])
}
/// Adjudicate changed/removed identities; unchanged historical records are context.
pub fn evaluate(
    req: &Request,
    a: &Registrations,
    b: &Registrations,
    impact: &Value,
    results: &Results,
    view: &Value,
    costs: &Value,
) -> Result<Value, String> {
    let old = a.node_data();
    let new = b.node_data();
    let requests = chrono_judge_registration::retirement_requests(b)?;
    let mut decisions = vec![];
    let mut ambiguities = vec![];
    for required in impact["required_tests"]
        .as_array()
        .ok_or("E_RETIREMENT: missing obligations")?
    {
        let node = required["node"].as_str().ok_or("E_RETIREMENT: test node")?;
        let Some(defs) = old.get(node).filter(|n| n.definitions.len() > 1) else {
            continue;
        };
        let definitions: Vec<_> = defs
            .definitions
            .iter()
            .map(|d| d.identity.clone())
            .collect();
        let target = requests
            .get(node)
            .and_then(|v| v.as_deref())
            .unwrap_or(node);
        if !chrono_judge_registration::ambiguity_repaired(b, node, &definitions)
            || results.tests.get(target).map(String::as_str) != Some("passed")
            || costs["declared_before"].get(node).is_none()
        {
            return Err(format!(
                "E_RETIREMENT: ambiguity needs explicit repair, old costs and executed replacement: {node}"
            ));
        }
        ambiguities.push(
            json!({"node":node,"definitions":definitions,"replacement":target,"status":"repaired"}),
        );
    }
    for (node, defs) in &old {
        if !node.starts_with("test:") || new.contains_key(node) {
            continue;
        }
        let id = &node[5..];
        let request = requests
            .get(node)
            .ok_or_else(|| format!("E_REQUIRED_TEST_REMOVED: {node}"))?;
        match request {
            Some(to) => {
                if !new.get(to).is_some_and(|n| n.unique().is_some())
                    || results.tests.get(to).map(String::as_str) != Some("passed")
                {
                    return Err(format!(
                        "E_REQUIRED_TEST_REMOVED: replacement not executed: {node}"
                    ));
                }
            }
            None => {
                for definition in &defs.definitions {
                    let d = &definition.value;
                    let owner = d["tests_for"]
                        .as_str()
                        .ok_or("E_RETIREMENT: retired test has no producer")?;
                    let (collection, kind, pair) = if definition.identity.starts_with("project:") {
                        ("projects", "project", "test_project")
                    } else {
                        ("scripts", "script", "test_script")
                    };
                    let originals: Vec<_> = a.projects()[collection]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|p| p["id"] == owner && p[pair] == id)
                        .collect();
                    if originals.len() != 1
                        || b.projects()[collection]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|p| p["id"] == owner || p["id"] == id)
                        || !declaration(b, kind, owner)?["replacement"].is_null()
                    {
                        return Err(format!(
                            "E_RETIREMENT: joint producer/test removal required: {node}"
                        ));
                    }
                }
            }
        }
        if impact["retired_tests"]
            .as_array()
            .unwrap()
            .contains(&json!(node))
            && results.removed.get(node) != Some(request)
        {
            return Err("E_RETIREMENT: execution did not preserve removed obligation".into());
        }
        if costs["declared_before"].get(node).is_none() {
            return Err(format!("E_RETIREMENT: old cost missing: {node}"));
        }
        decisions.push(json!({"kind":"test","id":id,"replacement":request,"status":if request.is_some(){"replaced"}else{"retired"},"executed_old_test":false}));
    }
    for (collection, kind) in [
        ("projects", "project"),
        ("scripts", "script"),
        ("judges", "judge"),
    ] {
        let old_rows: Vec<&Value> = if collection == "judges" {
            a.judges()[collection].as_array().unwrap().iter().collect()
        } else {
            // Include explicitly retained historical definitions removed by a decoder.
            old.iter()
                .filter(|(node, _)| node.starts_with(&format!("{kind}:")))
                .flat_map(|(_, node)| node.definitions.iter().map(|d| d.value))
                .collect()
        };
        let new_rows = if collection == "judges" {
            &b.judges()[collection]
        } else {
            &b.projects()[collection]
        };
        for row in old_rows {
            if row.get("tests_for").is_some() {
                continue;
            }
            let id = row["id"].as_str().unwrap();
            if new_rows.as_array().unwrap().iter().any(|p| p["id"] == id) {
                continue;
            }
            let d = declaration(b, kind, id)?;
            if let Some(to) = d["replacement"].as_str() {
                if !new_rows.as_array().unwrap().iter().any(|p| p["id"] == to) {
                    return Err("E_RETIREMENT: replacement identity missing".into());
                }
            }
            decisions.push(json!({"kind":kind,"id":id,"replacement":d["replacement"],"status":"declared-removal"}));
        }
    }
    let raw = facts::registry_values(&req.candidate.root, &req.base.commit, &req.config_path)?;
    let now = facts::registry_values(&req.candidate.root, &req.candidate.commit, &req.config_path)?;
    let mut changes = BTreeSet::new();
    for key in ["config", "filemap", "projects", "judges", "workflow"] {
        let op = if key == "config" {
            req.config_path.as_str()
        } else {
            raw[&req.config_path]["registries"][key].as_str().unwrap()
        };
        let np = if key == "config" {
            req.config_path.as_str()
        } else {
            now[&req.config_path]["registries"][key].as_str().unwrap()
        };
        let ov = raw[op]["schema_version"].as_u64().unwrap();
        let nv = now[np]["schema_version"].as_u64().unwrap();
        if ov != nv {
            changes.insert((ov, nv));
        }
    }
    let mut migrations = vec![];
    for (from, to) in changes {
        let rows: Vec<_> = b.workflow()["migrations"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|m| m["from_version"] == from && m["to_version"] == to)
            .collect();
        if rows.len() != 1 {
            return Err("E_MIGRATION_EVIDENCE: missing/ambiguous schema migration".into());
        }
        let m = rows[0];
        let test = format!("test:{}", m["test"].as_str().unwrap());
        if results.tests.get(&test).map(String::as_str) != Some("passed")
            || view["conversion"].is_null()
        {
            return Err("E_MIGRATION_EVIDENCE: retained conversion and executed compatibility test required".into());
        }
        let c = &view["conversion"];
        if m["script"] != c["input"]["profile"]["script"]
            || m["test"] != c["input"]["profile"]["test"]
        {
            return Err(
                "E_MIGRATION_EVIDENCE: declared script/test differs from actual conversion pair"
                    .into(),
            );
        }
        let p: chrono_harness::ProcessResult =
            serde_json::from_value(c["process"].clone()).map_err(|e| e.to_string())?;
        super::evidence::process(&p)?;
        if c["output_digest"] != p.stdout_sha256
            || c["input_digest"] != p.stdin_sha256
            || chrono_harness::wire::digest(&c["input"])? != p.stdin_sha256
            || c["output"] != chrono_harness::decode::<Value>(&p.stdout_bytes)?
            || c["output"]["mappings"] != m["mappings"]
        {
            return Err("E_MIGRATION_EVIDENCE: conversion differs from declaration".into());
        }
        migrations.push(m.clone());
    }
    Ok(
        json!({"retirements":decisions,"ambiguities":ambiguities,"migrations":migrations,"history_executed":false}),
    )
}
