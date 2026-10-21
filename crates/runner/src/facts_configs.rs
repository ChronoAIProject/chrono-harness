//! Explicit native-platform selection for a separate Git facts policy.
use crate::{json, no_symlink_parents, relative_path, sha256};
use serde_json::{Value, json as value};
use std::{fs, path::Path};

pub(crate) fn is_selector(config: &Value) -> bool {
    matches!(
        config["schema"].as_str(),
        Some("chrono-git-configs/v1" | "chrono-git-configs/v2")
    )
}

pub(crate) struct Selection {
    pub path: String,
    pub bytes: Vec<u8>,
    pub observation: Value,
}

/// The identity of the configuration used by a fixed endpoint.  `entry_path`
/// is always the path supplied by the caller; `effective_path` is the direct
/// v3 target selected from that entry (or the entry itself for a direct
/// configuration).  The two paths are deliberately kept separate so a
/// selector document is never represented as the selected policy document.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Identity {
    pub entry_path: String,
    pub effective_path: String,
    pub selection: Option<Value>,
}

/// A direct policy keeps its original semantics. A selector has one level only;
/// all paths are host declarations, and unsupported platforms have no fallback.
pub(crate) fn load(
    root: &Path,
    path: &str,
) -> Result<(String, Vec<u8>, Value, Option<Selection>), String> {
    let bytes = fs::read(no_symlink_parents(root, path)?).map_err(|e| e.to_string())?;
    resolve(path, bytes, |target| {
        fs::read(no_symlink_parents(root, target)?).map_err(|e| e.to_string())
    })
}

/// Resolve a selector from bytes belonging to a fixed endpoint.  The reader
/// supplied by the caller is the only way to acquire the selected target, so
/// historical endpoints cannot accidentally use the candidate working tree.
pub(crate) fn resolve(
    path: &str,
    bytes: Vec<u8>,
    mut read: impl FnMut(&str) -> Result<Vec<u8>, String>,
) -> Result<(String, Vec<u8>, Value, Option<Selection>), String> {
    let config = json(&bytes)?;
    if !is_selector(&config) {
        if config["schema"]
            .as_str()
            .is_some_and(|s| s.starts_with("chrono-git-configs/"))
        {
            return Err("unsupported Git config selector schema".into());
        }
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
    let version2 = config["schema"] == "chrono-git-configs/v2";
    for (platform, target) in platforms {
        if platform.is_empty() || platform.trim() != platform {
            return Err("Git config selector platform must be nonempty and literal".into());
        }
        let targets = if version2 {
            identity_targets(target)?
        } else {
            vec![target.as_str().ok_or("Git config selector path required")?]
        };
        for target in targets {
            host_path(target)?;
            if target == path {
                return Err("Git config selector cannot select itself".into());
            }
        }
    }
    let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    let host = platforms
        .get(&platform)
        .ok_or_else(|| format!("Git config selector has no registered platform {platform}"))?;
    let mut identity = None;
    let selected = if version2 {
        let program = host["program"]
            .as_str()
            .ok_or("Git selector program required")?;
        let hash = sha256(&fs::read(program).map_err(|e| format!("Git selector executable: {e}"))?);
        let row = host["configurations"].as_array().unwrap().iter()
            .find(|row| row["sha256"] == hash)
            .ok_or_else(|| format!("Git config selector has no registered executable identity for {platform}: {program} sha256={hash}"))?;
        identity = Some((program, hash));
        row["path"].as_str().unwrap()
    } else {
        host.as_str().ok_or("Git config selector path required")?
    };
    let selected_bytes = read(selected)?;
    let selected_config = json(&selected_bytes)?;
    if !matches!(selected_config["schema_version"].as_u64(), Some(3 | 4))
        || selected_config.get("schema").is_some()
    {
        return Err(
            "Git config selector requires a direct full-v3 policy or v4 policy, never another selector".into(),
        );
    }
    if let Some((program, hash)) = identity {
        let (tool, input) = crate::facts_binding::declaration(&selected_config)?;
        if tool["program"] != program || input["location"] != program || input["sha256"] != hash {
            return Err("Git config selector identity differs from selected policy binding".into());
        }
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

/// Resolve a selector using an already captured registry value map.  This is
/// used by registration consumers after the fixed snapshot has been built and
/// verifies that the real selected target value is present under its own path.
pub fn identity(
    values: &std::collections::BTreeMap<String, Value>,
    path: &str,
) -> Result<Identity, String> {
    let entry = values.get(path).ok_or("missing configuration entry")?;
    if !is_selector(entry) {
        return Ok(Identity {
            entry_path: path.into(),
            effective_path: path.into(),
            selection: None,
        });
    }
    // Reuse the same strict parser and platform key validation as file loading.
    let bytes = serde_json::to_vec(entry).map_err(|e| e.to_string())?;
    let (effective_path, _, _, selection) = resolve(path, bytes, |target| {
        let value = values
            .get(target)
            .ok_or_else(|| format!("missing selected configuration target {target}"))?;
        serde_json::to_vec(value).map_err(|e| e.to_string())
    })?;
    let selection = selection.map(|s| {
        let mut observation = s.observation;
        // The exact selector bytes are retained in the snapshot map.  The
        // value-only identity intentionally carries structural binding fields
        // so it can be reconstructed after JSON decoding without inventing a
        // digest for reformatted bytes.
        if let Some(object) = observation.as_object_mut() {
            object.remove("sha256");
            object.insert("schema".into(), value!("chrono-registry-selection/v1"));
        }
        observation
    });
    Ok(Identity {
        entry_path: path.into(),
        effective_path,
        selection,
    })
}

fn identity_targets(host: &Value) -> Result<Vec<&str>, String> {
    let object = host
        .as_object()
        .ok_or("Git selector identity object required")?;
    if object.len() != 2
        || !object.contains_key("program")
        || !object.contains_key("configurations")
    {
        return Err("Git selector identity requires only program and configurations".into());
    }
    let program = host["program"]
        .as_str()
        .filter(|p| !p.is_empty())
        .ok_or("Git selector program required")?;
    if !Path::new(program).is_absolute() || program.contains(['\0', '\n', '\r']) {
        return Err("Git selector program must be an absolute literal path".into());
    }
    let rows = host["configurations"]
        .as_array()
        .filter(|rows| !rows.is_empty())
        .ok_or("Git selector requires nonempty configurations")?;
    let mut hashes = std::collections::BTreeSet::new();
    let mut paths = vec![];
    for row in rows {
        let object = row
            .as_object()
            .ok_or("Git selector configuration object required")?;
        let hash = row["sha256"]
            .as_str()
            .filter(|s| crate::wire::is_digest(s))
            .ok_or("Git selector configuration digest required")?;
        if object.len() != 2 || !object.contains_key("path") || !hashes.insert(hash) {
            return Err("Git selector configuration requires unique digest and path only".into());
        }
        paths.push(
            row["path"]
                .as_str()
                .ok_or("Git selector configuration path required")?,
        );
    }
    Ok(paths)
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
