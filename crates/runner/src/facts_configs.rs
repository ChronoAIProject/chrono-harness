//! Explicit native-platform selection for a separate Git facts policy.
use crate::{json, no_symlink_parents, relative_path, sha256};
use serde_json::{Value, json as value};
use std::{fs, path::Path};

pub(crate) struct Selection {
    pub path: String,
    pub bytes: Vec<u8>,
    pub observation: Value,
}

/// A direct policy keeps its original semantics. A selector has one level only;
/// all paths are host declarations, and unsupported platforms have no fallback.
pub(crate) fn load(
    root: &Path,
    path: &str,
) -> Result<(String, Vec<u8>, Value, Option<Selection>), String> {
    let bytes = fs::read(no_symlink_parents(root, path)?).map_err(|e| e.to_string())?;
    let config = json(&bytes)?;
    if config["schema"] != "chrono-git-configs/v1" {
        return Ok((path.into(), bytes, config, None));
    }
    let object = config
        .as_object()
        .ok_or("Git config selector object required")?;
    if object.len() != 2 || !object.contains_key("platforms") {
        return Err("Git config selector requires only schema and platforms".into());
    }
    host_path(path)?;
    let platforms = config["platforms"]
        .as_object()
        .filter(|map| !map.is_empty())
        .ok_or("Git config selector requires nonempty platforms")?;
    for (platform, target) in platforms {
        if platform.is_empty() || platform.trim() != platform {
            return Err("Git config selector platform must be nonempty and literal".into());
        }
        let target = target.as_str().ok_or("Git config selector path required")?;
        host_path(target)?;
        if target == path {
            return Err("Git config selector cannot select itself".into());
        }
    }
    let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    let selected = platforms
        .get(&platform)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Git config selector has no registered platform {platform}"))?;
    let selected_bytes =
        fs::read(no_symlink_parents(root, selected)?).map_err(|e| e.to_string())?;
    let selected_config = json(&selected_bytes)?;
    if selected_config["schema_version"] != 3 || selected_config.get("schema").is_some() {
        return Err(
            "Git config selector requires a direct full-v3 policy, never another selector".into(),
        );
    }
    let selection = Selection {
        path: path.into(),
        observation: value!({"schema":"chrono-git-config-selection/v1",
            "path":path,"sha256":sha256(&bytes),"platform":platform,"config_path":selected}),
        bytes,
    };
    Ok((
        selected.into(),
        selected_bytes,
        selected_config,
        Some(selection),
    ))
}

fn host_path(path: &str) -> Result<(), String> {
    relative_path(path)?;
    if !path.starts_with(".chrono-harness/") {
        return Err("Git config selector paths must reside in .chrono-harness/".into());
    }
    Ok(())
}

impl Selection {
    pub fn unchanged(&self, root: &Path) -> Result<(), String> {
        if fs::read(no_symlink_parents(root, &self.path)?).map_err(|e| e.to_string())? != self.bytes
        {
            return Err("Git config selector changed".into());
        }
        Ok(())
    }
}
