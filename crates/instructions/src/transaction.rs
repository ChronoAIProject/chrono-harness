use std::fs::{self, Permissions};
use std::path::{Path, PathBuf};

pub(super) struct Snapshot {
    pub bytes: Vec<u8>,
    pub permissions: Permissions,
    pub identity: String,
}

pub(super) struct Change {
    pub path: PathBuf,
    pub before: Option<Snapshot>,
    pub after: After,
    pub permissions: Option<Permissions>,
}

pub(super) enum After {
    Regular(Vec<u8>),
    RelativeAlias,
}

impl Change {
    pub fn new(path: PathBuf, before: Option<Snapshot>, after: Vec<u8>) -> Self {
        let permissions = before.as_ref().map(|s| s.permissions.clone());
        Self {
            path,
            before,
            after: After::Regular(after),
            permissions,
        }
    }

    pub fn alias(path: PathBuf, before: Option<Snapshot>) -> Self {
        Self {
            path,
            before,
            after: After::RelativeAlias,
            permissions: None,
        }
    }

    pub fn unchanged(&self) -> bool {
        matches!((&self.before, &self.after), (Some(before), After::Regular(after)) if before.bytes == *after)
    }
}

use crate::publication_decision::{IntentLayer, Repair, can_release, intent_layer, record_bound, repair};
use chrono_harness::{
    artifact_disposal::{identity, publish_entry},
    decode, no_symlink_parents,
    ownership::DirectoryLease,
    retained_artifacts::retention::{Inventory, unique_path},
    sha256,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::os::unix::{ffi::{OsStrExt, OsStringExt}, fs::{MetadataExt, OpenOptionsExt, PermissionsExt, symlink}};
use std::time::{SystemTime, UNIX_EPOCH};

const STORE: &str = ".chrono-harness/state/instructions-operations";
const PRODUCER: &str = "instructions-operation";
const ACTIVE: &str = "instructions-publication-recovery";
const POLICY: &[u8] = include_bytes!("../../../assets/instructions/standalone-retention.json");
const LIMIT: u64 = 8 * 1024 * 1024;

#[derive(Default)]
pub(super) struct Fault {
    pub after: Option<usize>,
    pub rollback: Option<usize>,
    pub checkpoint: Option<fn(&str)>,
    pub fail: Option<String>,
}

impl Fault {
    fn at(&self, point: &str) -> Result<(), String> {
        if let Some(hook) = self.checkpoint {
            hook(point);
        }
        if self.fail.as_deref() == Some(point) { return Err(format!("injected {point} state/cleanup failure")); }
        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    host: String,
    attempt: String,
    directories: Vec<Entry>,
    files: Vec<Entry>,
    parents: BTreeMap<String, Object>,
    intent_sha256: String,
    bindings: Bindings,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Bindings {
    inputs: BTreeMap<String, Object>,
    unchanged: BTreeMap<String, Option<Object>>,
    pub external: Vec<ExternalInput>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExternalInput {
    requested: Vec<u8>,
    resolved: Vec<u8>,
    identity: String,
    sha256: String,
    mode: u32,
}

pub(super) fn external_input(path: &Path) -> Result<(Vec<u8>, ExternalInput), String> {
    let requested = if path.is_absolute() { path.to_path_buf() } else {
        std::env::current_dir().map_err(|e| e.to_string())?.join(path)
    };
    let resolved = fs::canonicalize(&requested).map_err(|e| super::error(path, e))?;
    let meta = fs::metadata(&resolved).map_err(|e| super::error(path, e))?;
    if !meta.is_file() { return Err(super::error(path, "expected a readable regular input file")); }
    let id = identity(&resolved)?;
    let bytes = fs::read(&resolved).map_err(|e| super::error(path, e))?;
    if identity(&resolved)? != id || stamp(&fs::metadata(&resolved).map_err(|e| e.to_string())?) != stamp(&meta) {
        return Err(super::error(path, "external input changed while reading"));
    }
    let binding = ExternalInput {
        requested: requested.as_os_str().as_bytes().to_vec(),
        resolved: resolved.as_os_str().as_bytes().to_vec(),
        identity: id, sha256: sha256(&bytes), mode: mode(&meta),
    };
    Ok((bytes, binding))
}

pub(super) fn bind_inputs(
    root: &Path, inputs: BTreeMap<String, String>, unchanged: Vec<&Change>, alias: bool,
) -> Result<Bindings, String> {
    let mut bound = Bindings::default();
    for (name, digest) in inputs {
        let path = target(root, &name)?;
        let object = observe(&path)?.ok_or_else(|| super::error(&path, "registered input disappeared"))?;
        if object.bytes.as_ref() != Some(&digest) {
            return Err(super::error(&path, "registered input changed during preparation"));
        }
        bound.inputs.insert(name, object);
    }
    for change in unchanged {
        let expected = change.before.as_ref().map(|b| Object {
            identity: b.identity.clone(), bytes: Some(sha256(&b.bytes)), mode: b.permissions.mode() & 0o777,
        });
        if observe(&change.path)? != expected {
            return Err(super::error(&change.path, "unchanged destination changed during preparation"));
        }
        bound.unchanged.insert(relative(root, &change.path)?, expected);
    }
    if alias {
        let path = root.join("AGENTS.md");
        let expected = observe(&path)?.ok_or("literal alias disappeared during preparation")?;
        if !fs::symlink_metadata(&path).map_err(|e| e.to_string())?.is_symlink()
            || expected.bytes.as_ref() != Some(&sha256(b"CLAUDE.md")) {
            return Err(super::error(&path, "literal alias changed during preparation"));
        }
        bound.unchanged.insert("AGENTS.md".into(), Some(expected));
    }
    Ok(bound)
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Commit {
    attempt: String,
    host: String,
    plan_sha256: String,
}

fn digest<T: Serialize>(value: &T) -> Result<String, String> {
    Ok(sha256(&serde_json::to_vec(value).map_err(|e| e.to_string())?))
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    target: String,
    before: Option<Object>,
    next: Object,
    backup: Option<Object>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Object {
    identity: String,
    bytes: Option<String>,
    mode: u32,
}

fn mode(meta: &fs::Metadata) -> u32 {
    meta.mode() & 0o777
}

fn stamp(meta: &fs::Metadata) -> (u64, i64, i64, i64, i64, u32, u64) {
    (meta.len(), meta.mtime(), meta.mtime_nsec(), meta.ctime(), meta.ctime_nsec(), meta.mode(), meta.nlink())
}

fn observe(path: &Path) -> Result<Option<Object>, String> {
    let Some(meta) = super::metadata(path)? else { return Ok(None) };
    let id = identity(path)?;
    let bytes = if meta.is_file() && meta.nlink() == 1 {
        Some(sha256(&fs::read(path).map_err(|e| super::error(path, e))?))
    } else if meta.is_symlink() {
        Some(sha256(fs::read_link(path).map_err(|e| super::error(path, e))?.as_os_str().as_bytes()))
    } else if meta.is_dir() {
        None
    } else {
        return Err(super::error(path, "unexpected publication object type or hard link"));
    };
    if identity(path)? != id || (!meta.is_dir() && stamp(&fs::symlink_metadata(path).map_err(|e| e.to_string())?) != stamp(&meta)) {
        return Err(super::error(path, "publication object identity/content changed during read"));
    }
    Ok(Some(Object { identity: id, bytes, mode: mode(&meta) }))
}

fn timestamp() -> Result<u64, String> {
    SystemTime::now().duration_since(UNIX_EPOCH)
        .map(|time| time.as_secs()).map_err(|e| e.to_string())
}

fn sync(path: &Path) -> Result<(), String> {
    fs::File::open(path).and_then(|f| f.sync_all()).map_err(|e| super::error(path, e))
}

fn write(path: &Path, bytes: &[u8], mode: u32) -> Result<(), String> {
    let mut f = fs::OpenOptions::new().write(true).create_new(true).mode(0o600)
        .open(path).map_err(|e| super::error(path, e))?;
    f.write_all(bytes).and_then(|()| f.set_permissions(Permissions::from_mode(mode)))
        .and_then(|()| f.sync_all()).map_err(|e| super::error(path, e))
}

fn record<T: Serialize>(directory: &Path, name: &str, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > LIMIT {
        return Err(format!("operation record {name} exceeds byte limit"));
    }
    let temporary = directory.join(format!("{name}.writing"));
    write(&temporary, &bytes, 0o600)?;
    publish_entry(&temporary, &directory.join(name))?;
    sync(directory)
}

fn read<T: serde::de::DeserializeOwned>(directory: &Path, name: &str) -> Result<Option<T>, String> {
    let path = directory.join(name);
    let file = match fs::OpenOptions::new().read(true).custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK).open(&path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(super::error(&path, e)),
    };
    if !file.metadata().map_err(|e| super::error(&path, e))?.is_file() {
        return Err(super::error(&path, "operation record is not regular"));
    }
    let mut bytes = Vec::new();
    file.take(LIMIT + 1).read_to_end(&mut bytes).map_err(|e| super::error(&path, e))?;
    if bytes.len() as u64 > LIMIT {
        return Err(super::error(&path, "operation record exceeds byte limit"));
    }
    decode(&bytes).map(Some).map_err(|e| super::error(&path, e))
}

fn scope(root: &Path) -> Result<PathBuf, String> {
    let directory = no_symlink_parents(root, STORE)?;
    fs::create_dir_all(&directory).map_err(|e| super::error(&directory, e))?;
    // Persist the infrastructure itself before a producer can publish intent.
    for parent in directory.ancestors().take_while(|p| *p != root) {
        sync(parent)?;
    }
    sync(root)?;
    Ok(directory)
}

fn relative(root: &Path, path: &Path) -> Result<String, String> {
    let name = path.strip_prefix(root).ok().and_then(Path::to_str)
        .ok_or_else(|| super::error(path, "invalid host-relative publication path"))?;
    chrono_harness::relative_path(name)?;
    Ok(name.into())
}

fn target(root: &Path, name: &str) -> Result<PathBuf, String> {
    super::relative(name, false)?;
    let path = root.join(name);
    if let Some((parent, _)) = name.rsplit_once('/') {
        no_symlink_parents(root, parent)?;
    }
    Ok(path)
}

fn build_plan(
    root: &Path,
    attempt: &str,
    directory: &Path,
    changes: Vec<Change>,
    directories: Vec<PathBuf>,
    bindings: Bindings,
    intent_sha256: String,
    byte_limit: u64,
    fault: &Fault,
) -> Result<Plan, String> {
    let mut parents = BTreeMap::new();
    let mut dir_entries = Vec::new();
    let mut files = Vec::new();
    let mut total = 0u64;
    for change in &changes {
        total = total.saturating_add(change.before.as_ref().map_or(0, |b| b.bytes.len() as u64));
        if let After::Regular(bytes) = &change.after {
            total = total.saturating_add(bytes.len() as u64);
        }
    }
    // The remaining bytes are available for metadata. The retention owner
    // validates the actual tree before it releases the recovery reference.
    if total > byte_limit {
        return Err("instruction staging exceeds declared output byte limit".into());
    }
    for destination in directories.iter().chain(changes.iter().map(|c| &c.path)) {
        for parent in destination.ancestors().skip(1).take_while(|p| *p != root) {
            if !directories.iter().any(|path| path == parent) {
                parents.insert(relative(root, parent)?, observe(parent)?.ok_or("publication parent disappeared")?);
            }
        }
    }
    for (i, path) in directories.into_iter().enumerate() {
        let name = relative(root, &path)?;
        if super::metadata(&target(root, &name)?)?.is_some() {
            return Err(super::error(&path, "directory destination changed during preparation"));
        }
        let staged = directory.join(format!("dir-{i}"));
        fs::create_dir(&staged).map_err(|e| super::error(&staged, e))?;
        dir_entries.push(Entry { target: name, before: None, next: observe(&staged)?.unwrap(), backup: None });
        fault.at(&format!("staged-dir-{i}"))?;
    }
    for (i, change) in changes.into_iter().enumerate() {
        let name = relative(root, &change.path)?;
        let before = change.before.as_ref().map(|b| Object {
            identity: b.identity.clone(), bytes: Some(sha256(&b.bytes)), mode: b.permissions.mode() & 0o777,
        });
        if observe(&target(root, &name)?)? != before {
            return Err(super::error(&change.path, "destination changed during preparation"));
        }
        let next = directory.join(format!("next-{i}"));
        match change.after {
            After::Regular(bytes) => write(&next, &bytes, change.permissions.map_or(0o600, |p| p.mode() & 0o777))?,
            After::RelativeAlias => symlink("CLAUDE.md", &next).map_err(|e| super::error(&next, e))?,
        }
        let backup = if let Some(before) = change.before {
            let path = directory.join(format!("original-{i}"));
            write(&path, &before.bytes, before.permissions.mode() & 0o777)?;
            observe(&path)?
        } else { None };
        files.push(Entry { target: name, before, next: observe(&next)?.unwrap(), backup });
        fault.at(&format!("staged-file-{i}"))?;
    }
    Ok(Plan { host: identity(root)?, attempt: attempt.into(), directories: dir_entries, files, parents, intent_sha256, bindings })
}

fn check_parents(root: &Path, plan: &Plan, path: &Path) -> Result<(), String> {
    if identity(root)? != plan.host { return Err("publication host identity changed; preserve state".into()); }
    for parent in path.ancestors().skip(1).take_while(|p| *p != root) {
        let name = relative(root, parent)?;
        let observed = super::metadata(parent)?;
        let current = observed.as_ref().map(|_| identity(parent)).transpose()?;
        if let Some(expected) = plan.parents.get(&name) {
            if !observed.is_some_and(|m| m.is_dir()) || observe(parent)?.as_ref() != Some(expected) {
                return Err(super::error(parent, "original publication parent changed; preserve pending state"));
            }
        } else if let Some(created) = plan.directories.iter().find(|e| e.target == name) {
            if current.as_ref().is_some_and(|id| *id != created.next.identity) {
                return Err(super::error(parent, "created publication parent changed; preserve pending state"));
            }
        } else {
            return Err(super::error(parent, "unbound publication parent"));
        }
    }
    Ok(())
}

fn check_bindings(root: &Path, plan: &Plan, committed: bool) -> Result<(), String> {
    for (name, expected) in &plan.bindings.inputs {
        let path = target(root, name)?;
        if observe(&path)?.as_ref() != Some(expected) {
            return Err(super::error(&path, "registered input changed after preparation"));
        }
    }
    for (name, expected) in &plan.bindings.unchanged {
        let path = target(root, name)?;
        if observe(&path)? != *expected {
            return Err(super::error(&path, "unchanged destination or literal alias changed"));
        }
    }
    // Recovery verifies the saved business plan and never repeats external
    // input IO. Its exact original observations remain in that immutable plan.
    if !committed {
        for external in &plan.bindings.external {
            let requested = PathBuf::from(std::ffi::OsString::from_vec(external.requested.clone()));
            let (_, observed) = external_input(&requested)?;
            if observed.resolved != external.resolved || observed.identity != external.identity
                || observed.sha256 != external.sha256 || observed.mode != external.mode {
                return Err(super::error(&requested, "external input changed after preparation"));
            }
        }
    }
    Ok(())
}

fn committed(directory: &Path, plan: Option<&Plan>) -> Result<bool, String> {
    let Some(commit) = read::<Commit>(directory, "commit.json")? else { return Ok(false) };
    let plan = plan.ok_or("commit without original plan; preserve evidence")?;
    if !record_bound(commit.host == plan.host, commit.attempt == plan.attempt, commit.plan_sha256 == digest(plan)?) {
        return Err("commit does not bind the exact original plan; preserve evidence".into());
    }
    Ok(true)
}

fn install(root: &Path, plan: &Plan, directory: &Path, fault: &Fault) -> Result<(), String> {
    check_bindings(root, plan, false)?;
    for (entries, prefix) in [(&plan.directories, "dir"), (&plan.files, "next")] {
        for (i, entry) in entries.iter().enumerate() {
            let path = target(root, &entry.target)?;
            check_parents(root, plan, &path)?;
            let staged = directory.join(format!("{prefix}-{i}"));
            if observe(&path)? != entry.before || observe(&staged)?.as_ref() != Some(&entry.next) {
                return Err(super::error(&path, "staging/destination drift; preserve pending state"));
            }
            if prefix == "next" && fault.after == Some(i) {
                return Err(super::error(&path, "injected publication failure"));
            }
            if entry.before.is_some() {
                fs::rename(&staged, &path).map_err(|e| super::error(&path, e))?;
            } else {
                publish_entry(&staged, &path)?;
            }
            sync(path.parent().unwrap())?;
            sync(directory)?;
            if prefix == "next" {
                fault.at(&format!("published-{i}"))?;
            } else { fault.at(&format!("published-dir-{i}"))?; }
        }
    }
    Ok(())
}

fn settle_effects(
    root: &Path, directory: &Path, plan: &Plan, committed: bool, fault: &Fault,
) -> Result<(), String> {
    let mut unrestored = Vec::new();
    let mut recovery = Vec::new();
    let mut published = Vec::new();
    for (entries, is_file) in [(&plan.files, true), (&plan.directories, false)] {
        for (i, entry) in entries.iter().enumerate().rev() {
            let path = root.join(&entry.target);
            let backup = directory.join(format!("original-{i}"));
            let action = (|| {
                let path = target(root, &entry.target)?;
                check_parents(root, plan, &path)?;
                let current = observe(&path)?;
                let before = current == entry.before || (entry.backup.is_some() && current == entry.backup);
                let installed = current.as_ref() == Some(&entry.next);
                if installed && is_file {
                    published.push(path.clone());
                }
                let ready = entry.backup.is_none() || observe(&backup)? == entry.backup;
                match repair(before, installed, ready, committed) {
                    Repair::Leave => Ok(()),
                    Repair::Preserve => Err("destination or original identity/content drift; preserve it".into()),
                    Repair::Restore => {
                        if is_file && fault.rollback == Some(i) {
                            return Err("injected rollback failure".into());
                        }
                        if !is_file && unrestored.iter().any(|unrestored: &PathBuf| unrestored.starts_with(&path)) {
                            return Ok(());
                        }
                        if entry.backup.is_some() {
                            fs::rename(&backup, &path).map_err(|e| super::error(&path, e))?;
                        } else if is_file {
                            fs::remove_file(&path).map_err(|e| super::error(&path, e))?;
                        } else {
                            fs::remove_dir(&path).map_err(|e| super::error(&path, e))?;
                        }
                        sync(path.parent().unwrap())?;
                        sync(directory)?;
                        fault.at(&format!("restored-{}-{i}", if is_file { "file" } else { "dir" }))?;
                        Ok(())
                    }
                }
            })();
            if let Err(error) = action {
                if entry.backup.is_some() && backup.is_file() {
                    recovery.push(format!("original retained at {}", backup.display()));
                }
                recovery.push(format!("{}: {error}", path.display()));
                unrestored.push(path);
            }
        }
    }
    if unrestored.is_empty() {
        Ok(())
    } else {
        Err(format!("published={published:?}; unrestored={unrestored:?}; recovery={recovery:?}"))
    }
}

fn clean_stage(directory: &Path, plan: Option<&Plan>, fault: &Fault) -> Result<(), String> {
    // Before the immutable plan exists, staging has never been allowed to
    // reach a business destination. Delete only this reserved attempt's slots.
    for item in fs::read_dir(directory).map_err(|e| super::error(directory, e))? {
        let item = item.map_err(|e| e.to_string())?;
        let name = item.file_name();
        let Some(name) = name.to_str() else { return Err("unexpected staging slot".into()) };
        let indexed = name.split_once('-').and_then(|(kind, n)| n.parse::<usize>().ok().map(|n| (kind, n)));
        let path = item.path();
        match indexed {
            Some(("dir" | "next", n)) => {
                let is_dir = name.starts_with("dir-");
                if let Some(plan) = plan {
                    let entries = if is_dir { &plan.directories } else { &plan.files };
                    if observe(&path)?.as_ref() != entries.get(n).map(|e| &e.next) {
                        return Err(super::error(&path, "staging identity changed; preserve it"));
                    }
                }
                if is_dir { fs::remove_dir(&path) } else { fs::remove_file(&path) }
                    .map_err(|e| super::error(&path, e))?;
            }
            Some(("original", _)) if plan.is_none() => {
                fs::remove_file(&path).map_err(|e| super::error(&path, e))?;
            }
            _ if ["plan.json.writing", "failure.json.writing", "commit.json.writing", "result.json.writing"].contains(&name) => {
                fs::remove_file(&path).map_err(|e| super::error(&path, e))?;
            }
            Some(("original", n)) => {
                if observe(&path)? != plan.and_then(|p| p.files.get(n)).and_then(|e| e.backup.clone()) {
                    return Err(super::error(&path, "original backup changed; preserve it"));
                }
            }
            _ if ["plan.json", "failure.json", "commit.json", "result.json"].contains(&name) => (),
            _ => return Err(super::error(&path, "unregistered staging member; preserve it")),
        }
        fault.at(&format!("cleaned-{name}"))?;
    }
    sync(directory)
}

fn inventory(root: &Path) -> Result<Inventory, String> {
    let inventory = Inventory::with_standalone_policy(root, POLICY)?;
    if inventory.terminal_limits(PRODUCER).is_none() {
        return Err("instructions-operation requires explicit finite terminal_limits in the adopted retention policy".into());
    }
    Ok(inventory)
}

fn release(root: &Path, attempt: &str, id: &str, result: &Value) -> Result<(), String> {
    let state = inventory(root)?;
    let path = root.join(attempt).join("result.json");
    let bytes = fs::read(&path).map_err(|e| super::error(&path, e))?;
    let bound = decode::<Value>(&bytes)? == *result
        && state.published_tree_member(PRODUCER, attempt, &format!("{attempt}/result.json"), &sha256(&bytes), bytes.len() as u64)?;
    let attached = state.rooted_original(PRODUCER, ACTIVE)?
        .is_some_and(|(actual, path, phase, _)| actual == id && path == attempt && phase == "released");
    if !can_release(attached, bound, matches!(result["publication"].as_str(), Some("committed" | "verified-committed" | "restored" | "not-started"))) {
        return Err("publication result is not sealed and bound; retain the recovery reference".into());
    }
    state.reference_paths(ACTIVE, &[])
}

fn original_intent(mut value: &Value) -> Result<&Value, String> {
    for _ in 0..8 {
        value = match intent_layer(value["schema"] == "chrono-instructions-intent/v1",
            value.get("original").is_some(), value.get("intent").is_some()) {
            IntentLayer::Done => return Ok(value),
            IntentLayer::Original => value.get("original").ok_or("missing original envelope")?,
            IntentLayer::Intent => value.get("intent").ok_or("missing intent envelope")?,
            IntentLayer::Invalid => return Err("retention outcome has no original instruction intent".into()),
        };
    }
    Err("instruction intent envelope nesting bound".into())
}

fn recover(root: &Path, fault: &Fault) -> Result<(), String> {
    let Some((id, attempt, mut phase, intent)) = inventory(root)?.rooted_original(PRODUCER, ACTIVE)? else {
        return Ok(());
    };
    let directory = no_symlink_parents(root, &attempt)?;
    if matches!(phase.as_str(), "reserving" | "creating") {
        inventory(root)?.materialize_rooted(PRODUCER, ACTIVE)?;
        fault.at("creation-recovered")?;
        phase = inventory(root)?.rooted_original(PRODUCER, ACTIVE)?
            .ok_or("original recovery reference disappeared")?.2;
    }
    let bound = original_intent(&intent)?;
    if bound["host_identity"] != identity(root)? || bound["attempt"] != attempt
        || bound["host"] != json!(root.as_os_str().as_bytes()) {
        return Err("instruction intent identity changed; preserve recovery".into());
    }
    if !directory.exists() {
        return Err("instruction recovery directory missing; preserve intent".into());
    }
    let prior: Option<Value> = read(&directory, "result.json")?;
    if let Some(result) = &prior {
        if result["schema"] != "chrono-instructions-result/v1" || result["attempt"] != attempt
            || result["host_identity"] != identity(root)? || result["intent_sha256"] != digest(bound)? {
            return Err("instruction result binding changed; preserve recovery".into());
        }
    }
    if phase == "sealing" && intent.get("intent").is_some() {
        inventory(root)?.maintain_originals(std::slice::from_ref(&id))?;
        phase = inventory(root)?.rooted_original(PRODUCER, ACTIVE)?
            .ok_or("original recovery reference disappeared")?.2;
        if phase == "sealing" {
            return Err(format!("instruction result sealing incomplete; resume same attempt {attempt}"));
        }
    }
    if phase == "released" {
        return release(root, &attempt, &id, &prior.ok_or("sealed instruction result missing")?);
    }
    let mut result = prior;
    let publication = inventory(root)?.recover_unfinished_directory(PRODUCER, &attempt, || {
        let plan: Option<Plan> = read(&directory, "plan.json")?;
        let commit = committed(&directory, plan.as_ref())?;
        if let Some(plan) = &plan {
            if !record_bound(plan.host == identity(root)?, plan.attempt == attempt, plan.intent_sha256 == digest(bound)?) {
                return Err("instruction recovery plan binding changed".into());
            }
            check_bindings(root, plan, true)?;
            settle_effects(root, &directory, plan, commit, fault)?;
        }
        clean_stage(&directory, plan.as_ref(), fault)?;
        if result.is_none() {
            let failure: Option<Value> = read(&directory, "failure.json")?;
            let value = json!({
                "schema":"chrono-instructions-result/v1", "attempt":attempt, "updated":timestamp()?,
                "host_identity":identity(root)?, "intent_sha256":digest(bound)?,
                "publication": if commit { "verified-committed" } else if plan.is_some() { "restored" } else { "not-started" },
                "known_failure":failure.unwrap_or(Value::Null),
                "original_process_exit":null, "recovery":"verified-from-original-attempt",
            });
            record(&directory, "result.json", &value)?;
            result = Some(value);
        }
        Ok(())
    })?.ok_or("instruction recovery publication unavailable")?;
    let result = result.ok_or("instruction recovery result missing")?;
    publication.complete(json!({"intent":bound, "result":"result.json", "publication":result["publication"]}))?;
    fault.at("sealed")?;
    release(root, &attempt, &id, &result)
}

pub(super) fn run(
    host: &Path, operation: &str, request: Value, fault: Fault,
    prepare: impl FnOnce(&Path) -> Result<(Vec<Change>, Vec<PathBuf>, Bindings), String>,
) -> Result<Vec<PathBuf>, String> {
    let root = fs::canonicalize(host).map_err(|e| super::error(host, e))?;
    let host_identity = identity(&root)?;
    let store = scope(&root)?;
    let exclusive = DirectoryLease::acquire(&store, true, None)?
        .ok_or("instructions operation busy on this host")?;
    recover(&root, &fault)?;
    exclusive.stable()?;
    if identity(&root)? != host_identity {
        return Err("instructions host identity changed".into());
    }
    let directory = unique_path(&store, "attempt-", "")?;
    let attempt = relative(&root, &directory)?;
    let intent = json!({
        "schema":"chrono-instructions-intent/v1", "producer":PRODUCER,
        "operation":operation, "attempt":attempt, "recovery_role":ACTIVE, "host":root.as_os_str().as_bytes(),
        "host_identity":host_identity, "request":request, "started":timestamp()?,
        "original_process_exit":null, "recovery":"resolve same attempt before prepare",
    });
    let publication = inventory(&root)?.plan_rooted(PRODUCER, &attempt, intent.clone(), ACTIVE)?;
    let id = publication.id().to_owned();
    let mut finalized = None;
    let evaluated = (|| {
    fault.at("intent")?;
    publication.materialize()?;
    sync(&store)?;
    fault.at("before-prepare")?;
    let mut plan = None;
    let mut did_commit = false;
    let result = (|| {
        let (changes, directories, bindings) = prepare(&root)?;
        let prepared = build_plan(&root, &attempt, &directory, changes, directories, bindings, digest(&intent)?, publication.byte_limit(), &fault)?;
        if prepared.host != host_identity { return Err("publication host identity changed during preparation".into()); }
        exclusive.stable()?;
        record(&directory, "plan.json", &prepared)?;
        plan = Some(prepared);
        fault.at("prepared")?;
        let plan = plan.as_ref().unwrap();
        install(&root, plan, &directory, &fault)?;
        record(&directory, "commit.json", &Commit { attempt: attempt.clone(), host: host_identity.clone(), plan_sha256: digest(plan)? })?;
        did_commit = true;
        fault.at("committed")?;
        Ok::<_, String>(plan.files.iter().map(|entry| root.join(&entry.target)).collect::<Vec<_>>())
    })();
    let result = match result {
        Ok(paths) => Ok(paths),
        Err(error) => {
            let failure = json!({"known_error":error, "updated":timestamp()?});
            record(&directory, "failure.json", &failure).map_err(|state| format!("{error}; state failure: {state}; attempt {attempt}"))?;
            // If a commit rename succeeded and only its directory sync failed,
            // leave the intent rooted; recovery re-observes the immutable decision.
            if committed(&directory, plan.as_ref())? {
                return Err(format!("{error}; commit state needs recovery; attempt {attempt}"));
            }
            if let Some(plan) = &plan {
                settle_effects(&root, &directory, plan, false, &fault)
                    .map_err(|repair| format!("{error}; {repair}; attempt {attempt}"))?;
            }
            Err(format!("{error}; unrestored=[]"))
        }
    };
    fault.at("cleanup")?;
    clean_stage(&directory, plan.as_ref(), &fault).map_err(|e| format!("{result:?}; cleanup failed: {e}; attempt {attempt}"))?;
    let state = json!({
        "schema":"chrono-instructions-result/v1", "attempt":attempt, "updated":timestamp()?,
        "host_identity":host_identity, "intent_sha256":digest(&intent)?,
        "transport_exit":null,
        "publication": if did_commit { "committed" } else if plan.is_some() { "restored" } else { "not-started" },
        "producer_evaluation": match &result {
            Ok(paths) => json!({"changed": paths.iter().map(|p| relative(&root, p)).collect::<Result<Vec<_>, _>>()?}),
            Err(error) => json!({"error":error}),
        },
        "original_process_exit":null,
    });
    fault.at("before-result")?;
    record(&directory, "result.json", &state)
        .map_err(|e| format!("{result:?}; state failure: {e}; attempt {attempt}"))?;
    fault.at("before-seal")?;
    publication.complete(json!({"intent":intent, "result":"result.json", "publication":state["publication"]}))
        .map_err(|e| format!("{result:?}; retention state failure: {e}; attempt {attempt}"))?;
    fault.at("sealed")?;
    fault.at("release")?;
    finalized = Some(state);
    result
    })();
    let observed = match &evaluated {
        Ok(paths) => json!({"method":"transaction-evaluator","result":{"changed":paths.iter().map(|p| relative(&root, p)).collect::<Result<Vec<_>, _>>()?},"transport_exit":null,"process_exit":null}),
        Err(error) => json!({"method":"transaction-evaluator","result":{"error":error},"transport_exit":null,"process_exit":null}),
    };
    inventory(&root)?.observe_producer_return(PRODUCER, &attempt, observed)
        .map_err(|error| format!("{evaluated:?}; return observation failed: {error}; attempt {attempt}"))?;
    if let Some(state) = finalized {
        release(&root, &attempt, &id, &state)
            .map_err(|error| format!("{evaluated:?}; recovery release failure: {error}; attempt {attempt}"))?;
    }
    evaluated
}


pub(super) fn status(host: &Path, attempt: Option<&str>) -> Result<Value, String> {
    let root = fs::canonicalize(host).map_err(|e| super::error(host, e))?;
    inventory(&root)?.producer_status(PRODUCER, attempt)
}

pub(super) fn maintain(host: &Path) -> Result<Value, String> {
    let root = fs::canonicalize(host).map_err(|e| super::error(host, e))?;
    let store = scope(&root)?;
    let _exclusive = DirectoryLease::acquire(&store, true, None)?
        .ok_or("instructions operation busy on this host")?;
    recover(&root, &Fault::default())?;
    let report = inventory(&root)?.maintain_producer(PRODUCER)?;
    if report["quota_errors"].as_array().is_some_and(|errors| !errors.is_empty())
        || report["round"]["accounting_errors"].as_array().is_some_and(|errors| !errors.is_empty()) {
        return Err(format!("instruction maintenance incomplete with original errors: {report}"));
    }
    Ok(report)
}
