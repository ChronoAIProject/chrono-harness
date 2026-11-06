//! Finite original/metadata lifetimes at the existing producer and retention owners.
use super::*;
use chrono_harness::retained_artifacts::retention::Inventory;

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Recovery {
    pub bucket: u8,
    pub after: String,
    #[serde(default)]
    pub round_started: u64,
    #[serde(default)]
    pub pending: Vec<String>,
}
impl Recovery {
    pub fn validate(&self) -> Result<(), String> {
        if self.bucket >= 16 || self.after.len() > 80 || self.after.contains(['/', '\\']) {
            return Err("invalid producer recovery address; preserve cursor".into());
        }
        if self.pending.len() > 65536 {
            return Err("recovery pending declaration capacity".into());
        }
        for name in &self.pending {
            Store::record_name(name)?;
        }
        Ok(())
    }
}
pub(super) fn metadata_name(object: &str) -> String {
    format!("producer-metadata/{}", sha256(object.as_bytes()))
}
pub(super) fn admit_metadata(
    directory: &Path,
    namespace: &Path,
    value: &Value,
) -> Result<(), String> {
    let context: Context = decode(
        &fs::read(no_symlink_parents(directory, "producer-context.json")?)
            .map_err(|e| e.to_string())?,
    )?;
    let mut count = 1;
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|e| e.to_string())?
        .len() as u64
        + 1;
    if bytes > context.capacity.max_metadata_bytes_per_round {
        return Err("fixture metadata byte capacity unmet before publication".into());
    }
    let began = std::time::Instant::now();
    for entry in fs::read_dir(namespace).map_err(|e| e.to_string())? {
        count += 1;
        if count > context.capacity.max_entries_per_checkpoint
            || began.elapsed().as_millis() >= context.capacity.max_millis_per_round as u128
        {
            return Err("fixture metadata node capacity unmet before publication".into());
        }
        bytes = bytes.saturating_add(
            entry
                .map_err(|e| e.to_string())?
                .metadata()
                .map_err(|e| e.to_string())?
                .len(),
        );
        if bytes > context.capacity.max_metadata_bytes_per_round {
            return Err("fixture metadata byte capacity unmet before publication".into());
        }
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Blocker {
    schema: String,
    producer: String,
    object: String,
    identity: String,
    reason: String,
    paths: Vec<String>,
}
impl Context {
    fn blocker_path(&self, object: &str) -> Result<PathBuf, String> {
        no_symlink_parents(
            &self.directory,
            &format!("capacity-blockers/{}.json", sha256(object.as_bytes())),
        )
    }
    pub(super) fn checkpoint(&self, allocation: &Path) -> Result<Value, String> {
        self.production_admission()?;
        self.observe_capacity(allocation, false)
    }
    pub(super) fn observe_capacity(
        &self,
        allocation: &Path,
        excluded: bool,
    ) -> Result<Value, String> {
        let name = allocation
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("allocation UTF-8")?;
        let mut store = Store::open(&self.directory, &self.producer, false)?;
        store.load(name)?;
        let object = store.state.objects[0].clone();
        if object.retiring {
            return Ok(value!({"status":"retiring","capacity":"pending actual absence"}));
        }
        store.stable(&object)?;
        drop(store);
        let usage = self.measure(allocation);
        let unreleased = excluded
            && (!object.captures.is_empty()
                || !object.shared_originals.is_empty()
                || !object.copy_intents.is_empty())
            && (object.original_release.is_none()
                || object.consumers.values().any(|c| !c.released));
        let path = self.blocker_path(name)?;
        if usage.is_err() {
            let mut paths = self
                .generated_outputs
                .iter()
                .map(|p| format!("{name}/{}", p.trim_end_matches('/')))
                .collect::<Vec<_>>();
            paths.extend(object.captures.keys().map(|p| format!("{name}/{p}")));
            paths.extend(object.metadata_directory.iter().cloned());
            let blocker = Blocker {
                schema: "chrono-fixture-capacity-obligation/v1".into(),
                producer: self.producer.clone(),
                object: name.into(),
                identity: object.identity.clone(),
                reason: usage.as_ref().err().cloned().unwrap_or_else(|| {
                    "excluded original still has an unreleased actual consumer; capacity unmet"
                        .into()
                }),
                paths,
            };
            fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
            write_json(&path, &blocker, false)?;
        } else if path.exists() {
            // The actual addressed producer has remeasured the same allocation;
            // this clears an obligation, never releases an original consumer.
            let old: Blocker = decode(&fs::read(&path).map_err(|e| e.to_string())?)?;
            if old.object != name
                || old.identity != object.identity
                || old.producer != self.producer
            {
                return Err("capacity obligation identity differs".into());
            }
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        usage.map(|mut value| {
            value["original_lifetime"] = value!(if unreleased {
                "protected-pending; no original release established"
            } else {
                "unchanged"
            });
            value
        })
    }
    pub(super) fn production_admission(&self) -> Result<(), String> {
        self.stable()?;
        let directory = no_symlink_parents(&self.directory, "capacity-blockers")?;
        let entries = match fs::read_dir(&directory) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e.to_string()),
        };
        for (index, entry) in entries.enumerate() {
            if index >= self.capacity.max_record_snapshot {
                return Err("fixture capacity obligation set exceeds read bound; production deferred, originals preserved".into());
            }
            let path = entry.map_err(|e| e.to_string())?.path();
            let original = fingerprint(&path)?;
            let blocker: Blocker = decode(&fs::read(&path).map_err(|e| e.to_string())?)?;
            if blocker.schema != "chrono-fixture-capacity-obligation/v1"
                || blocker.producer != self.producer
                || path != self.blocker_path(&blocker.object)?
            {
                return Err("capacity obligation ownership unknown; production deferred".into());
            }
            let mut store = Store::open(&self.directory, &self.producer, false)?;
            if store.contains(&blocker.object)? {
                let record = store.decode_record(&no_symlink_parents(
                    &self.directory,
                    &Store::record_name(&blocker.object)?,
                )?)?;
                if record.identity != blocker.identity {
                    return Err("capacity obligation allocation changed; preserve original".into());
                }
                // Retirement can remove the allocation before its continuation
                // record. Its remaining work no longer owns a directory gate at
                // that now-absent allocation address.
                if record.retiring {
                    drop(store);
                    self.entry(Some(&blocker.object))?;
                } else {
                    if !store.load_available(&blocker.object)? {
                        return Err(
                            "fixture capacity unmet; obligation metadata busy, production deferred"
                                .into(),
                        );
                    }
                    let object = store.state.objects[0].clone();
                    if object.identity != blocker.identity {
                        return Err(
                            "capacity obligation allocation changed; preserve original".into()
                        );
                    }
                    if !object.retiring {
                        let binding = object
                            .cache_lease
                            .as_ref()
                            .ok_or("capacity obligation lease missing")?;
                        let excluded = Lease::acquire(
                            &no_symlink_parents(&self.directory, &binding.path)?,
                            Some(&binding.id),
                            false,
                            true,
                            None,
                        )?;
                        let stale = excluded.is_some();
                        let recover = stale && object.consumers.values().all(|c| c.released);
                        drop(store);
                        drop(excluded);
                        if recover {
                            self.entry(Some(&blocker.object))?;
                        }
                        let current = Store::open(&self.directory, &self.producer, false)?;
                        if !current.contains(&blocker.object)? {
                            if path.exists() {
                                fs::remove_file(&path).map_err(|e| e.to_string())?;
                            }
                            continue;
                        }
                        let current = current.decode_record(&no_symlink_parents(
                            &self.directory,
                            &Store::record_name(&blocker.object)?,
                        )?)?;
                        if !current.retiring {
                            self.observe_capacity(&self.directory.join(&blocker.object), stale)?;
                            if !path.exists() {
                                continue;
                            }
                            return Err(format!(
                                "fixture capacity unmet; original obligation remains: {}",
                                blocker.object
                            ));
                        }
                    }
                }
            }
            // Retention's physical effect, not enrollment or a terminal wrapper,
            // settles a retired obligation. Unknown originals remain protected.
            let absent =
                blocker
                    .paths
                    .iter()
                    .try_fold(true, |absent, p| -> Result<bool, String> {
                        Ok(absent
                            && match fs::symlink_metadata(no_symlink_parents(&self.directory, p)?) {
                                Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
                                Err(e) => return Err(e.to_string()),
                                Ok(_) => false,
                            })
                    })?;
            if absent && fingerprint(&path)? == original {
                fs::remove_file(&path).map_err(|e| e.to_string())?;
            } else {
                return Err(format!(
                    "fixture capacity unmet; production refused across fresh allocation IDs; protected/unknown original preserved: {}: {}",
                    blocker.object, blocker.reason
                ));
            }
        }
        Ok(())
    }
}

