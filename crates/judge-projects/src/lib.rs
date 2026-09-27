//! Pair admissibility and actual operation/test evidence. Selection remains FILEMAP-owned.
use chrono_harness::{
    CommandSpec, facts, run_process_observed,
    wire::{Finding, Request, Response, Status},
};
use chrono_judge_registration::Registrations;
use chrono_judge_routes::{Execution, Receipt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path},
};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperationResult {
    pub operation: String,
    pub status: String,
    pub receipt: Option<Receipt>,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Results {
    pub plan: String,
    pub selected: Vec<String>,
    pub executed: Vec<OperationResult>,
    pub blocked: Vec<OperationResult>,
    pub tests: BTreeMap<String, String>,
    pub removed: BTreeMap<String, Option<String>>,
}
impl Results {
    pub fn has_errors(&self) -> bool {
        self.executed.iter().any(|r| r.status == "error")
    }
    pub fn passed(&self) -> bool {
        self.executed.iter().all(|r| r.status == "passed")
            && self.blocked.is_empty()
            && self.tests.values().all(|s| s == "passed")
    }
}
/// Shared by full projects and the explicitly scoped CI adapter; no alternate process engine.
pub fn execute(plan: &Execution) -> Result<Results, String> {
    plan.validate()?;
    let mut statuses: BTreeMap<String, String> = BTreeMap::new();
    let mut results = Results {
        plan: plan.identity.clone(),
        selected: plan.selected.keys().cloned().collect(),
        executed: vec![],
        blocked: vec![],
        tests: BTreeMap::new(),
        removed: BTreeMap::new(),
    };
    for op in &plan.operations {
        if op
            .predecessors
            .iter()
            .any(|id| statuses.get(id).map(String::as_str) != Some("passed"))
        {
            statuses.insert(op.method.operation.clone(), "blocked".into());
            results.blocked.push(OperationResult {
                operation: op.method.operation.clone(),
                status: "blocked".into(),
                receipt: None,
                error: Some("failed prerequisite".into()),
            });
            continue;
        }
        let tool = plan
            .tools
            .get(&op.method.tool)
            .ok_or("E_TOOL_BINDING: missing plan tool")?;
        let spec = CommandSpec {
            program: tool.path.to_str().ok_or("tool UTF-8")?.into(),
            args: op.method.argv.clone(),
            env: plan.environment.clone(),
            timeout_seconds: op.timeout_seconds,
            output_limit_bytes: op.output_limit_bytes,
        };
        let result = run_process_observed(&plan.root, &spec, &[], &tool.sha256)
            .and_then(|p| Receipt::new(plan, op, p));
        let (receipt, error, status) = match result {
            Ok(receipt) => {
                let infrastructure = chrono_judge_routes::compare(plan, op, Some(&receipt))
                    .err()
                    .or_else(|| receipt.process.failure.clone());
                let (error, status) = if let Some(error) = infrastructure {
                    (Some(error), "error")
                } else if receipt.process.exit_code != 0 {
                    (
                        Some(format!("operation exit {}", receipt.process.exit_code)),
                        "failed",
                    )
                } else {
                    (None, "passed")
                };
                (Some(receipt), error, status)
            }
            Err(e) => (None, Some(e), "error"),
        };
        let status = status.to_string();
        statuses.insert(op.method.operation.clone(), status.clone());
        results.executed.push(OperationResult {
            operation: op.method.operation.clone(),
            status,
            receipt,
            error,
        });
    }
    for (test, p) in &plan.selected {
        results.tests.insert(
            test.clone(),
            if p.operations
                .iter()
                .all(|id| statuses.get(id).map(String::as_str) == Some("passed"))
            {
                "passed"
            } else {
                "failed"
            }
            .into(),
        );
    }
    Ok(results)
}
fn normalized(path: &Path) -> Result<std::path::PathBuf, String> {
    let mut out = std::path::PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return Err("dependency path escapes host".into());
                }
            }
            _ => out.push(c),
        }
    }
    Ok(out)
}
pub fn pairs(root: &Path, r: &Registrations, affected: &BTreeSet<String>) -> Result<(), String> {
    let projects = r.projects()["projects"].as_array().unwrap();
    let scripts = r.projects()["scripts"].as_array().unwrap();
    let files = r.filemap()["files"].as_array().unwrap();
    let registered = |path: &str, owner: &str| -> Result<(), String> {
        let matches: Vec<_> = files
            .iter()
            .filter(|f| f["path"] == path && f["owner"] == owner)
            .collect();
        if matches.len() != 1 || !root.join(path).is_file() {
            return Err(format!(
                "E_TEST_PAIR: missing/exclusive owner {owner}: {path}"
            ));
        }
        Ok(())
    };
    let edge = |from: &str, kind: &str, to: &str| {
        r.filemap()["project_edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["from"] == from && e["kind"] == kind && e["to"] == to)
    };
    let mut owners = BTreeSet::new();
    for id in affected {
        if let Some(node) = r.node_data().get(id) {
            for d in &node.definitions {
                owners.insert(d.identity.clone());
            }
        }
    }
    for (collection, prefix, test_field) in [
        (projects, "project", "test_project"),
        (scripts, "script", "test_script"),
    ] {
        for row in collection {
            let id = row["id"].as_str().unwrap();
            if !owners.contains(&format!("{prefix}:{id}")) {
                continue;
            }
            let (production, test) = if let Some(test) = row[test_field].as_str() {
                (
                    row,
                    collection
                        .iter()
                        .find(|p| p["id"] == test)
                        .ok_or("E_TEST_PAIR: missing dedicated test")?,
                )
            } else {
                let prod = row["tests_for"]
                    .as_str()
                    .ok_or("E_TEST_PAIR: tests_for missing")?;
                (
                    collection
                        .iter()
                        .find(|p| p["id"] == prod)
                        .ok_or("E_TEST_PAIR: missing production")?,
                    row,
                )
            };
            let prod_id = production["id"].as_str().unwrap();
            let test_id = test["id"].as_str().unwrap();
            if production[test_field] != test["id"]
                || test["tests_for"] != production["id"]
                || prod_id == test_id
                || collection
                    .iter()
                    .filter(|p| p[test_field] == test["id"])
                    .count()
                    != 1
                || collection
                    .iter()
                    .filter(|p| p["tests_for"] == production["id"])
                    .count()
                    != 1
                || prefix == "project"
                    && (production["kind"] != "production" || test["kind"] != "test")
            {
                return Err("E_TEST_PAIR: nonreciprocal/shared dedicated pair".into());
            }
            if !edge(
                &format!("{prefix}:{prod_id}"),
                "test-execution",
                &format!("test:{test_id}"),
            ) {
                return Err("E_TEST_PAIR: missing explicit pair execution edge".into());
            }
            for member in [production, test] {
                let owner = member["id"].as_str().unwrap();
                if prefix == "script" {
                    let path = member["path"].as_str().unwrap();
                    registered(path, owner)?;
                    if collection.iter().filter(|p| p["path"] == path).count() != 1 {
                        return Err("E_TEST_PAIR: shared script path".into());
                    }
                    continue;
                }
                for field in ["manifest", "lockfile", "root"] {
                    if collection
                        .iter()
                        .filter(|p| p[field] == member[field])
                        .count()
                        != 1
                    {
                        return Err(format!("E_TEST_PAIR: shared {field}"));
                    }
                }
                for field in ["manifest", "lockfile"] {
                    registered(member[field].as_str().unwrap(), owner)?;
                }
                let target = format!("{}/target/", member["root"].as_str().unwrap());
                if r.config()["artifacts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|a| a["path"] == target && a["owner"] == owner)
                    .count()
                    != 1
                {
                    return Err("E_TEST_PAIR: distinct registered target required".into());
                }
                let path = member["manifest"].as_str().unwrap();
                let manifest: toml::Value = fs::read_to_string(root.join(path))
                    .map_err(|e| e.to_string())?
                    .parse()
                    .map_err(|e| format!("E_MANIFEST: {e}"))?;
                if manifest.get("workspace").is_some()
                    || manifest
                        .get("package")
                        .and_then(|p| p.get("workspace"))
                        .is_some()
                {
                    return Err("E_TEST_PAIR: Cargo workspace aggregation".into());
                }
                // Cargo searches parent manifests for workspace membership. Only registered
                // ancestors of this affected Rust manifest participate in this consistency check.
                for ancestor in Path::new(path).parent().unwrap().ancestors().skip(1) {
                    let ancestor_manifest = ancestor.join("Cargo.toml");
                    if !files
                        .iter()
                        .any(|f| f["path"].as_str() == ancestor_manifest.to_str())
                    {
                        continue;
                    }
                    let value: toml::Value = fs::read_to_string(root.join(&ancestor_manifest))
                        .map_err(|e| format!("E_MANIFEST: {}: {e}", ancestor_manifest.display()))?
                        .parse()
                        .map_err(|e| format!("E_MANIFEST: {}: {e}", ancestor_manifest.display()))?;
                    if value.get("workspace").is_some() {
                        return Err(format!(
                            "E_TEST_PAIR: registered ancestor workspace {} for {path}",
                            ancestor_manifest.display()
                        ));
                    }
                }
                // TOML is a consistency consumer. These references never add graph edges or select tests.
                let mut tables = vec![&manifest];
                if let Some(targets) = manifest.get("target").and_then(toml::Value::as_table) {
                    tables.extend(targets.values());
                }
                for table in tables {
                    for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
                        if let Some(deps) = table.get(key).and_then(toml::Value::as_table) {
                            for (name, dep) in deps {
                                if dep.get("workspace").is_some() {
                                    return Err("E_TEST_PAIR: workspace dependency".into());
                                }
                                if let Some(relative) =
                                    dep.get("path").and_then(toml::Value::as_str)
                                {
                                    let manifest_path = normalized(
                                        &Path::new(path)
                                            .parent()
                                            .unwrap()
                                            .join(relative)
                                            .join("Cargo.toml"),
                                    )?;
                                    let target = projects
                                        .iter()
                                        .find(|p| p["manifest"].as_str() == manifest_path.to_str())
                                        .ok_or_else(|| {
                                            format!("E_INPUT_UNDECLARED: dependency {name}")
                                        })?;
                                    if !edge(
                                        &format!("project:{}", target["id"].as_str().unwrap()),
                                        "compile",
                                        &format!("project:{owner}"),
                                    ) {
                                        return Err(format!(
                                            "E_DANGLING_EDGE: undeclared manifest dependency {name}"
                                        ));
                                    }
                                } else {
                                    return Err(format!(
                                        "E_INPUT_UNDECLARED: bounded project consumer requires explicit retained dependency closure for {name}; registry dependency support unresolved"
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
pub fn snapshot(root: &Path, candidate: &str, artifacts: &[String]) -> Result<(), String> {
    let state = facts::checkout(root, candidate)?;
    if state.head != candidate
        || !state.tracked.is_empty()
        || state
            .untracked
            .iter()
            .any(|p| !artifacts.iter().any(|a| p.starts_with(a)))
        || state
            .index_flags
            .iter()
            .any(|f| f.tag.as_bytes()[0].is_ascii_lowercase() || f.tag.eq_ignore_ascii_case("S"))
    {
        return Err("E_SNAPSHOT_DIRTY: operation changed candidate inputs".into());
    }
    Ok(())
}
pub fn judge(req: &Request) -> Response {
    let mut response = req.response(Status::Pass);
    match evaluate(req, &mut response) {
        Ok(()) => {}
        Err(message) => {
            if response.status != Status::Error {
                response.status = Status::Fail;
            }
            response.findings.push(Finding {
                code: message.split(':').next().unwrap_or("E_PROJECTS").into(),
                level: "error".into(),
                message,
                delta_refs: vec!["/request".into()],
                causes: vec![],
            });
        }
    }
    response
}
fn evaluate(req: &Request, response: &mut Response) -> Result<(), String> {
    req.validate()?;
    let (old, r, _) = chrono_judge_registration::views(req)?;
    let inputs = chrono_judge_registration::inputs::validate(req, &old, &r)?;
    let plans: Vec<_> = req
        .prior_results
        .iter()
        .filter_map(|r| r.outputs.get("execution_plan"))
        .collect();
    if plans.len() != 1 {
        return Err("E_ROUTE_MISSING: checked execution plan".into());
    }
    let plan: Execution = serde_json::from_value(plans[0].clone()).map_err(|e| e.to_string())?;
    plan.validate()?;
    if plan.binding != chrono_judge_routes::binding(req, &inputs) || plan.root != req.candidate.root
    {
        return Err("E_PLAN_IDENTITY: endpoints/registry/context/inputs/run".into());
    }
    let impact: chrono_judge_filemap::Impact =
        serde_json::from_value(req.impact.clone()).map_err(|e| e.to_string())?;
    let mut affected: BTreeSet<_> = impact.closure.reached.keys().cloned().collect();
    affected.extend(plan.selected.keys().cloned());
    pairs(&req.candidate.root, &r, &affected)?;
    let artifacts: Vec<_> = r.config()["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["path"].as_str().unwrap().to_string())
        .collect();
    snapshot(&req.candidate.root, &req.candidate.commit, &artifacts)?;
    let mut results = execute(&plan)?;
    if results.has_errors() {
        response.status = Status::Error;
    }
    let requests = chrono_judge_registration::retirement_requests(&r)?;
    let deferred = chrono_judge_registration::downstream_validator(&r, &req.judge_id);
    for removed in &impact.retired_tests {
        match requests.get(removed) {
            Some(Some(replacement)) => {
                results
                    .removed
                    .insert(removed.clone(), Some(replacement.clone()));
                if results.tests.get(replacement).map(String::as_str) != Some("passed") {
                    response
                        .outputs
                        .insert("tests".into(), serde_json::to_value(&results).unwrap());
                    return Err(format!(
                        "E_REQUIRED_TEST_REMOVED: replacement did not succeed: {removed}"
                    ));
                }
            }
            Some(None) if deferred => {
                results.removed.insert(removed.clone(), None);
            }
            _ => return Err(format!("E_REQUIRED_TEST_REMOVED: {removed}")),
        }
    }
    response
        .outputs
        .insert("tests".into(), serde_json::to_value(&results).unwrap());
    snapshot(&req.candidate.root, &req.candidate.commit, &artifacts)?;
    chrono_judge_registration::inputs::validate(req, &old, &r)
        .map_err(|e| format!("E_EVIDENCE_UNRESOLVED: post-execution {e}"))?;
    if !results.passed() {
        return Err("E_EXECUTION: selected operations/tests failed or blocked".into());
    }
    response.outputs.insert("execution_boundary".into(),json!({"input_completeness_proven":false,"retirement_certification":"unresolved: workflow-owned"}));
    Ok(())
}
