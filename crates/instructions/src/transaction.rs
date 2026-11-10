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

use crate::publication_decision::{Repair, can_release, repair};
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
use std::os::unix::{ffi::OsStrExt, fs::{MetadataExt, OpenOptionsExt, PermissionsExt, symlink}};
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
}

impl Fault {
    fn at(&self, point: &str) {
        if let Some(hook) = self.checkpoint {
            hook(point);
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    host: String,
    attempt: String,
    directories: Vec<Entry>,
    files: Vec<Entry>,
    parents: BTreeMap<String, String>,
    inputs: BTreeMap<String, String>,
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
    if identity(path)? != id {
        return Err(super::error(path, "publication object identity changed during read"));
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
    super::relative(name, false)?;
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
    inputs: BTreeMap<String, String>,
    byte_limit: u64,
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
                parents.insert(relative(root, parent)?, identity(parent)?);
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
    }
    Ok(Plan { host: identity(root)?, attempt: attempt.into(), directories: dir_entries, files, parents, inputs })
}

fn check_parents(root: &Path, plan: &Plan, path: &Path) -> Result<(), String> {
    for parent in path.ancestors().skip(1).take_while(|p| *p != root) {
        let name = relative(root, parent)?;
        let observed = super::metadata(parent)?;
        let current = observed.as_ref().map(|_| identity(parent)).transpose()?;
        if let Some(expected) = plan.parents.get(&name) {
            if !observed.is_some_and(|m| m.is_dir()) || current.as_ref() != Some(expected) {
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

fn install(root: &Path, plan: &Plan, directory: &Path, fault: &Fault) -> Result<(), String> {
    for (name, digest) in &plan.inputs {
        let path = no_symlink_parents(root, name)?;
        if observe(&path)?.and_then(|o| o.bytes).as_ref() != Some(digest) {
            return Err(super::error(&path, "registered input changed after preparation"));
        }
    }
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
                fault.at(&format!("published-{i}"));
            }
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
                        if !is_file && !unrestored.is_empty() {
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
                        fault.at(&format!("restored-{}-{i}", if is_file { "file" } else { "dir" }));
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

fn clean_stage(directory: &Path, plan: Option<&Plan>) -> Result<(), String> {
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
            _ => (),
        }
    }
    sync(directory)
}

fn inventory(root: &Path) -> Result<Inventory, String> {
    Inventory::with_standalone_policy(root, POLICY)
}

fn release(root: &Path, attempt: &str, id: &str, result: &Value) -> Result<(), String> {
    let state = inventory(root)?;
    let path = root.join(attempt).join("result.json");
    let bytes = fs::read(&path).map_err(|e| super::error(&path, e))?;
    let bound = decode::<Value>(&bytes)? == *result
        && state.published_tree_member(PRODUCER, attempt, &format!("{attempt}/result.json"), &sha256(&bytes), bytes.len() as u64)?;
    let attached = state.rooted_original(PRODUCER, ACTIVE)?
        .is_some_and(|(actual, path, phase, _)| actual == id && path == attempt && phase == "released");
    if !can_release(attached, bound, result["publication"].is_string()) {
        return Err("publication result is not sealed and bound; retain the recovery reference".into());
    }
    state.reference_paths(ACTIVE, &[])
}

fn recover(root: &Path, fault: &Fault) -> Result<(), String> {
    let Some((id, attempt, mut phase, intent)) = inventory(root)?.rooted_original(PRODUCER, ACTIVE)? else {
        return Ok(());
    };
    let directory = no_symlink_parents(root, &attempt)?;
    if matches!(phase.as_str(), "reserving" | "creating") {
        inventory(root)?.maintain_originals(std::slice::from_ref(&id))?;
        phase = inventory(root)?.rooted_original(PRODUCER, ACTIVE)?
            .ok_or("original recovery reference disappeared")?.2;
    }
    let bound = intent.get("original").unwrap_or(&intent);
    if bound["host_identity"] != identity(root)? || bound["attempt"] != attempt {
        return Err("instruction intent identity changed; preserve recovery");
    }
    if !directory.exists() {
        if phase == "sealing" && inventory(root)?.rooted_original(PRODUCER, ACTIVE)?
            .is_some_and(|(_, _, _, o)| o["publication"] == "interrupted-creation")
        {
            // The custodian verified that business IO could not start. Its
            // original unknown creation outcome remains in retention custody.
            return inventory(root)?.reference_paths(ACTIVE, &[]);
        }
        return Err("instruction recovery directory missing; preserve intent".into());
    }
    let prior: Option<Value> = read(&directory, "result.json")?;
    if phase == "released" {
        return release(root, &attempt, &id, &prior.ok_or("sealed instruction result missing")?);
    }
    let mut result = prior;
    let publication = inventory(root)?.recover_unfinished_directory(PRODUCER, &attempt, || {
        let plan: Option<Plan> = read(&directory, "plan.json")?;
        let commit: Option<Value> = read(&directory, "commit.json")?;
        if let Some(plan) = &plan {
            if plan.host != identity(root)? || plan.attempt != attempt {
                return Err("instruction recovery plan binding changed".into());
            }
            settle_effects(root, &directory, plan, commit.is_some(), fault)?;
        } else if commit.is_some() {
            return Err("commit without original plan; preserve evidence".into());
        }
        clean_stage(&directory, plan.as_ref())?;
        if result.is_none() {
            let failure: Option<Value> = read(&directory, "failure.json")?;
            let value = json!({
                "schema":"chrono-instructions-result/v1", "attempt":attempt, "updated":timestamp()?,
                "publication": if commit.is_some() { "verified-committed" } else if plan.is_some() { "restored" } else { "not-started" },
                "original_return":failure.unwrap_or(Value::Null),
                "original_process_exit":null, "recovery":"verified-after-interruption",
            });
            record(&directory, "result.json", &value)?;
            result = Some(value);
        }
        Ok(())
    })?.ok_or("instruction recovery publication unavailable")?;
    let result = result.ok_or("instruction recovery result missing")?;
    publication.complete(json!({"intent":intent, "result":"result.json", "publication":result["publication"]}))?;
    release(root, &attempt, &id, &result)
}

pub(super) fn run(
    host: &Path, operation: &str, request: Value, fault: Fault,
    prepare: impl FnOnce(&Path) -> Result<(Vec<Change>, Vec<PathBuf>, BTreeMap<String, String>), String>,
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
        "operation":operation, "attempt":attempt, "host":root.as_os_str().as_bytes(),
        "host_identity":host_identity, "request":request, "started":timestamp()?,
        "original_process_exit":null, "recovery":"resolve same attempt before prepare",
    });
    let publication = inventory(&root)?.plan_rooted(PRODUCER, &attempt, intent.clone(), ACTIVE)?;
    let id = publication.id().to_owned();
    fault.at("intent");
    publication.materialize()?;
    sync(&store)?;
    fault.at("before-prepare");
    let mut plan = None;
    let mut committed = false;
    let result = (|| {
        let (changes, directories, inputs) = prepare(&root)?;
        let prepared = build_plan(&root, &attempt, &directory, changes, directories, inputs, publication.byte_limit())?;
        record(&directory, "plan.json", &prepared)?;
        plan = Some(prepared);
        fault.at("prepared");
        let plan = plan.as_ref().unwrap();
        install(&root, plan, &directory, &fault)?;
        record(&directory, "commit.json", &json!({"attempt":attempt, "updated":timestamp()?}))?;
        committed = true;
        fault.at("committed");
        Ok::<_, String>(plan.files.iter().map(|entry| root.join(&entry.target)).collect::<Vec<_>>())
    })();
    let result = match result {
        Ok(paths) => Ok(paths),
        Err(error) => {
            let failure = json!({"known_error":error, "updated":timestamp()?});
            record(&directory, "failure.json", &failure).map_err(|state| format!("{error}; state failure: {state}; attempt {attempt}"))?;
            // If a commit rename succeeded and only its directory sync failed,
            // leave the intent rooted; recovery re-observes the immutable decision.
            if read::<Value>(&directory, "commit.json")?.is_some() {
                return Err(format!("{error}; commit state needs recovery; attempt {attempt}"));
            }
            if let Some(plan) = &plan {
                settle_effects(&root, &directory, plan, false, &fault)
                    .map_err(|repair| format!("{error}; {repair}; attempt {attempt}"))?;
            }
            Err(format!("{error}; unrestored=[]"))
        }
    };
    clean_stage(&directory, plan.as_ref()).map_err(|e| format!("{result:?}; cleanup failed: {e}; attempt {attempt}"))?;
    let state = json!({
        "schema":"chrono-instructions-result/v1", "attempt":attempt, "updated":timestamp()?,
        "publication": if committed { "committed" } else if plan.is_some() { "restored" } else { "not-started" },
        "producer_result": match &result {
            Ok(paths) => json!({"changed": paths.iter().map(|p| relative(&root, p)).collect::<Result<Vec<_>, _>>()?}),
            Err(error) => json!({"error":error}),
        },
        "original_process_exit":null,
    });
    fault.at("before-result");
    record(&directory, "result.json", &state)
        .map_err(|e| format!("{result:?}; state failure: {e}; attempt {attempt}"))?;
    publication.complete(json!({"intent":intent, "result":"result.json", "publication":state["publication"]}))
        .map_err(|e| format!("{result:?}; retention state failure: {e}; attempt {attempt}"))?;
    release(&root, &attempt, &id, &state)
        .map_err(|e| format!("{result:?}; recovery release failure: {e}; attempt {attempt}"))?;
    result
}
