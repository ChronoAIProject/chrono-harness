//! Versioned external originals. Logical addresses and original report bytes never change.
use crate::{
    copy_hashed, file_identity, no_symlink_parents,
    prepared::{ArtifactTransport, Original, original_path},
    units, wire,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub const SCHEMA: &str = "chrono-retained-blob/v1";
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Blob {
    schema: String,
    storage: String,
    sha256: String,
    length: u64,
}
fn descriptor(value: &Value) -> Result<Blob, String> {
    let b: Blob = serde_json::from_value(value.clone())
        .map_err(|e| format!("external artifact descriptor: {e}"))?;
    if b.schema != SCHEMA || !wire::is_digest(&b.sha256) {
        return Err("external artifact schema/digest".into());
    }
    units::artifact_path(&b.storage)?;
    Ok(b)
}
fn target(root: &Path, b: &Blob, transport: Option<&ArtifactTransport>) -> Result<PathBuf, String> {
    no_symlink_parents(
        root,
        &original_path(
            &Original {
                path: b.storage.clone(),
                sha256: b.sha256.clone(),
            },
            transport,
        )?,
    )
}
/// Stream into the declared upload root, retaining content identity and deduplicating original bytes.
pub fn stage(root: &Path, input: &Path, directory: &str) -> Result<Value, String> {
    units::artifact_path(directory)?;
    if !directory.ends_with('/') {
        return Err("retained artifact directory must end /".into());
    }
    let dir = no_symlink_parents(root, &format!("{directory}blobs"))?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    if !fs::symlink_metadata(input)
        .map_err(|e| e.to_string())?
        .is_file()
    {
        return Err("original artifact must be a regular file".into());
    }
    let mut tmp = tempfile::NamedTempFile::new_in(&dir).map_err(|e| e.to_string())?;
    let (sha256, length) =
        copy_hashed(fs::File::open(input).map_err(|e| e.to_string())?, &mut tmp)?;
    let storage = format!("{directory}blobs/{sha256}");
    let output = no_symlink_parents(root, &storage)?;
    if output.exists() {
        if file_identity(&output)? != (sha256.clone(), length) {
            return Err("retained artifact address collision".into());
        }
    } else if let Err(e) = tmp.persist_noclobber(&output) {
        if e.error.kind() != std::io::ErrorKind::AlreadyExists
            || file_identity(&output)? != (sha256.clone(), length)
        {
            return Err(e.to_string());
        }
    }
    Ok(json!({"schema":SCHEMA,"storage":storage,"sha256":sha256,"length":length}))
}
/// Resolve nested originals using the enclosing report's explicitly retained storage-address map.
/// This is a projection for reading, not a rewrite of the original report.
pub fn resolve_map(artifacts: &Value, closure: Option<&Value>) -> Result<Value, String> {
    let mut out = artifacts
        .as_object()
        .ok_or("original artifact map missing")?
        .clone();
    for value in out.values_mut() {
        if value.get("schema").is_none() {
            continue;
        }
        let b = descriptor(value)?;
        if let Some(closure) = closure {
            let mapped = closure
                .get(&b.storage)
                .ok_or_else(|| format!("missing nested original artifact {}", b.storage))?;
            let replacement = descriptor(mapped)?;
            if replacement.sha256 != b.sha256 || replacement.length != b.length {
                return Err("nested original artifact identity differs".into());
            }
            *value = mapped.clone();
        }
    }
    Ok(Value::Object(out))
}
pub fn identity(
    root: &Path,
    artifacts: &Value,
    address: &str,
    transport: Option<&ArtifactTransport>,
) -> Result<(String, u64), String> {
    let a = artifacts
        .get(address)
        .ok_or_else(|| format!("missing original artifact {address}"))?;
    if a.get("schema").is_none() {
        let raw = crate::full::artifact_bytes(artifacts, address)?;
        return Ok((crate::sha256(&raw), raw.len() as u64));
    }
    let b = descriptor(a)?;
    let observed = file_identity(&target(root, &b, transport)?)?;
    if observed != (b.sha256.clone(), b.length) {
        return Err(format!("original artifact digest/length: {address}"));
    }
    Ok(observed)
}
/// JSON/process originals remain bounded. Business blobs use identity(), never this allocation path.
pub fn bytes(
    root: &Path,
    artifacts: &Value,
    address: &str,
    transport: Option<&ArtifactTransport>,
) -> Result<Vec<u8>, String> {
    let a = artifacts
        .get(address)
        .ok_or_else(|| format!("missing original artifact {address}"))?;
    if a.get("schema").is_none() {
        return crate::full::artifact_bytes(artifacts, address);
    }
    let b = descriptor(a)?;
    if b.length > 64 * 1024 * 1024 {
        return Err("original JSON/process artifact bound".into());
    }
    let p = target(root, &b, transport)?;
    if !fs::symlink_metadata(&p)
        .map_err(|e| e.to_string())?
        .is_file()
    {
        return Err("original artifact must be regular".into());
    }
    let mut raw = Vec::new();
    fs::File::open(p)
        .map_err(|e| e.to_string())?
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    if raw.len() as u64 != b.length || crate::sha256(&raw) != b.sha256 {
        return Err("original artifact digest/length".into());
    }
    Ok(raw)
}
/// Retain every external reference under a new upload root, keyed by its original storage address.
pub fn restage(
    root: &Path,
    artifacts: &Value,
    transport: Option<&ArtifactTransport>,
    directory: &str,
) -> Result<Value, String> {
    let mut closure = serde_json::Map::new();
    for a in artifacts
        .as_object()
        .ok_or("original artifact map missing")?
        .values()
    {
        if a.get("schema").is_none() {
            continue;
        }
        let b = descriptor(a)?;
        let staged = stage(root, &target(root, &b, transport)?, directory)?;
        if staged["sha256"] != b.sha256 || staged["length"] != b.length {
            return Err("restaged original artifact identity differs".into());
        }
        if closure
            .insert(b.storage, staged.clone())
            .is_some_and(|v| v != staged)
        {
            return Err("nested artifact storage conflict".into());
        }
    }
    Ok(Value::Object(closure))
}
/// Stream an original descriptor to an explicit destination without decoding business bytes.
pub fn copy_to(
    root: &Path,
    artifacts: &Value,
    address: &str,
    transport: Option<&ArtifactTransport>,
    output: &mut impl Write,
) -> Result<(String, u64), String> {
    let a = artifacts.get(address).ok_or("missing original artifact")?;
    if a.get("schema").is_none() {
        return copy_hashed(
            crate::full::artifact_bytes(artifacts, address)?.as_slice(),
            output,
        );
    }
    let b = descriptor(a)?;
    let observed = copy_hashed(
        fs::File::open(target(root, &b, transport)?).map_err(|e| e.to_string())?,
        output,
    )?;
    if observed != (b.sha256, b.length) {
        return Err("original artifact digest/length".into());
    }
    Ok(observed)
}

/// Project a declared outer transport for reading, preserving every original logical key.
pub fn transport_map(
    artifacts: &Value,
    transport: Option<&ArtifactTransport>,
) -> Result<Value, String> {
    let mut out = artifacts
        .as_object()
        .ok_or("original artifact map missing")?
        .clone();
    for a in out.values_mut() {
        if a.get("schema").is_none() {
            continue;
        }
        let mut b = descriptor(a)?;
        b.storage = original_path(
            &Original {
                path: b.storage,
                sha256: b.sha256.clone(),
            },
            transport,
        )?;
        *a = serde_json::to_value(b).map_err(|e| e.to_string())?;
    }
    Ok(Value::Object(out))
}
