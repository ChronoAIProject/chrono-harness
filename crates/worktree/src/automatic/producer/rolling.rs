//! 已声明的临时目录按整个分配回收；目录内的测试输入与捕获共用寿命。
use super::*;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Policy {
    success_keep_seconds: u64,
    failure_keep_seconds: u64,
    interrupted_keep_seconds: u64,
    metadata_keep_seconds: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Terminal {
    observed_at: u64,
    failed: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Directory {
    pub(super) metadata: String,
    pub(super) metadata_identity: String,
    pub(super) born_at: u64,
    pub(super) diagnostics: bool,
    pub(super) terminal: Option<Terminal>,
    pub(super) disposal_intent: Option<Receipt>,
    pub(super) disposed: bool,
    pub(super) disposed_at: Option<u64>,
}

#[derive(Clone)]
pub(super) struct Owner {
    root: PathBuf,
    config_path: String,
    producer: String,
}

pub(super) fn now() -> Result<u64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| e.to_string())
}

pub(super) fn physical_stable(directory: &Path, object: &Object) -> Result<(), String> {
    let path = no_symlink_parents(directory, &object.path)?;
    if let Some(rolling) = &object.rolling {
        if rolling.metadata != format!("rolling-metadata/{}", sha256(object.path.as_bytes()))
            || directory_id(&no_symlink_parents(directory, &rolling.metadata)?)?
                != rolling.metadata_identity
        {
            return Err("rolling directory metadata differs; preserve allocation".into());
        }
        if let Some(receipt) = &rolling.disposal_intent {
            let bytes = fs::read(no_symlink_parents(directory, &receipt.path)?)
                .map_err(|e| e.to_string())?;
            let intent = json(&bytes)?;
            if sha256(&bytes) != receipt.sha256
                || intent["schema"] != "chrono-rolling-directory-disposal/v1"
                || intent["path"] != object.path
                || intent["identity"] != object.identity
                || intent["birth"] != value!(object.intent)
                || intent["terminal"] != value!(rolling.terminal)
            {
                return Err("rolling directory disposal binding changed".into());
            }
            if matches!(fs::symlink_metadata(&path), Err(e) if e.kind() == std::io::ErrorKind::NotFound)
            {
                return Ok(());
            }
        }
        if rolling.disposed {
            return Err("disposed rolling directory has reappeared".into());
        }
    }
    if directory_id(&path)? != object.identity {
        return Err("producer artifact identity changed; preserve object".into());
    }
    Ok(())
}

