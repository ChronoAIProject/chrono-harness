//! Pair admissibility and actual operation/test evidence. Selection remains FILEMAP-owned.
use chrono_harness::{
    CommandSpec, facts, run_process_observed,
    wire::{self, Binding, Finding, Request, Response, Status},
};
use chrono_judge_registration::Registrations;
use chrono_judge_routes::{Execution, Receipt};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion: Option<CollectedCompletion>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CollectedCompletion {
    pub schema: String,
    pub status: String,
    pub reports: Vec<Value>,
    pub global_selected: Vec<String>,
    pub required_units: Vec<String>,
    pub test_sources: BTreeMap<String, Vec<String>>,
    pub removal_sources: BTreeMap<String, Vec<String>>,
    pub source: String,
    pub source_sha256: String,
    pub tools: Value,
    pub warnings: Vec<Value>,
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
        scope: None,
        completion: None,
    };
    // Scoped threads are joined even if a launch, receipt or worker fails. Claims
    // are acquired together by this sole coordinator, never inside workers.
    let rows = std::thread::scope(|scope| {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut pending: BTreeSet<usize> = (0..plan.operations.len()).collect();
        let mut running = BTreeSet::new();
        let mut rows = BTreeMap::new();
        let cap = plan.scheduling.as_ref().map_or(1, |p| p.max_running);
        while !pending.is_empty() || !running.is_empty() {
            let mut progressed = false;
            for index in pending.iter().copied().collect::<Vec<_>>() {
                let op = &plan.operations[index];
                if op
                    .predecessors
                    .iter()
                    .any(|id| statuses.get(id).is_some_and(|s| s != "passed"))
                {
                    let row = OperationResult {
                        operation: op.method.operation.clone(),
                        status: "blocked".into(),
                        receipt: None,
                        error: Some("failed prerequisite".into()),
                    };
                    statuses.insert(row.operation.clone(), row.status.clone());
                    rows.insert(index, row);
                    pending.remove(&index);
                    progressed = true;
                    continue;
                }
                if running.len() >= cap
                    || op.predecessors.iter().any(|id| !statuses.contains_key(id))
                    || plan.scheduling.as_ref().is_some_and(|policy| {
                        running.iter().any(|i: &usize| {
                            policy.conflicts(
                                &op.method.operation,
                                &plan.operations[*i].method.operation,
                            )
                        })
                    })
                {
                    continue;
                }
                pending.remove(&index);
                running.insert(index);
                progressed = true;
                let tx = tx.clone();
                let launch = std::thread::Builder::new().spawn_scoped(scope, move || {
                    let row = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        execute_operation(plan, op)
                    }))
                    .unwrap_or_else(|_| OperationResult {
                        operation: op.method.operation.clone(),
                        status: "error".into(),
                        receipt: None,
                        error: Some("operation worker panicked".into()),
                    });
                    let _ = tx.send((index, row));
                });
                if let Err(error) = launch {
                    running.remove(&index);
                    let row = OperationResult {
                        operation: op.method.operation.clone(),
                        status: "error".into(),
                        receipt: None,
                        error: Some(format!("operation worker launch: {error}")),
                    };
                    statuses.insert(row.operation.clone(), row.status.clone());
                    rows.insert(index, row);
                }
            }
            if !running.is_empty() {
                let (index, row) = rx.recv().expect("owned workers always send terminal rows");
                running.remove(&index);
                statuses.insert(row.operation.clone(), row.status.clone());
                rows.insert(index, row);
            } else if !pending.is_empty() && !progressed {
                // Routes rejects cycles; a malformed retained graph is still a
                // pre-effect error through Execution::validate.
                unreachable!("validated operation graph has no ready work");
            }
        }
        rows
    });
    for row in rows.into_values() {
        if row.status == "blocked" {
            results.blocked.push(row);
        } else {
            results.executed.push(row);
        }
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
fn execute_operation(plan: &Execution, op: &chrono_judge_routes::Operation) -> OperationResult {
    let result = (|| {
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
        let process = run_process_observed(&plan.root, &spec, &[], &tool.sha256)?;
        Receipt::new(plan, op, process)
    })();
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
    OperationResult {
        operation: op.method.operation.clone(),
        status: status.into(),
        receipt,
        error,
    }
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
            for (identity, bindings) in r.test_bindings() {
                if bindings
                    .iter()
                    .any(|b| b.owner == format!("{prefix}:{test_id}"))
                    && !edge(&format!("{prefix}:{prod_id}"), "test-execution", &identity)
                {
                    return Err(format!(
                        "E_TEST_PAIR: missing explicit group execution edge {identity}"
                    ));
                }
            }
            for member in [production, test] {
                let owner = member["id"].as_str().unwrap();
                if prefix == "script" {
                    let path = member["path"].as_str().unwrap();
                    registered(path, owner)?;
                    if collection.iter().filter(|p| p["path"] == path).count() != 1 {
                        return Err("E_TEST_PAIR: shared script path".into());
                    }
                }
                // Legacy paths remain optional opaque owned inputs. No language parsing,
                // ancestor discovery or inferred output directory belongs to this judge.
                for field in ["manifest", "lockfile"] {
                    if let Some(path) = member[field].as_str() {
                        registered(path, owner)?;
                    }
                }
                let owned: Vec<_> = files.iter().filter(|f| f["owner"] == owner).collect();
                if owned.is_empty() {
                    return Err(format!(
                        "E_TEST_PAIR: project {owner} has no registered files"
                    ));
                }
                for file in owned {
                    registered(file["path"].as_str().unwrap(), owner)?;
                }
                let artifacts = r.config()["artifacts"].as_array().unwrap();
                for artifact in artifacts.iter().filter(|a| a["owner"] == owner) {
                    let path = artifact["path"].as_str().unwrap();
                    if files
                        .iter()
                        .any(|f| f["path"].as_str().unwrap().starts_with(path))
                    {
                        return Err(format!(
                            "E_TEST_PAIR: generated output contains registered input: {path}"
                        ));
                    }
                    for other in artifacts.iter().filter(|a| a["owner"] != owner) {
                        let other_path = other["path"].as_str().unwrap();
                        if path.starts_with(other_path) || other_path.starts_with(path) {
                            return Err(format!(
                                "E_TEST_PAIR: overlapping registered outputs: {path}, {other_path}"
                            ));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
pub fn snapshot(root: &Path, candidate: &str, artifacts: &[String]) -> Result<(), String> {
    snapshot_with_reader(root, candidate, artifacts, &facts::Reader::legacy())
}
pub fn snapshot_with_reader(
    root: &Path,
    candidate: &str,
    artifacts: &[String],
    reader: &facts::Reader,
) -> Result<(), String> {
    let artifacts: Vec<_> = artifacts.iter().map(String::as_str).collect();
    let state = reader.checkout_excluding(root, candidate, &artifacts)?;
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

/// Validate transported full unit reports without invoking any business
/// operation.  The collector consumes the original sealed request and judge
/// observations retained by each unit; it never reconstructs or replays a
/// process locally.
fn collect_reports(
    req: &Request,
    plan: &Execution,
    selected: &BTreeSet<String>,
    registrations: &Registrations,
    old: &Registrations,
    effective: &Value,
) -> Result<(Results, CollectedCompletion), String> {
    let chrono_harness::units::Scope::Collect { manifest } =
        req.scope.as_ref().ok_or("collection scope missing")?
    else {
        return Err("collection requires collect scope".into());
    };
    let units: BTreeMap<String, chrono_harness::units::Unit> =
        serde_json::from_value(req.observations["execution_units"]["units"].clone())
            .map_err(|e| format!("E_COLLECTION_INPUT: units: {e}"))?;
    let required = chrono_harness::units::required(&units, selected);
    let manifest_path = manifest.clone();
    let report_path = req.observations["execution_units"]["report_path"]
        .as_str()
        .ok_or("E_COLLECTION_INPUT: full report path missing")?;
    let unit_paths: Vec<String> = units
        .values()
        .map(|unit| unit.report_path.clone())
        .collect();
    chrono_harness::units::validate_collection_paths(&manifest_path, report_path, &unit_paths)
        .map_err(|error| format!("E_COLLECTION_INPUT: {error}"))?;
    let limits = req.observations["execution_units"]["collection_limits"]
        .as_object()
        .ok_or("E_COLLECTION_INPUT: collection limits missing")?;
    let manifest_limit = limits["manifest_bytes"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= 64 * 1024 * 1024)
        .ok_or("E_COLLECTION_INPUT: invalid manifest limit")?;
    let report_limit = limits["report_bytes"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= 64 * 1024 * 1024)
        .ok_or("E_COLLECTION_INPUT: invalid report limit")?;
    let read_bounded = |path: &str, limit: u64| {
        chrono_harness::units::read_bounded(&req.candidate.root, path, limit)
    };
    let bytes = read_bounded(&manifest_path, manifest_limit)
        .map_err(|e| format!("E_COLLECTION_INPUT: manifest: {e}"))?;
    let manifest_sha256 = chrono_harness::sha256(&bytes);
    let manifest: chrono_harness::units::FullManifest = chrono_harness::decode(&bytes)?;
    if !matches!(
        manifest.schema.as_str(),
        "chrono-full-collection/v1" | "chrono-ci-collection/v1"
    ) {
        return Err("E_COLLECTION_INPUT: unsupported manifest schema".into());
    }
    let mut seen = BTreeSet::new();
    let mut paths = BTreeSet::from([manifest_path.clone()]);
    paths.insert(report_path.to_owned());
    let mut reports = vec![];
    let mut warnings = vec![];
    let mut tools = serde_json::Map::new();
    let mut tests = BTreeMap::<String, String>::new();
    let mut removed = BTreeMap::<String, Option<String>>::new();
    let mut test_sources = BTreeMap::<String, Vec<String>>::new();
    let mut removal_sources = BTreeMap::<String, Vec<String>>::new();
    for input in &manifest.reports {
        if !units.contains_key(&input.unit)
            || !seen.insert(input.unit.clone())
            || !wire::is_digest(&input.sha256)
        {
            return Err("E_COLLECTION_INPUT: unknown or duplicate unit report".into());
        }
        let expected_path = chrono_harness::prepared::original_path(
            &chrono_harness::prepared::Original {
                path: units[&input.unit].report_path.clone(),
                sha256: input.sha256.clone(),
            },
            input.artifacts.as_ref(),
        )?;
        if input.path != expected_path {
            return Err(format!(
                "E_COLLECTION_INPUT: unit report path differs from registration: {}",
                input.unit
            ));
        }
        if paths
            .iter()
            .any(|existing| chrono_harness::units::overlap(existing, &input.path))
            || !paths.insert(input.path.clone())
        {
            return Err("E_COLLECTION_INPUT: report path collision/overlap".into());
        }
        let report_bytes = read_bounded(&input.path, report_limit)
            .map_err(|e| format!("E_COLLECTION_INPUT: {}: {e}", input.unit))?;
        if chrono_harness::sha256(&report_bytes) != input.sha256 {
            return Err(format!(
                "E_COLLECTION_INPUT: report digest mismatch: {}",
                input.unit
            ));
        }
        let report = chrono_harness::json(&report_bytes)?;
        if input
            .runner_sha256
            .as_deref()
            .is_some_and(|pin| pin != req.runner.sha256)
            || input
                .judge_sha256
                .as_deref()
                .is_some_and(|pin| pin != report["judge_sha256"].as_str().unwrap_or_default())
        {
            return Err(format!(
                "E_COLLECTION_INPUT: supplied executable pin differs: {}",
                input.unit
            ));
        }
        if registrations.config()["schema_version"] == 4 {
            if let Some(transport) = &input.artifacts {
                chrono_harness::prepared::validate_portable_binding(
                    &req.candidate.root,
                    &report["request"]["observations"]["preparation"],
                    Some(transport),
                )?;
            }
        }
        let unit_results = validate_unit_report(
            req,
            registrations,
            &units,
            selected,
            &required,
            &input.unit,
            &report,
            &plan,
            old,
            effective,
            &req.candidate.root,
            input.artifacts.as_ref(),
            None,
        )?;
        for (test, status) in &unit_results.tests {
            if let Some(previous) = tests.insert(test.clone(), status.clone()) {
                if previous != *status {
                    return Err(format!(
                        "E_COLLECTION_INPUT: conflicting test coverage: {test}"
                    ));
                }
            }
            test_sources
                .entry(test.clone())
                .or_default()
                .push(input.unit.clone());
        }
        for (retired, replacement) in &unit_results.removed {
            if let Some(previous) = removed.insert(retired.clone(), replacement.clone()) {
                if previous != *replacement {
                    return Err(format!(
                        "E_COLLECTION_INPUT: conflicting removal coverage: {retired}"
                    ));
                }
            }
            removal_sources
                .entry(retired.clone())
                .or_default()
                .push(input.unit.clone());
        }
        for finding in report["findings"].as_array().ok_or("unit findings")? {
            if finding["level"] == "warning" {
                warnings.push(json!({"unit":input.unit,"finding":finding}));
            }
        }
        let status = report["status"].as_str().unwrap_or("");
        if report["schema_version"] != 1
            || report["scope"] != "configured-judges"
            || !matches!(status, "pass" | "warn")
            || report["request"]["scope"] != serde_json::json!({"kind":"unit","unit":input.unit})
            || report["request"]["base"]["commit"] != req.base.commit
            || report["request"]["candidate"]["commit"] != req.candidate.commit
            || report["request"]["candidate"]["tree"] != req.candidate.tree
            || report["request"]["base"]["tree"] != req.base.tree
            || report["request"]["registries"]["digest"] != req.registries.digest
            || report["request"]["context"]["sha256"] != req.context.sha256
            || report["request"]["config_path"] != req.config_path
            || report["delta"] != serde_json::to_value(&req.delta).unwrap()
            || report["candidate_tree"] != req.candidate.tree
            || report["registry_digest"] != req.registries.digest
            || report["context_digest"] != req.context.sha256
        {
            return Err(format!(
                "E_COLLECTION_INPUT: stale or mismatched report: {}",
                input.unit
            ));
        }
        let judges = report["judges"]
            .as_array()
            .ok_or("E_COLLECTION_INPUT: report judges missing")?;
        if judges.is_empty() {
            return Err(format!(
                "E_COLLECTION_INPUT: incomplete judge evidence: {}",
                input.unit
            ));
        }
        let retained_artifacts = chrono_harness::retained_artifacts::restage(
            &req.candidate.root,
            &report["artifacts"],
            input.artifacts.as_ref(),
            ".chrono-harness/state/imported/",
        )?;
        let retained_path = format!(".chrono-harness/state/import-{}.json", input.sha256);
        let destination = chrono_harness::no_symlink_parents(&req.candidate.root, &retained_path)?;
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
        {
            Ok(mut file) => {
                use std::io::Write;
                file.write_all(&report_bytes).map_err(|e| e.to_string())?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                if fs::read(&destination).map_err(|e| e.to_string())? != report_bytes {
                    return Err("E_COLLECTION_INPUT: retained original replaced".into());
                }
            }
            Err(e) => return Err(e.to_string()),
        }
        for (id, observation) in unit_tools(&report)?
            .as_object()
            .ok_or("unit tool bindings")?
        {
            if let Some(previous) = tools.insert(id.clone(), observation.clone()) {
                if previous != *observation {
                    return Err("E_COLLECTION_INPUT: conflicting original tool observations".into());
                }
            }
        }
        reports.push(serde_json::json!({
            "unit": input.unit,
            "path": input.path,
            "retained_path": retained_path,
            "retained_artifacts": retained_artifacts,
            "sha256": input.sha256,
            "runner_sha256": req.runner.sha256,
            "judge_sha256": report["judge_sha256"],
            "status": status,
            "request_digest": report["request_digest"],
            "artifacts": input.artifacts,
        }));
    }
    if required.iter().any(|unit| !seen.contains(unit)) {
        return Err("E_COLLECTION_INPUT: missing required unit report".into());
    }
    for test in selected {
        if tests.get(test).map(String::as_str) != Some("passed") {
            return Err(format!(
                "E_COLLECTION_INPUT: selected test coverage is missing or failed: {test}"
            ));
        }
    }
    let requests = chrono_judge_registration::retirement_requests(registrations)?;
    let deferred = chrono_judge_registration::downstream_validator(registrations, &req.judge_id);
    for retired in req.impact["retired_tests"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        let expected = match requests.get(retired) {
            Some(Some(replacement)) => Some(replacement.clone()),
            Some(None) if deferred => None,
            _ => return Err(format!("E_REQUIRED_TEST_REMOVED: {retired}")),
        };
        match removed.get(retired) {
            Some(actual) if actual != &expected => {
                return Err(format!(
                    "E_COLLECTION_INPUT: removal obligation differs from registration: {retired}"
                ));
            }
            Some(_) => {}
            None => {
                removed.insert(retired.to_owned(), expected);
            }
        }
        removal_sources
            .entry(retired.to_owned())
            .or_default()
            .push("global-obligation".into());
    }
    let completion = CollectedCompletion {
        schema: "chrono-collected-completion/v1".into(),
        status: "passed".into(),
        reports,
        global_selected: selected.iter().cloned().collect(),
        required_units: required.clone(),
        test_sources,
        removal_sources,
        source: manifest_path,
        source_sha256: manifest_sha256,
        tools: Value::Object(tools),
        warnings,
    };
    let results = Results {
        plan: plan.identity.clone(),
        selected: selected.iter().cloned().collect(),
        executed: vec![],
        blocked: vec![],
        tests,
        removed,
        scope: Some("collect".into()),
        completion: Some(completion.clone()),
    };
    Ok((results, completion))
}

/// Validate one transported full-v3 report using only retained bytes and the
/// current declaration view.  Paths and run identities remain at their
/// original coordinates; only typed endpoint and declaration projections are
/// compared with the current collection request.
fn validate_unit_report(
    req: &Request,
    registrations: &Registrations,
    units: &BTreeMap<String, chrono_harness::units::Unit>,
    selected: &BTreeSet<String>,
    required: &[String],
    unit: &str,
    report: &Value,
    global_plan: &Execution,
    old: &Registrations,
    current_effective: &Value,
    declaration_root: &Path,
    transport: Option<&chrono_harness::prepared::ArtifactTransport>,
    closure: Option<&Value>,
) -> Result<Results, String> {
    let top_request: Request = serde_json::from_value(report["request"].clone())
        .map_err(|e| format!("E_COLLECTION_INPUT: retained request: {e}"))?;
    top_request.validate()?;
    if top_request.scope
        != Some(chrono_harness::units::Scope::Unit {
            unit: unit.to_owned(),
        })
        || report["request_digest"] != wire::digest(&top_request)?
    {
        return Err(format!(
            "E_COLLECTION_INPUT: retained request digest differs: {unit}"
        ));
    }
    if report["schema_version"] != 1
        || report["scope"] != "configured-judges"
        || report["execution_scope"] != json!({"kind":"unit","unit":unit})
        || report["base"] != req.base.commit
        || report["candidate"] != req.candidate.commit
        || report["candidate_tree"] != req.candidate.tree
        || report["registry_digest"] != req.registries.digest
        || report["context_digest"] != req.context.sha256
        || report["delta"] != json!(req.delta)
        || report["environment"] != req.observations["environment"]
        || report["base_snapshot"] != json!(top_request.base.root)
        || report["report_path"] != top_request.observations["report_path"]
        || top_request.observations["report_path"]
            != format!(
                "{}run-{}.json",
                top_request.observations["preparation"]["request"]["native_artifacts"]["directory"]
                    .as_str()
                    .unwrap_or(".chrono-harness/state/"),
                wire::digest(&top_request.observations["run"])?
            )
        || top_request.runner.version != req.runner.version
    {
        return Err("E_COLLECTION_INPUT: original report/root/run/runner summary differs".into());
    }
    if report["runner"]["sha256"] != req.runner.sha256 || report["judge_sha256"].as_str().is_none()
    {
        return Err(format!(
            "E_COLLECTION_INPUT: executable pins differ: {unit}"
        ));
    }
    let judges_cfg = registrations.judges()["judges"]
        .as_array()
        .ok_or("E_COLLECTION_INPUT: configured judges missing")?;
    let pins: Vec<_> = judges_cfg
        .iter()
        .map(|j| json!({"id":j["id"],"sha256":j["sha256"]}))
        .collect();
    let judge_pin = wire::digest(&pins)?;
    if report["judge_sha256"] != judge_pin {
        return Err(format!(
            "E_COLLECTION_INPUT: configured judge pins differ: {unit}"
        ));
    }
    let records = report["judges"]
        .as_array()
        .ok_or("E_COLLECTION_INPUT: judge records missing")?;
    let bindings: Vec<Binding> = serde_json::from_value(registrations.judges()["judges"].clone())
        .map_err(|e| e.to_string())?;
    let parsed = chrono_harness::full::retained_judges(&top_request, &bindings, records)?;
    let resolved = chrono_harness::retained_artifacts::resolve_map(&report["artifacts"], closure)?;
    let artifacts = &resolved;
    if registrations.config()["schema_version"] == 4 {
        chrono_harness::prepared::validate_retained_binding(
            declaration_root,
            &top_request.observations["preparation"],
            transport,
            Some(artifacts),
        )?;
    }

    let effective = validate_retained_request_at(
        &top_request,
        old,
        registrations,
        artifacts,
        declaration_root,
        transport,
    )?;
    let reader = facts::Reader::for_config(declaration_root, &top_request.config_path)?;
    chrono_judge_registration::validate_retained_view(
        &top_request,
        &reader,
        declaration_root,
        parsed
            .get("registration")
            .ok_or("original registration")?
            .2
            .outputs
            .get("registration_view")
            .ok_or(
                "E_COLLECTION_INPUT: registration judge missing required output registration_view",
            )?,
    )?;
    if effective
        != chrono_judge_registration::inputs::project_effective(
            current_effective,
            old,
            registrations,
            top_request.scope.as_ref(),
        )?
    {
        return Err(
            "E_COLLECTION_INPUT: original effective inputs differ from current obligations".into(),
        );
    }
    if top_request.observations["execution_units"] != req.observations["execution_units"]
        || top_request.registries.base != top_request.base.root.join(".chrono-harness")
        || top_request.registries.candidate != top_request.candidate.root.join(".chrono-harness")
        || top_request.checkout.head != top_request.candidate.commit
        || !top_request.checkout.tracked.is_empty()
        || !chrono_judge_registration::nonartifact_paths(
            registrations.config(),
            &top_request.checkout.untracked,
        )
        .is_empty()
        || top_request.checkout.index_flags.iter().any(|flag| {
            flag.tag.is_empty()
                || flag.tag.as_bytes()[0].is_ascii_lowercase()
                || flag.tag.eq_ignore_ascii_case("S")
        })
        || top_request.observations["run"]
            .as_str()
            .is_none_or(|s| s.is_empty())
    {
        return Err("E_COLLECTION_INPUT: original root/run/declaration binding".into());
    }
    let mut aggregate = Status::Pass;
    let mut findings = vec![];
    for record in records {
        let (original, _, response, process) = parsed
            .get(record["id"].as_str().ok_or("original judge id")?)
            .ok_or("original judge")?;
        if original.base.commit != req.base.commit
            || original.base.tree != req.base.tree
            || original.candidate.commit != req.candidate.commit
            || original.candidate.tree != req.candidate.tree
            || original.config_path != req.config_path
            || original.registries.digest != req.registries.digest
            || original.context.sha256 != req.context.sha256
            || original.delta != req.delta
            || original.runner.sha256 != req.runner.sha256
            || original.observations["registry_bindings"] != req.observations["registry_bindings"]
            || original.observations["environment"] != req.observations["environment"]
        {
            return Err("E_COLLECTION_INPUT: original fixed request binding differs".into());
        }
        validate_retained_response(original, response, process.exit_code)?;
        aggregate = aggregate.max(response.status.clone());
        findings.extend(
            response
                .findings
                .iter()
                .map(|f| serde_json::to_value(f).unwrap()),
        );
        for evidence in &response.evidence {
            let identity = chrono_harness::retained_artifacts::identity(
                declaration_root,
                artifacts,
                &evidence.path,
                transport,
            )?;
            if identity.0 != evidence.sha256 {
                return Err("E_COLLECTION_INPUT: original judge artifact differs".into());
            }
        }
    }
    if report["status"] != serde_json::to_value(&aggregate).unwrap()
        || report["findings"] != json!(findings)
    {
        return Err("E_COLLECTION_INPUT: report summary differs from actual judges".into());
    }
    let route = parsed
        .get("routes")
        .ok_or("E_COLLECTION_INPUT: routes judge missing")?;
    let route_plan: Execution = serde_json::from_value(
        route
            .2
            .outputs
            .get("execution_plan")
            .ok_or("E_COLLECTION_INPUT: routes judge missing required output execution_plan")?
            .clone(),
    )
    .map_err(|e| format!("E_COLLECTION_INPUT: unit plan: {e}"))?;
    route_plan.validate()?;
    let filemap = parsed
        .get("filemap")
        .ok_or("E_COLLECTION_INPUT: filemap judge missing")?;
    if filemap.2.outputs.get("impact") != Some(&req.impact) || route.0.impact != req.impact {
        return Err("E_COLLECTION_INPUT: original global provenance differs".into());
    }
    let workflow = parsed
        .get("workflow")
        .ok_or("E_COLLECTION_INPUT: workflow judge missing")?;
    let verdict = workflow
        .2
        .outputs
        .get("workflow")
        .ok_or("E_COLLECTION_INPUT: contribution verdict missing")?;
    if verdict["mode"] != "contribution"
        || verdict["unit"] != unit
        || !verdict["integration"].is_null()
        || verdict["completion"] != "pending collection"
    {
        return Err("E_COLLECTION_INPUT: original contribution verdict differs".into());
    }
    let projects = parsed
        .get("projects")
        .ok_or("E_COLLECTION_INPUT: projects judge missing")?;
    let original_tests = projects
        .2
        .outputs
        .get("tests")
        .ok_or("E_COLLECTION_INPUT: projects judge missing required output tests")?;
    if route_plan.binding != chrono_judge_routes::binding(&route.0, &effective)
        || route.2.outputs.get("effective_inputs") != Some(&effective)
        || route.2.outputs.get("global_selected") != Some(&json!(selected))
        || report["effective_inputs"] != effective
        || &report["tests"] != original_tests
    {
        return Err("E_COLLECTION_INPUT: original plan/input/global binding".into());
    }
    validate_declared_tools(&route_plan, registrations, &effective, declaration_root)?;
    let owned = chrono_harness::units::select(units, selected, unit)?;
    let plans = chrono_judge_registration::execution::plans(registrations.filemap())?;
    let methods = chrono_judge_registration::execution::methods(registrations.projects())?;
    let actions = chrono_judge_routes::execute_actions(registrations);
    let (global_chosen, global_operations) =
        chrono_judge_routes::order(selected, &plans, &methods, &actions)?;
    if route.2.outputs.get("global_graph")
        != Some(
            &json!({"schema":"chrono-global-operation-graph/v1","selected":global_chosen,"operations":global_operations}),
        )
    {
        return Err("E_COLLECTION_INPUT: original global graph differs".into());
    }
    let (expected_selected, mut expected_operations) =
        chrono_judge_routes::order(&owned, &plans, &methods, &actions)?;
    for operation in &mut expected_operations {
        operation.method.argv =
            chrono_judge_routes::expand(&operation.method.argv, &route_plan.binding);
    }
    if route_plan.scheduling
        != chrono_judge_registration::execution::scheduling(
            registrations.filemap(),
            registrations.projects(),
            &registrations.config()["artifacts"],
        )?
        || route_plan.selected != expected_selected
        || serde_json::to_value(&route_plan.operations).unwrap()
            != serde_json::to_value(&expected_operations).unwrap()
    {
        return Err(format!(
            "E_COLLECTION_INPUT: unit plan differs from declarations: {unit}"
        ));
    }
    if route_plan.root != route.0.candidate.root
        || route_plan.environment
            != serde_json::from_value(route.0.observations["environment"]["effective"].clone())
                .map_err(|e| e.to_string())?
    {
        return Err(format!(
            "E_COLLECTION_INPUT: unit plan binding differs: {unit}"
        ));
    }
    let results: Results = serde_json::from_value(original_tests.clone())
        .map_err(|e| format!("E_COLLECTION_INPUT: unit results: {e}"))?;
    if results.plan != route_plan.identity
        || results.selected != owned.iter().cloned().collect::<Vec<_>>()
        || results.scope != Some(format!("unit:{unit}"))
        || results.completion.is_some()
        || results.blocked.len() > 0
        || results.tests.keys().cloned().collect::<BTreeSet<_>>() != owned
        || results.tests.values().any(|s| s != "passed")
        || results.executed.len() != route_plan.operations.len()
    {
        return Err(format!(
            "E_COLLECTION_INPUT: unit contribution is incomplete: {unit}"
        ));
    }
    let requests = chrono_judge_registration::retirement_requests(registrations)?;
    let expected_removed: BTreeMap<String, Option<String>> = req.impact["retired_tests"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|test| {
            let id = test.as_str().ok_or("removed node")?;
            Ok((
                id.to_owned(),
                requests.get(id).ok_or("removed declaration")?.clone(),
            ))
        })
        .collect::<Result<_, String>>()?;
    if results.removed != expected_removed {
        return Err("E_COLLECTION_INPUT: original removal obligations differ".into());
    }
    for operation in &route_plan.operations {
        let rows: Vec<_> = results
            .executed
            .iter()
            .filter(|row| row.operation == operation.method.operation)
            .collect();
        if rows.len() != 1
            || rows[0].status != "passed"
            || rows[0].error.is_some()
            || chrono_judge_routes::compare(&route_plan, operation, rows[0].receipt.as_ref())
                .is_err()
            || rows[0]
                .receipt
                .as_ref()
                .is_none_or(|r| chrono_judge_routes::process_success(&r.process).is_err())
        {
            return Err(format!(
                "E_COLLECTION_INPUT: receipt differs: {unit}/{}",
                operation.method.operation
            ));
        }
    }
    let evidence = &report["unit_evidence"];
    let all_tests: BTreeSet<_> = units
        .values()
        .flat_map(|u| u.tests.iter().cloned())
        .collect();
    let assigned_elsewhere: BTreeSet<_> = selected.difference(&owned).cloned().collect();
    let not_required: BTreeSet<_> = all_tests.difference(selected).cloned().collect();
    let list =
        |set: &BTreeSet<String>| Value::Array(set.iter().cloned().map(Value::String).collect());
    if evidence["global_selected"] != list(selected)
        || evidence["own"] != list(&owned)
        || evidence["assigned_elsewhere"] != list(&assigned_elsewhere)
        || evidence["required_units"] != serde_json::to_value(required).unwrap()
        || evidence["not_required"] != list(&not_required)
        || evidence["acceptance"] != "contribution-only"
    {
        return Err(format!(
            "E_COLLECTION_INPUT: unit obligation partition differs: {unit}"
        ));
    }
    if global_plan
        .selected
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>()
        != *selected
    {
        return Err("E_COLLECTION_INPUT: current global selection differs".into());
    }
    Ok(results)
}

/// Validate the transported original context and declared input closure for
/// both contributions and their finalized collector proof.
pub fn validate_retained_request(
    req: &Request,
    old: &Registrations,
    r: &Registrations,
    artifacts: &Value,
) -> Result<Value, String> {
    validate_retained_request_at(req, old, r, artifacts, &req.candidate.root, None)
}
pub fn validate_retained_request_at(
    req: &Request,
    old: &Registrations,
    r: &Registrations,
    artifacts: &Value,
    root: &Path,
    transport: Option<&chrono_harness::prepared::ArtifactTransport>,
) -> Result<Value, String> {
    req.validate()?;
    let ctx = chrono_harness::json(&chrono_harness::retained_artifacts::bytes(
        root,
        artifacts,
        req.context.path.to_str().ok_or("context address")?,
        transport,
    )?)?;
    if wire::digest(&ctx)? != req.context.sha256
        || ctx["base"] != req.base.commit
        || ctx["candidate"] != req.candidate.commit
    {
        return Err("E_COLLECTION_INPUT: original context bytes differ".into());
    }
    if let Some(path) = ctx["retained_inputs"].as_str() {
        if chrono_harness::json(&chrono_harness::retained_artifacts::bytes(
            root, artifacts, path, transport,
        )?)? != req.observations["retained"]
        {
            return Err("E_COLLECTION_INPUT: original retained snapshot differs".into());
        }
    }
    if r.config()["schema_version"] == 4 {
        let binding = &req.observations["preparation"];
        let prepared: chrono_harness::prepared::PreparedCheck =
            serde_json::from_value(binding["result"].clone())
                .map_err(|e| format!("original preparation missing: {e}"))?;
        if prepared.base.as_deref() != Some(req.base.commit.as_str())
            || prepared.candidate != req.candidate.commit
            || prepared.scope != req.scope
            || prepared
                .context
                .as_ref()
                .is_none_or(|c| c.semantic_digest != req.context.sha256)
        {
            return Err("original preparation differs from full request".into());
        }
    }
    validate_original_entry(req, r)?;
    chrono_judge_registration::inputs::validate_retained_at(req, old, r, artifacts, root, transport)
}
fn validate_original_entry(req: &Request, r: &Registrations) -> Result<(), String> {
    let mut expected: Vec<String> =
        serde_json::from_value(r.config()["canonical_check"]["argv"].clone())
            .map_err(|e| e.to_string())?;
    expected.extend(
        req.scope
            .as_ref()
            .ok_or("original unit scope")?
            .contract_argv(r.config()),
    );
    chrono_harness::validate_invocation_observation(
        &req.observations["entry"],
        &expected,
        &json!({"base":req.base.commit,"candidate":req.candidate.commit}),
    )
    .map_err(|error| format!("E_COLLECTION_INPUT: original entry: {error}"))?;
    let runner = req.candidate.root.join(
        r.config()["runner"]["path"]
            .as_str()
            .ok_or("runner declaration")?,
    );
    let entry = &req.observations["entry"];
    if entry["resolved_paths"]["paths"][0]["actual"] != json!(runner) {
        return Err("E_COLLECTION_INPUT: original runner invocation differs".into());
    }
    // The system may retain a host-root alias in current_exe. Keep that original
    // coordinate, and reuse the live entry resolution without reading old roots.
    if req.runner.path != runner.to_string_lossy() {
        let cwd = entry["cwd"].as_str().ok_or("original entry cwd")?;
        let program = entry["argv"][0].as_str().ok_or("original entry program")?;
        if Path::new(cwd).join(program) != Path::new(&req.runner.path) {
            return Err("E_COLLECTION_INPUT: original runner invocation differs".into());
        }
    }
    Ok(())
}
fn validate_declared_tools(
    plan: &Execution,
    r: &Registrations,
    effective: &Value,
    current_root: &Path,
) -> Result<(), String> {
    let expected: BTreeSet<_> = plan
        .operations
        .iter()
        .map(|op| op.method.tool.clone())
        .collect();
    if plan.tools.keys().cloned().collect::<BTreeSet<_>>() != expected {
        return Err("E_COLLECTION_INPUT: original tool set".into());
    }
    for (id, tool) in &plan.tools {
        let declarations: Vec<_> = r.config()["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|t| t["id"] == *id)
            .collect();
        if declarations.len() != 1 {
            return Err("E_COLLECTION_INPUT: tool declaration".into());
        }
        let d = declarations[0];
        let program = d["program"].as_str().ok_or("tool program")?;
        let argv: Vec<String> =
            serde_json::from_value(d["version_argv"].clone()).map_err(|e| e.to_string())?;
        chrono_judge_routes::validate_retained_tool(
            tool,
            program,
            &argv,
            &plan.environment,
            &plan.root,
            d["expected_version"].as_str(),
        )?;
        let declared_path = Path::new(program);
        if (declared_path.is_absolute() || program.contains('/'))
            && plan.root.join(declared_path) != tool.path
        {
            return Err("E_COLLECTION_INPUT: original declared tool path".into());
        }
        let input = r.config()["environment"]["inputs"].as_array().unwrap().iter().any(|i| {
            plan.root.join(i["location"].as_str().unwrap()) == tool.path && i["sha256"] == tool.sha256
                && effective["endpoints"]["candidate"]["files"][i["id"].as_str().unwrap()]["sha256"] == tool.sha256
        });
        let registered = tool
            .path
            .strip_prefix(&plan.root)
            .ok()
            .and_then(|p| p.to_str())
            .is_some_and(|p| {
                r.filemap()["files"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|f| f["path"] == p)
            });
        if !input && registered {
            let relative = tool
                .path
                .strip_prefix(&plan.root)
                .map_err(|e| e.to_string())?;
            let current = chrono_harness::no_symlink_parents(
                current_root,
                relative.to_str().ok_or("registered tool path")?,
            )?;
            if chrono_harness::sha256(&fs::read(current).map_err(|e| e.to_string())?) != tool.sha256
            {
                return Err("E_COLLECTION_INPUT: candidate tool bytes differ".into());
            }
        }
        if !input && !registered {
            return Err("E_COLLECTION_INPUT: original tool input is undeclared".into());
        }
    }
    Ok(())
}
fn unit_tools(report: &Value) -> Result<Value, String> {
    let route = report["judges"]
        .as_array()
        .ok_or("judges")?
        .iter()
        .find(|r| r["id"] == "routes")
        .ok_or("routes")?;
    let plan: Execution =
        serde_json::from_value(route["response"]["outputs"]["execution_plan"].clone())
            .map_err(|e| e.to_string())?;
    let mut tools = serde_json::Map::new();
    for (id, t) in &plan.tools {
        let path = t
            .path
            .strip_prefix(&plan.root)
            .map(|p| format!("host:{}", p.display()))
            .unwrap_or_else(|_| format!("external:{}", t.path.display()));
        tools.insert(id.clone(),json!({"program":t.program,"path":path,"sha256":t.sha256,"version_argv":&t.version.argv[1..],"version_stdout_sha256":t.version.stdout_sha256,"version_stderr_sha256":t.version.stderr_sha256}));
    }
    Ok(Value::Object(tools))
}
/// Revalidate the original source closure for every complete consumer, without historical execution.
pub fn verify_completion(
    req: &Request,
    r: &Registrations,
    old: &Registrations,
    effective: &Value,
    plan: &Execution,
    results: &Results,
    artifacts: Option<&Value>,
    declaration_root: &Path,
) -> Result<(), String> {
    let completion = results
        .completion
        .as_ref()
        .ok_or("E_COLLECTION_INPUT: missing completion")?;
    let units: BTreeMap<String, chrono_harness::units::Unit> =
        serde_json::from_value(req.observations["execution_units"]["units"].clone())
            .map_err(|e| e.to_string())?;
    let selected: BTreeSet<_> = plan.selected.keys().cloned().collect();
    let required = chrono_harness::units::required(&units, &selected);
    let mut seen = BTreeSet::new();
    let mut tests = BTreeMap::new();
    let mut removed = BTreeMap::<String, Option<String>>::new();
    let mut test_sources = BTreeMap::<String, Vec<String>>::new();
    let mut removal_sources = BTreeMap::<String, Vec<String>>::new();
    let mut tools = serde_json::Map::new();
    let mut warnings = vec![];
    if req.scope
        != Some(chrono_harness::units::Scope::Collect {
            manifest: completion.source.clone(),
        })
    {
        return Err("E_COLLECTION_INPUT: completed collection scope differs".into());
    }
    let manifest_bound = r.config()["execution_units"]["collection_limits"]["manifest_bytes"]
        .as_u64()
        .ok_or("manifest bound")?;
    let manifest_bytes = if let Some(artifacts) = artifacts {
        chrono_harness::retained_artifacts::bytes(
            declaration_root,
            artifacts,
            &completion.source,
            None,
        )?
    } else {
        let path = chrono_harness::no_symlink_parents(&req.candidate.root, &completion.source)?;
        let bound = r.config()["execution_units"]["collection_limits"]["manifest_bytes"]
            .as_u64()
            .ok_or("manifest bound")?;
        if fs::metadata(&path).map_err(|e| e.to_string())?.len() > bound {
            return Err("manifest bound".into());
        }
        fs::read(path).map_err(|e| e.to_string())?
    };
    if manifest_bytes.len() as u64 > manifest_bound
        || chrono_harness::sha256(&manifest_bytes) != completion.source_sha256
    {
        return Err("E_COLLECTION_INPUT: completed manifest bytes/bound differ".into());
    }
    let manifest: chrono_harness::units::FullManifest = chrono_harness::decode(&manifest_bytes)?;
    if !matches!(
        manifest.schema.as_str(),
        "chrono-full-collection/v1" | "chrono-ci-collection/v1"
    ) {
        return Err("E_COLLECTION_INPUT: completed manifest schema differs".into());
    }
    if manifest.reports.len() != completion.reports.len()
        || manifest
            .reports
            .iter()
            .zip(&completion.reports)
            .any(|(a, b)| b["unit"] != a.unit || b["path"] != a.path || b["sha256"] != a.sha256)
    {
        return Err("E_COLLECTION_INPUT: completion manifest sources differ".into());
    }
    if manifest
        .reports
        .iter()
        .zip(&completion.reports)
        .any(|(a, b)| {
            a.runner_sha256
                .as_deref()
                .is_some_and(|pin| pin != req.runner.sha256)
                || a.judge_sha256
                    .as_deref()
                    .is_some_and(|pin| Some(pin) != b["judge_sha256"].as_str())
        })
    {
        return Err("E_COLLECTION_INPUT: completed manifest executable pin differs".into());
    }
    let limit = r.config()["execution_units"]["collection_limits"]["report_bytes"]
        .as_u64()
        .ok_or("report limit")?;
    for source in &completion.reports {
        let unit = source["unit"].as_str().ok_or("unit")?;
        let address = source["retained_path"]
            .as_str()
            .ok_or("original source address")?;
        if !units.contains_key(unit)
            || !seen.insert(unit.to_owned())
            || source["path"]
                != chrono_harness::prepared::original_path(
                    &chrono_harness::prepared::Original {
                        path: units[unit].report_path.clone(),
                        sha256: source["sha256"].as_str().ok_or("source digest")?.into(),
                    },
                    serde_json::from_value::<Option<chrono_harness::prepared::ArtifactTransport>>(
                        source["artifacts"].clone(),
                    )
                    .map_err(|e| e.to_string())?
                    .as_ref(),
                )?
        {
            return Err("E_COLLECTION_INPUT: completion source set".into());
        }
        let bytes = if let Some(artifacts) = artifacts {
            chrono_harness::retained_artifacts::bytes(declaration_root, artifacts, address, None)?
        } else {
            let p = chrono_harness::no_symlink_parents(&req.candidate.root, address)?;
            if fs::metadata(&p).map_err(|e| e.to_string())?.len() > limit {
                return Err("original report bound".into());
            }
            fs::read(p).map_err(|e| e.to_string())?
        };
        if bytes.len() as u64 > limit
            || source["sha256"] != chrono_harness::sha256(&bytes)
            || address
                != format!(
                    ".chrono-harness/state/import-{}.json",
                    source["sha256"].as_str().ok_or("original digest")?
                )
        {
            return Err("E_COLLECTION_INPUT: original source bytes/address".into());
        }
        let report = chrono_harness::json(&bytes)?;
        if source["runner_sha256"] != req.runner.sha256
            || source["judge_sha256"] != report["judge_sha256"]
            || source["status"] != report["status"]
            || source["request_digest"] != report["request_digest"]
        {
            return Err("E_COLLECTION_INPUT: completion source projection differs".into());
        }
        for finding in report["findings"].as_array().ok_or("unit findings")? {
            if finding["level"] == "warning" {
                warnings.push(json!({"unit":unit,"finding":finding}));
            }
        }
        let verified = validate_unit_report(
            req,
            r,
            &units,
            &selected,
            &required,
            unit,
            &report,
            plan,
            old,
            effective,
            declaration_root,
            None,
            Some(&chrono_harness::retained_artifacts::resolve_map(
                source.get("retained_artifacts").unwrap_or(&json!({})),
                artifacts,
            )?),
        )?;
        for (test, status) in verified.tests {
            tests.insert(test.clone(), status);
            test_sources.entry(test).or_default().push(unit.to_owned());
        }
        for (test, replacement) in verified.removed {
            removed.insert(test.clone(), replacement);
            removal_sources
                .entry(test)
                .or_default()
                .push(unit.to_owned());
        }
        for (id, t) in unit_tools(&report)?.as_object().unwrap() {
            if tools
                .insert(id.clone(), t.clone())
                .is_some_and(|previous| previous != *t)
            {
                return Err("original tool conflict".into());
            }
        }
    }
    let requests = chrono_judge_registration::retirement_requests(r)?;
    for retired in req.impact["retired_tests"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        let expected = requests.get(retired).ok_or("retirement declaration")?;
        if removed.get(retired).is_some_and(|r| r != expected) {
            return Err("removal source conflict".into());
        }
        removed.insert(retired.to_owned(), expected.clone());
        removal_sources
            .entry(retired.to_owned())
            .or_default()
            .push("global-obligation".into());
    }
    if completion.schema != "chrono-collected-completion/v1"
        || completion.status != "passed"
        || results.scope.as_deref() != Some("collect")
        || results.plan != plan.identity
        || !results.executed.is_empty()
        || !results.blocked.is_empty()
        || results.selected != selected.iter().cloned().collect::<Vec<_>>()
        || results.removed != removed
        || completion.warnings != warnings
        || completion.test_sources != test_sources
        || completion.removal_sources != removal_sources
        || required.iter().any(|unit| !seen.contains(unit))
        || completion.required_units != required
        || completion.global_selected != selected.iter().cloned().collect::<Vec<_>>()
        || results.tests != tests
        || completion.tools != Value::Object(tools)
    {
        return Err("E_COLLECTION_INPUT: completed original coverage/tools differ".into());
    }
    Ok(())
}

fn validate_retained_response(req: &Request, response: &Response, exit: i32) -> Result<(), String> {
    if response.protocol != wire::PROTOCOL
        || response.request_id != req.request_id
        || response.judge_id != req.judge_id
        || response.status.exit_code() != exit
    {
        return Err("E_COLLECTION_INPUT: response identity/status/exit mismatch".into());
    }
    for finding in &response.findings {
        if finding.code.is_empty()
            || finding.message.is_empty()
            || !matches!(finding.level.as_str(), "info" | "warning" | "error")
            || finding.delta_refs.is_empty()
            || finding
                .delta_refs
                .iter()
                .any(|p| !p.starts_with('/') && !req.delta.iter().any(|d| d.path == *p))
        {
            return Err("E_COLLECTION_INPUT: invalid retained finding".into());
        }
    }
    let errors = response.findings.iter().any(|f| f.level == "error");
    let warnings = response.findings.iter().any(|f| f.level == "warning");
    if (matches!(response.status, Status::Fail | Status::Error) != errors)
        || (response.status == Status::Pass && warnings)
        || (response.status == Status::Warn && !warnings)
    {
        return Err("E_COLLECTION_INPUT: retained response status/findings mismatch".into());
    }
    for evidence in &response.evidence {
        if !evidence.path.starts_with(".chrono-harness/state/")
            || !wire::is_digest(&evidence.sha256)
            || evidence.kind.is_empty()
        {
            return Err("E_COLLECTION_INPUT: invalid retained evidence identity".into());
        }
    }
    Ok(())
}

pub fn judge(req: &Request) -> Response {
    let mut response = req.response(Status::Pass);
    let reader = facts::Reader::for_request(req);
    let result = reader
        .as_ref()
        .map_err(Clone::clone)
        .and_then(|reader| evaluate(req, &mut response, reader));
    if let Ok(reader) = &reader {
        reader.record(&mut response);
    }
    match result {
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
fn evaluate(req: &Request, response: &mut Response, reader: &facts::Reader) -> Result<(), String> {
    req.validate()?;
    let (old, r, _) = chrono_judge_registration::views_with_reader(req, reader)?;
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
    if matches!(
        req.scope,
        Some(chrono_harness::units::Scope::Collect { .. })
    ) {
        let (results, completion) = collect_reports(
            req,
            &plan,
            &plan.selected.keys().cloned().collect(),
            &r,
            &old,
            &inputs,
        )?;
        if !completion.warnings.is_empty() {
            response.status = Status::Warn;
            for source in &completion.warnings {
                response.findings.push(
                    serde_json::from_value(source["finding"].clone()).map_err(|e| e.to_string())?,
                );
            }
        }
        response.evidence.push(wire::Evidence {
            path: completion.source.clone(),
            sha256: completion.source_sha256.clone(),
            kind: "full collection manifest".into(),
        });
        response
            .outputs
            .insert("tests".into(), serde_json::to_value(&results).unwrap());
        response.outputs.insert(
            "collected_completion".into(),
            serde_json::to_value(completion).unwrap(),
        );
        response.outputs.insert(
            "execution_boundary".into(),
            json!({
                "input_completeness_proven":false,
                "retirement_certification":"unresolved: workflow-owned",
                "collection":"contribution evidence verified; no business operations executed"
            }),
        );
        return Ok(());
    }
    snapshot_with_reader(
        &req.candidate.root,
        &req.candidate.commit,
        &artifacts,
        reader,
    )?;
    let mut results = execute(&plan)?;
    if let Some(chrono_harness::units::Scope::Unit { unit }) = &req.scope {
        results.scope = Some(format!("unit:{unit}"));
    }
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
                    // A unit contributes only its owned plans.  A replacement
                    // assigned to another required unit remains an explicit
                    // obligation in `removed`; collection adjudicates the
                    // complete transition once every unit is present.
                    let assigned_elsewhere =
                        if let Some(chrono_harness::units::Scope::Unit { unit }) = &req.scope {
                            req.observations["execution_units"]["units"]
                                .as_object()
                                .into_iter()
                                .flat_map(|units| units.iter())
                                .any(|(owner, definition)| {
                                    owner != unit
                                        && definition["tests"]
                                            .as_array()
                                            .into_iter()
                                            .flatten()
                                            .any(|test| test.as_str() == Some(replacement))
                                })
                        } else {
                            false
                        };
                    if assigned_elsewhere {
                        continue;
                    }
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
    snapshot_with_reader(
        &req.candidate.root,
        &req.candidate.commit,
        &artifacts,
        reader,
    )?;
    chrono_judge_registration::inputs::validate(req, &old, &r)
        .map_err(|e| format!("E_EVIDENCE_UNRESOLVED: post-execution {e}"))?;
    if !results.passed() {
        return Err("E_EXECUTION: selected operations/tests failed or blocked".into());
    }
    response.outputs.insert("execution_boundary".into(),json!({"input_completeness_proven":false,"retirement_certification":"unresolved: workflow-owned"}));
    Ok(())
}
