#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn keep_scheduler_diagnostics(execution_passed: bool, trace_complete: bool) -> (keep: bool)
    ensures keep == !(execution_passed && trace_complete),
{
    !execution_passed || !trace_complete
}
}

#[cfg(not(verus_keep_ghost))]
use chrono_harness::retained_artifacts::retention::{self, Inventory, Publication};
#[cfg(not(verus_keep_ghost))]
use serde_json::json;
#[cfg(not(verus_keep_ghost))]
use std::path::{Path, PathBuf};

/// The release collector reads project diagnostics under this checkout's failure directory.
#[cfg(not(verus_keep_ghost))]
pub fn allocate(checkout: &Path, prefix: &str) -> Result<(PathBuf, Publication), String> {
    let root = checkout.canonicalize().map_err(|e| e.to_string())?;
    let family = ".chrono-harness/state/test-failures";
    let directory = chrono_harness::no_symlink_parents(&root, family)?;
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let path = retention::unique_path(&directory, prefix, "")?;
    let relative = path
        .strip_prefix(&root)
        .ok()
        .and_then(Path::to_str)
        .ok_or("invalid project failure directory")?;
    let owner =
        Inventory::adopted(&root)?.ok_or("project diagnostics require a retention owner")?;
    let publication = owner.create_batch("check-original", relative, json!({"fixture":prefix}))?;
    Ok((path, publication))
}
