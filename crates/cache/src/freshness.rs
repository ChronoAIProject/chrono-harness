//! Restore-time timestamps for declared inputs; Cargo still owns compilation and incremental reuse.
use chrono_harness::file_identity;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    time::{Duration, Instant, SystemTime},
};

pub(crate) const METHOD: &str = "mtime-after-compatible-restore/v1";

pub(crate) struct Prepared {
    latest: SystemTime,
    nodes: usize,
    sources: BTreeMap<String, Value>,
}

fn latest(path: &Path, modified: &mut SystemTime, nodes: &mut usize) -> Result<(), String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.to_string()),
    };
    *nodes += 1;
    *modified = (*modified).max(metadata.modified().map_err(|e| e.to_string())?);
    // Inspect only the explicitly selected artifact. Never follow archive links.
    if metadata.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            latest(&entry.map_err(|e| e.to_string())?.path(), modified, nodes)?;
        }
    }
    Ok(())
}

pub(crate) fn prepare(
    root: &Path,
    cache: &super::Cache,
    entry: &Value,
    inputs: &Value,
) -> Result<Prepared, String> {
    let mut sources = BTreeMap::new();
    for input in &cache.source_inputs {
        let selected = &inputs[input];
        let declaration = &selected["declaration"];
        match declaration["kind"].as_str() {
            Some("file") if selected["observation"]["presence"] == "present" => {
                sources.insert(
                    declaration["path"]
                        .as_str()
                        .ok_or("E_CACHE_FRESHNESS: input path")?
                        .to_owned(),
                    selected["observation"].clone(),
                );
            }
            Some("json-value") => {
                sources.insert(
                    declaration["path"]
                        .as_str()
                        .ok_or("E_CACHE_FRESHNESS: JSON input path")?
                        .to_owned(),
                    selected["source_file"].clone(),
                );
            }
            Some("registered-files") => {
                for (path, original) in selected["observation"]["files"]
                    .as_object()
                    .ok_or("E_CACHE_FRESHNESS: registered inputs missing")?
                {
                    sources.insert(path.clone(), original.clone());
                }
            }
            _ => {}
        }
    }
    if sources.is_empty() {
        return Err("E_CACHE_FRESHNESS: source refresh needs declared file inputs".into());
    }
    for (name, original) in &sources {
        let path = super::path(root, name)?;
        if !fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())?
            .is_file()
        {
            return Err(format!(
                "E_CACHE_FRESHNESS: input is not a regular file: {name}"
            ));
        }
        let (digest, length) = file_identity(&path)?;
        if original["sha256"] != digest || original["length"] != length {
            return Err(format!(
                "E_CACHE_FRESHNESS: current input differs from original plan: {name}"
            ));
        }
    }
    let mut prepared = Prepared {
        latest: SystemTime::UNIX_EPOCH,
        nodes: 0,
        sources,
    };
    for artifact in entry["artifacts"]
        .as_object()
        .ok_or("E_CACHE_FRESHNESS: artifacts missing")?
        .values()
    {
        let path = super::path(
            root,
            artifact["path"]
                .as_str()
                .ok_or("E_CACHE_FRESHNESS: artifact path")?,
        )?;
        latest(&path, &mut prepared.latest, &mut prepared.nodes)?;
    }
    Ok(prepared)
}

pub(crate) fn apply(root: &Path, prepared: Prepared, report: &mut Value) -> Result<(), String> {
    let start = Instant::now();
    let after = prepared
        .latest
        .max(SystemTime::now())
        .checked_add(Duration::from_nanos(1))
        .ok_or("E_CACHE_FRESHNESS: timestamp overflow")?;
    *report = json!({"method":METHOD,"status":"started","source_files":prepared.sources.len(),"artifact_nodes":prepared.nodes,"sources":[]});
    for name in prepared.sources.keys() {
        let mut row = json!({"path":name,"status":"pending"});
        let result = (|| -> Result<&str, String> {
            let path = super::path(root, name)?;
            let file =
                fs::File::open(path).map_err(|e| format!("E_CACHE_FRESHNESS: {name}: {e}"))?;
            let current_newer = file
                .metadata()
                .map_err(|e| e.to_string())?
                .modified()
                .map_err(|e| e.to_string())?
                > prepared.latest;
            match super::verified_planner::freshness_decision(current_newer, false, false, false) {
                super::verified_planner::FreshnessDecision::AlreadyNewer => {
                    return Ok("already-newer");
                }
                super::verified_planner::FreshnessDecision::Updated
                | super::verified_planner::FreshnessDecision::Retry
                | super::verified_planner::FreshnessDecision::Failed => {}
            }
            file.set_modified(after)
                .map_err(|e| format!("E_CACHE_FRESHNESS: {name}: {e}"))?;
            // Coarse timestamp filesystems may round a nanosecond adjustment away.
            let first_newer = file
                .metadata()
                .map_err(|e| e.to_string())?
                .modified()
                .map_err(|e| e.to_string())?
                > prepared.latest;
            match super::verified_planner::freshness_decision(false, first_newer, false, false) {
                super::verified_planner::FreshnessDecision::Updated => {}
                super::verified_planner::FreshnessDecision::Retry => {
                    file.set_modified(
                        after
                            .checked_add(Duration::from_secs(1))
                            .ok_or("E_CACHE_FRESHNESS: timestamp overflow")?,
                    )
                    .map_err(|e| e.to_string())?;
                }
                super::verified_planner::FreshnessDecision::AlreadyNewer
                | super::verified_planner::FreshnessDecision::Failed => {}
            }
            let second_newer = file
                .metadata()
                .map_err(|e| e.to_string())?
                .modified()
                .map_err(|e| e.to_string())?
                > prepared.latest;
            match super::verified_planner::freshness_decision(false, false, second_newer, true) {
                super::verified_planner::FreshnessDecision::Updated => Ok("updated"),
                super::verified_planner::FreshnessDecision::Failed => Err(format!(
                    "E_CACHE_FRESHNESS: input timestamp did not advance past restored outputs: {name}"
                )),
                super::verified_planner::FreshnessDecision::AlreadyNewer
                | super::verified_planner::FreshnessDecision::Retry => Err(format!(
                    "E_CACHE_FRESHNESS: invalid freshness transition: {name}"
                )),
            }
        })();
        match &result {
            Ok(status) => row["status"] = json!(status),
            Err(error) => {
                row["status"] = json!("failed");
                row["error"] = json!(error);
                report["status"] = json!("failed");
            }
        }
        report["sources"].as_array_mut().unwrap().push(row);
        report["apply_elapsed_micros"] = json!(start.elapsed().as_micros());
        result?;
    }
    report["status"] = json!("refreshed");
    Ok(())
}
