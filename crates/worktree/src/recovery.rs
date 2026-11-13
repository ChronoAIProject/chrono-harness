//! Immutable pre-effect identity; an intent never certifies completion or a stopped process.
use chrono_harness::{decode, facts, json, no_symlink_parents, relative_path, sha256};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json as value};
use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

/// A compact outcome points to the already published complete diagnostic.
/// Keeping the error body again in inventory would duplicate the original and
/// could prevent publication of a legitimate large failure.
pub(crate) fn publication_outcome(report: &Value, raw: &[u8]) -> Value {
    let mut outcome = value!({"operation":report["operation"],"status":report["status"]});
    if !report["error"].is_null() {
        outcome["error_original"] = value!({"path":report["report_path"],
            "sha256":sha256(raw),"pointer":"/error"});
    }
    outcome
}

fn state_path(root: &Path, path: &str) -> Result<PathBuf, String> {
    relative_path(path)?;
    if !path.starts_with(".chrono-harness/state/") {
        return Err("maintenance inputs must be under .chrono-harness/state".into());
    }
    no_symlink_parents(root, path)
}
pub(crate) fn state_bytes(root: &Path, path: &str) -> Result<Vec<u8>, String> {
    let _input = chrono_harness::retained_artifacts::retention::read_guard(root, path)?;
    fs::read(state_path(root, path)?).map_err(|e| e.to_string())
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RetainedInput {
    schema: String,
    source_root: PathBuf,
    path: String,
    sha256: String,
    byte_length: usize,
    format: String,
}
impl RetainedInput {
    fn bytes(&self, root: &Path) -> Result<Vec<u8>, String> {
        if self.schema != "chrono-worktree-retained-input/v1"
            || self.source_root != root
            || !matches!(self.format.as_str(), "bytes" | "receipt")
        {
            return Err("invalid retained lifecycle input binding".into());
        }
        let bytes = state_bytes(root, &self.path)?;
        if bytes.len() != self.byte_length || sha256(&bytes) != self.sha256 {
            return Err("original lifecycle receipt changed: retained input mismatch".into());
        }
        Ok(bytes)
    }
}

pub(crate) fn retained_bytes(root: &Path, input: &Value) -> Result<Vec<u8>, String> {
    let input: RetainedInput = serde_json::from_value(input.clone())
        .map_err(|e| format!("invalid retained lifecycle input: {e}"))?;
    input.bytes(root)
}

pub(crate) fn retained_input(root: &Path, path: &str, bytes: &[u8], format: &str) -> Value {
    value!(RetainedInput {
        schema: "chrono-worktree-retained-input/v1".into(),
        source_root: root.to_owned(),
        path: path.into(),
        sha256: sha256(bytes),
        byte_length: bytes.len(),
        format: format.into(),
    })
}

/// Follow only the reference-bearing fields of the lifecycle contract.
fn collect_inputs(report: &Value, inputs: &mut Vec<RetainedInput>) -> Result<(), String> {
    if report
        .get("prior_report")
        .is_some_and(|prior| prior.get("report").is_some() || prior.get("input_bytes").is_some())
    {
        return Err("inline lifecycle prior reports are not the current contract".into());
    }
    {
        for pointer in [
            "/terminal_input",
            "/prior_report/input",
            "/prior_cache_attempt/intent_input",
            "/prior_cache_attempt/result/input",
            "/original_report/result/input",
            "/unreferenced_original_receipt/input",
            "/original_result/input",
            "/declaration_input",
            "/age_retirement_input",
            "/prior_age_result/input",
            "/age_reuse/input",
            "/prior_terminal_input",
        ] {
            if let Some(input) = report.pointer(pointer) {
                inputs.push(
                    serde_json::from_value(input.clone()).map_err(|e| {
                        format!("invalid retained lifecycle input at {pointer}: {e}")
                    })?,
                );
            }
        }
        if let Some(rows) = report.get("migration_inputs").and_then(Value::as_array) {
            for row in rows {
                if let Some(input) = row.get("input") {
                    inputs.push(
                        serde_json::from_value(input.clone())
                            .map_err(|e| format!("invalid retained migration input: {e}"))?,
                    );
                }
            }
        }
        if let Some(drain) = report.get("drain").and_then(Value::as_array) {
            for entry in drain {
                if let Some(input) = entry.get("input") {
                    inputs.push(
                        serde_json::from_value(input.clone())
                            .map_err(|e| format!("invalid retained drain input: {e}"))?,
                    );
                }
            }
            if drain.iter().any(|entry| entry.get("report").is_some()) {
                return Err("inline lifecycle drain reports are not the current contract".into());
            }
        }
    }
    Ok(())
}
pub(crate) fn verify_inputs(root: &Path, report: &Value) -> Result<(), String> {
    let mut inputs = Vec::new();
    let mut checked = BTreeSet::new();
    collect_inputs(report, &mut inputs)?;
    while let Some(input) = inputs.pop() {
        if input.schema != "chrono-worktree-retained-input/v1"
            || input.source_root != root
            || !matches!(input.format.as_str(), "bytes" | "receipt")
        {
            return Err("invalid retained lifecycle input binding".into());
        }
        if !checked.insert((
            input.path.clone(),
            input.sha256.clone(),
            input.byte_length,
            input.format.clone(),
        )) {
            continue;
        }
        let bytes = input.bytes(root)?;
        if input.format == "receipt" {
            collect_inputs(&json(&bytes)?, &mut inputs)?;
        }
    }
    Ok(())
}

fn original_bytes(root: &Path, path: &str) -> Result<Option<Vec<u8>>, String> {
    let path = state_path(root, path)?;
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
        Ok(m) if m.is_file() => fs::read(path).map(Some).map_err(|e| e.to_string()),
        Ok(_) => Err("original result is not a regular file".into()),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Intent {
    schema: String,
    #[serde(default)]
    creator_agent: Option<crate::AgentIdentity>,
    operation: String,
    source_root: String,
    source_commit: String,
    config_path: String,
    config_sha256: String,
    registry_digest: String,
    base: String,
    destination: String,
    branch_ref: String,
    lock_reason: String,
    report_path: String,
}

pub(crate) fn publish(root: &Path, report: &mut Value) -> Result<(), String> {
    publish_fields(
        root,
        report,
        "chrono-worktree-recovery-intent/v1",
        ".intent.json",
        "recovery_intent",
        &[
            "operation",
            "source_root",
            "source_commit",
            "config_path",
            "config_sha256",
            "registry_digest",
            "base",
            "destination",
            "branch_ref",
            "lock_reason",
            "report_path",
        ],
    )
}
pub(crate) fn publish_fields(
    root: &Path,
    report: &mut Value,
    schema: &str,
    suffix: &str,
    binding: &str,
    fields: &[&str],
) -> Result<(), String> {
    let mut intent = value!({"schema":schema});
    for &field in fields {
        let text = report[field]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| format!("recovery intent lacks {field}"))?;
        intent[field] = value!(text);
    }
    if schema == "chrono-worktree-recovery-intent/v1"
        && matches!(report["operation"].as_str(), Some("start" | "reconstruct"))
    {
        intent["creator_agent"] = value!(crate::AgentIdentity::from_report(report)?);
    }
    let path = format!("{}{suffix}", intent["report_path"].as_str().unwrap());
    let target = state_path(root, &path)?;
    let mut bytes = serde_json::to_vec_pretty(&intent).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    if chrono_harness::retained_artifacts::retention::publish_owned_original(
        root,
        "lifecycle-original",
        &path,
        &bytes,
        value!({"kind":"recovery-intent","completion":"not-established"}),
    )? {
        if let Some(inventory) =
            chrono_harness::retained_artifacts::retention::Inventory::adopted(root)?
        {
            inventory.reference_paths(
                &format!("recovery:{}", intent["report_path"].as_str().unwrap()),
                &[path.clone(), intent["report_path"].as_str().unwrap().into()],
            )?;
        }
        report[binding] = value!({"path":path,"sha256":sha256(&bytes)});
        return Ok(());
    }
    let mut output = tempfile::Builder::new()
        .prefix("intent-")
        .tempfile_in(target.parent().ok_or("intent directory missing")?)
        .map_err(|e| format!("recovery intent preparation: {e}"))?;
    output
        .write_all(&bytes)
        .map_err(|e| format!("recovery intent write: {e}"))?;
    output
        .as_file()
        .sync_all()
        .map_err(|e| format!("recovery intent sync: {e}"))?;
    output
        .persist_noclobber(&target)
        .map_err(|e| format!("recovery intent publication: {e}"))?;
    report[binding] = value!({"path":path,"sha256":sha256(&bytes)});
    Ok(())
}

#[derive(Deserialize)]
#[serde(tag = "presence", rename_all = "lowercase", deny_unknown_fields)]
pub(crate) enum ExpectedResult {
    Absent {},
    Present { sha256: String },
}
pub(crate) struct Evidence {
    pub(crate) descriptor: Value,
    root: PathBuf,
    intent_path: String,
    intent_bytes: Vec<u8>,
    result_path: String,
    result_bytes: Option<Vec<u8>>,
    _retention: Vec<chrono_harness::ownership::Lease>,
}
impl Evidence {
    pub(crate) fn stable(&self) -> Result<(), String> {
        if state_bytes(&self.root, &self.intent_path)? != self.intent_bytes
            || original_bytes(&self.root, &self.result_path)? != self.result_bytes
        {
            return Err("interrupted-operation evidence changed during recovery".into());
        }
        Ok(())
    }
}

pub(crate) fn inspect(
    root: &Path,
    config_path: &str,
    config_bytes: &[u8],
    path: &str,
    digest: &str,
    expected: &ExpectedResult,
    report: &mut Value,
) -> Result<Evidence, String> {
    let (bytes, descriptor) = read_intent(root, path, digest, report)?;
    let intent: Intent = decode(&bytes)?;
    let config: crate::Config = decode(config_bytes)?;
    if intent.schema != "chrono-worktree-recovery-intent/v1"
        || intent
            .creator_agent
            .as_ref()
            .is_some_and(|agent| !agent.valid())
        || !matches!(
            intent.operation.as_str(),
            "start" | "reconstruct" | "cleanup"
        )
        || intent.source_root != root.to_str().ok_or("source root is not UTF-8")?
        || intent.config_path != config_path
        || intent.config_sha256 != sha256(config_bytes)
        || intent.report_path != format!("{}{}.json", config.report_directory, intent.lock_reason)
        || path != format!("{}.intent.json", intent.report_path)
        || !intent
            .lock_reason
            .starts_with(&format!("{}-", intent.operation))
        || intent.registry_digest.is_empty()
        || intent.branch_ref.is_empty()
        || !Path::new(&intent.destination).is_absolute()
    {
        return Err("interrupted-operation/configuration identity mismatch".into());
    }
    facts::full_oid(&intent.source_commit)?;
    facts::full_oid(&intent.base)?;
    observe_result(
        root,
        path,
        bytes,
        descriptor,
        &intent.report_path,
        expected,
        report,
    )
}

pub(crate) fn read_intent(
    root: &Path,
    path: &str,
    digest: &str,
    report: &mut Value,
) -> Result<(Vec<u8>, Value), String> {
    let bytes = state_bytes(root, path)?;
    if sha256(&bytes) != digest {
        return Err("recovery intent digest mismatch".into());
    }
    let descriptor = json(&bytes)?;
    report["prior_intent"] =
        value!({"path":path,"sha256":digest,"input_bytes":bytes,"intent":descriptor});
    Ok((bytes, descriptor))
}

pub(crate) fn observe_result(
    root: &Path,
    path: &str,
    bytes: Vec<u8>,
    descriptor: Value,
    result_path: &str,
    expected: &ExpectedResult,
    report: &mut Value,
) -> Result<Evidence, String> {
    observe_result_kind(
        root,
        path,
        bytes,
        descriptor,
        result_path,
        expected,
        report,
        false,
    )
}

pub(crate) fn observe_rebind_result(
    root: &Path,
    path: &str,
    bytes: Vec<u8>,
    descriptor: Value,
    result_path: &str,
    expected: &ExpectedResult,
    report: &mut Value,
) -> Result<Evidence, String> {
    observe_result_kind(
        root,
        path,
        bytes,
        descriptor,
        result_path,
        expected,
        report,
        true,
    )
}
fn observe_result_kind(
    root: &Path,
    path: &str,
    bytes: Vec<u8>,
    descriptor: Value,
    result_path: &str,
    expected: &ExpectedResult,
    report: &mut Value,
    allow_failed_rebind: bool,
) -> Result<Evidence, String> {
    let result = original_bytes(root, result_path)?;
    match (expected, &result) {
        (ExpectedResult::Absent {}, None) => (),
        (ExpectedResult::Present { sha256: digest }, Some(bytes)) if sha256(bytes) == *digest => (),
        _ => return Err("original result presence or digest mismatch".into()),
    }
    report["prior_result"] = match &result {
        Some(bytes) => {
            value!({"path":result_path,"presence":"present","sha256":sha256(bytes),"input_bytes":bytes})
        }
        None => value!({"path":result_path,"presence":"absent"}),
    };
    // A retained terminal result is never silently recast as an interrupted attempt.
    if let Some(bytes) = &result {
        if let Ok(original) = json(bytes) {
            if original["schema"] == "chrono-worktree-report/v1"
                && matches!(
                    original["status"].as_str(),
                    Some(
                        "failed"
                            | "created"
                            | "reconstructed"
                            | "recovered"
                            | "cleaned"
                            | "observed"
                            | "rebound"
                    )
                )
            {
                if !allow_failed_rebind
                    || original["status"] != "failed"
                    || original["operation"] != "rebind"
                {
                    return Err("original result declares a terminal outcome; use its ordinary maintenance contract".into());
                }
                for field in [
                    "source_root",
                    "source_commit",
                    "config_path",
                    "config_sha256",
                    "registry_digest",
                    "report_path",
                    "lock_reason",
                    "destination",
                    "branch_ref",
                    "head",
                    "index_tree",
                ] {
                    if original[field] != descriptor[field] {
                        return Err("original failed rebind differs from retained intent".into());
                    }
                }
                report["original_outcome"] = value!("failed");
            }
        }
    }
    if report["original_outcome"] != "failed" {
        report["original_outcome"] = value!("unknown");
    }
    let mut retention = Vec::new();
    for original in [path, result_path] {
        if let Some(guard) =
            chrono_harness::retained_artifacts::retention::read_guard(root, original)?
        {
            retention.push(guard);
        }
    }
    Ok(Evidence {
        _retention: retention,
        descriptor,
        root: root.into(),
        intent_path: path.into(),
        intent_bytes: bytes,
        result_path: result_path.into(),
        result_bytes: result,
    })
}

/// Hand the actual caller its exact root before relinquishing publication.
pub(crate) fn protect_report_consumer(
    report: &mut Value,
    publication: Option<&chrono_harness::retained_artifacts::retention::Publication>,
    consumer: Option<&str>,
) -> Result<(), String> {
    if let Some(consumer) = consumer {
        let original = publication
            .ok_or("report consumer requires adopted lifecycle retention")?
            .protect_consumer(consumer)?;
        report["report_consumer"] = value!({"schema":"chrono-retention-consumer/v1",
            "consumer":consumer,"purpose":"Lifecycle caller consumes producer original and declared receipt closure",
            "originals":[original]});
    }
    Ok(())
}

/// The schema's declared input paths, shared by automatic originals and final
/// reports. Neither directory scans nor recursive JSON guesses define edges.
pub(crate) fn original_paths(root: &Path, report: &Value) -> Result<Vec<String>, String> {
    let mut originals = Vec::new();
    let mut inputs = Vec::new();
    collect_inputs(report, &mut inputs)?;
    for input in inputs {
        if input.source_root != root
            || input.schema != "chrono-worktree-retained-input/v1"
            || !matches!(input.format.as_str(), "bytes" | "receipt")
        {
            return Err("invalid retained lifecycle input binding".into());
        }
        state_path(root, &input.path)?;
        originals.push(input.path);
    }
    for binding in [
        "recovery_intent",
        "fetch_intent",
        "cache_intent",
        "main_cache_release",
        "managed_use_receipt",
        "managed_process_original",
        "intent",
        "terminal_receipt",
    ] {
        if let Some(path) = report[binding]["path"].as_str() {
            originals.push(path.into());
        }
    }
    if report["schema"] == "chrono-worktree-owner-adoption/v1" {
        let path = report["stage_result"]
            .as_str()
            .ok_or("owner adoption stage path")?;
        state_path(root, path)?;
        originals.push(path.into());
    }
    for pointer in [
        "/owner_deployment/receipt/path",
        "/owner_deployment/stage_result",
    ] {
        if let Some(path) = report.pointer(pointer).and_then(Value::as_str) {
            originals.push(path.into());
        }
    }
    if matches!(
        report["schema"].as_str(),
        Some("chrono-worktree-managed-use-result/v1" | "chrono-main-cache-release/v1")
    ) {
        if let Some(path) = report["report_path"].as_str() {
            originals.push(path.into());
        }
    }
    for pass in [report, &report["post_use_cleanup"]] {
        for field in ["drain", "producer_drains"] {
            for item in pass[field].as_array().into_iter().flatten() {
                if let Some(path) = item["receipt"]["path"].as_str() {
                    originals.push(path.into());
                }
            }
        }
    }
    originals.sort();
    originals.dedup();
    Ok(originals)
}

/// The actual result handoff or exclusive lifecycle reconciliation owns this
/// release. It does not manufacture a missing command result.
pub(crate) fn release_managed_process_original(root: &Path, report: &Value) -> Result<(), String> {
    if let Some(path) = report["managed_process_original"]["path"].as_str() {
        state_path(root, path)?;
        if let Some(inventory) =
            chrono_harness::retained_artifacts::retention::Inventory::adopted(root)?
        {
            inventory.reference_paths(&format!("recovery:{path}"), &[])?;
        }
    }
    Ok(())
}

/// Attach recovery inputs before the domain releases a use token or returns
/// an original which its still-unpublished report must consume.
pub(crate) fn bind_report_inputs(root: &Path, report: &Value) -> Result<(), String> {
    if let Some(inventory) =
        chrono_harness::retained_artifacts::retention::Inventory::adopted(root)?
    {
        let path = report["report_path"]
            .as_str()
            .ok_or("lifecycle result path")?;
        inventory.dependencies_paths(path, &original_paths(root, report)?)?;
        bind_result_handoffs(&inventory, root, report)?;
    }
    Ok(())
}

/// The pending result is the ledger's actual reader of its final report.
/// Retention already collects unrooted cycles: neither direction is a root.
fn bind_result_handoffs(
    inventory: &chrono_harness::retained_artifacts::retention::Inventory,
    root: &Path,
    report: &Value,
) -> Result<(), String> {
    for binding in ["managed_use_receipt", "main_cache_release"] {
        if let Some(path) = report[binding]["path"].as_str() {
            let bytes = state_bytes(root, path)?;
            if report[binding]["sha256"] != sha256(&bytes) {
                return Err("pending report handoff original changed".into());
            }
            let original = json(&bytes)?;
            if original["report_path"] != report["report_path"] {
                return Err("pending result names another final report".into());
            }
            inventory.dependencies_paths(path, &original_paths(root, &original)?)?;
        }
    }
    Ok(())
}

/// A producer terminal result ends its interrupted-operation recovery need.
/// A failed rebind has an explicit resume consumer and keeps that root.
pub(crate) fn settle_report(root: &Path, report: &Value) -> Result<(), String> {
    let Some(inventory) = chrono_harness::retained_artifacts::retention::Inventory::adopted(root)?
    else {
        return Ok(());
    };
    let path = report["report_path"]
        .as_str()
        .ok_or("lifecycle result path")?;
    let originals = original_paths(root, report)?;
    inventory.dependencies_paths(path, &originals)?;
    bind_result_handoffs(&inventory, root, report)?;
    if report["managed_process_original"]["joined"] == true {
        release_managed_process_original(root, report)?;
    }
    if report["status"] == "failed" {
        inventory.retain_diagnostic_path(path)?;
    }
    // The joined terminal producer accounts its unfinished-operation role.
    // Failed rebind still has a distinct actual resume consumer.
    if report["operation"] != "rebind" || report["status"] != "failed" {
        inventory.reference_paths(&format!("recovery:{path}"), &[])?;
    }
    Ok(())
}
