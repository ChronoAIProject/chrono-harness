//! Explicit rule/product DELTA classification. Warnings neither authorize nor block work.
mod semantic;
use chrono_harness::{
    facts,
    wire::{Finding, Request, Response, Status},
};
use chrono_judge_filemap::{IMPACT_SCHEMA, Impact};
use chrono_judge_registration::Registrations;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
pub const MIXED_SCHEMA: &str = "chrono-rule-changes/v1";
pub type Documents = BTreeMap<String, Value>;
fn patterns(a: &Registrations, b: &Registrations) -> BTreeSet<(String, String, String)> {
    [a, b]
        .into_iter()
        .flat_map(|r| r.config()["semantic_fields"].as_array().unwrap())
        .flat_map(|row| {
            row["pointers"].as_array().unwrap().iter().map(|p| {
                (
                    row["path"].as_str().unwrap().into(),
                    p.as_str().unwrap().into(),
                    row["on"].as_str().unwrap().into(),
                )
            })
        })
        .collect()
}
fn surface(r: &Registrations, path: &str) -> Option<String> {
    r.filemap()["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["path"] == path)
        .map(|f| f["surface"].as_str().unwrap().to_string())
}
/// Consume existing impact, costs and explicitly named endpoint documents.
/// Registry documents must be the registration judge's interpreted views.
pub fn produce(
    a: &Registrations,
    b: &Registrations,
    impact: &Impact,
    before: &Documents,
    after: &Documents,
    costs: &Value,
) -> Result<Value, String> {
    if impact.schema != IMPACT_SCHEMA {
        return Err("E_IMPACT: unsupported schema".into());
    }
    if costs["schema"] != chrono_judge_cost::COST_SCHEMA
        || !costs["declared_before"].is_object()
        || !costs["declared_after"].is_object()
    {
        return Err("E_COST_INPUT: missing or unsupported cost report".into());
    }
    let patterns = patterns(a, b);
    let mut changes = BTreeMap::new();
    let mut products = BTreeMap::new();
    for d in &impact.delta {
        let old = surface(a, &d.path);
        let new = surface(b, &d.path);
        let surfaces: BTreeSet<_> = old.iter().chain(new.iter()).map(String::as_str).collect();
        if surfaces.is_empty() {
            return Err(format!(
                "E_SURFACE_REFERENCE: no registration for {}",
                d.path
            ));
        }
        let executable_rule = surfaces
            .iter()
            .any(|s| matches!(*s, "judge-policy" | "judge-implementation"));
        let mut fields = vec![];
        for (path, pointer, on) in patterns.iter().filter(|(p, _, _)| p == &d.path) {
            for (exists, docs) in [
                (d.old_blob.is_some(), before),
                (d.new_blob.is_some(), after),
            ] {
                if exists && !docs.contains_key(path) {
                    return Err(format!("E_SEMANTIC_INPUT: missing named document {path}"));
                }
            }
            fields.extend(semantic::changes(
                before.get(path),
                after.get(path),
                pointer,
                on,
            )?);
        }
        let is_product = surfaces.iter().any(|s| matches!(*s, "product" | "test"));
        let classification = json!({"path":d.path,"before_surface":old,"after_surface":new});
        if is_product {
            products.insert(d.path.clone(), classification.clone());
        }
        if executable_rule || !fields.is_empty() {
            let mut rule = classification;
            rule["whole_file"] = executable_rule.into();
            rule["semantic"] = fields.into();
            changes.insert(d.path.clone(), rule);
        }
        let node = format!("file:{}", d.path);
        for key in ["declared_before", "declared_after"] {
            if costs[key].get(&node).is_none() {
                return Err(format!("E_COST_INPUT: {key} lacks changed path {node}"));
            }
        }
    }
    Ok(
        json!({"schema":MIXED_SCHEMA,"mixed":!changes.is_empty()&&!products.is_empty(),
        "rule_paths":changes.keys().collect::<Vec<_>>(),"product_paths":products.keys().collect::<Vec<_>>(),
        "changes":changes.values().collect::<Vec<_>>(),"product_changes":products.values().collect::<Vec<_>>(),
        "costs":costs,"requires_acknowledgement":false}),
    )
}
fn documents(
    req: &Request,
    r: &Registrations,
    base: bool,
    paths: &BTreeSet<String>,
) -> Result<Documents, String> {
    let mut docs = Documents::new();
    for path in paths {
        let exists = req.delta.iter().find(|d| &d.path == path).is_some_and(|d| {
            if base {
                d.old_blob.is_some()
            } else {
                d.new_blob.is_some()
            }
        });
        if !exists {
            continue;
        }
        let mut value = (path == &req.config_path).then(|| r.config().clone());
        for (key, registry) in [
            ("judges", r.judges()),
            ("projects", r.projects()),
            ("filemap", r.filemap()),
            ("workflow", r.workflow()),
        ] {
            if r.config()["registries"][key] == *path {
                value = Some(registry.clone());
            }
        }
        let value = match value {
            Some(v) => v,
            None => {
                let commit = if base {
                    &req.base.commit
                } else {
                    &req.candidate.commit
                };
                let bytes = facts::blob(&req.candidate.root, commit, path)?;
                chrono_harness::decode(&bytes)
                    .map_err(|e| format!("E_SEMANTIC_INPUT: {path}: {e}"))?
            }
        };
        docs.insert(path.clone(), value);
    }
    Ok(docs)
}
fn evaluate(req: &Request) -> Result<(Value, String), String> {
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
    let binding = json!({"base":req.base.commit,"candidate":req.candidate.commit,
        "candidate_tree":req.candidate.tree,"registry_digest":req.registries.digest,"context_digest":req.context.sha256});
    let sources: Vec<_> = req
        .prior_results
        .iter()
        .filter_map(|r| r.outputs.get("costs"))
        .collect();
    let costs = *sources
        .first()
        .ok_or("E_COST_INPUT: missing predecessor costs")?;
    if sources.iter().any(|c| **c != *costs) || costs["binding"] != binding {
        return Err(
            "E_COST_INPUT: conflicting costs or mismatched endpoint/context binding".into(),
        );
    }
    let paths = patterns(&base, &candidate)
        .into_iter()
        .map(|(p, _, _)| p)
        .collect();
    let before = documents(req, &base, true, &paths)?;
    let after = documents(req, &candidate, false, &paths)?;
    let mut output = produce(&base, &candidate, &impact, &before, &after, costs)?;
    output["binding"] = binding;
    Ok((
        output,
        candidate.workflow()["mixed_change"]["code"]
            .as_str()
            .unwrap()
            .into(),
    ))
}
pub fn judge(req: &Request) -> Response {
    let mut response = req.response(Status::Pass);
    match evaluate(req) {
        Ok((output, code)) => {
            if output["mixed"] == true {
                response.status = Status::Warn;
                response.findings.push(Finding {code,level:"warning".into(),
                    message:format!("Rule and product changes share this DELTA. Rules: {}; products: {}. Before/after costs: outputs.rule_changes.costs; unknown coordinates remain unknown.",output["rule_paths"],output["product_paths"]),
                    delta_refs:output["rule_paths"].as_array().unwrap().iter().chain(output["product_paths"].as_array().unwrap()).map(|v|v.as_str().unwrap().into()).collect(),
                    causes:vec!["/outputs/rule_changes".into()]});
            }
            response.outputs.insert("rule_changes".into(), output);
        }
        Err(message) => {
            response.status = Status::Error;
            response.findings.push(Finding {
                code: message
                    .split(':')
                    .next()
                    .filter(|c| c.starts_with("E_"))
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