impl TemporaryHost {
    /// Read the existing producer's retirement state for its private test owner.
    /// The caller must first join all allocators. This neither retires an object
    /// nor grants deletion rights over a shared store or any enclosing checkout.
    #[cfg(feature = "producer-birth-test")]
    #[doc(hidden)]
    pub fn retired_fixture_store(directory: &Path, producer: &str) -> Result<bool, String> {
        match fs::symlink_metadata(directory) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(true),
            Err(error) => return Err(error.to_string()),
            Ok(kind) if !kind.is_dir() || kind.file_type().is_symlink() => {
                return Err("fixture producer namespace is not a physical directory".into());
            }
            Ok(_) => (),
        }
        let Some(gate) = DirectoryGate::available(directory)? else {
            return Ok(false);
        };
        let store = Store::open(directory, producer, false)?;
        let (objects, errors) = store.records()?;
        if !errors.is_empty() {
            return Err("fixture producer registrations are unknown".into());
        }
        let mut metadata_empty = true;
        let mut members_known = true;
        for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            use crate::verified_lifecycle::{ProducerNamespaceMember, producer_namespace_member};
            let name = entry.file_name();
            let member = name
                .to_str()
                .map(producer_namespace_member)
                .unwrap_or(ProducerNamespaceMember::Unknown);
            match member {
                ProducerNamespaceMember::Owner => members_known &= kind.is_file(),
                ProducerNamespaceMember::Records => members_known &= kind.is_dir(),
                ProducerNamespaceMember::Metadata => {
                    if !kind.is_dir() {
                        return Ok(false);
                    }
                    metadata_empty &= fs::read_dir(entry.path())
                        .map_err(|e| e.to_string())?
                        .next()
                        .transpose()
                        .map_err(|e| e.to_string())?
                        .is_none();
                }
                ProducerNamespaceMember::Disposals if kind.is_dir() => {
                    let (trash, _) = trash(&store)?;
                    for pending in fs::read_dir(trash).map_err(|e| e.to_string())? {
                        let pending = pending.map_err(|e| e.to_string())?;
                        metadata_empty &= pending.file_name() == "owner.json";
                    }
                }
                _ => members_known = false,
            }
        }
        gate.stable()?;
        store.owner_stable()?;
        Ok(crate::verified_lifecycle::producer_namespace_retired(
            objects.is_empty(),
            metadata_empty,
            members_known,
        ))
    }

    /// 由现役宿主政策取得整个临时目录的所有权，不接管已有分配。
    pub fn registered_allocate(
        host_root: &Path,
        config_path: &str,
        producer: &str,
        prefix: &str,
    ) -> Result<Self, String> {
        let root = fs::canonicalize(host_root).map_err(|e| e.to_string())?;
        let (config, bytes) = crate::configuration(&root, config_path)?;
        let mut allocated = None;
        let report = start::with_report(
            &root,
            config_path,
            config,
            &bytes,
            "producer-allocation",
            "allocated",
            |r, _, report| {
                let inputs = PolicyInputs::load(r, &root, config_path, &bytes)?
                    .ok_or("temporary allocation owner is not adopted")?;
                let declaration = inputs
                    .policy
                    .retained_producers
                    .iter()
                    .find(|p| p.id == producer && p.temporary && p.rolling_directory.is_some())
                    .ok_or("whole temporary directory ownership is not adopted")?;
                inputs.stable(r)?;
                let allocation = Self::allocate_owned(
                    &declaration.directory(&inputs.policy.coordinator_root)?,
                    producer,
                    &declaration.generated_outputs,
                    prefix,
                    Some(Owner {
                        root: inputs.policy.coordinator_root.clone(),
                        config_path: inputs.config_path.clone(),
                        producer: producer.into(),
                    }),
                )?;
                report["allocation"] = value!(allocation.path());
                allocated = Some(allocation);
                inputs.stable(r)
            },
        )?;
        if report["status"] != "allocated" {
            return Err(report["error"]
                .as_str()
                .unwrap_or("temporary allocation failed")
                .into());
        }
        allocated.ok_or("temporary allocation was not published".into())
    }

    /// 调用方已加入自身任务并结束原件消费；失败现场按宿主期限保留。
    /// 不改变工作树任务的终态，也不据句柄消失推断测试成功。
    pub fn finish(&mut self, failed: bool) -> Result<Value, String> {
        let owner = self
            .owner
            .clone()
            .ok_or("allocation has no whole-directory ownership")?;
        let path = self
            .path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("allocation name UTF8")?
            .to_owned();
        let (config, bytes) = crate::configuration(&owner.root, &owner.config_path)?;
        start::with_report(
            &owner.root,
            &owner.config_path,
            config,
            &bytes,
            "producer-finish",
            "finished",
            |r, _, report| {
                let inputs = PolicyInputs::load(r, &owner.root, &owner.config_path, &bytes)?
                    .ok_or("temporary allocation owner is not adopted")?;
                let declaration = inputs
                    .policy
                    .retained_producers
                    .iter()
                    .find(|p| p.id == owner.producer && p.rolling_directory.is_some())
                    .ok_or("rolling directory policy is no longer adopted")?;
                let directory = declaration.directory(&inputs.policy.coordinator_root)?;
                let mut store = Store::open(&directory, &owner.producer, false)?;
                let i = store.load(&path)?;
                store.stable(&store.state.objects[i])?;
                self.lease
                    .as_ref()
                    .ok_or("temporary allocation was handed off")?
                    .stable()?;
                inputs.stable(r)?;
                let record = Arc::make_mut(&mut store.state.objects[i]);
                let rolling = record
                    .rolling
                    .as_mut()
                    .ok_or("whole-directory birth missing")?;
                if rolling.terminal.is_some() {
                    return Err("temporary directory already has a terminal handoff".into());
                }
                rolling.terminal = Some(Terminal {
                    observed_at: now()?,
                    failed,
                });
                store.save(i)?;
                drop(store);
                drop(self.lease.take());
                report["producer_objects"] = value!([]);
                declaration.drain_selected(&inputs, r, report, Some(&path))
            },
        )
    }

    /// 生产者声明有限诊断用途，不建立需要人工释放的永久引用。
    pub fn retain_diagnostic(&self) -> Result<(), String> {
        let owner = self
            .owner
            .as_ref()
            .ok_or("finite diagnostic retention requires whole-directory ownership")?;
        let directory = self.path.parent().ok_or("allocation parent")?;
        self.lease
            .as_ref()
            .ok_or("temporary allocation was handed off")?
            .stable()?;
        Self::retain_registered_diagnostic(directory, &owner.producer, &self.path)
    }

    pub fn retain_registered_diagnostic(
        directory: &Path,
        producer: &str,
        allocation: &Path,
    ) -> Result<(), String> {
        let relative = allocation
            .strip_prefix(directory)
            .map_err(|e| e.to_string())?;
        let name = relative.to_str().ok_or("allocation UTF8")?;
        let mut store = Store::open(directory, producer, false)?;
        let i = store.load(name)?;
        store.stable(&store.state.objects[i])?;
        let binding = store.state.objects[i]
            .cache_lease
            .as_ref()
            .ok_or("allocation lease missing")?;
        let held = Lease::acquire(
            &no_symlink_parents(directory, &binding.path)?,
            Some(&binding.id),
            false,
            false,
            None,
        )?
        .ok_or("allocation is being recovered")?;
        held.stable()?;
        Arc::make_mut(&mut store.state.objects[i])
            .rolling
            .as_mut()
            .ok_or("whole directory ownership missing")?
            .diagnostics = true;
        store.save(i)
    }
}

