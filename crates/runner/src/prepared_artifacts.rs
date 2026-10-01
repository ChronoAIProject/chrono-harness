//! Addressed short preparation originals and the selected native upload contract.
use super::*;
use std::io::{Read, Write};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeArtifacts {
    pub config_path: String,
    pub config_sha256: String,
    pub directory: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Original {
    pub path: String,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactTransport {
    pub source_directory: String,
    pub directory: String,
}
fn directory(path: &str) -> Result<(), String> {
    units::artifact_path(path)?;
    if !path.ends_with('/') {
        return Err("artifact directory must end /".into());
    }
    Ok(())
}
/// Resolve the declared CI action's existing provider selection, never a host-wide upload fallback.
pub fn native_artifacts(
    root: &Path,
    cfg: &Value,
    source: &str,
    selection: &Selection,
) -> Result<Option<NativeArtifacts>, String> {
    if source == "local" {
        return Ok(None);
    }
    let c = declaration(cfg)?;
    let action = c.inputs.ci.as_ref().ok_or("native input binding missing")?;
    let paths: Vec<_> = action
        .argv
        .windows(2)
        .filter(|a| a[0] == "--config")
        .map(|a| a[1].as_str())
        .collect();
    if paths.len() != 1 {
        return Err("native producer requires one explicit provider --config".into());
    }
    let path = paths[0];
    relative_path(path)?;
    let raw = fs::read(no_symlink_parents(root, path)?).map_err(|e| e.to_string())?;
    let provider = crate::json(&raw)?;
    let settings = match provider["schema"].as_str() {
        Some("chrono-github-units/v1" | "chrono-github-units/v2") => match selection {
            Selection::Unit { unit } => &provider["units"][unit],
            _ => &provider["collection"],
        },
        Some("chrono-github-ci/v4") if *selection == Selection::All => &provider,
        Some("chrono-github-full-ci/v2") => {
            let scope: Option<crate::units::Scope> =
                serde_json::from_value(provider.get("scope").cloned().unwrap_or(Value::Null))
                    .map_err(|e| e.to_string())?;
            if !selection.matches(&scope) {
                return Err("full provider scope differs from short selection".into());
            }
            &provider
        }
        _ => return Err("unsupported native short provider/scope".into()),
    };
    let dir = settings["artifact_directory"]
        .as_str()
        .ok_or("selected native artifact_directory missing")?;
    directory(dir)?;
    if !cfg["artifacts"]
        .as_array()
        .ok_or("artifacts missing")?
        .iter()
        .any(|a| a["tracked"] == false && a["path"].as_str().is_some_and(|p| dir.starts_with(p)))
    {
        return Err("native upload directory lacks declared artifact ownership".into());
    }
    Ok(Some(NativeArtifacts {
        config_path: path.into(),
        config_sha256: sha256(&raw),
        directory: dir.into(),
    }))
}
pub fn retention_directory(req: &InputRequest) -> String {
    req.native_artifacts
        .as_ref()
        .map(|a| format!("{}preparation/", a.directory))
        .unwrap_or_else(|| ".chrono-harness/state/preparation/".into())
}
pub fn original(root: &Path, path: &str) -> Result<Original, String> {
    units::artifact_path(path)?;
    Ok(Original {
        path: path.into(),
        sha256: file_identity(&no_symlink_parents(root, path)?)?.0,
    })
}
/// Reuse exact original bytes in the selected evidence owner; no recursive evidence duplication.
pub fn retain_original(
    root: &Path,
    dir: &str,
    prefix: &str,
    bytes: &[u8],
) -> Result<Original, String> {
    directory(dir)?;
    units::id(prefix)?;
    let path = format!("{dir}{prefix}-{}.json", sha256(bytes));
    let target = no_symlink_parents(root, &path)?;
    fs::create_dir_all(target.parent().ok_or("original parent")?).map_err(|e| e.to_string())?;
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
    {
        Ok(mut f) => f.write_all(bytes).map_err(|e| e.to_string())?,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            if fs::read(&target).map_err(|e| e.to_string())? != bytes {
                return Err("addressed original bytes mismatch".into());
            }
        }
        Err(e) => return Err(e.to_string()),
    }
    original(root, &path)
}
pub fn original_path(
    o: &Original,
    transport: Option<&ArtifactTransport>,
) -> Result<String, String> {
    units::artifact_path(&o.path)?;
    let path = if let Some(t) = transport {
        directory(&t.source_directory)?;
        directory(&t.directory)?;
        let relative = o
            .path
            .strip_prefix(&t.source_directory)
            .ok_or("original evidence outside selected native upload root")?;
        relative_path(relative)?;
        format!("{}{relative}", t.directory)
    } else {
        o.path.clone()
    };
    Ok(path)
}
pub fn read_original(
    root: &Path,
    o: &Original,
    transport: Option<&ArtifactTransport>,
) -> Result<Vec<u8>, String> {
    let path = original_path(o, transport)?;
    let target = no_symlink_parents(root, &path)?;
    let meta = fs::symlink_metadata(&target)
        .map_err(|e| format!("missing original evidence {path}: {e}"))?;
    if !meta.is_file() || meta.len() > 64 * 1024 * 1024 {
        return Err("original evidence is not a bounded regular file".into());
    }
    let mut raw = vec![];
    fs::File::open(target)
        .map_err(|e| e.to_string())?
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    if raw.len() > 64 * 1024 * 1024 || sha256(&raw) != o.sha256 {
        return Err(format!("original evidence digest mismatch: {path}"));
    }
    Ok(raw)
}
fn read_binding_original(
    root: &Path,
    original: &Original,
    transport: Option<&ArtifactTransport>,
    artifacts: Option<&Value>,
) -> Result<Vec<u8>, String> {
    if let Some(artifacts) = artifacts {
        units::artifact_path(&original.path)?;
        let raw = crate::retained_artifacts::bytes(root, artifacts, &original.path, transport)?;
        if sha256(&raw) != original.sha256 {
            return Err("retained preparation original digest mismatch".into());
        }
        Ok(raw)
    } else {
        read_original(root, original, transport)
    }
}
pub fn validate_originals(
    root: &Path,
    p: &PreparedCheck,
    transport: Option<&ArtifactTransport>,
) -> Result<(), String> {
    validate_originals_with_artifacts(root, p, transport, None)
}
fn validate_originals_with_artifacts(
    root: &Path,
    p: &PreparedCheck,
    transport: Option<&ArtifactTransport>,
    artifacts: Option<&Value>,
) -> Result<(), String> {
    let mut paths = BTreeSet::new();
    for o in &p.originals {
        if !paths.insert(&o.path) {
            return Err("duplicate original evidence path".into());
        }
        read_binding_original(root, o, transport, artifacts)?;
    }
    let report = Original {
        path: p.evidence["report_path"]
            .as_str()
            .ok_or("original producer report missing")?
            .into(),
        sha256: p.evidence["report_sha256"]
            .as_str()
            .ok_or("original producer report digest missing")?
            .into(),
    };
    if !p.originals.contains(&report) {
        return Err("producer report missing from original evidence closure".into());
    }
    if let Some(c) = &p.context {
        let o = Original {
            path: c.path.clone(),
            sha256: c.sha256.clone(),
        };
        if !p.originals.contains(&o)
            || read_binding_original(root, &o, transport, artifacts)? != c.raw
            || sha256(&c.raw) != c.sha256
            || wire::digest(&crate::json(&c.raw)?)? != c.semantic_digest
        {
            return Err("prepared context raw/semantic identity mismatch".into());
        }
    }
    // The retained producer explicitly addresses its other originals; omissions cannot be hidden in a report.
    let evidence = crate::json(&read_binding_original(root, &report, transport, artifacts)?)?;
    if let Some(refs) = evidence.get("originals") {
        let refs: Vec<Original> =
            serde_json::from_value(refs.clone()).map_err(|e| e.to_string())?;
        if refs.iter().any(|o| !p.originals.contains(o)) {
            return Err("producer original evidence closure incomplete".into());
        }
    }
    Ok(())
}
/// Offline native consumers retain original observation identities and resolve only addressed artifact paths.
pub fn validate_portable_binding(
    root: &Path,
    binding: &Value,
    transport: Option<&ArtifactTransport>,
) -> Result<(), String> {
    validate_retained_binding(root, binding, transport, None)
}
pub fn validate_retained_binding(
    root: &Path,
    binding: &Value,
    transport: Option<&ArtifactTransport>,
    artifacts: Option<&Value>,
) -> Result<(), String> {
    let req: InputRequest =
        serde_json::from_value(binding["request"].clone()).map_err(|e| e.to_string())?;
    let p: PreparedCheck =
        serde_json::from_value(binding["result"].clone()).map_err(|e| e.to_string())?;
    let (_, _, cfg, _) = crate::facts_configs::load(root, &req.host_config)?;
    if native_artifacts(root, &cfg, &req.source, &req.selection)? != req.native_artifacts {
        return Err("producer/upload contract mismatch".into());
    }
    if let Some(t) = transport {
        if req
            .native_artifacts
            .as_ref()
            .is_none_or(|a| a.directory != t.source_directory)
        {
            return Err("original upload/transport root mismatch".into());
        }
    }
    validate_result_identity(&req, &p)?;
    for (path, hash) in [
        (&req.host_config, &req.host_config_sha256),
        (&req.effective_config, &req.effective_config_sha256),
        (&req.profile, &req.profile_sha256),
    ] {
        if file_identity(&no_symlink_parents(root, path)?)?.0 != *hash {
            return Err("portable preparation policy identity mismatch".into());
        }
    }
    if let Some(a) = &req.native_artifacts {
        if p.originals
            .iter()
            .any(|o| !o.path.starts_with(&a.directory))
        {
            return Err("original evidence outside selected native upload root".into());
        }
    }
    validate_originals_with_artifacts(root, &p, transport, artifacts)?;
    let refs: Vec<Original> =
        serde_json::from_value(binding["receipts"].clone()).map_err(|e| e.to_string())?;
    if refs.len()
        != if req.source == "local" && req.selection == Selection::Collect {
            2
        } else {
            1
        }
    {
        return Err("short producer receipt count mismatch".into());
    }
    for (index, o) in refs.iter().enumerate() {
        if req
            .native_artifacts
            .as_ref()
            .is_some_and(|a| !o.path.starts_with(&a.directory))
        {
            return Err("acquisition outside selected native upload root".into());
        }
        let r = crate::json(&read_binding_original(root, o, transport, artifacts)?)?;
        let request: InputRequest =
            serde_json::from_value(r["request"].clone()).map_err(|e| e.to_string())?;
        let process: ProcessResult =
            serde_json::from_value(r["process"].clone()).map_err(|e| e.to_string())?;
        let result: PreparedCheck = decode(&process.stdout_bytes)?;
        validate_result_identity(&request, &result)?;
        let c = declaration(&cfg)?;
        let expected = if req.source == "local" && index == 0 {
            &c.inputs.local
        } else {
            c.inputs.ci.as_ref().ok_or("missing CI binding")?
        };
        if r["action"] != serde_json::to_value(expected).map_err(|e| e.to_string())?
            || request.native_artifacts != req.native_artifacts
            || process.stdout != String::from_utf8_lossy(&process.stdout_bytes)
            || process.stderr != String::from_utf8_lossy(&process.stderr_bytes)
        {
            return Err("original acquisition binding mismatch".into());
        }
        if process.stdin_sha256 != sha256(&serde_json::to_vec(&request).map_err(|e| e.to_string())?)
            || process.stdout_sha256 != sha256(&process.stdout_bytes)
            || process.stderr_sha256 != sha256(&process.stderr_bytes)
            || process.failure.is_some()
            || process.exit_code != 0
            || result.request_sha256 != process.stdin_sha256
        {
            return Err("original acquisition transport mismatch".into());
        }
        validate_originals_with_artifacts(root, &result, transport, artifacts)?;
        if index + 1 == refs.len()
            && (r["request"] != binding["request"]
                || crate::json(&process.stdout_bytes)? != binding["result"])
        {
            return Err("short final producer result mismatch".into());
        }
    }
    Ok(())
}
