//! Compare an explicit Git tree/index inventory with literal filesystem objects.
use crate::{
    facts::{Entry, Tree, full_oid},
    relative_path,
};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, Read},
    os::unix::{
        ffi::OsStrExt,
        fs::{OpenOptionsExt, PermissionsExt},
    },
    path::{Path, PathBuf},
};

fn rows(raw: &[u8]) -> Result<impl Iterator<Item = &str>, String> {
    if !raw.is_empty() && !raw.ends_with(&[0]) {
        return Err("incomplete Git inventory".into());
    }
    Ok(std::str::from_utf8(raw)
        .map_err(|_| "non UTF-8 Git inventory")?
        .split('\0')
        .filter(|r| !r.is_empty()))
}

pub fn tree(raw: &[u8]) -> Result<Tree, String> {
    let mut result = Tree::new();
    for row in rows(raw)? {
        let (meta, path) = row.split_once('\t').ok_or("invalid Git tree row")?;
        let fields: Vec<_> = meta.split(' ').collect();
        if fields.len() != 3 {
            return Err("invalid Git tree metadata".into());
        }
        relative_path(path)?;
        full_oid(fields[2])?;
        let entry = Entry {
            mode: fields[0].into(),
            kind: fields[1].into(),
            oid: fields[2].into(),
        };
        if result.insert(path.into(), entry).is_some() {
            return Err("duplicate Git tree path".into());
        }
    }
    Ok(result)
}

/// Index stages are compared as data; Git filters/configuration never decide equality.
pub fn changes(root: &Path, expected: &Tree, index: &[u8]) -> Result<Vec<String>, String> {
    let mut changed = BTreeSet::new();
    let mut stages = BTreeMap::new();
    for row in rows(index)? {
        let (meta, path) = row.split_once('\t').ok_or("invalid Git index row")?;
        let fields: Vec<_> = meta.split(' ').collect();
        if fields.len() != 3 || !matches!(fields[2], "0" | "1" | "2" | "3") {
            return Err("invalid Git index metadata".into());
        }
        relative_path(path)?;
        full_oid(fields[1])?;
        if stages.insert((path, fields[2]), ()).is_some() {
            return Err("duplicate Git index stage".into());
        }
        if fields[2] != "0"
            || !expected
                .get(path)
                .is_some_and(|e| e.mode == fields[0] && e.oid == fields[1])
        {
            changed.insert(path.to_string());
        }
    }
    for (path, entry) in expected {
        if !stages.contains_key(&(path.as_str(), "0")) || !matches_entry(root, path, entry)? {
            changed.insert(path.clone());
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
        return Err(format!(
            "unsupported checkout entry {name}: {} {}",
            entry.kind, entry.mode
        ));
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
