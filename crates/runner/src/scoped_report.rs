//! Scoped report storage keeps original process streams once, independently of read projections.
use crate::{ProcessResult, prepared, sha256};
use serde_json::{Value, json};
use std::path::Path;

pub const SCHEMA: &str = "chrono-check-report/v2";
const INLINE_STREAMS: &[&str] = &["stdout", "stderr", "stdout_bytes", "stderr_bytes"];

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
