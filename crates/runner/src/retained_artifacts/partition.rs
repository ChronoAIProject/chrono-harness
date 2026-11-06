//! Addressed retention metadata. The namespace contains declarations, not a
//! mutable inventory. Multi-record exclusion is limited to actual graph edges.
use super::*;
use crate::ownership::DirectoryLease;

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
    schema: String,
    directory_identity: String,
    role: String,
    ids: Vec<String>,
    attached: BTreeSet<String>,
    metadata_expires: u64,
    #[serde(deserialize_with = "strict_summaries")]
    summaries: Vec<Value>,
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
    /// Reclaim only an actual producer's declared retirement relationship.
    /// The normal node, byte, metadata and time bounds still apply.
    pub fn maintain_paths(&self, paths: &[String]) -> Result<Value, String> {
        if paths.is_empty() || paths.len() > self.policy.max_related_outputs {
            return Err("retirement path scope is empty or exceeds capacity".into());
        }
        self.round_metadata
            .set(Some(self.policy.max_metadata_bytes_per_round));
        self.round_began.set(Some(Instant::now()));
        self.round_node_limit
            .set(Some(self.policy.max_nodes_per_round));
        let result = (|| {
            // Address discovery grants no deletion capability. Resolve only the
            // requested paths and their ancestors, then acquire and revalidate
            // the actual relationship once, under the maintenance gate below.
            // A stale address cannot authorize a replacement's physical bytes.
            let mut directories = BTreeMap::new();
            for path in paths {
                crate::relative_path(path)?;
                let parts: Vec<_> = path.split('/').collect();
                for n in 1..=parts.len() {
                    let prefix = parts[..n].join("/");
                    directories.insert(prefix.clone(), self.path_directory(&prefix)?);
                }
            }
            let ids: Vec<_> = self.path_ids(&directories, false)?.into_iter().collect();
            let gate = self.addressed_gate(&ids, &[], &[], None, true, false)?;
            let owned: Vec<_> = gate.objects.borrow().keys().cloned().collect();
            let mut result = self.maintain_locked(now()?, &gate)?;
            drop(gate);
            let mut nodes = result["nodes"].as_u64().ok_or("retirement nodes")? as usize;
            for id in owned {
                if let Err(error) = self.prune_object(&id, now()?, &mut nodes) {
                    result["accounting_errors"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({"id":id,"reason":error}));
                }
            }
            result["nodes"] = json!(nodes);
            result["scope"] = json!({"kind":"declared-retirement-paths","paths":paths});
            Ok(result)
        })();
        self.round_metadata.set(None);
        self.round_began.set(None);
        self.round_node_limit.set(None);
        result
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
    pub(super) fn all_partition_gate(&self) -> Result<Gate, String> {
        let mut ids = Vec::new();
        let mut roles = BTreeSet::new();
        for entry in fs::read_dir(self.directory.join("objects")).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let id = entry
                .file_name()
                .into_string()
                .map_err(|_| "output registration address UTF8")?;
            let record = self.read_object(&id)?;
            roles.extend(record.roles);
            ids.push(id);
        }
        self.addressed_gate_bounded(
            &ids,
            &[],
            &roles.into_iter().collect::<Vec<_>>(),
            None,
            true,
            true,
            None,
        )
    }
    pub(super) fn partition_round(&self, observed: u64) -> Result<Value, String> {
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
        for key in self.transaction_keys()? {
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
        for entry in fs::read_dir(self.directory.join("objects")).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let id = entry
                .file_name()
                .into_string()
                .map_err(|_| "output registration address UTF8")?;
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
        match self.prune_roles(observed, &mut nodes) {
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
    fn object_directory(&self, id: &str) -> Result<PathBuf, String> {
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
        let mut directories = BTreeMap::new();
        let began = Instant::now();
        for (path, exclusive) in requests {
            // If an ancestor is already exclusive, a second flock of one of its
            // descendants adds no exclusion and could reverse another lock order.
            let directory = self.path_directory(&path)?;
            fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
            let marker = no_symlink_parents(&directory, "address.json")?;
            if self.read_metadata::<Address>(&marker)?.is_none() {
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
        self.addressed_gate_bounded(
            ids,
            paths,
            roles,
            birth,
            maintenance,
            wait,
            Some(self.policy.max_related_outputs),
        )
    }
    fn addressed_gate_bounded(
        &self,
        ids: &[String],
        paths: &[String],
        roles: &[String],
        birth: Option<(&str, &str)>,
        maintenance: bool,
        wait: bool,
        limit: Option<usize>,
    ) -> Result<Gate, String> {
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
        // Discover only declared relationships before taking metadata locks.
        // Recheck the closed set under exclusion below; drift refuses this
        // activity rather than silently inspecting an unlocked dependency.
        loop {
            let next_role = wanted_roles
                .iter()
                .find(|role| !observed_roles.contains_key(*role))
                .cloned();
            if let Some(role) = next_role {
                let record = self.read_role(&role)?;
                if let Some(record) = &record {
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
            let record = self.read_object(&id)?;
            if let Some(output) = &record.output {
                selected.extend(output.references.iter().cloned());
            }
            if maintenance {
                selected.extend(record.incoming.iter().cloned());
                wanted_roles.extend(record.roles.iter().cloned());
            }
            observed.insert(id, record);
            if limit.is_some_and(|limit| selected.len() > limit) {
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
            fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
            leases.push(
                DirectoryLease::acquire(&directory, true, wait.then(|| Duration::from_secs(5)))?
                    .ok_or("retention consumer metadata is busy")?,
            );
            let actual = self.read_role(&role)?;
            if actual.as_ref().map(|r| (&r.ids, &r.attached))
                != expected.as_ref().map(|r| (&r.ids, &r.attached))
            {
                return Err(
                    "addressed consumer changed during exclusion; retry this activity".into(),
                );
            }
            let fresh = actual.is_none();
            let record = actual.unwrap_or(Role {
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
            leases.push(
                DirectoryLease::acquire(&directory, true, wait.then(|| Duration::from_secs(5)))?
                    .ok_or_else(|| format!("retention output metadata is busy: {id}"))?,
            );
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
            if record.policy_sha256 != policy_digest {
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
            if role.ids.is_empty() {
                ledger.roots.remove(&role.role);
            } else {
                ledger.roots.insert(role.role.clone(), role.ids.clone());
            }
        }
        if gate.maintenance {
            ledger.cursor = objects.values().next().map_or(0, |o| o.cursor);
        }
        Ok(ledger)
    }
    pub(super) fn partition_save(&self, ledger: &Ledger, gate: &Gate) -> Result<(), String> {
        gate.stable()?;
        let mut objects = gate.objects.borrow_mut();
        let mut roles = gate.roles.borrow_mut();
        let before = objects.clone();
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
            if gate.maintenance {
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
            let next = ledger.roots.get(&role.role).cloned().unwrap_or_default();
            if role.ids != next {
                role.metadata_expires = now()?.saturating_add(self.policy.metadata_keep_seconds);
            }
            role.ids = next;
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
            role.attached = role.ids.iter().cloned().collect();
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
