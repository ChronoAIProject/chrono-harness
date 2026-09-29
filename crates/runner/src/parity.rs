//! Pairwise local/CI parity evidence.
//!
//! The check runner owns transport, but it cannot infer whether the host has
//! declared every input that can affect a verdict.  This module therefore
//! fails closed: a report may be marked established only when the producer
//! explicitly proves complete effective inputs and both reports have the same
//! verdict-bearing observations.

use serde_json::{Map, Value, json};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const REQUIRED: &[&str] = &[
    "status",
    "base",
    "candidate",
    "candidate_tree",
    "context_digest",
    "registry_digest",
    "environment",
    "executables",
    "tools",
    "effective_inputs",
    "delta",
    "impact",
    "judges",
    "tests",
    "findings",
];

/// Establish parity for two independently produced full reports.
///
/// This is intentionally pairwise evidence.  It does not claim that an
/// undisclosed toolchain or host input is complete; that obligation is why
/// `effective_inputs.completeness_proven` is required and why unknown reports
/// are rejected rather than treated as equal.
pub fn establish(
    current: &Value,
    compared: &Value,
    compared_report: &str,
) -> Result<Value, String> {
    validate_report("current", current)?;
    validate_report("compared", compared)?;
    if compared_report.is_empty() {
        return Err("E_PARITY_UNESTABLISHED: compared report path is empty".into());
    }
    let left = projection(current)?;
    let right = projection(compared)?;
    if left != right {
        let differing = differing_fields(&left, &right);
        return Err(format!(
            "E_PARITY_UNESTABLISHED: verdict-bearing reports differ at {differing:?}"
        ));
    }
    Ok(json!({
        "status": "established",
        "compared_report": compared_report,
        "evidence": {
            "kind": "pairwise-report-v1",
            "complete_inputs": true,
            "verdict_projection": "v1",
            "matched_fields": REQUIRED,
        }
    }))
}

/// Read, compare and publish one report while preserving the retained report
/// named by its `report_path` field.  A failed comparison is still published
/// with `/parity` unresolved and returns `established = false`; callers must
/// propagate that as an error exit.
pub fn update_files(root: &Path, report: &Path, compared: &Path) -> Result<(Value, bool), String> {
    if report.is_absolute() || compared.is_absolute() || report == compared {
        return Err("E_PARITY_INPUT: report paths must be distinct relative paths".into());
    }
    let report_path =
        crate::no_symlink_parents(root, report.to_str().ok_or("non UTF-8 report path")?)?;
    let compared_path = crate::no_symlink_parents(
        root,
        compared.to_str().ok_or("non UTF-8 compared report path")?,
    )?;
    if report_path == compared_path {
        return Err("E_PARITY_INPUT: reports must be distinct".into());
    }
    let mut current =
        crate::json(&fs::read(&report_path).map_err(|e| format!("E_PARITY_INPUT: {e}"))?)?;
    let compared_value =
        crate::json(&fs::read(&compared_path).map_err(|e| format!("E_PARITY_INPUT: {e}"))?)?;
    let compared_name = compared_path
        .strip_prefix(root)
        .ok()
        .and_then(|p| p.to_str())
        .ok_or("compared report is outside host root")?;
    let established = match establish(&current, &compared_value, compared_name) {
        Ok(value) => {
            current["parity"] = value;
            if let Some(unresolved) = current["unresolved"].as_object_mut() {
                unresolved.remove("/parity");
            }
            true
        }
        Err(error) => {
            if !current["unresolved"].is_object() {
                current["unresolved"] = json!({});
            }
            current["unresolved"]["/parity"] = Value::String(error);
            false
        }
    };
    let text = serde_json::to_string_pretty(&current).map_err(|e| e.to_string())? + "\n";
    let mut targets = vec![report_path];
    if let Some(retained) = current.get("report_path").and_then(Value::as_str) {
        let retained = crate::no_symlink_parents(root, retained)?;
        if !targets.contains(&retained) {
            targets.push(retained);
        }
    }
    for target in targets {
        fs::write(target, &text).map_err(|e| e.to_string())?;
    }
    Ok((current, established))
}

