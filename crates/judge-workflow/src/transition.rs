use chrono_harness::{facts, wire::Request};
use chrono_judge_projects::Results;
use chrono_judge_registration::{Registrations, retirement_declaration as declaration};
use serde_json::{Value, json};
use std::collections::BTreeSet;
/// Adjudicate changed/removed identities; unchanged historical records are context.
pub fn evaluate(
    reader: &facts::Reader,
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
    for node in old.keys() {
        if !node.starts_with("test:") || new.contains_key(node) {
            continue;
        }
        let id = &node[5..];
        let request = chrono_judge_registration::validate_test_retirement(a, b, node)?;
        if request
            .as_ref()
            .is_some_and(|to| results.tests.get(to).map(String::as_str) != Some("passed"))
        {
            return Err(format!(
                "E_REQUIRED_TEST_REMOVED: replacement not executed: {node}"
            ));
        }
        if impact["retired_tests"]
            .as_array()
            .unwrap()
            .contains(&json!(node))
            && results.removed.get(node) != Some(&request)
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
    let raw = reader.registry_values(&req.candidate.root, &req.base.commit, &req.config_path)?;
    let now =
        reader.registry_values(&req.candidate.root, &req.candidate.commit, &req.config_path)?;
    let mut changes = BTreeSet::new();
    let raw_config =
        chrono_harness::facts::registry_identity(&raw, &req.config_path)?.effective_path;
    let now_config =
        chrono_harness::facts::registry_identity(&now, &req.config_path)?.effective_path;
    for key in ["config", "filemap", "projects", "judges", "workflow"] {
        let op = if key == "config" {
            raw_config.as_str()
        } else {
            raw[&raw_config]["registries"][key].as_str().unwrap()
        };
        let np = if key == "config" {
            now_config.as_str()
        } else {
            now[&now_config]["registries"][key].as_str().unwrap()
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
            let admitted = super::verified_transition::admit_migration(
                false, false, false, false, false, false, false, false, false, false, false,
            );
            if admitted {
                return Err("E_MIGRATION_EVIDENCE: invalid migration admission facts".into());
            }
            return Err("E_MIGRATION_EVIDENCE: missing/ambiguous schema migration".into());
        }
        let m = rows[0];
        let c = &view["conversion"];
        let has_script = m.get("script").is_some();
        if !has_script {
            let admitted = super::verified_transition::admit_migration(
                true,
                false,
                c.is_null(),
                false,
                false,
                false,
                false,
                false,
                false,
                false,
                false,
            );
            if !admitted {
                return Err("E_MIGRATION_EVIDENCE: actual data conversion requires its declared script/test pair".into());
            }
            migrations.push(m.clone());
            continue;
        }
        let test = m
            .get("test")
            .and_then(Value::as_str)
            .map(|name| format!("test:{name}"));
        let replacement_test_passed = test
            .as_ref()
            .and_then(|test| results.tests.get(test))
            .map(String::as_str)
            == Some("passed");
        let conversion_present = !c.is_null();
        let script_test_matches = m["script"] == c["input"]["profile"]["script"]
            && m["test"] == c["input"]["profile"]["test"];
        let mut process_succeeded = false;
        let mut input_digest_matches = false;
        let mut output_digest_matches = false;
        let mut decoded_output_matches = false;
        let mappings_match = c["output"]["mappings"] == m["mappings"];
        if let Ok(p) = serde_json::from_value::<chrono_harness::ProcessResult>(c["process"].clone())
        {
            process_succeeded = super::evidence::process(&p).is_ok();
            input_digest_matches = c["input_digest"] == p.stdin_sha256
                && chrono_harness::wire::digest(&c["input"])
                    .map(|digest| digest == p.stdin_sha256)
                    .unwrap_or(false);
            output_digest_matches = c["output_digest"] == p.stdout_sha256;
            decoded_output_matches = chrono_harness::decode::<Value>(&p.stdout_bytes)
                .map(|output| c["output"] == output)
                .unwrap_or(false);
        }
        let admitted = super::verified_transition::admit_migration(
            true,
            true,
            false,
            replacement_test_passed,
            conversion_present,
            script_test_matches,
            process_succeeded,
            input_digest_matches,
            output_digest_matches,
            decoded_output_matches,
            mappings_match,
        );
        if !admitted {
            if !replacement_test_passed || !conversion_present {
                return Err("E_MIGRATION_EVIDENCE: declared data conversion requires its retained execution and passing test".into());
            }
            if !script_test_matches {
                return Err(
                    "E_MIGRATION_EVIDENCE: declared script/test differs from actual conversion pair"
                        .into(),
                );
            }
            return Err("E_MIGRATION_EVIDENCE: conversion differs from declaration".into());
        }
        migrations.push(m.clone());
    }
    Ok(
        json!({"retirements":decisions,"ambiguities":ambiguities,"migrations":migrations,"history_executed":false}),
    )
}
