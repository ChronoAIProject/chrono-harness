//! Ordinary fixture recovery and admission live with the existing producer.
//! A context is an address/binding, not an alternate policy authority.
use super::*;
use std::time::{Duration, Instant};

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde(deny_unknown_fields)]
pub(super) struct Capacity {
    pub max_generated_bytes: u64,
    pub max_copy_bytes: u64,
    pub max_capture_bytes: u64,
    pub max_captures: usize,
    pub min_available_bytes: u64,
    pub max_records_per_round: usize,
    pub max_record_snapshot: usize,
    pub max_metadata_bytes_per_round: u64,
    pub max_millis_per_round: u64,
    pub max_entries_per_checkpoint: usize,
}
impl Default for Capacity {
    fn default() -> Self {
        Self {
            max_generated_bytes: 2 * 1024 * 1024 * 1024,
            max_copy_bytes: 1024 * 1024 * 1024,
            max_capture_bytes: 64 * 1024 * 1024,
            max_captures: 256,
            min_available_bytes: 512 * 1024 * 1024,
            max_records_per_round: 16,
            max_record_snapshot: 16384,
            max_metadata_bytes_per_round: 64 * 1024 * 1024,
            max_millis_per_round: 2000,
            max_entries_per_checkpoint: 65536,
        }
    }
}
impl Capacity {
    pub(super) fn validate(&self) -> Result<(), String> {
        if self.max_generated_bytes == 0
            || self.max_copy_bytes == 0
            || self.max_capture_bytes == 0
            || self.max_captures == 0
            || self.max_captures > 4096
            || self.max_records_per_round == 0
            || self.max_record_snapshot < self.max_records_per_round
            || self.max_record_snapshot > 65536
            || self.max_metadata_bytes_per_round == 0
            || self.max_millis_per_round == 0
            || self.max_millis_per_round > 60000
            || self.max_entries_per_checkpoint == 0
            || self.max_entries_per_checkpoint > 1048576
        {
            return Err("invalid finite fixture capacity policy".into());
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Context {
    schema: String,
    pub(super) root: PathBuf,
    config_path: String,
    policy_path: String,
    policy_sha256: String,
    pub(super) directory: PathBuf,
    pub(super) producer: String,
    pub(super) generated_outputs: Vec<String>,
    pub(super) capacity: Capacity,
}
impl Context {
    pub(super) fn register(
        directory: &Path,
        config_path: &str,
        inputs: &PolicyInputs,
        producer: &ProducerPolicy,
    ) -> Result<(), String> {
        producer.capacity.validate()?;
        fs::create_dir_all(directory).map_err(|e| e.to_string())?;
        let context = Self {
            schema: "chrono-fixture-producer-context/v1".into(),
            root: inputs.policy.coordinator_root.clone(),
            config_path: config_path.into(),
            policy_path: inputs.policy_path.clone(),
            policy_sha256: sha256(&inputs.policy_bytes),
            directory: directory.into(),
            producer: producer.id.clone(),
            generated_outputs: producer.generated_outputs.clone(),
            capacity: producer.capacity.clone(),
        };
        let path = no_symlink_parents(directory, "producer-context.json")?;
        let bytes = serde_json::to_vec(&context).map_err(|e| e.to_string())?;
        match fs::read(&path) {
            Ok(previous) if json(&previous)? == json(&bytes)? => Ok(()),
            Ok(_) => {
                Err("fixture producer context differs; explicit owner migration required".into())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                match write_json(&path, &context, true) {
                    Ok(_) => Ok(()),
                    Err(_)
                        if fs::read(&path).ok().and_then(|b| json(&b).ok())
                            == json(&bytes).ok() =>
                    {
                        Ok(())
                    }
                    Err(error) => Err(error),
                }
            }
            Err(e) => Err(e.to_string()),
        }
    }
    pub(super) fn migrate(
        directory: &Path,
        inputs: &PolicyInputs,
        producer: &ProducerPolicy,
        r: &mut Runner,
    ) -> Result<Value, String> {
        let current = Self {
            schema: "chrono-fixture-producer-context/v1".into(),
            root: inputs.policy.coordinator_root.clone(),
            config_path: inputs.config_path.clone(),
            policy_path: inputs.policy_path.clone(),
            policy_sha256: sha256(&inputs.policy_bytes),
            directory: directory.into(),
            producer: producer.id.clone(),
            generated_outputs: producer.generated_outputs.clone(),
            capacity: producer.capacity.clone(),
        };
        let current_bytes = serde_json::to_vec(&current).map_err(|e| e.to_string())?;
        let context_path = no_symlink_parents(directory, "producer-context.json")?;
        let cursor_path = no_symlink_parents(directory, "recovery-state/cursor.json")?;
        let cursor_current = match fs::read(&cursor_path) {
            Ok(bytes) => decode::<retirement::Recovery>(&bytes)
                .and_then(|c| c.validate())
                .is_ok(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
            Err(e) => return Err(e.to_string()),
        };
        match fs::read(&context_path) {
            Ok(bytes) if json(&bytes)? == json(&current_bytes)? && cursor_current => {
                // This performs no context/cursor mutation. Existing consumers
                // need no exclusion merely to observe the same binding.
                current.stable()?;
                return Ok(
                    value!({"status":"already-current","historical_outcomes":"unchanged","capacity_reservations":"unchanged"}),
                );
            }
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.to_string()),
        }
        let store = Store::open(directory, &producer.id, false)?;
        let (objects, errors) = store.records()?;
        if !errors.is_empty() {
            return Err(format!(
                "context migration has unknown registrations; preserve originals: {}",
                value!(errors)
            ));
        }
        let mut metadata = vec![];
        for object in &objects {
            let guard = DirectoryGate::available(&directory.join(&object.path))?
                .ok_or("context migration metadata busy")?;
            if let Some(binding) = &object.cache_lease {
                let lease = Lease::acquire(
                    &no_symlink_parents(directory, &binding.path)?,
                    Some(&binding.id),
                    false,
                    true,
                    None,
                )?
                .ok_or("context migration has active allocation consumers")?;
                lease.stable()?;
                // Every new consumer of this existing allocation first enters
                // its metadata gate. Retain that EX gate after proving current
                // cache holders quiescent; a second retained FD adds no exclusion.
            }
            metadata.push(guard);
        }
        let path = no_symlink_parents(directory, "producer-context.json")?;
        let previous = match fs::read(&path) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.to_string()),
        };
        if let Some(bytes) = &previous {
            let context: Self = decode(bytes)?;
            if context.schema != "chrono-fixture-producer-context/v1"
                || context.root != inputs.policy.coordinator_root
                || context.directory != directory
                || context.producer != producer.id
                || context.config_path != inputs.config_path
                || context.policy_path != inputs.policy_path
            {
                return Err("unknown previous fixture context binding; preserve original".into());
            }
            let name = format!("producer-context-source-{}.json", sha256(bytes));
            let source = no_symlink_parents(directory, &name)?;
            match fs::read(&source) {
                Ok(old) if old != *bytes => return Err("context source collision".into()),
                Ok(_) => (),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    write_bytes(&source, bytes, true)?
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        let cursor = no_symlink_parents(directory, "recovery-state/cursor.json")?;
        let mut cursor_conversion = None;
        let cursor_bytes = match fs::read(&cursor) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.to_string()),
        };
        if let Some(bytes) = cursor_bytes {
            if decode::<retirement::Recovery>(&bytes).is_err() {
                let value: Value = json(&bytes)?;
                let pending = if let Some(names) = value.as_array() {
                    names
                        .iter()
                        .map(|n| {
                            n.as_str()
                                .map(str::to_owned)
                                .ok_or("pending address is not a string")
                        })
                        .collect::<Result<Vec<_>, _>>()?
                } else if let Some(name) = value.as_str() {
                    if name.is_empty() {
                        vec![]
                    } else {
                        vec![name.into()]
                    }
                } else {
                    return Err("unknown recovery cursor; preserve original".into());
                };
                for name in &pending {
                    if !store.contains(name)? {
                        return Err(
                            "old pending recovery lacks its exact registration; preserve cursor"
                                .into(),
                        );
                    }
                }
                let recovery = retirement::Recovery {
                    bucket: 0,
                    after: String::new(),
                    round_started: retirement::now_nanos()?,
                    pending,
                };
                recovery.validate()?;
                let name = format!("producer-recovery-source-{}.json", sha256(&bytes));
                let source = no_symlink_parents(directory, &name)?;
                if !source.exists() {
                    write_bytes(&source, &bytes, true)?;
                }
                cursor_conversion = Some((
                    bytes,
                    serde_json::to_vec(&recovery).map_err(|e| e.to_string())?,
                ));
            }
        }
        for guard in &metadata {
            guard.stable()?;
        }
        store.owner_stable()?;
        inputs.stable(r)?;
        if fs::read(&path).ok() != previous {
            return Err("fixture context changed during migration".into());
        }
        if let Some((before, after)) = cursor_conversion {
            if fs::read(&cursor).map_err(|e| e.to_string())? != before {
                return Err("recovery cursor changed during migration".into());
            }
            write_bytes(&cursor, &after, false)?;
        }
        write_json(&path, &current, false)?;
        Ok(
            value!({"status":"migrated","objects":objects.len(),"historical_outcomes":"unchanged","capacity_reservations":"unchanged","previous_context_sha256":previous.as_ref().map(|b|sha256(b)),"new_policy_sha256":current.policy_sha256}),
        )
    }

    pub(super) fn load(directory: &Path, producer: &str) -> Result<Self, String> {
        let path = no_symlink_parents(directory, "producer-context.json")?;
        let bytes =
            fs::read(path).map_err(|e| format!("registered fixture context required: {e}"))?;
        let context: Self = decode(&bytes)?;
        if context.schema != "chrono-fixture-producer-context/v1"
            || context.directory != directory
            || context.producer != producer
        {
            return Err("fixture producer context identity differs".into());
        }
        context.capacity.validate()?;
        context.stable()?;
        Ok(context)
    }
    pub(super) fn for_allocation(path: &Path) -> Result<Option<Self>, String> {
        let Some(directory) = path.parent() else {
            return Ok(None);
        };
        let context_path = no_symlink_parents(directory, "producer-context.json")?;
        if !context_path.exists() {
            return Ok(None);
        }
        let context: Self = decode(&fs::read(context_path).map_err(|e| e.to_string())?)?;
        Self::load(directory, &context.producer).map(Some)
    }
    pub(super) fn stable(&self) -> Result<(), String> {
        let bytes = fs::read(no_symlink_parents(&self.root, &self.policy_path)?)
            .map_err(|e| e.to_string())?;
        if sha256(&bytes) != self.policy_sha256 {
            return Err("fixture capacity policy changed; preserve original outputs".into());
        }
        let policy: Policy = decode(&bytes)?;
        let producer = policy
            .retained_producers
            .iter()
            .find(|p| p.id == self.producer)
            .ok_or("fixture context producer is not declared")?;
        if producer.capacity != self.capacity
            || producer.generated_outputs != self.generated_outputs
            || producer.directory(&self.root)? != self.directory
        {
            return Err("fixture context differs from authoritative capacity policy".into());
        }
        Ok(())
    }
    pub(super) fn admit(&self, directory: &Path, requested: u64) -> Result<Value, String> {
        self.stable()?;
        let available = available_bytes(directory)?;
        let required = self
            .capacity
            .min_available_bytes
            .checked_add(requested)
            .ok_or("fixture admission byte overflow")?;
        let observation = value!({"available_bytes":available,"minimum_available_bytes":self.capacity.min_available_bytes,
            "requested_bytes":requested,"physical_reclaim_bytes":Value::Null,
            "enforcement":"admission-and-external-writer-checkpoint; not a reservation or filesystem quota"});
        if available < required {
            return Err(format!(
                "fixture capacity unmet; production refused, protected/unknown originals preserved: {observation}"
            ));
        }
        Ok(observation)
    }
    pub(super) fn entry(&self, object: Option<&str>) -> Result<Value, String> {
        self.stable()?;
        let (config, bytes) = crate::configuration(&self.root, &self.config_path)?;
        let mut report = start::with_report(
            &self.root,
            &self.config_path,
            config,
            &bytes,
            "fixture-recovery",
            "observed",
            |r, _, report| {
                let inputs = PolicyInputs::load(r, &self.root, &self.config_path, &bytes)?
                    .ok_or("fixture cleanup owner disappeared")?;
                if sha256(&inputs.policy_bytes) != self.policy_sha256 {
                    return Err("fixture context no longer binds current committed owner".into());
                }
                let producer = inputs
                    .policy
                    .retained_producers
                    .iter()
                    .find(|p| p.id == self.producer)
                    .ok_or("fixture producer is no longer adopted")?;
                if producer.directory(&inputs.policy.coordinator_root)? != self.directory
                    || producer.generated_outputs != self.generated_outputs
                    || producer.capacity != self.capacity
                {
                    return Err("fixture context differs from authoritative producer policy".into());
                }
                report["producer_objects"] = value!([]);
                // Object failures remain actual failed rows. They do not veto
                // unrelated allocations; owner/context failures still do.
                if let Err(error) = producer.drain_selected(&inputs, r, report, object) {
                    report["recovery_error"] = value!(error);
                    return Err(report["recovery_error"].as_str().unwrap().into());
                }
                Ok(())
            },
        )?;
        if report["status"] != "observed" && report["recovery_error"].is_null() {
            return Err(format!(
                "fixture recovery owner failed: {} (report {})",
                report["error"], report["report_path"]
            ));
        }
        let inventory =
            chrono_harness::retained_artifacts::retention::Inventory::adopted(&self.root)?;
        let mut retirements = vec![];
        for row in report["producer_objects"].as_array().into_iter().flatten() {
            let retirement = row.get("original_retirement").unwrap_or(row);
            let Some(paths) = retirement["retention_paths"].as_array() else {
                continue;
            };
            let paths = paths
                .iter()
                .map(|p| {
                    p.as_str()
                        .map(str::to_owned)
                        .ok_or("retirement path is not a string")
                })
                .collect::<Result<Vec<_>, _>>()?;
            if let Some(inventory) = &inventory {
                match inventory.maintain_paths(&paths) {
                    Ok(result) => retirements.push(result),
                    Err(error) => {
                        retirements.push(value!({"paths":paths,"status":"deferred","error":error}))
                    }
                }
            }
        }
        report["original_retention"] = value!(retirements);
        Ok(report)
    }
    pub(super) fn measure(&self, allocation: &Path) -> Result<Value, String> {
        self.admit(&self.directory, 0)?;
        let mut store = Store::open(&self.directory, &self.producer, false)?;
        let object = allocation
            .file_name()
            .and_then(|p| p.to_str())
            .ok_or("allocation UTF-8")?;
        store.load(object)?;
        let record = &store.state.objects[0];
        if record.retiring {
            return Err("fixture original lifetime released; further production refused".into());
        }
        store.stable(record)?;
        let mut paths = self
            .generated_outputs
            .iter()
            .map(|p| no_symlink_parents(allocation, p.trim_end_matches('/')))
            .collect::<Result<Vec<_>, _>>()?;
        let began = Instant::now();
        let mut entries = 0;
        let mut bytes = 0u64;
        let mut allocated = 0u64;
        while let Some(path) = paths.pop() {
            if entries >= self.capacity.max_entries_per_checkpoint
                || began.elapsed() >= Duration::from_millis(self.capacity.max_millis_per_round)
            {
                return Err("fixture checkpoint bounded; usage unknown, production deferred and originals preserved".into());
            }
            entries += 1;
            let metadata = match fs::symlink_metadata(&path) {
                Ok(m) => m,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e.to_string()),
            };
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                allocated = allocated.saturating_add(metadata.blocks().saturating_mul(512));
            }
            if metadata.is_dir() {
                for child in fs::read_dir(path).map_err(|e| e.to_string())? {
                    if paths.len() >= self.capacity.max_entries_per_checkpoint {
                        return Err(
                            "fixture checkpoint entry capacity; usage unknown, production deferred"
                                .into(),
                        );
                    }
                    paths.push(child.map_err(|e| e.to_string())?.path());
                }
            } else {
                bytes = bytes.saturating_add(metadata.len());
            }
        }
        let (capture_bytes, capture_accounted_bytes, reserved_input_bytes) =
            self.capture_usage(record)?;
        let mut metadata_bytes = 0u64;
        let mut metadata_entries = 0usize;
        if let Some(name) = &record.metadata_directory {
            for entry in fs::read_dir(no_symlink_parents(&self.directory, name)?)
                .map_err(|e| e.to_string())?
            {
                metadata_entries += 1;
                if metadata_entries > self.capacity.max_entries_per_checkpoint
                    || began.elapsed() >= Duration::from_millis(self.capacity.max_millis_per_round)
                {
                    return Err("fixture metadata checkpoint bounded; capacity unknown, production deferred".into());
                }
                let entry = entry.map_err(|e| e.to_string())?;
                let m = fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?;
                if !m.is_file() || m.file_type().is_symlink() {
                    return Err(
                        "fixture metadata ownership/type unknown; preserve originals".into(),
                    );
                }
                metadata_bytes = metadata_bytes.saturating_add(m.len());
            }
        }
        let result = value!({"generated_logical_bytes":bytes,"generated_allocated_bytes":allocated,
            "capture_logical_bytes":capture_bytes,"capture_accounted_bytes":capture_accounted_bytes,
            "capture_reserved_input_bytes":reserved_input_bytes,"physical_reclaim_bytes":Value::Null,
            "metadata_logical_bytes":metadata_bytes,"metadata_entries":metadata_entries,
            "capture_count":record.captures.len(),"max_captures":self.capacity.max_captures,
            "max_generated_bytes":self.capacity.max_generated_bytes,"max_capture_bytes":self.capacity.max_capture_bytes,
            "scope":"exact allocation and declared outputs","producer_outcome":"unchanged"});
        if metadata_bytes > self.capacity.max_metadata_bytes_per_round
            || bytes > self.capacity.max_generated_bytes
            || capture_accounted_bytes > self.capacity.max_capture_bytes
            || record.captures.len() > self.capacity.max_captures
        {
            return Err(format!(
                "fixture capacity unmet; further production refused, originals preserved: {result}"
            ));
        }
        Ok(result)
    }
    // Account known input promises under the allocation's metadata exclusion.
    // A still-unwritten input consumes capacity; materialization does not charge
    // it twice. External stdout/stderr retain their checkpoint boundary.
    fn capture_usage(&self, record: &Object) -> Result<(u64, u64, u64), String> {
        let allocation = self.directory.join(&record.path);
        let mut actual = 0u64;
        let mut accounted = 0u64;
        let mut reserved = 0u64;
        for (path, intent) in &record.captures {
            let input = intent["reserved_input_bytes"]
                .as_u64()
                .ok_or("native capture input reservation unknown; preserve originals")?;
            reserved = reserved
                .checked_add(input)
                .ok_or("capture reservation overflow")?;
            let mut charged = input;
            for name in ["stdin.bytes", "stdout.bytes", "stderr.bytes"] {
                let path = no_symlink_parents(&allocation, &format!("{path}/{name}"))?;
                match fs::symlink_metadata(path) {
                    Ok(m) if m.is_file() && !m.file_type().is_symlink() => {
                        actual = actual.checked_add(m.len()).ok_or("capture byte overflow")?;
                        charged = if name == "stdin.bytes" {
                            input.max(m.len())
                        } else {
                            charged
                                .checked_add(m.len())
                                .ok_or("capture byte overflow")?
                        };
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                    Err(e) => return Err(e.to_string()),
                    Ok(_) => {
                        return Err(
                            "native capture identity/type differs; preserve originals".into()
                        );
                    }
                }
            }
            accounted = accounted
                .checked_add(charged)
                .ok_or("capture byte overflow")?;
        }
        Ok((actual, accounted, reserved))
    }
    pub(super) fn copy_admission(
        &self,
        record: &Object,
        member: &str,
        length: u64,
    ) -> Result<(), String> {
        if record.original_release.is_some() || record.retiring {
            return Err("fixture original lifetime released; copy refused".into());
        }
        self.production_admission()?;
        let existing = record
            .bodies
            .iter()
            .filter(|(p, _)| *p != member)
            .try_fold(0u64, |sum, (_, body)| sum.checked_add(body.length))
            .ok_or("fixture copy accounting overflow")?;
        let pending = record
            .copy_intents
            .iter()
            .filter(|(p, _)| *p != member && !record.bodies.contains_key(*p))
            .try_fold(0u64, |sum, (_, length)| sum.checked_add(*length))
            .ok_or("fixture pending copy accounting overflow")?;
        if record
            .copy_intents
            .get(member)
            .is_some_and(|reserved| *reserved != length)
        {
            return Err(
                "fixture pending copy length changed; preserve original reservation".into(),
            );
        }
        if existing.saturating_add(pending).saturating_add(length) > self.capacity.max_copy_bytes
            || (!record.bodies.contains_key(member)
                && !record.copy_intents.contains_key(member)
                && record.bodies.len() + record.copy_intents.len()
                    >= self.capacity.max_entries_per_checkpoint)
        {
            return Err(
                "fixture copied-body capacity exceeded before writing; originals preserved".into(),
            );
        }
        self.admit(&self.directory, length)?;
        Ok(())
    }
    pub(super) fn capture_admission(&self, record: &Object, input: u64) -> Result<(), String> {
        if record.original_release.is_some() || record.retiring {
            return Err("fixture original lifetime released; capture refused".into());
        }
        let (_, accounted, _) = self.capture_usage(record)?;
        if record.captures.len() >= self.capacity.max_captures
            || accounted.saturating_add(input) > self.capacity.max_capture_bytes
        {
            return Err(
                "fixture capture capacity exceeded before spawn; existing originals preserved"
                    .into(),
            );
        }
        self.admit(&self.directory, input)?;
        Ok(())
    }
}

