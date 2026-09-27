//! Explicit declared costs for the already-selected FILEMAP impact. No cost inference.
use chrono_harness::wire::{Finding, Request, Response, Status};
use chrono_judge_filemap::{IMPACT_SCHEMA, Impact};
use chrono_judge_registration::Registrations;
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;

pub const COST_SCHEMA: &str = "chrono-costs/v1";
const COORDINATES: [&str; 4] = ["cpu_ms", "wall_ms", "peak_rss_bytes", "io_bytes"];
fn escape(s: &str) -> String {
    s.replace('~', "~0").replace('/', "~1")
}
fn reference(r: &Registrations, id: &str, member: &str, pointer: String) -> Result<Value, String> {
    let model = r.filemap()["cost_models"]
        .get(id)
        .ok_or_else(|| format!("E_COST_REFERENCE: missing cost model {id} for {member}"))?;
    let registry = &r.config()["registries"]["filemap"];
    Ok(json!({"reference":id,"member":member,"value":model,
        "source":{"registry":registry,"pointer":format!("/cost_models/{}",escape(id))},
        "declaration":{"registry":registry,"pointer":pointer}}))
}
fn file_cost(r: &Registrations, index: usize, file: &Value) -> Result<Value, String> {
    reference(
        r,
        file["cost"].as_str().unwrap(),
        &format!("file:{}", file["path"].as_str().unwrap()),
        format!("/files/{index}/cost"),
    )
}
fn entry(r: &Registrations, node: &str) -> Result<Value, String> {
    let files = r.filemap()["files"].as_array().unwrap();
    if let Some(path) = node.strip_prefix("file:") {
        let Some((i, f)) = files.iter().enumerate().find(|(_, f)| f["path"] == path) else {
            return Ok(Value::Null);
        };
        return Ok(json!({"kind":"file","costs":[file_cost(r,i,f)?]}));
    }
    if let Some(id) = node.strip_prefix("test:") {
        if !r.node_data().contains_key(node) {
            return Ok(Value::Null);
        }
        let (i, row) = r.filemap()["test_costs"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .find(|(_, row)| row["test"] == id)
            .ok_or_else(|| format!("E_COST_REFERENCE: missing test cost {node}"))?;
        return Ok(
            json!({"kind":"test","costs":[reference(r,row["cost"].as_str().unwrap(),node,
            format!("/test_costs/{i}/cost"))?]}),
        );
    }
    let (kind, id) = if let Some(id) = node.strip_prefix("project:") {
        ("project", id)
    } else if let Some(id) = node.strip_prefix("script:") {
        ("script", id)
    } else {
        return Err(format!("E_COST_REFERENCE: unsupported cost subject {node}"));
    };
    // The candidate decoder retains obsolete definitions in the historical view.
    // Their declared member costs remain meaningful without executing old code.
    if !r.node_data().contains_key(node) {
        return Ok(Value::Null);
    }
    // Ownership is declared data. Keep each member and its source; costs are not additive by default.
    let mut members: Vec<_> = files
        .iter()
        .enumerate()
        .filter(|(_, f)| f["owner"] == id)
        .collect();
    members.sort_by_key(|(_, f)| f["path"].as_str().unwrap());
    let costs = members
        .into_iter()
        .map(|(i, f)| file_cost(r, i, f))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({"kind":kind,"costs":costs}))
}

