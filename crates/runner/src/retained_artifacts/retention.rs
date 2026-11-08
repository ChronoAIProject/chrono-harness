//! Explicit producer output lifetimes. No directory discovery or inferred consumers.
//! Uses the lifecycle kernel capability owner for publication, use and disposal exclusion.
use crate::{decode, no_symlink_parents, ownership::Lease, sha256};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    rc::{Rc, Weak},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[path = "manifest.rs"]
mod manifest;
use manifest::Manifest;
#[path = "partition.rs"]
mod partition;
use partition::Gate;

pub const POLICY_PATH: &str = ".chrono-harness/retention.json";
const STATE: &str = ".chrono-harness/state/output-retention-v1";
const MAX_STATE: u64 = 8 * 1024 * 1024;
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub schema: String,
    pub producers: BTreeMap<String, Vec<String>>,
    pub keep_seconds: u64,
    #[serde(default = "default_diagnostic_seconds")]
    pub diagnostic_keep_seconds: u64,
    pub max_related_outputs: usize,
    pub max_summaries: usize,
    pub max_nodes_per_round: usize,
    pub max_bytes_per_round: u64,
    pub max_millis_per_round: u64,
    #[serde(default = "default_output_bytes")]
    pub max_output_bytes: u64,
    #[serde(default = "default_metadata_seconds")]
    pub metadata_keep_seconds: u64,
    #[serde(default = "default_manifest_nodes")]
    pub max_manifest_nodes: usize,
    #[serde(default = "default_metadata_bytes")]
    pub max_metadata_bytes_per_round: u64,
    #[serde(default)]
    historical: BTreeMap<String, super::retention_command::Historical>,
}
fn default_metadata_bytes() -> u64 {
    64 * 1024 * 1024
}
fn default_output_bytes() -> u64 {
    1024 * 1024 * 1024
}
fn default_metadata_seconds() -> u64 {
    86400
}
fn default_diagnostic_seconds() -> u64 {
    86400
}
fn default_manifest_nodes() -> usize {
    16384
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Node {
    path: String,
    identity: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    entry: Option<manifest::Entry>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Creation {
    parent_identity: String,
    staged: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Output {
    id: String,
    producer: String,
    path: String,
    identity: String,
    tree: bool,
    sha256: Option<String>,
    length: Option<u64>,
    lease: String,
    lease_id: String,
    #[serde(deserialize_with = "crate::strict_json_value")]
    outcome: Value,
    phase: String,
    expires: u64,
    references: Vec<String>,
    stack: Vec<Node>,
    removed_logical_bytes: u64,
    #[serde(default)]
    manifest: Manifest,
    #[serde(default)]
    reserved_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    byte_limit: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    creation: Option<Creation>,
    #[serde(default)]
    overflow_disposal: bool,
}

// A round owns typed records for only the explicitly addressed graph.
#[derive(Clone)]
struct OutputRecord {
    value: Arc<Output>,
}
impl From<Output> for OutputRecord {
    fn from(value: Output) -> Self {
        Self {
            value: Arc::new(value),
        }
    }
}
impl std::ops::Deref for OutputRecord {
    type Target = Output;
    fn deref(&self) -> &Output {
        &self.value
    }
}
impl std::ops::DerefMut for OutputRecord {
    fn deref_mut(&mut self) -> &mut Output {
        Arc::make_mut(&mut self.value)
    }
}
impl Serialize for OutputRecord {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(s)
    }
}
#[derive(Clone, Serialize)]
struct Ledger {
    policy_sha256: String,
    outputs: Vec<OutputRecord>,
    roots: BTreeMap<String, Vec<String>>,
    summaries: Vec<Value>,
    cursor: usize,
    #[serde(default)]
    retired_leases: Vec<Node>,
}
impl Default for Ledger {
    fn default() -> Self {
        Self {
            policy_sha256: String::new(),
            outputs: Vec::new(),
            roots: BTreeMap::new(),
            summaries: Vec::new(),
            cursor: 0,
            retired_leases: Vec::new(),
        }
    }
}
thread_local! {
    static READ_SCOPES: RefCell<Vec<Weak<ReadInputs>>> = const { RefCell::new(Vec::new()) };
}
trait Exclusion {
    fn verify(&self) -> Result<(), String>;
}
impl Exclusion for Lease {
    fn verify(&self) -> Result<(), String> {
        self.stable()
    }
}
impl Exclusion for Gate {
    fn verify(&self) -> Result<(), String> {
        self.stable()
    }
}
impl Exclusion for crate::ownership::DirectoryLease {
    fn verify(&self) -> Result<(), String> {
        self.stable()
    }
}
struct ReadInputs {
    root: PathBuf,
    #[cfg(unix)]
    capabilities: RefCell<BTreeMap<String, crate::process_fds::Scope>>,
}
/// A caller keeps the originals it explicitly reads or produces until its
/// output handoff. Enclosing callers receive the exact child output capability;
/// the existing process engine forwards these capabilities to subprocesses.
pub struct ReadScope(Rc<ReadInputs>);
impl Drop for ReadScope {
    fn drop(&mut self) {
        let this = Rc::downgrade(&self.0);
        READ_SCOPES.with(|scopes| {
            scopes
                .borrow_mut()
                .retain(|scope| !scope.ptr_eq(&this) && scope.strong_count() != 0)
        });
    }
}
pub fn read_scope(root: &Path) -> Result<ReadScope, String> {
    let inputs = Rc::new(ReadInputs {
        root: fs::canonicalize(root).map_err(|e| e.to_string())?,
        #[cfg(unix)]
        capabilities: RefCell::new(BTreeMap::new()),
    });
    READ_SCOPES.with(|scopes| scopes.borrow_mut().push(Rc::downgrade(&inputs)));
    Ok(ReadScope(inputs))
}
fn strict_summaries<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Value>, D::Error> {
    serde_json::from_value(crate::strict_json_value(d)?).map_err(serde::de::Error::custom)
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Transaction {
    schema: String,
    key: String,
    directory_identity: String,
    lease_id: String,
    phase: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    paths: Vec<String>,
}
pub struct Inventory {
    root: PathBuf,
    policy: Policy,
    policy_bytes: Vec<u8>,
    directory: PathBuf,
    round_metadata: Cell<Option<u64>>,
    round_began: Cell<Option<Instant>>,
    round_node_limit: Cell<Option<usize>>,
    round_verified_left: Cell<Option<u64>>,
}
/// Exact existing owner identity supplied by a declared original consumer.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OriginalIdentity {
    pub id: String,
    pub path: String,
    pub identity: String,
}
/// Exact originals held for one actual caller until its durable output accepts them.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub schema: String,
    pub consumer: String,
    pub purpose: String,
    pub originals: Vec<OriginalIdentity>,
}
/// Holding this capability protects producer descendants too. Closing it never fabricates completion.
pub struct Publication {
    inventory: Inventory,
    id: String,
    lease: Lease,
    initial: Arc<Output>,
    byte_limit: u64,
    path: String,
    identity: RefCell<String>,
    tree: bool,
    written: Cell<u64>,
    written_nodes: Cell<usize>,
}
fn now() -> Result<u64, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs())
}
use crate::artifact_disposal::identity;

