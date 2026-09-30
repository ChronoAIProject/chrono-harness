//! Immutable Git facts. Admissibility belongs to the consuming judge.
pub use crate::facts_configs::Identity as ConfigIdentity;
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
pub use crate::facts_binding::{OpenFailure, Reader, declaration as git_declaration};

pub type Tree = BTreeMap<String, Entry>;
pub use crate::checkout::{changes as checkout_changes, tree as parse_tree};
/// Inventory uses literal, case-sensitive registered directory prefixes.
pub const LITERAL_PATHSPEC_CONTROLS: [&str; 4] = [
    "GIT_LITERAL_PATHSPECS",
    "GIT_GLOB_PATHSPECS",
    "GIT_NOGLOB_PATHSPECS",
    "GIT_ICASE_PATHSPECS",
];
/// Do not normalize a declaration into a broader exclusion. Callers retain
/// schema validation and classify any paths that the inventory returns.
pub fn artifact_exclusions(artifacts: &[&str]) -> Vec<String> {
    artifacts
        .iter()
        .filter(|path| {
            path.strip_suffix('/').is_some_and(|directory| {
                directory
                    .split('/')
                    .all(|part| !matches!(part, "" | "." | ".."))
            })
        })
        .map(|path| format!(":(top,exclude,literal){path}"))
        .collect()
}
/// Read explicit host declarations; ownership and schema admission stay with
/// registration. This never discovers output directories or dependency edges.
pub fn artifact_directories(config: &Value) -> Result<Vec<&str>, String> {
    config["artifacts"]
        .as_array()
        .ok_or("missing artifact declarations")?
        .iter()
        .map(|artifact| {
            artifact["path"]
                .as_str()
                .ok_or("invalid artifact path".into())
        })
        .collect()
}
pub fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    git_inventory(root, args, false)
}
pub(crate) fn git_inventory(
    root: &Path,
    args: &[&str],
    literal_inventory: bool,
) -> Result<Vec<u8>, String> {
    let mut command = Command::new("git");
    command
        .arg("--no-optional-locks")
        .arg("--no-replace-objects")
        .arg("-C")
        .arg(root)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE");
    if literal_inventory {
        for key in LITERAL_PATHSPEC_CONTROLS {
            command.env(key, "0");
        }
    }
    let o = command.output().map_err(|e| e.to_string())?;
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
    Reader::legacy().verify_oid(root, oid)
}
pub fn blob(root: &Path, oid: &str, path: &str) -> Result<Vec<u8>, String> {
    Reader::legacy().blob(root, oid, path)
}
/// Read original commit headers, including parents hidden by a shallow boundary.
pub fn parents(root: &Path, oid: &str) -> Result<Vec<String>, String> {
    Reader::legacy().parents(root, oid)
}
pub fn tree(root: &Path, oid: &str) -> Result<Tree, String> {
    Reader::legacy().tree(root, oid)
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
    Reader::legacy().index_flags(root)
}
pub fn checkout(root: &Path, candidate: &str) -> Result<Checkout, String> {
    Reader::legacy().checkout(root, candidate)
}
/// Export immutable bytes, never run base code; supported files are read-only.
pub fn export(root: &Path, oid: &str, t: &Tree, dest: &Path) -> Result<(), String> {
    Reader::legacy().export(root, oid, t, dest)
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
    Reader::legacy().registry_values(root, oid, path)
}
/// Original bytes and parsed values from one registry read, without a global cache.
pub struct RegistrySnapshot {
    pub bytes: BTreeMap<String, Vec<u8>>,
    pub values: BTreeMap<String, Value>,
    /// The caller's stable entry and the direct endpoint used for policy.
    pub entry_path: String,
    pub effective_path: String,
    pub selection: Option<Value>,
}
pub fn registry_snapshot(root: &Path, oid: &str, path: &str) -> Result<RegistrySnapshot, String> {
    Reader::legacy().registry_snapshot(root, oid, path)
}
pub fn registry_digest(
    base: &BTreeMap<String, Value>,
    candidate: &BTreeMap<String, Value>,
) -> Result<String, String> {
    crate::wire::digest(&value!({"base":base,"candidate":candidate}))
}

/// Resolve the effective policy identity from a fixed registry map while
/// retaining the caller's entry path.  Consumers use this after snapshot
/// acquisition to bind views and provenance to the same selector decision.
pub fn registry_identity(
    values: &BTreeMap<String, Value>,
    path: &str,
) -> Result<ConfigIdentity, String> {
    crate::facts_configs::identity(values, path)
}