fn available_bytes(path: &Path) -> Result<u64, String> {
    #[cfg(unix)]
    {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        let path = CString::new(path.as_os_str().as_bytes()).map_err(|e| e.to_string())?;
        let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        if unsafe { libc::statvfs(path.as_ptr(), stats.as_mut_ptr()) } != 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let stats = unsafe { stats.assume_init() };
        (stats.f_bavail as u64)
            .checked_mul(stats.f_frsize as u64)
            .ok_or("available filesystem byte overflow".into())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err("fixture capacity filesystem observation unsupported".into())
    }
}

impl TemporaryHost {
    /// A checkpoint refuses further production; it never truncates evidence or
    /// claims a byte reservation against concurrently running external writers.
    pub fn checkpoint(&self) -> Result<Value, String> {
        self.lease
            .as_ref()
            .ok_or("allocation already released")?
            .stable()?;
        self.context.checkpoint(&self.path)
    }
    /// Release this owner, then attempt its exact outputs. Descendant/Command
    /// copies and actual diagnostic references continue to prevent deletion.
    /// Neither an unknown result nor this handle's release means task success.
    pub fn finish(&mut self) -> Result<Value, String> {
        if self.lease.is_some() {
            RetainedArtifact { directory:self.context.directory.clone(),producer:self.context.producer.clone(), object:self.path.file_name().and_then(|p|p.to_str()).ok_or("allocation UTF-8")?.into() }
                .release_originals("responsible fixture consumer completed original inspection or diagnostic handoff; no producer outcome inferred")?;
        }
        drop(self.lease.take());
        self.context.entry(Some(
            self.path
                .file_name()
                .and_then(|p| p.to_str())
                .ok_or("allocation UTF-8")?,
        ))
    }
}
