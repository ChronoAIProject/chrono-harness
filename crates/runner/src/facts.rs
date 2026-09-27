//! Immutable Git facts. Admissibility belongs to the consuming judge.
use crate::{
    json, no_symlink_parents, relative_path,
    wire::{Checkout, Delta, IndexFlag},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json as value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    process::Command,
};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub mode: String,
    pub oid: String,
    pub kind: String,
}
pub type Tree = BTreeMap<String, Entry>;
pub fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let o = Command::new("git")
        .arg("--no-optional-locks")
        .arg("--no-replace-objects")
        .arg("-C")
        .arg(root)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .map_err(|e| e.to_string())?;
    if !o.status.success() {
        return Err(format!(
            "E_HISTORY_MISSING: git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&o.stderr)
        ));
    }
    Ok(o.stdout)
}
pub fn utf8(b: Vec<u8>) -> Result<String, String> {
    String::from_utf8(b).map_err(|_| "E_INPUT_UNSUPPORTED: non UTF-8 Git output".into())
}
pub fn full_oid(s: &str) -> Result<(), String> {
    if !matches!(s.len(), 40 | 64)
        || s.bytes()
            .any(|b| !b.is_ascii_digit() && !(b'a'..=b'f').contains(&b))
        || s.bytes().all(|b| b == b'0')
    {
        return Err("expected full nonzero lowercase commit OID".into());
    }
    Ok(())
}
pub fn verify_oid(root: &Path, oid: &str) -> Result<String, String> {
    full_oid(oid)?;
    if utf8(git(
        root,
        &["rev-parse", "--verify", &format!("{oid}^{{commit}}")],
    )?)?
    .trim()
        != oid
    {
        return Err("OID is not a commit".into());
    }
    Ok(
        utf8(git(root, &["rev-parse", &format!("{oid}^{{tree}}")])?)?
            .trim()
            .into(),
    )
}
pub fn blob(root: &Path, oid: &str, path: &str) -> Result<Vec<u8>, String> {
    relative_path(path)?;
    git(root, &["show", &format!("{oid}:{path}")])
}
/// Read original commit headers, including parents hidden by a shallow boundary.
pub fn parents(root: &Path, oid: &str) -> Result<Vec<String>, String> {
    full_oid(oid)?;
    let bytes = git(root, &["cat-file", "commit", oid])?;
    let headers = bytes
        .split(|b| *b == b'\n')
        .take_while(|line| !line.is_empty());
    let mut parents = vec![];
    for line in headers {
        if let Some(oid) = line.strip_prefix(b"parent ") {
            let oid = std::str::from_utf8(oid).map_err(|_| "invalid parent OID")?;
            full_oid(oid)?;
            parents.push(oid.into());
        }
    }
    Ok(parents)
}
pub fn tree(root: &Path, oid: &str) -> Result<Tree, String> {
    let raw = git(root, &["ls-tree", "-rz", "--full-tree", oid])?;
    let mut out = Tree::new();
    for row in raw.split(|b| *b == 0).filter(|v| !v.is_empty()) {
        let s = std::str::from_utf8(row).map_err(|_| "E_INPUT_UNSUPPORTED: non UTF-8 tree path")?;
        let (meta, path) = s.split_once('\t').ok_or("invalid Git tree row")?;
        let f: Vec<_> = meta.split(' ').collect();
        if f.len() != 3 {
            return Err("invalid Git tree metadata".into());
        }
        relative_path(path)?;
        out.insert(
            path.into(),
            Entry {
                mode: f[0].into(),
                kind: f[1].into(),
                oid: f[2].into(),
            },
        );
    }
    Ok(out)
}
pub fn delta(a: &Tree, b: &Tree) -> Vec<Delta> {
    a.keys()
        .chain(b.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|p| a.get(*p) != b.get(*p))
        .map(|p| {
            let old = a.get(p);
            let new = b.get(p);
            Delta {
                kind: if old.is_none() {
                    "A"
                } else if new.is_none() {
                    "D"
                } else {
                    "M"
                }
                .into(),
                path: p.clone(),
                old_blob: old.map(|e| e.oid.clone()),
                new_blob: new.map(|e| e.oid.clone()),
                old_mode: old.map(|e| e.mode.clone()),
                new_mode: new.map(|e| e.mode.clone()),
            }
        })
        .collect()
}
fn paths(raw: Vec<u8>) -> Result<Vec<String>, String> {
    raw.split(|b| *b == 0)
        .filter(|v| !v.is_empty())
        .map(|v| utf8(v.to_vec()))
        .collect()
}
pub fn index_flags(root: &Path) -> Result<Vec<IndexFlag>, String> {
    git(root, &["ls-files", "-v", "-z"])?
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|row| {
            if row.len() < 3 || row[1] != b' ' || !row[0].is_ascii_alphabetic() {
                return Err("invalid Git index flag record".into());
            }
            Ok(IndexFlag {
                path: utf8(row[2..].to_vec())?,
                tag: (row[0] as char).to_string(),
            })
        })
        .collect()
}
pub fn checkout(root: &Path, candidate: &str) -> Result<Checkout, String> {
    let head = utf8(git(root, &["rev-parse", "HEAD"])?)?.trim().into();
    let mut tracked = paths(git(
        root,
        &[
            "diff",
            "--no-ext-diff",
            "--name-only",
            "-z",
            candidate,
            "--",
        ],
    )?)?;
    tracked.extend(paths(git(
        root,
        &[
            "diff",
            "--no-ext-diff",
            "--cached",
            "--name-only",
            "-z",
            candidate,
            "--",
        ],
    )?)?);
    tracked.sort();
    tracked.dedup();
    let mut untracked = paths(git(
        root,
        &["ls-files", "--others", "--exclude-standard", "-z"],
    )?)?;
    untracked.extend(paths(git(
        root,
        &[
            "ls-files",
            "--others",
            "--ignored",
            "--exclude-standard",
            "-z",
        ],
    )?)?);
    untracked.sort();
    untracked.dedup();
    Ok(Checkout {
        head,
        tracked,
        untracked,
        index_flags: index_flags(root)?,
    })
}
/// Export immutable bytes, never run base code; supported files are read-only.
pub fn export(root: &Path, oid: &str, t: &Tree, dest: &Path) -> Result<(), String> {
    for (path, e) in t {
        if e.kind != "blob" {
            continue;
        }
        let p = no_symlink_parents(dest, path)?;
        fs::create_dir_all(p.parent().ok_or("snapshot parent")?).map_err(|e| e.to_string())?;
        let bytes = blob(root, oid, path)?;
        if e.mode == "120000" {
            #[cfg(unix)]
            std::os::unix::fs::symlink(utf8(bytes)?, &p).map_err(|e| e.to_string())?;
            #[cfg(not(unix))]
            return Err("E_INPUT_UNSUPPORTED: symlink snapshot".into());
        } else {
            fs::write(&p, bytes).map_err(|e| e.to_string())?;
            let mut perm = fs::metadata(&p).map_err(|e| e.to_string())?.permissions();
            perm.set_readonly(true);
            fs::set_permissions(&p, perm).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
pub fn registry_paths(config: &Value, path: &str) -> Result<Vec<String>, String> {
    let mut paths = vec![path.to_string()];
    for k in ["judges", "projects", "filemap", "workflow"] {
        let p = config["registries"][k]
            .as_str()
            .ok_or_else(|| format!("missing registry path {k}"))?;
        relative_path(p)?;
        paths.push(p.into());
    }
    Ok(paths)
}
pub fn registry_values(
    root: &Path,
    oid: &str,
    path: &str,
) -> Result<BTreeMap<String, Value>, String> {
    Ok(registry_snapshot(root, oid, path)?.values)
}
/// Original bytes and parsed values from one registry read, without a global cache.
pub struct RegistrySnapshot {
    pub bytes: BTreeMap<String, Vec<u8>>,
    pub values: BTreeMap<String, Value>,
}
pub fn registry_snapshot(root: &Path, oid: &str, path: &str) -> Result<RegistrySnapshot, String> {
    let original = blob(root, oid, path)?;
    let config = json(&original)?;
    let paths = registry_paths(&config, path)?;
    let mut snapshot = RegistrySnapshot {
        bytes: BTreeMap::from([(path.into(), original)]),
        values: BTreeMap::from([(path.into(), config)]),
    };
    for path in paths {
        if !snapshot.bytes.contains_key(&path) {
            let original = blob(root, oid, &path)?;
            let value = json(&original)?;
            snapshot.bytes.insert(path.clone(), original);
            snapshot.values.insert(path, value);
        }
    }
    Ok(snapshot)
}
pub fn registry_digest(
    base: &BTreeMap<String, Value>,
    candidate: &BTreeMap<String, Value>,
) -> Result<String, String> {
    crate::wire::digest(&value!({"base":base,"candidate":candidate}))
}
