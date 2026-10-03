//! Explicit directory disposal shared by legacy checkout cleanup and lifecycle cleanup.
use crate::start::Runner;
use chrono_harness::{facts, no_symlink_parents};
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
    let tree = facts::parse_tree(&r.git(target, &["ls-tree", "-rz", "--full-tree", head])?)?;
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

fn bytes(path: &Path) -> Result<u64, String> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(e) => Err(e.to_string()),
        Ok(m) if m.is_dir() => {
            let mut size = 0u64;
            for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
                size = size
                    .checked_add(bytes(&entry.map_err(|e| e.to_string())?.path())?)
                    .ok_or("artifact byte count overflow")?;
            }
            Ok(size)
        }
        Ok(m) if m.is_file() || m.file_type().is_symlink() => Ok(m.len()),
        Ok(_) => Err("artifact contains a special file; preserve it".into()),
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
            effect["bytes_before"] = json!(bytes(path)?);
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
        fs::remove_dir_all(path).map_err(|e| format!("artifact disposal {name}: {e}"))?;
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
        }
    }
    Ok(())
}
