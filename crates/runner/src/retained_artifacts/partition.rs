//! Addressed retention metadata. The namespace contains declarations, not a
//! mutable inventory. Multi-record exclusion is limited to actual graph edges.
//! These synchronous mutations have no descendant owner. Their local leases
//! end on return even while an unrelated fork holds incidental fd copies.
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
use super::super::contains;
#[cfg(verus_keep_ghost)]
use super::identifiers;
#[cfg(not(verus_keep_ghost))]
use crate::verified_identifiers as identifiers;
#[cfg(not(verus_keep_ghost))]
use crate::verified_release::contains;
#[cfg(verus_keep_ghost)]
use std::collections::BTreeSet;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub open spec fn output_edge_matches(row: (&str, &[String]), source: Seq<char>,
    target: Seq<char>) -> bool
{
    row.0@ == source && exists|i: int| 0 <= i < row.1@.len()
        && (#[trigger] row.1@[i])@ == target
}

pub open spec fn output_reference_trace<'a, T, F>(outputs: &'a [T], observe: F,
    source: Seq<char>, target: Seq<char>, found: bool,
    trace: Seq<(&'a str, &'a [String])>) -> bool
    where F: Fn(&'a T) -> (&'a str, &'a [String]),
{
    trace.len() <= outputs.len()
        && (forall|i: int| 0 <= i < trace.len() ==>
            #[trigger] observe.ensures((&outputs@[i],), trace[i]))
        && if found {
            trace.len() > 0 && output_edge_matches(trace.last(), source, target)
                && forall|i: int| 0 <= i < trace.len() - 1 ==>
                    !output_edge_matches(#[trigger] trace[i], source, target)
        } else {
            trace.len() == outputs.len() && forall|i: int| 0 <= i < trace.len() ==>
                !output_edge_matches(#[trigger] trace[i], source, target)
        }
}

/// 借用实际记录；只有同一来源的完整引用匹配才提前返回，不分配投影库存。
pub fn objects_reference<'a, T, F>(outputs: &'a [T], source: &str, target: &str,
    observe: F) -> (found: bool)
    where F: Fn(&'a T) -> (&'a str, &'a [String]),
    requires forall|row: &'a T| observe.requires((row,)),
    ensures exists|trace: Seq<(&'a str, &'a [String])>| output_reference_trace(
        outputs, observe, source@, target@, found, trace),
{
    let ghost mut trace: Seq<(&'a str, &'a [String])> = Seq::empty();
    let mut index = 0;
    while index < outputs.len()
        invariant index <= outputs.len(), trace.len() == index,
            forall|row: &'a T| observe.requires((row,)),
            forall|i: int| 0 <= i < trace.len() ==>
                #[trigger] observe.ensures((&outputs@[i],), trace[i]),
            forall|i: int| 0 <= i < trace.len() ==>
                !output_edge_matches(#[trigger] trace[i], source@, target@),
        decreases outputs.len() - index,
    {
        let row = observe(&outputs[index]);
        proof { trace = trace.push(row); }
        if identifiers::equal_text(row.0, source) && contains(row.1, target) {
            assert(output_edge_matches(row, source@, target@));
            assert(output_reference_trace(outputs, observe, source@, target@, true, trace));
            return true;
        }
        index += 1;
    }
    assert(output_reference_trace(outputs, observe, source@, target@, false, trace));
    false
}

/// 选择父报告退役时，已观察的其他原件继续受保护。
pub fn preserve_frontier_original(id: &str, retiring: &str) -> (retain: bool)
    ensures retain == (id@ != retiring@),
{
    !identifiers::equal_text(id, retiring)
}

/// Only pending metadata edges of an already empty retiring report can advance
/// without physical path exclusion. Final address retirement still needs it.
pub fn metadata_only_retirement(retiring: bool, disposing: bool, sealed: bool,
    entries_empty: bool, pending_references: bool) -> (metadata_only: bool)
    ensures metadata_only == (retiring && disposing && sealed && entries_empty
        && pending_references),
{
    retiring && disposing && sealed && entries_empty && pending_references
}

/// 只有退役条件已满足且每个剩余引用均已载入时，才尝试共同持有物理路径。
pub fn metadata_path_ready(metadata_only: bool, references: &[String],
    loaded: &BTreeSet<String>) -> (ready: bool)
    requires vstd::laws_cmp::obeys_cmp::<String>(),
    ensures ready == (metadata_only && forall|i: int| 0 <= i < references@.len()
        ==> loaded@.contains(#[trigger] references@[i])),
{
    if !metadata_only { return false; }
    let mut index = 0;
    while index < references.len()
        invariant
            vstd::laws_cmp::obeys_cmp::<String>(),
            0 <= index <= references@.len(),
            forall|i: int| 0 <= i < index
                ==> loaded@.contains(#[trigger] references@[i]),
        decreases references@.len() - index,
    {
        if !loaded.contains(&references[index]) { return false; }
        index += 1;
    }
    true
}
}

#[cfg(not(verus_keep_ghost))]
use super::*;
#[cfg(not(verus_keep_ghost))]
use crate::ownership::DirectoryLease;
#[cfg(not(verus_keep_ghost))]
use crate::verified_release as verified;

// A sweep observes independently bounded rounds. These counts describe actual
// observations, not a deduplicated inventory or a new source of deletion rights.
#[cfg(not(verus_keep_ghost))]
struct SweepReport {
    value: Value,
}
#[cfg(not(verus_keep_ghost))]
impl SweepReport {
    fn new(policy: &Policy) -> Self {
        Self {
            value: json!({
                "schema":"chrono-output-retention-sweep/v1",
                "scope":{"kind":"visited-declarations","complete":false},
                "remaining_outputs":null,"physical_bytes_reclaimed":null,
                "completed_work":"not-claimed","removed_logical_bytes":0,
                "limits_per_round":{"nodes":policy.max_nodes_per_round,
                    "metadata_bytes":policy.max_metadata_bytes_per_round,
                    "verified_bytes":policy.max_bytes_per_round,
                    "millis":policy.max_millis_per_round},
                "observed_work":{"attempted_rounds":0,"reported_rounds":0,
                    "failed_rounds":0,"nodes":0,"metadata_bytes":0,"verified_bytes":0},
                "round_maxima":{"nodes":0,"metadata_bytes":0,"verified_bytes":0},
                "observations":{"disposed":0,"protected":0,"accounting_errors":0},
                "samples":{"payload_byte_limit":65536,"payload_bytes":0,"omitted":0,"items":[]},
                "enumeration":{}
            }),
        }
    }
    fn add(value: &mut Value, amount: u64) -> Result<(), String> {
        *value = json!(
            value
                .as_u64()
                .ok_or("sweep counter format")?
                .checked_add(amount)
                .ok_or("sweep counter overflow; completion not established")?
        );
        Ok(())
    }
    fn observation(
        &mut self,
        namespace: &str,
        address: Option<&str>,
        kind: &str,
        observation: Value,
    ) -> Result<(), String> {
        Self::add(&mut self.value["observations"][kind], 1)?;
        let sample = json!({"namespace":namespace,"address":address,
            "kind":kind,"observation":observation});
        let bytes = serde_json::to_vec(&sample)
            .map_err(|e| e.to_string())?
            .len() as u64;
        let samples = &mut self.value["samples"];
        if verified::observation_bytes_fit(
            samples["payload_bytes"]
                .as_u64()
                .ok_or("sweep sample bytes")?,
            bytes,
            samples["payload_byte_limit"]
                .as_u64()
                .ok_or("sweep sample limit")?,
        ) {
            Self::add(&mut samples["payload_bytes"], bytes)?;
            samples["items"]
                .as_array_mut()
                .ok_or("sweep samples")?
                .push(sample);
        } else {
            Self::add(&mut samples["omitted"], 1)?;
        }
        Ok(())
    }
    fn round(
        &mut self,
        namespace: &str,
        address: &str,
        round: Result<Value, String>,
    ) -> Result<(), String> {
        Self::add(&mut self.value["observed_work"]["attempted_rounds"], 1)?;
        let round = match round {
            Ok(round) => round,
            Err(error) => {
                Self::add(&mut self.value["observed_work"]["failed_rounds"], 1)?;
                return self.observation(
                    namespace,
                    Some(address),
                    "accounting_errors",
                    json!({"reason":error,"completion":"not-established"}),
                );
            }
        };
        Self::add(&mut self.value["observed_work"]["reported_rounds"], 1)?;
        for field in ["nodes", "metadata_bytes", "verified_bytes"] {
            let amount = round[field].as_u64().ok_or("sweep round cost")?;
            Self::add(&mut self.value["observed_work"][field], amount)?;
            let maximum = self.value["round_maxima"][field]
                .as_u64()
                .ok_or("sweep round maximum")?;
            self.value["round_maxima"][field] = json!(maximum.max(amount));
        }
        Self::add(
            &mut self.value["removed_logical_bytes"],
            round["removed_logical_bytes"]
                .as_u64()
                .ok_or("sweep removed logical bytes")?,
        )?;
        if let Some(retired) = round["retired_object_metadata"].as_array() {
            for id in retired {
                self.observation(
                    namespace,
                    Some(address),
                    "disposed",
                    json!({"kind":"object-metadata","id":id,"original":"verified-absent"}),
                )?;
            }
        }
        for kind in ["disposed", "protected", "accounting_errors"] {
            for observation in round[kind].as_array().ok_or("sweep round observations")? {
                self.observation(namespace, Some(address), kind, observation.clone())?;
            }
        }
        Ok(())
    }
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Object {
    schema: String,
    directory_identity: String,
    policy_sha256: String,
    id: String,
    path: String,
    output: Option<OutputRecord>,
    incoming: BTreeSet<String>,
    roles: BTreeSet<String>,
    #[serde(deserialize_with = "strict_summaries")]
    summaries: Vec<Value>,
    retired_leases: Vec<Node>,
    cursor: usize,
    metadata_expires: u64,
    prune_path: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
struct Role {
    observation: Option<ObservationOrder>,
    schema: String,
    directory_identity: String,
    role: String,
    ids: Vec<String>,
    attached: BTreeSet<String>,
    metadata_expires: u64,
    #[serde(deserialize_with = "strict_summaries")]
    summaries: Vec<Value>,
}
/// One actual request, qualified by its enrolled producer output rather than a clock.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
pub struct ObservationToken {
    pub role: String,
    pub epoch: String,
    pub serial: u64,
    pub attempt: u64,
    pub original: OriginalIdentity,
}
#[cfg(not(verus_keep_ghost))]
impl ObservationToken {
    pub fn binding(&self) -> verified::ObservationBinding<'_> {
        verified::ObservationBinding {
            role: &self.role,
            epoch: &self.epoch,
            id: &self.original.id,
            path: &self.original.path,
            physical: &self.original.identity,
            serial: self.serial,
            attempt: self.attempt,
        }
    }
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
struct ObservationOrder {
    epoch: String,
    issued: u64,
    accepted: u64,
    current: Option<String>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
struct Address {
    path: String,
    output: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    metadata_expires: Option<u64>,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Copy, PartialEq)]
enum PathAccess {
    Lookup,
    Reserve,
    Maintain,
    Transfer,
}
#[cfg(not(verus_keep_ghost))]
pub(super) struct Gate {
    leases: Vec<DirectoryLease>,
    objects: RefCell<BTreeMap<String, Object>>,
    roles: RefCell<BTreeMap<String, Role>>,
    paths: BTreeMap<String, PathBuf>,
    exclusive_paths: BTreeSet<String>,
    maintenance: bool,
    partial_membership: bool,
    retiring_report: Option<String>,
    retained_dependencies: BTreeSet<String>,
    maintenance_targets: BTreeSet<String>,
    unvisited_relations: BTreeSet<String>,
}
#[cfg(not(verus_keep_ghost))]
impl Gate {
    pub(super) fn excludes_path(&self, path: &str) -> bool {
        self.exclusive_paths.contains(path)
    }
    pub(super) fn retiring_report(&self, id: &str) -> bool {
        self.retiring_report.as_deref() == Some(id)
    }
    pub(super) fn retained_dependencies(&self) -> &BTreeSet<String> {
        &self.retained_dependencies
    }
    pub(super) fn maintains_output(&self, id: &str) -> bool {
        !self.retained_dependencies.contains(id) || self.maintenance_targets.contains(id)
    }
    fn measurements(&self, max_output_bytes: u64) -> BTreeMap<String, [u64; 8]> {
        self.objects
            .borrow()
            .iter()
            .map(|(id, record)| {
                (
                    id.clone(),
                    retained_measurements(
                        record.output.as_deref(),
                        record.retired_leases.len(),
                        max_output_bytes,
                    ),
                )
            })
            .collect()
    }
    pub(super) fn stable(&self) -> Result<(), String> {
        for lease in &self.leases {
            lease.stable()?;
        }
        Ok(())
    }
}
#[cfg(not(verus_keep_ghost))]
impl Inventory {
    fn metadata_creation_expiry(&self) -> Result<u64, String> {
        let created = now()?;
        let requested = created.saturating_add(self.policy.metadata_keep_seconds);
        Ok(match self.policy.max_lifetime_seconds {
            Some(window) => super::bounded_expiry(requested, created.saturating_add(window)),
            None => requested,
        })
    }

    /// The held role is the existing owner's authority. Follow only reciprocal
    /// incoming edges of the requested original, preferring that role's direct
    /// members. Discovery uses one local metadata descriptor at a time; the
    /// resulting path is then excluded and revalidated as one snapshot. No
    /// membership, deadline, recovery root or caller dependency is written here.
    pub(super) fn partition_consumer_read(
        &self,
        consumer: &str,
        original: &Output,
        observe: &mut dyn FnMut(&str),
    ) -> Result<Lease, String> {
        let directory = self.role_directory(consumer)?;
        self.read_role(consumer)?
            .ok_or("existing retention consumer unavailable")?;
        let owner = DirectoryLease::acquire_local(&directory, false, None)?
            .ok_or("retention consumer metadata is busy")?;
        let role = self
            .read_role(consumer)?
            .ok_or("existing retention consumer disappeared")?;
        let mut pending = BTreeMap::from([(original.id.clone(), BTreeSet::<String>::new())]);
        let mut successors = BTreeMap::new();
        let mut capability = None;
        while !pending.is_empty() {
            let id = pending
                .keys()
                .find(|id| role.ids.contains(id))
                .or_else(|| pending.keys().next())
                .unwrap()
                .clone();
            let children = pending.remove(&id).unwrap();
            if successors.contains_key(&id) {
                continue;
            }
            let directory = self.object_directory(&id)?;
            let guard = DirectoryLease::acquire_local(&directory, false, None)?
                .ok_or_else(|| format!("retention output metadata is busy: {id}"))?;
            let record = self.read_object(&id, &directory)?;
            if record.policy_sha256 != sha256(&self.policy_bytes) {
                return Err(
                    "retention policy migration required; original record preserved".into(),
                );
            }
            let output = record
                .output
                .as_ref()
                .ok_or("existing consumer original unavailable")?;
            let successor = children
                .iter()
                .find(|child| output.references.contains(child))
                .cloned();
            if !children.is_empty() && successor.is_none() {
                continue; // Interrupted removals can leave conservative incoming edges.
            }
            if output.phase != "released" || !output.manifest.sealed {
                return Err("existing consumer original is not a sealed released output".into());
            }
            if id == original.id {
                if output.path != original.path
                    || output.identity != original.identity
                    || output.lease != original.lease
                    || output.lease_id != original.lease_id
                {
                    return Err("existing consumer original address changed".into());
                }
                crate::relative_path(&output.lease)?;
                capability = Some(
                    Lease::acquire(
                        &no_symlink_parents(&self.directory, &output.lease)?,
                        Some(&output.lease_id),
                        false,
                        false,
                        None,
                    )?
                    .ok_or("original evidence disposal busy")?,
                );
                if identity(&no_symlink_parents(&self.root, &output.path)?)? != output.identity {
                    return Err("existing consumer original identity changed".into());
                }
            }
            self.stable(&guard)?;
            successors.insert(id.clone(), successor);
            drop(guard);
            observe(&id);
            if role.ids.contains(&id) {
                if !record.roles.contains(consumer) {
                    return Err("existing consumer membership is not reciprocal".into());
                }
                self.verify_consumer_read_path(consumer, original, &id, &successors)?;
                self.stable(&owner)?;
                let capability =
                    capability.ok_or("existing consumer read lacks original capability")?;
                self.stable(&capability)?;
                return Ok(capability);
            }
            for parent in &record.incoming {
                if !successors.contains_key(parent) {
                    pending
                        .entry(parent.clone())
                        .or_default()
                        .insert(id.clone());
                }
            }
            if successors.len() + pending.len() > self.policy.max_related_outputs {
                return Err(
                    "existing consumer read exceeds related retention graph capacity".into(),
                );
            }
        }
        Err("original is not protected by the existing retention consumer".into())
    }

    // The caller still holds the original consumer and original-byte leases.
    // Discovery observations may span different graph states. Only this final
    // simultaneous exclusion establishes a path that can authorize the read.
    fn verify_consumer_read_path(
        &self,
        consumer: &str,
        original: &Output,
        member: &str,
        successors: &BTreeMap<String, Option<String>>,
    ) -> Result<(), String> {
        let mut path = Vec::new();
        let mut current = member;
        loop {
            if path.len() >= self.policy.max_related_outputs {
                return Err(
                    "existing consumer path exceeds related retention graph capacity".into(),
                );
            }
            path.push(current.to_owned());
            match successors.get(current) {
                Some(Some(next)) => current = next,
                Some(None) if current == original.id => break,
                _ => return Err("existing consumer reference path is incomplete".into()),
            }
        }
        let mut guards = Vec::new();
        // Never wait while holding the role in the opposite acquisition order
        // to publishers. Width does not increase this descriptor set; resource
        // exhaustion or a busy path fails without granting a read capability.
        for id in path.iter().collect::<BTreeSet<_>>() {
            guards.push(
                DirectoryLease::acquire_local(&self.object_directory(id)?, false, None)?
                    .ok_or_else(|| format!("retention output metadata is busy: {id}"))?,
            );
        }
        let role = self
            .read_role(consumer)?
            .ok_or("existing retention consumer disappeared")?;
        if !role.ids.iter().any(|id| id == member) {
            return Err("existing consumer membership changed".into());
        }
        let policy = sha256(&self.policy_bytes);
        for (index, id) in path.iter().enumerate() {
            let record = self.read_object(id, &self.object_directory(id)?)?;
            if record.policy_sha256 != policy {
                return Err(
                    "retention policy migration required; original record preserved".into(),
                );
            }
            let output = record
                .output
                .as_ref()
                .ok_or("existing consumer original unavailable")?;
            if output.phase != "released" || !output.manifest.sealed {
                return Err("existing consumer original is not a sealed released output".into());
            }
            if index == 0 && !record.roles.contains(consumer) {
                return Err("existing consumer membership is not reciprocal".into());
            }
            if (index > 0 && !record.incoming.contains(&path[index - 1]))
                || path
                    .get(index + 1)
                    .is_some_and(|child| !output.references.contains(child))
            {
                return Err("existing consumer reference path changed".into());
            }
            if id == &original.id
                && (output.path != original.path
                    || output.identity != original.identity
                    || output.lease != original.lease
                    || output.lease_id != original.lease_id
                    || output.lifetime != original.lifetime)
            {
                return Err("existing consumer original address or lifetime changed".into());
            }
        }
        if identity(&no_symlink_parents(&self.root, &original.path)?)? != original.identity {
            return Err("existing consumer original identity changed".into());
        }
        for guard in &guards {
            self.stable(guard)?;
        }
        Ok(())
    }

    /// Keep the consumer excluded, visiting only one member at a time. Install
    /// every requested member before releasing obsolete ones; failures retain
    /// conservative protection. No file descriptor set scales with membership.
    pub(super) fn partition_reference_paths(
        &self,
        consumer: &str,
        paths: &[String],
        prune_only: bool,
    ) -> Result<(), String> {
        if consumer.is_empty() || consumer.len() > 256 {
            return Err("retention consumer capacity/identity".into());
        }
        for path in paths {
            crate::relative_path(path)?;
        }
        let directory = self.role_directory(consumer)?;
        fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        let lease = DirectoryLease::acquire_local(&directory, true, Some(Duration::from_secs(5)))?
            .ok_or("retention consumer metadata is busy")?;
        self.stable(&lease)?;
        let current = self.read_role(consumer)?;
        let fresh = current.is_none();
        let mut role = current.unwrap_or(Role {
            observation: None,
            schema: "chrono-retained-role/v2".into(),
            directory_identity: identity(&directory)?,
            role: consumer.into(),
            ids: vec![],
            attached: BTreeSet::new(),
            metadata_expires: self.metadata_creation_expiry()?,
            summaries: vec![],
        });
        // Empty and failed admissions still leave a typed, finitely retained
        // declaration rather than an unknown directory in the consumer namespace.
        if fresh {
            self.write_metadata(&directory.join("record.json"), &role)?;
        }
        let mut requested = BTreeMap::new();
        let mut next = BTreeSet::new();
        // Resolve and validate the complete explicit input before changing any
        // membership. Under the held role, path/object gates must never wait:
        // ordinary publishers acquire paths before the same consumer role.
        for path in paths {
            let gate =
                self.addressed_gate_scope(&[], &[path.clone()], &[], None, false, false, true)?;
            let ledger = self.load(&gate)?;
            let output = ledger.outputs.iter().find(|output| {
                output.path == *path
                    || (output.tree && path.starts_with(&format!("{}/", output.path)))
            });
            if output.is_some_and(|output| output.phase == "disposing") {
                return Err("referenced original disposal has started".into());
            }
            if !prune_only {
                if let Some(output) = output.filter(|output| {
                    !role.ids.contains(&output.id) && !role.attached.contains(&output.id)
                }) {
                    output.admit(now()?)?;
                }
            }
            let id = output.map(|output| output.id.clone());
            next.extend(id.iter().cloned());
            requested.insert(path.clone(), id);
        }
        if prune_only {
            // The domain has already committed its smaller declaration. Only
            // release obsolete protection; installing a new member still goes
            // through the ordinary conservative admission below.
            next.retain(|id| role.ids.contains(id) || role.attached.contains(id));
            requested.clear();
        }
        let related: BTreeSet<_> = role
            .ids
            .iter()
            .chain(role.attached.iter())
            .chain(next.iter())
            .collect();
        if related.len() > self.policy.max_related_outputs {
            return Err("related retention graph capacity; originals preserved".into());
        }
        if role.observation.is_some() {
            if role.ids.iter().cloned().collect::<BTreeSet<_>>() != next {
                return Err("observation membership requires its domain transition".into());
            }
            return self.stable(&lease);
        }
        for (path, expected) in requested {
            let gate =
                self.addressed_gate_scope(&[], &[path.clone()], &[], None, false, false, true)?;
            let ledger = self.load(&gate)?;
            if let Some(id) = expected
                .as_ref()
                .filter(|id| !role.ids.contains(id) && !role.attached.contains(*id))
            {
                self.admit(&ledger, std::slice::from_ref(id))?;
            }
            let actual = ledger
                .outputs
                .iter()
                .find(|output| {
                    output.path == path
                        || (output.tree && path.starts_with(&format!("{}/", output.path)))
                })
                .map(|output| output.id.clone());
            if actual != expected {
                return Err("referenced original address changed; originals preserved".into());
            }
            if let Some(id) = expected {
                self.save_membership_component(gate, &lease, &mut role, Some(&id))?;
            }
        }
        let obsolete: BTreeSet<_> = role
            .ids
            .iter()
            .chain(role.attached.iter())
            .filter(|id| !next.contains(*id))
            .cloned()
            .collect();
        for id in obsolete {
            let gate = self.addressed_gate_scope(&[id], &[], &[], None, false, false, true)?;
            self.save_membership_component(gate, &lease, &mut role, None)?;
        }
        let ordered: Vec<_> = next.into_iter().collect();
        self.stable(&lease)?;
        if role.ids != ordered {
            role.ids = ordered;
            self.write_metadata(&directory.join("record.json"), &role)?;
        }
        self.stable(&lease)
    }

    fn save_membership_component(
        &self,
        mut gate: Gate,
        lease: &DirectoryLease,
        role: &mut Role,
        keep: Option<&str>,
    ) -> Result<(), String> {
        self.stable(lease)?;
        // Reuse the existing component projection and conservative inbound /
        // root / obsolete-backreference publication order. Outward dependencies
        // and all members outside this exclusion stay unchanged.
        gate.partial_membership = true;
        gate.roles
            .borrow_mut()
            .insert(role.role.clone(), role.clone());
        let mut ledger = self.load(&gate)?;
        if let Some(id) = keep {
            self.reference_locked(&mut ledger, &role.role, &[id.into()])?;
        } else {
            ledger.roots.remove(&role.role);
        }
        self.save(&ledger, &gate)?;
        *role = gate
            .roles
            .borrow()
            .get(&role.role)
            .ok_or("consumer component disappeared")?
            .clone();
        self.stable(lease)
    }

    /// Reserve one request under only this object's existing consumer metadata.
    /// Membership protects the actual pending output, including unknown tasks.
    pub fn reserve_observation(
        &self,
        role: &str,
        path: &str,
        attempt: u64,
    ) -> Result<ObservationToken, String> {
        if !verified::observation_role(role.trim(), role.len(), attempt) {
            return Err("invalid observation role/attempt".into());
        }
        let gate = self.addressed_gate_scope(
            &[],
            &[path.into()],
            &[role.into()],
            None,
            false,
            true,
            true,
        )?;
        let mut ledger = self.load(&gate)?;
        let output = ledger
            .outputs
            .iter()
            .find(|o| o.path == path)
            .ok_or("observation producer is not enrolled")?;
        if !verified::observation_producer(
            output.tree,
            output.phase == "publishing",
            identity(&no_symlink_parents(&self.root, path)?)? == output.identity,
        ) {
            return Err("observation requires the actual publishing producer tree".into());
        }
        let original = OriginalIdentity {
            id: output.id.clone(),
            path: path.into(),
            identity: output.identity.clone(),
        };
        let record = gate
            .roles
            .borrow()
            .get(role)
            .cloned()
            .ok_or("observation role is not excluded")?;
        if !verified::observation_owner(
            record.observation.is_some(),
            record.ids.is_empty(),
            record.attached.is_empty(),
        ) {
            return Err("ordinary consumer cannot become an observation owner".into());
        }
        let mut order = record.observation.unwrap_or(ObservationOrder {
            epoch: sha256(
                &serde_json::to_vec(&(role, &original, attempt)).map_err(|e| e.to_string())?,
            ),
            issued: 0,
            accepted: 0,
            current: None,
        });
        order.issued =
            verified::next_observation(order.issued).ok_or("observation sequence exhausted")?;
        let mut ids = record.ids;
        if !ids.contains(&original.id) {
            ids.push(original.id.clone());
        }
        self.reference_locked(&mut ledger, role, &ids)?;
        let token = ObservationToken {
            role: role.into(),
            epoch: order.epoch.clone(),
            serial: order.issued,
            attempt,
            original,
        };
        self.stable(&gate)?;
        self.partition_save_observation(&ledger, &gate, Some((role, order)))?;
        Ok(token)
    }

    /// The domain owner has joined the command, sealed its result and handed off
    /// all inputs. Accept its last valid observation, independently of its exit.
    /// A missing result must never call this method.
    pub fn settle_observation(
        &self,
        request: &ObservationToken,
        observed: Option<(&ObservationToken, bool)>,
    ) -> Result<(), String> {
        if !verified::observation_token(
            request.role.trim(),
            request.role.len(),
            request.serial,
            request.attempt,
        ) {
            return Err("invalid observation token".into());
        }
        let gate = self.addressed_gate_scope(
            &[request.original.id.clone()],
            &[request.original.path.clone()],
            &[request.role.clone()],
            None,
            false,
            true,
            true,
        )?;
        let mut ledger = self.load(&gate)?;
        let output = ledger
            .outputs
            .iter()
            .find(|o| o.id == request.original.id)
            .ok_or("observation result is unavailable")?;
        if !verified::observation_result(
            output.tree,
            output.phase == "released",
            output.manifest.sealed,
            output.path == request.original.path && output.identity == request.original.identity,
            identity(&no_symlink_parents(&self.root, &output.path)?)? == output.identity,
        ) {
            return Err("observation result is unsealed or replaced".into());
        }
        let record = gate
            .roles
            .borrow()
            .get(&request.role)
            .cloned()
            .ok_or("observation role is not excluded")?;
        let Some(mut order) = record.observation else {
            // An already empty, expired role may have been reclaimed. This never
            // creates a new root or sequence from an old completed operation.
            if verified::observation_owner(false, record.ids.is_empty(), record.attached.is_empty())
            {
                return Ok(());
            }
            return Err("observation owner is not registered".into());
        };
        if let Some((observation, _)) = observed {
            if !verified::matching_observation(&request.binding(), &observation.binding()) {
                return Err("observation does not belong to this returned request".into());
            }
        }
        if !record.ids.contains(&request.original.id) {
            return Ok(());
        }
        // Completed output is no longer a member, even after empty metadata
        // expires and another epoch starts. Only a real pending/current member
        // can make a transition, and it must qualify that current epoch.
        if !verified::observation_epoch(
            true,
            order.epoch == request.epoch,
            request.serial,
            order.issued,
        ) {
            return Err("observation epoch/sequence differs".into());
        }
        let settled = verified::settle_observation_members(
            &record.ids,
            &request.original.id,
            &order.current,
            order.accepted,
            observed.map(|(token, active)| (token.serial, active)),
        );
        order.accepted = settled.accepted;
        order.current = settled.current;
        self.reference_locked(&mut ledger, &request.role, &settled.members)?;
        self.stable(&gate)?;
        self.partition_save_observation(&ledger, &gate, Some((&request.role, order)))
    }

    fn remove_metadata_directory(
        &self,
        directory: &Path,
        gate: &DirectoryLease,
        names: &[&str],
    ) -> Result<(), String> {
        let mut files = Vec::new();
        for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "metadata entry UTF8")?;
            if !names.contains(&name.as_str())
                || !entry.file_type().map_err(|e| e.to_string())?.is_file()
            {
                return Err("unknown metadata member; preserve directory".into());
            }
            files.push((name, identity(&entry.path())?));
        }
        for (name, id) in files {
            crate::artifact_disposal::dispose_entry(directory, &name, &id, || self.stable(gate))?;
        }
        self.stable(gate)?;
        fs::remove_dir(directory).map_err(|e| e.to_string())
    }

    fn path_metadata_occupied(&self, directory: &Path, path: &str) -> Result<bool, String> {
        let address: Address = self
            .read_metadata(&no_symlink_parents(directory, "address.json")?)?
            .ok_or("path metadata declaration missing")?;
        if address.path != path {
            return Err("path metadata identity differs".into());
        }
        if address.output.is_some() {
            return Ok(true);
        }
        for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
            if entry.map_err(|e| e.to_string())?.file_name() != "address.json" {
                return Ok(true);
            }
        }
        Ok(false)
    }

    // Only the owner can publish into this namespace. The name carries both
    // physical identities, including across interruption after unlinking the
    // marker. No replacement directory at the original address is touched.
    fn finish_metadata_retirement(
        &self,
        pending: &Path,
        nodes: &mut usize,
    ) -> Result<bool, String> {
        let relative = pending
            .strip_prefix(&self.directory)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("path retirement address UTF8")?;
        no_symlink_parents(&self.directory, relative)?;
        let Some(gate) = DirectoryLease::acquire_local(pending, true, None)? else {
            return Ok(false);
        };
        let name = pending
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("invalid retiring path name")?;
        let mut fields = name
            .strip_prefix("retiring-")
            .ok_or("invalid path retirement address")?
            .split('_');
        let path_digest = fields.next().unwrap_or_default();
        let dir_id = fields
            .next()
            .ok_or("retiring path directory identity missing")?;
        let marker_id = fields
            .next()
            .ok_or("retiring path marker identity missing")?;
        if fields.next().is_some()
            || path_digest.len() != 64
            || !path_digest.bytes().all(|b| b.is_ascii_hexdigit())
            || !marker_id
                .strip_suffix(":file")
                .and_then(|id| id.split_once(':'))
                .is_some_and(|(dev, ino)| dev.parse::<u64>().is_ok() && ino.parse::<u64>().is_ok())
            || identity(pending)? != dir_id
        {
            return Err("retiring path identity changed".into());
        }
        let parent = pending.parent().ok_or("retiring metadata parent")?;
        let marker_name = if parent == self.directory.join("roles") {
            "record.json"
        } else if parent == self.directory.join("paths") {
            "address.json"
        } else {
            return Err("unknown metadata retirement namespace".into());
        };
        let marker = pending.join(marker_name);
        let mut has_marker = false;
        for entry in fs::read_dir(pending).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.file_name() != marker_name || identity(&marker)? != marker_id {
                return Err("retiring metadata contains an unknown or replaced member".into());
            }
            has_marker = true;
        }
        if has_marker {
            let matches = if marker_name == "record.json" {
                self.read_metadata::<Role>(&marker)?.is_some_and(|role| {
                    role.schema == "chrono-retained-role/v2"
                        && role.directory_identity == dir_id
                        && role.ids.is_empty()
                        && role.attached.is_empty()
                        && sha256(role.role.as_bytes()) == path_digest
                })
            } else {
                self.read_metadata::<Address>(&marker)?
                    .is_some_and(|address| {
                        address.output.is_none() && sha256(address.path.as_bytes()) == path_digest
                    })
            };
            if !matches {
                return Err("retiring metadata mapping differs".into());
            }
            if !self.metadata_step_available(*nodes) {
                return Ok(false);
            }
            crate::artifact_disposal::dispose_entry(pending, marker_name, marker_id, || {
                self.stable(&gate)
            })?;
            *nodes += 1;
        }
        if !self.metadata_step_available(*nodes) {
            return Ok(false);
        }
        crate::artifact_disposal::dispose_entry(parent, name, dir_id, || self.stable(&gate))?;
        *nodes += 1;
        Ok(true)
    }

    fn metadata_step_available(&self, nodes: usize) -> bool {
        nodes < self.policy.max_nodes_per_round
            && self.metadata_room()
            && self.round_began.get().unwrap().elapsed().as_millis()
                < self.policy.max_millis_per_round as u128
    }

    fn stage_empty_path(
        &self,
        directory: &Path,
        observed: u64,
        nodes: &mut usize,
    ) -> Result<bool, String> {
        if !self.metadata_step_available(*nodes) {
            return Ok(false);
        }
        let dir_id = identity(directory)?;
        let marker = directory.join("address.json");
        let marker_id = identity(&marker)?;
        let initial_address: Address = self
            .read_metadata(&marker)?
            .ok_or("path address declaration missing")?;
        if self.path_directory(&initial_address.path)? != directory {
            return Err("path declaration address changed".into());
        }
        if initial_address.output.is_some() {
            return Ok(false);
        }
        let deadline = match initial_address.metadata_expires {
            Some(deadline) => deadline,
            None => {
                // Older markers only have the directory's birth time. A zero
                // window also works on file systems that lack birth times.
                let keep = self
                    .policy
                    .metadata_keep_seconds
                    .min(self.policy.max_lifetime_seconds.unwrap_or(u64::MAX));
                if keep == 0 {
                    0
                } else {
                    fs::symlink_metadata(directory)
                        .and_then(|entry| entry.created())
                        .map_err(|e| e.to_string())?
                        .duration_since(UNIX_EPOCH)
                        .map_err(|e| e.to_string())?
                        .as_secs()
                        .saturating_add(keep)
                }
            }
        };
        if observed < deadline {
            return Ok(false);
        }
        let Some(gate) = DirectoryLease::acquire_local(directory, true, None)? else {
            return Ok(false);
        };
        if identity(directory)? != dir_id || identity(&marker)? != marker_id {
            return Err("path identity changed during exclusion".into());
        }
        Ok(self
            .detach_empty_path(directory, &initial_address.path, &gate, nodes)?
            .is_some())
    }

    fn detach_empty_path(
        &self,
        directory: &Path,
        path: &str,
        gate: &DirectoryLease,
        nodes: &mut usize,
    ) -> Result<Option<PathBuf>, String> {
        if self.path_metadata_occupied(directory, path)? || !self.metadata_step_available(*nodes) {
            return Ok(None);
        }
        let dir_id = identity(directory)?;
        let marker = directory.join("address.json");
        let marker_id = identity(&marker)?;
        let pending_root = no_symlink_parents(&self.directory, "paths")?;
        let pending = pending_root.join(format!(
            "retiring-{}_{}_{}",
            sha256(path.as_bytes()),
            dir_id,
            marker_id
        ));
        let relative = directory
            .strip_prefix(&self.directory)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("path address UTF8")?;
        no_symlink_parents(&self.directory, relative)?;
        if !self.metadata_step_available(*nodes) {
            return Ok(None);
        }
        self.stable(gate)?;
        if identity(directory)? != dir_id || identity(&marker)? != marker_id {
            return Err("empty path identity changed before retirement".into());
        }
        crate::artifact_disposal::publish_entry(directory, &pending)?;
        *nodes += 1;
        Ok(Some(pending))
    }

    // Stream the registered hash tree bottom-up. Revalidate each mapping in
    // its own bounded round. A queued directory retains its marker until the
    // owner can finish it; the queue also covers a previous interrupted pass.
    fn sweep_path_metadata(
        &self,
        sweep: &mut SweepReport,
        observed: u64,
        observer: &mut Option<&mut dyn FnMut(&Value) -> Result<(), String>>,
    ) -> Result<(), String> {
        let root = no_symlink_parents(&self.directory, "paths")?;
        let mut record = |scope: &str,
                          path: &Path,
                          mut round: Result<Value, String>|
         -> Result<(), String> {
            let address = path
                .strip_prefix(&self.directory)
                .map_err(|e| e.to_string())?
                .to_string_lossy();
            // Detaching the canonical address leaves actual bytes in custody.
            // Only removal of the pending directory discharges that obligation.
            if let Ok(original) = &mut round {
                if scope == "paths"
                    && original["disposed"]
                        .as_array()
                        .is_some_and(|v| !v.is_empty())
                {
                    original["disposed"] = json!([]);
                    original["retirement_pending"] = json!(true);
                }
            }
            if let Some(observer) = observer.as_deref_mut() {
                let row = match &round {
                    Ok(round)
                        if round["retirement_pending"] != true
                            && round["disposed"].as_array().is_some_and(|v| v.is_empty())
                            && round["accounting_errors"]
                                .as_array()
                                .is_some_and(|v| v.is_empty()) =>
                    {
                        None
                    }
                    Ok(round) => Some(
                        json!({"kind":"round","namespace":scope,"address":address,"original":round}),
                    ),
                    Err(error) => Some(
                        json!({"kind":"round","namespace":scope,"address":address,"error":error}),
                    ),
                };
                if let Some(row) = row {
                    observer(&row)?;
                }
            }
            sweep.round(scope, &address, round)
        };
        let mut stack = Vec::new();
        match fs::read_dir(&root) {
            Ok(entries) => stack.push((root.clone(), entries)),
            Err(error) => record("paths", &root, Err(error.to_string()))?,
        }
        while let Some((_, entries)) = stack.last_mut() {
            match entries.next() {
                Some(Ok(entry)) => {
                    let child = entry.path();
                    let name = entry.file_name();
                    if child.parent() == Some(root.as_path())
                        && name.to_str().is_some_and(|s| s.starts_with("retiring-"))
                    {
                        continue;
                    }
                    if name == "address.json" || name == "address.pending" {
                        continue;
                    }
                    let child_directory = (|| {
                        let valid_name = name.to_str().is_some_and(|s| {
                            s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
                        });
                        if !valid_name || !entry.file_type().map_err(|e| e.to_string())?.is_dir() {
                            return Err("unknown path member; preserved".to_string());
                        }
                        let relative = child
                            .strip_prefix(&self.directory)
                            .map_err(|e| e.to_string())?
                            .to_str()
                            .ok_or("path directory UTF8")?;
                        let verified = no_symlink_parents(&self.directory, relative)?;
                        fs::read_dir(verified).map_err(|e| e.to_string())
                    })();
                    match child_directory {
                        Ok(entries) => stack.push((child, entries)),
                        Err(error) => record("paths", &child, Err(error))?,
                    }
                }
                Some(Err(error)) => {
                    let (path, _) = stack.pop().unwrap();
                    record("paths", &path, Err(error.to_string()))?;
                }
                None => {
                    let (path, _) = stack.pop().unwrap();
                    if path != root {
                        record(
                            "paths",
                            &path,
                            self.metadata_round(|nodes| {
                                self.stage_empty_path(&path, observed, nodes)
                            }),
                        )?;
                    }
                }
            }
        }
        let entries = match fs::read_dir(&root) {
            Ok(entries) => entries,
            Err(error) => return record("retiring-paths", &root, Err(error.to_string())),
        };
        for entry in entries {
            match entry {
                Ok(entry) => {
                    if !entry
                        .file_name()
                        .to_str()
                        .is_some_and(|s| s.starts_with("retiring-"))
                    {
                        continue;
                    }
                    let path = entry.path();
                    record(
                        "retiring-paths",
                        &path,
                        self.metadata_round(|nodes| self.finish_metadata_retirement(&path, nodes)),
                    )?;
                }
                Err(error) => record("retiring-paths", &root, Err(error.to_string()))?,
            }
        }
        Ok(())
    }

    // Old writers could restart the metadata TTL when a body or role member
    // retired. The excluded, identity-checked directory is the same generation
    // across atomic replacements of record.json.
    fn metadata_directory_deadline(
        &self,
        directory: &Path,
        stored: u64,
        observed: u64,
    ) -> Result<u64, String> {
        if stored <= observed {
            return Ok(stored);
        }
        let Some(window) = self.policy.max_lifetime_seconds else {
            return Ok(stored);
        };
        let metadata = fs::symlink_metadata(directory).map_err(|e| e.to_string())?;
        let birth = match metadata.created() {
            Ok(time) => time.duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?,
            Err(error) if error.kind() == std::io::ErrorKind::Unsupported => return Ok(stored),
            Err(error) => return Err(error.to_string()),
        };
        Ok(super::bounded_expiry(
            stored,
            birth.as_secs().saturating_add(window),
        ))
    }

    fn prune_object(
        &self,
        id: &str,
        observed: u64,
        nodes: &mut usize,
        retired: &mut Vec<String>,
    ) -> Result<(), String> {
        let directory = self.object_directory(id)?;
        let Some(gate) = DirectoryLease::acquire_local(&directory, true, None)? else {
            return Ok(());
        };
        let mut record = self.read_object(id, &directory)?;
        if record.output.is_some()
            || !record.incoming.is_empty()
            || !record.roles.is_empty()
            || !record.retired_leases.is_empty()
            || self.directory.join(id).exists()
        {
            return Ok(());
        }
        let deadline =
            self.metadata_directory_deadline(&directory, record.metadata_expires, observed)?;
        if deadline > observed {
            if deadline < record.metadata_expires
                && *nodes < self.policy.max_nodes_per_round
                && self.metadata_room()
            {
                record.metadata_expires = deadline;
                self.stable(&gate)?;
                self.write_metadata(&directory.join("record.json"), &record)?;
                *nodes += 1;
            }
            return Ok(());
        }
        let previous_deadline = record.metadata_expires;
        record.metadata_expires = deadline;
        let initial_prune_path = record.prune_path.clone();
        while let Some(path) = record.prune_path.clone() {
            if *nodes + 4 > self.policy.max_nodes_per_round
                || !self.metadata_room()
                || self.round_began.get().unwrap().elapsed().as_millis()
                    >= self.policy.max_millis_per_round as u128
            {
                break;
            }
            *nodes += 1;
            let path_directory = self.path_directory(&path)?;
            let next = path.rsplit_once('/').map(|(parent, _)| parent.to_owned());
            match fs::symlink_metadata(&path_directory) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => record.prune_path = next,
                Err(e) => return Err(e.to_string()),
                Ok(_) => {
                    // A populated ancestor cannot be removed. Observe it under
                    // a shared lease so sibling publishers can keep using it.
                    // This only abandons pruning; it grants no removal right.
                    let Some(observation) =
                        DirectoryLease::acquire_local(&path_directory, false, None)?
                    else {
                        break;
                    };
                    let occupied = self.path_metadata_occupied(&path_directory, &path)?;
                    self.stable(&observation)?;
                    drop(observation);
                    if occupied {
                        record.prune_path = None;
                        break;
                    }
                    let Some(path_gate) =
                        DirectoryLease::acquire_local(&path_directory, true, None)?
                    else {
                        break;
                    };
                    // The shared observation may already be stale. Recheck the
                    // actual mapping and members under the original exclusion.
                    let Some(pending) =
                        self.detach_empty_path(&path_directory, &path, &path_gate, nodes)?
                    else {
                        break;
                    };
                    drop(path_gate);
                    // The pending name now owns any work left by this pass or
                    // an interruption, even after the object cursor is retired.
                    self.finish_metadata_retirement(&pending, nodes)?;
                    record.prune_path = next;
                }
            }
        }
        if record.prune_path.is_none() && *nodes + 3 <= self.policy.max_nodes_per_round {
            self.remove_metadata_directory(&directory, &gate, &["record.json", "record.pending"])?;
            *nodes += 3;
            retired.push(id.to_owned());
        } else if record.prune_path != initial_prune_path
            || record.metadata_expires != previous_deadline
        {
            // Missing path declarations are recoverable from the old cursor:
            // every resumed step checks its current identity, occupancy and
            // exclusion again. Persist once when this bounded pass pauses;
            // completed retirement needs no intermediate record rewrite.
            self.write_metadata(&directory.join("record.json"), &record)?;
        }
        Ok(())
    }
    fn prune_role(
        &self,
        directory: &Path,
        observed: u64,
        nodes: &mut usize,
    ) -> Result<bool, String> {
        if !self.metadata_step_available(*nodes) {
            return Ok(false);
        }
        // A protected observation still consumes a visit. When a mutation fits,
        // charge the actual step instead so a one-node round can make progress.
        let before = *nodes;
        let result = if directory
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("retiring-"))
        {
            self.finish_metadata_retirement(directory, nodes)
        } else {
            self.prune_registered_role(directory, observed, nodes)
        };
        if *nodes == before {
            *nodes += 1;
        }
        result
    }

    fn prune_registered_role(
        &self,
        directory: &Path,
        observed: u64,
        nodes: &mut usize,
    ) -> Result<bool, String> {
        let Some(gate) = DirectoryLease::acquire_local(directory, true, None)? else {
            return Ok(false);
        };
        let record_path = no_symlink_parents(directory, "record.json")?;
        let initial: Role = match self.read_metadata(&record_path)? {
            Some(record) => record,
            None => {
                let pending_path = no_symlink_parents(directory, "record.pending")?;
                let pending: Role = self
                    .read_metadata(&pending_path)?
                    .ok_or("unattached consumer birth; preserve metadata")?;
                self.validate_role(&pending.role, directory, &pending)?;
                // Only the initial declaration can precede record.json. A
                // replacement of an existing role leaves the old record in
                // place until rename, so nonempty residue is unresolved.
                if !pending.ids.is_empty()
                    || !pending.attached.is_empty()
                    || !pending.summaries.is_empty()
                    || pending.observation.is_some()
                {
                    return Err(
                        "consumer pending record is not an empty birth; preserve metadata".into(),
                    );
                }
                if !self.metadata_step_available(*nodes) {
                    return Ok(false);
                }
                self.stable(&gate)?;
                // Preserve the exact original bytes and deadline. The role's
                // original exclusion remains held through publication/pruning.
                fs::rename(&pending_path, &record_path).map_err(|error| error.to_string())?;
                *nodes += 1;
                if !self.metadata_step_available(*nodes) {
                    return Ok(false);
                }
                pending
            }
        };
        self.validate_role(&initial.role, directory, &initial)?;
        if !initial.ids.is_empty() || !initial.attached.is_empty() {
            return Ok(false);
        }
        let mut role = initial;
        let deadline =
            self.metadata_directory_deadline(directory, role.metadata_expires, observed)?;
        if deadline < role.metadata_expires {
            role.metadata_expires = deadline;
            if deadline > observed {
                self.stable(&gate)?;
                self.write_metadata(&record_path, &role)?;
                *nodes += 1;
            }
        }
        if role.metadata_expires > observed || !self.metadata_step_available(*nodes) {
            return Ok(false);
        }
        // A published record is authoritative over an interrupted replacement.
        // Check every name before removing even that unused partial file.
        let mut pending_id = None;
        for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if !entry.file_type().map_err(|e| e.to_string())?.is_file() {
                return Err("unknown metadata member; preserve directory".into());
            }
            if entry.file_name() == "record.pending" {
                pending_id = Some(identity(&entry.path())?);
            } else if entry.file_name() != "record.json" {
                return Err("unknown metadata member; preserve directory".into());
            }
        }
        if let Some(id) = pending_id {
            crate::artifact_disposal::dispose_entry(directory, "record.pending", &id, || {
                self.stable(&gate)
            })?;
            *nodes += 1;
        }
        if !self.metadata_step_available(*nodes) {
            return Ok(false);
        }
        let marker_id = identity(&record_path)?;
        let pending = directory.with_file_name(format!(
            "retiring-{}_{}_{}",
            sha256(role.role.as_bytes()),
            role.directory_identity,
            marker_id
        ));
        self.stable(&gate)?;
        crate::artifact_disposal::publish_entry(directory, &pending)?;
        *nodes += 1;
        drop(gate);
        if !self.metadata_step_available(*nodes) {
            return Ok(false);
        }
        self.finish_metadata_retirement(&pending, nodes)
    }
    fn prune_roles(&self, observed: u64, nodes: &mut usize) -> Result<Vec<Value>, String> {
        let mut errors = Vec::new();
        for entry in fs::read_dir(self.directory.join("roles")).map_err(|e| e.to_string())? {
            if !self.metadata_step_available(*nodes) {
                break;
            }
            let directory = entry.map_err(|e| e.to_string())?.path();
            if let Err(error) = self.prune_role(&directory, observed, nodes) {
                errors.push(json!({"scope":"consumer-metadata","path":directory,"reason":error}));
            }
        }
        Ok(errors)
    }

    fn metadata_round(
        &self,
        action: impl FnOnce(&mut usize) -> Result<bool, String>,
    ) -> Result<Value, String> {
        self.with_round_budget(|| {
            let mut nodes = 0;
            let attempt = action(&mut nodes);
            let mut round = json!({"nodes":nodes,"metadata_bytes":
                self.policy.max_metadata_bytes_per_round - self.round_metadata.get().unwrap_or(0),
                "verified_bytes":0,"removed_logical_bytes":0,
                "disposed":[],"protected":[],"accounting_errors":[]});
            match attempt {
                Ok(true) => round["disposed"] = json!([{"kind":"metadata","original":"verified-absent"}]),
                Ok(false) => round["protected"] = json!([{"kind":"metadata","reason":"preserved by existing owner guards or round budget"}]),
                Err(error) => round["accounting_errors"] = json!([{"reason":error,"completion":"not-established"}]),
            }
            Ok(round)
        })
    }

    /// One streamed explicit pass. Each declared address gets one existing
    /// bounded round, without a global cursor, admission lock or retry loop.
    pub fn maintain_explicit(&self) -> Result<Value, String> {
        self.maintain_explicit_reporting_at(now()?, None)
    }
    /// Stream every declared member and its unabridged original round to the
    /// actual maintain report. The compact sweep's sample ceiling is unchanged.
    pub fn maintain_explicit_observed(
        &self,
        mut observer: impl FnMut(&Value) -> Result<(), String>,
    ) -> Result<Value, String> {
        self.maintain_explicit_reporting_at(now()?, Some(&mut observer))
    }

    // The explicit sweep already visits every declared object. Quota inputs
    // must cover that same namespace, rather than repeating a truncated global
    // scan for each producer. Only compact immutable fields survive each read;
    // each read and expiry update uses the existing single-address budget.
    fn explicit_terminal_quotas(
        &self,
        observed: u64,
        observer: &mut Option<&mut dyn FnMut(&Value) -> Result<(), String>>,
    ) -> Result<Value, String> {
        let mut groups: BTreeMap<String, Vec<(String, String, u64, u64)>> = self
            .policy
            .terminal_limits
            .keys()
            .map(|producer| (producer.clone(), Vec::new()))
            .collect();
        if groups.is_empty() {
            return Ok(json!({}));
        }
        let mut errors = Vec::new();
        match fs::read_dir(self.directory.join("objects")) {
            Err(error) => errors.push(json!({"error":error.to_string()})),
            Ok(entries) => {
                for entry in entries {
                    let candidate = (|| {
                        let entry = entry.map_err(|error| error.to_string())?;
                        let id = entry
                            .file_name()
                            .into_string()
                            .map_err(|_| "quota address is not UTF8")?;
                        self.with_round_budget(|| {
                            let gate = self.addressed_gate_scope(
                                std::slice::from_ref(&id),
                                &[],
                                &[],
                                None,
                                false,
                                false,
                                true,
                            )?;
                            let ledger = self.load(&gate)?;
                            self.stable(&gate)?;
                            Ok(ledger
                                .outputs
                                .iter()
                                .find(|output| output.id == id)
                                .filter(|output| {
                                    crate::verified_release::terminal_eligible(
                                        output.phase == "released",
                                        output.manifest.sealed,
                                    )
                                })
                                .map(|output| {
                                    (
                                        output.producer.clone(),
                                        (
                                            output.id.clone(),
                                            output.identity.clone(),
                                            output.expires,
                                            output.manifest.bytes,
                                        ),
                                    )
                                }))
                        })
                    })();
                    match candidate {
                        Ok(Some((producer, candidate))) => {
                            if let Some(group) = groups.get_mut(&producer) {
                                group.push(candidate);
                            }
                        }
                        Ok(None) => (),
                        Err(error) => errors.push(json!({"error":error})),
                    }
                }
            }
        }
        let mut quotas = serde_json::Map::new();
        for (producer, mut candidates) in groups {
            candidates.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| b.0.cmp(&a.0)));
            let limits = &self.policy.terminal_limits[&producer];
            let mut producer_errors = errors.clone();
            let mut count = 0;
            let mut bytes = 0;
            let mut expired = Vec::new();
            let mut expired_count = 0u64;
            for (id, identity, expires, length) in candidates {
                let keep = crate::verified_release::terminal_budget(
                    count,
                    bytes,
                    length,
                    limits.max_count,
                    limits.max_bytes,
                );
                let decision = if !errors.is_empty() {
                    "unknown-scope-preserved"
                } else if keep {
                    count += 1;
                    bytes += length;
                    "within-quota"
                } else {
                    match self
                        .with_round_budget(|| self.expire_terminal_output(&id, &identity, observed))
                    {
                        Ok(true) => {
                            expired_count += 1;
                            if expired.len() < self.policy.max_nodes_per_round {
                                expired.push(id.clone());
                            }
                            "expired"
                        }
                        Ok(false) => {
                            producer_errors
                                .push(json!({"id":id,"error":"quota member changed; preserve it"}));
                            "changed-preserved"
                        }
                        Err(error) => {
                            producer_errors.push(json!({"id":id,"error":error}));
                            "failed-preserved"
                        }
                    }
                };
                if let Some(observer) = observer.as_deref_mut() {
                    observer(&json!({"kind":"quota-member","producer":producer,"id":id,
                        "identity":identity,"observed_expires":expires,"logical_bytes":length,
                        "decision":decision,"disposal":"not-claimed"}))?;
                }
            }
            quotas.insert(
                producer,
                json!({"scope_complete":producer_errors.is_empty(),
                "errors":producer_errors,"expired":expired,"expired_count":expired_count,
                "expired_list_complete":expired_count <= self.policy.max_nodes_per_round as u64}),
            );
        }
        Ok(Value::Object(quotas))
    }

    fn maintain_explicit_reporting_at(
        &self,
        observed: u64,
        mut observer: Option<&mut dyn FnMut(&Value) -> Result<(), String>>,
    ) -> Result<Value, String> {
        let began = Instant::now();
        let mut sweep = SweepReport::new(&self.policy);
        // Observe all declaration namespaces before any round can recreate a
        // missing sibling namespace. Keep the original IO error, not a later
        // empty directory manufactured by an ordinary owner operation.
        let declarations = ["transaction-gates", "objects", "roles"]
            .map(|namespace| (namespace, fs::read_dir(self.directory.join(namespace))));
        sweep.value["terminal_quotas"] = self.explicit_terminal_quotas(observed, &mut observer)?;
        for (namespace, entries) in declarations {
            sweep.value["enumeration"][namespace] = json!({"entries":0,"exhausted":false});
            let entries = match entries {
                Ok(entries) => entries,
                Err(error) => {
                    if let Some(observer) = observer.as_deref_mut() {
                        observer(&json!({"kind":"enumeration-error","namespace":namespace,
                            "error":error.to_string()}))?;
                    }
                    sweep.observation(
                        namespace,
                        None,
                        "accounting_errors",
                        json!({"reason":error.to_string()}),
                    )?;
                    continue;
                }
            };
            let mut exhausted = true;
            for entry in entries {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(error) => {
                        if let Some(observer) = observer.as_deref_mut() {
                            observer(&json!({"kind":"enumeration-error","namespace":namespace,
                                "error":error.to_string()}))?;
                        }
                        sweep.observation(
                            namespace,
                            None,
                            "accounting_errors",
                            json!({"reason":error.to_string()}),
                        )?;
                        exhausted = false;
                        break;
                    }
                };
                SweepReport::add(&mut sweep.value["enumeration"][namespace]["entries"], 1)?;
                let address = match entry.file_name().into_string() {
                    Ok(address) => address,
                    Err(_) => {
                        if let Some(observer) = observer.as_deref_mut() {
                            observer(&json!({"kind":"enumeration-error","namespace":namespace,
                                "error":"declaration address is not UTF8; preserve it"}))?;
                        }
                        sweep.observation(
                            namespace,
                            None,
                            "accounting_errors",
                            json!({"reason":"declaration address is not UTF8; preserve it"}),
                        )?;
                        continue;
                    }
                };
                // An earlier component in this sweep can retire a related
                // declaration which ReadDir has already buffered. Absence is
                // an observation, not permission to reconstruct its metadata
                // or a failure of the later component's original program.
                if matches!(fs::symlink_metadata(entry.path()),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound)
                {
                    let observation = json!({"reason":"declaration disappeared after enumeration; no effects attempted"});
                    if let Some(observer) = observer.as_deref_mut() {
                        observer(
                            &json!({"kind":"declaration-unavailable","namespace":namespace,
                            "address":address,"availability":"absent","action":"not-attempted"}),
                        )?;
                    }
                    sweep.observation(namespace, Some(&address), "protected", observation)?;
                    continue;
                }
                if let Some(observer) = observer.as_deref_mut() {
                    let mut declaration = json!({"kind":"declaration","namespace":namespace,
                        "address":address,"observed_unix_seconds":observed,
                        "policy_sha256":sha256(&self.policy_bytes),"owner_root":self.root,
                        "qualification":"owned by retention; reporting grants no disposal permission"});
                    declaration["state"] = json!("see original bounded round");
                    observer(&declaration)?;
                }
                let mut round = match namespace {
                    "transaction-gates" => self.metadata_round(|nodes| {
                        *nodes = 1;
                        self.retire_transaction(&address)
                    }),
                    // This address came from the declaration namespace, not
                    // an explicit caller selection. Let the same nonblocking
                    // reader refuse non-files, links and oversized metadata.
                    "objects" => {
                        if observer.is_some() {
                            self.with_round_budget(|| {
                                self.partition_round_inventory(
                                    observed,
                                    Some(std::slice::from_ref(&address)),
                                    true,
                                )
                            })
                        } else {
                            self.maintain_round(observed, Some(std::slice::from_ref(&address)))
                        }
                    }
                    _ => {
                        self.metadata_round(|nodes| self.prune_role(&entry.path(), observed, nodes))
                    }
                };
                if let Some(observer) = observer.as_deref_mut() {
                    if let Ok(round) = &mut round {
                        if let Some(declarations) = round
                            .as_object_mut()
                            .unwrap()
                            .remove("inventory_declarations")
                        {
                            for record in declarations.as_array().unwrap() {
                                observer(
                                    &json!({"kind":"declaration-state","namespace":namespace,
                                    "address":record["id"],"source_round":address,"record":record,
                                    "freshness":"state observed before original round; path measured after it"}),
                                )?;
                            }
                        }
                    }
                    observer(&match &round {
                        Ok(round) => json!({"kind":"round","namespace":namespace,
                            "address":address,"original":round}),
                        Err(error) => json!({"kind":"round","namespace":namespace,
                            "address":address,"error":error,"action":"not-established"}),
                    })?;
                }
                sweep.round(namespace, &address, round)?;
            }
            sweep.value["enumeration"][namespace]["exhausted"] = json!(exhausted);
        }
        self.sweep_path_metadata(&mut sweep, observed, &mut observer)?;
        // Iterators are not a fixed snapshot, and related components may overlap.
        // Never sum their retained inventories or infer a global remaining count.
        sweep.value["elapsed_millis"] = json!(u64::try_from(began.elapsed().as_millis()).ok());
        Ok(sweep.value)
    }

    fn inventory_declaration(record: &Object, directory: &Path) -> Value {
        let output = record.output.as_ref().map(|o| {
            let allocated = o
                .manifest
                .entries
                .iter()
                .try_fold(0u64, |sum, e| sum.checked_add(e.allocated_bytes?));
            json!({"id":o.id,"producer":o.producer,"identity":o.identity,
                "phase":o.phase,"tree":o.tree,"expires":o.expires,"lifetime":o.lifetime,
                "outcome":o.outcome,"observed_return":o.observed_return,
                "references":o.references,"removed_logical_bytes":o.removed_logical_bytes,
                "sealed_measurement":{"sealed":o.manifest.sealed,
                    "logical_bytes":if o.manifest.sealed {Some(o.manifest.bytes)} else {None},
                    "allocated_bytes":if o.manifest.sealed {allocated} else {None},
                    "unit":"bytes","freshness":"historical-manifest; observation time unknown",
                    "scope":"manifest entries; may overlap other declarations"}})
        });
        json!({"id":record.id,"path":record.path,"output":output,
            "metadata_path":directory.join("record.json"),
            "incoming":record.incoming,"roles":record.roles,
            "retired_capabilities":record.retired_leases,"metadata_expires":record.metadata_expires})
    }
    pub(super) fn removal_gate(&self, paths: &[String]) -> Result<Gate, String> {
        let (leases, directories) = self.path_gates(paths, true, PathAccess::Transfer)?;
        let targets = paths
            .iter()
            .map(|path| Ok((path.clone(), self.path_directory(path)?)))
            .collect::<Result<BTreeMap<_, _>, String>>()?;
        let mut ids = self.path_ids(&directories, false)?;
        ids.extend(self.path_ids(&targets, true)?);
        if !ids.is_empty() {
            return Err("final-original publication overlaps fixture effect; retain until its owner retires it".into());
        }
        Ok(Gate {
            leases,
            objects: RefCell::new(BTreeMap::new()),
            roles: RefCell::new(BTreeMap::new()),
            paths: directories,
            exclusive_paths: paths.iter().cloned().collect(),
            maintenance: false,
            partial_membership: false,
            retiring_report: None,
            retained_dependencies: BTreeSet::new(),
            maintenance_targets: BTreeSet::new(),
            unvisited_relations: BTreeSet::new(),
        })
    }
    pub(super) fn expire_archived_slot_address(
        &self,
        gate: &Gate,
        slot: &str,
        deadline: u64,
    ) -> Result<(), String> {
        let directory = gate
            .paths
            .get(slot)
            .filter(|_| gate.exclusive_paths.contains(slot))
            .ok_or("archived slot requires its exclusive path lease")?;
        let marker = directory.join("address.json");
        let mut address: Address = self
            .read_metadata(&marker)?
            .ok_or("archived slot address is missing")?;
        if address.path != slot || address.output.is_some() {
            return Err("archived slot address changed".into());
        }
        let expires = super::bounded_expiry(address.metadata_expires.unwrap_or(deadline), deadline);
        if address.metadata_expires != Some(expires) {
            self.stable(gate)?;
            address.metadata_expires = Some(expires);
            self.write_metadata(&marker, &address)?;
        }
        Ok(())
    }
    pub(super) fn partition_ids(&self) -> Result<Vec<String>, String> {
        let mut ids = Vec::new();
        for entry in fs::read_dir(self.directory.join("objects")).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let id = entry
                .file_name()
                .into_string()
                .map_err(|_| "output registration address UTF8")?;
            self.object_directory(&id)?;
            ids.push(id);
        }
        Ok(ids)
    }
    pub(super) fn partition_round(
        &self,
        observed: u64,
        originals: Option<&[String]>,
    ) -> Result<Value, String> {
        self.partition_round_inventory(observed, originals, false)
    }
    fn partition_round_inventory(
        &self,
        observed: u64,
        originals: Option<&[String]>,
        report_inventory: bool,
    ) -> Result<Value, String> {
        let began = self
            .round_began
            .get()
            .ok_or("retention round has no owner deadline")?;
        let mut result = json!({"schema":"chrono-output-retention-round/v2","nodes":0,
            "metadata_inventory_entries":0,"metadata_graph_edges":0,"accounting_errors":[],
            "metadata_read_bound":MAX_STATE,"metadata_bytes":0,
            "max_metadata_bytes_per_round":self.policy.max_metadata_bytes_per_round,
            "directory_snapshot_entries_bound":self.policy.max_manifest_nodes,
            "removed_logical_bytes":0,"verified_bytes":0,"physical_bytes_reclaimed":null,
            "remaining_outputs":0,"remaining_capability_retirements":0,
            "known_retained_file_logical_bytes":0,"known_retained_allocated_bytes":0,
            "unmeasured_allocated_outputs":0,"unmeasured_tree_outputs":0,
            "reserved_logical_bytes":0,"overflow_outputs":0,
            "max_output_bytes":self.policy.max_output_bytes,"related_output_capacity":self.policy.max_related_outputs,
            "protected":[],"disposed":[],"scope":{"kind":"visited-components","complete":false},
            "legacy_unenrolled":"not-scanned; owner migration required","completed_work":"not-claimed"});
        let mut nodes = 0usize;
        if report_inventory {
            result["inventory_declarations"] = json!([]);
        }
        let mut complete = true;
        let global = crate::verified_release::global_maintenance(originals);
        if let Some(originals) = originals {
            result["scope"]["kind"] = json!("declared-components");
            result["scope"]["originals"] = json!(originals);
        }
        for key in if !global {
            Vec::new()
        } else {
            self.transaction_keys()?
        } {
            if nodes >= self.policy.max_nodes_per_round
                || !self.metadata_room()
                || began.elapsed().as_millis() >= self.policy.max_millis_per_round as u128
            {
                complete = false;
                break;
            }
            nodes += 1;
            if let Err(error) = self.retire_transaction(&key) {
                result["accounting_errors"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"transaction":key,"reason":error}));
            }
        }
        let mut seen = BTreeSet::new();
        let mut processed = BTreeSet::new();
        let mut unvisited_relations = BTreeSet::new();
        let mut measurements = BTreeMap::new();
        let mut retired_object_metadata = Vec::new();
        let addresses: Box<dyn Iterator<Item = Result<String, String>> + '_> = match originals {
            Some(originals) => Box::new(originals.iter().cloned().map(Ok)),
            None => Box::new(
                fs::read_dir(self.directory.join("objects"))
                    .map_err(|e| e.to_string())?
                    .map(|entry| {
                        entry
                            .map_err(|e| e.to_string())?
                            .file_name()
                            .into_string()
                            .map_err(|_| "output registration address UTF8".into())
                    }),
            ),
        };
        // The explicit sweep starts one round per declared address. Admission
        // covers its lookup and the first eligible independent member together.
        let single_address = matches!(originals, Some([_]));
        for id in addresses {
            let id = id?;
            if processed.contains(&id) {
                continue;
            }
            if nodes >= self.policy.max_nodes_per_round
                || !self.metadata_room()
                || !crate::verified_release::retirement_time_available(
                    single_address,
                    began.elapsed().as_millis(),
                    self.policy.max_millis_per_round,
                )
            {
                complete = false;
                break;
            }
            self.round_node_limit
                .set(Some(self.policy.max_nodes_per_round - nodes));
            self.round_verified_left.set(Some(
                self.policy
                    .max_bytes_per_round
                    .saturating_sub(result["verified_bytes"].as_u64().unwrap()),
            ));
            let attempted = (|| {
                let gate = self.addressed_gate(&[id.clone()], &[], &[], None, true, false)?;
                let owned: Vec<_> = gate.objects.borrow().keys().cloned().collect();
                // Reuse exactly the records already loaded by this bounded
                // component. No second metadata read, graph or round budget.
                let declarations: Vec<_> = if report_inventory {
                    gate.objects
                        .borrow()
                        .values()
                        .map(|record| {
                            Self::inventory_declaration(
                                record,
                                &self.directory.join("objects").join(&record.id),
                            )
                        })
                        .collect()
                } else {
                    vec![]
                };
                seen.extend(owned.iter().cloned());
                // 保留的叶原件只被观察，本轮尚未维护它自己的输出。
                processed.extend(
                    owned
                        .iter()
                        .filter(|id| !gate.retained_dependencies.contains(*id))
                        .cloned(),
                );
                let independent = single_address
                    && owned.len() == 1
                    && gate.objects.borrow().get(&id).is_some_and(|record| {
                        record.roles.is_empty()
                            && record.incoming.is_empty()
                            && record.output.as_ref().is_some_and(|output| {
                                output.references.is_empty()
                                    && output.manifest.sealed
                                    && !output.overflow_disposal
                                    && output.expires <= observed
                                    && matches!(output.phase.as_str(), "released" | "disposing")
                            })
                    });
                self.maintain_locked(observed, &gate, independent)
                    .map(|round| {
                        let observations = gate.measurements(self.policy.max_output_bytes);
                        (
                            round,
                            owned,
                            observations,
                            gate.unvisited_relations.clone(),
                            declarations,
                        )
                    })
            })();
            match attempted {
                Ok((round, owned, observations, unvisited, declarations)) => {
                    if report_inventory {
                        result["inventory_declarations"]
                            .as_array_mut()
                            .unwrap()
                            .extend(declarations);
                    }
                    measurements.extend(observations);
                    unvisited_relations.extend(unvisited);
                    nodes += round["nodes"].as_u64().ok_or("component round nodes")? as usize;
                    for key in [
                        "metadata_inventory_entries",
                        "metadata_graph_edges",
                        "removed_logical_bytes",
                        "verified_bytes",
                    ] {
                        result[key] = json!(
                            result[key].as_u64().unwrap()
                                + round[key].as_u64().ok_or("component round measurement")?
                        );
                    }
                    for key in ["accounting_errors", "protected", "disposed"] {
                        result[key].as_array_mut().unwrap().extend(
                            round[key]
                                .as_array()
                                .ok_or("component round observations")?
                                .iter()
                                .cloned(),
                        );
                    }
                    for id in owned {
                        if let Err(error) = self.prune_object(
                            &id,
                            observed,
                            &mut nodes,
                            &mut retired_object_metadata,
                        ) {
                            result["accounting_errors"]
                                .as_array_mut()
                                .unwrap()
                                .push(json!({"id":id,"reason":error}));
                        }
                    }
                }
                Err(error) => {
                    result["protected"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({"id":id,"reason":error}));
                    result["accounting_errors"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({"id":id,"reason":error}));
                    // A refused component has no established remaining count.
                    complete = false;
                }
            }
        }
        match if !global {
            Ok(Vec::new())
        } else {
            self.prune_roles(observed, &mut nodes)
        } {
            Ok(errors) => result["accounting_errors"]
                .as_array_mut()
                .unwrap()
                .extend(errors),
            Err(error) => result["accounting_errors"]
                .as_array_mut()
                .unwrap()
                .push(json!({"scope":"consumer-metadata","reason":error})),
        }
        self.round_verified_left.set(None);
        for values in measurements.values() {
            for (key, value) in RETAINED_MEASUREMENT_FIELDS.iter().zip(values) {
                result[*key] = json!(result[*key].as_u64().unwrap() + value);
            }
        }
        if !retired_object_metadata.is_empty() {
            result["retired_object_metadata"] = json!(retired_object_metadata);
        }
        complete &= unvisited_relations.iter().all(|id| seen.contains(id));
        result["nodes"] = json!(nodes);
        result["metadata_bytes"] = json!(
            self.policy.max_metadata_bytes_per_round - self.round_metadata.get().unwrap_or(0)
        );
        result["scope"]["complete"] = json!(complete);
        if !complete {
            result["remaining_outputs"] = Value::Null;
        }
        Ok(result)
    }
    pub(super) fn object_directory(&self, id: &str) -> Result<PathBuf, String> {
        if !id.starts_with("output-")
            || id.len() > 128
            || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err("invalid retained output address".into());
        }
        no_symlink_parents(&self.directory, &format!("objects/{id}"))
    }
    fn role_directory(&self, role: &str) -> Result<PathBuf, String> {
        if role.is_empty() || role.len() > 256 {
            return Err("invalid retention role".into());
        }
        no_symlink_parents(
            &self.directory,
            &format!("roles/{}", sha256(role.as_bytes())),
        )
    }
    fn read_metadata<T: for<'a> Deserialize<'a>>(&self, path: &Path) -> Result<Option<T>, String> {
        let mut options = fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            // Refuse links and avoid blocking on a FIFO before fstat can reject it.
            options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        let file = match options.open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        if !metadata.is_file() {
            return Err("retention declaration must be a physical regular file".into());
        }
        // O_NOFOLLOW already rejects a linked leaf on Unix. Validate the opened
        // file once; callers have checked/excluded its declaration directory.
        #[cfg(not(unix))]
        if fs::symlink_metadata(path)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err("retention declaration must be a physical regular file".into());
        }
        if metadata.len() > MAX_STATE {
            return Err("retention declaration byte bound".into());
        }
        // Take's default read_to_end otherwise probes and grows an empty buffer
        // through several small reads. The observed length is only a capacity
        // hint: retain the actual-read bound in case the file grows meanwhile.
        let mut raw = Vec::with_capacity(metadata.len() as usize);
        file.take(MAX_STATE + 1)
            .read_to_end(&mut raw)
            .map_err(|e| e.to_string())?;
        if raw.len() as u64 > MAX_STATE {
            return Err("retention declaration byte bound".into());
        }
        self.charge_metadata(raw.len() as u64)?;
        decode(&raw).map(Some)
    }
    fn persist_change<T: Serialize>(
        &self,
        path: &Path,
        value: &T,
        persisted: &mut BTreeMap<PathBuf, Vec<u8>>,
    ) -> Result<(), String> {
        let raw = serde_json::to_vec(value).map_err(|e| e.to_string())?;
        if persisted.get(path) == Some(&raw) {
            return Ok(());
        }
        if raw.len() as u64 > MAX_STATE {
            return Err("retention declaration byte bound".into());
        }
        self.charge_metadata(raw.len() as u64)?;
        write(path, &raw)?;
        persisted.insert(path.into(), raw);
        Ok(())
    }
    fn write_metadata<T: Serialize>(&self, path: &Path, value: &T) -> Result<(), String> {
        let raw = serde_json::to_vec(value).map_err(|e| e.to_string())?;
        if raw.len() as u64 > MAX_STATE {
            return Err("retention declaration byte bound".into());
        }
        self.charge_metadata(raw.len() as u64)?;
        write(path, &raw)
    }
    fn read_object(&self, id: &str, directory: &Path) -> Result<Object, String> {
        let record: Object = self
            .read_metadata(&directory.join("record.json"))?
            .ok_or_else(|| format!("retained output unavailable: {id}"))?;
        if record.schema != "chrono-retained-output/v2"
            || record.id != id
            || record.directory_identity != identity(directory)?
            || record
                .output
                .as_ref()
                .is_some_and(|o| o.id != id || o.path != record.path)
            || record.incoming.len() > self.policy.max_related_outputs
            || record.roles.len() > self.policy.max_related_outputs
            || record.summaries.len() > self.policy.max_summaries
        {
            return Err(format!("retained output declaration changed: {id}"));
        }
        crate::relative_path(&record.path)?;
        Ok(record)
    }
    fn read_role(&self, role: &str) -> Result<Option<Role>, String> {
        let directory = self.role_directory(role)?;
        let record: Option<Role> = self.read_metadata(&directory.join("record.json"))?;
        if let Some(record) = &record {
            self.validate_role(role, &directory, record)?;
        }
        Ok(record)
    }
    fn validate_role(&self, role: &str, directory: &Path, r: &Role) -> Result<(), String> {
        if self.role_directory(role)? != directory
            || r.schema != "chrono-retained-role/v2"
            || r.role != role
            || r.ids.len() > self.policy.max_related_outputs
            || r.summaries.len() > self.policy.max_summaries
            || r.observation.as_ref().is_some_and(|o| {
                !verified::observation_order(
                    o.epoch.len(),
                    o.issued,
                    o.accepted,
                    o.current.is_some(),
                    o.current.as_ref().is_some_and(|id| r.ids.contains(id)),
                )
            })
            || identity(directory).as_ref() != Ok(&r.directory_identity)
        {
            return Err("retained consumer declaration changed".into());
        }
        Ok(())
    }
    fn path_directory(&self, path: &str) -> Result<PathBuf, String> {
        crate::relative_path(path)?;
        let mut directory = no_symlink_parents(&self.directory, "paths")?;
        for component in path.split('/') {
            directory = directory.join(sha256(component.as_bytes()));
        }
        Ok(directory)
    }
    // Ancestors are shared and the actual target is exclusive. Sibling outputs
    // never acquire an exclusive common ancestor. A tree effect excludes exactly
    // its declared descendants, including new concurrent registrations.
    fn path_gates(
        &self,
        paths: &[String],
        wait: bool,
        access: PathAccess,
    ) -> Result<(Vec<DirectoryLease>, BTreeMap<String, PathBuf>), String> {
        let mut requests = BTreeMap::<String, bool>::new();
        for path in paths {
            crate::relative_path(path)?;
            let parts: Vec<_> = path.split('/').collect();
            for n in 1..=parts.len() {
                let prefix = parts[..n].join("/");
                let exclusive = n == parts.len();
                *requests.entry(prefix).or_default() |= exclusive;
            }
        }
        let mut leases = Vec::new();
        let mut directories = BTreeMap::new();
        let began = Instant::now();
        for (path, exclusive) in requests {
            // If an ancestor is already exclusive, a second flock of one of its
            // descendants adds no exclusion and could reverse another lock order.
            let directory = self.path_directory(&path)?;
            if access == PathAccess::Lookup {
                // A missing prefix is a point-in-time miss. A concurrent birth
                // may be observed by the next lookup. Never follow a descendant
                // of a prefix this lookup did not actually lock.
                if path
                    .rsplit_once('/')
                    .is_some_and(|(parent, _)| !directories.contains_key(parent))
                    || match fs::symlink_metadata(&directory) {
                        Ok(_) => false,
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
                        Err(error) => return Err(error.to_string()),
                    }
                {
                    continue;
                }
            } else {
                fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
            }
            let marker = directory.join("address.json");
            // Maintenance already holds the actual output declarations. It
            // needs each target's mapping, plus physical ancestor exclusion.
            // Ordinary publication also consumes ancestor mappings to detect
            // overlapping reservations and therefore keeps reading them all.
            let read_address = access != PathAccess::Maintain || exclusive;
            // Birth needs only an absence observation here. Decode and validate
            // an existing declaration once, under the acquired path lease.
            let missing = access != PathAccess::Lookup
                && read_address
                && match fs::symlink_metadata(&marker) {
                    Ok(_) => false,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
                    Err(error) => return Err(error.to_string()),
                };
            if missing {
                let initial = DirectoryLease::acquire_local(
                    &directory,
                    true,
                    wait.then(|| Duration::from_secs(5).saturating_sub(began.elapsed())),
                )?
                .ok_or("retention path birth is busy")?;
                if self.read_metadata::<Address>(&marker)?.is_none() {
                    self.write_metadata(
                        &marker,
                        &Address {
                            path: path.clone(),
                            output: None,
                            metadata_expires: Some(self.metadata_creation_expiry()?),
                        },
                    )?;
                }
                initial.stable()?;
            }
            // External removal belongs to another owner and may explicitly
            // hand its capability to a native child. Metadata calls are local.
            let acquire = if access == PathAccess::Transfer {
                DirectoryLease::acquire
            } else {
                DirectoryLease::acquire_local
            };
            let lease = acquire(
                &directory,
                exclusive,
                wait.then(|| Duration::from_secs(5).saturating_sub(began.elapsed())),
            )?
            .ok_or_else(|| format!("retention path is busy: {path}"))?;
            if read_address {
                let address = self
                    .read_metadata::<Address>(&marker)?
                    .ok_or("retention path declaration disappeared")?;
                if address.path != path {
                    return Err("retention path address changed".into());
                }
            }
            directories.insert(path, directory);
            leases.push(lease);
        }
        Ok((leases, directories))
    }
    fn path_ids(
        &self,
        paths: &BTreeMap<String, PathBuf>,
        descendants: bool,
    ) -> Result<BTreeSet<String>, String> {
        let mut ids = BTreeSet::new();
        let mut pending: Vec<_> = paths.values().cloned().collect();
        let mut seen = BTreeSet::new();
        while let Some(directory) = pending.pop() {
            if !seen.insert(directory.clone()) {
                continue;
            }
            if let Some(address) =
                self.read_metadata::<Address>(&no_symlink_parents(&directory, "address.json")?)?
            {
                if self.path_directory(&address.path)? != directory {
                    return Err("retention path declaration address changed".into());
                }
                if let Some(id) = address.output {
                    ids.insert(id);
                }
            }
            if descendants {
                for entry in fs::read_dir(&directory).map_err(|e| e.to_string())? {
                    let entry = entry.map_err(|e| e.to_string())?;
                    if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
                        pending.push(entry.path());
                    }
                }
            }
        }
        Ok(ids)
    }
    pub(super) fn addressed_gate(
        &self,
        ids: &[String],
        paths: &[String],
        roles: &[String],
        birth: Option<(&str, &str)>,
        maintenance: bool,
        wait: bool,
    ) -> Result<Gate, String> {
        self.addressed_gate_scope(ids, paths, roles, birth, maintenance, wait, false)
    }
    /// Reading or updating one member's own fields preserves its durable
    /// dependencies; it does not need directory descriptors for their closure.
    pub(super) fn member_gate(&self, ids: &[String], paths: &[String]) -> Result<Gate, String> {
        self.addressed_gate_scope(ids, paths, &[], None, false, true, true)
    }
    pub(super) fn addressed_gate_scope(
        &self,
        ids: &[String],
        paths: &[String],
        roles: &[String],
        birth: Option<(&str, &str)>,
        maintenance: bool,
        wait: bool,
        membership_only: bool,
    ) -> Result<Gate, String> {
        let discovery =
            verified::gate_discovery(membership_only, maintenance, wait, birth.is_some());
        let discover_under_lease = discovery.object_excluded;
        let access = if birth.is_some() {
            PathAccess::Reserve
        } else {
            PathAccess::Lookup
        };
        let (mut leases, mut path_dirs) = self.path_gates(paths, wait, access)?;
        let mut exclusive_paths: BTreeSet<String> = paths
            .iter()
            .filter(|path| path_dirs.contains_key(*path))
            .cloned()
            .collect();
        let mut selected: BTreeSet<String> = ids.iter().cloned().collect();
        let occupied = self.path_ids(&path_dirs, false)?;
        if birth.is_some() && !occupied.is_empty() {
            return Err(
                "overlapping/already enrolled output or interrupted path reservation".into(),
            );
        }
        selected.extend(occupied);
        if maintenance || birth.is_some() {
            let targets = paths
                .iter()
                .map(|path| Ok((path.clone(), self.path_directory(path)?)))
                .collect::<Result<BTreeMap<_, _>, String>>()?;
            let descendants = self.path_ids(&targets, true)?;
            if birth.is_some() && !descendants.is_empty() {
                return Err(
                    "overlapping/already enrolled descendant or interrupted path reservation"
                        .into(),
                );
            }
            selected.extend(descendants);
        }
        let birth = birth.map(|(id, path)| (id.to_owned(), path.to_owned()));
        if let Some((id, _)) = &birth {
            selected.insert(id.clone());
        }
        let mut observed = BTreeMap::<String, Object>::new();
        let mut observed_roles = BTreeMap::<String, Option<Role>>::new();
        let mut wanted_roles: BTreeSet<String> = roles.iter().cloned().collect();
        let initial = (discover_under_lease && selected.len() == 1)
            .then(|| selected.iter().next().unwrap().clone());
        let mut deferred = verified::DeferredOriginal::new();
        let mut report_frontier = false;
        let mut retiring_report = None;
        let mut retained_dependencies = BTreeSet::new();
        let mut maintenance_targets = BTreeSet::new();
        let mut pending_dependencies = BTreeSet::new();
        // Nonblocking maintenance can hold each declaration while discovering
        // its edges. A busy metadata or path lease refuses the activity, so this
        // order never waits behind an ordinary publisher holding path locks.
        // Observation membership holds its known role before discovering its
        // members. Other waiting activities recheck their discovered closure
        // under the same path/role/object exclusion order below.
        loop {
            if selected.union(&retained_dependencies).count() > self.policy.max_related_outputs {
                return Err("related retention graph capacity; originals preserved".into());
            }
            let next_role = wanted_roles
                .iter()
                .find(|role| !observed_roles.contains_key(*role))
                .cloned();
            if let Some(role) = next_role {
                if discovery.role_excluded {
                    let directory = self.role_directory(&role)?;
                    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
                    leases.push(
                        DirectoryLease::acquire_local(
                            &directory,
                            true,
                            wait.then(|| Duration::from_secs(5)),
                        )?
                        .ok_or("retention consumer metadata is busy")?,
                    );
                }
                let record = self.read_role(&role)?;
                // Maintenance needs the consumer's membership of the actual
                // reference component, not every independent output sharing
                // that consumer. Keep its full declaration under exclusion;
                // load/save below project only the visited members.
                if !maintenance {
                    if let Some(record) = &record {
                        selected.extend(record.ids.iter().cloned());
                        selected.extend(record.attached.iter().cloned());
                    }
                }
                observed_roles.insert(role, record);
                continue;
            }
            if let Some(id) = deferred.take() {
                let record = observed
                    .get(&id)
                    .ok_or("deferred declaration was not read")?;
                let protected = verified::current_roles_protect(
                    &record
                        .roles
                        .iter()
                        .map(|role| {
                            observed_roles
                                .get(role)
                                .map(|current| current.as_ref().map(|r| r.ids.contains(&id)))
                        })
                        .collect::<Vec<_>>(),
                );
                let output = record.output.as_ref().unwrap();
                match verified::original_expansion(
                    protected,
                    finalized(record) || archived_interrupted_report(record, output),
                    initial.as_deref(),
                    &id,
                    record.incoming.len(),
                    record.roles.len(),
                ) {
                    verified::FinalizedExpansion::RetainOnly => {
                        retained_dependencies.insert(id.clone());
                        if !finalized(record) {
                            maintenance_targets.insert(id.clone());
                        }
                    }
                    verified::FinalizedExpansion::RootFrontier => {
                        // The source and its current roots stay excluded. Check
                        // every direct original one at a time; the durable edge
                        // cannot change while this source is held. Their full
                        // width must not become simultaneous descriptors.
                        retained_dependencies.insert(id.clone());
                        let mut related: BTreeSet<_> =
                            selected.union(&retained_dependencies).cloned().collect();
                        related.extend(output.references.iter().cloned());
                        if related.len() > self.policy.max_related_outputs {
                            return Err(
                                "related retention graph capacity; originals preserved".into()
                            );
                        }
                        let policy_sha256 = sha256(&self.policy_bytes);
                        for target in &output.references {
                            if observed.contains_key(target) {
                                continue;
                            }
                            let directory = self.object_directory(target)?;
                            let _original = DirectoryLease::acquire_local(&directory, true, None)?
                                .ok_or_else(|| {
                                    format!("retention output metadata is busy: {target}")
                                })?;
                            let original = self.read_object(target, &directory)?;
                            if original.policy_sha256 != policy_sha256 {
                                return Err("retention policy migration required; original record preserved".into());
                            }
                        }
                        retained_dependencies.extend(output.references.iter().cloned());
                    }
                    verified::FinalizedExpansion::RetireFrontier => {
                        // Retire this consumer before releasing its originals.
                        // The source keeps all other edges durably.
                        retiring_report = Some(id.clone());
                        // 从叶进入时，反向路径上的原件已先于父报告被读到。
                        // 将它们与后续读到的原件一起保留，只退役选中的报告。
                        for previous in observed.keys() {
                            if preserve_frontier_original(previous.as_str(), id.as_str()) {
                                retained_dependencies.insert(previous.clone());
                            }
                        }
                        selected.extend(output.references.first().cloned());
                        report_frontier = true;
                    }
                    verified::FinalizedExpansion::Related => {
                        // Find current consumers before opening this original's
                        // inputs. An upstream root may preserve this whole
                        // boundary without holding its width simultaneously.
                        selected.extend(record.incoming.iter().cloned());
                        pending_dependencies.insert(id);
                    }
                }
                continue;
            }
            let next = selected
                .iter()
                .find(|id| {
                    !observed.contains_key(*id) && !birth.as_ref().is_some_and(|b| b.0 == **id)
                })
                .cloned();
            let Some(id) = next else {
                if let Some(id) = pending_dependencies.pop_first() {
                    if !retained_dependencies.contains(&id) {
                        let record = observed
                            .get(&id)
                            .ok_or("pending dependency declaration was not read")?;
                        selected.extend(record.output.as_ref().unwrap().references.iter().cloned());
                    }
                    continue;
                }
                break;
            };
            let directory = self.object_directory(&id)?;
            if discover_under_lease {
                leases.push(
                    DirectoryLease::acquire_local(&directory, true, None)?
                        .ok_or_else(|| format!("retention output metadata is busy: {id}"))?,
                );
            }
            let record = self.read_object(&id, &directory)?;
            if verified::preserve_discovered_original(
                discovery.retain_member,
                retained_dependencies.contains(&id),
                report_frontier,
            ) {
                // Observation members and actual root dependencies keep their
                // edges. A retiring consumer also preserves its originals until
                // their individual edges can be durably detached.
                retained_dependencies.insert(id.clone());
                observed.insert(id, record);
                continue;
            }
            if verified::defer_declared_original(discover_under_lease, record.output.is_some()) {
                wanted_roles.extend(record.roles.iter().cloned());
                observed.insert(id.clone(), record);
                deferred.defer(id);
                continue;
            }
            if let Some(output) = &record.output {
                selected.extend(output.references.iter().cloned());
            }
            if maintenance {
                selected.extend(record.incoming.iter().cloned());
                wanted_roles.extend(record.roles.iter().cloned());
            }
            observed.insert(id, record);
        }
        if maintenance {
            let target_paths: Vec<_> = observed
                .values()
                .filter(|r| {
                    verified::maintenance_path_required(
                        r.output.is_some(),
                        initial.as_ref() == Some(&r.id),
                        retained_dependencies.contains(&r.id),
                        retiring_report.is_some(),
                    ) && !r.output.as_ref().is_some_and(|output| {
                        metadata_only_retirement(
                            retiring_report.as_ref() == Some(&r.id),
                            output.phase == "disposing",
                            output.manifest.sealed,
                            output.manifest.entries.is_empty(),
                            !output.references.is_empty(),
                        )
                    })
                })
                .map(|r| r.path.clone())
                .collect();
            let access = if discover_under_lease {
                PathAccess::Maintain
            } else {
                PathAccess::Reserve
            };
            let (additional, dirs) = self.path_gates(&target_paths, wait, access)?;
            leases.extend(additional);
            path_dirs.extend(dirs);
            exclusive_paths.extend(target_paths);
            // Only the final metadata component can retire the report path.
            // Earlier components keep the original object/edge exclusion;
            // repeatedly locking an unused physical path consumes their budget.
            // When all remaining edges are loaded, retain the available path
            // now so their detachment and final retirement share one component.
            for record in observed.values().filter(|record| {
                record.output.as_ref().is_some_and(|output| {
                    metadata_path_ready(
                        metadata_only_retirement(
                            retiring_report.as_ref() == Some(&record.id),
                            output.phase == "disposing",
                            output.manifest.sealed,
                            output.manifest.entries.is_empty(),
                            !output.references.is_empty(),
                        ),
                        &output.references,
                        &retained_dependencies,
                    )
                })
            }) {
                match self.path_gates(&[record.path.clone()], false, PathAccess::Maintain) {
                    Ok((additional, dirs)) => {
                        leases.extend(additional);
                        path_dirs.extend(dirs);
                        exclusive_paths.insert(record.path.clone());
                    }
                    Err(error)
                        if error.starts_with("retention path is busy: ")
                            || error == "retention path birth is busy" => {}
                    Err(error) => return Err(error),
                }
            }
        }
        let mut role_records = BTreeMap::new();
        for (role, expected) in observed_roles {
            let directory = self.role_directory(&role)?;
            let actual = if discovery.role_excluded {
                expected
            } else {
                fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
                leases.push(
                    DirectoryLease::acquire_local(
                        &directory,
                        true,
                        wait.then(|| Duration::from_secs(5)),
                    )?
                    .ok_or("retention consumer metadata is busy")?,
                );
                let actual = self.read_role(&role)?;
                if actual
                    .as_ref()
                    .map(|r| (&r.ids, &r.attached, &r.observation))
                    != expected
                        .as_ref()
                        .map(|r| (&r.ids, &r.attached, &r.observation))
                {
                    return Err(
                        "addressed consumer changed during exclusion; retry this activity".into(),
                    );
                }
                actual
            };
            let fresh = actual.is_none();
            let record = actual.unwrap_or(Role {
                observation: None,
                schema: "chrono-retained-role/v2".into(),
                directory_identity: identity(&directory)?,
                role: role.clone(),
                ids: vec![],
                attached: BTreeSet::new(),
                metadata_expires: self.metadata_creation_expiry()?,
                summaries: vec![],
            });
            if fresh {
                self.write_metadata(&directory.join("record.json"), &record)?;
            }
            role_records.insert(role, record);
        }
        let mut records = BTreeMap::new();
        for id in selected {
            if discover_under_lease {
                records.insert(
                    id.clone(),
                    observed.remove(&id).ok_or("excluded output was not read")?,
                );
                continue;
            }
            let directory = self.object_directory(&id)?;
            let fresh = birth.as_ref().is_some_and(|b| b.0 == id);
            if fresh {
                fs::create_dir(&directory).map_err(|e| format!("retention output birth: {e}"))?;
            }
            if !discover_under_lease {
                leases.push(
                    DirectoryLease::acquire_local(
                        &directory,
                        true,
                        wait.then(|| Duration::from_secs(5)),
                    )?
                    .ok_or_else(|| format!("retention output metadata is busy: {id}"))?,
                );
            }
            let record = if let Some((_, path)) = birth.as_ref().filter(|b| b.0 == id) {
                Object {
                    schema: "chrono-retained-output/v2".into(),
                    directory_identity: identity(&directory)?,
                    policy_sha256: sha256(&self.policy_bytes),
                    id: id.clone(),
                    path: path.clone(),
                    output: None,
                    incoming: BTreeSet::new(),
                    roles: BTreeSet::new(),
                    summaries: vec![],
                    retired_leases: vec![],
                    cursor: 0,
                    metadata_expires: self.metadata_creation_expiry()?,
                    prune_path: Some(path.clone()),
                }
            } else {
                self.read_object(&id, &directory)?
            };
            if fresh {
                self.write_metadata(&directory.join("record.json"), &record)?;
            }
            records.insert(id, record);
        }
        for record in records.values() {
            if !retained_dependencies.contains(&record.id)
                && ((retiring_report.as_ref() != Some(&record.id)
                    && record
                        .output
                        .as_ref()
                        .is_some_and(|o| o.references.iter().any(|id| !records.contains_key(id))))
                    || (maintenance
                        && (record.incoming.iter().any(|id| !records.contains_key(id))
                            || record
                                .roles
                                .iter()
                                .any(|role| !role_records.contains_key(role)))))
            {
                return Err(
                    "addressed retention graph changed during exclusion; retry this activity"
                        .into(),
                );
            }
        }
        let unvisited_relations = records
            .values()
            .filter(|record| {
                retained_dependencies.contains(&record.id)
                    || retiring_report.as_ref() == Some(&record.id)
            })
            .flat_map(|record| {
                record
                    .incoming
                    .iter()
                    .chain(record.output.iter().flat_map(|o| &o.references))
                    .filter(|related| !records.contains_key(*related))
                    .cloned()
            })
            .collect();
        Ok(Gate {
            leases,
            objects: RefCell::new(records),
            roles: RefCell::new(role_records),
            paths: path_dirs,
            exclusive_paths,
            maintenance,
            partial_membership: maintenance,
            retiring_report,
            retained_dependencies,
            maintenance_targets,
            unvisited_relations,
        })
    }
    pub(super) fn partition_load(
        &self,
        gate: &Gate,
        policy_digest: &str,
    ) -> Result<Ledger, String> {
        gate.stable()?;
        let objects = gate.objects.borrow();
        let roles = gate.roles.borrow();
        let mut ledger = Ledger {
            policy_sha256: policy_digest.into(),
            ..Ledger::default()
        };
        for record in objects.values() {
            // Ordinary reads pass the current digest. Explicit migration may
            // also encounter current records from an independent publication
            // or an interrupted migration; an unknown third binding still fails.
            if record.policy_sha256 != policy_digest
                && record.policy_sha256 != sha256(&self.policy_bytes)
            {
                return Err(
                    "retention policy migration required; original record preserved".into(),
                );
            }
            if let Some(output) = &record.output {
                ledger.outputs.push(output.clone());
            }
            ledger.summaries.extend(record.summaries.clone());
            ledger.retired_leases.extend(record.retired_leases.clone());
            // Frontier originals remain protected during retirement. Observation
            // members also keep their outputs and edges intact; their explicit
            // role below owns the only permitted membership transition.
            if gate.retained_dependencies.contains(&record.id) {
                continue;
            }
            for role in &record.roles {
                let loaded;
                let current = if let Some(current) = roles.get(role) {
                    Some(current)
                } else {
                    loaded = self.read_role(role)?;
                    loaded.as_ref()
                };
                match current {
                    Some(current) if current.ids.contains(&record.id) => {
                        ledger
                            .roots
                            .entry(role.clone())
                            .or_default()
                            .push(record.id.clone());
                    }
                    None => {
                        ledger
                            .roots
                            .entry(format!("interrupted-role:{}", sha256(role.as_bytes())))
                            .or_default()
                            .push(record.id.clone());
                    }
                    _ => (),
                }
            }
        }
        for role in roles.values() {
            ledger.summaries.extend(role.summaries.clone());
            let ids: Vec<_> = role
                .ids
                .iter()
                .filter(|id| !gate.partial_membership || objects.contains_key(*id))
                .cloned()
                .collect();
            if ids.is_empty() {
                ledger.roots.remove(&role.role);
            } else {
                ledger.roots.insert(role.role.clone(), ids);
            }
        }
        if gate.maintenance {
            ledger.cursor = objects
                .values()
                .find(|o| !gate.retained_dependencies.contains(&o.id))
                .map_or(0, |o| o.cursor);
        }
        Ok(ledger)
    }
    pub(super) fn partition_rebind_policy(
        &self,
        gate: &Gate,
        lifetimes: &BTreeMap<String, Option<super::Lifetime>>,
    ) -> Result<(), String> {
        self.stable(gate)?;
        let digest = sha256(&self.policy_bytes);
        for record in gate.objects.borrow_mut().values_mut() {
            let mut changed = record.policy_sha256 != digest;
            if let Some(output) = record.output.as_mut() {
                let lifetime = lifetimes
                    .get(&output.id)
                    .ok_or("migration original lifetime missing")?
                    .clone();
                changed |= output.lifetime != lifetime;
                output.lifetime = lifetime;
                let expires = output.bound_expiry(output.expires);
                changed |= output.expires != expires;
                output.expires = expires;
                let metadata_expires = output.bound_expiry(record.metadata_expires);
                changed |= record.metadata_expires != metadata_expires;
                record.metadata_expires = metadata_expires;
            }
            if changed {
                record.policy_sha256 = digest.clone();
                self.write_metadata(
                    &self.object_directory(&record.id)?.join("record.json"),
                    record,
                )?;
            }
        }
        Ok(())
    }
    pub(super) fn partition_save(&self, ledger: &Ledger, gate: &Gate) -> Result<(), String> {
        self.partition_save_observation(ledger, gate, None)
    }
    fn partition_save_observation(
        &self,
        ledger: &Ledger,
        gate: &Gate,
        observation: Option<(&str, ObservationOrder)>,
    ) -> Result<(), String> {
        gate.stable()?;
        let mut objects = gate.objects.borrow_mut();
        let mut roles = gate.roles.borrow_mut();
        // Only output presence and outbound references are read from the old state.
        let before: BTreeMap<_, _> = objects
            .iter()
            .map(|(id, record)| {
                (
                    id.clone(),
                    record
                        .output
                        .as_ref()
                        .map(|output| output.references.clone()),
                )
            })
            .collect();
        // partition_load reads the first addressed declaration's cursor. Keep
        // that single owner rather than synchronously rewriting every related
        // original whenever bounded disposal advances the component cursor.
        let cursor_owner = gate
            .maintenance
            .then(|| {
                objects
                    .keys()
                    .find(|id| !gate.retained_dependencies.contains(*id))
                    .cloned()
            })
            .flatten();
        // Exact semantic snapshots already read under these directory leases.
        // Only a real durable transition needs another atomic write/fsync.
        let mut persisted = BTreeMap::new();
        for record in objects.values() {
            persisted.insert(
                self.object_directory(&record.id)?.join("record.json"),
                serde_json::to_vec(record).map_err(|e| e.to_string())?,
            );
        }
        for role in roles.values() {
            persisted.insert(
                self.role_directory(&role.role)?.join("record.json"),
                serde_json::to_vec(role).map_err(|e| e.to_string())?,
            );
        }

        for role in roles.values() {
            if !verified::observation_edit(
                role.observation.is_some(),
                observation
                    .as_ref()
                    .is_some_and(|(name, _)| *name == role.role),
                ledger.roots.get(&role.role).into_iter().flatten().eq(role
                    .ids
                    .iter()
                    .filter(|id| !gate.maintenance || objects.contains_key(*id))),
            ) {
                return Err("observation membership requires its domain transition".into());
            }
        }
        // Publish the path's reservation before any output/source attachment.
        // An interrupted birth remains discoverable by the next same-path user.
        for output in &ledger.outputs {
            if before.get(&output.id).is_some_and(Option::is_none) {
                let directory = gate
                    .paths
                    .get(&output.path)
                    .ok_or("publication requires addressed path exclusion")?;
                let marker = directory.join("address.json");
                let mut address = self
                    .read_metadata::<Address>(&marker)?
                    .ok_or("path declaration missing")?;
                if address.path != output.path {
                    return Err("path declaration address changed".into());
                }
                address.output = Some(output.id.clone());
                self.write_metadata(&marker, &address)?;
            }
        }
        for role in roles.values_mut() {
            role.attached.extend(role.ids.iter().cloned());
            role.attached
                .extend(ledger.roots.get(&role.role).into_iter().flatten().cloned());
            self.persist_change(
                &self.role_directory(&role.role)?.join("record.json"),
                role,
                &mut persisted,
            )?;
        }
        // Install all conservative inbound protection before publishing a source
        // edge or consumer-root change. An interruption can retain extra work;
        // it cannot expose a still-referenced original to disposal.
        for output in &ledger.outputs {
            let previous = before.get(&output.id).and_then(Option::as_ref);
            for target in output
                .references
                .iter()
                .filter(|id| previous.is_none_or(|references| !references.contains(id)))
            {
                let record = objects
                    .get_mut(target)
                    .ok_or("dependency target was not excluded")?;
                if record
                    .output
                    .as_ref()
                    .is_none_or(|o| o.phase == "disposing")
                {
                    return Err("dependency target unavailable/disposing".into());
                }
                record.incoming.insert(output.id.clone());
                self.persist_change(
                    &self.object_directory(target)?.join("record.json"),
                    record,
                    &mut persisted,
                )?;
            }
        }
        for role in roles.values() {
            for id in ledger.roots.get(&role.role).into_iter().flatten() {
                let record = objects
                    .get_mut(id)
                    .ok_or("consumer target was not excluded")?;
                if !ledger
                    .outputs
                    .iter()
                    .any(|o| o.id == *id && o.phase != "disposing")
                {
                    return Err("consumer target unavailable/disposing".into());
                }
                // The object pass below saves this backreference together with
                // the output, before publishing the role's authoritative ids.
                record.roles.insert(role.role.clone());
            }
        }
        for (id, record) in objects.iter_mut() {
            record.policy_sha256 = ledger.policy_sha256.clone();
            if let Some(output) = record
                .output
                .as_ref()
                .filter(|_| !ledger.outputs.iter().any(|o| o.id == *id))
            {
                let keep = self
                    .policy
                    .terminal_limits
                    .get(&output.producer)
                    .map_or(self.policy.metadata_keep_seconds, |limits| {
                        limits.metadata_keep_seconds
                    });
                let requested = now()?.saturating_add(keep);
                record.metadata_expires = if output.lifetime.is_some() {
                    super::bounded_expiry(output.bound_expiry(requested), record.metadata_expires)
                } else {
                    requested
                };
                record.prune_path = Some(record.path.clone());
            }
            record.output = ledger.outputs.iter().find(|o| o.id == *id).cloned();
            if let Some(output) = &record.output {
                // An adopted file can predate its metadata declaration.
                record.metadata_expires = output.bound_expiry(record.metadata_expires);
            }
            record.retired_leases = ledger
                .retired_leases
                .iter()
                .filter(|n| n.path == *id)
                .cloned()
                .collect();
            record.summaries = ledger
                .summaries
                .iter()
                .filter(|s| s["id"] == *id || s["report"] == record.path)
                .cloned()
                .collect();
            let excess = record
                .summaries
                .len()
                .saturating_sub(self.policy.max_summaries);
            record.summaries.drain(..excess);
            if cursor_owner.as_ref() == Some(id) {
                record.cursor = ledger.cursor;
            }
            self.persist_change(
                &self.object_directory(id)?.join("record.json"),
                record,
                &mut persisted,
            )?;
            if record.output.is_none()
                && (!self.directory.join(id).exists() || !record.retired_leases.is_empty())
            {
                if let Some(directory) = gate.paths.get(&record.path) {
                    let marker = directory.join("address.json");
                    let mut address = self
                        .read_metadata::<Address>(&marker)?
                        .ok_or("path declaration missing")?;
                    if address.output.as_deref() == Some(id.as_str()) {
                        address.output = None;
                        self.write_metadata(&marker, &address)?;
                    }
                }
            }
        }

        for role in roles.values_mut() {
            let mut next = ledger.roots.get(&role.role).cloned().unwrap_or_default();
            if gate.partial_membership {
                // The ledger contains only this reference component. Preserve
                // every unvisited member and the original membership order.
                let selected = next;
                next = role
                    .ids
                    .iter()
                    .filter(|id| !objects.contains_key(*id) || selected.contains(*id))
                    .cloned()
                    .collect();
                for id in selected {
                    if !next.contains(&id) {
                        next.push(id);
                    }
                }
            }
            if role.ids != next && self.policy.max_lifetime_seconds.is_none() {
                role.metadata_expires = now()?.saturating_add(self.policy.metadata_keep_seconds);
            }
            role.ids = next;
            // Sequence and roots change in the same atomic role replacement,
            // after the persisted-byte snapshot and conservative backreferences.
            if let Some((name, order)) = &observation {
                if *name == role.role {
                    role.observation = Some(order.clone());
                }
            }
            role.summaries = ledger
                .summaries
                .iter()
                .filter(|s| {
                    s["consumer"]
                        .as_str()
                        .is_some_and(|c| role.role == format!("consumer:{c}"))
                })
                .cloned()
                .collect();
            self.persist_change(
                &self.role_directory(&role.role)?.join("record.json"),
                role,
                &mut persisted,
            )?;
        }
        // Prune only after the new source declarations are durable.
        for record in objects.values_mut() {
            record.incoming.retain(|source| {
                !before.contains_key(source)
                    || objects_reference(&ledger.outputs, source, &record.id, |output| {
                        (output.id.as_str(), output.references.as_slice())
                    })
            });
            for role in roles.values() {
                if !role.ids.contains(&record.id) {
                    record.roles.remove(&role.role);
                }
            }
            self.persist_change(
                &self.object_directory(&record.id)?.join("record.json"),
                record,
                &mut persisted,
            )?;
        }
        for role in roles.values_mut() {
            if gate.partial_membership {
                // Unvisited attachments may be conservative protection from an
                // interrupted transition. Only reconcile the excluded members.
                role.attached.retain(|id| !objects.contains_key(id));
                role.attached.extend(role.ids.iter().cloned());
            } else {
                role.attached = role.ids.iter().cloned().collect();
            }
            self.persist_change(
                &self.role_directory(&role.role)?.join("record.json"),
                role,
                &mut persisted,
            )?;
        }
        Ok(())
    }
}
#[cfg(not(verus_keep_ghost))]
fn archived_interrupted_report(record: &Object, output: &Output) -> bool {
    if output.tree
        || !output.manifest.sealed
        || output.phase != "sealing"
        || output.outcome["publication"] != "interrupted"
        || output.outcome["completion"] != "not-established"
        || !record.retired_leases.is_empty()
        || !record.incoming.is_empty()
        || !record.roles.is_empty()
    {
        return false;
    }
    // The release receipt was written under the original and survivor checks.
    // An interrupted command still has no execution outcome.
    record.summaries.iter().any(|receipt| {
        let original = &receipt["original"];
        receipt["schema"] == "chrono-preserved-original-release/v1"
            && original["id"] == record.id
            && original["path"] == output.path
            && original["identity"] == output.identity
    })
}

#[cfg(not(verus_keep_ghost))]
fn finalized(record: &Object) -> bool {
    verified::original_finalized(
        record.retired_leases.len(),
        record
            .output
            .as_ref()
            .map(|output| (output.manifest.sealed, output.phase.as_str())),
    )
}
