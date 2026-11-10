//! Compare an explicit Git tree/index inventory with literal filesystem objects.
use crate::{
    facts::{Entry, Tree, full_oid},
    relative_path,
    verified_identifiers::{git_inventory_records, git_inventory_termination, git_inventory_text},
    verified_status::{
        GitIndexStage, GitIndexStages, git_index_entry_changed, parse_git_index_row,
        parse_git_tree_rows, record_git_index_stage,
    },
};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::{self, Read},
    os::unix::{
        ffi::OsStrExt,
        fs::{OpenOptionsExt, PermissionsExt},
    },
    path::{Path, PathBuf},
};

fn rows(raw: &[u8]) -> Result<Vec<&str>, String> {
    git_inventory_termination(raw)?;
    Ok(git_inventory_records(git_inventory_text(
        std::str::from_utf8(raw).ok(),
    )?))
}

pub fn tree(raw: &[u8]) -> Result<Tree, String> {
    parse_git_tree_rows(&rows(raw)?)
        .map_err(|error| error.render(|path| format!("invalid relative path: {path:?}")))
}

/// Index stages are compared as data; Git filters/configuration never decide equality.
pub fn changes(root: &Path, expected: &Tree, index: &[u8]) -> Result<Vec<String>, String> {
    let mut changed = BTreeSet::new();
    let mut stages = GitIndexStages::new();
    for raw in rows(index)? {
        let row = parse_git_index_row(raw)
            .map_err(|error| error.render(|path| format!("invalid relative path: {path:?}")))?;
        record_git_index_stage(&mut stages, row.path, row.stage)?;
        if git_index_entry_changed(&row, expected.get(row.path)) {
            changed.insert(row.path.to_string());
        }
    }
    for (path, entry) in expected {
        if !stages.contains_key(&GitIndexStage {
            path: path.as_str(),
            stage: "0",
        }) || !matches_entry(root, path, entry)?
        {
            changed.insert(path.text.clone());
        }
    }
    Ok(changed.into_iter().collect())
}

fn physical(root: &Path, path: &str) -> Result<Option<PathBuf>, String> {
    relative_path(path)?;
    let mut current = root.to_path_buf();
    let mut parts = Path::new(path).components().peekable();
    while let Some(part) = parts.next() {
        current.push(part);
        if parts.peek().is_some() {
            match fs::symlink_metadata(&current) {
                Ok(m) if m.is_dir() => (),
                Ok(_) => return Ok(None),
                Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
                Err(e) => return Err(format!("checkout path {path}: {e}")),
            }
        }
    }
    Ok(Some(current))
}

fn hash<D: Digest>(size: u64, source: &mut impl Read) -> Result<String, String> {
    let mut digest = D::new();
    digest.update(format!("blob {size}\0").as_bytes());
    let mut buffer = [0u8; 65536];
    let mut total = 0u64;
    loop {
        let n = source.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        total = total
            .checked_add(n as u64)
            .ok_or("checkout file size overflow")?;
        if total > size {
            return Err("checkout file grew during observation".into());
        }
        digest.update(&buffer[..n]);
    }
    if total != size {
        return Err("checkout file shrank during observation".into());
    }
    Ok(digest
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}
fn object_hash(oid: &str, size: u64, source: &mut impl Read) -> Result<String, String> {
    full_oid(oid)?;
    match oid.len() {
        40 => hash::<Sha1>(size, source),
        64 => hash::<Sha256>(size, source),
        _ => unreachable!(),
    }
}
fn matches_entry(root: &Path, name: &str, entry: &Entry) -> Result<bool, String> {
    if entry.kind != "blob" || !matches!(entry.mode.as_str(), "100644" | "100755" | "120000") {
        // Unsupported entries never match. Keep observations available so the
        // registration judge can emit its structured unsupported-input finding.
        return Ok(false);
    }
    let Some(path) = physical(root, name)? else {
        return Ok(false);
    };
    let metadata = match fs::symlink_metadata(&path) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(format!("checkout file {name}: {e}")),
    };
    if entry.mode == "120000" {
        if !metadata.is_symlink() {
            return Ok(false);
        }
        let target = fs::read_link(path).map_err(|e| e.to_string())?;
        let bytes = target.as_os_str().as_bytes();
        return Ok(object_hash(&entry.oid, bytes.len() as u64, &mut &bytes[..])? == entry.oid);
    }
    if !metadata.is_file()
        || (metadata.permissions().mode() & 0o100 != 0) != (entry.mode == "100755")
    {
        return Ok(false);
    }
    // Never open a symlink or block on a FIFO substituted after the type observation.
    let mut file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|e| format!("checkout file {name}: {e}"))?;
    let opened = file.metadata().map_err(|e| e.to_string())?;
    if !opened.is_file()
        || opened.len() != metadata.len()
        || opened.permissions().mode() != metadata.permissions().mode()
    {
        return Err(format!(
            "checkout file identity changed while opening: {name}"
        ));
    }
    Ok(object_hash(&entry.oid, opened.len(), &mut file)? == entry.oid)
}