fn validate_report(label: &str, report: &Value) -> Result<(), String> {
    if report.get("schema_version") != Some(&Value::from(1))
        || report.get("scope") != Some(&Value::from("configured-judges"))
    {
        return Err(format!(
            "E_PARITY_UNESTABLISHED: {label} is not a full configured-judges report"
        ));
    }
    for field in REQUIRED {
        if report.get(*field).is_none() || report[*field].is_null() {
            return Err(format!(
                "E_PARITY_UNESTABLISHED: {label} has missing or unresolved {field}"
            ));
        }
    }
    match report["status"].as_str() {
        Some("pass" | "warn" | "fail") => {}
        _ => {
            return Err(format!(
                "E_PARITY_UNESTABLISHED: {label} has no completed functional verdict"
            ));
        }
    }
    let complete = report["effective_inputs"]
        .get("completeness_proven")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !complete {
        return Err(format!(
            "E_PARITY_UNESTABLISHED: {label} does not prove complete effective inputs"
        ));
    }
    let unresolved = report
        .get("unresolved")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("E_PARITY_UNESTABLISHED: {label} has no unresolved map"))?;
    let critical = unresolved.keys().filter(|path| {
        matches!(
            path.as_str(),
            "/tools" | "/effective_inputs" | "/impact" | "/tests" | "/judges" | "/findings"
        )
    });
    if critical.into_iter().next().is_some() {
        return Err(format!(
            "E_PARITY_UNESTABLISHED: {label} retains unresolved verdict input"
        ));
    }
    let judges = report["judges"]
        .as_array()
        .ok_or_else(|| format!("E_PARITY_UNESTABLISHED: {label} judges is not an array"))?;
    if judges.is_empty() {
        return Err(format!(
            "E_PARITY_UNESTABLISHED: {label} has no judge observations"
        ));
    }
    for judge in judges {
        if judge["state"] != "executed" || !judge["response"].is_object() {
            return Err(format!(
                "E_PARITY_UNESTABLISHED: {label} contains a non-executed judge"
            ));
        }
    }
    Ok(())
}

fn projection(report: &Value) -> Result<Value, String> {
    let mut out = Map::new();
    for field in REQUIRED {
        let value = if *field == "executables" {
            executable_projection(&report[*field])?
        } else if *field == "judges" {
            judge_projection(&report[*field])?
        } else {
            report[*field].clone()
        };
        out.insert((*field).into(), value);
    }
    Ok(Value::Object(out))
}

fn executable_projection(value: &Value) -> Result<Value, String> {
    let rows = value
        .as_array()
        .ok_or("E_PARITY_UNESTABLISHED: executables is not an array")?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let sha = row
            .get("sha256")
            .and_then(Value::as_str)
            .ok_or("E_PARITY_UNESTABLISHED: executable digest is missing")?;
        if sha.is_empty() {
            return Err("E_PARITY_UNESTABLISHED: executable digest is empty".into());
        }
        // Absolute paths differ between local and CI checkouts.  The digest
        // and observed version are the executable identity; path-sensitive
        // behavior must be declared in effective_inputs.
        out.push(json!({
            "sha256": sha,
            "version": row.get("version").cloned().unwrap_or(Value::Null),
        }));
    }
    Ok(Value::Array(out))
}

fn judge_projection(value: &Value) -> Result<Value, String> {
    let rows = value
        .as_array()
        .ok_or("E_PARITY_UNESTABLISHED: judges is not an array")?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let response = row
            .get("response")
            .and_then(Value::as_object)
            .ok_or("E_PARITY_UNESTABLISHED: judge response is missing")?;
        let mut response_out = Map::new();
        for key in [
            "protocol", "judge_id", "status", "findings", "evidence", "outputs",
        ] {
            let value = response.get(key).ok_or_else(|| {
                format!("E_PARITY_UNESTABLISHED: response field {key} is missing")
            })?;
            response_out.insert(key.into(), value.clone());
        }
        let binding = row
            .get("binding")
            .ok_or("E_PARITY_UNESTABLISHED: judge binding is missing")?;
        out.push(json!({
            "id": row.get("id"),
            "state": row.get("state"),
            "blocked_by": row.get("blocked_by").cloned().unwrap_or(Value::Null),
            "exit_code": row.get("exit_code").cloned().unwrap_or(Value::Null),
            "binding": binding,
            "response": Value::Object(response_out),
            "process": process_projection(row.get("process"))?,
        }));
    }
    Ok(Value::Array(out))
}

fn process_projection(value: Option<&Value>) -> Result<Value, String> {
    let Some(value) = value else {
        return Ok(Value::Null);
    };
    let object = value
        .as_object()
        .ok_or("E_PARITY_UNESTABLISHED: process observation is not an object")?;
    let mut out = Map::new();
    for key in [
        "exit_code",
        "failure",
        "stdout_sha256",
        "stderr_sha256",
        "sha256",
    ] {
        if let Some(v) = object.get(key) {
            out.insert(key.into(), v.clone());
        }
    }
    Ok(Value::Object(out))
}

fn differing_fields(left: &Value, right: &Value) -> Vec<String> {
    let Some(left) = left.as_object() else {
        return vec!["/".into()];
    };
    let Some(right) = right.as_object() else {
        return vec!["/".into()];
    };
    let keys: BTreeSet<_> = left.keys().chain(right.keys()).cloned().collect();
    keys.into_iter()
        .filter(|key| left.get(key) != right.get(key))
        .map(|key| format!("/{key}"))
        .collect()
}