impl Reader {
    pub fn verify_oid(&self, root: &Path, oid: &str) -> Result<String, String> {
        full_oid(oid)?;
        if utf8(self.git(
            root,
            &["rev-parse", "--verify", &format!("{oid}^{{commit}}")],
        )?)?
        .trim()
            != oid
        {
            return Err("OID is not a commit".into());
        }
        Ok(
            utf8(self.git(root, &["rev-parse", &format!("{oid}^{{tree}}")])?)?
                .trim()
                .into(),
        )
    }
    pub fn blob(&self, root: &Path, oid: &str, path: &str) -> Result<Vec<u8>, String> {
        relative_path(path)?;
        self.git(root, &["show", &format!("{oid}:{path}")])
    }
    pub fn parents(&self, root: &Path, oid: &str) -> Result<Vec<String>, String> {
        full_oid(oid)?;
        let bytes = self.git(root, &["cat-file", "commit", oid])?;
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
    pub fn tree(&self, root: &Path, oid: &str) -> Result<Tree, String> {
        let raw = self.git(root, &["ls-tree", "-rz", "--full-tree", oid])?;
        parse_tree(&raw)
    }
    pub fn tracked_changes(&self, root: &Path, candidate: &str) -> Result<Vec<String>, String> {
        let tree = self.tree(root, candidate)?;
        let index = self.git(root, &["ls-files", "--stage", "-z"])?;
        checkout_changes(root, &tree, &index)
    }
    pub fn index_flags(&self, root: &Path) -> Result<Vec<IndexFlag>, String> {
        self.git(root, &["ls-files", "-v", "-z"])?
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
    pub fn checkout(&self, root: &Path, candidate: &str) -> Result<Checkout, String> {
        let head = utf8(self.git(root, &["rev-parse", "HEAD"])?)?.trim().into();
        let tracked = self.tracked_changes(root, candidate)?;
        let mut untracked =
            paths(self.git(root, &["ls-files", "--others", "--exclude-standard", "-z"])?)?;
        untracked.extend(paths(self.git(
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
            index_flags: self.index_flags(root)?,
        })
    }
    pub fn untracked_excluding(
        &self,
        root: &Path,
        artifacts: &[&str],
    ) -> Result<Vec<String>, String> {
        let exclusions = artifact_exclusions(artifacts);
        let mut args = vec!["ls-files", "--others", "-z", "--", "."];
        args.extend(exclusions.iter().map(String::as_str));
        let mut paths = paths(self.literal_inventory(root, &args)?)?;
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
    pub fn checkout_excluding(
        &self,
        root: &Path,
        candidate: &str,
        artifacts: &[&str],
    ) -> Result<Checkout, String> {
        Ok(Checkout {
            head: utf8(self.git(root, &["rev-parse", "HEAD"])?)?.trim().into(),
            tracked: self.tracked_changes(root, candidate)?,
            untracked: self.untracked_excluding(root, artifacts)?,
            index_flags: self.index_flags(root)?,
        })
    }
    pub fn export(&self, root: &Path, oid: &str, t: &Tree, dest: &Path) -> Result<(), String> {
        for (path, e) in t {
            if e.kind != "blob" {
                continue;
            }
            let p = no_symlink_parents(dest, path)?;
            fs::create_dir_all(p.parent().ok_or("snapshot parent")?).map_err(|e| e.to_string())?;
            let bytes = self.blob(root, oid, path)?;
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
    pub fn registry_values(
        &self,
        root: &Path,
        oid: &str,
        path: &str,
    ) -> Result<BTreeMap<String, Value>, String> {
        Ok(self.registry_snapshot(root, oid, path)?.values)
    }
    pub fn registry_snapshot(
        &self,
        root: &Path,
        oid: &str,
        path: &str,
    ) -> Result<RegistrySnapshot, String> {
        let original = self.blob(root, oid, path)?;
        let (effective_path, effective_bytes, effective_config, selection) =
            crate::facts_configs::resolve(path, original.clone(), |target| {
                self.blob(root, oid, target)
            })?;
        let config = json(&original)?;
        let paths = registry_paths(&effective_config, &effective_path)?;
        let mut snapshot = RegistrySnapshot {
            bytes: BTreeMap::from([(path.into(), original)]),
            values: BTreeMap::from([(path.into(), config)]),
            entry_path: path.into(),
            effective_path: effective_path.clone(),
            selection: selection.map(|s| {
                let mut observation = s.observation;
                if let Some(object) = observation.as_object_mut() {
                    object.remove("sha256");
                    object.insert("schema".into(), value!("chrono-registry-selection/v1"));
                }
                observation
            }),
        };
        if effective_path != path {
            snapshot
                .bytes
                .insert(effective_path.clone(), effective_bytes.clone());
            snapshot.values.insert(effective_path, effective_config);
        }
        for path in paths {
            if !snapshot.bytes.contains_key(&path) {
                let original = self.blob(root, oid, &path)?;
                let value = json(&original)?;
                snapshot.bytes.insert(path.clone(), original);
                snapshot.values.insert(path, value);
            }
        }
        Ok(snapshot)
    }
}