impl RetainedArtifact {
    /// The responsible original consumer releases only its exact allocation.
    /// This does not assert success, exit, join, or release diagnostic references.
    pub fn release_originals(&self, reason: &str) -> Result<(), String> {
        if reason.trim().is_empty() {
            return Err("actual original consumer release reason required".into());
        }
        let mut store = Store::open(&self.directory, &self.producer, false)?;
        store.load(&self.object)?;
        if store.state.objects[0].retiring {
            return Ok(());
        }
        store.stable(&store.state.objects[0])?;
        if store.state.objects[0].cache_lease.is_none() {
            return Err("original release requires a temporary allocation".into());
        }
        let object = Arc::make_mut(&mut store.state.objects[0]);
        if object.original_release.is_none() {
            object.original_release = Some(reason.into());
        }
        store.save(0)
    }
}

// Only the native producer's explicitly created container chain is eligible.
// Any additional source, checkout or unknown member keeps the allocation root.
fn capture_only_allocation(directory: &Path, object: &Object) -> Result<bool, String> {
    if !object.evidence.is_empty() {
        return Ok(false);
    }
    let allocation = no_symlink_parents(directory, &object.path)?;
    let captures = ".chrono-harness/state/command-captures";
    let mut allowed = BTreeSet::new();
    for name in object.captures.keys() {
        let Some(tail) = name.strip_prefix(&format!("{captures}/")) else {
            return Ok(false);
        };
        if tail.is_empty() || Path::new(tail).components().count() != 1 {
            return Ok(false);
        }
        allowed.insert(tail.to_owned());
    }
    for (parent, names) in [
        ("", BTreeSet::from([".chrono-harness".to_owned()])),
        (".chrono-harness", BTreeSet::from(["state".to_owned()])),
        (
            ".chrono-harness/state",
            BTreeSet::from(["command-captures".to_owned()]),
        ),
        (captures, allowed),
    ] {
        let path = if parent.is_empty() {
            allocation.clone()
        } else {
            no_symlink_parents(&allocation, parent)?
        };
        let entries = match fs::read_dir(&path) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e.to_string()),
        };
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            if !entry.file_type().map_err(|e| e.to_string())?.is_dir()
                || !names.contains(
                    entry
                        .file_name()
                        .to_str()
                        .ok_or("capture container UTF-8")?,
                )
            {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn retained_paths(directory: &Path, object: &Object) -> Result<Vec<PathBuf>, String> {
    let whole = capture_only_allocation(directory, object)?;
    let mut paths = if whole {
        vec![no_symlink_parents(directory, &object.path)?]
    } else {
        object
            .captures
            .keys()
            .map(|p| no_symlink_parents(&directory.join(&object.path), p))
            .collect::<Result<Vec<_>, _>>()?
    };
    if !whole {
        for name in object.evidence.keys() {
            paths.push(no_symlink_parents(&directory.join(&object.path), name)?);
        }
    }
    if let Some(metadata) = &object.metadata_directory {
        paths.push(no_symlink_parents(directory, metadata)?);
    }
    paths.push(no_symlink_parents(
        directory,
        &Store::record_name(&object.path)?,
    )?);
    if let Some(lease) = &object.cache_lease {
        paths.push(no_symlink_parents(directory, &lease.path)?);
    }
    Ok(paths)
}
pub(super) fn resume_retirement(
    inputs: &PolicyInputs,
    r: &mut Runner,
    directory: &Path,
    object: &Object,
) -> Result<Value, String> {
    if object.disposable_directory {
        return dispose_owned_directory(inputs, r, directory, object, None);
    }
    let allocation = no_symlink_parents(directory, &object.path)?;
    let allocation_matches = match fs::symlink_metadata(&allocation) {
        Ok(_) => directory_id(&allocation)? == object.identity,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
        Err(e) => return Err(e.to_string()),
    };
    if !object.retiring
        || object.original_release.is_none()
        || object.consumers.values().any(|c| !c.released)
        || object
            .metadata_directory
            .as_deref()
            .is_some_and(|p| p != metadata_name(&object.path))
        || !allocation_matches
    {
        return Err(
            "retiring allocation identity, release or metadata binding differs; preserve originals"
                .into(),
        );
    }
    let inventory = Inventory::adopted(&inputs.policy.coordinator_root)?
        .ok_or("fixture retirement requires adopted output retention; capacity unmet")?;
    let record = no_symlink_parents(directory, &Store::record_name(&object.path)?)?;
    let record_relative = record
        .strip_prefix(&inputs.policy.coordinator_root)
        .map_err(|e| e.to_string())?
        .to_str()
        .ok_or("record UTF-8")?
        .to_owned();
    let role = format!("fixture-retirement:{}", sha256(object.path.as_bytes()));
    let mut paths = retained_paths(directory, object)?;
    let mut remaining = false;
    for path in paths.iter().filter(|path| *path != &record) {
        match fs::symlink_metadata(path) {
            Ok(_) => remaining = true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error.to_string()),
        }
    }
    // Keep the resumable producer record until every declared effect is done.
    // Its temporary root also protects interrupted enrollment/link publication.
    paths.sort_by_key(|path| path != &record);
    let mut selected = vec![];
    for path in paths {
        let relative = path
            .strip_prefix(&inputs.policy.coordinator_root)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("retirement UTF-8")?;
        selected.push(relative.to_owned());
        if !path.exists() {
            continue;
        }
        if inventory.enrolled(relative)? {
            if path == record && remaining {
                inventory.reference_paths(&role, &[record_relative.clone()])?;
            }
            continue;
        }
        if path == allocation {
            for (name, capture) in &object.captures {
                let member = no_symlink_parents(&allocation, name)?;
                if Some(directory_id(&member)?.as_str()) != capture["identity"].as_str() {
                    return Err(
                        "resumed capture-only original identity differs; preserve it".into(),
                    );
                }
            }
        }
        if object
            .metadata_directory
            .as_ref()
            .is_some_and(|name| path == directory.join(name))
            && object.metadata_identity.as_deref() != Some(directory_id(&path)?.as_str())
        {
            return Err(
                "resumed fixture metadata identity differs or is unknown; preserve it".into(),
            );
        }
        // Resumption may follow only partial enrollment. Validate each remaining
        // original again; a prior retiring bit does not authorize its replacement.
        if let Ok(name) = path.strip_prefix(directory.join(&object.path)) {
            let name = name.to_str().ok_or("retirement member UTF-8")?;
            if let Some(capture) = object.captures.get(name) {
                if Some(directory_id(&path)?.as_str()) != capture["identity"].as_str() {
                    return Err("resumed capture original identity differs; preserve it".into());
                }
            }
            if let Some(evidence) = object.evidence.get(name) {
                let bytes = fs::read(&path).map_err(|e| e.to_string())?;
                if *evidence != value!({"sha256":sha256(&bytes),"length":bytes.len()}) {
                    return Err("resumed evidence original differs; preserve it".into());
                }
            }
        }
        if !inventory.accepts("fixture-retirement", relative) {
            return Err("fixture retirement producer/scope not adopted; preserve originals".into());
        }
        let publication=Inventory::adopted(&inputs.policy.coordinator_root)?.unwrap().begin("fixture-retirement",relative,
            value!({"producer_outcome":"unchanged","original_consumer_release":object.original_release,"operation":"fixture-original-retirement"}))?;
        if path == record && remaining {
            inventory.reference_paths(&role, &[record_relative.clone()])?;
        }
        publication.complete(value!({"producer_outcome":"unchanged","original_consumer_release":object.original_release,"operation":"fixture-original-retirement"}))?;
    }
    for digest in object.shared_originals.keys() {
        selected.push(
            no_symlink_parents(directory, &format!("body-inputs/{digest}.bin"))?
                .strip_prefix(&inputs.policy.coordinator_root)
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("shared original UTF-8")?
                .to_owned(),
        );
    }
    for path in &selected {
        if path != &record_relative && inventory.enrolled(path)? {
            inventory.dependencies_paths(path, &[record_relative.clone()])?;
        }
    }
    // A bounded pass can remove a capture container before its metadata. Keep
    // the immutable continuation rooted until every other declared path is
    // absent; a later ordinary entry can then retire the record itself.
    let remaining = selected.iter().try_fold(false, |remaining, path| {
        if path == &record_relative {
            return Ok::<_, String>(remaining);
        }
        match fs::symlink_metadata(no_symlink_parents(&inputs.policy.coordinator_root, path)?) {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(remaining),
            Err(error) => Err(error.to_string()),
        }
    })?;
    if !remaining {
        inventory.reference(&role, &[])?;
    }
    Ok(
        value!({"status":"retiring","owner":"output-retention/v2","physical_absence":"pending","producer_outcome":"unchanged","retention_paths":selected}),
    )
}

