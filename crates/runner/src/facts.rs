//! Immutable Git facts. Admissibility belongs to the consuming judge.
pub use crate::facts_binding::{
    OpenFailure, Reader, RegistryBlob, acquire_immutable_input, acquire_registry_blobs,
    declaration as git_declaration,
};
pub use crate::facts_configs::Identity as ConfigIdentity;
pub use crate::verified_identifiers::full_oid;
pub use crate::verified_status::{Tree, TreeEntry as Entry, TreePath};
use crate::{
    json, no_symlink_parents, relative_path,
    wire::{Checkout, Delta, IndexFlag},
};
use serde_json::{Value, json as value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    process::Command,
};

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
    crate::verified_paths::literal_artifact_directories(artifacts)
        .into_iter()
        .map(|path| format!(":(top,exclude,literal){path}"))
        .collect()
}
/// Read explicit host declarations; ownership and schema admission stay with
/// registration. This never discovers output directories or dependency edges.
pub fn artifact_directories(config: &Value) -> Result<Vec<&str>, String> {
    let rows = config["artifacts"].as_array().map(Vec::as_slice);
    crate::verified_paths::collect_artifact_declarations(rows, |artifact| artifact["path"].as_str())
        .map_err(|error| error.message().to_owned())
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
    crate::verified_status::git_output_text(String::from_utf8(b).ok())
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
/// Parse physical commit headers, without revision-walk shallow semantics.
pub fn commit_parents(bytes: &[u8]) -> Result<Vec<String>, String> {
    crate::verified_identifiers::parse_commit_parents(bytes, |record| {
        std::str::from_utf8(record).ok()
    })
    .map_err(crate::verified_identifiers::CommitParentError::into_message)
}
pub fn tree(root: &Path, oid: &str) -> Result<Tree, String> {
    Reader::legacy().tree(root, oid)
}
pub fn delta(a: &Tree, b: &Tree) -> Vec<Delta> {
    crate::verified_status::delta_for_trees(a, b)
}
fn paths(raw: Vec<u8>) -> Result<Vec<String>, String> {
    let records = crate::verified_status::git_nul_records(&raw);
    let decoded = records
        .into_iter()
        .map(|record| String::from_utf8(record.to_vec()).ok())
        .collect();
    crate::verified_status::parse_git_text_rows(decoded)
}
pub fn checkout(root: &Path, candidate: &str) -> Result<Checkout, String> {
    Reader::legacy().checkout(root, candidate)
}
pub fn registry_paths(config: &Value, path: &str) -> Result<Vec<String>, String> {
    let paths = [
        config["registries"]["judges"].as_str(),
        config["registries"]["projects"].as_str(),
        config["registries"]["filemap"].as_str(),
        config["registries"]["workflow"].as_str(),
    ];
    crate::verified_status::collect_registry_paths(path, &paths)
        .map_err(|error| error.render(|invalid| format!("invalid relative path: {invalid:?}")))
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
/// Assemble one fixed registry snapshot through the caller's blob reader.
/// The caller binds acquisition to its immutable endpoint and retains any
/// process observations; this helper performs no disk or Git acquisition.
pub fn registry_snapshot_with(
    path: &str,
    read: impl FnMut(&str) -> Result<Vec<u8>, String>,
) -> Result<RegistrySnapshot, String> {
    registry_snapshot_with_target(path, None, read)
}
fn registry_snapshot_with_target(
    path: &str,
    target: Option<crate::facts_configs::RegistryTarget<'_>>,
    mut read: impl FnMut(&str) -> Result<Vec<u8>, String>,
) -> Result<RegistrySnapshot, String> {
    let (mut snapshot, paths) = registry_snapshot_start(path, target, &mut read)?;
    for path in paths {
        if !snapshot.bytes.contains_key(&path) {
            let original = read(&path)?;
            let value = json(&original)?;
            snapshot.bytes.insert(path.clone(), original);
            snapshot.values.insert(path, value);
        }
    }
    Ok(snapshot)
}
/// Resolve the entry and selector before acquiring their explicit registry list.
/// The reader must return the requested paths in order with their original bytes.
pub fn registry_snapshot_with_batches(
    path: &str,
    read: impl FnMut(&[String]) -> Result<Vec<(String, Vec<u8>)>, String>,
) -> Result<RegistrySnapshot, String> {
    registry_snapshot_with_batches_target(path, None, read)
}
fn registry_snapshot_with_batches_target(
    path: &str,
    target: Option<crate::facts_configs::RegistryTarget<'_>>,
    mut read: impl FnMut(&[String]) -> Result<Vec<(String, Vec<u8>)>, String>,
) -> Result<RegistrySnapshot, String> {
    let (mut snapshot, paths) = registry_snapshot_start(path, target, |path| {
        let originals = read(&[path.to_owned()])?;
        crate::verified_status::single_registry_blob(path, originals)
    })?;
    let mut pending = vec![];
    for path in paths {
        if !snapshot.bytes.contains_key(&path) && !pending.contains(&path) {
            pending.push(path);
        }
    }
    let originals = read(&pending)?;
    crate::verified_status::registry_acquisition_count(pending.len(), originals.len())?;
    for (expected, (path, original)) in pending.iter().zip(originals) {
        crate::verified_status::registry_acquisition_path(expected, &path)?;
        let value = json(&original)?;
        snapshot.bytes.insert(path.clone(), original);
        snapshot.values.insert(path, value);
    }
    Ok(snapshot)
}
fn registry_snapshot_start(
    path: &str,
    target: Option<crate::facts_configs::RegistryTarget<'_>>,
    mut read: impl FnMut(&str) -> Result<Vec<u8>, String>,
) -> Result<(RegistrySnapshot, Vec<String>), String> {
    let original = read(path)?;
    let (effective_path, effective_bytes, effective_config, selection) =
        crate::facts_configs::resolve_target(path, original.clone(), target, &mut read)?;
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
    Ok((snapshot, paths))
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
        let tree = utf8(self.git(root, &["rev-parse", &format!("{oid}^{{tree}}")])?)?
            .trim()
            .to_string();
        if full_oid(&tree).is_ok() {
            self.record_verified_oid(oid);
        }
        Ok(tree)
    }
    pub fn blob(&self, root: &Path, oid: &str, path: &str) -> Result<Vec<u8>, String> {
        relative_path(path)?;
        self.blob_with_reuse(root, oid, path)
    }
    pub fn parents(&self, root: &Path, oid: &str) -> Result<Vec<String>, String> {
        full_oid(oid)?;
        let bytes = self.git(root, &["cat-file", "commit", oid])?;
        commit_parents(&bytes)
    }
    pub fn tree(&self, root: &Path, oid: &str) -> Result<Tree, String> {
        self.tree_with_reuse(root, oid)
    }
    pub fn tracked_changes(&self, root: &Path, candidate: &str) -> Result<Vec<String>, String> {
        let tree = self.tree(root, candidate)?;
        let index = self.git(root, &["ls-files", "--stage", "-z"])?;
        checkout_changes(root, &tree, &index)
    }
    pub fn index_flags(&self, root: &Path) -> Result<Vec<IndexFlag>, String> {
        let raw = self.git(root, &["ls-files", "-v", "-z"])?;
        let records = crate::verified_status::git_nul_records(&raw);
        let observations: Vec<_> = records
            .into_iter()
            .map(|raw| crate::verified_status::GitIndexFlagObservation {
                raw,
                text: std::str::from_utf8(raw).ok(),
            })
            .collect();
        crate::verified_status::parse_git_index_flag_rows(&observations)
    }
    pub fn checkout(&self, root: &Path, candidate: &str) -> Result<Checkout, String> {
        let head = utf8(self.git(root, &["rev-parse", "HEAD"])?)?.trim().into();
        let tracked = self.tracked_changes(root, candidate)?;
        let untracked =
            paths(self.git(root, &["ls-files", "--others", "--exclude-standard", "-z"])?)?;
        let ignored = paths(self.git(
            root,
            &[
                "ls-files",
                "--others",
                "--ignored",
                "--exclude-standard",
                "-z",
            ],
        )?)?;
        let untracked = crate::verified_status::normalize_checkout_paths(untracked, ignored);
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
        let paths = paths(self.literal_inventory(root, &args)?)?;
        Ok(crate::verified_status::normalize_checkout_paths(
            paths,
            Vec::new(),
        ))
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
    /// Preserve the exact source bytes while enforcing the output owner's reservation before writes.
    pub fn export_limited(
        &self,
        root: &Path,
        oid: &str,
        t: &Tree,
        dest: &Path,
        max_bytes: u64,
    ) -> Result<(), String> {
        if self.can_reuse_oid(oid) {
            let paths = t
                .iter()
                .filter(|(_, entry)| entry.kind == "blob")
                .map(|(path, _)| path.text.clone())
                .collect::<Vec<_>>();
            self.immutable_blobs(root, oid, &paths)?;
        }
        let mut written = 0u64;
        for (path, e) in t {
            if e.kind != "blob" {
                continue;
            }
            let p = no_symlink_parents(dest, path)?;
            fs::create_dir_all(p.parent().ok_or("snapshot parent")?).map_err(|e| e.to_string())?;
            let bytes = self.blob(root, oid, path)?;
            written = written
                .checked_add(bytes.len() as u64)
                .ok_or("snapshot byte accounting overflow")?;
            if written > max_bytes {
                return Err("snapshot output byte capacity exceeded before writing".into());
            }
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
        let target = self.registry_target(path);
        if !self.can_reuse_oid(oid) {
            return registry_snapshot_with_target(path, target, |path| self.blob(root, oid, path));
        }
        registry_snapshot_with_batches_target(path, target, |paths| {
            self.immutable_blobs(root, oid, paths)
        })
    }
}
