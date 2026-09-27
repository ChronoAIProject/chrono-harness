use chrono_harness::{
    ProcessResult, sha256,
    wire::{self, Request, Response, Status},
};
use chrono_judge_projects::Results;
use chrono_judge_registration::Registrations;
use chrono_judge_routes::Execution;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub fn output<'a>(req: &'a Request, key: &str) -> Result<&'a Value, String> {
    let values: Vec<_> = req
        .prior_results
        .iter()
        .filter_map(|r| r.outputs.get(key))
        .collect();
    if values.is_empty()
        || values.iter().any(|v| v.is_null())
        || values.windows(2).any(|v| v[0] != v[1])
    {
        return Err(format!("E_WORKFLOW_INPUT: missing/conflicting {key}"));
    }
    Ok(values[0])
}
pub fn process(p: &ProcessResult) -> Result<(), String> {
    if p.exit_code != 0
        || p.failure.is_some()
        || p.stdout_sha256 != sha256(&p.stdout_bytes)
        || p.stderr_sha256 != sha256(&p.stderr_bytes)
        || p.stdout != String::from_utf8_lossy(&p.stdout_bytes)
        || p.stderr != String::from_utf8_lossy(&p.stderr_bytes)
        || p.environment_digest != wire::digest(&p.environment)?
    {
        return Err("E_WORKFLOW_EVIDENCE: invalid process result".into());
    }
    Ok(())
}
pub fn record(v: &Value) -> Result<(Response, ProcessResult), String> {
    let r: Response = serde_json::from_value(v["response"].clone())
        .map_err(|e| format!("E_WORKFLOW_EVIDENCE: {e}"))?;
    let p: ProcessResult = serde_json::from_value(v["process"].clone())
        .map_err(|e| format!("E_WORKFLOW_EVIDENCE: {e}"))?;
    process(&p)?;
    if v["state"] != "executed"
        || v["id"] != r.judge_id
        || v["request_id"] != r.request_id
        || v["request_digest"] != p.stdin_sha256
        || v["exit_code"] != 0
        || r.status >= Status::Fail
        || chrono_harness::decode::<Value>(&p.stdout_bytes)? != v["response"]
    {
        return Err("E_WORKFLOW_EVIDENCE: response/process identity".into());
    }
    Ok((r, p))
}
pub fn execution(plan: &Execution, results: &Results) -> Result<(), String> {
    plan.validate()?;
    let selected: Vec<_> = plan.selected.keys().cloned().collect();
    if results.plan != plan.identity
        || results.selected != selected
        || results.tests.keys().cloned().collect::<Vec<_>>() != selected
        || !results.passed()
        || results.executed.len() != plan.operations.len()
    {
        return Err("E_WORKFLOW_EVIDENCE: incomplete test execution".into());
    }
    let mut seen = BTreeSet::new();
    for op in &plan.operations {
        let matches: Vec<_> = results
            .executed
            .iter()
            .filter(|x| x.operation == op.method.operation)
            .collect();
        if matches.len() != 1 || !seen.insert(&op.method.operation) {
            return Err("E_WORKFLOW_EVIDENCE: duplicate/missing operation".into());
        }
        let result = matches[0];
        if result.error.is_some() || result.status != "passed" {
            return Err("E_WORKFLOW_EVIDENCE: failed operation".into());
        }
        chrono_judge_routes::compare(plan, op, result.receipt.as_ref())?;
        process(&result.receipt.as_ref().unwrap().process)?;
    }
    Ok(())
}
pub fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| format!("host:{}", p.display()))
        .unwrap_or_else(|_| format!("external:{}", path.display()))
}
fn tools(plan: &Execution) -> Result<Value, String> {
    let mut out = BTreeMap::new();
    for (id, t) in &plan.tools {
        process(&t.version)?;
        if t.version.sha256 != t.sha256
            || t.version.executable != t.path
            || t.version.cwd != plan.root
            || t.version.environment != plan.environment
            || t.version.argv.first() != t.path.to_str().map(|s| s.to_string()).as_ref()
        {
            return Err("E_WORKFLOW_EVIDENCE: tool observation mismatch".into());
        }
        out.insert(id,json!({"program":t.program,"path":relative(&plan.root,&t.path),"sha256":t.sha256,"version_argv":&t.version.argv[1..],"version_stdout_sha256":t.version.stdout_sha256,"version_stderr_sha256":t.version.stderr_sha256}));
    }
    Ok(json!(out))
}
pub fn inputs(
    v: &Value,
    base: &str,
    candidate: &str,
    tree: &str,
    root: &Path,
) -> Result<Value, String> {
    if !matches!(
        v["schema"].as_str(),
        Some("chrono-effective-inputs/v1" | "chrono-effective-inputs/v2")
    ) || v["identity"] != wire::digest(&v["endpoints"])?
        || v["endpoints"]["base"]["commit"] != base
        || v["endpoints"]["candidate"]["commit"] != candidate
    {
        return Err("E_WORKFLOW_INPUT: effective input identity".into());
    }
    let mut e = v["endpoints"].clone();
    e["candidate"]
        .as_object_mut()
        .ok_or("E_WORKFLOW_INPUT: endpoint")?
        .remove("commit");
    e["candidate"]["tree"] = tree.into();
    for name in ["base", "candidate"] {
        for f in e[name]["files"]
            .as_object_mut()
            .ok_or("E_WORKFLOW_INPUT: files")?
            .values_mut()
        {
            let p = f["location"].as_str().ok_or("E_WORKFLOW_INPUT: location")?;
            let p = Path::new(p);
            let normalized = if p.is_absolute() {
                relative(root, p)
            } else {
                format!("host:{}", p.display())
            };
            f["location"] = normalized.into();
        }
    }
    Ok(e)
}
pub fn binding(
    req: &Request,
    r: &Registrations,
    plan: &Execution,
    results: &Results,
    impact: &Value,
    effective: &Value,
) -> Result<Value, String> {
    execution(plan, results)?;
    if plan.binding != chrono_judge_routes::binding(req, effective)
        || plan.root != req.candidate.root
    {
        return Err("E_WORKFLOW_EVIDENCE: current plan binding".into());
    }
    let mut executables = BTreeMap::new();
    executables.insert("runner".to_string(),json!({"path":relative(&req.candidate.root,Path::new(&req.runner.path)),"sha256":req.runner.sha256}));
    let records = req.observations["judges"]
        .as_array()
        .ok_or("E_WORKFLOW_EVIDENCE: missing actual judge observations")?;
    let configured = r.judges()["judges"].as_array().unwrap();
    let mut seen = BTreeSet::new();
    for v in records {
        let (response, p) = record(v)?;
        let id = &response.judge_id;
        let defs: Vec<_> = configured.iter().filter(|j| j["id"] == *id).collect();
        if defs.len() != 1
            || !seen.insert(id.clone())
            || v["binding"] != *defs[0]
            || p.executable
                != req
                    .candidate
                    .root
                    .join(defs[0]["executable"].as_str().unwrap())
            || defs[0]["sha256"] != p.sha256
            || p.cwd != req.candidate.root
            || p.environment != plan.environment
        {
            return Err("E_WORKFLOW_EVIDENCE: executed judge binding".into());
        }
        executables.insert(
            format!("judge:{id}"),
            json!({"path":relative(&req.candidate.root,&p.executable),"sha256":p.sha256}),
        );
    }
    let expected: BTreeSet<_> = configured
        .iter()
        .map(|j| j["id"].as_str().unwrap().to_string())
        .filter(|id| id != &req.judge_id)
        .collect();
    if expected != seen {
        return Err("E_WORKFLOW_EVIDENCE: workflow must follow all configured judges".into());
    }
    for p in &req.prior_results {
        let v = records
            .iter()
            .find(|v| v["id"] == p.judge_id)
            .ok_or("E_WORKFLOW_EVIDENCE: predecessor not observed")?;
        if v["response"] != serde_json::to_value(p).unwrap() {
            return Err("E_WORKFLOW_EVIDENCE: predecessor differs".into());
        }
    }
    let myself = std::env::current_exe().map_err(|e| e.to_string())?;
    let bytes = std::fs::read(&myself).map_err(|e| e.to_string())?;
    let defs: Vec<_> = configured
        .iter()
        .filter(|j| j["id"] == req.judge_id)
        .collect();
    if defs.len() != 1
        || defs[0]["sha256"] != sha256(&bytes)
        || myself
            != req
                .candidate
                .root
                .join(defs[0]["executable"].as_str().unwrap())
    {
        return Err("E_WORKFLOW_EVIDENCE: current workflow executable".into());
    }
    executables.insert(
        format!("judge:{}", req.judge_id),
        json!({"path":relative(&req.candidate.root,&myself),"sha256":sha256(&bytes)}),
    );
    let methods = chrono_judge_registration::execution::methods(r.projects())?;
    let mut contract = vec![];
    for op in &plan.operations {
        let d = methods
            .get(&op.method.operation)
            .filter(|v| v.len() == 1)
            .ok_or("E_WORKFLOW_EVIDENCE: method declaration")?;
        if op.method.tool != d[0].tool
            || op.method.argv != chrono_judge_routes::expand(&d[0].argv, &plan.binding)
        {
            return Err("E_WORKFLOW_EVIDENCE: changed method".into());
        }
        contract.push(json!({"method":d[0],"predecessors":op.predecessors,"timeout_seconds":op.timeout_seconds,"output_limit_bytes":op.output_limit_bytes}));
    }
    Ok(
        json!({"base":req.base.commit,"candidate_tree":req.candidate.tree,"registry_digest":req.registries.digest,"executables":executables,"tools":tools(plan)?,"environment":plan.environment,"effective_inputs":inputs(effective,&req.base.commit,&req.candidate.commit,&req.candidate.tree,&req.candidate.root)?,"required_tests":{"obligations":impact["required_tests"],"selected":plan.selected,"methods":contract}}),
    )
}
