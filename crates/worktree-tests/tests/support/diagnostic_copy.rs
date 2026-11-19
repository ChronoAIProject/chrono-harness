//! Publish one diagnostic copy in the checkout that owns its retention policy.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub enum CopyMember { Directory, File, Skip }

pub fn short_copy_member(directory: bool, file: bool) -> (member: CopyMember)
    ensures
        matches!(member, CopyMember::Directory) == directory,
        matches!(member, CopyMember::File) == (!directory && file),
        matches!(member, CopyMember::Skip) == (!directory && !file),
{
    if directory { CopyMember::Directory } else if file { CopyMember::File } else { CopyMember::Skip }
}
}

#[cfg(not(verus_keep_ghost))]
use chrono_harness::retained_artifacts::retention::{self, Inventory, Publication};
#[cfg(not(verus_keep_ghost))]
use serde_json::Value;
#[cfg(not(verus_keep_ghost))]
use std::{
    fs,
    path::{Path, PathBuf},
};

#[cfg(not(verus_keep_ghost))]
pub fn publish(
    checkout: &Path,
    producer: &str,
    family: &str,
    prefix: &str,
    mut outcome: Value,
    copy: impl FnOnce(&Path, &Publication) -> Result<(), String>,
) -> Result<PathBuf, String> {
    let root = fs::canonicalize(checkout).map_err(|error| error.to_string())?;
    let owner = Inventory::adopted(&root)?.ok_or("missing fixture retention policy")?;
    let directory = chrono_harness::no_symlink_parents(&root, family)?;
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let target = retention::unique_path(&directory, prefix, "")?;
    let relative = target
        .strip_prefix(&root)
        .ok()
        .and_then(Path::to_str)
        .ok_or("invalid diagnostic copy address")?;
    let publication = owner.plan(producer, relative, true, outcome.clone())?;
    publication.materialize()?;
    let copying = copy(&target, &publication);
    // Sealing releases the writer. The reader holds the copy through diagnostic handoff.
    let reader_owner = Inventory::adopted(&root)?.ok_or("missing fixture retention policy")?;
    let _reader = reader_owner.consume(publication.id())?;
    outcome["copy_error"] = serde_json::json!(copying.as_ref().err());
    let handoff = publication
        .complete_sealed(outcome)
        .and_then(|()| reader_owner.retain_diagnostic_path(relative));
    if let Err(error) = copying {
        return Err(match handoff {
            Ok(()) => format!("{}: {error}", target.display()),
            Err(seal_error) => format!("{}: {error}; publication: {seal_error}", target.display()),
        });
    }
    handoff?;
    Ok(target)
}

#[cfg(not(verus_keep_ghost))]
pub fn copy_tree(
    source: &Path,
    target: &Path,
    publication: Option<&retention::Publication>,
) -> Result<(), String> {
    retention::directory_adopted(publication, target)?;
    for entry in fs::read_dir(source).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let target = target.join(entry.file_name());
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        match short_copy_member(kind.is_dir(), kind.is_file()) {
            CopyMember::Directory => copy_tree(&entry.path(), &target, publication)?,
            CopyMember::File => retention::copy_adopted(publication, &entry.path(), &target)?,
            CopyMember::Skip => {}
        }
    }
    Ok(())
}
