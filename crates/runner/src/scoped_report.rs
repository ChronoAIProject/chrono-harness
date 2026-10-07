//! Scoped report storage keeps original process streams once, independently of read projections.
use crate::{ProcessResult, prepared, sha256};
use serde_json::{Value, json};
use std::path::Path;

pub const SCHEMA: &str = "chrono-check-report/v2";
const INLINE_STREAMS: &[&str] = &["stdout", "stderr", "stdout_bytes", "stderr_bytes"];

pub fn retain_process(
    root: &Path,
    directory: &str,
    process: &ProcessResult,
) -> Result<Value, String> {
    let stdout = prepared::retain_original(root, directory, "judge-stdout", &process.stdout_bytes)
        .map_err(|e| {
            format!(
                "cannot retain judge stdout (exit {}, failure {:?}): {e}",
                process.exit_code, process.failure
            )
        })?;
    let stderr = prepared::retain_original(root, directory, "judge-stderr", &process.stderr_bytes)
        .map_err(|e| {
            format!(
                "cannot retain judge stderr (exit {}, failure {:?}); stdout retained at {}: {e}",
                process.exit_code, process.failure, stdout.path
            )
        })?;
    let mut value = serde_json::to_value(process).map_err(|e| e.to_string())?;
    let fields = value.as_object_mut().ok_or("judge process object")?;
    for key in INLINE_STREAMS {
        fields.remove(*key);
    }
    fields.insert("stdout_original".into(), json!(stdout));
    fields.insert("stderr_original".into(), json!(stderr));
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
    serde_json::from_value(value).map_err(|e| e.to_string())
}