pub(super) fn drain(
    policy: &ProducerPolicy,
    inputs: &PolicyInputs,
    r: &mut Runner,
    directory: &Path,
    object: &Object,
    held: Option<&Lease>,
) -> Result<Value, String> {
    let rolling = object
        .rolling
        .as_ref()
        .ok_or("rolling directory birth missing")?;
    let protected = |reason: &str| value!({"status":"protected","reason":reason});
    let Some(retention) = policy.rolling_directory.as_ref() else {
        return Ok(protected("whole-directory policy is no longer adopted"));
    };
    let since = rolling
        .terminal
        .as_ref()
        .map_or(rolling.born_at, |t| t.observed_at);
    let keep = crate::verified_lifecycle::rolling_keep(
        rolling.terminal.is_some(),
        rolling.terminal.as_ref().is_some_and(|t| t.failed),
        rolling.diagnostics,
        retention.success_keep_seconds,
        retention.failure_keep_seconds,
        retention.interrupted_keep_seconds,
    );
    let expires = since.checked_add(keep).ok_or("rolling expiry overflow")?;
    let released = object.consumers.values().all(|c| c.released);
    if !crate::verified_lifecycle::rolling_disposal(
        true,
        now()? >= expires,
        released,
        held.is_some(),
    ) {
        return Ok(protected(
            "retention window, live capability or unreleased actual consumer",
        ));
    }
    let held = held.unwrap();
    let mut store = Store::open(directory, &policy.id, false)?;
    let i = store.load(&object.path)?;
    store.stable(&store.state.objects[i])?;
    if value!(store.state.objects[i]) != value!(object) {
        return Err("rolling allocation changed before disposal".into());
    }
    if rolling.disposed {
        return Ok(
            value!({"status":"already-disposed","physical_absence":"verified",
            "metadata":retire(policy, inputs, r, &mut store, object, held, false)?}),
        );
    }
    if rolling.disposal_intent.is_none() {
        let intent = Store::receipt_in(
            directory,
            &format!("{}/disposal-intent", rolling.metadata),
            &value!({"schema":"chrono-rolling-directory-disposal/v1","path":object.path,
                "identity":object.identity,"birth":object.intent,"terminal":rolling.terminal,
                "policy_sha256":sha256(&inputs.policy_bytes)}),
        )?;
        Arc::make_mut(&mut store.state.objects[i])
            .rolling
            .as_mut()
            .unwrap()
            .disposal_intent = Some(intent);
        store.save(i)?;
    }
    let current = store.state.objects[i].clone();
    let record_path = no_symlink_parents(directory, &Store::record_name(&object.path)?)?;
    let record = fs::read(&record_path).map_err(|e| e.to_string())?;
    let allocation = directory.join(&object.path);
    let name = allocation
        .strip_prefix(&inputs.policy.coordinator_root)
        .map_err(|e| e.to_string())?
        .to_str()
        .ok_or("rolling path UTF8")?
        .to_owned();
    let mut result = value!({"automatic_cleanup":true,"artifact_disposals":[],
        "status":"failed","unit":"owned-directory","expires":expires});
    let effect = artifact_disposal::dispose(
        &inputs.policy.coordinator_root,
        &[name.clone()],
        &[allocation],
        &mut result,
        || {
            inputs.stable(r)?;
            held.stable()?;
            store.stable(&current)?;
            if fs::read(&record_path).map_err(|e| e.to_string())? != record {
                return Err("rolling allocation references changed before deletion".into());
            }
            chrono_harness::artifact_disposal::source_safe(&inputs.policy.coordinator_root, &name)
        },
    );
    match effect {
        Ok(()) => {
            let completed = Arc::make_mut(&mut store.state.objects[i])
                .rolling
                .as_mut()
                .unwrap();
            completed.disposed = true;
            completed.disposed_at = Some(now()?);
            store.save(i)?;
            result["status"] = value!("disposed");
            let completed = store.state.objects[i].clone();
            result["metadata"] = retire(policy, inputs, r, &mut store, &completed, held, false)?;
        }
        Err(error) => result["error"] = value!(error),
    }
    Ok(result)
}

