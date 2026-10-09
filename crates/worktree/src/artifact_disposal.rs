//! Explicit directory disposal shared by legacy checkout cleanup and lifecycle cleanup.
use crate::start::Runner;
use chrono_harness::no_symlink_parents;
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn registered(names: &[String], endpoints: &[&Value]) -> Result<(), String> {
    for name in names {
        for host in endpoints {
            if !host["artifacts"]
                .as_array()
                .ok_or("missing artifact registrations")?
                .iter()
                .any(|a| a["path"] == *name && a["tracked"] == false)
            {
                return Err(
                    "disposal directory is not an artifact in both endpoint registries".into(),
                );
            }
        }
    }
    Ok(())
}

pub(crate) fn paths(
    r: &mut Runner,
    target: &Path,
    head: &str,
    names: &[String],
) -> Result<Vec<PathBuf>, String> {
    if names.is_empty() {
        return Ok(vec![]);
    }
    let tree = r.tree(target, head)?;
    // Staged files are also source, including additions within an artifact prefix.
    let index = crate::start::paths(r.git(target, &["ls-files", "-z"])?)?;
    names
        .iter()
        .map(|name| {
            if tree
                .keys()
                .chain(index.iter())
                .any(|p| p.starts_with(name) || p == name.trim_end_matches('/'))
            {
                return Err(format!("disposal directory contains tracked paths: {name}"));
            }
            let path = no_symlink_parents(target, name.trim_end_matches('/'))?;
            match fs::symlink_metadata(&path) {
                Ok(m) if m.is_dir() => Ok(path),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(path),
                Err(e) => Err(e.to_string()),
                Ok(_) => Err(format!("disposal requires a physical directory: {name}")),
            }
        })
        .collect()
}

#[derive(Default, Serialize)]
pub struct ArtifactFootprint {
    pub file_entries: u64,
    pub logical_bytes: u64,
    pub allocated_bytes: Option<u64>,
}

/// Measure only the caller's exact object. Block allocation is not physical savings.
pub fn artifact_footprint(path: &Path) -> Result<ArtifactFootprint, String> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ArtifactFootprint {
            allocated_bytes: if cfg!(unix) { Some(0) } else { None },
            ..Default::default()
        }),
        Err(e) => Err(e.to_string()),
        Ok(m) => {
            #[cfg(unix)]
            let allocated = {
                use std::os::unix::fs::MetadataExt;
                Some(
                    m.blocks()
                        .checked_mul(512)
                        .ok_or("artifact allocation overflow")?,
                )
            };
            #[cfg(not(unix))]
            let allocated = None;
            let mut result = ArtifactFootprint {
                allocated_bytes: allocated,
                ..Default::default()
            };
            if m.is_dir() {
                for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
                    let child = artifact_footprint(&entry.map_err(|e| e.to_string())?.path())?;
                    result.logical_bytes = result
                        .logical_bytes
                        .checked_add(child.logical_bytes)
                        .ok_or("artifact byte count overflow")?;
                    result.file_entries = result
                        .file_entries
                        .checked_add(child.file_entries)
                        .ok_or("artifact count overflow")?;
                    result.allocated_bytes = match (result.allocated_bytes, child.allocated_bytes) {
                        (Some(a), Some(b)) => {
                            Some(a.checked_add(b).ok_or("artifact allocation overflow")?)
                        }
                        _ => None,
                    };
                }
            } else if m.is_file() || m.file_type().is_symlink() {
                result.logical_bytes = m.len();
                result.file_entries = 1;
            } else {
                return Err("artifact contains a special file; preserve it".into());
            }
            Ok(result)
        }
    }
}

pub(crate) fn dispose(
    target: &Path,
    names: &[String],
    paths: &[PathBuf],
    report: &mut Value,
    mut before_effect: impl FnMut() -> Result<(), String>,
) -> Result<(), String> {
    let state = |path: &Path, name: &str| match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok("already-absent"),
        Err(e) => Err(e.to_string()),
        Ok(m) if m.is_dir() => Ok("attempted-unverified"),
        Ok(_) => Err(format!("disposal requires a physical directory: {name}")),
    };
    for (name, path) in names.iter().zip(paths) {
        no_symlink_parents(target, name.trim_end_matches('/'))?;
        let mut observed = state(path, name)?;
        // An absent output has no removal effect. Recheck both ownership and
        // the physical path before every actual removal.
        if observed != "already-absent" {
            before_effect()?;
            no_symlink_parents(target, name.trim_end_matches('/'))?;
            observed = state(path, name)?;
        }
        let measured = report["automatic_cleanup"] == true;
        let mut effect = json!({"path":name,"status":observed});
        if measured {
            effect["byte_measure"] = json!("literal-entry-lengths");
            let footprint = artifact_footprint(path)?;
            effect["bytes_before"] = json!(footprint.logical_bytes);
            effect["footprint_before"] = json!(footprint);
            effect["allocated_measure"] = json!("lstat-blocks-times-512-including-directories");
            effect["physical_release_bytes"] = Value::Null;
            effect["bytes_after"] = json!(if observed == "already-absent" {
                Some(0)
            } else {
                None::<u64>
            });
        }
        report["artifact_disposals"]
            .as_array_mut()
            .ok_or("missing artifact disposal report")?
            .push(effect);
        if observed == "already-absent" {
            continue;
        }
        // Internal links are unlinked; their external targets are never traversed.
        if let Err(error) = fs::remove_dir_all(path) {
            let effect = report["artifact_disposals"]
                .as_array_mut()
                .unwrap()
                .last_mut()
                .unwrap();
            effect["status"] = json!("failed-partial-effects-possible");
            effect["error"] = json!(error.to_string());
            if measured {
                match artifact_footprint(path) {
                    Ok(remaining) => {
                        effect["bytes_after"] = json!(remaining.logical_bytes);
                        effect["footprint_after"] = json!(remaining);
                    }
                    Err(observation_error) => {
                        effect["remaining_measure_error"] = json!(observation_error)
                    }
                }
            }
            return Err(format!("artifact disposal {name}: {error}"));
        }
        match fs::symlink_metadata(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.to_string()),
            Ok(_) => return Err(format!("disposed artifact still exists: {name}")),
        }
        let effect = report["artifact_disposals"]
            .as_array_mut()
            .unwrap()
            .last_mut()
            .unwrap();
        effect["status"] = json!("verified-absent");
        if measured {
            effect["bytes_after"] = json!(0);
            effect["footprint_after"] = json!(artifact_footprint(path)?);
        }
    }
    Ok(())
}