fn write(path: &Path, raw: &[u8]) -> Result<(), String> {
    if raw.len() as u64 > MAX_STATE {
        return Err("retention metadata bound; preserve outputs".into());
    }
    // One declared replacement slot under admission exclusion. Hard termination
    // cannot accumulate a new random temporary file on every retry.
    let temporary = path.with_extension("pending");
    match fs::symlink_metadata(&temporary) {
        Ok(m) if !m.is_file() || m.file_type().is_symlink() => {
            return Err("retention replacement slot identity/type changed".into());
        }
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.to_string()),
        _ => (),
    }
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    let mut f = options.open(&temporary).map_err(|e| e.to_string())?;
    f.write_all(raw).map_err(|e| e.to_string())?;
    f.sync_all().map_err(|e| e.to_string())?;
    fs::rename(temporary, path).map_err(|e| e.to_string())?;
    Ok(())
}
impl Inventory {
    pub fn read_scope(&self) -> Result<ReadScope, String> {
        read_scope(&self.root)
    }
    fn remember_input(&self, lease: &Lease) -> Result<(), String> {
        READ_SCOPES.with(|scopes| {
            for inputs in scopes
                .borrow()
                .iter()
                .filter_map(Weak::upgrade)
                .filter(|inputs| inputs.root == self.root)
            {
                #[cfg(unix)]
                {
                    use std::os::fd::AsFd;
                    let mut capabilities = inputs.capabilities.borrow_mut();
                    if !capabilities.contains_key(lease.id()) {
                        if capabilities.len() >= self.policy.max_related_outputs {
                            return Err(
                                "caller input capability capacity; preserve originals".into()
                            );
                        }
                        capabilities.insert(
                            lease.id().into(),
                            crate::process_fds::Scope::new(&[lease.capability()?.as_fd()])?,
                        );
                    }
                }
            }
            Ok(())
        })
    }
    /// Absence preserves old semantics. Existing unknown output directories are never enrolled by opening.
    pub fn adopted(root: &Path) -> Result<Option<Self>, String> {
        let physical_root = fs::canonicalize(root).map_err(|e| e.to_string())?;
        let root = physical_root.as_path();
        let path = no_symlink_parents(root, POLICY_PATH)?;
        let bytes = match fs::File::open(path) {
            Ok(f) => {
                let mut bytes = Vec::new();
                f.take(1024 * 1024 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|e| e.to_string())?;
                if bytes.len() > 1024 * 1024 {
                    return Err("retention policy byte bound".into());
                }
                bytes
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        let policy: Policy = decode(&bytes)?;
        if policy.schema != "chrono-output-retention/v2"
            || policy.max_related_outputs == 0
            || policy.max_related_outputs > 4096
            || policy.max_summaries == 0
            || policy.max_summaries > 4096
            || policy.max_nodes_per_round == 0
            || policy.max_nodes_per_round > 4096
            || policy.max_bytes_per_round == 0
            || policy.max_millis_per_round == 0
            || policy.max_millis_per_round > 60000
            || policy.max_output_bytes == 0
            || policy.max_manifest_nodes == 0
            || policy.max_manifest_nodes > 65536
            || policy.max_metadata_bytes_per_round < 3 * MAX_STATE
        {
            return Err("invalid finite output retention policy".into());
        }
        for scopes in policy.producers.values() {
            if scopes.is_empty() {
                return Err("producer has no output scopes".into());
            }
            for scope in scopes {
                crate::relative_path(scope.trim_end_matches('/'))?;
                let managed_bin = scope.starts_with(".chrono-harness/bin/")
                    && scope != ".chrono-harness/bin/";
                if (!scope.starts_with(".chrono-harness/state/") && !managed_bin)
                    || !scope.ends_with('/')
                    || scope.starts_with(STATE)
                {
                    return Err(
                        "retention scope must be explicit state output or bin subdirectory, outside retention metadata"
                            .into(),
                    );
                }
            }
        }
        let directory = no_symlink_parents(root, STATE)?;
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        if directory.join("inventory.json").exists() {
            return Err(
                "aggregate output retention requires explicit owner cutover; originals preserved"
                    .into(),
            );
        }
        for namespace in ["objects", "roles", "paths", "transaction-gates"] {
            fs::create_dir_all(no_symlink_parents(&directory, namespace)?)
                .map_err(|e| e.to_string())?;
        }
        Ok(Some(Self {
            root: root.into(),
            policy,
            policy_bytes: bytes,
            directory,
            round_metadata: Cell::new(None),
            round_began: Cell::new(None),
            round_node_limit: Cell::new(None),
            round_verified_left: Cell::new(None),
        }))
    }
    fn charge_metadata(&self, bytes: u64) -> Result<(), String> {
        if let Some(remaining) = self.round_metadata.get() {
            self.round_metadata.set(Some(
                remaining
                    .checked_sub(bytes)
                    .ok_or("round metadata byte capacity; resume next ordinary entry")?,
            ));
        }
        Ok(())
    }
    fn metadata_room(&self) -> bool {
        self.round_metadata.get().is_none_or(|n| n >= 2 * MAX_STATE)
    }
    fn stable(&self, gate: &impl Exclusion) -> Result<(), String> {
        self.charge_metadata(self.policy_bytes.len() as u64)?;
        gate.verify()?;
        if fs::read(no_symlink_parents(&self.root, POLICY_PATH)?).map_err(|e| e.to_string())?
            != self.policy_bytes
        {
            return Err("output retention policy changed; preserve outputs".into());
        }
        Ok(())
    }
    fn load(&self, gate: &Gate) -> Result<Ledger, String> {
        self.load_bound(gate, &sha256(&self.policy_bytes))
    }
    fn load_bound(&self, gate: &Gate, policy_digest: &str) -> Result<Ledger, String> {
        let l = self.partition_load(gate, policy_digest)?;
        let mut ids = BTreeSet::new();
        for o in &l.outputs {
            crate::relative_path(&o.path)?;
            crate::relative_path(&o.lease)?;
            if !ids.insert(o.id.clone())
                || o.path.len() > 4096
                || o.byte_limit
                    .is_some_and(|n| n == 0 || n > self.policy.max_output_bytes)
                || o.stack.len() > 129
                || o.references.len() > self.policy.max_related_outputs
                || o.manifest.entries.len() > self.policy.max_manifest_nodes
                || !self
                    .policy
                    .producers
                    .get(&o.producer)
                    .is_some_and(|scopes| scopes.iter().any(|p| o.path.starts_with(p)))
                || !matches!(
                    o.phase.as_str(),
                    "reserving" | "creating" | "publishing" | "sealing" | "released" | "disposing"
                )
            {
                return Err("invalid retained output inventory; preserve originals".into());
            }
            if let Some(creation) = &o.creation {
                if creation.staged != format!("creation-{}", o.id)
                    || !matches!(o.phase.as_str(), "reserving" | "creating")
                {
                    return Err("invalid output creation intent; preserve originals".into());
                }
            }
            let mut manifest_paths = BTreeSet::new();
            for entry in &o.manifest.entries {
                crate::relative_path(&entry.path)?;
                if entry.path.len() > 4096
                    || !manifest_paths.insert(&entry.path)
                    || (entry.path != o.path
                        && (!o.tree || !entry.path.starts_with(&format!("{}/", o.path))))
                {
                    return Err("invalid sealed descendant manifest; preserve originals".into());
                }
            }
            for node in &o.stack {
                if node.path != o.path
                    && (!o.tree || !node.path.starts_with(&format!("{}/", o.path)))
                {
                    return Err("disposal cursor outside exact output identity".into());
                }
                if node
                    .entry
                    .as_ref()
                    .is_some_and(|entry| entry.path != node.path || entry.identity != node.identity)
                {
                    return Err("overflow cursor original identity differs".into());
                }
            }
        }
        for o in &l.outputs {
            if o.references.iter().any(|id| !ids.contains(id)) {
                return Err("missing retained output dependency".into());
            }
        }
        for refs in l.roots.values() {
            if refs.len() > self.policy.max_related_outputs
                || refs.iter().any(|id| !ids.contains(id))
            {
                return Err("missing current consumer output reference".into());
            }
        }
        Ok(l)
    }
    fn save(&self, l: &Ledger, gate: &Gate) -> Result<(), String> {
        self.stable(gate)?;
        self.partition_save(l, gate)
    }
    fn transaction_path(&self, key: &str) -> PathBuf {
        self.directory.join(format!("transaction-{key}.json"))
    }
    fn transaction(&self, key: &str) -> Result<Option<Transaction>, String> {
        let path = self.transaction_path(key);
        let file = match fs::File::open(no_symlink_parents(
            &self.directory,
            path.file_name()
                .unwrap()
                .to_str()
                .ok_or("transaction path UTF8")?,
        )?) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        if !file.metadata().map_err(|e| e.to_string())?.is_file() {
            return Err("retention transaction declaration must be a regular file".into());
        }
        let mut bytes = Vec::new();
        file.take(16385)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        self.charge_metadata(bytes.len() as u64)?;
        if bytes.len() > 16384 {
            return Err("retention transaction declaration byte capacity".into());
        }
        let record: Transaction = decode(&bytes)?;
        let digest = key
            .strip_prefix("writer-")
            .or_else(|| key.strip_prefix("slot-"))
            .or_else(|| key.strip_prefix("removal-"));
        let removal = key.starts_with("removal-");
        for path in &record.paths {
            crate::relative_path(path)?;
            if path.len() > 4096 {
                return Err("removal path identity capacity".into());
            }
        }
        if record.schema != "chrono-output-transaction/v1"
            || record.key != key
            || !digest.is_some_and(crate::wire::is_digest)
            || record.directory_identity != identity(&self.directory)?
            || record.lease_id.len() > 256
            || !matches!(record.phase.as_str(), "reserving" | "active" | "retiring")
            || (record.phase == "reserving") != record.lease_id.is_empty()
            || removal == record.paths.is_empty()
            || record.paths.len() > self.policy.max_related_outputs
            || (removal
                && key
                    != format!(
                        "removal-{}",
                        sha256(&serde_json::to_vec(&record.paths).map_err(|e| e.to_string())?)
                    ))
        {
            return Err("retention transaction binding changed; preserve capability".into());
        }
        Ok(Some(record))
    }
    // Every transaction publishes its address gate before its declaration and
    // removes that gate after retiring the declaration. Enumerate that existing
    // namespace, so unrelated output capabilities do not consume the round.
    fn transaction_keys(&self) -> Result<Vec<String>, String> {
        let mut keys = Vec::new();
        for entry in
            fs::read_dir(self.directory.join("transaction-gates")).map_err(|e| e.to_string())?
        {
            let entry = entry.map_err(|e| e.to_string())?;
            keys.push(
                entry
                    .file_name()
                    .into_string()
                    .map_err(|_| "transaction address UTF8")?,
            );
        }
        keys.sort();
        Ok(keys)
    }
    // Birth, capability attachment and retirement exclude only this address.
    fn address_gate(&self, kind: &str, address: &str) -> Result<Lease, String> {
        self.address_gate_scoped(kind, address, &[])
    }
    fn address_gate_scoped(
        &self,
        kind: &str,
        address: &str,
        paths: &[String],
    ) -> Result<Lease, String> {
        let key = format!("{kind}-{}", sha256(address.as_bytes()));
        let directory = no_symlink_parents(&self.directory, &format!("transaction-gates/{key}"))?;
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let gate = crate::ownership::DirectoryLease::acquire(
            &directory,
            true,
            Some(Duration::from_secs(5)),
        )?
        .ok_or("addressed retention transaction busy")?;
        let path = self.directory.join(&key);
        let mut record = match self.transaction(&key)? {
            Some(record) if record.phase == "retiring" => {
                return Err(
                    "addressed transaction retirement incomplete; resume maintenance".into(),
                );
            }
            Some(record) => record,
            None => {
                if path.exists() {
                    return Err("unregistered retention transaction capability; preserve it".into());
                }
                let record = Transaction {
                    schema: "chrono-output-transaction/v1".into(),
                    key: key.clone(),
                    directory_identity: identity(&self.directory)?,
                    lease_id: String::new(),
                    phase: "reserving".into(),
                    paths: paths.into(),
                };
                self.stable(&gate)?;
                write(
                    &self.transaction_path(&key),
                    &serde_json::to_vec(&record).map_err(|e| e.to_string())?,
                )?;
                record
            }
        };
        let lease = Lease::acquire(
            &path,
            (!record.lease_id.is_empty()).then_some(record.lease_id.as_str()),
            record.lease_id.is_empty(),
            true,
            Some(Duration::from_secs(5)),
        )?
        .ok_or("addressed retention transaction busy")?;
        record.lease_id = lease.id().into();
        record.phase = "active".into();
        self.stable(&gate)?;
        write(
            &self.transaction_path(&key),
            &serde_json::to_vec(&record).map_err(|e| e.to_string())?,
        )?;
        Ok(lease)
    }
    /// Explicit owner migration. The caller supplies the exact prior policy bytes;
    /// every existing output must still be declared, and all old producers/readers must be excluded.
    /// This changes no output identity, original outcome, expiry, root, or completion state.
    pub fn migrate_policy(&self, previous_policy: &[u8]) -> Result<Value, String> {
        let previous: Policy = decode(previous_policy)?;
        if previous.schema != self.policy.schema {
            return Err("unsupported retention policy migration".into());
        }
        for key in self.transaction_keys()? {
            let record = self
                .transaction(&key)?
                .ok_or("transaction disappeared during migration")?;
            if record.phase != "active" {
                return Err("recover unattached retention transaction before migration".into());
            }
            let _owner = Lease::acquire(
                &self.directory.join(&key),
                Some(&record.lease_id),
                false,
                true,
                None,
            )?
            .ok_or("live addressed publisher prevents retention migration")?;
        }
        let ids = self.partition_ids()?;
        let previous_digest = sha256(previous_policy);
        let current_digest = sha256(&self.policy_bytes);
        let mut preserved = BTreeSet::new();
        let mut roots = BTreeSet::new();
        // Preflight before changing bindings. Reacquire and revalidate each actual
        // relationship component for the write; never hold unrelated descriptors.
        // A later interruption may leave old/current components, which the same
        // exact migration request already accepts on reentry.
        for publish in [false, true] {
            let mut seen = BTreeSet::new();
            for id in &ids {
                if seen.contains(id) {
                    continue;
                }
                let gate = self.addressed_gate(&[id.clone()], &[], &[], None, true, true)?;
                let mut l = self.load_bound(&gate, &previous_digest)?;
                seen.insert(id.clone());
                seen.extend(l.outputs.iter().map(|output| output.id.clone()));
                let mut owners = Vec::new();
                for output in &l.outputs {
                    if matches!(output.phase.as_str(), "reserving" | "creating") {
                        return Err("recover reserved output before policy migration".into());
                    }
                    owners.push(
                        Lease::acquire(
                            &self.directory.join(&output.lease),
                            Some(&output.lease_id),
                            false,
                            true,
                            None,
                        )?
                        .ok_or("live output producer/consumer prevents retention migration")?,
                    );
                }
                if publish {
                    l.policy_sha256 = current_digest.clone();
                    self.save(&l, &gate)?;
                    preserved.extend(l.outputs.iter().map(|output| output.id.clone()));
                    roots.extend(l.roots.keys().cloned());
                }
            }
        }
        Ok(
            json!({"schema":"chrono-output-policy-migration/v1","previous_policy_sha256":previous_digest,"policy_sha256":current_digest,"outputs_preserved":preserved.len(),"roots_preserved":roots.len(),"effects":"metadata-binding-only","completion":"not-established"}),
        )
    }
    /// Enroll exactly the producer-owned path before writing. Caller must exclude old producers
    /// for legacy enrollment; this operation never discovers historical paths by scanning.
    pub fn begin(self, producer: &str, path: &str, outcome: Value) -> Result<Publication, String> {
        self.begin_rooted(producer, path, outcome, &[])
    }
    pub fn begin_rooted(
        self,
        producer: &str,
        path: &str,
        outcome: Value,
        roots: &[String],
    ) -> Result<Publication, String> {
        self.reserve(producer, path, outcome, roots, None, None)
    }
    /// Durable creation intent. No producer-visible output is created until its
    /// inode has been bound at the exact owner staging slot.
    pub fn plan(
        self,
        producer: &str,
        path: &str,
        tree: bool,
        outcome: Value,
    ) -> Result<Publication, String> {
        self.reserve(producer, path, outcome, &[], Some(tree), None)
    }
    fn reserve(
        self,
        producer: &str,
        path: &str,
        outcome: Value,
        roots: &[String],
        create: Option<bool>,
        requested_limit: Option<u64>,
    ) -> Result<Publication, String> {
        let (publication, _gate, _ledger) =
            self.reserve_excluded(producer, path, outcome, roots, create, requested_limit)?;
        Ok(publication)
    }
    // Reservation persists intent and capability identity under addressed
    // exclusion before the producer can create or change original content.
    fn reserve_excluded(
        self,
        producer: &str,
        path: &str,
        outcome: Value,
        roots: &[String],
        create: Option<bool>,
        requested_limit: Option<u64>,
    ) -> Result<(Publication, Gate, Ledger), String> {
        let byte_limit = requested_limit.unwrap_or(self.policy.max_output_bytes);
        if byte_limit == 0 || byte_limit > self.policy.max_output_bytes {
            return Err("declared output reservation exceeds policy ceiling".into());
        }
        crate::relative_path(path)?;
        if path == STATE || path.starts_with(&format!("{STATE}/")) {
            return Err("retention metadata is never a producer output".into());
        }
        if path.len() > 4096
            || serde_json::to_vec(&outcome)
                .map_err(|e| e.to_string())?
                .len()
                > 16384
        {
            return Err("output identity/outcome metadata bound".into());
        }
        if !self
            .policy
            .producers
            .get(producer)
            .is_some_and(|scopes| scopes.iter().any(|scope| path.starts_with(scope)))
        {
            return Err(format!(
                "unregistered output producer/scope: {producer}: {path}"
            ));
        }
        // Git source/index facts are independent of inventory admission. Query
        // them before serializing the durable reservation; the destination is
        // still checked and published without replacement under owner exclusion.
        crate::artifact_disposal::source_safe(&self.root, path)?;
        let id = unique_path(&self.directory, "output-", "")?
            .file_name()
            .unwrap()
            .to_str()
            .ok_or("output id UTF8")?
            .to_owned();
        let gate =
            self.addressed_gate(&[], &[path.into()], roots, Some((&id, path)), false, true)?;
        let mut l = self.load(&gate)?;
        if l.outputs.iter().any(|o| {
            o.path == path
                || path.starts_with(&format!("{}/", o.path))
                || o.path.starts_with(&format!("{path}/"))
        }) {
            return Err("overlapping/already enrolled output identity".into());
        }
        let physical = no_symlink_parents(&self.root, path)?;
        let (output_identity, tree) = if let Some(tree) = create {
            match fs::symlink_metadata(&physical) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(e.to_string()),
                Ok(_) => return Err("creation destination already exists; preserve it".into()),
            }
            (String::new(), tree)
        } else {
            let m = fs::symlink_metadata(&physical).map_err(|e| e.to_string())?;
            if !m.is_file() && !m.is_dir() {
                return Err("output must be a physical file/directory".into());
            }
            (identity(&physical)?, m.is_dir())
        };
        let lease_path = self.directory.join(&id);
        if lease_path.exists() {
            return Err("reserved output capability already exists; preserve it".into());
        }
        let object = Output {
            id: id.clone(),
            producer: producer.into(),
            path: path.into(),
            identity: output_identity,
            tree,
            sha256: None,
            length: None,
            lease: id.clone(),
            lease_id: String::new(),
            outcome,
            phase: "reserving".into(),
            expires: now()?.saturating_add(self.policy.keep_seconds),
            references: vec![],
            stack: vec![],
            removed_logical_bytes: 0,
            manifest: Manifest::default(),
            reserved_bytes: byte_limit,
            byte_limit: requested_limit,
            creation: if create.is_some() {
                Some(Creation {
                    parent_identity: identity(physical.parent().ok_or("output parent")?)?,
                    staged: format!("creation-{id}"),
                })
            } else {
                None
            },
            overflow_disposal: false,
        };
        for role in roots {
            if role.is_empty()
                || role.len() > 256
                || l.roots.contains_key(role)
                || l.roots.len() >= self.policy.max_related_outputs
            {
                return Err("historical/current role identity collision or capacity".into());
            }
            l.roots.insert(role.clone(), vec![id.clone()]);
        }
        l.outputs.push(object.into());
        // Reservation is durable before the capability file is created. No producer
        // receives a capability until its exact identity has been attached.
        self.save(&l, &gate)?;
        let lease = Lease::acquire(&lease_path, None, true, false, None)?
            .ok_or("output publication busy")?;
        let last = l
            .outputs
            .last_mut()
            .ok_or("publication reservation missing")?;
        last.lease_id = lease.id().into();
        last.phase = if create.is_some() {
            "creating"
        } else {
            "publishing"
        }
        .into();
        self.save(&l, &gate)?;
        Ok((
            Publication {
                inventory: self,
                id,
                lease,
                initial: Arc::clone(&l.outputs.last().ok_or("publication missing")?.value),
                byte_limit,
                path: path.into(),
                identity: RefCell::new(
                    l.outputs
                        .last()
                        .ok_or("publication missing")?
                        .identity
                        .clone(),
                ),
                tree,
                written: Cell::new(0),
                written_nodes: Cell::new(1),
            },
            gate,
            l,
        ))
    }
    fn validate_original_reuse(&self, path: &str) -> Result<Option<String>, String> {
        let gate = self.addressed_gate(&[], &[path.into()], &[], None, false, true)?;
        let l = self.load(&gate)?;
        if let Some(o) = l.outputs.iter().find(|o| o.path == path) {
            if o.phase == "disposing"
                || identity(&no_symlink_parents(&self.root, path)?)? != o.identity
            {
                return Err("retained original unavailable, disposing or replaced".into());
            }
        }
        self.stable(&gate)?;
        Ok(l.outputs
            .iter()
            .find(|o| o.path == path)
            .map(|o| o.id.clone()))
    }
    pub fn enrolled(&self, path: &str) -> Result<bool, String> {
        let gate = self.addressed_gate(&[], &[path.into()], &[], None, false, true)?;
        let l = self.load(&gate)?;
        self.stable(&gate)?;
        Ok(l.outputs.iter().any(|o| o.path == path))
    }
    /// A domain may discharge an interim handoff only from the producer's
    /// sealed original, with matching bytes and the original physical inode.
    /// The caller keeps its read capability through the ensuing handoff.
    pub fn published_original(
        &self,
        path: &str,
        digest: &str,
        length: u64,
    ) -> Result<bool, String> {
        let gate = self.addressed_gate(&[], &[path.into()], &[], None, false, true)?;
        let ledger = self.load(&gate)?;
        let published = match ledger.outputs.iter().find(|o| o.path == path) {
            Some(output) if !output.tree && output.phase == "released" => {
                output.sha256.as_deref() == Some(digest)
                    && output.length == Some(length)
                    && identity(&no_symlink_parents(&self.root, path)?)? == output.identity
            }
            _ => false,
        };
        self.stable(&gate)?;
        Ok(published)
    }
    pub fn accepts(&self, producer: &str, path: &str) -> bool {
        self.policy
            .producers
            .get(producer)
            .is_some_and(|scopes| scopes.iter().any(|scope| path.starts_with(scope)))
    }
    /// A producer selects a finite diagnostic window, including interruption
    /// before completion. Actual caller
    /// and recovery roots remain authoritative and are never released here.
    pub fn retain_diagnostic_path(&self, path: &str) -> Result<(), String> {
        let gate = self.addressed_gate(&[], &[path.into()], &[], None, false, true)?;
        let mut ledger = self.load(&gate)?;
        let output = ledger
            .outputs
            .iter_mut()
            .find(|output| output.path == path)
            .ok_or("diagnostic original is not enrolled")?;
        if !matches!(output.phase.as_str(), "publishing" | "released" | "sealing")
            || identity(&no_symlink_parents(&self.root, path)?)? != output.identity
        {
            return Err("diagnostic original is unfinished, disposing or replaced".into());
        }
        output.expires = output
            .expires
            .max(now()?.saturating_add(self.policy.diagnostic_keep_seconds));
        self.save(&ledger, &gate)
    }
    /// Explicit original addresses supplied by the actual consumer, never inferred from JSON or names.
    pub fn reference_paths(&self, consumer: &str, paths: &[String]) -> Result<(), String> {
        let gate = self.addressed_gate(&[], paths, &[consumer.into()], None, false, true)?;
        let mut l = self.load(&gate)?;
        let mut ids = BTreeSet::new();
        for path in paths {
            crate::relative_path(path)?;
            if let Some(o) = l
                .outputs
                .iter()
                .find(|o| o.path == *path || (o.tree && path.starts_with(&format!("{}/", o.path))))
            {
                if o.phase == "disposing" {
                    return Err("referenced original disposal has started".into());
                }
                ids.insert(o.id.clone());
            } // Unenrolled legacy originals retain their original semantics.
        }
        if consumer.is_empty()
            || consumer.len() > 256
            || ids.len() > self.policy.max_related_outputs
            || (!l.roots.contains_key(consumer) && l.roots.len() >= self.policy.max_related_outputs)
        {
            return Err("retention consumer capacity/identity".into());
        }
        if ids.is_empty() {
            l.roots.remove(consumer);
        } else {
            l.roots.insert(consumer.into(), ids.into_iter().collect());
        }
        self.save(&l, &gate)
    }
    fn extend_reference_path(&self, consumer: &str, path: &str) -> Result<(), String> {
        let gate =
            self.addressed_gate(&[], &[path.into()], &[consumer.into()], None, false, true)?;
        let mut l = self.load(&gate)?;
        let Some(o) = l
            .outputs
            .iter()
            .find(|o| o.path == path && o.phase != "disposing")
        else {
            return Ok(());
        };
        let id = o.id.clone();
        if !l.roots.contains_key(consumer) && l.roots.len() >= self.policy.max_related_outputs {
            return Err("retention consumer capacity".into());
        }
        let ids = l.roots.entry(consumer.into()).or_default();
        if !ids.contains(&id) {
            ids.push(id);
        }
        self.save(&l, &gate)
    }
    /// Bind a retained report to the exact originals its producer already declares.
    pub fn dependencies_paths(&self, path: &str, paths: &[String]) -> Result<(), String> {
        let gate = self.addressed_gate(
            &[],
            &[paths, &[path.into()]].concat(),
            &[],
            None,
            false,
            true,
        )?;
        let mut l = self.load(&gate)?;
        let ids: BTreeSet<_> = l
            .outputs
            .iter()
            .filter(|o| {
                paths
                    .iter()
                    .any(|p| *p == o.path || (o.tree && p.starts_with(&format!("{}/", o.path))))
            })
            .map(|o| o.id.clone())
            .collect();
        if l.outputs
            .iter()
            .any(|o| ids.contains(&o.id) && o.phase == "disposing")
        {
            return Err("report original disposal has started".into());
        }
        if let Some(o) = l.outputs.iter_mut().find(|o| o.path == path) {
            if o.phase == "disposing" {
                return Err("report disposal has started".into());
            }
            o.references = ids.into_iter().collect();
        }
        self.save(&l, &gate)
    }
    /// Consumer roles are explicit roots. Replacing/releasing a role releases its prior closure.
    pub fn reference(&self, consumer: &str, ids: &[String]) -> Result<(), String> {
        if consumer.is_empty() || consumer.len() > 256 {
            return Err("invalid retention consumer role".into());
        }
        let gate = self.addressed_gate(ids, &[], &[consumer.into()], None, false, true)?;
        let mut l = self.load(&gate)?;
        self.reference_locked(&mut l, consumer, ids)?;
        self.save(&l, &gate)
    }
    fn reference_locked(
        &self,
        l: &mut Ledger,
        consumer: &str,
        ids: &[String],
    ) -> Result<(), String> {
        if ids.len() > self.policy.max_related_outputs
            || (ids.is_empty() == false
                && !l.roots.contains_key(consumer)
                && l.roots.len() >= self.policy.max_related_outputs)
        {
            return Err("retention reference capacity".into());
        }
        for id in ids {
            if !l
                .outputs
                .iter()
                .any(|o| o.id == *id && o.phase != "disposing")
            {
                return Err(format!("original output unavailable/disposing: {id}"));
            }
        }
        if ids.is_empty() {
            l.roots.remove(consumer);
        } else {
            l.roots.insert(consumer.into(), ids.into());
        }
        Ok(())
    }
    fn consumer_role(&self, consumer: &str) -> Result<String, String> {
        if consumer.trim().is_empty() || consumer.len() > 247 {
            return Err("invalid retention consumer identity".into());
        }
        Ok(format!("consumer:{consumer}"))
    }
    fn consumer_ids(&self, originals: &[OriginalIdentity]) -> Result<Vec<String>, String> {
        let ids: BTreeSet<_> = originals.iter().map(|o| o.id.clone()).collect();
        if originals.is_empty()
            || originals.len() > self.policy.max_related_outputs
            || ids.len() != originals.len()
        {
            return Err("consumer originals empty, duplicated or over capacity".into());
        }
        for original in originals {
            crate::relative_path(&original.path)?;
            if original.path.len() > 4096
                || original.id.is_empty()
                || original.id.len() > 256
                || original.identity.len() > 256
            {
                return Err("invalid bounded consumer original identity".into());
            }
        }
        Ok(ids.into_iter().collect())
    }
    fn original_leases(
        &self,
        l: &Ledger,
        originals: &[OriginalIdentity],
        exclusive: bool,
    ) -> Result<Vec<Lease>, String> {
        let mut leases = Vec::new();
        for original in originals {
            let o = l
                .outputs
                .iter()
                .find(|o| o.id == original.id)
                .ok_or("required consumer original is unavailable")?;
            if o.path != original.path
                || o.identity != original.identity
                || !matches!(o.phase.as_str(), "publishing" | "sealing" | "released")
            {
                return Err("required consumer original binding/phase changed".into());
            }
            let lease = Lease::acquire(
                &self.directory.join(&o.lease),
                Some(&o.lease_id),
                false,
                exclusive,
                None,
            )?
            .ok_or("required consumer original has live producer/readers")?;
            if identity(&no_symlink_parents(&self.root, &o.path)?)? != original.identity {
                return Err("required consumer original identity changed".into());
            }
            leases.push(lease);
        }
        Ok(leases)
    }
    /// The actual producer names its new diagnostic consumer, bound to this exact output.
    pub fn protect_path_consumer(&self, consumer: &str, path: &str) -> Result<(), String> {
        let gate = self.addressed_gate(&[], &[path.into()], &[], None, false, true)?;
        let ledger = self.load(&gate)?;
        let Some(output) = ledger.outputs.iter().find(|o| o.path == path) else {
            return Ok(());
        };
        let originals = vec![OriginalIdentity {
            id: output.id.clone(),
            path: output.path.clone(),
            identity: output.identity.clone(),
        }];
        drop(gate);
        self.protect_consumer(consumer, &originals)
    }
    /// Durable roots survive interrupted repair work; ordinary entry never guesses release.
    /// Existing roots can only be repeated, never silently replaced by a smaller declaration.
    pub fn protect_consumer(
        &self,
        consumer: &str,
        originals: &[OriginalIdentity],
    ) -> Result<(), String> {
        let role = self.consumer_role(consumer)?;
        let ids = self.consumer_ids(originals)?;
        let gate = self.addressed_gate(&ids, &[], &[role.clone()], None, false, true)?;
        let mut l = self.load(&gate)?;
        if l.roots.get(&role).is_some_and(|previous| previous != &ids) {
            return Err("consumer root differs; release its original declaration first".into());
        }
        let _leases = self.original_leases(&l, originals, false)?;
        self.reference_locked(&mut l, &role, &ids)?;
        self.save(&l, &gate)
    }
    /// Transfer an actual caller's root to its already protected output before
    /// releasing the producer handoff. No historical consumer is inferred.
    pub fn transfer_consumer(&self, consumer: &Consumer, destination: &str) -> Result<(), String> {
        self.transfer_consumer_to(consumer, destination, None)
    }
    fn transfer_consumer_to(
        &self,
        consumer: &Consumer,
        destination: &str,
        publication: Option<&Publication>,
    ) -> Result<(), String> {
        if consumer.schema != "chrono-retention-consumer/v1" || consumer.purpose.trim().is_empty() {
            return Err("invalid caller original handoff".into());
        }
        let role = self.consumer_role(&consumer.consumer)?;
        let ids = self.consumer_ids(&consumer.originals)?;
        let gate = self.addressed_gate(
            &ids,
            &[destination.into()],
            &[role.clone()],
            None,
            false,
            true,
        )?;
        let mut l = self.load(&gate)?;
        if l.roots.get(&role) != Some(&ids) {
            return Err("caller original handoff root differs".into());
        }
        let leases = self.original_leases(&l, &consumer.originals, false)?;
        let target = l
            .outputs
            .iter()
            .position(|o| o.path == destination && o.phase != "disposing")
            .ok_or("caller handoff destination unavailable")?;
        let output = &l.outputs[target];
        let protected = if let Some(publication) = publication {
            publication.lease.stable()?;
            if output.id != publication.id
                || output.identity != *publication.identity.borrow()
                || output.lease != publication.initial.lease
                || output.lease_id != publication.lease.id()
                || identity(&self.root.join(destination))? != output.identity
            {
                return Err("caller handoff publication binding changed".into());
            }
            true
        } else {
            l.roots.values().any(|roots| roots.contains(&output.id))
        };
        if ids.contains(&output.id) || !protected {
            return Err("caller handoff requires a distinct protected destination".into());
        }
        let references: BTreeSet<_> = l.outputs[target]
            .references
            .iter()
            .chain(&ids)
            .cloned()
            .collect();
        if references.len() > self.policy.max_related_outputs {
            return Err("caller handoff dependency capacity".into());
        }
        l.outputs[target].references = references.into_iter().collect();
        l.roots.remove(&role);
        self.save(&l, &gate)?;
        for lease in &leases {
            self.remember_input(lease)?;
        }
        Ok(())
    }
    /// Called after the responsible consumer's exact, external domain receipt is verified.
    /// Never changes expiry or original outcomes and never declares work complete.
    pub fn release_consumer(
        &self,
        consumer: &str,
        originals: &[OriginalIdentity],
        declaration_digest: &str,
        receipt_digest: &str,
    ) -> Result<Value, String> {
        if !crate::wire::is_digest(declaration_digest) || !crate::wire::is_digest(receipt_digest) {
            return Err("consumer release evidence digest invalid".into());
        }
        let role = self.consumer_role(consumer)?;
        let ids = self.consumer_ids(originals)?;
        let gate = self.addressed_gate(&ids, &[], &[role.clone()], None, false, true)?;
        let mut l = self.load(&gate)?;
        if !l.roots.contains_key(&role) {
            let receipt = l
                .summaries
                .iter()
                .find(|s| {
                    s["schema"] == "chrono-retention-consumer-release/v1"
                        && s["consumer"] == consumer
                        && s["declaration_sha256"] == declaration_digest
                        && s["receipt_sha256"] == receipt_digest
                })
                .ok_or("consumer protection/release receipt unavailable")?
                .clone();
            self.stable(&gate)?;
            return Ok(receipt);
        }
        if l.roots.get(&role) != Some(&ids) {
            return Err("consumer release root differs from exact declaration".into());
        }
        let _leases = self.original_leases(&l, originals, true)?;
        self.reference_locked(&mut l, &role, &[])?;
        let receipt = json!({"schema":"chrono-retention-consumer-release/v1","consumer":consumer,"declaration_sha256":declaration_digest,"receipt_sha256":receipt_digest,"original_count":ids.len(),"effects":"consumer-root-release-only","disposal":"not-attempted","completion":"not-established"});
        l.summaries.push(receipt.clone());
        let excess = l.summaries.len().saturating_sub(self.policy.max_summaries);
        l.summaries.drain(..excess);
        self.save(&l, &gate)?;
        Ok(receipt)
    }
    /// A terminal caller preserves an exact original outside the retired host.
    /// This ends only its explicitly named roots; maintenance still owns disposal.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn release_preserved(
        &self,
        original: &OriginalIdentity,
        roles: &[String],
        survivor_root: &Path,
        survivor_path: &str,
        digest: &str,
        length: u64,
        current_consumers_released: bool,
        reason: &str,
    ) -> Result<Value, String> {
        if !crate::verified_release::valid_request(
            roles,
            self.policy.max_related_outputs,
            reason.trim(),
            reason.len(),
            current_consumers_released,
            length,
            self.policy.max_output_bytes,
        ) || !crate::wire::is_digest(digest)
            || !survivor_root.is_absolute()
            || fs::canonicalize(survivor_root).map_err(|e| e.to_string())? != survivor_root
        {
            return Err("preserved release requires exact roots, external original and explicit current consumer release".into());
        }
        crate::relative_path(&original.path)?;
        let survivor = no_symlink_parents(survivor_root, survivor_path)?;
        if survivor.starts_with(&self.root) {
            return Err("preserved original must survive outside the retired host".into());
        }
        let verify = |path: &Path| -> Result<String, String> {
            let before = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
            if !before.is_file() || before.file_type().is_symlink() || before.len() != length {
                return Err(
                    "preserved release requires a physical file with the declared length".into(),
                );
            }
            let signature = identity(path)?;
            let file = fs::File::open(path).map_err(|e| e.to_string())?;
            if identity(path)? != signature
                || crate::copy_hashed(&file, std::io::sink())? != (digest.into(), length)
                || identity(path)? != signature
                || before.modified().map_err(|e| e.to_string())?
                    != fs::symlink_metadata(path)
                        .map_err(|e| e.to_string())?
                        .modified()
                        .map_err(|e| e.to_string())?
            {
                return Err("preserved original bytes or identity changed".into());
            }
            Ok(signature)
        };
        let survivor_identity = verify(&survivor)?;
        let receipt = json!({"schema":"chrono-preserved-original-release/v1","id":original.id,
            "original":original,"roles":roles,"survivor_root":survivor_root,
            "survivor_path":survivor_path,"survivor_identity":survivor_identity,
            "sha256":digest,"length":length,"reason":reason,
            "effects":"consumer-root-release-only","disposal":"not-attempted",
            "completion":"not-established"});
        let gate = self.addressed_gate(
            &[original.id.clone()],
            &[original.path.clone()],
            roles,
            None,
            false,
            true,
        )?;
        let mut ledger = self.load(&gate)?;
        if ledger.summaries.iter().any(|s| s == &receipt) {
            self.stable(&gate)?;
            return Ok(receipt);
        }
        let output = ledger
            .outputs
            .iter()
            .find(|o| o.id == original.id)
            .ok_or("preserved original is unavailable")?;
        if output.tree {
            return Err(
                "preserved release requires an explicitly retained regular original".into(),
            );
        }
        let actual: Vec<_> = ledger
            .roots
            .iter()
            .filter(|(_, ids)| crate::verified_release::contains(ids, &original.id))
            .map(|(role, _)| role.clone())
            .collect();
        if !crate::verified_release::same_members(&actual, roles) {
            return Err(
                "preserved release roots differ from the current original consumers".into(),
            );
        }
        let leases = self.original_leases(&ledger, std::slice::from_ref(original), true)?;
        let source = no_symlink_parents(&self.root, &original.path)?;
        if verify(&source)? != original.identity || verify(&survivor)? != survivor_identity {
            return Err("preserved release original identity changed".into());
        }
        for role in roles {
            let remaining = crate::verified_release::without(&ledger.roots[role], &original.id);
            self.reference_locked(&mut ledger, role, &remaining)?;
        }
        for lease in &leases {
            lease.stable()?;
        }
        self.stable(&gate)?;
        ledger.summaries.push(receipt.clone());
        let excess = ledger
            .summaries
            .len()
            .saturating_sub(self.policy.max_summaries);
        ledger.summaries.drain(..excess);
        self.save(&ledger, &gate)?;
        Ok(receipt)
    }
    /// Edges are not roots. Released historical cycles can be reclaimed.
    pub fn dependencies(&self, id: &str, references: &[String]) -> Result<(), String> {
        let gate = self.addressed_gate(
            &[references, &[id.into()]].concat(),
            &[],
            &[],
            None,
            false,
            true,
        )?;
        let mut l = self.load(&gate)?;
        if references.len() > self.policy.max_related_outputs
            || references.iter().any(|id| {
                !l.outputs
                    .iter()
                    .any(|o| o.id == *id && o.phase != "disposing")
            })
        {
            return Err("unavailable output dependency".into());
        }
        let o = l
            .outputs
            .iter_mut()
            .find(|o| o.id == id && o.phase != "disposing")
            .ok_or("output unavailable/disposing")?;
        o.references = references.into();
        self.save(&l, &gate)
    }
    pub fn maintain(&self) -> Result<Value, String> {
        self.maintain_at(now()?)
    }
    /// Timestamp is a supplied observation for deterministic owner tests, not a live-user heuristic.
    pub fn maintain_at(&self, observed: u64) -> Result<Value, String> {
        self.round_metadata
            .set(Some(self.policy.max_metadata_bytes_per_round));
        let result = self.maintain_round(observed, None);
        self.round_metadata.set(None);
        result
    }
    /// A caller can name its actual remaining originals without re-enumerating
    /// unrelated diagnostic metadata. The same limits and dependency guards apply.
    pub fn maintain_originals(&self, originals: &[String]) -> Result<Value, String> {
        if !crate::verified_release::unique_selection(originals, self.policy.max_related_outputs) {
            return Err(
                "explicit maintenance originals are empty, duplicated or over capacity".into(),
            );
        }
        for id in originals {
            let path = self.object_directory(id)?.join("record.json");
            if !fs::symlink_metadata(path)
                .map_err(|e| e.to_string())?
                .is_file()
            {
                return Err("explicit maintenance original declaration is unavailable".into());
            }
        }
        self.round_metadata
            .set(Some(self.policy.max_metadata_bytes_per_round));
        let result = self.maintain_round(now()?, Some(originals));
        self.round_metadata.set(None);
        result
    }
    /// Ordinary entries coalesce concurrent reclamation without waiting behind
    /// another bounded pass. Publication still acquires authoritative admission.
    pub fn maintain_available(&self) -> Result<Value, String> {
        self.maintain()
    }
    fn maintain_round(&self, observed: u64, originals: Option<&[String]>) -> Result<Value, String> {
        self.round_began.set(Some(Instant::now()));
        self.round_node_limit
            .set(Some(self.policy.max_nodes_per_round));
        let result = self.partition_round(observed, originals);
        self.round_node_limit.set(None);
        self.round_began.set(None);
        result
    }
    /// Under admission exclusion, only reverse-reachable outputs can still
    /// consume this original. Keep their exclusive capabilities through the
    /// physical effect; an unknown or live consumer preserves the candidate.
    fn exclude_consumers(
        &self,
        l: &Ledger,
        id: &str,
        consumers: &BTreeMap<String, Vec<String>>,
        began: Instant,
    ) -> Result<Vec<Lease>, String> {
        let mut seen = BTreeSet::from([id.to_owned()]);
        let mut pending = consumers.get(id).cloned().unwrap_or_default();
        let mut leases = Vec::new();
        while let Some(consumer) = pending.pop() {
            if !seen.insert(consumer.clone()) {
                continue;
            }
            let Some(output) = l.outputs.iter().find(|o| o.id == consumer) else {
                continue; // This consumer was already retired in the current round.
            };
            if began.elapsed().as_millis() >= self.policy.max_millis_per_round as u128 {
                return Err("bounded consumer exclusion; original preserved".into());
            }
            let lease = Lease::acquire(
                &self.directory.join(&output.lease),
                Some(&output.lease_id),
                false,
                true,
                None,
            )?
            .ok_or("live producer/consumer descendants")?;
            leases.push(lease);
            pending.extend(consumers.get(&consumer).into_iter().flatten().cloned());
        }
        Ok(leases)
    }
    fn retire_transaction(&self, key: &str) -> Result<(), String> {
        let directory = no_symlink_parents(&self.directory, &format!("transaction-gates/{key}"))?;
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let Some(gate) = crate::ownership::DirectoryLease::acquire(&directory, true, None)? else {
            return Ok(());
        };
        let Some(mut record) = self.transaction(key)? else {
            return Ok(());
        };
        let path = self.directory.join(key);
        match fs::symlink_metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.to_string()),
            Ok(_) => {
                let Some(owner) = Lease::acquire(
                    &path,
                    (!record.lease_id.is_empty()).then_some(record.lease_id.as_str()),
                    false,
                    true,
                    None,
                )?
                else {
                    return Ok(());
                };
                record.lease_id = owner.id().into();
                record.phase = "retiring".into();
                self.stable(&gate)?;
                let bytes = serde_json::to_vec(&record).map_err(|e| e.to_string())?;
                self.charge_metadata(bytes.len() as u64)?;
                write(&self.transaction_path(key), &bytes)?;
                crate::artifact_disposal::dispose_entry(
                    &self.directory,
                    key,
                    &identity(&path)?,
                    || {
                        self.stable(&gate)?;
                        owner.stable()
                    },
                )?;
            }
        }
        let declaration = self.transaction_path(key);
        crate::artifact_disposal::dispose_entry(
            &self.directory,
            declaration.file_name().unwrap().to_str().unwrap(),
            &identity(&declaration)?,
            || self.stable(&gate),
        )?;
        fs::remove_dir(&directory).map_err(|e| e.to_string())
    }
    fn maintain_locked(&self, observed: u64, gate: &Gate) -> Result<Value, String> {
        let began = self.round_began.get().unwrap_or_else(Instant::now);
        let mut l = self.load(&gate)?;
        let mut metadata_steps = 0usize;
        let mut accounting_errors = Vec::new();
        // Recover only explicit reservations; never discover unknown files by scanning.
        for index in 0..l.outputs.len() {
            if metadata_steps
                >= self
                    .round_node_limit
                    .get()
                    .unwrap_or(self.policy.max_nodes_per_round)
                || !self.metadata_room()
                || began.elapsed().as_millis() >= self.policy.max_millis_per_round as u128
            {
                break;
            }
            if l.outputs[index].phase == "reserving" {
                metadata_steps += 1;
                let path = self.directory.join(&l.outputs[index].lease);
                if let Some(capability) = Lease::acquire(&path, None, true, true, None)? {
                    l.outputs[index].lease_id = capability.id().into();
                    l.outputs[index].phase = if l.outputs[index].creation.is_some() {
                        "creating"
                    } else {
                        "publishing"
                    }
                    .into();
                    self.save(&l, &gate)?;
                }
            }
            if l.outputs[index].phase == "creating"
                && metadata_steps
                    < self
                        .round_node_limit
                        .get()
                        .unwrap_or(self.policy.max_nodes_per_round)
                && self.metadata_room()
                && began.elapsed().as_millis() < self.policy.max_millis_per_round as u128
            {
                let output = l.outputs[index].clone();
                if let Ok(Some(owner)) = Lease::acquire(
                    &self.directory.join(&output.lease),
                    Some(&output.lease_id),
                    false,
                    true,
                    None,
                ) {
                    metadata_steps += 1;
                    match self.recover_creation(&mut l.outputs[index], &gate, &owner) {
                        Ok(()) => self.save(&l, &gate)?,
                        Err(error) => {
                            accounting_errors.push(json!({"id":output.id,"reason":error}))
                        }
                    }
                }
            }
        }
        while metadata_steps
            < self
                .round_node_limit
                .get()
                .unwrap_or(self.policy.max_nodes_per_round)
            && !l.retired_leases.is_empty()
            && self.metadata_room()
            && began.elapsed().as_millis() < self.policy.max_millis_per_round as u128
        {
            metadata_steps += 1;
            let pending = l.retired_leases[0].clone();
            let path = self.directory.join(&pending.path);
            match fs::symlink_metadata(&path) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(e.to_string()),
                Ok(_) => crate::artifact_disposal::dispose_entry(
                    &self.directory,
                    &pending.path,
                    &pending.identity,
                    || self.stable(gate),
                )?,
            }
            l.retired_leases.remove(0);
            self.save(&l, &gate)?;
        }
        // Accounting is independent of retirement eligibility: a current root
        // may retain a sealed output, but must not defer byte accounting forever.
        // Legacy entries reserve the policy ceiling until this bounded pass seals them.
        let account_order: Vec<_> = (0..l.outputs.len())
            .map(|n| (l.cursor + n) % l.outputs.len().max(1))
            .collect();
        for index in account_order {
            if !self.metadata_room()
                || metadata_steps
                    >= self
                        .round_node_limit
                        .get()
                        .unwrap_or(self.policy.max_nodes_per_round)
                || began.elapsed().as_millis() >= self.policy.max_millis_per_round as u128
            {
                break;
            }
            if l.outputs[index].manifest.sealed
                || l.outputs[index].overflow_disposal
                || matches!(l.outputs[index].phase.as_str(), "reserving" | "creating")
            {
                continue;
            }
            let output = l.outputs[index].clone();
            let Ok(Some(_owner)) = Lease::acquire(
                &self.directory.join(&output.lease),
                Some(&output.lease_id),
                false,
                true,
                None,
            ) else {
                continue;
            };
            // A publishing producer which vanished retains its unknown original outcome.
            if l.outputs[index].phase == "publishing" {
                l.outputs[index].outcome = json!({"original":output.outcome,"publication":"interrupted","completion":"not-established"});
                l.outputs[index].phase = "sealing".into();
            }
            while self.metadata_room()
                && !l.outputs[index].manifest.sealed
                && metadata_steps
                    < self
                        .round_node_limit
                        .get()
                        .unwrap_or(self.policy.max_nodes_per_round)
                && began.elapsed().as_millis() < self.policy.max_millis_per_round as u128
            {
                metadata_steps += 1;
                if let Err(error) = l.outputs[index].manifest.step(
                    &self.root,
                    &output.path,
                    self.policy.max_manifest_nodes,
                    output.byte_limit.unwrap_or(self.policy.max_output_bytes),
                ) {
                    accounting_errors.push(json!({"id":output.id,"reason":error}));
                    break;
                }
                if l.outputs[index].manifest.sealed {
                    l.outputs[index].reserved_bytes = l.outputs[index].manifest.bytes;
                }
                self.save(&l, &gate)?;
            }
            l.cursor = (index + 1) % l.outputs.len().max(1);
        }
        let mut rooted = BTreeSet::new();
        let mut protection_reasons = BTreeMap::new();
        let mut pending: Vec<_> = l.roots.values().flatten().cloned().collect();
        for o in &l.outputs {
            if matches!(o.phase.as_str(), "reserving" | "creating") {
                pending.push(o.id.clone());
            }
            // A finite window retains bytes that still exist; it cannot turn an
            // already removed successful fixture into permanent metadata.
            if o.expires > observed {
                let absent = matches!(fs::symlink_metadata(self.root.join(&o.path)),Err(ref e) if e.kind()==std::io::ErrorKind::NotFound);
                if !absent {
                    protection_reasons.insert(o.id.clone(), "retention window".to_string());
                    pending.push(o.id.clone());
                }
            }
        }
        while let Some(id) = pending.pop() {
            if rooted.insert(id.clone()) {
                if let Some(o) = l.outputs.iter().find(|o| o.id == id) {
                    pending.extend(o.references.clone());
                }
            }
        }
        let mut nodes = metadata_steps;
        let mut bytes = 0u64;
        let mut verified_bytes = 0u64;
        let mut protected = vec![];
        let mut disposed = vec![];
        let count = l.outputs.len();
        let start = if count == 0 { 0 } else { l.cursor % count };
        // Retire an unrooted report before the originals it references. This
        // spends a bounded round on retiring the consumer first and prevents
        // an obsolete report from outliving reclaimed inputs. Continue partial
        // disposal before opening new candidates, and prefer independent
        // continuations so a dependency closure cannot consume the whole
        // admission window. Stable sorting preserves the durable cursor among
        // peers; cycles remain reclaimable.
        let referenced: BTreeSet<_> = l.outputs.iter().flat_map(|o| o.references.iter()).collect();
        let mut consumers: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for output in &l.outputs {
            for original in &output.references {
                consumers
                    .entry(original.clone())
                    .or_default()
                    .push(output.id.clone());
            }
        }
        // Compute priorities from each existing output once. Looking up the
        // output inside every comparison makes preparation quadratic and can
        // exhaust a bounded round before any retirement is attempted.
        let mut order: Vec<_> = (0..count)
            .map(|n| {
                let output = &l.outputs[(start + n) % count];
                (
                    output.id.clone(),
                    (
                        output.phase != "disposing",
                        consumers.contains_key(&output.id) || !output.references.is_empty(),
                        referenced.contains(&output.id),
                    ),
                )
            })
            .collect();
        order.sort_by_key(|(_, priority)| *priority);
        for (id, _) in order {
            if !self.metadata_room()
                || nodes
                    >= self
                        .round_node_limit
                        .get()
                        .unwrap_or(self.policy.max_nodes_per_round)
                || began.elapsed().as_millis() >= self.policy.max_millis_per_round as u128
            {
                break;
            }
            let i = l
                .outputs
                .iter()
                .position(|o| o.id == id)
                .ok_or("output disappeared")?;
            l.cursor = (i + 1) % count.max(1);
            if rooted.contains(&id) {
                protected.push(
                    json!({"id":id,"reason":protection_reasons.get(&id).map(String::as_str).unwrap_or("current consumer/reference closure")}),
                );
                continue;
            }
            nodes += 1;
            let o = l.outputs[i].clone();
            let lease_path = self.directory.join(&o.lease);
            let lease = match Lease::acquire(&lease_path, Some(&o.lease_id), false, true, None) {
                Ok(Some(v)) => v,
                Ok(None) => {
                    protected.push(json!({"id":id,"reason":"live producer/consumer descendants"}));
                    continue;
                }
                Err(e) => {
                    protected.push(json!({"id":id,"reason":e}));
                    continue;
                }
            };
            let consumer_leases = match self.exclude_consumers(&l, &id, &consumers, began) {
                Ok(leases) => leases,
                Err(error) => {
                    protected.push(json!({"id":id,"reason":error}));
                    continue;
                }
            };
            let mut disposal_fact = "verified-absent";
            let effect = (|| -> Result<bool, String> {
                let root = no_symlink_parents(&self.root, &o.path)?;
                match fs::symlink_metadata(&root) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        disposal_fact = if o.phase == "disposing" {
                            "absent-after-partial-disposal"
                        } else {
                            "already-absent-before-disposal"
                        };
                        return Ok(true);
                    }
                    Err(e) => return Err(e.to_string()),
                    Ok(_) => (),
                }
                if identity(&root)? != o.identity {
                    return Err("output identity changed; preserve replacement".into());
                }
                if l.outputs[i].phase == "publishing" {
                    l.outputs[i].outcome = json!({"original":o.outcome,"publication":"interrupted","completion":"not-established"});
                    l.outputs[i].phase = "sealing".into();
                }
                while self.metadata_room()
                    && !l.outputs[i].manifest.sealed
                    && !l.outputs[i].overflow_disposal
                    && nodes
                        < self
                            .round_node_limit
                            .get()
                            .unwrap_or(self.policy.max_nodes_per_round)
                    && began.elapsed().as_millis() < self.policy.max_millis_per_round as u128
                {
                    nodes += 1;
                    if let Err(error) = l.outputs[i].manifest.step(
                        &self.root,
                        &o.path,
                        self.policy.max_manifest_nodes,
                        self.policy.max_output_bytes,
                    ) {
                        if o.tree && error.contains("capacity") {
                            l.outputs[i].overflow_disposal = true;
                            l.outputs[i].phase = "disposing".into();
                            self.save(&l, &gate)?;
                            break;
                        }
                        return Err(error);
                    }
                    self.save(&l, &gate)?;
                }
                if l.outputs[i].overflow_disposal {
                    while self.metadata_room()
                        && nodes
                            < self
                                .round_node_limit
                                .get()
                                .unwrap_or(self.policy.max_nodes_per_round)
                        && began.elapsed().as_millis() < self.policy.max_millis_per_round as u128
                    {
                        nodes += 1;
                        if l.outputs[i].stack.is_empty() {
                            let entry = manifest::observe(&self.root, &o.path)?;
                            l.outputs[i].stack.push(Node {
                                path: o.path.clone(),
                                identity: o.identity.clone(),
                                entry: Some(entry),
                            });
                            self.save(&l, &gate)?;
                        }
                        let pending = l.outputs[i]
                            .stack
                            .last()
                            .cloned()
                            .ok_or("overflow disposal cursor")?;
                        let physical =
                            crate::artifact_disposal::entry_path(&self.root, &pending.path)?;
                        match fs::symlink_metadata(&physical) {
                            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                                l.outputs[i].stack.pop();
                                self.save(&l, &gate)?;
                                if pending.path == o.path {
                                    return Ok(true);
                                }
                                continue;
                            }
                            Err(e) => return Err(e.to_string()),
                            Ok(_) => (),
                        }
                        if identity(&physical)? != pending.identity {
                            return Err(
                                "overflow descendant identity changed; preserve replacement".into(),
                            );
                        }
                        let entry = pending
                            .entry
                            .as_ref()
                            .ok_or("overflow cursor lacks original entry")?;
                        Manifest::validate(entry, &self.root)?;
                        if let Some(known) = l.outputs[i]
                            .manifest
                            .entries
                            .iter()
                            .find(|e| e.path == pending.path)
                        {
                            Manifest::validate(known, &self.root)?;
                        }
                        if entry.directory {
                            if let Some(child) =
                                fs::read_dir(&physical).map_err(|e| e.to_string())?.next()
                            {
                                if l.outputs[i].stack.len() >= 128 {
                                    return Err(
                                        "overflow disposal depth capacity; preserve remainder"
                                            .into(),
                                    );
                                }
                                let name = child
                                    .map_err(|e| e.to_string())?
                                    .file_name()
                                    .into_string()
                                    .map_err(|_| "overflow path UTF8")?;
                                let child = manifest::observe(
                                    &self.root,
                                    &format!("{}/{name}", pending.path),
                                )?;
                                l.outputs[i].stack.push(Node {
                                    path: child.path.clone(),
                                    identity: child.identity.clone(),
                                    entry: Some(child),
                                });
                                self.save(&l, &gate)?;
                                continue;
                            }
                        }
                        crate::artifact_disposal::dispose_entry(
                            &self.root,
                            &pending.path,
                            &pending.identity,
                            || {
                                self.stable(gate)?;
                                lease.stable()?;
                                for consumer in &consumer_leases {
                                    consumer.stable()?;
                                }
                                crate::artifact_disposal::source_safe(&self.root, &o.path)?;
                                if identity(&root)? != o.identity {
                                    return Err("output changed before overflow disposal".into());
                                }
                                Manifest::validate(entry, &self.root)
                            },
                        )?;
                        bytes = bytes.saturating_add(entry.bytes);
                        l.outputs[i].removed_logical_bytes = l.outputs[i]
                            .removed_logical_bytes
                            .saturating_add(entry.bytes);
                        l.outputs[i].reserved_bytes =
                            l.outputs[i].reserved_bytes.saturating_sub(entry.bytes);
                        l.outputs[i]
                            .manifest
                            .entries
                            .retain(|e| e.path != pending.path);
                        l.outputs[i].stack.pop();
                        self.save(&l, &gate)?;
                        if pending.path == o.path {
                            return Ok(true);
                        }
                    }
                    return Ok(false);
                }
                if !l.outputs[i].manifest.sealed {
                    return Ok(false);
                }
                l.outputs[i].reserved_bytes = l.outputs[i]
                    .manifest
                    .bytes
                    .saturating_sub(l.outputs[i].removed_logical_bytes);
                let entered_disposal = l.outputs[i].phase != "disposing";
                l.outputs[i].phase = "disposing".into();
                // A regular output already in disposal has a durable phase and
                // manifest. Defer the replacement write until its bounded
                // physical effect (or the round's final save), avoiding a full
                // inventory sync before every one-file retirement.
                if entered_disposal || o.tree {
                    self.save(&l, &gate)?;
                }
                while self.metadata_room()
                    && nodes
                        < self
                            .round_node_limit
                            .get()
                            .unwrap_or(self.policy.max_nodes_per_round)
                    && began.elapsed().as_millis() < self.policy.max_millis_per_round as u128
                {
                    let Some(entry) = l.outputs[i].manifest.entries.last().cloned() else {
                        return Ok(true);
                    };
                    nodes += 1;
                    let path = crate::artifact_disposal::entry_path(&self.root, &entry.path)?;
                    match fs::symlink_metadata(&path) {
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            l.outputs[i].manifest.entries.pop();
                            if o.tree {
                                self.save(&l, &gate)?;
                            }
                            continue;
                        }
                        Err(e) => return Err(e.to_string()),
                        Ok(_) => (),
                    }
                    Manifest::validate(&entry, &self.root)?;
                    if !o.tree
                        && o.sha256.is_some()
                        && entry.bytes <= self.policy.max_bytes_per_round
                        && entry.bytes
                            > self
                                .round_verified_left
                                .get()
                                .unwrap_or(self.policy.max_bytes_per_round)
                                .saturating_sub(verified_bytes)
                    {
                        return Err(
                            "output exceeds remaining round byte bound; preserve remainder".into(),
                        );
                    }
                    if !o.tree
                        && o.sha256.is_some()
                        && entry.bytes <= self.policy.max_bytes_per_round
                    {
                        verified_bytes += entry.bytes;
                        if let Some(expected) = &o.sha256 {
                            if crate::file_identity(&path)?
                                != (expected.clone(), o.length.ok_or("output length")?)
                            {
                                return Err("original output content changed".into());
                            }
                        }
                    }
                    crate::artifact_disposal::dispose_entry(
                        &self.root,
                        &entry.path,
                        &entry.identity,
                        || {
                            self.stable(gate)?;
                            lease.stable()?;
                            for consumer in &consumer_leases {
                                consumer.stable()?;
                            }
                            crate::artifact_disposal::source_safe(&self.root, &o.path)?;
                            Manifest::validate(&entry, &self.root)?;
                            if identity(&root)? != o.identity {
                                return Err("output changed before disposal".into());
                            }
                            Ok(())
                        },
                    )?;
                    bytes = bytes.saturating_add(entry.bytes);
                    l.outputs[i].removed_logical_bytes = l.outputs[i]
                        .removed_logical_bytes
                        .saturating_add(entry.bytes);
                    l.outputs[i].reserved_bytes =
                        l.outputs[i].reserved_bytes.saturating_sub(entry.bytes);
                    l.outputs[i].manifest.entries.pop();
                    // Missing regular entries are safely reconciled from the
                    // durable disposing manifest on the next round. Tree and
                    // partial-disposal cursors retain their per-step writes.
                    if o.tree {
                        self.save(&l, &gate)?;
                    }
                }
                Ok(l.outputs[i].manifest.entries.is_empty())
            })();
            match effect {
                Ok(true) => {
                    let o = Arc::unwrap_or_clone(l.outputs.remove(i).value);
                    let summary = json!({"id":o.id,"producer":o.producer,"path":o.path,"identity":o.identity,"sha256":o.sha256,"original_outcome":o.outcome,"original":disposal_fact,"removed_logical_bytes":o.removed_logical_bytes,"physical_bytes_reclaimed":null,"large_original_recheck":if o.length.is_some_and(|n|n>self.policy.max_bytes_per_round) {"sealed-identity-and-content-stamp; digest-not-reread"} else {"not-applicable"}});
                    disposed.push(summary.clone());
                    l.summaries.push(summary);
                    let excess = l.summaries.len().saturating_sub(self.policy.max_summaries);
                    l.summaries.drain(..excess);
                    for output in &mut l.outputs {
                        output.references.retain(|r| r != &id);
                    }
                    l.retired_leases.push(Node {
                        path: o.lease,
                        identity: identity(&lease_path)?,
                        entry: None,
                    });
                    self.save(&l, &gate)?;
                    drop(lease);
                    // The explicit retirement queue survives interruption at either side
                    // of unlink. Its size participates in admission capacity.
                    if nodes
                        < self
                            .round_node_limit
                            .get()
                            .unwrap_or(self.policy.max_nodes_per_round)
                    {
                        nodes += 1;
                        crate::artifact_disposal::dispose_entry(
                            &self.directory,
                            &l.retired_leases.last().ok_or("retirement pending")?.path,
                            &l.retired_leases.last().unwrap().identity,
                            || self.stable(gate),
                        )?;
                        l.retired_leases.pop();
                        self.save(&l, &gate)?;
                    }
                }
                Ok(false) => protected.push(
                    json!({"id":id,"reason":"bounded partial disposal; resumes on ordinary entry"}),
                ),
                Err(error) => protected.push(json!({"id":id,"reason":error})),
            }
        }
        self.save(&l, &gate)?;
        Ok(
            json!({"schema":"chrono-output-retention-round/v1","nodes":nodes,"metadata_inventory_entries":count,"accounting_errors":accounting_errors,"metadata_graph_edges":l.outputs.iter().map(|o|o.references.len()).sum::<usize>(),"metadata_read_bound":MAX_STATE,"metadata_bytes":self.policy.max_metadata_bytes_per_round-self.round_metadata.get().unwrap_or(0),"max_metadata_bytes_per_round":self.policy.max_metadata_bytes_per_round,"directory_snapshot_entries_bound":self.policy.max_manifest_nodes,"removed_logical_bytes":bytes,"verified_bytes":verified_bytes,"physical_bytes_reclaimed":null,"remaining_outputs":l.outputs.len(),"remaining_capability_retirements":l.retired_leases.len(),"known_retained_file_logical_bytes":l.outputs.iter().filter_map(|o|o.length).sum::<u64>(),"known_retained_allocated_bytes":l.outputs.iter().flat_map(|o|o.manifest.entries.iter()).filter_map(|e|e.allocated_bytes).sum::<u64>(),"unmeasured_allocated_outputs":l.outputs.iter().filter(|o|!o.manifest.sealed || o.manifest.entries.iter().any(|e|e.allocated_bytes.is_none())).count(),"unmeasured_tree_outputs":l.outputs.iter().filter(|o|!o.manifest.sealed).count(),"reserved_logical_bytes":l.outputs.iter().map(|o|o.reserved_bytes).sum::<u64>(),"overflow_outputs":l.outputs.iter().filter(|o|o.manifest.bytes.saturating_sub(o.removed_logical_bytes)>o.byte_limit.unwrap_or(self.policy.max_output_bytes)).count(),"max_output_bytes":self.policy.max_output_bytes,"inventory_capacity":self.policy.max_related_outputs,"protected":protected,"disposed":disposed,"legacy_unenrolled":"not-scanned; owner migration required","completed_work":"not-claimed"}),
        )
    }
    fn recover_creation(
        &self,
        output: &mut Output,
        gate: &Gate,
        owner: &Lease,
    ) -> Result<(), String> {
        let creation = output.creation.clone().ok_or("creation intent missing")?;
        let target = no_symlink_parents(&self.root, &output.path)?;
        if identity(target.parent().ok_or("output parent")?)? != creation.parent_identity {
            return Err("creation parent identity changed; preserve output".into());
        }
        self.stable(gate)?;
        owner.stable()?;
        match fs::symlink_metadata(&target) {
            Ok(_) if output.identity.is_empty() || identity(&target)? != output.identity => {
                return Err("creation destination identity changed; preserve replacement".into());
            }
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Only this registered empty staging slot is recoverable before
                // inode attachment. Content writes cannot precede attachment.
                let staged = self.directory.join(&creation.staged);
                match fs::symlink_metadata(&staged) {
                    Ok(m) => {
                        let empty = if output.tree && m.is_dir() {
                            fs::read_dir(&staged)
                                .map_err(|e| e.to_string())?
                                .next()
                                .is_none()
                        } else {
                            !output.tree && m.is_file() && m.len() == 0
                        };
                        if !empty
                            || (!output.identity.is_empty()
                                && identity(&staged)? != output.identity)
                        {
                            return Err(
                                "creation staging identity/content changed; preserve original"
                                    .into(),
                            );
                        }
                        crate::artifact_disposal::dispose_entry(
                            &self.directory,
                            &creation.staged,
                            &identity(&staged)?,
                            || {
                                self.stable(gate)?;
                                owner.stable()
                            },
                        )?;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                    Err(e) => return Err(e.to_string()),
                }
            }
            Err(e) => return Err(e.to_string()),
        }
        output.outcome = json!({"original":output.outcome,"publication":"interrupted-creation","completion":"not-established"});
        output.phase = "sealing".into();
        output.creation = None;
        Ok(())
    }
    pub(super) fn historical(
        &self,
        key: &str,
    ) -> Result<&super::retention_command::Historical, String> {
        self.policy
            .historical
            .get(key)
            .ok_or("historical owner enrollment is not registered".into())
    }
    pub(super) fn release_historical(&self, key: &str, roles: &[String]) -> Result<(), String> {
        let gate = self.addressed_gate(
            &[],
            &[self.historical(key)?.path.clone()],
            roles,
            None,
            false,
            true,
        )?;
        let mut l = self.load(&gate)?;
        let ids: BTreeSet<_> = l
            .outputs
            .iter()
            .filter(|o| o.outcome["enrollment"] == key)
            .map(|o| o.id.clone())
            .collect();
        if ids.is_empty() {
            return Err("historical enrollment missing".into());
        }
        for role in roles {
            if l.roots
                .get(role)
                .is_some_and(|refs| refs.iter().any(|id| !ids.contains(id)))
            {
                return Err("historical root belongs to another output".into());
            }
            l.roots.remove(role);
        }
        self.save(&l, &gate)
    }
    pub fn acknowledge_delivery(
        &self,
        path: &str,
        digest: &str,
        artifact_id: u64,
        artifact_digest: &str,
    ) -> Result<Value, String> {
        if artifact_id == 0 || !crate::wire::is_digest(artifact_digest) {
            return Err("native artifact delivery identity missing".into());
        }
        let _original = read_guard(&self.root, path)?;
        let raw = fs::read(no_symlink_parents(&self.root, path)?).map_err(|e| e.to_string())?;
        if sha256(&raw) != digest {
            return Err("native delivery report digest differs".into());
        }
        let report: Value = decode(&raw)?;
        let request_id = report["request"]["request_id"]
            .as_str()
            .or(report["request_id"].as_str())
            .ok_or("native delivery request identity")?;
        let role = format!("native-delivery:{request_id}");
        let gate = self.addressed_gate(&[], &[path.into()], &[role.clone()], None, false, true)?;
        let mut l = self.load(&gate)?;
        let o = l
            .outputs
            .iter()
            .find(|o| o.path == path)
            .ok_or("native delivery output not enrolled")?;
        if l.roots.get(&role) != Some(&vec![o.id.clone()]) {
            return Err("native delivery root differs or already released".into());
        }
        let acknowledgement = json!({"schema":"chrono-native-delivery/v1","report":path,"report_sha256":digest,"artifact_id":artifact_id,"artifact_digest":artifact_digest,"original_outcome":o.outcome,"effects":"delivery-root-release-only","completion":"not-established"});
        l.roots.remove(&role);
        l.summaries.push(acknowledgement.clone());
        let excess = l.summaries.len().saturating_sub(self.policy.max_summaries);
        l.summaries.drain(..excess);
        self.save(&l, &gate)?;
        Ok(acknowledgement)
    }
    /// Actual consumers use a shared capability while reading an original.
    pub fn consume(&self, id: &str) -> Result<Lease, String> {
        let gate = self.addressed_gate(&[id.into()], &[], &[], None, false, true)?;
        let l = self.load(&gate)?;
        let o = l
            .outputs
            .iter()
            .find(|o| o.id == id && o.phase != "disposing")
            .ok_or("original evidence unavailable or disposed")?;
        let lease = Lease::acquire(
            &self.directory.join(&o.lease),
            Some(&o.lease_id),
            false,
            false,
            None,
        )?
        .ok_or("original evidence disposal busy")?;
        self.stable(&gate)?;
        self.remember_input(&lease)?;
        Ok(lease)
    }
}
impl Publication {
    /// Attach the empty inode durably before publishing its exact destination.
    pub fn materialize(&self) -> Result<(), String> {
        self.materialize_for_consumer(None)
    }
    fn materialize_for_consumer(&self, consumer: Option<&str>) -> Result<(), String> {
        self.materialize_with_reference(consumer, None)
    }
    /// The owning domain holds its own metadata exclusion and supplies one
    /// durable reference shared by its originals. Attach before exposing bytes;
    /// that owner later narrows the reference from its published domain state.
    pub fn materialize_for_reference(&self, reference: &str) -> Result<(), String> {
        if reference.is_empty() || reference.len() > 256 {
            return Err("invalid original owner reference".into());
        }
        self.materialize_with_reference(None, Some(reference))
    }
    fn materialize_with_reference(
        &self,
        consumer: Option<&str>,
        reference: Option<&str>,
    ) -> Result<(), String> {
        self.lease.stable()?;
        let mut roles = consumer
            .map(|c| self.inventory.consumer_role(c))
            .transpose()?
            .into_iter()
            .collect::<Vec<_>>();
        roles.extend(reference.map(str::to_owned));
        let gate =
            self.inventory
                .addressed_gate(&[self.id.clone()], &[], &roles, None, false, true)?;
        let mut l = self.inventory.load(&gate)?;
        self.materialize_excluded(&mut l, &gate, consumer, reference)
    }
    fn materialize_excluded(
        &self,
        l: &mut Ledger,
        gate: &Gate,
        consumer: Option<&str>,
        reference: Option<&str>,
    ) -> Result<(), String> {
        self.lease.stable()?;
        let o = l
            .outputs
            .iter_mut()
            .find(|o| o.id == self.id)
            .ok_or("creation intent missing")?;
        let creation = o.creation.clone().ok_or("output is already materialized")?;
        let target = no_symlink_parents(&self.inventory.root, &o.path)?;
        if identity(target.parent().ok_or("output parent")?)? != creation.parent_identity {
            return Err("creation parent identity changed; preserve output".into());
        }
        self.inventory.stable(gate)?;
        let staged = self.inventory.directory.join(&creation.staged);
        if o.tree {
            fs::create_dir(&staged).map_err(|e| e.to_string())?;
        } else {
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&staged)
                .map_err(|e| e.to_string())?
                .sync_all()
                .map_err(|e| e.to_string())?;
        }
        o.identity = identity(&staged)?;
        if let Some(reference) = reference {
            let mut ids = l.roots.get(reference).cloned().unwrap_or_default();
            if !ids.contains(&self.id) {
                ids.push(self.id.clone());
            }
            self.inventory.reference_locked(l, reference, &ids)?;
        }
        self.inventory.save(l, gate)?;
        crate::artifact_disposal::publish_entry(&staged, &target)?;
        let o = l
            .outputs
            .iter_mut()
            .find(|o| o.id == self.id)
            .ok_or("creation intent missing")?;
        if identity(&target)? != o.identity {
            return Err("created output identity changed".into());
        }
        o.creation = None;
        o.phase = "publishing".into();
        let attached = o.identity.clone();
        if let Some(consumer) = consumer {
            let role = self.inventory.consumer_role(consumer)?;
            if l.roots.contains_key(&role) {
                return Err("creation consumer role already exists".into());
            }
            self.inventory
                .reference_locked(l, &role, &[self.id.clone()])?;
        }
        self.inventory.save(l, gate)?;
        *self.identity.borrow_mut() = attached;
        Ok(())
    }
    fn write_destination(&self, path: &Path, length: u64) -> Result<PathBuf, String> {
        let total = self
            .written
            .get()
            .checked_add(length)
            .ok_or("output byte overflow")?;
        self.check_length(total)?;
        if self.written_nodes.get() >= self.inventory.policy.max_manifest_nodes {
            return Err("output node capacity exceeded before writing".into());
        }
        // The live publisher capability excludes disposal of its immutable
        // attachment. Other inventory admissions cannot change this output.
        self.inventory.stable(&self.lease)?;
        if self.identity.borrow().is_empty() {
            return Err("output is not materialized".into());
        }
        let physical = fs::canonicalize(path.parent().ok_or("write parent")?)
            .map_err(|e| e.to_string())?
            .join(path.file_name().ok_or("write name")?);
        let relative = physical
            .strip_prefix(&self.inventory.root)
            .map_err(|_| "write outside adopted output")?
            .to_str()
            .ok_or("write path UTF8")?;
        if relative != self.path
            && (!self.tree || !relative.starts_with(&format!("{}/", self.path)))
        {
            return Err("write outside exact output".into());
        }
        if identity(&self.inventory.root.join(&self.path))? != *self.identity.borrow() {
            return Err("publication output identity changed".into());
        }
        self.written.set(total);
        self.written_nodes.set(self.written_nodes.get() + 1);
        no_symlink_parents(&self.inventory.root, relative)
    }
    pub fn create_directory(&self, path: &Path) -> Result<(), String> {
        match fs::symlink_metadata(path) {
            Ok(m) if m.is_dir() => return Ok(()),
            Ok(_) => return Err("fixture directory identity/type changed".into()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.to_string()),
        }
        self.create_directory(path.parent().ok_or("fixture directory parent")?)?;
        let path = self.write_destination(path, 0)?;
        fs::create_dir(path).map_err(|e| e.to_string())
    }
    pub fn write_file(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        let path = self.write_destination(path, bytes.len() as u64)?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())
    }
    pub fn replace_file(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        let path = self.write_destination(path, bytes.len() as u64)?;
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
        }
        let mut file = options.open(path).map_err(|e| e.to_string())?;
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())
    }
    pub fn copy_file(&self, input: &Path, destination: &Path) -> Result<(), String> {
        let mut input = fs::File::open(input).map_err(|e| e.to_string())?;
        let metadata = input.metadata().map_err(|e| e.to_string())?;
        if !metadata.is_file() {
            return Err("fixture input must be a regular file".into());
        }
        let path = self.write_destination(destination, metadata.len())?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        let copied = std::io::copy(&mut (&mut input).take(metadata.len()), &mut file)
            .map_err(|e| e.to_string())?;
        let mut extra = [0];
        if copied != metadata.len() || input.read(&mut extra).map_err(|e| e.to_string())? != 0 {
            return Err("fixture input length changed during bounded copy".into());
        }
        file.set_permissions(metadata.permissions())
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())
    }
    /// An adopted producer checks this reservation before writing new output bytes.
    pub fn byte_limit(&self) -> u64 {
        self.byte_limit
    }
    pub fn check_length(&self, bytes: u64) -> Result<(), String> {
        self.lease.stable()?;
        if bytes > self.byte_limit() {
            return Err("output byte capacity exceeded before writing".into());
        }
        Ok(())
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    /// Transfer an explicitly named consumer before the producer capability ends.
    pub fn protect_consumer(&self, consumer: &str) -> Result<OriginalIdentity, String> {
        self.lease.stable()?;
        let original = OriginalIdentity {
            id: self.id.clone(),
            path: self.path.clone(),
            identity: self.identity.borrow().clone(),
        };
        self.inventory
            .protect_consumer(consumer, &[original.clone()])?;
        Ok(original)
    }
    /// A live producer accepts the caller's exact originals as dependencies.
    /// Its real capability protects the destination without a durable root
    /// that would otherwise survive an interrupted producer indefinitely.
    pub fn accept_consumer(&self, consumer: &Consumer) -> Result<(), String> {
        self.lease.stable()?;
        self.inventory
            .transfer_consumer_to(consumer, &self.path, Some(self))
    }
    /// Keep producer status distinct from storage status; no synthetic success on interruption.
    pub fn complete(self, outcome: Value) -> Result<(), String> {
        self.complete_with_release(outcome, None)
    }
    /// A joined producer can release its own publication consumer after the
    /// actual caller has taken custody. Unsettled or interrupted publication
    /// uses complete and preserves the repair handoff.
    pub fn complete_releasing_consumer(self, outcome: Value, consumer: &str) -> Result<(), String> {
        self.complete_with_release(outcome, Some(consumer))
    }
    /// Deterministic observation at the content-IO boundary for the dedicated
    /// regression consumer; it does not change the publication method.
    #[cfg(feature = "retention-test")]
    #[doc(hidden)]
    pub fn complete_observed(self, outcome: Value, observe: impl FnOnce()) -> Result<(), String> {
        self.complete_with_release_observed(outcome, None, observe)
    }
    fn complete_with_release(self, outcome: Value, consumer: Option<&str>) -> Result<(), String> {
        self.complete_with_release_observed(outcome, consumer, || {})
    }
    fn complete_with_release_observed(
        self,
        outcome: Value,
        consumer: Option<&str>,
        observe: impl FnOnce(),
    ) -> Result<(), String> {
        // Returning a completed original hands its capability to any enclosing
        // caller before the producer closes. A nested caller may have no durable
        // root left after success, while its parent still needs its exact result.
        self.inventory.remember_input(&self.lease)?;
        if serde_json::to_vec(&outcome)
            .map_err(|e| e.to_string())?
            .len()
            > 16384
        {
            return Err("output outcome metadata bound".into());
        }
        self.lease.stable()?;
        // Reservation already returned this producer's exact declaration. Its
        // live capability excludes recovery/accounting of these owned fields;
        // only graph owners may change references before the final merge.
        let mut owned = self.initial.as_ref().clone();
        owned.identity = self.identity.borrow().clone();
        // The producer's exact capability excludes accounting/disposal of this
        // output. Hashing and tree sealing need no enrollment-wide exclusion.
        // Reference owners may update their own edges while these IOs proceed.
        observe();
        let o = &mut owned;
        let path = no_symlink_parents(&self.inventory.root, &o.path)?;
        if identity(&path)? != o.identity {
            return Err("publication output identity changed".into());
        }
        if !o.tree {
            let (digest, length) = crate::file_identity(&path)?;
            o.sha256 = Some(digest);
            o.length = Some(length);
        }
        o.creation = None;
        o.outcome = outcome;
        o.phase = "sealing".into();
        let began = Instant::now();
        let mut steps = 0;
        let mut accounting_error = None;
        while !o.manifest.sealed
            && steps < self.inventory.policy.max_manifest_nodes.saturating_mul(4)
            && began.elapsed().as_millis() < self.inventory.policy.max_millis_per_round as u128
        {
            steps += 1;
            if let Err(error) = o.manifest.step(
                &self.inventory.root,
                &o.path,
                self.inventory.policy.max_manifest_nodes,
                self.byte_limit,
            ) {
                accounting_error = Some(error);
                break;
            }
        }
        if o.manifest.sealed {
            o.reserved_bytes = o.manifest.bytes;
            o.phase = "released".into();
        }
        o.expires = now()?.saturating_add(self.inventory.policy.keep_seconds);
        self.lease.stable()?;
        let roles = consumer
            .map(|c| self.inventory.consumer_role(c))
            .transpose()?
            .into_iter()
            .collect::<Vec<_>>();
        let gate =
            self.inventory
                .addressed_gate(&[self.id.clone()], &[], &roles, None, false, true)?;
        let mut l = self.inventory.load(&gate)?;
        let current = l
            .outputs
            .iter_mut()
            .find(|o| o.id == self.id)
            .ok_or("publication disappeared while sealing")?;
        if current.path != owned.path
            || current.identity != owned.identity
            || current.lease != owned.lease
            || current.lease_id != owned.lease_id
            || current.phase == "disposing"
        {
            return Err("publication binding changed while sealing; preserve originals".into());
        }
        // Keep graph changes made since the original observation. Only this
        // producer owns the outcome, content accounting and completion fields.
        owned.references = current.references.clone();
        *current = owned.into();
        let o = l.outputs.iter().find(|o| o.id == self.id).unwrap();
        if let Some(consumer) = consumer.filter(|_| accounting_error.is_none()) {
            // Release only after all original accounting succeeded; any earlier error
            // preserves the root and original partial producer outcome.
            if o.manifest.bytes > self.byte_limit() {
                self.inventory.save(&l, &gate)?;
                return Err("output byte capacity exceeded; consumer original preserved".into());
            }
            let role = self.inventory.consumer_role(consumer)?;
            if l.roots.get(&role) != Some(&vec![self.id.clone()]) {
                return Err("producer consumer root differs".into());
            }
            self.inventory.reference_locked(&mut l, &role, &[])?;
        }
        self.inventory.save(&l, &gate)?;
        if let Some(error) = accounting_error {
            return Err(error);
        }
        if l.outputs
            .iter()
            .find(|o| o.id == self.id)
            .is_some_and(|o| o.manifest.bytes > self.byte_limit())
        {
            return Err(
                "output byte capacity exceeded; original accounted and preserved until released"
                    .into(),
            );
        }
        Ok(())
    }
}
/// Ordinary adopted owners share this entry. Unknown legacy outputs remain protected.
pub fn maintain_adopted(root: &Path) -> Result<Option<Value>, String> {
    Inventory::adopted(root)?
        .map(|i| i.maintain_available())
        .transpose()
}

