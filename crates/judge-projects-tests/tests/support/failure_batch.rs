use chrono_harness::retained_artifacts::retention::{self, Inventory, Publication};
use serde_json::json;
use std::path::{Path, PathBuf};

/// The release collector reads project diagnostics under this checkout's failure directory.
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
