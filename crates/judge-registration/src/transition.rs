//! Finite candidate-owned decoders for explicitly named historical profiles.
use crate::{Registrations, execution};
use chrono_harness::{
    CommandSpec, facts, json,
    observation::{self, Tool},
    run_process_observed, sha256,
    wire::{self, Request},
};
use serde_json::{Value, json as value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
type Values = BTreeMap<String, Value>;
fn load_view(view: &Value, config: &str) -> Result<(Registrations, Registrations), String> {
    let a: Values = serde_json::from_value(view["base"].clone()).map_err(|e| e.to_string())?;
    let b: Values = serde_json::from_value(view["candidate"].clone()).map_err(|e| e.to_string())?;
    let mut old = Registrations::load(&a, config)?;
    old.historical =
        serde_json::from_value(view["historical"].clone()).map_err(|e| e.to_string())?;
    if let Some(raw) = view["conversion"]["input"].get("original") {
        let raw: Values = serde_json::from_value(raw.clone()).map_err(|e| e.to_string())?;
        old.original_schemas = Registrations::schemas(&raw, config)?;
        // Supported config versions already have readers. Historical snapshots
        // bind their original config, never a decoder's replacement semantics.
        if a.get(config) != raw.get(config) {
            return Err(
                "E_MIGRATION_INPUT_SEMANTICS: historical config must remain original".into(),
            );
        }
    }
    Ok((old, Registrations::load(&b, config)?))
}
pub fn views(req: &Request) -> Result<(Registrations, Registrations, Value), String> {
    let reader = facts::Reader::for_request(req)?;
    views_with_reader(req, &reader)
}
pub fn views_with_reader(
    req: &Request,
    reader: &facts::Reader,
) -> Result<(Registrations, Registrations, Value), String> {
    let binding = value!({"base":req.base.commit,"candidate":req.candidate.commit,"registry":req.registries.digest,"context":req.context.sha256});
    let prior: Vec<_> = req
        .prior_results
        .iter()
        .filter_map(|r| r.outputs.get("registration_view"))
        .collect();
    if let Some(v) = prior.first() {
        if prior.iter().any(|p| *p != *v) || v["binding"] != binding {
            return Err("registration view binding mismatch".into());
        }
        let (a, b) = load_view(v, &req.config_path)?;
        return Ok((a, b, (*v).clone()));
    }
    let a = reader.registry_values(&req.candidate.root, &req.base.commit, &req.config_path)?;
    let b = reader.registry_values(&req.candidate.root, &req.candidate.commit, &req.config_path)?;
    if facts::registry_digest(&a, &b)? != req.registries.digest {
        return Err("registry digest mismatch".into());
    }
    let env = serde_json::from_value(req.observations["environment"]["effective"].clone())
        .unwrap_or_default();
    let (a, b, mut view) = interpret_mode(
        reader,
        &req.candidate.root,
        &req.base.commit,
        &req.candidate.commit,
        &req.config_path,
        a,
        b,
        &env,
        Some(&req.judge_id),
    )?;
    view["binding"] = binding;
    Ok((a, b, view))
}
pub fn interpret(
    root: &Path,
    base: &str,
    candidate_oid: &str,
    config: &str,
    raw: Values,
    candidate: Values,
    env: &BTreeMap<String, String>,
) -> Result<(Registrations, Registrations, Value), String> {
    let reader = facts::Reader::for_config(root, config)?;
    interpret_with_reader(
        &reader,
        root,
        base,
        candidate_oid,
        config,
        raw,
        candidate,
        env,
    )
}
/// Reuse the caller's selected facts reader and retain its complete acquisition record.
pub fn interpret_with_reader(
    reader: &facts::Reader,
    root: &Path,
    base: &str,
    candidate_oid: &str,
    config: &str,
    raw: Values,
    candidate: Values,
    env: &BTreeMap<String, String>,
) -> Result<(Registrations, Registrations, Value), String> {
    reader.verify_config(root, candidate_oid)?;
    interpret_mode(
        reader,
        root,
        base,
        candidate_oid,
        config,
        raw,
        candidate,
        env,
        None,
    )
}
fn interpret_mode(
    reader: &facts::Reader,
    root: &Path,
    base: &str,
    candidate_oid: &str,
    config: &str,
    raw: Values,
    candidate: Values,
    env: &BTreeMap<String, String>,
    consumer: Option<&str>,
) -> Result<(Registrations, Registrations, Value), String> {
    let new = Registrations::load(&candidate, config).map_err(|e| format!("candidate: {e}"))?;
    let deferred = consumer.is_some_and(|id| downstream_validator(&new, id));
    let filemap = raw[config]["registries"]["filemap"]
        .as_str()
        .ok_or("historical filemap path")?;
    let mut view = value!({"base":raw,"candidate":candidate,"historical":{},"conversion":null});
    let versions = |values: &Values| -> Result<Value, String> {
        Ok(value!(
            Registrations::schemas(values, config)?
                .into_iter()
                .map(|(key, (_, version))| (key, version))
                .collect::<BTreeMap<_, _>>()
        ))
    };
    let from = versions(&raw)?;
    let to = versions(&candidate)?;
    let mut matching = vec![];
    let mut legacy_required = false;
    for profile in new.workflow()["historical_profiles"]
        .as_array()
        .into_iter()
        .flatten()
    {
        if profile.get("from_versions").is_some() {
            if profile["from_versions"] == from && profile["to_versions"] == to {
                matching.push((profile, Value::Null, Value::Null, Value::Null));
            }
        } else if profile["filemap_version"] == raw[filemap]["schema_version"] {
            legacy_required = true;
            let path = profile["profile_path"].as_str().unwrap();
            let original = reader.blob(root, base, path);
            if let Err(e) = &original {
                if e.starts_with("E_GIT_FACTS:") {
                    return Err(e.clone());
                }
            }
            if let Ok(bytes) = original {
                let value = json(&bytes)?;
                if value["schema"] == profile["id"] {
                    matching.push((profile, value!(path), value!(bytes), value));
                }
            }
        }
    }
    if matching.is_empty() && !legacy_required {
        let old = Registrations::load(&raw, config).map_err(|e| format!("base: {e}"))?;
        return Ok((old, new, view));
    }
    if matching.len() != 1 {
        return Err("E_MIGRATION_PROFILE: missing/ambiguous historical profile".into());
    }
    let (profile, path, profile_bytes, profile_value) = matching.remove(0);
    let scripts = new.projects()["scripts"].as_array().unwrap();
    let script = scripts
        .iter()
        .find(|s| s["id"] == profile["script"])
        .ok_or("missing registered migration script")?;
    let test = scripts
        .iter()
        .find(|s| s["id"] == profile["test"])
        .ok_or("missing dedicated migration test")?;
    if script["test_script"] != test["id"] || test["tests_for"] != script["id"] {
        return Err("E_TEST_PAIR: migration script".into());
    }
    let op = script["actions"]["execute"]["operation"]
        .as_str()
        .ok_or("migration execute action")?;
    let all = execution::methods(new.projects())?;
    let methods = all.get(op).ok_or("migration method missing")?;
    if methods.len() != 1 {
        return Err("E_ROUTE_AMBIGUOUS: migration".into());
    }
    let method = &methods[0];
    let t = new.config()["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == method.tool)
        .ok_or("migration tool missing")?;
    let argv: Vec<String> =
        serde_json::from_value(t["version_argv"].clone()).map_err(|e| e.to_string())?;
    let before = reader.checkout(root, candidate_oid)?;
    let tool = observation::tool(
        root,
        t["program"].as_str().unwrap(),
        &argv,
        env,
        30,
        1048576,
    )?;
    if tool.version.exit_code != 0
        || tool.version.failure.is_some()
        || t["expected_version"].as_str() != Some(tool.version.stdout.trim_end())
    {
        return Err(format!(
            "E_TOOL_BINDING: migration version mismatch; observation={}",
            value!({"tool_id":method.tool,"expected_version":t["expected_version"],"tool":tool})
        ));
    }
    let raw_bytes: BTreeMap<_, _> = raw
        .keys()
        .map(|p| Ok((p.clone(), reader.blob(root, base, p)?)))
        .collect::<Result<_, String>>()?;
    let input = if profile.get("from_versions").is_some() {
        value!({"schema":"chrono-historical-decode/v2","config_path":config,"original":raw,"original_bytes":raw_bytes,"candidate":candidate,"profile":profile})
    } else {
        value!({"schema":"chrono-historical-decode/v1","config_path":config,"original":raw,"original_bytes":raw_bytes,"profile":profile,"profile_bytes":profile_bytes,"profile_value":profile_value})
    };
    let script_path = script["path"].as_str().unwrap();
    let script_bytes = fs::read(root.join(script_path)).map_err(|e| e.to_string())?;
    if script_bytes != reader.blob(root, candidate_oid, script_path)? {
        return Err("E_MIGRATION: decoder differs from fixed candidate".into());
    }
    let spec = CommandSpec {
        program: tool.path.to_str().unwrap().into(),
        args: method.argv.clone(),
        env: env.clone(),
        timeout_seconds: 30,
        output_limit_bytes: 16 * 1024 * 1024,
    };
    let receipt = run_process_observed(root, &spec, &wire::canonical(&input)?, &tool.sha256)?;
    if receipt.exit_code != 0 || receipt.failure.is_some() {
        return Err(format!(
            "E_MIGRATION: exit {}: {}",
            receipt.exit_code, receipt.stderr
        ));
    }
    let after = reader.checkout(root, candidate_oid)?;
    if before != after {
        return Err("E_SNAPSHOT_DIRTY: migration changed candidate inputs".into());
    }
    let output = json(&receipt.stdout_bytes)?;
    if output["mappings"] != profile["mappings"] {
        return Err("E_MIGRATION: mapping mismatch".into());
    }
    view["base"] = output["values"].clone();
    view["historical"] = output["historical"].clone();
    view["conversion"] = value!({"input":input,"input_digest":receipt.stdin_sha256,"output":output,"output_digest":receipt.stdout_sha256,"profile_path":path,"script":script_path,"script_sha256":sha256(&script_bytes),"tool_id":method.tool,"tool":tool,"process":receipt,"certification":"unresolved: workflow producer required"});
    let (old, _) = load_view(&view, config)?;
    // Every explicitly removed malformed definition must survive as its original bytes/value.
    for record in profile["legacy_records"].as_array().unwrap() {
        let original = raw[raw[config]["registries"]["projects"].as_str().unwrap()]
            [record["collection"].as_str().unwrap()]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == record["id"]);
        if original != Some(&record["definition"]) {
            return Err("E_MIGRATION: historical definition mismatch".into());
        }
        for node in record["nodes"].as_array().unwrap() {
            if !old
                .historical
                .get(node.as_str().unwrap())
                .is_some_and(|defs| defs.iter().any(|(_, v)| Some(v) == original))
            {
                return Err("E_MIGRATION: lost historical definition".into());
            }
        }
    }
    let nodes = new.node_data();
    let mappings = profile["mappings"].as_array().unwrap();
    for record in profile["legacy_records"].as_array().unwrap() {
        for node in record["nodes"].as_array().unwrap() {
            let matches: Vec<_> = mappings.iter().filter(|m| m["from"] == *node).collect();
            if matches.len() != 1
                || !nodes
                    .get(matches[0]["to"].as_str().unwrap())
                    .is_some_and(|n| n.unique().is_some())
            {
                return Err(
                    "E_MIGRATION: missing/ambiguous mapping to unique candidate identity".into(),
                );
            }
        }
        let owner = format!("owner:{}", record["id"].as_str().unwrap());
        if !new.projects()["owners"]
            .as_array()
            .unwrap()
            .contains(&record["id"])
        {
            let matches: Vec<_> = mappings.iter().filter(|m| m["from"] == owner).collect();
            if matches.len() != 1
                || !matches[0]["to"]
                    .as_str()
                    .and_then(|s| s.strip_prefix("owner:"))
                    .is_some_and(|id| {
                        new.projects()["owners"]
                            .as_array()
                            .unwrap()
                            .contains(&value!(id))
                    })
            {
                return Err("E_MIGRATION: missing retired owner mapping".into());
            }
        }
        for action in record["definition"]["actions"]
            .as_object()
            .unwrap()
            .values()
        {
            if deferred {
                continue;
            }
            let current = all
                .get(action["operation"].as_str().unwrap())
                .ok_or("E_MIGRATION: historical method lost")?;
            if current.len() != 1
                || current[0].tool != action["tool"]
                || value!(current[0].argv) != action["argv"]
            {
                return Err(
                    "E_MIGRATION: historical method changed without supported mapping".into(),
                );
            }
        }
    }
    Ok((old, new, view))
}
pub fn reused_tools(view: &Value) -> Result<BTreeMap<String, Tool>, String> {
    let mut out = BTreeMap::new();
    let c = &view["conversion"];
    if let Some(id) = c["tool_id"].as_str() {
        out.insert(
            id.into(),
            serde_json::from_value(c["tool"].clone()).map_err(|e| e.to_string())?,
        );
    }
    Ok(out)
}
pub fn replacements(r: &Registrations) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    for row in r.workflow()["retirements"].as_array().unwrap() {
        if row["kind"] != "test" {
            continue;
        }
        if let Some(to) = row["replacement"].as_str() {
            let from = format!("test:{}", row["id"].as_str().unwrap());
            let to = if to.starts_with("test:") {
                to.to_string()
            } else {
                format!("test:{to}")
            };
            if out.insert(from, to).is_some() {
                return Err("E_REQUIRED_TEST_REMOVED: ambiguous replacement".into());
            }
        }
    }
    Ok(out)
}
pub fn ambiguity_repaired(r: &Registrations, node: &str, definitions: &[String]) -> bool {
    let repairs: Vec<_> = r.workflow()["historical_profiles"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|p| p["ambiguities"].as_array().into_iter().flatten())
        .filter(|p| p["node"] == node)
        .collect();
    if repairs.len() != 1 {
        return false;
    }
    let repair = repairs[0];
    let declared: BTreeSet<_> = repair["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let actual: BTreeSet<_> = definitions.iter().map(String::as_str).collect();
    let Some(to) = repair["replacement"].as_str() else {
        return false;
    };
    declared == actual
        && r.node_data().get(to).is_some_and(|n| n.unique().is_some())
        && (to == node
            || replacements(r)
                .ok()
                .is_some_and(|m| m.get(node).is_some_and(|v| v == to)))
}

/// Explicit requests only: null is a requested retirement, never its approval.
pub fn retirement_requests(r: &Registrations) -> Result<BTreeMap<String, Option<String>>, String> {
    let mut out = BTreeMap::new();
    for row in r.workflow()["retirements"].as_array().unwrap() {
        if row["kind"] != "test" {
            continue;
        }
        let from = format!("test:{}", row["id"].as_str().unwrap());
        let to = row["replacement"].as_str().map(|s| {
            if s.starts_with("test:") {
                s.into()
            } else {
                format!("test:{s}")
            }
        });
        if out.insert(from, to).is_some() {
            return Err("E_REQUIRED_TEST_REMOVED: ambiguous retirement request".into());
        }
    }
    Ok(out)
}
/// A declared later validator must consume this judge before a pending obligation
/// can leave planning/execution. The validator, not this graph query, approves it.
pub fn downstream_validator(r: &Registrations, id: &str) -> bool {
    let judges = r.judges()["judges"].as_array().unwrap();
    let Some(validator) = r.judges()["migration_validator"].as_str() else {
        return false;
    };
    if validator == id {
        return false;
    }
    let mut pending = vec![validator];
    let mut seen = BTreeSet::new();
    while let Some(n) = pending.pop() {
        if !seen.insert(n) {
            continue;
        }
        let Some(j) = judges.iter().find(|j| j["id"] == n) else {
            return false;
        };
        for p in j["after"].as_array().unwrap() {
            let Some(p) = p.as_str() else { return false };
            if p == id {
                return true;
            }
            pending.push(p)
        }
    }
    false
}