// This producer owns the complete temporary directory. Its existing kernel
// capability and one durable retiring record protect the whole operation; no
// per-member output registrations or deletion manifest are needed.
fn dispose_owned_directory(
    inputs: &PolicyInputs,
    r: &mut Runner,
    directory: &Path,
    object: &Object,
    held: Option<&Lease>,
) -> Result<Value, String> {
    let context = Context::for_allocation(&directory.join(&object.path))?
        .ok_or("owned directory context missing; preserve allocation")?;
    if !context.disposable_directory
        || !object.retiring
        || object.original_release.is_none()
        || object.consumers.values().any(|c| !c.released)
        || object.metadata_directory.as_deref() != Some(metadata_name(&object.path).as_str())
    {
        return Err("owned directory retirement binding or consumer release differs".into());
    }
    let store = Store::open(directory, &context.producer, false)?;
    let record = no_symlink_parents(directory, &Store::record_name(&object.path)?)?;
    let original = fs::read(&record).map_err(|e| e.to_string())?;
    if value!(store.decode_record(&record)?.as_ref()) != value!(object) {
        return Err("owned directory retirement record changed".into());
    }
    let allocation = no_symlink_parents(directory, &object.path)?;
    let metadata = no_symlink_parents(directory, &metadata_name(&object.path))?;
    let binding = object
        .cache_lease
        .as_ref()
        .ok_or("owned directory lease missing")?;
    let lease_path = no_symlink_parents(directory, &binding.path)?;
    let acquired;
    let lease = if let Some(held) = held {
        Some(held)
    } else if lease_path.exists() {
        acquired = Lease::acquire(&lease_path, Some(&binding.id), false, true, None)?;
        let Some(lease) = acquired.as_ref() else {
            return Ok(value!({"status":"protected","reason":"live owned directory consumer"}));
        };
        Some(lease)
    } else if !allocation.exists() && !metadata.exists() {
        // An interrupted final unlink may leave only its continuation record.
        None
    } else {
        return Err("owned directory capability is missing; preserve remaining data".into());
    };
    let allocation_gate = if allocation.exists() {
        Some(DirectoryGate::available(&allocation)?.ok_or("owned directory metadata busy")?)
    } else {
        None
    };
    if metadata.exists() {
        let bytes = fs::read(no_symlink_parents(directory, &object.intent.path)?)
            .map_err(|e| e.to_string())?;
        let intent = json(&bytes)?;
        if sha256(&bytes) != object.intent.sha256
            || intent["disposable_directory"] != true
            || intent["identity"] != object.identity
            || intent["path"] != object.path
            || intent["producer"] != context.producer
            || intent["cache_lease"] != value!(object.cache_lease)
        {
            return Err("owned directory original intent differs; preserve allocation".into());
        }
    } else if allocation.exists() {
        return Err("owned directory original intent is missing; preserve allocation".into());
    }
    let paths = [allocation.clone(), metadata.clone()];
    let names = paths
        .iter()
        .map(|p| {
            p.strip_prefix(&inputs.policy.coordinator_root)
                .map_err(|e| e.to_string())?
                .to_str()
                .map(str::to_owned)
                .ok_or("owned directory path UTF-8".into())
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut result = value!({"automatic_cleanup":true,"artifact_disposals":[],
        "owner":"temporary-directory","producer_outcome":"unchanged"});
    artifact_disposal::dispose(
        &inputs.policy.coordinator_root,
        &names,
        &paths,
        &mut result,
        || {
            inputs.stable(r)?;
            context.stable()?;
            store.owner_stable()?;
            if fs::read(&record).map_err(|e| e.to_string())? != original {
                return Err("owned directory continuation changed before disposal".into());
            }
            if let Some(lease) = lease {
                lease.stable()?;
            }
            if allocation.exists() {
                allocation_gate
                    .as_ref()
                    .ok_or("owned directory exclusion missing")?
                    .stable()?;
                if directory_id(&allocation)? != object.identity {
                    return Err("owned directory identity changed before disposal".into());
                }
            }
            if metadata.exists()
                && Some(directory_id(&metadata)?.as_str()) != object.metadata_identity.as_deref()
            {
                return Err("owned directory metadata identity changed before disposal".into());
            }
            chrono_harness::artifact_disposal::source_safe(
                &inputs.policy.coordinator_root,
                &names[0],
            )?;
            chrono_harness::artifact_disposal::source_safe(
                &inputs.policy.coordinator_root,
                &names[1],
            )
        },
    )?;
    if let Some(lease) = lease {
        lease.stable()?;
    }
    if lease_path.exists() {
        fs::remove_file(&lease_path).map_err(|e| e.to_string())?;
    }
    if fs::read(&record).map_err(|e| e.to_string())? != original {
        return Err("owned directory continuation changed before final unlink".into());
    }
    fs::remove_file(record).map_err(|e| e.to_string())?;
    result["status"] = value!("disposed");
    result["physical_absence"] = value!("verified");
    // Shared originals have a different lifetime from this directory. Forward
    // their existing exact addresses to the content owner after releasing our
    // claims; other directories' claims still protect the same bytes.
    if !object.shared_originals.is_empty() {
        result["retention_paths"] = value!(
            object
                .shared_originals
                .keys()
                .map(|digest| {
                    directory
                        .join(format!("body-inputs/{digest}.bin"))
                        .strip_prefix(&inputs.policy.coordinator_root)
                        .map_err(|e| e.to_string())?
                        .to_str()
                        .map(str::to_owned)
                        .ok_or("shared original path UTF-8".into())
                })
                .collect::<Result<Vec<_>, String>>()?
        );
    }
    Ok(result)
}
pub(super) fn retire(
    policy: &ProducerPolicy,
    inputs: &PolicyInputs,
    r: &mut Runner,
    directory: &Path,
    object: &Object,
    lease: Option<&Lease>,
    row: &Value,
) -> Result<Value, String> {
    if object.disposable_directory && (!policy.temporary || !policy.disposable_directory) {
        return Err("whole-directory disposal is no longer adopted; preserve allocation".into());
    }
    if object.original_release.is_none() {
        return Ok(
            value!({"status":"protected","reason":"actual original consumer has not released; interruption outcome unchanged"}),
        );
    }
    if !["cleaned", "already-absent", "already-disposed"]
        .iter()
        .any(|s| row["status"] == *s)
        || (!object.bodies.is_empty() && row["fixture_bodies"]["status"] != "recovered")
    {
        return Ok(
            value!({"status":"protected","reason":"declared outputs/body custody still pending"}),
        );
    }
    let Some(inventory) = Inventory::adopted(&inputs.policy.coordinator_root)? else {
        return Err(
            "fixture original retirement not adopted; originals preserved, capacity unmet".into(),
        );
    };
    let mut store = Store::open(directory, &policy.id, false)?;
    store.load(&object.path)?;
    let current = store.state.objects[0].clone();
    store.stable(&current)?;
    if let Some(lease) = lease {
        lease.stable()?;
    }
    for path in object.copy_intents.keys().chain(object.bodies.keys()) {
        if directory.join(&object.path).join(path).exists() {
            return Ok(
                value!({"status":"protected","reason":"copied member or interrupted copy still present"}),
            );
        }
    }
    for (path, capture) in &object.captures {
        let actual = no_symlink_parents(&directory.join(&object.path), path)?;
        if directory_id(&actual)?
            != capture["identity"]
                .as_str()
                .ok_or("capture original identity unknown; explicit owner migration required")?
        {
            return Err("capture original identity differs; preserve it".into());
        }
        let relative = actual
            .strip_prefix(&inputs.policy.coordinator_root)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("capture UTF-8")?;
        if !inventory.accepts("fixture-retirement", relative) {
            return Err("capture retirement scope is not adopted".into());
        }
    }
    // Shared originals have one actual hard-link claim per allocation. The
    // content owner serializes only this digest, never unrelated allocations.
    for (digest, length) in &current.shared_originals {
        body::release_original(
            inputs,
            r,
            directory,
            &current,
            digest,
            *length,
            lease.ok_or("shared original exclusion missing")?,
        )?;
    }
    Arc::make_mut(&mut store.state.objects[0]).retiring = true;
    store.save(0)?;
    let retiring = store.state.objects[0].clone();
    drop(store);
    if retiring.disposable_directory {
        dispose_owned_directory(inputs, r, directory, &retiring, lease)
    } else {
        resume_retirement(inputs, r, directory, &retiring)
    }
}

pub(super) fn now_nanos() -> Result<u64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos()
        .try_into()
        .map_err(|_| "producer birth timestamp overflow".into())
}
