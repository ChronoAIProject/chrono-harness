//! Explicit native-platform selection for a separate Git facts policy.
use crate::verified_git_binding::{
    GitConfigSchema, admit_git_selector_target, admit_selector_binding, classify_git_config_schema,
};
use crate::{json, no_symlink_parents, relative_path, sha256};
use serde_json::{Value, json as value};
use std::{fs, path::Path};

fn config_schema(config: &Value) -> GitConfigSchema {
    classify_git_config_schema(config["schema"].as_str())
}

pub(crate) fn is_selector(config: &Value) -> bool {
    config_schema(config).is_selector()
}

pub(crate) struct Selection {
    pub path: String,
    pub bytes: Vec<u8>,
    pub observation: Value,
    pub registry_path: String,
}

#[derive(Clone, Copy)]
pub(crate) struct RegistryTarget<'a> {
    pub path: &'a str,
    pub sha256: Option<&'a str>,
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
    read: impl FnMut(&str) -> Result<Vec<u8>, String>,
) -> Result<(String, Vec<u8>, Value, Option<Selection>), String> {
    resolve_target(path, bytes, None, read)
}

/// A request-bound reader fixes the policy path, while an immutable endpoint
/// supplies that path's original executable declaration as data. Only opening
/// the candidate's execution binding reads the current executable bytes.
pub(crate) fn resolve_target(
    path: &str,
    bytes: Vec<u8>,
    target: Option<RegistryTarget<'_>>,
    mut read: impl FnMut(&str) -> Result<Vec<u8>, String>,
) -> Result<(String, Vec<u8>, Value, Option<Selection>), String> {
    use crate::verified_git_binding::{admit_selector_object, admit_selector_platforms};

    let config = json(&bytes)?;
    let schema = config_schema(&config);
    match schema {
        GitConfigSchema::Direct => return Ok((path.into(), bytes, config, None)),
        GitConfigSchema::Unsupported => {
            return Err("unsupported Git config selector schema".into());
        }
        GitConfigSchema::V1 | GitConfigSchema::V2 | GitConfigSchema::V3 => {}
    }
    let object = config.as_object();
    admit_selector_object(
        object.map(|map| map.len()),
        object.is_some_and(|map| map.contains_key("platforms")),
    )?;
    host_path(path, None)?;
    let platforms =
        admit_selector_platforms(config["platforms"].as_object().map(|map| (map, map.len())))?;
    let version3 = schema.has_registry_path();
    let identities = schema.has_identity();
    for (platform, target) in platforms {
        if platform.is_empty() || platform.trim() != platform {
            return Err("Git config selector platform must be nonempty and literal".into());
        }
        let targets = if identities {
            identity_targets(target, version3)?
        } else {
            vec![target.as_str().ok_or("Git config selector path required")?]
        };
        for target in targets {
            host_path(target, Some(path))?;
        }
    }
    let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    let host = platforms
        .get(&platform)
        .ok_or_else(|| format!("Git config selector has no registered platform {platform}"))?;
    let mut identity = None;
    let mut registry_path = None;
    let selected = if identities {
        use crate::verified_git_binding::select_git_target;

        let program = host["program"]
            .as_str()
            .ok_or("Git selector program required")?;
        let rows = host["configurations"].as_array().unwrap();
        let index = if let Some(target) = target {
            // Current executable identities retain each endpoint's explicit
            // target. Otherwise only the candidate-declared data path is eligible;
            // it never admits an executable or selects an unregistered policy.
            select_git_target(&JsonTargetRows(rows), target.sha256, Some(target.path)).ok_or_else(
                || {
                    format!(
                        "Git config selector has no bound registry policy path for {platform}: {}",
                        target.path
                    )
                },
            )?
        } else {
            let hash =
                sha256(&fs::read(program).map_err(|e| format!("Git selector executable: {e}"))?);
            select_git_target(&JsonTargetRows(rows), Some(&hash), None)
                .ok_or_else(|| format!("Git config selector has no registered executable identity for {platform}: {program} sha256={hash}"))?
        };
        let row = &rows[index];
        identity = Some((program, row["sha256"].as_str().unwrap()));
        if version3 {
            registry_path = Some(row["registry_path"].as_str().unwrap());
        }
        row["path"].as_str().unwrap()
    } else {
        host.as_str().ok_or("Git config selector path required")?
    };
    let selected_bytes = read(selected)?;
    let selected_config = json(&selected_bytes)?;
    admit_git_selector_target(
        selected_config["schema_version"].as_u64(),
        selected_config.get("schema").is_some(),
    )?;
    if let Some((program, hash)) = identity {
        let (tool, input) = crate::facts_binding::declaration(&selected_config)?;
        admit_selector_binding(
            program,
            hash,
            tool["program"].as_str(),
            input.map(|value| (value["location"].as_str(), value["sha256"].as_str())),
        )?;
    }
    let mut observation = value!({"schema":"chrono-git-config-selection/v1",
        "path":path,"sha256":sha256(&bytes),"platform":platform,"config_path":selected});
    if let Some(registry_path) = registry_path {
        observation["registry_path"] = value!(registry_path);
    }
    let selection = Selection {
        path: path.into(),
        registry_path: registry_path.unwrap_or(selected).into(),
        observation,
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
    let schema = config_schema(entry);
    if !schema.is_selector() {
        return Ok(Identity {
            entry_path: path.into(),
            effective_path: path.into(),
            selection: None,
        });
    }
    // A captured value map contains exactly one effective policy. Reconstruct
    // its declared identity from that data, without consulting today's Git.
    let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    let target = if schema.has_identity() {
        let host = &entry["platforms"][&platform];
        let targets = identity_targets(host, schema.has_registry_path())?;
        Some(RegistryTarget {
            path: crate::verified_git_binding::select_captured_target(
                &targets,
                &CapturedRegistry(values),
            )?,
            sha256: None,
        })
    } else {
        None
    };
    let bytes = serde_json::to_vec(entry).map_err(|e| e.to_string())?;
    let (effective_path, _, _, selection) = resolve_target(path, bytes, target, |target| {
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

struct CapturedRegistry<'a>(&'a std::collections::BTreeMap<String, Value>);

impl crate::verified_git_binding::RegistryMembership for CapturedRegistry<'_> {
    fn contains_path(&self, path: &str) -> bool {
        self.0.contains_key(path)
    }
}

fn identity_targets(host: &Value, version3: bool) -> Result<Vec<&str>, String> {
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
    let mut unique_paths = std::collections::BTreeSet::new();
    let mut paths = vec![];
    for row in rows {
        let object = row
            .as_object()
            .ok_or("Git selector configuration object required")?;
        let hash = row["sha256"]
            .as_str()
            .filter(|s| crate::wire::is_digest(s))
            .ok_or("Git selector configuration digest required")?;
        if object.len() != if version3 { 3 } else { 2 }
            || !object.contains_key("path")
            || (version3 && !object.contains_key("registry_path"))
            || !hashes.insert(hash)
        {
            return Err("Git selector configuration requires a unique digest/path and the declared v3 registry_path only".into());
        }
        let path = row["path"]
            .as_str()
            .ok_or("Git selector configuration path required")?;
        if !unique_paths.insert(path) {
            return Err("Git selector configuration paths must be unique".into());
        }
        paths.push(path);
    }
    if version3 {
        for row in rows {
            let path = row["registry_path"]
                .as_str()
                .ok_or("Git selector registry_path required")?;
            host_path(path, None)?;
            if !paths.contains(&path) {
                return Err(
                    "Git selector registry_path must name a registered policy target".into(),
                );
            }
        }
    }
    Ok(paths)
}

fn host_path(path: &str, entry: Option<&str>) -> Result<(), String> {
    relative_path(path)?;
    crate::verified_git_binding::admit_selector_host_path(path, entry)?;
    Ok(())
}

struct JsonTargetRows<'a>(&'a [Value]);

impl crate::verified_git_binding::GitTargetRows for JsonTargetRows<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn text(
        &self,
        index: usize,
        field: &crate::verified_git_binding::GitTargetField,
    ) -> Option<&str> {
        use crate::verified_git_binding::GitTargetField;

        let key = match field {
            GitTargetField::Digest => "sha256",
            GitTargetField::Path => "path",
        };
        self.0[index][key].as_str()
    }
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