// A constant owner declaration gives this staging directory one narrow purpose:
// complete removal of already retired producer metadata. No per-file catalogue.
fn trash(store: &Store) -> Result<(PathBuf, Vec<u8>), String> {
    let path = no_symlink_parents(&store.directory, "rolling-disposals")?;
    let fresh = match fs::create_dir(&path) {
        Ok(()) => true,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => false,
        Err(e) => return Err(e.to_string()),
    };
    let expected = value!({"schema":"chrono-producer-metadata-disposal/v1",
        "identity":directory_id(&path)?,"owner_sha256":sha256(&store.owner_bytes),
        "children":"exclusively-retired-metadata"});
    let declaration = path.join("owner.json");
    if fresh {
        write_json(&declaration, &expected, true)?;
    }
    let bytes = fs::read(&declaration).map_err(|e| e.to_string())?;
    if json(&bytes)? != expected {
        return Err("metadata disposal namespace differs".into());
    }
    Ok((path, bytes))
}

fn empty_trash(store: &Store, inputs: &PolicyInputs, r: &mut Runner) -> Result<usize, String> {
    let (path, owner) = trash(store)?;
    let Some(gate) = DirectoryGate::available(&path)? else {
        return Ok(0);
    };
    let relative = path
        .strip_prefix(&inputs.policy.coordinator_root)
        .map_err(|e| e.to_string())?
        .to_str()
        .ok_or("metadata path UTF8")?;
    chrono_harness::artifact_disposal::source_safe(&inputs.policy.coordinator_root, relative)?;
    let mut count = 0;
    for entry in fs::read_dir(&path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_name() == "owner.json" {
            continue;
        }
        inputs.stable(r)?;
        store.owner_stable()?;
        gate.stable()?;
        if fs::read(path.join("owner.json")).map_err(|e| e.to_string())? != owner {
            return Err("metadata disposal owner changed".into());
        }
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_dir() {
            fs::remove_dir_all(entry.path())
        } else if kind.is_file() || kind.is_symlink() {
            fs::remove_file(entry.path())
        } else {
            return Err("special file in metadata disposal namespace".into());
        }
        .map_err(|e| e.to_string())?;
        count += 1;
    }
    Ok(count)
}

