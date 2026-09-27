//! Snapshot transport, not dependency discovery or a governance verdict.
use chrono_harness::{copy_hashed, facts, file_identity, json, no_symlink_parents, wire};
pub use chrono_judge_registration::inputs::SNAPSHOT_SCHEMA;
use chrono_judge_registration::{
    Registrations,
    inputs::{Retained, retained_shape, snapshot_shape},
};
use serde_json::{Value, json as value};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};
fn state(root: &Path, path: &str) -> Result<PathBuf, String> {
    if !path.starts_with(".chrono-harness/state/") {
        return Err("E_INPUT_PATH: output/reference must be within host state".into());
    }
    no_symlink_parents(root, path)
}
fn publish(root: &Path, path: &str, v: &Value) -> Result<(), String> {
    let target = state(root, path)?;
    let bytes = wire::canonical(v)?;
    if target.exists() {
        if fs::read(&target).map_err(|e| e.to_string())? == bytes {
            return Ok(());
        }
        return Err("E_SNAPSHOT_EXISTS: retain old snapshot; choose a new output path".into());
    }
    let parent = target.parent().ok_or("snapshot parent")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    tmp.write_all(&bytes).map_err(|e| e.to_string())?;
    tmp.persist_noclobber(&target).map_err(|e| e.to_string())?;
    Ok(())
}
fn store(root: &Path, input: &Path) -> Result<Value, String> {
    let dir = state(root, ".chrono-harness/state/inputs/blobs")?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    if !fs::metadata(input)
        .map_err(|e| format!("E_INPUT_READ: {}: {e}", input.display()))?
        .is_file()
    {
        return Err("E_INPUT_READ: not a regular file".into());
    }
    let file =
        fs::File::open(input).map_err(|e| format!("E_INPUT_READ: {}: {e}", input.display()))?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("E_INPUT_READ: not a regular file".into());
    }
    let mut tmp = tempfile::NamedTempFile::new_in(&dir).map_err(|e| e.to_string())?;
    let (sha256, length) = copy_hashed(file, &mut tmp)?;
    let path = format!(".chrono-harness/state/inputs/blobs/{sha256}");
    let target = state(root, &path)?;
    if target.exists() {
        if file_identity(&target)? != (sha256.clone(), length) {
            return Err("E_INPUT_BLOB: existing content does not match its address".into());
        }
    } else if let Err(error) = tmp.persist_noclobber(&target) {
        if error.error.kind() != std::io::ErrorKind::AlreadyExists
            || file_identity(&target)? != (sha256.clone(), length)
        {
            return Err(format!("E_INPUT_BLOB: {error}"));
        }
    }
    Ok(value!({"blob":path,"sha256":sha256,"length":length}))
}
pub fn capture(root: &Path, config: &str, commit: &str, output: &str) -> Result<Value, String> {
    state(root, output)?;
    facts::verify_oid(root, commit)?;
    if facts::utf8(facts::git(root, &["rev-parse", "HEAD"])?)?.trim() != commit {
        return Err("E_SNAPSHOT_IDENTITY: capture requires the explicit checked-out commit".into());
    }
    let bytes = facts::blob(root, commit, config)?;
    if fs::read(no_symlink_parents(root, config)?).map_err(|e| e.to_string())? != bytes {
        return Err("E_SNAPSHOT_IDENTITY: config differs from the fixed commit".into());
    }
    let values = facts::registry_values(root, commit, config)?;
    let r = Registrations::load(&values, config)?;
    let mut environment = serde_json::Map::new();
    for key in r.config()["environment"]["inherit"].as_array().unwrap() {
        let key = key.as_str().ok_or("invalid inherited variable")?;
        let v = match std::env::var(key) {
            Ok(v) => Value::String(v),
            Err(std::env::VarError::NotPresent) => Value::Null,
            Err(_) => return Err(format!("E_INPUT_ENVIRONMENT: non-UTF8 value for {key}")),
        };
        environment.insert(key.into(), v);
    }
    let mut files = serde_json::Map::new();
    for input in r.config()["environment"]["inputs"].as_array().unwrap() {
        let path = root.join(input["location"].as_str().unwrap());
        files.insert(input["id"].as_str().unwrap().into(), store(root, &path)?);
    }
    let snapshot = value!({"schema":SNAPSHOT_SCHEMA,"commit":commit,"config_path":config,"config_digest":wire::digest(r.config())?,"environment":environment,"files":files});
    publish(root, output, &snapshot)?;
    Ok(snapshot)
}
fn transport(from: &Path, snapshot: &str, to: &Path) -> Result<Value, String> {
    let bytes = fs::read(state(from, snapshot)?).map_err(|e| e.to_string())?;
    let mut v = json(&bytes)?;
    snapshot_shape(&v)?;
    let files = v["files"]
        .as_object_mut()
        .ok_or("E_INPUT_SNAPSHOT: missing files")?;
    for f in files.values_mut() {
        let Retained::Blob { path: blob, .. } = retained_shape(f)? else {
            return Err("E_INPUT_BLOB: pair requires captured blob references".into());
        };
        let path = state(from, blob)?;
        let observed = store(to, &path)?;
        if observed["sha256"] != f["sha256"] || observed["length"] != f["length"] {
            return Err("E_INPUT_BLOB: retained bytes differ from snapshot".into());
        }
        *f = observed;
    }
    Ok(v)
}
pub fn pair(
    root: &Path,
    base_root: &Path,
    base: &str,
    candidate_root: &Path,
    candidate: &str,
    output: &str,
) -> Result<Value, String> {
    state(root, output)?;
    let a = transport(base_root, base, root)?;
    let b = transport(candidate_root, candidate, root)?;
    let v = value!({"base":a,"candidate":b});
    publish(root, output, &v)?;
    Ok(v)
}
pub fn run(cwd: &Path, args: &[String]) -> Result<String, String> {
    if args == ["--version"] {
        return Ok(format!("chrono-inputs {}\n", env!("CARGO_PKG_VERSION")));
    }
    if args == ["--help"] {
        return Ok("chrono-inputs capture --host-root H --config P --commit OID --output P\nchrono-inputs pair --host-root H --base-snapshot P --candidate-snapshot P --output P [--base-root H] [--candidate-root H]\nExplicit input transport only; snapshots are not governance verdicts.\n".into());
    }
    let command = args.first().ok_or("E_USAGE: capture or pair required")?;
    let allowed = match command.as_str() {
        "capture" => vec!["--host-root", "--config", "--commit", "--output"],
        "pair" => vec![
            "--host-root",
            "--base-snapshot",
            "--candidate-snapshot",
            "--output",
            "--base-root",
            "--candidate-root",
        ],
        _ => return Err("E_USAGE: unknown command".into()),
    };
    let mut options = BTreeMap::new();
    let mut chunks = args[1..].chunks_exact(2);
    for chunk in &mut chunks {
        if !allowed.contains(&chunk[0].as_str())
            || options
                .insert(chunk[0].as_str(), chunk[1].as_str())
                .is_some()
        {
            return Err("E_USAGE: unknown or duplicate option".into());
        }
    }
    if !chunks.remainder().is_empty() {
        return Err("E_USAGE: option needs a value".into());
    }
    let required = |key: &str| {
        options
            .get(key)
            .copied()
            .ok_or_else(|| format!("E_USAGE: missing {key}"))
    };
    let root = fs::canonicalize(cwd.join(required("--host-root")?)).map_err(|e| e.to_string())?;
    let result = if command == "capture" {
        capture(
            &root,
            required("--config")?,
            required("--commit")?,
            required("--output")?,
        )?
    } else {
        let location = |key: &str| -> Result<PathBuf, String> {
            match options.get(key) {
                Some(p) => fs::canonicalize(cwd.join(p)).map_err(|e| e.to_string()),
                None => Ok(root.clone()),
            }
        };
        pair(
            &root,
            &location("--base-root")?,
            required("--base-snapshot")?,
            &location("--candidate-root")?,
            required("--candidate-snapshot")?,
            required("--output")?,
        )?
    };
    Ok(serde_json::to_string_pretty(&result).map_err(|e| e.to_string())? + "\n")
}
