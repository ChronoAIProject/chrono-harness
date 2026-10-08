//! Explicit producer output lifetimes. No directory discovery or inferred consumers.
//! Uses the lifecycle kernel capability owner for publication, use and disposal exclusion.
use crate::{decode, no_symlink_parents, ownership::Lease, sha256};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[path = "manifest.rs"]
mod manifest;
use manifest::Manifest;

pub const POLICY_PATH: &str = ".chrono-harness/retention.json";
const STATE: &str = ".chrono-harness/state/output-retention-v1";
const MAX_STATE: u64 = 8 * 1024 * 1024;
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub schema: String,
    pub producers: BTreeMap<String, Vec<String>>,
    pub keep_seconds: u64,
    pub max_entries: usize,
    pub max_summaries: usize,
    pub max_nodes_per_round: usize,
    pub max_bytes_per_round: u64,
    pub max_millis_per_round: u64,
    #[serde(default = "default_output_bytes")]
    pub max_output_bytes: u64,
    #[serde(default = "default_total_bytes")]
    pub max_total_bytes: u64,
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
fn default_total_bytes() -> u64 {
    8 * 1024 * 1024 * 1024
}
fn default_manifest_nodes() -> usize {
    16384
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Node {
    path: String,
    identity: String,
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
}
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Ledger {
    policy_sha256: String,
    outputs: Vec<Output>,
    roots: BTreeMap<String, Vec<String>>,
    summaries: Vec<Value>,
    cursor: usize,
    #[serde(default)]
    next_id: u64,
    #[serde(default)]
    retired_leases: Vec<Node>,
}
pub struct Inventory {
    root: PathBuf,
    policy: Policy,
    policy_bytes: Vec<u8>,
    directory: PathBuf,
    round_metadata: Cell<Option<u64>>,
}
/// Holding this capability protects producer descendants too. Closing it never fabricates completion.
pub struct Publication {
    inventory: Inventory,
    id: String,
    lease: Lease,
}
fn now() -> Result<u64, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs())
}
use crate::artifact_disposal::identity;

