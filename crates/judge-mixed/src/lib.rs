//! Explicit rule/product DELTA classification. Warnings neither authorize nor block work.
mod semantic;
use chrono_harness::{
    facts,
    wire::{Delta, Finding, Request, Response, Status},
};
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
/// Classify only explicit endpoint surfaces and named semantic fields.
/// No impact computation, costs, test selection or process execution.
pub fn classify(
    a: &Registrations,
    b: &Registrations,
    delta: &[Delta],
    before: &Documents,
    after: &Documents,
) -> Result<Value, String> {
    let patterns = patterns(a, b);
    let mut changes = BTreeMap::new();
    let mut products = BTreeMap::new();
    for d in delta {
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
                a.config()["registries"]["workflow"] == *path,
                b.config()["registries"]["workflow"] == *path,
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
    }
    Ok(json!({"mixed":!changes.is_empty()&&!products.is_empty(),
        "rule_paths":changes.keys().collect::<Vec<_>>(),"product_paths":products.keys().collect::<Vec<_>>(),
        "changes":changes.values().collect::<Vec<_>>(),"product_changes":products.values().collect::<Vec<_>>(),
        "requires_acknowledgement":false}))
}
/// Package the shared classification with the already-produced cost wire value.
pub fn produce(
    a: &Registrations,
    b: &Registrations,
    delta: &[Delta],
    before: &Documents,
    after: &Documents,
    costs: &Value,
) -> Result<Value, String> {
    if costs["schema"] != "chrono-costs/v1"
        || !costs["declared_before"].is_object()
        || !costs["declared_after"].is_object()
    {
        return Err("E_COST_INPUT: missing or unsupported cost report".into());
    }
    let mut output = classify(a, b, delta, before, after)?;
    for d in delta {
        let node = format!("file:{}", d.path);
        for key in ["declared_before", "declared_after"] {
            if costs[key].get(&node).is_none() {
                return Err(format!("E_COST_INPUT: {key} lacks changed path {node}"));
            }
        }
    }
    output["schema"] = MIXED_SCHEMA.into();
    output["costs"] = costs.clone();
    Ok(output)
}
/// Known interpreted registries for declaration-only consumers. Other named JSON
/// must be supplied explicitly; absence is an error when a pattern consumes it.
pub fn registry_documents(r: &Registrations, _config_path: &str) -> Documents {
    let mut out = BTreeMap::from([(r.effective_config_path().into(), r.config().clone())]);
    out.insert(r.entry_path().into(), r.entry().clone());
    for (key, value) in [
        ("judges", r.judges()),
        ("projects", r.projects()),
        ("filemap", r.filemap()),
        ("workflow", r.workflow()),
    ] {
        out.insert(
            r.config()["registries"][key].as_str().unwrap().into(),
            value.clone(),
        );
    }
    out
}
/// Fixed-object reader shared by the mixed and FILEMAP process boundaries.
pub fn load_documents(
    req: &Request,
    a: &Registrations,
    b: &Registrations,
) -> Result<[Documents; 2], String> {
    let reader = facts::Reader::for_request(req)?;
    load_documents_with_reader(req, a, b, &reader)
}
pub fn load_documents_with_reader(
    req: &Request,
    a: &Registrations,
    b: &Registrations,
    reader: &facts::Reader,
) -> Result<[Documents; 2], String> {
    load_documents_for_endpoints(
        &req.candidate.root,
        &req.base.commit,
        &req.candidate.commit,
        &req.delta,
        a,
        b,
        reader,
    )
}
/// Read only explicitly named semantic documents from immutable Git endpoints.
#[allow(clippy::too_many_arguments)]
pub fn load_documents_for_endpoints(
    root: &std::path::Path,
    base: &str,
    candidate: &str,
    delta: &[Delta],
    a: &Registrations,
    b: &Registrations,
    reader: &facts::Reader,
) -> Result<[Documents; 2], String> {
    let paths = patterns(a, b).into_iter().map(|(p, _, _)| p).collect();
    Ok([
        documents(root, base, delta, a, true, &paths, reader)?,
        documents(root, candidate, delta, b, false, &paths, reader)?,
    ])
}

fn documents(
    root: &std::path::Path,
    commit: &str,
    delta: &[Delta],
    r: &Registrations,
    base: bool,
    paths: &BTreeSet<String>,
    reader: &facts::Reader,
) -> Result<Documents, String> {
    let mut docs = Documents::new();
    for path in paths {
        let exists = delta.iter().find(|d| &d.path == path).is_some_and(|d| {
            if base {
                d.old_blob.is_some()
            } else {
                d.new_blob.is_some()
            }
        });
        if !exists {
            continue;
        }
        let mut value = (path == r.effective_config_path()).then(|| r.config().clone());
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
                let bytes = reader.blob(root, commit, path)?;
                chrono_harness::decode(&bytes)
                    .map_err(|e| format!("E_SEMANTIC_INPUT: {path}: {e}"))?
            }
        };
        docs.insert(path.clone(), value);
    }
    Ok(docs)
}
fn evaluate(req: &Request, reader: &facts::Reader) -> Result<(Value, String), String> {
    req.validate()?;
    let (base, candidate, _) = chrono_judge_registration::views_with_reader(req, reader)?;
    if req.impact["schema"] != "chrono-filemap-impact/v2" {
        return Err("E_IMPACT: unsupported schema".into());
    }
    let delta: Vec<Delta> = serde_json::from_value(req.impact["delta"].clone())
        .map_err(|e| format!("E_IMPACT: {e}"))?;
    let sources: Vec<_> = req
        .prior_results
        .iter()
        .filter_map(|r| r.outputs.get("impact"))
        .collect();
    if sources.is_empty() || sources.iter().any(|v| **v != req.impact) || delta != req.delta {
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
    let [before, after] = load_documents_with_reader(req, &base, &candidate, reader)?;
    let mut output = produce(&base, &candidate, &req.delta, &before, &after, costs)?;
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
    let reader = facts::Reader::for_request(req);
    let result = reader
        .as_ref()
        .map_err(Clone::clone)
        .and_then(|reader| evaluate(req, reader));
    if let Ok(reader) = &reader {
        reader.record(&mut response);
    }
    match result {
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
