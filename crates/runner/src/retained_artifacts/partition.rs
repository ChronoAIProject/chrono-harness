//! Addressed retention metadata. The namespace contains declarations, not a
//! mutable inventory. Multi-record exclusion is limited to actual graph edges.
use super::*;
use crate::ownership::DirectoryLease;
use crate::verified_release as verified;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Object {
    schema: String,
    directory_identity: String,
    policy_sha256: String,
    id: String,
    path: String,
    output: Option<Output>,
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
pub struct ObservationToken {
    pub role: String,
    pub epoch: String,
    pub serial: u64,
    pub attempt: u64,
    pub original: OriginalIdentity,
}
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
struct ObservationOrder {
    epoch: String,
    issued: u64,
    accepted: u64,
    current: Option<String>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Address {
    path: String,
    output: Option<String>,
}
pub(super) struct Gate {
    leases: Vec<DirectoryLease>,
    objects: RefCell<BTreeMap<String, Object>>,
    roles: RefCell<BTreeMap<String, Role>>,
    paths: BTreeMap<String, PathBuf>,
    maintenance: bool,
}
impl Gate {
    pub(super) fn stable(&self) -> Result<(), String> {
        for lease in &self.leases {
            lease.stable()?;
        }
        Ok(())
    }
}
impl Inventory {
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
        let gate = self.addressed_gate(&[], &[path.into()], &[role.into()], None, false, true)?;
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
        let gate = self.addressed_gate(
            &[request.original.id.clone()],
            &[request.original.path.clone()],
            &[request.role.clone()],
            None,
            false,
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
    fn prune_object(&self, id: &str, observed: u64, nodes: &mut usize) -> Result<(), String> {
        let directory = self.object_directory(id)?;
        let Some(gate) = DirectoryLease::acquire(&directory, true, None)? else {
            return Ok(());
        };
        let mut record = self.read_object(id)?;
        if record.output.is_some()
            || !record.incoming.is_empty()
            || !record.roles.is_empty()
            || !record.retired_leases.is_empty()
            || record.metadata_expires > observed
            || self.directory.join(id).exists()
        {
            return Ok(());
        }
        while let Some(path) = record.prune_path.clone() {
            if *nodes + 4 > self.policy.max_nodes_per_round
                || !self.metadata_room()
                || self.round_began.get().unwrap().elapsed().as_millis()
                    >= self.policy.max_millis_per_round as u128
            {
                return Ok(());
            }
            *nodes += 1;
            let path_directory = self.path_directory(&path)?;
            let next = path.rsplit_once('/').map(|(parent, _)| parent.to_owned());
            match fs::symlink_metadata(&path_directory) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => record.prune_path = next,
                Err(e) => return Err(e.to_string()),
                Ok(_) => {
                    let Some(path_gate) = DirectoryLease::acquire(&path_directory, true, None)?
                    else {
                        return Ok(());
                    };
                    let address: Address = self
                        .read_metadata(&no_symlink_parents(&path_directory, "address.json")?)?
                        .ok_or("path metadata declaration missing")?;
                    if address.path != path {
                        return Err("path metadata identity differs".into());
                    }
                    let has_children = fs::read_dir(&path_directory)
                        .map_err(|e| e.to_string())?
                        .any(|entry| {
                            entry.map_or(true, |entry| {
                                entry.file_name() != "address.json"
                                    && entry.file_name() != "address.pending"
                            })
                        });
                    if address.output.is_some() || has_children {
                        record.prune_path = None;
                    } else {
                        self.remove_metadata_directory(
                            &path_directory,
                            &path_gate,
                            &["address.json", "address.pending"],
                        )?;
                        *nodes += 3;
                        record.prune_path = next;
                    }
                }
            }
            self.write_metadata(&directory.join("record.json"), &record)?;
        }
        if *nodes + 3 <= self.policy.max_nodes_per_round {
            self.remove_metadata_directory(&directory, &gate, &["record.json", "record.pending"])?;
            *nodes += 3;
        }
        Ok(())
    }
    fn prune_roles(&self, observed: u64, nodes: &mut usize) -> Result<Vec<Value>, String> {
        let mut errors = Vec::new();
        for entry in fs::read_dir(self.directory.join("roles")).map_err(|e| e.to_string())? {
            if *nodes + 4 > self.policy.max_nodes_per_round
                || !self.metadata_room()
                || self.round_began.get().unwrap().elapsed().as_millis()
                    >= self.policy.max_millis_per_round as u128
            {
                break;
            }
            *nodes += 1;
            let directory = entry.map_err(|e| e.to_string())?.path();
            let attempt = (|| {
                let Some(gate) = DirectoryLease::acquire(&directory, true, None)? else {
                    return Ok(());
                };
                let initial: Role = self
                    .read_metadata(&no_symlink_parents(&directory, "record.json")?)?
                    .ok_or("unattached consumer birth; preserve metadata")?;
                if self.role_directory(&initial.role)? != directory {
                    return Err("consumer record address changed".into());
                }
                let role = self
                    .read_role(&initial.role)?
                    .ok_or("consumer record disappeared")?;
                if role.ids.is_empty()
                    && role.attached.is_empty()
                    && role.metadata_expires <= observed
                {
                    self.remove_metadata_directory(
                        &directory,
                        &gate,
                        &["record.json", "record.pending"],
                    )?;
                    *nodes += 3;
                }
                Ok::<_, String>(())
            })();
            if let Err(error) = attempt {
                errors.push(json!({"scope":"consumer-metadata","path":directory,"reason":error}));
            }
        }
        Ok(errors)
    }
    pub(super) fn removal_gate(&self, paths: &[String]) -> Result<Gate, String> {
        let (leases, directories) = self.path_gates(paths, true)?;
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
            maintenance: false,
        })
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
        for id in addresses {
            let id = id?;
            if seen.contains(&id) {
                continue;
            }
            if nodes >= self.policy.max_nodes_per_round
                || !self.metadata_room()
                || began.elapsed().as_millis() >= self.policy.max_millis_per_round as u128
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
                seen.extend(owned.iter().cloned());
                self.maintain_locked(observed, &gate)
                    .map(|round| (round, owned))
            })();
            match attempted {
                Ok((round, owned)) => {
                    nodes += round["nodes"].as_u64().ok_or("component round nodes")? as usize;
                    for key in [
                        "metadata_inventory_entries",
                        "metadata_graph_edges",
                        "removed_logical_bytes",
                        "verified_bytes",
                        "remaining_outputs",
                        "remaining_capability_retirements",
                        "known_retained_file_logical_bytes",
                        "known_retained_allocated_bytes",
                        "unmeasured_allocated_outputs",
                        "unmeasured_tree_outputs",
                        "reserved_logical_bytes",
                        "overflow_outputs",
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
                        if let Err(error) = self.prune_object(&id, observed, &mut nodes) {
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
        let file = match fs::File::open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        if !file.metadata().map_err(|e| e.to_string())?.is_file()
            || fs::symlink_metadata(path)
                .map_err(|e| e.to_string())?
                .file_type()
                .is_symlink()
        {
            return Err("retention declaration must be a physical regular file".into());
        }
        let mut raw = Vec::new();
        file.take(MAX_STATE + 1)
            .read_to_end(&mut raw)
            .map_err(|e| e.to_string())?;
        if raw.len() as u64 > MAX_STATE {
            return Err("retention declaration byte bound".into());
        }
        self.charge_metadata(raw.len() as u64)?;
        serde_json::from_slice(&raw)
            .map(Some)
            .map_err(|error| error.to_string())
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
    fn read_object(&self, id: &str) -> Result<Object, String> {
        let directory = self.object_directory(id)?;
        let record: Object = self
            .read_metadata(&no_symlink_parents(&directory, "record.json")?)?
            .ok_or_else(|| format!("retained output unavailable: {id}"))?;
        if record.schema != "chrono-retained-output/v2"
            || record.id != id
            || record.directory_identity != identity(&directory)?
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
        let record: Option<Role> =
            self.read_metadata(&no_symlink_parents(&directory, "record.json")?)?;
        if record.as_ref().is_some_and(|r| {
            r.schema != "chrono-retained-role/v2"
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
                || identity(&directory).as_ref() != Ok(&r.directory_identity)
        }) {
            return Err("retained consumer declaration changed".into());
        }
        Ok(record)
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
        let mut directories = BTreeMap::<String, PathBuf>::new();
        let began = Instant::now();
        for (path, exclusive) in requests {
            // Lexical prefix order acquires every parent first. Reuse its held
            // address rather than hashing and resolving the full prefix again.
            let directory = match path.rsplit_once('/') {
                Some((parent, component)) => directories
                    .get(parent)
                    .ok_or("retention parent path gate is unavailable")?
                    .join(sha256(component.as_bytes())),
                None => self.path_directory(&path)?,
            };
            fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
            let marker = no_symlink_parents(&directory, "address.json")?;
            // Birth needs only an absence observation here. Decode and validate
            // an existing declaration once, under the acquired path lease.
            let missing = match fs::symlink_metadata(&marker) {
                Ok(_) => false,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
                Err(error) => return Err(error.to_string()),
            };
            if missing {
                let initial = DirectoryLease::acquire(
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
                        },
                    )?;
                }
                initial.stable()?;
            }
            let lease = DirectoryLease::acquire(
                &directory,
                exclusive,
                wait.then(|| Duration::from_secs(5).saturating_sub(began.elapsed())),
            )?
            .ok_or_else(|| format!("retention path is busy: {path}"))?;
            let address = self
                .read_metadata::<Address>(&marker)?
                .ok_or("retention path declaration disappeared")?;
            if address.path != path {
                return Err("retention path address changed".into());
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
        let discover_under_lease = maintenance && !wait && birth.is_none();
        let (mut leases, mut path_dirs) = self.path_gates(paths, wait)?;
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
        let mut observed = BTreeMap::new();
        let mut observed_roles = BTreeMap::<String, Option<Role>>::new();
        let mut wanted_roles: BTreeSet<String> = roles.iter().cloned().collect();
        // Nonblocking maintenance can hold each declaration while discovering
        // its edges. A busy metadata or path lease refuses the activity, so this
        // order never waits behind an ordinary publisher holding path locks.
        // Waiting activities retain their path/role/object order and recheck
        // the discovered closure under exclusion below.
        loop {
            let next_role = wanted_roles
                .iter()
                .find(|role| !observed_roles.contains_key(*role))
                .cloned();
            if let Some(role) = next_role {
                if discover_under_lease {
                    let directory = self.role_directory(&role)?;
                    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
                    leases.push(
                        DirectoryLease::acquire(&directory, true, None)?
                            .ok_or("retention consumer metadata is busy")?,
                    );
                }
                let record = self.read_role(&role)?;
                // Maintenance needs the consumer's membership of the actual
                // reference component, not every independent output sharing
                // that consumer. Keep its full declaration under exclusion;
                // load/save below project only the visited members.
                if let Some(record) = &record
                    && !maintenance
                {
                    selected.extend(record.ids.iter().cloned());
                    selected.extend(record.attached.iter().cloned());
                }
                observed_roles.insert(role, record);
                continue;
            }
            let next = selected
                .iter()
                .find(|id| {
                    !observed.contains_key(*id) && !birth.as_ref().is_some_and(|b| b.0 == **id)
                })
                .cloned();
            let Some(id) = next else {
                break;
            };
            if discover_under_lease {
                leases.push(
                    DirectoryLease::acquire(&self.object_directory(&id)?, true, None)?
                        .ok_or_else(|| format!("retention output metadata is busy: {id}"))?,
                );
            }
            let record = self.read_object(&id)?;
            if let Some(output) = &record.output {
                selected.extend(output.references.iter().cloned());
            }
            if maintenance {
                selected.extend(record.incoming.iter().cloned());
                wanted_roles.extend(record.roles.iter().cloned());
            }
            observed.insert(id, record);
            if selected.len() > self.policy.max_related_outputs {
                return Err("related retention graph capacity; originals preserved".into());
            }
        }
        if maintenance {
            let target_paths: Vec<_> = observed.values().map(|r| r.path.clone()).collect();
            let (additional, dirs) = self.path_gates(&target_paths, wait)?;
            leases.extend(additional);
            path_dirs.extend(dirs);
        }
        let mut role_records = BTreeMap::new();
        for (role, expected) in observed_roles {
            let directory = self.role_directory(&role)?;
            let actual = if discover_under_lease {
                expected
            } else {
                fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
                leases.push(
                    DirectoryLease::acquire(
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
                metadata_expires: now()?.saturating_add(self.policy.metadata_keep_seconds),
                summaries: vec![],
            });
            if fresh {
                self.write_metadata(&directory.join("record.json"), &record)?;
            }
            role_records.insert(role, record);
        }
        let mut records = BTreeMap::new();
        for id in selected {
            let directory = self.object_directory(&id)?;
            let fresh = birth.as_ref().is_some_and(|b| b.0 == id);
            if fresh {
                fs::create_dir(&directory).map_err(|e| format!("retention output birth: {e}"))?;
            }
            if !discover_under_lease {
                leases.push(
                    DirectoryLease::acquire(
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
                    metadata_expires: now()?.saturating_add(self.policy.metadata_keep_seconds),
                    prune_path: Some(path.clone()),
                }
            } else if discover_under_lease {
                observed.remove(&id).ok_or("excluded output was not read")?
            } else {
                self.read_object(&id)?
            };
            if fresh {
                self.write_metadata(&directory.join("record.json"), &record)?;
            }
            records.insert(id, record);
        }
        for record in records.values() {
            if record
                .output
                .as_ref()
                .is_some_and(|o| o.references.iter().any(|id| !records.contains_key(id)))
                || (maintenance
                    && (record.incoming.iter().any(|id| !records.contains_key(id))
                        || record
                            .roles
                            .iter()
                            .any(|role| !role_records.contains_key(role))))
            {
                return Err(
                    "addressed retention graph changed during exclusion; retry this activity"
                        .into(),
                );
            }
        }
        Ok(Gate {
            leases,
            objects: RefCell::new(records),
            roles: RefCell::new(role_records),
            paths: path_dirs,
            maintenance,
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
                ledger.outputs.push(output.clone().into());
            }
            ledger.summaries.extend(record.summaries.clone());
            ledger.retired_leases.extend(record.retired_leases.clone());
            for role in &record.roles {
                let current = if let Some(current) = roles.get(role) {
                    Some(current.clone())
                } else {
                    self.read_role(role)?
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
                .filter(|id| !gate.maintenance || objects.contains_key(*id))
                .cloned()
                .collect();
            if ids.is_empty() {
                ledger.roots.remove(&role.role);
            } else {
                ledger.roots.insert(role.role.clone(), ids);
            }
        }
        if gate.maintenance {
            ledger.cursor = objects.values().next().map_or(0, |o| o.cursor);
        }
        Ok(ledger)
    }
    pub(super) fn partition_rebind_policy(&self, gate: &Gate) -> Result<(), String> {
        self.stable(gate)?;
        let digest = sha256(&self.policy_bytes);
        for record in gate.objects.borrow_mut().values_mut() {
            if record.policy_sha256 != digest {
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
        let before = objects.clone();
        // partition_load reads the first addressed declaration's cursor. Keep
        // that single owner rather than synchronously rewriting every related
        // original whenever bounded disposal advances the component cursor.
        let cursor_owner = gate
            .maintenance
            .then(|| objects.keys().next().cloned())
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
                ledger.roots.get(&role.role).cloned().unwrap_or_default()
                    == role
                        .ids
                        .iter()
                        .filter(|id| !gate.maintenance || objects.contains_key(*id))
                        .cloned()
                        .collect::<Vec<_>>(),
            ) {
                return Err("observation membership requires its domain transition".into());
            }
        }
        // Publish the path's reservation before any output/source attachment.
        // An interrupted birth remains discoverable by the next same-path user.
        for output in &ledger.outputs {
            if before.get(&output.id).is_some_and(|r| r.output.is_none()) {
                let directory = gate
                    .paths
                    .get(&output.path)
                    .ok_or("publication requires addressed path exclusion")?;
                self.write_metadata(
                    &directory.join("address.json"),
                    &Address {
                        path: output.path.clone(),
                        output: Some(output.id.clone()),
                    },
                )?;
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
            let previous = before.get(&output.id).and_then(|r| r.output.as_ref());
            for target in output
                .references
                .iter()
                .filter(|id| previous.is_none_or(|o| !o.references.contains(id)))
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
                if record.roles.insert(role.role.clone()) {
                    self.persist_change(
                        &self.object_directory(id)?.join("record.json"),
                        record,
                        &mut persisted,
                    )?;
                }
            }
        }
        for (id, record) in objects.iter_mut() {
            record.policy_sha256 = ledger.policy_sha256.clone();
            if record.output.is_some() && !ledger.outputs.iter().any(|o| o.id == *id) {
                record.metadata_expires = now()?.saturating_add(self.policy.metadata_keep_seconds);
                record.prune_path = Some(record.path.clone());
            }
            record.output = ledger
                .outputs
                .iter()
                .find(|o| o.id == *id)
                .map(|o| o.value.as_ref().clone());
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
                    let address = self
                        .read_metadata::<Address>(&marker)?
                        .ok_or("path declaration missing")?;
                    if address.output.as_deref() == Some(id.as_str()) {
                        self.write_metadata(
                            &marker,
                            &Address {
                                path: record.path.clone(),
                                output: None,
                            },
                        )?;
                    }
                }
            }
        }

        for role in roles.values_mut() {
            let mut next = ledger.roots.get(&role.role).cloned().unwrap_or_default();
            if gate.maintenance {
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
            if role.ids != next {
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
                    || objects_reference(&ledger.outputs, source, &record.id)
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
            if gate.maintenance {
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
fn objects_reference(outputs: &[OutputRecord], source: &str, target: &str) -> bool {
    outputs
        .iter()
        .any(|o| o.id == source && o.references.iter().any(|r| r == target))
}