fn write(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let raw = serde_json::to_vec(value).map_err(|e| e.to_string())?;
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
    f.write_all(&raw).map_err(|e| e.to_string())?;
    f.sync_all().map_err(|e| e.to_string())?;
    fs::rename(temporary, path).map_err(|e| e.to_string())?;
    Ok(())
}
impl Inventory {
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
        if policy.schema != "chrono-output-retention/v1"
            || policy.max_entries == 0
            || policy.max_entries > 4096
            || policy.max_summaries == 0
            || policy.max_summaries > 4096
            || policy.max_nodes_per_round == 0
            || policy.max_nodes_per_round > 4096
            || policy.max_bytes_per_round == 0
            || policy.max_millis_per_round == 0
            || policy.max_millis_per_round > 60000
            || policy.max_output_bytes == 0
            || policy.max_total_bytes < policy.max_output_bytes
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
                if !scope.starts_with(".chrono-harness/state/")
                    || !scope.ends_with('/')
                    || scope.starts_with(STATE)
                {
                    return Err(
                        "retention scope must be explicit state output, outside retention metadata"
                            .into(),
                    );
                }
            }
        }
        let directory = no_symlink_parents(root, STATE)?;
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        Ok(Some(Self {
            root: root.into(),
            policy,
            policy_bytes: bytes,
            directory,
            round_metadata: Cell::new(None),
        }))
    }
    pub(super) fn gate(&self) -> Result<Lease, String> {
        Lease::acquire(
            &self.directory.join("admission.lease"),
            None,
            true,
            true,
            Some(Duration::from_secs(5)),
        )?
        .ok_or("retention admission busy".into())
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
    pub(super) fn stable(&self, gate: &Lease) -> Result<(), String> {
        self.charge_metadata(self.policy_bytes.len() as u64)?;
        gate.stable()?;
        if fs::read(no_symlink_parents(&self.root, POLICY_PATH)?).map_err(|e| e.to_string())?
            != self.policy_bytes
        {
            return Err("output retention policy changed; preserve outputs".into());
        }
        Ok(())
    }
    fn load(&self) -> Result<Ledger, String> {
        self.load_bound(&sha256(&self.policy_bytes))
    }
    fn load_bound(&self, policy_digest: &str) -> Result<Ledger, String> {
        let path = self.directory.join("inventory.json");
        let mut raw = Vec::new();
        if let Ok(metadata) = fs::symlink_metadata(&path) {
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err("retention inventory must be a physical regular file".into());
            }
        }
        match fs::File::open(path) {
            Ok(f) => {
                f.take(MAX_STATE + 1)
                    .read_to_end(&mut raw)
                    .map_err(|e| e.to_string())?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Ledger {
                    policy_sha256: sha256(&self.policy_bytes),
                    ..Ledger::default()
                });
            }
            Err(e) => return Err(e.to_string()),
        };
        if raw.len() as u64 > MAX_STATE {
            return Err("retention metadata bound".into());
        }
        self.charge_metadata(raw.len() as u64)?;
        let l: Ledger = decode(&raw)?;
        if l.policy_sha256 != policy_digest
            || l.outputs.len() > self.policy.max_entries
            || l.summaries.len() > self.policy.max_summaries
            || l.retired_leases.len() > self.policy.max_entries
            || l.roots.len() > self.policy.max_entries
        {
            return Err("retention policy migration required; original inventory preserved".into());
        }
        let mut ids = BTreeSet::new();
        for o in &l.outputs {
            crate::relative_path(&o.path)?;
            crate::relative_path(&o.lease)?;
            if !ids.insert(o.id.clone())
                || o.path.len() > 4096
                || o.stack.len() > 129
                || o.references.len() > self.policy.max_entries
                || o.manifest.entries.len() > self.policy.max_manifest_nodes
                || !self
                    .policy
                    .producers
                    .get(&o.producer)
                    .is_some_and(|scopes| scopes.iter().any(|p| o.path.starts_with(p)))
                || !matches!(
                    o.phase.as_str(),
                    "reserving" | "publishing" | "sealing" | "released" | "disposing"
                )
            {
                return Err("invalid retained output inventory; preserve originals".into());
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
            }
        }
        for o in &l.outputs {
            if o.references.iter().any(|id| !ids.contains(id)) {
                return Err("missing retained output dependency".into());
            }
        }
        for refs in l.roots.values() {
            if refs.len() > self.policy.max_entries || refs.iter().any(|id| !ids.contains(id)) {
                return Err("missing current consumer output reference".into());
            }
        }
        Ok(l)
    }
    fn save(&self, l: &Ledger, gate: &Lease) -> Result<(), String> {
        self.stable(gate)?;
        self.charge_metadata(serde_json::to_vec(l).map_err(|e| e.to_string())?.len() as u64)?;
        write(&self.directory.join("inventory.json"), l)
    }
    /// Explicit owner migration. The caller supplies the exact prior policy bytes;
    /// every existing output must still be declared, and all old producers/readers must be excluded.
    /// This changes no output identity, original outcome, expiry, root, or completion state.
    pub fn migrate_policy(&self, previous_policy: &[u8]) -> Result<Value, String> {
        let previous: Policy = decode(previous_policy)?;
        if previous.schema != self.policy.schema {
            return Err("unsupported retention policy migration".into());
        }
        let gate = self.gate()?;
        let mut l = self.load_bound(&sha256(previous_policy))?;
        let _reports = Lease::acquire(
            &self.directory.join("reports.lease"),
            None,
            true,
            true,
            None,
        )?
        .ok_or("live report producer/consumer prevents retention migration")?;
        let mut owners = Vec::new();
        for output in &l.outputs {
            if output.phase == "reserving" {
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
        let accounted = l
            .outputs
            .iter()
            .try_fold(0u64, |n, o| {
                let bytes = if o.manifest.sealed {
                    o.manifest.bytes.saturating_sub(o.removed_logical_bytes)
                } else {
                    self.policy.max_output_bytes
                };
                if bytes > self.policy.max_output_bytes {
                    return None;
                }
                n.checked_add(bytes)
            })
            .ok_or("candidate retention capacity does not cover original outputs")?;
        if accounted > self.policy.max_total_bytes {
            return Err("candidate total byte capacity does not cover originals; account/release through owner first".into());
        }
        let before = l.policy_sha256.clone();
        l.policy_sha256 = sha256(&self.policy_bytes);
        self.save(&l, &gate)?;
        Ok(
            json!({"schema":"chrono-output-policy-migration/v1","previous_policy_sha256":before,"policy_sha256":l.policy_sha256,"outputs_preserved":l.outputs.len(),"roots_preserved":l.roots.len(),"effects":"metadata-binding-only","completion":"not-established"}),
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
        let gate = self.gate()?;
        let mut l = self.load()?;
        if l.outputs.len() + l.retired_leases.len() >= self.policy.max_entries {
            return Err("protected output inventory capacity; run bounded maintenance or release references".into());
        }
        if l.outputs.iter().any(|o| {
            o.path == path
                || path.starts_with(&format!("{}/", o.path))
                || o.path.starts_with(&format!("{path}/"))
        }) {
            return Err("overlapping/already enrolled output identity".into());
        }
        crate::artifact_disposal::source_safe(&self.root, path)?;
        let charged = l
            .outputs
            .iter()
            .try_fold(0u64, |n, o| {
                n.checked_add(o.reserved_bytes.max(if o.manifest.sealed {
                    o.manifest.bytes.saturating_sub(o.removed_logical_bytes)
                } else {
                    self.policy.max_output_bytes
                }))
            })
            .ok_or("retained byte accounting overflow")?;
        if charged.saturating_add(self.policy.max_output_bytes) > self.policy.max_total_bytes {
            return Err(
                "protected total retained byte capacity; release references or maintain".into(),
            );
        }
        let physical = no_symlink_parents(&self.root, path)?;
        let m = fs::symlink_metadata(&physical).map_err(|e| e.to_string())?;
        if !m.is_file() && !m.is_dir() {
            return Err("output must be a physical file/directory".into());
        }
        let id = format!("output-{}", l.next_id);
        l.next_id = l.next_id.checked_add(1).ok_or("output id capacity")?;
        let lease_path = self.directory.join(&id);
        if lease_path.exists() {
            return Err("reserved output capability already exists; preserve it".into());
        }
        let object = Output {
            id: id.clone(),
            producer: producer.into(),
            path: path.into(),
            identity: identity(&physical)?,
            tree: m.is_dir(),
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
            reserved_bytes: self.policy.max_output_bytes,
        };
        for role in roots {
            if role.is_empty()
                || role.len() > 256
                || l.roots.contains_key(role)
                || l.roots.len() >= self.policy.max_entries
            {
                return Err("historical/current role identity collision or capacity".into());
            }
            l.roots.insert(role.clone(), vec![id.clone()]);
        }
        l.outputs.push(object);
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
        last.phase = "publishing".into();
        self.save(&l, &gate)?;
        drop(gate);
        Ok(Publication {
            inventory: self,
            id,
            lease,
        })
    }
    /// A complete producer/collector operation protects its explicitly adopted report family.
    /// The inherited kernel capability closes on interruption even without a finish callback.
    pub fn report_activity(&self) -> Result<Lease, String> {
        let gate = self.gate()?;
        let lease = Lease::acquire(
            &self.directory.join("reports.lease"),
            None,
            true,
            false,
            None,
        )?
        .ok_or("report retention exclusion busy")?;
        self.stable(&gate)?;
        Ok(lease)
    }
    fn validate_original_reuse(&self, path: &str) -> Result<(), String> {
        let gate = self.gate()?;
        let l = self.load()?;
        if let Some(o) = l.outputs.iter().find(|o| o.path == path) {
            if o.phase == "disposing"
                || identity(&no_symlink_parents(&self.root, path)?)? != o.identity
            {
                return Err("retained original unavailable, disposing or replaced".into());
            }
        }
        self.stable(&gate)
    }
    pub fn enrolled(&self, path: &str) -> Result<bool, String> {
        let gate = self.gate()?;
        let l = self.load()?;
        self.stable(&gate)?;
        Ok(l.outputs.iter().any(|o| o.path == path))
    }
    pub fn accepts(&self, producer: &str, path: &str) -> bool {
        self.policy
            .producers
            .get(producer)
            .is_some_and(|scopes| scopes.iter().any(|scope| path.starts_with(scope)))
    }
    /// Explicit original addresses supplied by the actual consumer, never inferred from JSON or names.
    pub fn reference_paths(&self, consumer: &str, paths: &[String]) -> Result<(), String> {
        let gate = self.gate()?;
        let mut l = self.load()?;
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
            || ids.len() > self.policy.max_entries
            || (!l.roots.contains_key(consumer) && l.roots.len() >= self.policy.max_entries)
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
        let gate = self.gate()?;
        let mut l = self.load()?;
        let Some(o) = l
            .outputs
            .iter()
            .find(|o| o.path == path && o.phase != "disposing")
        else {
            return Ok(());
        };
        let id = o.id.clone();
        if !l.roots.contains_key(consumer) && l.roots.len() >= self.policy.max_entries {
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
        let gate = self.gate()?;
        let mut l = self.load()?;
        let ids: BTreeSet<_> = l
            .outputs
            .iter()
            .filter(|o| paths.iter().any(|p| *p == o.path))
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
        let gate = self.gate()?;
        let mut l = self.load()?;
        if ids.len() > self.policy.max_entries
            || (ids.is_empty() == false
                && !l.roots.contains_key(consumer)
                && l.roots.len() >= self.policy.max_entries)
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
        self.save(&l, &gate)
    }
    /// Edges are not roots. Released historical cycles can be reclaimed.
    pub fn dependencies(&self, id: &str, references: &[String]) -> Result<(), String> {
        let gate = self.gate()?;
        let mut l = self.load()?;
        if references.len() > self.policy.max_entries
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
        let result = self.maintain_round(observed);
        self.round_metadata.set(None);
        result
    }
    fn maintain_round(&self, observed: u64) -> Result<Value, String> {
        let began = Instant::now();
        let gate = self.gate()?;
        let mut l = self.load()?;
        let mut metadata_steps = 0usize;
        let mut accounting_errors = Vec::new();
        // Recover only explicit reservations; never discover unknown files by scanning.
        for index in 0..l.outputs.len() {
            if metadata_steps >= self.policy.max_nodes_per_round || !self.metadata_room() {
                break;
            }
            if l.outputs[index].phase == "reserving" {
                metadata_steps += 1;
                let path = self.directory.join(&l.outputs[index].lease);
                if let Some(capability) = Lease::acquire(&path, None, true, true, None)? {
                    l.outputs[index].lease_id = capability.id().into();
                    l.outputs[index].phase = "publishing".into();
                    self.save(&l, &gate)?;
                }
            }
        }
        while metadata_steps < self.policy.max_nodes_per_round
            && !l.retired_leases.is_empty()
            && self.metadata_room()
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
                    || self.stable(&gate),
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
                || metadata_steps >= self.policy.max_nodes_per_round
                || began.elapsed().as_millis() >= self.policy.max_millis_per_round as u128
            {
                break;
            }
            if l.outputs[index].manifest.sealed || l.outputs[index].phase == "reserving" {
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
                && metadata_steps < self.policy.max_nodes_per_round
                && began.elapsed().as_millis() < self.policy.max_millis_per_round as u128
            {
                metadata_steps += 1;
                if let Err(error) = l.outputs[index].manifest.step(
                    &self.root,
                    &output.path,
                    self.policy.max_manifest_nodes,
                    self.policy.max_output_bytes,
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
        let report_exclusion = Lease::acquire(
            &self.directory.join("reports.lease"),
            None,
            true,
            true,
            None,
        )?;
        let mut rooted = BTreeSet::new();
        let mut pending: Vec<_> = l.roots.values().flatten().cloned().collect();
        for o in &l.outputs {
            if matches!(
                o.producer.as_str(),
                "check-original" | "full-original" | "lifecycle-original"
            ) && report_exclusion.is_none()
            {
                pending.push(o.id.clone());
            }
            // A finite window retains bytes that still exist; it cannot turn an
            // already removed successful fixture into permanent metadata.
            let absent = matches!(fs::symlink_metadata(self.root.join(&o.path)),Err(ref e) if e.kind()==std::io::ErrorKind::NotFound);
            if o.expires > observed && !absent {
                pending.push(o.id.clone());
            }
            match Lease::acquire(
                &self.directory.join(&o.lease),
                Some(&o.lease_id),
                false,
                true,
                None,
            ) {
                Ok(Some(_)) => (),
                _ => pending.push(o.id.clone()),
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
        let mut order: Vec<_> = (0..count)
            .map(|n| l.outputs[(start + n) % count].id.clone())
            .collect();
        // Retire an unrooted report before the originals it references. This
        // spends a bounded round on retiring the consumer first and prevents
        // an obsolete report from outliving reclaimed inputs. Stable sorting
        // preserves the durable cursor among peers; cycles remain reclaimable.
        let referenced: BTreeSet<_> = l.outputs.iter().flat_map(|o| o.references.iter()).collect();
        order.sort_by_key(|id| referenced.contains(id));
        for id in order {
            if !self.metadata_room()
                || nodes >= self.policy.max_nodes_per_round
                || began.elapsed().as_millis() >= self.policy.max_millis_per_round as u128
            {
                break;
            }
            let i = l
                .outputs
                .iter()
                .position(|o| o.id == id)
                .ok_or("output disappeared")?;
            nodes += 1;
            l.cursor = (i + 1) % count.max(1);
            if rooted.contains(&id) {
                protected.push(
                    json!({"id":id,"reason":"current consumer/reference or retention window"}),
                );
                continue;
            }
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
                    && nodes < self.policy.max_nodes_per_round
                    && began.elapsed().as_millis() < self.policy.max_millis_per_round as u128
                {
                    nodes += 1;
                    l.outputs[i].manifest.step(
                        &self.root,
                        &o.path,
                        self.policy.max_manifest_nodes,
                        self.policy.max_output_bytes,
                    )?;
                    self.save(&l, &gate)?;
                }
                if !l.outputs[i].manifest.sealed {
                    return Ok(false);
                }
                l.outputs[i].reserved_bytes = l.outputs[i]
                    .manifest
                    .bytes
                    .saturating_sub(l.outputs[i].removed_logical_bytes);
                l.outputs[i].phase = "disposing".into();
                self.save(&l, &gate)?;
                while self.metadata_room()
                    && nodes < self.policy.max_nodes_per_round
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
                            self.save(&l, &gate)?;
                            continue;
                        }
                        Err(e) => return Err(e.to_string()),
                        Ok(_) => (),
                    }
                    Manifest::validate(&entry, &self.root)?;
                    if !o.tree
                        && entry.bytes
                            > self
                                .policy
                                .max_bytes_per_round
                                .saturating_sub(verified_bytes)
                    {
                        return Err(
                            "output exceeds remaining round byte bound; preserve remainder".into(),
                        );
                    }
                    if !o.tree {
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
                            self.stable(&gate)?;
                            lease.stable()?;
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
                    self.save(&l, &gate)?;
                }
                Ok(l.outputs[i].manifest.entries.is_empty())
            })();
            match effect {
                Ok(true) => {
                    let o = l.outputs.remove(i);
                    let summary = json!({"id":o.id,"producer":o.producer,"path":o.path,"identity":o.identity,"sha256":o.sha256,"original_outcome":o.outcome,"original":disposal_fact,"removed_logical_bytes":o.removed_logical_bytes,"physical_bytes_reclaimed":null});
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
                    });
                    self.save(&l, &gate)?;
                    drop(lease);
                    // The explicit retirement queue survives interruption at either side
                    // of unlink. Its size participates in admission capacity.
                    if nodes < self.policy.max_nodes_per_round {
                        nodes += 1;
                        crate::artifact_disposal::dispose_entry(
                            &self.directory,
                            &l.retired_leases.last().ok_or("retirement pending")?.path,
                            &l.retired_leases.last().unwrap().identity,
                            || self.stable(&gate),
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
            json!({"schema":"chrono-output-retention-round/v1","nodes":nodes,"metadata_inventory_entries":count,"accounting_errors":accounting_errors,"metadata_graph_edges":l.outputs.iter().map(|o|o.references.len()).sum::<usize>(),"metadata_read_bound":MAX_STATE,"metadata_bytes":self.policy.max_metadata_bytes_per_round-self.round_metadata.get().unwrap_or(0),"max_metadata_bytes_per_round":self.policy.max_metadata_bytes_per_round,"directory_snapshot_entries_bound":self.policy.max_manifest_nodes,"removed_logical_bytes":bytes,"verified_bytes":verified_bytes,"physical_bytes_reclaimed":null,"remaining_outputs":l.outputs.len(),"remaining_capability_retirements":l.retired_leases.len(),"known_retained_file_logical_bytes":l.outputs.iter().filter_map(|o|o.length).sum::<u64>(),"unmeasured_tree_outputs":l.outputs.iter().filter(|o|!o.manifest.sealed).count(),"reserved_logical_bytes":l.outputs.iter().map(|o|o.reserved_bytes).sum::<u64>(),"max_total_bytes":self.policy.max_total_bytes,"max_output_bytes":self.policy.max_output_bytes,"inventory_capacity":self.policy.max_entries,"protected":protected,"disposed":disposed,"legacy_unenrolled":"not-scanned; owner migration required","completed_work":"not-claimed"}),
        )
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
        let gate = self.gate()?;
        let mut l = self.load()?;
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
        let _activity = self.report_activity()?;
        let gate = self.gate()?;
        let mut l = self.load()?;
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
        let gate = self.gate()?;
        let l = self.load()?;
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
        Ok(lease)
    }
}
impl Publication {
    /// An adopted producer checks this reservation before writing new output bytes.
    pub fn byte_limit(&self) -> u64 {
        self.inventory.policy.max_output_bytes
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
    /// Keep producer status distinct from storage status; no synthetic success on interruption.
    pub fn complete(self, outcome: Value) -> Result<(), String> {
        if serde_json::to_vec(&outcome)
            .map_err(|e| e.to_string())?
            .len()
            > 16384
        {
            return Err("output outcome metadata bound".into());
        }
        self.lease.stable()?;
        let gate = self.inventory.gate()?;
        let mut l = self.inventory.load()?;
        let o = l
            .outputs
            .iter_mut()
            .find(|o| o.id == self.id)
            .ok_or("publication identity missing")?;
        let path = no_symlink_parents(&self.inventory.root, &o.path)?;
        if identity(&path)? != o.identity {
            return Err("publication output identity changed".into());
        }
        if !o.tree {
            let (digest, length) = crate::file_identity(&path)?;
            o.sha256 = Some(digest);
            o.length = Some(length);
        }
        o.outcome = outcome;
        o.phase = "sealing".into();
        let began = Instant::now();
        let mut steps = 0;
        while !o.manifest.sealed
            && steps < self.inventory.policy.max_manifest_nodes.saturating_mul(4)
            && began.elapsed().as_millis() < self.inventory.policy.max_millis_per_round as u128
        {
            steps += 1;
            if let Err(error) = o.manifest.step(
                &self.inventory.root,
                &o.path,
                self.inventory.policy.max_manifest_nodes,
                self.inventory.policy.max_output_bytes,
            ) {
                self.inventory.save(&l, &gate)?;
                return Err(error);
            }
        }
        if o.manifest.sealed {
            o.reserved_bytes = o.manifest.bytes;
            o.phase = "released".into();
        }
        o.expires = now()?.saturating_add(self.inventory.policy.keep_seconds);
        self.inventory.save(&l, &gate)
    }
}
/// Ordinary adopted owners share this entry. Unknown legacy outputs remain protected.
pub fn maintain_adopted(root: &Path) -> Result<Option<Value>, String> {
    Inventory::adopted(root)?.map(|i| i.maintain()).transpose()
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
    inventory.maintain()?;
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
/// A registered reader protects an enrolled original. Unknown legacy evidence retains old semantics.
pub fn read_guard(root: &Path, path: &str) -> Result<Option<Lease>, String> {
    let Some(inventory) = Inventory::adopted(root)? else {
        return Ok(None);
    };
    let gate = inventory.gate()?;
    let l = inventory.load()?;
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
    Ok(Some(lease))
}

/// Cross-process original publication/collection lifetime, independent of linked worktree enrollment.
pub fn report_activity(root: &Path) -> Result<Option<Lease>, String> {
    Inventory::adopted(root)?
        .map(|i| i.report_activity())
        .transpose()
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
    publish_owned_with(root, producer, path, length, digest, outcome, |file| {
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
    })
}

fn publish_owned_with(
    root: &Path,
    producer: &str,
    path: &str,
    length: u64,
    digest: &str,
    outcome: Value,
    write_content: impl FnOnce(&mut fs::File) -> Result<(), String>,
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
    let _activity = inventory.report_activity()?;
    let _writer = Lease::acquire(
        &inventory.directory.join("report-writer.lease"),
        None,
        true,
        true,
        Some(Duration::from_secs(5)),
    )?
    .ok_or("report original publisher busy")?;
    let target = no_symlink_parents(root, path)?;
    fs::create_dir_all(target.parent().ok_or("original parent")?).map_err(|e| e.to_string())?;
    let mut file = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
    {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            if crate::file_identity(&target)? != (digest.to_owned(), length) {
                return Err("addressed original bytes mismatch".into());
            }
            inventory.validate_original_reuse(path)?;
            // Reuse leaves the first immutable operation outcome intact.
            return Ok(true);
        }
        Err(e) => return Err(e.to_string()),
    };
    let publication = inventory.begin(
        producer,
        path,
        json!({"publication":"started","outcome":"not-established"}),
    )?;
    write_content(&mut file)?;
    file.sync_all().map_err(|e| e.to_string())?;
    publication.complete(outcome)?;
    Ok(true)
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
    let publisher = Lease::acquire(
        &inventory.directory.join("report-slot.lease"),
        None,
        true,
        true,
        Some(Duration::from_secs(5)),
    )?
    .ok_or("report slot publisher busy")?;
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
        inventory.reference_paths(
            &format!("current-report:{}", sha256(slot.as_bytes())),
            &[original.into()],
        )?;
    }
    Ok(())
}
