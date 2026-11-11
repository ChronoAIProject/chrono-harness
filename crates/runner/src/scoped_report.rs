//! Scoped report storage keeps original process streams once, independently of read projections.
use crate::{ProcessResult, prepared, sha256};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;

pub const SCHEMA: &str = "chrono-check-report/v2";
const INLINE_STREAMS: &[&str] = &["stdout", "stderr", "stdout_bytes", "stderr_bytes"];

/// The final original owns the exact remaining publication obligation. It is
/// not another operation ledger and never authorizes judge dispatch.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Publication {
    schema: String,
    slot: String,
    previous_sha256: Option<String>,
    snapshot_failure: Option<String>,
    originals: Vec<String>,
    native_delivery: Option<String>,
    recovery_root: Option<String>,
    exit_code: u8,
}

fn slot_digest(root: &Path, slot: &str) -> Result<Option<String>, String> {
    let path = crate::no_symlink_parents(root, slot)?;
    match std::fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_file() => {
            let raw = crate::units::read_bounded(root, slot, 64 * 1024 * 1024)?;
            Ok(Some(sha256(&raw)))
        }
        // A conflicting directory remains a real write failure. After its
        // owner repairs the destination, recovery still requires an empty slot.
        Ok(_) => Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

pub(crate) fn publication(
    root: &Path,
    slot: &str,
    originals: Vec<String>,
    native_delivery: Option<String>,
    recovery_root: Option<String>,
    exit_code: u8,
) -> (Publication, Option<crate::ownership::Lease>) {
    let (guard, previous_sha256, snapshot_failure) =
        match crate::retained_artifacts::retention::report_slot(root, slot) {
            Ok(guard) => match slot_digest(root, slot) {
                Ok(digest) => (guard, digest, None),
                Err(error) => (guard, None, Some(error)),
            },
            Err(error) => (None, None, Some(error)),
        };
    let publication = Publication {
        schema: "chrono-report-publication/v1".into(),
        slot: slot.into(),
        previous_sha256,
        snapshot_failure,
        originals,
        native_delivery,
        recovery_root,
        exit_code,
    };
    (publication, guard)
}

impl Publication {
    pub(crate) fn failure(&self) -> Option<&str> {
        self.snapshot_failure.as_deref()
    }
}

pub(crate) fn recover(root: &Path, original: &prepared::Original) -> Result<Value, String> {
    use crate::retained_artifacts::retention;
    use crate::verified_release as policy;
    let _activity = retention::read_scope(root)?;
    let inventory = retention::Inventory::adopted(root)?
        .ok_or("report recovery requires the original retained producer")?;
    let _reader = retention::read_guard(root, &original.path)?
        .ok_or("original report is unavailable or not enrolled; preserve recovery")?;
    let raw = prepared::read_original(root, original, None)?;
    if !inventory.published_original(&original.path, &original.sha256, raw.len() as u64)? {
        return Err(
            "original report is not sealed or its physical identity changed; preserve recovery"
                .into(),
        );
    }
    let report: Value = crate::json(&raw)?;
    let publication: Publication = serde_json::from_value(report["publication"].clone())
        .map_err(|e| format!("original report publication obligation: {e}"))?;
    if !policy::report_publication(&publication.schema, publication.exit_code) {
        return Err("original report publication schema/exit code".into());
    }
    let kind = policy::recovered_report_kind(
        report["schema"].as_str(),
        report["schema_version"].as_u64(),
        report["retained_report"].as_str(),
        report["report_path"].as_str(),
        &original.path,
    );
    let (producer, request_id) = match kind {
        1 | 2 => {
            let request: crate::Request =
                serde_json::from_value(report["request"].clone()).map_err(|e| e.to_string())?;
            if request.host_root != root {
                return Err("original report belongs to another host".into());
            }
            if report["judge"].is_object() {
                let process: crate::ProcessResult = if kind == 2 {
                    restore_process(&report["judge"], |o| prepared::read_original(root, o, None))?
                } else {
                    serde_json::from_value(report["judge"].clone()).map_err(|e| e.to_string())?
                };
                crate::full::validate_process_receipt(&process, "recovered judge")?;
                if report.get("transport_failure").is_none() {
                    let response: crate::Response = crate::decode(&process.stdout_bytes)?;
                    crate::validate_response_protocol(
                        &response,
                        &request.request_id,
                        process.exit_code,
                        &request.protocol,
                    )?;
                }
            }
            ("check-original", request.request_id)
        }
        3 => {
            let request: crate::wire::Request =
                serde_json::from_value(report["request"].clone()).map_err(|e| e.to_string())?;
            if request.candidate.root != root {
                return Err("original report belongs to another host".into());
            }
            let records = report["judges"]
                .as_array()
                .ok_or("original full judges")?
                .iter()
                .map(crate::full::expand_record)
                .collect::<Result<Vec<_>, _>>()?;
            let bindings = records
                .iter()
                .map(|row| {
                    serde_json::from_value(row["binding"].clone()).map_err(|e| e.to_string())
                })
                .collect::<Result<Vec<_>, _>>()?;
            let status: crate::wire::Status =
                serde_json::from_value(report["status"].clone()).map_err(|e| e.to_string())?;
            crate::full::validate_execution(&request, &bindings, &status, &records)?;
            // The retained request is the first judge's sealed input. The
            // producer seals the enclosing operation, which has its own ID.
            let operation_request_id = report["operation_request_id"]
                .as_str()
                .ok_or("original full operation request identity")?;
            ("full-original", operation_request_id.to_owned())
        }
        _ => return Err("original report format/address is not recoverable".into()),
    };
    let status = inventory.producer_status(producer, Some(&original.path))?;
    let records = status["records"]
        .as_array()
        .ok_or("original report producer status")?;
    let [record] = records.as_slice() else {
        return Err("original report has no unique matching producer".into());
    };
    if !policy::report_outcome(
        records.len(),
        record["outcome"]["request_id"].as_str(),
        record["outcome"]["exit_code"].as_u64(),
        &request_id,
        publication.exit_code,
    ) {
        return Err("original report outcome differs from sealed producer".into());
    }
    let _slot = retention::report_slot(root, &publication.slot)?;
    let bytes = if kind == 2 {
        serde_json::to_vec(&crate::units::ReportReference {
            schema: crate::units::REPORT_REFERENCE.into(),
            original: original.clone(),
        })
        .map_err(|e| e.to_string())?
    } else {
        raw.clone()
    };
    let current = slot_digest(root, &publication.slot)?;
    let plan = policy::report_slot(
        current.as_deref(),
        publication.previous_sha256.as_deref(),
        &sha256(&bytes),
    );
    let already_published = plan.already_published;
    if !plan.allowed {
        return Err(
            "report slot changed after this original; preserve the newer report and recovery"
                .into(),
        );
    }
    retention::reference_report_held(
        root,
        &publication.slot,
        &original.path,
        &publication.originals,
        if already_published {
            None
        } else {
            publication.native_delivery.as_deref()
        },
    )?;
    if !already_published {
        retention::replace_report(&crate::no_symlink_parents(root, &publication.slot)?, &bytes)
            .map_err(|e| e.to_string())?;
    } else {
        retention::sync_report(&crate::no_symlink_parents(root, &publication.slot)?)
            .map_err(|e| e.to_string())?;
    }
    retention::settle_report(root, &publication.slot, &original.path)?;
    if let Some(role) = &publication.recovery_root {
        inventory.reference_paths(role, &[])?;
    }
    Ok(
        json!({"schema":"chrono-report-recovery/v1","status":"recovered",
        "report":original,"slot":publication.slot,"request_id":request_id,
        "original_exit_code":publication.exit_code,"already_published":already_published,
        "judge_dispatch":"not-performed"}),
    )
}

fn publication_error(
    process: &ProcessResult,
    stream: &str,
    error: &str,
    stdout: Option<&prepared::Original>,
    paths: Option<(&str, &str, &str)>,
) -> String {
    let original = json!(process);
    // Even an inconsistent receipt must survive a secondary codec failure.
    let evidence = crate::full::compact_process(&original).unwrap_or(original);
    format!(
        "E_PROCESS_EVIDENCE: {}",
        json!({"message":format!("cannot retain judge {stream} (exit {}, failure {:?}): {error}",
            process.exit_code, process.failure),"process":evidence,"stdout_original":stdout,
            "evidence_paths":paths.map(|(stdout, stderr, launch)| json!({"stdout":stdout,"stderr":stderr,"launch":launch}))})
    )
}

pub fn retain_process(
    root: &Path,
    directory: &str,
    process: &ProcessResult,
) -> Result<Value, String> {
    let stdout = prepared::retain_original(root, directory, "judge-stdout", &process.stdout_bytes)
        .map_err(|e| publication_error(process, "stdout", &e, None, None))?;
    let stderr = prepared::retain_original(root, directory, "judge-stderr", &process.stderr_bytes)
        .map_err(|e| publication_error(process, "stderr", &e, Some(&stdout), None))?;
    let mut value = serde_json::to_value(process).map_err(|e| e.to_string())?;
    let fields = value.as_object_mut().ok_or("judge process object")?;
    for key in INLINE_STREAMS {
        fields.remove(*key);
    }
    fields.insert("stdout_original".into(), json!(stdout));
    fields.insert("stderr_original".into(), json!(stderr));
    Ok(value)
}

/// Publish a process receipt whose stream files were registered before spawn.
/// The files are the originals: do not copy them into content-addressed names
/// after the process has exited, since an enclosing owner can terminate the
/// runner between those two points.  The launch record remains an independently
/// addressed original and makes a partial receipt traceable.
pub fn retain_process_evidence(
    root: &Path,
    process: &ProcessResult,
    stdout_path: &str,
    stderr_path: &str,
    launch_path: &str,
) -> Result<Value, String> {
    let paths = Some((stdout_path, stderr_path, launch_path));
    let stdout = prepared::original(root, stdout_path)
        .map_err(|e| publication_error(process, "stdout", &e, None, paths))?;
    let stderr = prepared::original(root, stderr_path)
        .map_err(|e| publication_error(process, "stderr", &e, Some(&stdout), paths))?;
    if stdout.sha256 != process.stdout_sha256 {
        return Err(publication_error(
            process,
            "stdout",
            "pre-registered stdout digest differs from process receipt",
            None,
            paths,
        ));
    }
    if stderr.sha256 != process.stderr_sha256 {
        return Err(publication_error(
            process,
            "stderr",
            "pre-registered stderr digest differs from process receipt",
            Some(&stdout),
            paths,
        ));
    }
    let launch = prepared::original(root, launch_path)
        .map_err(|e| publication_error(process, "launch", &e, Some(&stdout), paths))?;
    crate::validate_launch_evidence(&crate::no_symlink_parents(root, launch_path)?, process)
        .map_err(|e| publication_error(process, "launch", &e, Some(&stdout), paths))?;
    let mut value = serde_json::to_value(process).map_err(|e| e.to_string())?;
    let fields = value.as_object_mut().ok_or("judge process object")?;
    for key in INLINE_STREAMS {
        fields.remove(*key);
    }
    fields.insert("stdout_original".into(), json!(stdout));
    fields.insert("stderr_original".into(), json!(stderr));
    fields.insert("launch_original".into(), json!(launch));
    Ok(value)
}

/// The caller owns path admission, transport and the cumulative read budget.
/// Reconstructed text/byte views are transient; they are not new originals.
pub fn restore_process(
    value: &Value,
    mut read: impl FnMut(&prepared::Original) -> Result<Vec<u8>, String>,
) -> Result<ProcessResult, String> {
    let mut value = value.clone();
    let fields = value
        .as_object_mut()
        .ok_or("retained judge process object")?;
    if INLINE_STREAMS.iter().any(|key| fields.contains_key(*key)) {
        return Err("retained judge process cannot contain inline streams".into());
    }
    for stream in ["stdout", "stderr"] {
        let original: prepared::Original = serde_json::from_value(
            fields
                .remove(&format!("{stream}_original"))
                .ok_or_else(|| format!("missing judge {stream} original"))?,
        )
        .map_err(|e| e.to_string())?;
        let raw = read(&original)?;
        if sha256(&raw) != original.sha256
            || fields.get(&format!("{stream}_sha256")) != Some(&json!(original.sha256))
        {
            return Err(format!(
                "judge {stream} original digest mismatch: {}",
                original.path
            ));
        }
        fields.insert(stream.into(), json!(String::from_utf8_lossy(&raw)));
        fields.insert(format!("{stream}_bytes"), json!(raw));
    }
    let launch = fields
        .get("launch_original")
        .cloned()
        .map(|value| serde_json::from_value::<prepared::Original>(value).map_err(|e| e.to_string()))
        .transpose()?;
    let process: ProcessResult = serde_json::from_value(value).map_err(|e| e.to_string())?;
    if let Some(original) = launch {
        let raw = read(&original)?;
        if sha256(&raw) != original.sha256 {
            return Err(format!(
                "judge launch original digest mismatch: {}",
                original.path
            ));
        }
        crate::validate_launch_evidence_bytes(&raw, &process)?;
    }
    Ok(process)
}
