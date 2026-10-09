//! Observed filesystem identities for an explicitly selected recovery scope.
use chrono_harness::{file_identity, sha256, wire};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json as value};
use std::{
    fs,
    os::unix::{ffi::OsStrExt, fs::PermissionsExt},
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct FileIdentity {
    pub(crate) sha256: String,
    pub(crate) size: u64,
    pub(crate) mode: u32,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Expected {
    pub(crate) gitfile: Option<FileIdentity>,
    pub(crate) metadata: Option<String>,
    pub(crate) visible: String,
}
pub(crate) struct Observation {
    pub(crate) expected: Expected,
    pub(crate) gitfile_bytes: Option<Vec<u8>>,
    pub(crate) visible: Value,
    pub(crate) metadata: Option<Value>,
}
pub(crate) fn pointer(base: &Path, value: &str) -> Result<PathBuf, String> {
    let mut path = PathBuf::new();
    for part in base.join(value).components() {
        match part {
            Component::CurDir => (),
            Component::ParentDir => {
                if !path.pop() {
                    return Err("Git pointer escapes filesystem root".into());
                }
            }
            _ => path.push(part.as_os_str()),
        }
    }
    Ok(path)
}
pub(crate) fn snapshot(root: &Path, exclude_gitfile: bool) -> Result<Value, String> {
    fn visit(root: &Path, path: &Path, exclude: bool, rows: &mut Vec<Value>) -> Result<(), String> {
        let relative = path.strip_prefix(root).map_err(|e| e.to_string())?;
        if exclude && relative == Path::new(".git") {
            return Ok(());
        }
        let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        let mut row = value!({"path_bytes":relative.as_os_str().as_bytes(),"mode":metadata.permissions().mode() & 0o7777});
        if metadata.is_symlink() {
            row["kind"] = value!("symlink");
            row["target_bytes"] = value!(
                fs::read_link(path)
                    .map_err(|e| e.to_string())?
                    .as_os_str()
                    .as_bytes()
            );
        } else if metadata.is_file() {
            let (hash, size) = file_identity(path)?;
            row["kind"] = value!("file");
            row["sha256"] = value!(hash);
            row["size"] = value!(size);
        } else if metadata.is_dir() {
            row["kind"] = value!("directory");
        } else {
            return Err("rebind scope contains an unsupported special file".into());
        }
        rows.push(row);
        if metadata.is_dir() {
            let mut children = fs::read_dir(path)
                .map_err(|e| e.to_string())?
                .map(|e| e.map(|e| e.path()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            children.sort_by(|a, b| a.as_os_str().as_bytes().cmp(b.as_os_str().as_bytes()));
            for child in children {
                visit(root, &child, exclude, rows)?;
            }
        }
        Ok(())
    }
    let mut rows = Vec::new();
    visit(root, root, exclude_gitfile, &mut rows)?;
    let entries = value!(rows);
    Ok(value!({"sha256":wire::digest(&entries)?,"entries":entries}))
}
pub(crate) fn observe(target: &Path, metadata: &Path) -> Result<Observation, String> {
    let gitfile = target.join(".git");
    let (identity, bytes) = match fs::symlink_metadata(&gitfile) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (None, None),
        Err(e) => return Err(e.to_string()),
        Ok(m) if m.is_file() => {
            let bytes = fs::read(&gitfile).map_err(|e| e.to_string())?;
            let mode = m.permissions().mode() & 0o7777;
            (
                Some(FileIdentity {
                    sha256: sha256(&bytes),
                    size: bytes.len() as u64,
                    mode,
                }),
                Some(bytes),
            )
        }
        Ok(_) => {
            return Err(
                "rebind requires an absent or regular .git file, never a directory or symlink"
                    .into(),
            );
        }
    };
    let metadata = match fs::symlink_metadata(metadata) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.to_string()),
        Ok(m) if m.is_dir() => Some(snapshot(metadata, false)?),
        Ok(_) => return Err("selected worktree metadata is not a real directory".into()),
    };
    let visible = snapshot(target, true)?;
    Ok(Observation {
        expected: Expected {
            gitfile: identity,
            metadata: metadata
                .as_ref()
                .map(|v| v["sha256"].as_str().unwrap().to_string()),
            visible: visible["sha256"].as_str().unwrap().to_string(),
        },
        gitfile_bytes: bytes,
        visible,
        metadata,
    })
}

/// Literal optional regular-file identity; absence never includes a dangling link.
pub(crate) fn identity(path: &Path) -> Result<Option<FileIdentity>, String> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
        Ok(m) if m.is_file() => {
            let (sha256, size) = file_identity(path)?;
            Ok(Some(FileIdentity {
                sha256,
                size,
                mode: m.permissions().mode() & 0o7777,
            }))
        }
        Ok(_) => Err(format!(
            "expected an absent or regular file: {}",
            path.display()
        )),
    }
}