/// Declaration-only report over FILEMAP's existing impact. No operations or resource measurement.
pub fn produce(
    base: &Registrations,
    candidate: &Registrations,
    impact: &Impact,
) -> Result<Value, String> {
    if impact.schema != IMPACT_SCHEMA {
        return Err("E_IMPACT: unsupported cost input schema".into());
    }
    let mut subjects: BTreeSet<String> = impact
        .delta
        .iter()
        .map(|d| format!("file:{}", d.path))
        .collect();
    subjects.extend(
        impact
            .closure
            .reached
            .keys()
            .filter(|n| {
                n.starts_with("file:") || n.starts_with("project:") || n.starts_with("script:")
            })
            .cloned(),
    );
    subjects.extend(impact.required_tests.iter().map(|t| t.node.clone()));
    let mut before = Map::new();
    let mut after = Map::new();
    let mut unknown = vec![];
    let mut removed = vec![];
    for subject in subjects {
        let a = entry(base, &subject)?;
        let b = entry(candidate, &subject)?;
        if a.is_null() && b.is_null() {
            return Err(format!(
                "E_COST_REFERENCE: no declaration at either endpoint for {subject}"
            ));
        }
        if !a.is_null() && b.is_null() {
            removed.push(subject.clone());
        }
        for (endpoint, value) in [("base", &a), ("candidate", &b)] {
            let Some(costs) = value["costs"].as_array() else {
                continue;
            };
            if costs.is_empty() {
                unknown.push(json!({"endpoint":endpoint,"node":subject,"reference":null,
                    "coordinates":COORDINATES,"reason":"no declared member cost references"}));
            }
            for cost in costs {
                let coordinates: Vec<_> = COORDINATES
                    .iter()
                    .filter(|k| cost["value"][**k].is_null())
                    .collect();
                if !coordinates.is_empty() {
                    unknown.push(json!({"endpoint":endpoint,"node":subject,"member":cost["member"],
                        "reference":cost["reference"],"source":cost["source"],"coordinates":coordinates,
                        "reason":cost["value"]["basis"]}));
                }
            }
        }
        before.insert(subject.clone(), a);
        after.insert(subject, b);
    }
    Ok(
        json!({"schema":COST_SCHEMA,"declared_before":before,"declared_after":after,
        "affected_tests":impact.tests,"retired_tests":impact.retired_tests,"removed_nodes":removed,
        "unknown":unknown,"measured":null,
        "measurement_status":"not measured; declarations are estimates, not process measurements",
        "aggregation":"none; member references are retained without summing resource peaks or parallel wall time"}),
    )
}

pub fn judge(req: &Request) -> Response {
    let mut response = req.response(Status::Pass);
    match evaluate(req) {
        Ok(costs) => {
            if !costs["unknown"].as_array().unwrap().is_empty() {
                response.status = Status::Warn;
                response.findings.push(Finding {
                    code: "W_COST_UNKNOWN".into(),
                    level: "warning".into(),
                    message:
                        "Affected declared costs contain unknown coordinates; see costs.unknown"
                            .into(),
                    delta_refs: vec!["/impact".into()],
                    causes: vec!["/outputs/costs/unknown".into()],
                });
            }
            response.outputs.insert("costs".into(), costs);
        }
        Err(message) => {
            response.status = Status::Error;
            response.findings.push(Finding {
                code: message
                    .split(':')
                    .next()
                    .filter(|s| s.starts_with("E_"))
                    .unwrap_or("E_INPUT")
                    .into(),
                level: "error".into(),
                message,
                delta_refs: vec!["/request".into()],
                causes: vec![],
            });
        }
    }
    response
}
fn evaluate(req: &Request) -> Result<Value, String> {
    req.validate()?;
    let (base, candidate, _) = chrono_judge_registration::views(req)?;
    let impact: Impact =
        serde_json::from_value(req.impact.clone()).map_err(|e| format!("E_IMPACT: {e}"))?;
    let sources: Vec<_> = req
        .prior_results
        .iter()
        .filter_map(|r| r.outputs.get("impact"))
        .collect();
    if sources.is_empty() || sources.iter().any(|v| **v != req.impact) || impact.delta != req.delta
    {
        return Err("E_IMPACT: missing or conflicting bound FILEMAP output".into());
    }
    let mut costs = produce(&base, &candidate, &impact)?;
    costs["binding"] = json!({"base":req.base.commit,"candidate":req.candidate.commit,
        "candidate_tree":req.candidate.tree,"registry_digest":req.registries.digest,"context_digest":req.context.sha256});
    Ok(costs)
}
