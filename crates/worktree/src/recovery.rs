//! Immutable pre-effect identity; an intent never certifies completion or a stopped process.
use chrono_harness::{decode, facts, json, no_symlink_parents, relative_path, sha256};
use serde::Deserialize;
use serde_json::{Value, json as value};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

fn state_path(root: &Path, path: &str) -> Result<PathBuf, String> {
    relative_path(path)?;
    if !path.starts_with(".chrono-harness/state/") {
        return Err("maintenance inputs must be under .chrono-harness/state".into());
    }
    no_symlink_parents(root, path)
}
pub(crate) fn state_bytes(root: &Path, path: &str) -> Result<Vec<u8>, String> {
    fs::read(state_path(root, path)?).map_err(|e| e.to_string())
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
    let mut intent = value!({"schema":"chrono-worktree-recovery-intent/v1"});
    for field in [
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
    ] {
        let text = report[field]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| format!("recovery intent lacks {field}"))?;
        intent[field] = value!(text);
    }
    let path = format!("{}.intent.json", intent["report_path"].as_str().unwrap());
    let target = state_path(root, &path)?;
    let mut bytes = serde_json::to_vec_pretty(&intent).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
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
    report["recovery_intent"] = value!({"path":path,"sha256":sha256(&bytes)});
    Ok(())
}

#[derive(Deserialize)]
#[serde(tag = "presence", rename_all = "lowercase", deny_unknown_fields)]
pub(crate) enum ExpectedResult {
    Absent,
    Present { sha256: String },
}
pub(crate) struct Evidence {
    pub(crate) descriptor: Value,
    root: PathBuf,
    intent_path: String,
    intent_bytes: Vec<u8>,
    result_path: String,
    result_bytes: Option<Vec<u8>>,
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
    let bytes = state_bytes(root, path)?;
    if sha256(&bytes) != digest {
        return Err("recovery intent digest mismatch".into());
    }
    let intent: Intent = decode(&bytes)?;
    let descriptor = json(&bytes)?;
    report["prior_intent"] =
        value!({"path":path,"sha256":digest,"input_bytes":bytes,"intent":descriptor});
    let config: crate::Config = decode(config_bytes)?;
    if intent.schema != "chrono-worktree-recovery-intent/v1"
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
    let result = original_bytes(root, &intent.report_path)?;
    match (expected, &result) {
        (ExpectedResult::Absent, None) => (),
        (ExpectedResult::Present { sha256: digest }, Some(bytes)) if sha256(bytes) == *digest => (),
        _ => return Err("original result presence or digest mismatch".into()),
    }
    report["prior_result"] = match &result {
        Some(bytes) => {
            value!({"path":intent.report_path,"presence":"present","sha256":sha256(bytes),"input_bytes":bytes})
        }
        None => value!({"path":intent.report_path,"presence":"absent"}),
    };
    // A retained terminal result is never silently recast as an interrupted attempt.
    if let Some(bytes) = &result {
        if let Ok(original) = json(bytes) {
            if original["schema"] == "chrono-worktree-report/v1"
                && matches!(
                    original["status"].as_str(),
                    Some("failed" | "created" | "reconstructed" | "recovered" | "cleaned")
                )
            {
                return Err("original result declares a terminal outcome; use its ordinary maintenance contract".into());
            }
        }
    }
    report["original_outcome"] = value!("unknown");
    Ok(Evidence {
        descriptor,
        root: root.into(),
        intent_path: path.into(),
        intent_bytes: bytes,
        result_path: intent.report_path,
        result_bytes: result,
    })
}