fn retire(
    policy: &ProducerPolicy,
    inputs: &PolicyInputs,
    r: &mut Runner,
    store: &mut Store,
    object: &Object,
    held: &Lease,
    staged: bool,
) -> Result<Value, String> {
    let rolling = object.rolling.as_ref().ok_or("rolling ownership missing")?;
    let retention = policy
        .rolling_directory
        .as_ref()
        .ok_or("rolling policy missing")?;
    let expires = rolling
        .disposed_at
        .ok_or("physical disposal time missing")?
        .checked_add(retention.metadata_keep_seconds)
        .ok_or("metadata expiry overflow")?;
    if now()? < expires {
        return Ok(value!({"status":"retained","expires":expires}));
    }
    if !rolling.disposed
        || object.consumers.values().any(|c| !c.released)
        || fs::symlink_metadata(store.directory.join(&object.path)).is_ok()
    {
        return Err(
            "metadata retirement requires released consumers and verified physical absence".into(),
        );
    }
    inputs.stable(r)?;
    held.stable()?;
    store.stable(object)?;
    let (trash, owner) = trash(store)?;
    let Some(gate) = DirectoryGate::available(&trash)? else {
        return Ok(value!({"status":"retained","reason":"metadata disposal busy"}));
    };
    let metadata = no_symlink_parents(&store.directory, &rolling.metadata)?;
    let archived = metadata.join("retired-record.json");
    let active = no_symlink_parents(&store.directory, &Store::record_name(&object.path)?)?;
    if !staged {
        let bytes = fs::read(&active).map_err(|e| e.to_string())?;
        let declaration = json(&bytes)?;
        if declaration["object"] != value!(object)
            || declaration["owner_sha256"] != sha256(&store.owner_bytes)
        {
            return Err("allocation record changed before metadata retirement".into());
        }
        chrono_harness::artifact_disposal::publish_entry(&active, &archived)?;
    } else if active.exists() {
        return Err("retired allocation has an active declaration".into());
    }
    inputs.stable(r)?;
    held.stable()?;
    store.stable(object)?;
    gate.stable()?;
    if fs::read(trash.join("owner.json")).map_err(|e| e.to_string())? != owner {
        return Err("metadata disposal owner changed".into());
    }
    let destination = trash.join(sha256(object.path.as_bytes()));
    // This atomic handoff moves every receipt and the held allocation lease.
    // Interrupted physical removal is recovered by the constant namespace owner.
    chrono_harness::artifact_disposal::publish_entry(&metadata, &destination)?;
    if directory_id(&destination)? != rolling.metadata_identity {
        return Err("retired metadata identity changed".into());
    }
    drop(store.object_guard.take());
    fs::remove_dir_all(&destination).map_err(|e| e.to_string())?;
    match fs::symlink_metadata(&destination) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
        _ => return Err("metadata disposal absence not established".into()),
    }
    Ok(value!({"status":"disposed","physical_absence":"verified"}))
}

pub(super) fn recover(
    policy: &ProducerPolicy,
    inputs: &PolicyInputs,
    r: &mut Runner,
    directory: &Path,
    report: &mut Value,
) -> Result<(), String> {
    if policy.rolling_directory.is_none() {
        return Ok(());
    }
    let mut store = Store::open(directory, &policy.id, false)?;
    let removed = empty_trash(&store, inputs, r)?;
    let metadata = no_symlink_parents(directory, "rolling-metadata")?;
    let mut recovered = 0;
    if metadata.exists() {
        for entry in fs::read_dir(metadata).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let record = entry.path().join("retired-record.json");
            if !record.exists() {
                continue;
            }
            let declaration = json(&fs::read(&record).map_err(|e| e.to_string())?)?;
            let object: Object =
                serde_json::from_value(declaration["object"].clone()).map_err(|e| e.to_string())?;
            if declaration["schema"] != "chrono-retained-producer-object/v1"
                || declaration["owner_sha256"] != sha256(&store.owner_bytes)
                || object
                    .rolling
                    .as_ref()
                    .is_none_or(|d| directory.join(&d.metadata) != entry.path())
            {
                return Err("staged metadata retirement binding differs".into());
            }
            let binding = object
                .cache_lease
                .as_ref()
                .ok_or("staged allocation lease missing")?;
            let Some(held) = Lease::acquire(
                &no_symlink_parents(directory, &binding.path)?,
                Some(&binding.id),
                false,
                true,
                None,
            )?
            else {
                continue;
            };
            let Some(gate) = DirectoryGate::available(&entry.path())? else {
                continue;
            };
            store.object_guard = Some(gate);
            retire(policy, inputs, r, &mut store, &object, &held, true)?;
            drop(store.object_guard.take());
            recovered += 1;
        }
    }
    report["rolling_metadata"] =
        value!({"retired_namespaces_removed":removed,"interrupted_handoffs_recovered":recovered});
    Ok(())
}
