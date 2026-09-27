//! Full-format orchestration: fixed facts, declared DAG, external processes and aggregation.
use crate::{
    facts, json, no_symlink_parents, sha256,
    wire::{self, Binding, Context, Endpoint, Executable, Registries, Request, Response, Status},
};
use serde_json::{Value, json as value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

pub fn schedule(bindings: &[Binding]) -> Result<Vec<Binding>, String> {
    schedule_mode(bindings, "every-delta", "evaluate")
}
pub(crate) fn schedule_mode(
    bindings: &[Binding],
    selector: &str,
    mode: &str,
) -> Result<Vec<Binding>, String> {
    let mut pending = BTreeMap::new();
    for b in bindings {
        if b.id.is_empty() || pending.insert(b.id.clone(), b.clone()).is_some() {
            return Err("duplicate/empty judge ID".into());
        }
        if b.selector != selector || b.modes != [mode] {
            return Err("unsupported judge transport selection/mode".into());
        }
        if b.after.iter().collect::<BTreeSet<_>>().len() != b.after.len() {
            return Err("duplicate predecessor".into());
        }
    }
    if pending.is_empty() {
        return Err("no configured judges".into());
    }
    for b in bindings {
        for id in &b.after {
            if !pending.contains_key(id) {
                return Err(format!("missing predecessor {id}"));
            }
        }
    }
    let mut ordered = vec![];
    let mut done = BTreeSet::new();
    while !pending.is_empty() {
        let id = pending
            .iter()
            .find(|(_, b)| b.after.iter().all(|id| done.contains(id)))
            .map(|(id, _)| id.clone())
            .ok_or("judge dependency cycle")?;
        ordered.push(pending.remove(&id).ok_or("missing scheduled judge")?);
        done.insert(id);
    }
    Ok(ordered)
}
pub fn execute(
    template: &Request,
    bindings: &[Binding],
    env: &BTreeMap<String, String>,
    timeout: u64,
    limit: usize,
) -> Result<(Status, Vec<Value>), String> {
    let ordered = schedule(bindings)?;
    let mut responses: BTreeMap<String, Response> = BTreeMap::new();
    let mut records = vec![];
    let mut status = Status::Pass;
    for b in ordered {
        let blocked: Vec<_> = b
            .after
            .iter()
            .filter(|id| responses.get(*id).is_none_or(|r| r.status >= Status::Fail))
            .cloned()
            .collect();
        if !blocked.is_empty() {
            records.push(value!({"id":b.id,"binding":b,"state":"blocked","blocked_by":blocked,"exit_code":null}));
            status = status.max(Status::Error);
            continue;
        }
        let mut req = template.clone();
        req.judge_id = b.id.clone();
        req.prior_results = b.after.iter().map(|id| responses[id].clone()).collect();
        if !req.observations.is_object() {
            req.observations = value!({});
        }
        req.observations["judges"] = value!(records);
        let impacts: Vec<_> = req
            .prior_results
            .iter()
            .filter_map(|r| r.outputs.get("impact"))
            .collect();
        if impacts.windows(2).any(|p| p[0] != p[1]) {
            records.push(value!({"id":b.id,"binding":b,"state":"blocked","exit_code":null,"transport_failure":"conflicting predecessor impact outputs"}));
            status = Status::Error;
            continue;
        }
        if let Some(impact) = impacts.first() {
            req.impact = (*impact).clone();
        }
        req.seal()?;
        match wire::invoke_detailed(&req, &b, env, timeout, limit) {
            Ok((r, p)) => {
                status = status.max(r.status.clone());
                records.push(value!({"id":b.id,"binding":b,"state":"executed","request_id":req.request_id,"request_digest":sha256(&wire::canonical(&req)?),"exit_code":p.exit_code,"response":r,"process":p}));
                responses.insert(b.id, r);
            }
            Err(e) => {
                status = Status::Error;
                let exit = e.process.as_ref().map(|p| p.exit_code);
                records.push(value!({"id":b.id,"binding":b,"state":if e.process.is_some(){"executed"}else{"error"},"request_id":req.request_id,"transport_failure":e.message,"exit_code":exit,"process":e.process}));
            }
        }
    }
    Ok((status, records))
}
pub fn check_observed(
    root: &Path,
    config_path: &str,
    base: &str,
    candidate: &str,
    context: &Path,
    entry: Value,
) -> Result<(u8, String), String> {
    let base_tree = facts::verify_oid(root, base)?;
    let candidate_tree = facts::verify_oid(root, candidate)?;
    let base_values = facts::registry_values(root, base, config_path)?;
    let candidate_values = facts::registry_values(root, candidate, config_path)?;
    let cfg = &candidate_values[config_path];
    let judges_path = cfg["registries"]["judges"]
        .as_str()
        .ok_or("missing judges path")?;
    let bindings: Vec<Binding> =
        serde_json::from_value(candidate_values[judges_path]["judges"].clone())
            .map_err(|e| e.to_string())?;
    let mut env = BTreeMap::new();
    let mut observed = BTreeMap::new();
    for k in cfg["environment"]["inherit"]
        .as_array()
        .ok_or("missing environment.inherit")?
    {
        let k = k.as_str().ok_or("environment key must be string")?;
        let v = std::env::var(k).ok();
        observed.insert(k.to_string(), v.clone());
        if let Some(v) = v {
            env.insert(k.into(), v);
        }
    }
    for (k, v) in cfg["environment"]["values"]
        .as_object()
        .ok_or("missing environment.values")?
    {
        env.insert(
            k.clone(),
            v.as_str().ok_or("environment value must be string")?.into(),
        );
    }
    if cfg["protocol"]["id"] != wire::PROTOCOL || cfg["protocol"]["encoding"] != "UTF-8" {
        return Err("unsupported protocol/encoding".into());
    }
    let timeout = cfg["protocol"]["timeout_seconds"]
        .as_u64()
        .filter(|n| *n > 0)
        .ok_or("invalid protocol timeout")?;
    let limit = cfg["protocol"]["stdout_limit_bytes"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= 64 * 1024 * 1024)
        .ok_or("invalid protocol output bound")? as usize;
    let a = facts::tree(root, base)?;
    let b = facts::tree(root, candidate)?;
    let delta = facts::delta(&a, &b);
    let checkout = facts::checkout(root, candidate)?;
    let state = no_symlink_parents(root, ".chrono-harness/state")?;
    fs::create_dir_all(&state).map_err(|e| e.to_string())?;
    let snapshot = tempfile::Builder::new()
        .prefix("base-")
        .tempdir_in(&state)
        .map_err(|e| e.to_string())?;
    facts::export(root, base, &a, snapshot.path())?;
    let snapshot = snapshot.keep();
    let context = fs::canonicalize(if context.is_absolute() {
        context.to_path_buf()
    } else {
        root.join(context)
    })
    .map_err(|e| e.to_string())?;
    let context_value = json(&fs::read(&context).map_err(|e| e.to_string())?)?;
    let context_digest = wire::digest(&context_value)?;
    let retained = match context_value.get("retained_inputs").and_then(Value::as_str) {
        Some(path) => json(&fs::read(no_symlink_parents(root, path)?).map_err(|e| e.to_string())?)?,
        None => Value::Null,
    };
    let runner = std::env::current_exe().map_err(|e| e.to_string())?;
    let runner = Executable {
        path: runner.to_str().ok_or("non UTF-8 runner path")?.into(),
        sha256: sha256(&fs::read(&runner).map_err(|e| e.to_string())?),
        version: env!("CARGO_PKG_VERSION").into(),
    };
    let run = format!("{}:{:?}", std::process::id(), std::time::SystemTime::now());
    let report_path = format!(".chrono-harness/state/run-{}.json", wire::digest(&run)?);
    let retained_report = no_symlink_parents(root, &report_path)?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&retained_report)
        .map_err(|e| e.to_string())?;
    let req = Request {
        observations: value!({"entry":entry,"run":run,"report_path":report_path,"environment":{"inherited":observed,"effective":env},"retained":retained}),
        protocol: wire::PROTOCOL.into(),
        request_id: String::new(),
        judge_id: String::new(),
        mode: "evaluate".into(),
        base: Endpoint {
            commit: base.into(),
            tree: base_tree,
            root: snapshot.clone(),
        },
        candidate: Endpoint {
            commit: candidate.into(),
            tree: candidate_tree.clone(),
            root: root.into(),
        },
        delta: delta.clone(),
        registries: Registries {
            base: snapshot.join(".chrono-harness"),
            candidate: root.join(".chrono-harness"),
            digest: facts::registry_digest(&base_values, &candidate_values)?,
        },
        context: Context {
            path: context,
            sha256: context_digest.clone(),
        },
        impact: Value::Null,
        prior_results: vec![],
        config_path: config_path.into(),
        checkout,
        runner: runner.clone(),
    };
    let (status, mut judges) = execute(&req, &bindings, &env, timeout, limit)?;
    let mut unresolved = BTreeMap::<String, String>::new();
    let mut sources = BTreeMap::<String, Vec<String>>::new();
    let mut executables = vec![value!(runner)];
    let mut findings = vec![];
    let mut missing_responses = vec![];
    let mut missing_processes = vec![];
    for (index, judge) in judges.iter_mut().enumerate() {
        if judge["process"].is_object() {
            let executable_index = executables.len();
            executables.push(value!({
                "path":judge["process"]["executable"],
                "sha256":judge["process"]["sha256"],
                "version":null
            }));
            judge["executable_index"] = value!(executable_index);
            for (field, process_field) in [("path", "executable"), ("sha256", "sha256")] {
                sources.insert(
                    format!("/executables/{executable_index}/{field}"),
                    vec![format!("/judges/{index}/process/{process_field}")],
                );
            }
            unresolved.insert(
                format!("/executables/{executable_index}/version"),
                format!("judge version was not observed; configured metadata is /judges/{index}/binding/version"),
            );
        } else {
            missing_processes.push(judge["id"].clone());
        }
        if let Some(items) = judge["response"]["findings"].as_array() {
            for (finding_index, finding) in items.iter().enumerate() {
                sources.insert(
                    format!("/findings/{}", findings.len()),
                    vec![format!("/judges/{index}/response/findings/{finding_index}")],
                );
                findings.push(finding.clone());
            }
        } else {
            missing_responses.push(judge["id"].clone());
        }
    }
    if !missing_processes.is_empty() {
        unresolved.insert("/executables".into(), format!("only observed executables listed; process observations unavailable for {missing_processes:?}; see judge states/transport failures"));
    }
    if !missing_responses.is_empty() {
        unresolved.insert("/findings".into(), format!("only validated responses aggregated; responses unavailable for {missing_responses:?}; see judge states/transport failures"));
    }
    let findings = if missing_responses.len() == judges.len() {
        Value::Null
    } else {
        value!(findings)
    };
    let mut report = value!({"schema_version":1,"scope":"configured-judges","status":status,"base":base,"candidate":candidate,"candidate_tree":candidate_tree,"context_digest":context_digest,"registry_digest":req.registries.digest,"executables":executables,"environment":{"inherited":observed,"effective":env},"parity":{"status":"unestablished","compared_report":null},"delta":delta,"judges":judges,"findings":findings,"base_snapshot":snapshot});
    report["report_path"] = value!(report_path);
    // Only validated named outputs supply these fields. Configuration and absent
    // producers cannot stand in for observations; conflicting results remain visible.
    for field in ["tools", "effective_inputs", "impact", "tests", "costs"] {
        let outputs: Vec<_> = judges
            .iter()
            .enumerate()
            .filter_map(|(index, judge)| {
                judge["response"]["outputs"]
                    .get(field)
                    .map(|output| (index, output))
            })
            .collect();
        sources.insert(
            format!("/{field}"),
            outputs
                .iter()
                .map(|(index, _)| format!("/judges/{index}/response/outputs/{field}"))
                .collect(),
        );
        let reason = if outputs.is_empty() {
            Some(format!(
                "no validated judge response supplied outputs.{field}; configured metadata is not an observed result"
            ))
        } else if outputs.iter().any(|(_, output)| output.is_null()) {
            Some(format!(
                "a producer supplied null for outputs.{field}; see sources"
            ))
        } else if outputs.windows(2).any(|pair| pair[0].1 != pair[1].1) {
            Some(format!(
                "conflicting outputs.{field}; see sources and original judge responses"
            ))
        } else {
            None
        };
        report[field] = if let Some(reason) = reason {
            unresolved.insert(format!("/{field}"), reason);
            Value::Null
        } else {
            outputs[0].1.clone()
        };
    }
    report["unresolved"] = value!(unresolved);
    report["sources"] = value!(sources);
    let text = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())? + "\n";
    let path = no_symlink_parents(root, ".chrono-harness/state/report.json")?;
    fs::write(retained_report, &text).map_err(|e| e.to_string())?;
    fs::write(path, &text).map_err(|e| e.to_string())?;
    Ok((status.exit_code() as u8, text))
}

/// Library callers supply their own observed entry through check_observed for routes validation.
pub fn check(
    root: &Path,
    config_path: &str,
    base: &str,
    candidate: &str,
    context: &Path,
) -> Result<(u8, String), String> {
    check_observed(root, config_path, base, candidate, context, Value::Null)
}