/// Enroll a real producer output in the adopted owner, recovering prior released work first.
pub fn begin_adopted(
    root: &Path,
    producer: &str,
    path: &Path,
    outcome: Value,
) -> Result<Option<Publication>, String> {
    let Some(inventory) = Inventory::adopted(root)? else {
        return Ok(None);
    };
    let physical_path = fs::canonicalize(path).map_err(|e| e.to_string())?;
    let relative = physical_path
        .strip_prefix(&inventory.root)
        .map_err(|_| "output is outside adopted host")?
        .to_str()
        .ok_or("output UTF8")?;
    if !inventory.accepts(producer, relative) {
        return Ok(None);
    }
    let roots = if producer == "lifecycle-original" {
        vec![format!("recovery:{relative}")]
    } else {
        Vec::new()
    };
    inventory
        .begin_rooted(producer, relative, outcome, &roots)
        .map(Some)
}
/// Create only after admission and a recoverable owner intent. Unadopted hosts
/// keep their existing output semantics, without enrolling unknown paths.
pub fn create_adopted(
    root: &Path,
    producer: &str,
    path: &Path,
    tree: bool,
    outcome: Value,
) -> Result<Option<Publication>, String> {
    create_adopted_with_limit(root, producer, path, tree, outcome, None)
}
/// The producer declares a finite reservation within the host ceiling. Unknown
/// external writers keep the default ceiling; this is not a physical quota.
pub fn create_adopted_with_limit(
    root: &Path,
    producer: &str,
    path: &Path,
    tree: bool,
    outcome: Value,
    byte_limit: Option<u64>,
) -> Result<Option<Publication>, String> {
    create_with_consumer(root, producer, path, tree, outcome, byte_limit, None)
}
/// Bind an actual consumer in the existing durable publication, before any
/// content write. The producer supplies the role prefix; the owner mints its
/// independent output identity. No extra inventory transaction is required.
pub fn create_adopted_for_consumer(
    root: &Path,
    producer: &str,
    path: &Path,
    tree: bool,
    outcome: Value,
    byte_limit: Option<u64>,
    consumer_prefix: &str,
) -> Result<Option<Publication>, String> {
    if consumer_prefix.trim().is_empty() || consumer_prefix.len() > 210 {
        return Err("invalid bounded creation consumer prefix".into());
    }
    create_with_consumer(
        root,
        producer,
        path,
        tree,
        outcome,
        byte_limit,
        Some(consumer_prefix),
    )
}
fn create_with_consumer(
    root: &Path,
    producer: &str,
    path: &Path,
    tree: bool,
    outcome: Value,
    byte_limit: Option<u64>,
    consumer_prefix: Option<&str>,
) -> Result<Option<Publication>, String> {
    let canonical_root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let physical_path = fs::canonicalize(path.parent().ok_or("output parent")?)
        .map_err(|e| e.to_string())?
        .join(path.file_name().ok_or("output name")?);
    let relative = physical_path
        .strip_prefix(&canonical_root)
        .map_err(|_| "output outside adopted host")?
        .to_str()
        .ok_or("output path UTF8")?;
    crate::relative_path(relative)?;
    if let Some(inventory) = Inventory::adopted(root)? {
        if inventory.accepts(producer, relative) {
            let roots = if producer == "lifecycle-original" {
                vec![format!("recovery:{relative}")]
            } else {
                vec![]
            };
            let (publication, gate, ledger) = inventory.reserve_excluded(
                producer,
                relative,
                outcome,
                &roots,
                Some(tree),
                byte_limit,
            )?;
            let consumer = consumer_prefix.map(|prefix| format!("{prefix}:{}", publication.id()));
            drop((gate, ledger));
            publication.materialize_for_consumer(consumer.as_deref())?;
            return Ok(Some(publication));
        }
    }
    if tree {
        fs::create_dir(path).map_err(|e| e.to_string())?;
    } else {
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| e.to_string())?;
    }
    Ok(None)
}
/// Choose an address without creating it; creation belongs to the same publisher.
pub fn unique_path(directory: &Path, prefix: &str, suffix: &str) -> Result<PathBuf, String> {
    static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    if prefix.contains('/') || suffix.contains('/') {
        return Err("output address prefix/suffix".into());
    }
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let directory = fs::canonicalize(directory).map_err(|e| e.to_string())?;
    Ok(directory.join(format!(
        "{prefix}{}-{time}-{}{suffix}",
        std::process::id(),
        SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )))
}
pub fn copy_adopted(
    publication: Option<&Publication>,
    input: &Path,
    output: &Path,
) -> Result<(), String> {
    if let Some(publication) = publication {
        publication.copy_file(input, output)
    } else {
        fs::copy(input, output)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}
pub fn write_adopted(
    publication: Option<&Publication>,
    output: &Path,
    bytes: &[u8],
) -> Result<(), String> {
    if let Some(publication) = publication {
        publication.write_file(output, bytes)
    } else {
        fs::write(output, bytes).map_err(|e| e.to_string())
    }
}
pub fn directory_adopted(publication: Option<&Publication>, output: &Path) -> Result<(), String> {
    if let Some(publication) = publication {
        publication.create_directory(output)
    } else {
        fs::create_dir_all(output).map_err(|e| e.to_string())
    }
}
/// A registered reader protects an enrolled original. Unknown legacy evidence retains old semantics.
pub fn read_guard(root: &Path, path: &str) -> Result<Option<Lease>, String> {
    let Some(inventory) = Inventory::adopted(root)? else {
        return Ok(None);
    };
    let gate = inventory.addressed_gate(&[], &[path.into()], &[], None, false, true)?;
    let l = inventory.load(&gate)?;
    let found = l
        .outputs
        .iter()
        .find(|o| o.path == path || (o.tree && path.starts_with(&format!("{}/", o.path))));
    let Some(o) = found else { return Ok(None) };
    if o.phase == "disposing" {
        return Err("original evidence unavailable: disposal has started".into());
    }
    let lease = Lease::acquire(
        &inventory.directory.join(&o.lease),
        Some(&o.lease_id),
        false,
        false,
        None,
    )?
    .ok_or("original evidence disposal busy")?;
    inventory.stable(&gate)?;
    inventory.remember_input(&lease)?;
    Ok(Some(lease))
}

/// Publish a newly addressed original under its opted-in natural producer.
/// Existing unenrolled originals are verified and reused but require explicit legacy migration.
pub fn publish_original(
    root: &Path,
    path: &str,
    bytes: &[u8],
    outcome: Value,
) -> Result<bool, String> {
    publish_owned_original(root, "check-original", path, bytes, outcome)
}
pub fn publish_owned_original(
    root: &Path,
    producer: &str,
    path: &str,
    bytes: &[u8],
    outcome: Value,
) -> Result<bool, String> {
    publish_owned_with(
        root,
        producer,
        path,
        bytes.len() as u64,
        &sha256(bytes),
        outcome,
        |file| file.write_all(bytes).map_err(|e| e.to_string()),
        None,
    )
}

#[cfg(feature = "retention-test")]
#[doc(hidden)]
pub fn publish_owned_observed(
    root: &Path,
    producer: &str,
    path: &str,
    bytes: &[u8],
    outcome: Value,
    observe: impl FnOnce(),
) -> Result<bool, String> {
    publish_owned_with(
        root,
        producer,
        path,
        bytes.len() as u64,
        &sha256(bytes),
        outcome,
        |file| {
            observe();
            file.write_all(bytes).map_err(|e| e.to_string())
        },
        None,
    )
}

/// Stream a known original only after its output byte reservation is admitted.
/// The source's declared length and digest remain authoritative if it changes while copying.
pub fn publish_owned_file(
    root: &Path,
    producer: &str,
    path: &str,
    input: &Path,
    digest: &str,
    length: u64,
    outcome: Value,
) -> Result<bool, String> {
    publish_owned_with(
        root,
        producer,
        path,
        length,
        digest,
        outcome,
        |file| {
            let mut input = fs::File::open(input).map_err(|e| e.to_string())?;
            if !input.metadata().map_err(|e| e.to_string())?.is_file() {
                return Err("original input must be a regular file".into());
            }
            let observed = crate::copy_hashed((&mut input).take(length), file)?;
            let mut extra = [0];
            if observed != (digest.to_owned(), length)
                || input.read(&mut extra).map_err(|e| e.to_string())? != 0
            {
                return Err("original input changed during bounded publication".into());
            }
            Ok(())
        },
        None,
    )
}

fn publish_owned_with(
    root: &Path,
    producer: &str,
    path: &str,
    length: u64,
    digest: &str,
    outcome: Value,
    write_content: impl FnOnce(&mut fs::File) -> Result<(), String>,
    consumer: Option<(&str, bool)>,
) -> Result<bool, String> {
    let Some(inventory) = Inventory::adopted(root)? else {
        return Ok(false);
    };
    if !inventory.accepts(producer, path) {
        return Ok(false);
    }
    if length > inventory.policy.max_output_bytes {
        return Err("output byte capacity exceeded before writing".into());
    }
    let _writer = inventory.address_gate("writer", path)?;
    let target = no_symlink_parents(root, path)?;
    fs::create_dir_all(target.parent().ok_or("original parent")?).map_err(|e| e.to_string())?;
    match fs::symlink_metadata(&target) {
        Ok(_) => {
            let _input = read_guard(root, path)?;
            if crate::file_identity(&target)? != (digest.to_owned(), length) {
                return Err("addressed original bytes mismatch".into());
            }
            if let Some(id) = inventory.validate_original_reuse(path)? {
                if let Some((prefix, _)) = consumer {
                    inventory.protect_path_consumer(&format!("{prefix}:{id}"), path)?;
                }
            }
            // Reuse leaves the first immutable operation outcome intact.
            return Ok(true);
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
        Err(e) => return Err(e.to_string()),
    }
    let publication = inventory.plan(
        producer,
        path,
        false,
        json!({"publication":"started","outcome":"not-established"}),
    )?;
    let role =
        consumer.map(|(prefix, release)| (format!("{prefix}:{}", publication.id()), release));
    publication.materialize_for_consumer(role.as_ref().map(|(role, _)| role.as_str()))?;
    publication.check_length(length)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .open(&target)
        .map_err(|e| e.to_string())?;
    write_content(&mut file)?;
    file.sync_all().map_err(|e| e.to_string())?;
    match role {
        Some((role, true)) => publication.complete_releasing_consumer(outcome, &role)?,
        _ => publication.complete(outcome)?,
    }
    Ok(true)
}

/// The final report keeps its publication role until the actual fixed-slot
/// settlement transfers its closure to the current report. A settled failure
/// then has a finite diagnostic window rather than an invented permanent reader.
pub fn publish_final_original(
    root: &Path,
    producer: &str,
    path: &str,
    bytes: &[u8],
    outcome: Value,
    succeeded: bool,
) -> Result<bool, String> {
    publish_owned_with(
        root,
        producer,
        path,
        bytes.len() as u64,
        &sha256(bytes),
        outcome,
        |file| file.write_all(bytes).map_err(|e| e.to_string()),
        Some((
            if succeeded {
                "publication-recheck"
            } else {
                "report-recheck"
            },
            false,
        )),
    )
}
/// Inputs of an unfinished report stay rooted until the actual fixed-slot
/// publisher hands their closure to a final original. Interruption is not release.
pub fn publish_pending_original(
    root: &Path,
    producer: &str,
    path: &str,
    bytes: &[u8],
    outcome: Value,
) -> Result<bool, String> {
    publish_owned_with(
        root,
        producer,
        path,
        bytes.len() as u64,
        &sha256(bytes),
        outcome,
        |file| file.write_all(bytes).map_err(|e| e.to_string()),
        Some(("publication-recheck", false)),
    )
}

/// Streaming equivalent for the actual retained-blob publisher.
pub fn publish_pending_file(
    root: &Path,
    producer: &str,
    path: &str,
    input: &Path,
    digest: &str,
    length: u64,
    outcome: Value,
) -> Result<bool, String> {
    publish_owned_with(
        root,
        producer,
        path,
        length,
        digest,
        outcome,
        |file| {
            let mut source = fs::File::open(input).map_err(|e| e.to_string())?;
            if !source.metadata().map_err(|e| e.to_string())?.is_file() {
                return Err("original input must be regular".into());
            }
            let observed = crate::copy_hashed((&mut source).take(length), file)?;
            let mut extra = [0];
            if observed != (digest.to_owned(), length)
                || source.read(&mut extra).map_err(|e| e.to_string())? != 0
            {
                return Err("original input changed during bounded publication".into());
            }
            Ok(())
        },
        Some(("publication-recheck", false)),
    )
}

/// Commit exact report edges and the current fixed report slot before publishing its pointer.
/// Pending native delivery is a distinct explicit root; only its owner may acknowledge/release it.
pub fn reference_report(
    root: &Path,
    slot: &str,
    original: &str,
    originals: &[String],
    pending_native: Option<&str>,
) -> Result<Option<Lease>, String> {
    let Some(inventory) = Inventory::adopted(root)? else {
        return Ok(None);
    };
    let publisher = inventory.address_gate("slot", slot)?;
    inventory.dependencies_paths(original, originals)?;
    // Both old and new originals stay rooted until the fixed pointer was written.
    inventory.extend_reference_path(
        &format!("current-report:{}", sha256(slot.as_bytes())),
        original,
    )?;
    if let Some(identity) = pending_native {
        inventory.reference_paths(&format!("native-delivery:{identity}"), &[original.into()])?;
    }
    Ok(Some(publisher))
}
/// Release the previous fixed-slot reference only after its replacement is durably published.
pub fn settle_report(root: &Path, slot: &str, original: &str) -> Result<(), String> {
    if let Some(inventory) = Inventory::adopted(root)? {
        let role = format!("current-report:{}", sha256(slot.as_bytes()));
        inventory.reference_paths(&role, &[original.into()])?;
        let gate = inventory.addressed_gate(&[], &[original.into()], &[], None, false, true)?;
        let ledger = inventory.load(&gate)?;
        let outputs = ledger.outputs;
        drop(gate);
        for output in outputs {
            let id = &output.id;
            let diagnostic = format!("consumer:report-recheck:{id}");
            if output.path == original
                && matches!(output.phase.as_str(), "released" | "sealing")
                && ledger.roots.contains_key(&diagnostic)
            {
                inventory.retain_diagnostic_path(&output.path)?;
                inventory.reference(&diagnostic, &[])?;
            }
            inventory.reference(&format!("consumer:publication-recheck:{id}"), &[])?;
        }
    }
    Ok(())
}

/// A second domain may remove a copied body/cache only when the final-original
/// owner has retired every overlapping publication. Hold only an addressed
/// removal capability across the effect; independent publication remains free.
pub struct RemovalGuard {
    inventory: Inventory,
    exclusion: Gate,
}
impl RemovalGuard {
    /// Recheck the final-original policy immediately before the other owner's effect.
    /// The declared paths exclude overlapping publications through this capability.
    pub fn stable(&self) -> Result<(), String> {
        self.inventory.stable(&self.exclusion)
    }
}
pub fn external_removal_guard(root: &Path, path: &str) -> Result<Option<RemovalGuard>, String> {
    external_removal_guards(root, &[path.to_owned()])
}
pub fn external_removal_guards(
    root: &Path,
    paths: &[String],
) -> Result<Option<RemovalGuard>, String> {
    for path in paths {
        crate::relative_path(path)?;
    }
    let Some(inventory) = Inventory::adopted(root)? else {
        return Ok(None);
    };
    let paths: Vec<_> = paths
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if paths.is_empty() || paths.len() > inventory.policy.max_related_outputs {
        return Err("external removal scope is empty or exceeds capacity".into());
    }
    let exclusion = inventory.removal_gate(&paths)?;
    inventory.stable(&exclusion)?;
    Ok(Some(RemovalGuard {
        inventory,
        exclusion,
    }))
}
