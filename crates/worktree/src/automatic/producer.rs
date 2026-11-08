//! Explicit per-object producer registrations; no shared mutable catalogue.
use super::*;
use std::sync::Arc;
mod body;
mod rolling;
use body::fingerprint;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProducerPolicy {
    pub(super) id: String,
    pub(super) directory: String,
    pub(super) generated_outputs: Vec<String>,
    #[serde(default)]
    pub(super) temporary: bool,
    /// Exact Git roots within each allocation; no nested repository discovery.
    #[serde(default)]
    pub(super) source_roots: Vec<String>,
    /// Exact copied input members are enrolled by the producer, never discovered.
    #[serde(default)]
    pub(super) fixture_bodies: bool,
    #[serde(default)]
    rolling_directory: Option<rolling::Policy>,
    /// These exact registered operations publish one whole run directory.
    #[serde(default)]
    pub(super) operations: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CacheLease {
    path: String,
    id: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reference {
    released: bool,
    reason: String,
    receipt: Option<Receipt>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Object {
    path: String,
    identity: String,
    intent: Receipt,
    sealed: bool,
    publication: Option<Receipt>,
    #[serde(deserialize_with = "strict_map")]
    generated: BTreeMap<String, String>,
    #[serde(deserialize_with = "strict_evidence")]
    evidence: BTreeMap<String, Value>,
    #[serde(deserialize_with = "strict_map")]
    consumers: BTreeMap<String, Reference>,
    retention_reason: String,
    disposed: bool,
    attempts: Vec<Receipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cache_lease: Option<CacheLease>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[serde(deserialize_with = "strict_map")]
    bodies: BTreeMap<String, body::Body>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rolling: Option<rolling::Directory>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyRegistry {
    schema: String,
    producer: String,
    directory_id: String,
    lease_id: String,
    objects: Vec<Object>,
}

// Decode typed members directly while rejecting duplicate declarations.
fn strict_map<'de, D, T>(d: D) -> Result<BTreeMap<String, T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Entries<T>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Entries<T> {
        type Value = BTreeMap<String, T>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a map without duplicate members")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut out = BTreeMap::new();
            while let Some(key) = map.next_key::<String>()? {
                if out.contains_key(&key) {
                    return Err(serde::de::Error::custom(format!(
                        "duplicate JSON member {key}"
                    )));
                }
                out.insert(key, map.next_value::<T>()?);
            }
            Ok(out)
        }
    }
    d.deserialize_map(Entries(std::marker::PhantomData))
}

// Opaque evidence still needs recursive duplicate detection inside its values.
fn strict_evidence<'de, D>(d: D) -> Result<BTreeMap<String, Value>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    serde_json::from_value(chrono_harness::strict_json_value(d)?).map_err(serde::de::Error::custom)
}

// Immutable namespace identity. Membership and mutable state live only in the
// producer-written object registrations, never in a shared mutable catalogue.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Owner {
    schema: String,
    producer: String,
    directory_id: String,
    records_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObjectRecord<T = Object> {
    schema: String,
    owner_sha256: String,
    object: T,
}

struct Loaded {
    producer: String,
    directory_id: String,
    owner_id: String,
    objects: Vec<Arc<Object>>,
}

struct Store {
    directory: PathBuf,
    owner: Owner,
    owner_bytes: Vec<u8>,
    state: Loaded,
    object_guard: Option<DirectoryGate>,
    began: std::time::Instant,
}

// The physical store already exists. Lock it before examining birth state, so
// publishing the named lease cannot expose an unlocked, unexplained birth to
// another ordinary publisher. This gate never creates or adopts an owner file.
struct DirectoryGate(chrono_harness::ownership::DirectoryLease);

impl DirectoryGate {
    fn acquire(
        directory: &Path,
        began: std::time::Instant,
        bound: std::time::Duration,
        observer: &mut impl FnMut(&str),
    ) -> Result<Self, String> {
        loop {
            if let Some(gate) = Self::available(directory)? {
                return Ok(gate);
            }
            observer("birth-gate-busy");
            if began.elapsed() >= bound {
                return Err(format!(
                    "producer directory admission remains busy; preserve objects: directory={} elapsed_millis={} bound_millis={}",
                    directory.display(),
                    began.elapsed().as_millis(),
                    bound.as_millis()
                ));
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    fn available(directory: &Path) -> Result<Option<Self>, String> {
        chrono_harness::ownership::DirectoryLease::acquire(directory, true, None)
            .map(|lease| lease.map(Self))
    }
    fn stable(&self) -> Result<(), String> {
        self.0.stable()
    }
}

impl Store {
    fn partition(directory: &Path, producer: &str, candidate: &str) -> Result<Value, String> {
        Self::partition_observed(directory, producer, candidate, |_| Ok(()))
    }

    fn partition_observed(
        directory: &Path,
        producer: &str,
        candidate: &str,
        mut observer: impl FnMut(&str) -> Result<(), String>,
    ) -> Result<Value, String> {
        let path = no_symlink_parents(directory, "objects.json")?;
        let bytes = fs::read(&path).map_err(|e| e.to_string())?;
        if serde_json::from_slice::<Owner>(&bytes).is_ok() {
            Self::open(directory, producer, false)?;
            return Ok(value!({"status":"already-current","producer":producer,
                "disposal":"not-performed","historical_outcomes":"unchanged"}));
        }
        // Only this explicit current-owner operation reads the superseded data.
        // Ordinary publication has no v1 decoder or fallback.
        let previous: LegacyRegistry = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        let original_identity = fingerprint(&path)?;
        let began = std::time::Instant::now();
        let bound = std::time::Duration::from_secs(30);
        if previous.schema != "chrono-retained-producer/v1"
            || previous.producer != producer
            || previous.directory_id != directory_id(directory)?
        {
            return Err("producer partition input identity differs; preserve originals".into());
        }
        let lease = Lease::acquire(
            &no_symlink_parents(directory, "registry.lease")?,
            Some(&previous.lease_id),
            false,
            true,
            Some(bound),
        )?
        .ok_or("producer partition has live v1 writers")?;
        let birth = DirectoryGate::acquire(directory, began, bound, &mut |_| {})?;
        if fs::read(&path).map_err(|e| e.to_string())? != bytes
            || fingerprint(&path)? != original_identity
        {
            return Err("producer partition input changed during exclusion".into());
        }
        let mut seen = BTreeSet::new();
        for object in &previous.objects {
            Self::record_name(&object.path)?;
            if !seen.insert(&object.path)
                || (object.consumers.is_empty() && object.cache_lease.is_none())
            {
                return Err("invalid exact producer object registration".into());
            }
        }
        let records = no_symlink_parents(directory, "producer-records")?;
        let created = match fs::create_dir(&records) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => false,
            Err(error) => return Err(error.to_string()),
        };
        let owner = Owner {
            schema: "chrono-retained-producer/v2".into(),
            producer: producer.into(),
            directory_id: previous.directory_id.clone(),
            records_id: directory_id(&records)?,
        };
        let namespace = value!({"schema":"chrono-producer-partition-namespace/v1",
            "owner":owner,"source_sha256":sha256(&bytes),"source_length":bytes.len(),
            "original_identity":original_identity,"task_completion":"not-claimed"});
        let namespace_name = format!(
            "producer-partition-namespace-{}.json",
            sha256(&serde_json::to_vec(&namespace).map_err(|e| e.to_string())?)
        );
        if !created {
            let retained = fs::read(no_symlink_parents(directory, &namespace_name)?)
                .map_err(|e| format!("unknown producer metadata namespace; preserve it: {e}"))?;
            if json(&retained)? != namespace {
                return Err("producer metadata birth binding changed".into());
            }
        }
        let namespace = Self::receipt_in(directory, "producer-partition-namespace", &namespace)?;
        let source_name = format!("producer-partition-source-{}.json", sha256(&bytes));
        let source_path = no_symlink_parents(directory, &source_name)?;
        match fs::read(&source_path) {
            Ok(retained) if retained == bytes => (),
            Ok(_) => return Err("producer partition original collision".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                write_bytes(&source_path, &bytes, true)?;
            }
            Err(error) => return Err(error.to_string()),
        }
        let mut owner_bytes = serde_json::to_vec_pretty(&owner).map_err(|e| e.to_string())?;
        owner_bytes.push(b'\n');
        let mut declarations = vec![];
        for object in &previous.objects {
            let name = Self::record_name(&object.path)?;
            let destination = no_symlink_parents(directory, &name)?;
            let next = ObjectRecord {
                schema: "chrono-retained-producer-object/v1".into(),
                owner_sha256: sha256(&owner_bytes),
                object,
            };
            let next_bytes = serde_json::to_vec(&next).map_err(|e| e.to_string())?;
            match fs::read(&destination) {
                Ok(retained) => {
                    if retained != next_bytes {
                        return Err(
                            "producer partition cannot replace an unknown registration".into()
                        );
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                Err(error) => return Err(error.to_string()),
            }
            write_bytes_staged(&destination, &next_bytes, false, directory)?;
            declarations.push(value!({"path":name,"sha256":sha256(&next_bytes)}));
        }
        observer("partition-records-published")?;
        let record = value!({"schema":"chrono-producer-partition/v1","producer":producer,
            "candidate":candidate,"source":{"path":source_name,"sha256":sha256(&bytes),
            "length":bytes.len(),"original_identity":original_identity},"namespace":namespace,
            "registrations":declarations,"owner_sha256":sha256(&owner_bytes),
            "historical_outcomes":"unchanged","disposal":"not-performed",
            "task_completion":"not-claimed"});
        let receipt = Self::receipt_in(directory, "producer-partition", &record)?;
        lease.stable()?;
        birth.stable()?;
        if fs::read(&path).map_err(|e| e.to_string())? != bytes
            || fingerprint(&path)? != original_identity
        {
            return Err("producer partition source changed before activation".into());
        }
        // One atomic activation. Excluded old writers subsequently reject this
        // current strict owner shape; current writers never take their old lock.
        write_bytes(&path, &owner_bytes, false)?;
        Ok(
            value!({"status":"partitioned","producer":producer,"receipt":receipt,
            "objects":previous.objects.len(),"disposal":"not-performed",
            "historical_outcomes":"unchanged"}),
        )
    }

    fn open(directory: &Path, producer: &str, create: bool) -> Result<Self, String> {
        Self::open_observed(directory, producer, create, |_| {})
    }

    fn open_observed(
        directory: &Path,
        producer: &str,
        create: bool,
        mut observer: impl FnMut(&str),
    ) -> Result<Self, String> {
        observer("store-open-entered");
        absolute(directory, false)?;
        if producer.is_empty() {
            return Err("producer identity is required".into());
        }
        let began = std::time::Instant::now();
        let path = no_symlink_parents(directory, "objects.json")?;
        if !path.exists() && create {
            observer("birth-gate-entered");
            let birth = DirectoryGate::acquire(
                directory,
                began,
                std::time::Duration::from_secs(30),
                &mut observer,
            )?;
            observer("birth-gate-acquired");
            if !path.exists() {
                // A missing v1 registry beside its old exclusion capability is
                // an unfinished/unknown birth, never a fresh namespace.
                if no_symlink_parents(directory, "registry.lease")?.exists() {
                    return Err(
                        "producer registry birth is unknown; preserve original lease".into(),
                    );
                }
                let records = no_symlink_parents(directory, "producer-records")?;
                fs::create_dir(&records).map_err(|e| {
                    format!("producer record birth unavailable; preserve originals: {e}")
                })?;
                let owner = Owner {
                    schema: "chrono-retained-producer/v2".into(),
                    producer: producer.into(),
                    directory_id: directory_id(directory)?,
                    records_id: directory_id(&records)?,
                };
                birth.stable()?;
                write_json(&path, &owner, true)?;
            }
        }
        observer("registry-read-entered");
        let owner_bytes = fs::read(&path)
            .map_err(|e| format!("producer owner unavailable; preserve objects: {e}"))?;
        let owner: Owner = serde_json::from_slice(&owner_bytes)
            .map_err(|e| format!("producer owner requires current v2 format; use explicit producer partition migration for retained v1 data: {e}"))?;
        let owner_id = fingerprint(&path)?;
        if owner.schema != "chrono-retained-producer/v2"
            || owner.producer != producer
            || owner.directory_id != directory_id(directory)?
            || owner.records_id
                != directory_id(&no_symlink_parents(directory, "producer-records")?)?
        {
            return Err("producer owner identity differs; preserve objects".into());
        }
        observer("registry-read-returned");
        let store = Self {
            directory: directory.into(),
            state: Loaded {
                producer: owner.producer.clone(),
                directory_id: owner.directory_id.clone(),
                owner_id,
                objects: vec![],
            },
            owner,
            owner_bytes,
            object_guard: None,
            began,
        };
        store.owner_stable()?;
        observer("store-open-returned");
        Ok(store)
    }

    fn owner_stable(&self) -> Result<(), String> {
        let path = no_symlink_parents(&self.directory, "objects.json")?;
        if directory_id(&self.directory)? != self.owner.directory_id
            || directory_id(&no_symlink_parents(&self.directory, "producer-records")?)?
                != self.owner.records_id
            || fingerprint(&path)? != self.state.owner_id
            || fs::read(path).map_err(|e| e.to_string())? != self.owner_bytes
        {
            return Err("producer owner changed; preserve objects".into());
        }
        Ok(())
    }

    fn record_name(object: &str) -> Result<String, String> {
        relative_path(object)?;
        if Path::new(object).components().count() != 1 || object == "producer-records" {
            return Err("producer artifact must be one exact child outside owner metadata".into());
        }
        Ok(format!(
            "producer-records/{}.json",
            sha256(object.as_bytes())
        ))
    }

    fn contains(&self, object: &str) -> Result<bool, String> {
        Ok(no_symlink_parents(&self.directory, &Self::record_name(object)?)?.exists())
    }

    fn bind(&mut self, object: &str) -> Result<(), String> {
        self.owner_stable()?;
        self.state.objects.clear();
        drop(self.object_guard.take());
        self.object_guard = Some(DirectoryGate::acquire(
            &self.gate_path(object)?,
            self.began,
            std::time::Duration::from_secs(30),
            &mut |_| {},
        )?);
        self.owner_stable()
    }

    fn decode_record(&self, path: &Path) -> Result<Arc<Object>, String> {
        if !fs::symlink_metadata(path)
            .map_err(|e| e.to_string())?
            .is_file()
            || fs::symlink_metadata(path)
                .map_err(|e| e.to_string())?
                .file_type()
                .is_symlink()
        {
            return Err("producer registration must be a physical regular file".into());
        }
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        let record: ObjectRecord = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if record.schema != "chrono-retained-producer-object/v1"
            || record.owner_sha256 != sha256(&self.owner_bytes)
            || path
                != no_symlink_parents(&self.directory, &Self::record_name(&record.object.path)?)?
            || (record.object.consumers.is_empty() && record.object.cache_lease.is_none())
        {
            return Err("invalid exact producer object registration".into());
        }
        Ok(Arc::new(record.object))
    }

    fn load(&mut self, object: &str) -> Result<usize, String> {
        if self.state.objects.first().is_some_and(|o| o.path == object) {
            return Ok(0);
        }
        self.bind(object)?;
        self.load_owned(object)
    }

    fn load_available(&mut self, object: &str) -> Result<bool, String> {
        self.owner_stable()?;
        if self
            .state
            .objects
            .first()
            .is_some_and(|current| current.path == object)
        {
            self.object_guard
                .as_ref()
                .ok_or("loaded producer metadata has no exclusion")?
                .stable()?;
            return Ok(true);
        }
        self.state.objects.clear();
        drop(self.object_guard.take());
        self.object_guard = DirectoryGate::available(&self.gate_path(object)?)?;
        if self.object_guard.is_none() {
            return Ok(false);
        }
        self.load_owned(object)?;
        Ok(true)
    }

    fn gate_path(&self, object: &str) -> Result<PathBuf, String> {
        let record = no_symlink_parents(&self.directory, &Self::record_name(object)?)?;
        if record.exists() {
            let snapshot = self.decode_record(&record)?;
            if let Some(rolling) = &snapshot.rolling {
                let path = no_symlink_parents(&self.directory, &rolling.metadata)?;
                if directory_id(&path)? != rolling.metadata_identity {
                    return Err("rolling directory metadata identity changed".into());
                }
                return Ok(path);
            }
        }
        no_symlink_parents(&self.directory, object)
    }

    fn load_owned(&mut self, object: &str) -> Result<usize, String> {
        let path = no_symlink_parents(&self.directory, &Self::record_name(object)?)?;
        let record = self.decode_record(&path)?;
        if record.path != object {
            return Err("producer record addresses a different allocation".into());
        }
        self.object_guard.as_ref().unwrap().stable()?;
        self.owner_stable()?;
        self.state.objects.push(record);
        Ok(0)
    }

    // This directory contains only explicit producer-written registrations. The
    // consumer reads their declared ownership; it never scans artifact trees or
    // infers membership, references or deletion eligibility from a filename.
    fn records(&self) -> Result<(Vec<Arc<Object>>, Vec<Value>), String> {
        self.owner_stable()?;
        let directory = no_symlink_parents(&self.directory, "producer-records")?;
        let mut records = vec![];
        let mut errors = vec![];
        for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            match self.decode_record(&entry.path()) {
                Ok(object) => records.push(object),
                Err(error) => errors.push(value!({"producer":self.owner.producer,
                    "registration":entry.path(),"status":"failed","error":error,"evidence":"retained"})),
            }
        }
        records.sort_by(|a, b| a.path.cmp(&b.path));
        self.owner_stable()?;
        Ok((records, errors))
    }

    fn stable(&self, object: &Object) -> Result<(), String> {
        self.owner_stable()?;
        self.object_guard
            .as_ref()
            .ok_or("producer object has no metadata exclusion")?
            .stable()?;
        Self::stable_in(
            &self.directory,
            &self.state.producer,
            &self.state.directory_id,
            object,
        )
    }

    fn stable_in(
        directory: &Path,
        producer: &str,
        directory_id_expected: &str,
        object: &Object,
    ) -> Result<(), String> {
        if directory_id(directory)? != directory_id_expected {
            return Err("producer artifact identity changed; preserve object".into());
        }
        rolling::physical_stable(directory, object)?;
        let intent = no_symlink_parents(directory, &object.intent.path)?;
        let bytes = fs::read(intent).map_err(|e| e.to_string())?;
        let intent = json(&bytes)?;
        if sha256(&bytes) != object.intent.sha256
            || intent["path"] != object.path
            || intent["identity"] != object.identity
            || intent["producer"] != producer
            || intent["rolling_directory"].as_bool().unwrap_or(false) != object.rolling.is_some()
            || object
                .rolling
                .as_ref()
                .is_some_and(|r| intent["born_at"] != r.born_at)
            || (object.cache_lease.is_some() && intent["cache_lease"] != value!(object.cache_lease))
        {
            return Err("producer intent changed; preserve object".into());
        }
        if object.sealed {
            let publication = object
                .publication
                .as_ref()
                .ok_or("producer publication is missing")?;
            let bytes = fs::read(no_symlink_parents(directory, &publication.path)?)
                .map_err(|e| e.to_string())?;
            if sha256(&bytes) != publication.sha256
                || json(&bytes)?
                    != value!({
                        "schema":"chrono-retained-artifact-publication/v1","intent":object.intent,
                        "generated":object.generated,"evidence":object.evidence
                    })
            {
                return Err("producer publication identity changed".into());
            }
        }
        for (consumer, reference) in &object.consumers {
            if let Some(receipt) = &reference.receipt {
                let bytes = fs::read(no_symlink_parents(directory, &receipt.path)?)
                    .map_err(|e| e.to_string())?;
                if sha256(&bytes) != receipt.sha256
                    || json(&bytes)?
                        != value!({
                            "schema":"chrono-retained-artifact-reference/v1","path":object.path,
                            "identity":object.identity,"intent":object.intent,"consumer":consumer,
                            "released":reference.released,"reason":reference.reason
                        })
                {
                    return Err("producer consumer state differs from its publication".into());
                }
            } else if reference.released || intent["consumer"] != *consumer {
                return Err(
                    "producer consumer has no explicit reference/release publication".into(),
                );
            }
        }
        for (path, observed) in &object.evidence {
            let bytes = fs::read(no_symlink_parents(&directory.join(&object.path), path)?)
                .map_err(|e| e.to_string())?;
            if observed != &value!({"sha256":sha256(&bytes),"length":bytes.len()}) {
                return Err("producer original evidence changed; preserve object".into());
            }
        }
        Ok(())
    }

    fn save(&mut self, changed: usize) -> Result<(), String> {
        self.owner_stable()?;
        self.object_guard
            .as_ref()
            .ok_or("producer object has no metadata exclusion")?
            .stable()?;
        if changed != 0 || self.state.objects.len() != 1 {
            return Err("producer save requires one exact owned object".into());
        }
        let object = &self.state.objects[changed];
        let path = no_symlink_parents(&self.directory, &Self::record_name(&object.path)?)?;
        let publication = ObjectRecord {
            schema: "chrono-retained-producer-object/v1".into(),
            owner_sha256: sha256(&self.owner_bytes),
            object: object.as_ref(),
        };
        let bytes = serde_json::to_vec(&publication).map_err(|e| e.to_string())?;
        write_bytes_staged(&path, &bytes, false, &self.directory)
    }

    fn receipt(&self, prefix: &str, value: &Value) -> Result<Receipt, String> {
        Self::receipt_in(&self.directory, prefix, value)
    }

    fn receipt_in(directory: &Path, prefix: &str, value: &Value) -> Result<Receipt, String> {
        let name = format!(
            "{prefix}-{}.json",
            sha256(&serde_json::to_vec(value).map_err(|e| e.to_string())?)
        );
        let physical = no_symlink_parents(directory, &name)?;
        let bytes = match fs::read(&physical) {
            Ok(bytes) if json(&bytes)? == *value => bytes,
            Ok(_) => return Err("producer receipt collision".into()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                write_json(&physical, value, true)?
            }
            Err(e) => return Err(e.to_string()),
        };
        Ok(Receipt {
            path: name,
            sha256: sha256(&bytes),
        })
    }
}

impl Manager {
    pub(in crate::automatic) fn partition_producer(
        &self,
        r: &mut Runner,
        target: &Path,
        producer: &str,
        report: &mut Value,
    ) -> Result<(), String> {
        if target != self.inputs.policy.coordinator_root {
            return Err("producer partition requires the surviving coordinator".into());
        }
        let i = self
            .current_entry(target)
            .ok_or("producer partition requires current enrollment")?;
        let entry = &self.ledger.entries[i];
        let lease = self
            .lease(entry, true)?
            .ok_or("producer partition has live coordinator consumers")?;
        self.check_entry(r, entry, &self.inputs.anchor_head, None)?;
        let policy = self
            .inputs
            .policy
            .retained_producers
            .iter()
            .find(|p| p.id == producer)
            .ok_or("producer partition is not explicitly adopted")?;
        self.stable(r)?;
        report["producer_partition"] = Store::partition(
            &policy.directory(target)?,
            producer,
            &self.inputs.anchor_head,
        )?;
        lease.stable()?;
        self.stable(r)
    }
}

/// Handle to one exact producer artifact; dropping it never publishes release.
pub struct RetainedArtifact {
    directory: PathBuf,
    producer: String,
    object: String,
}

fn generated_paths(paths: &[String]) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for path in paths {
        if !path.ends_with('/') || !seen.insert(path) {
            return Err("generated producer outputs require unique literal directories".into());
        }
        relative_path(path.trim_end_matches('/'))?;
        if Path::new(path)
            .components()
            .any(|part| part.as_os_str() == ".git")
        {
            return Err("generated outputs cannot include Git identity or objects".into());
        }
        if [
            "stdout.bin",
            "stderr.bin",
            "binding.json",
            "configuration",
            ".chrono-harness/state/command-captures",
        ]
        .iter()
        .any(|p| {
            Path::new(p).starts_with(path.trim_end_matches('/')) || Path::new(path).starts_with(p)
        }) || paths.iter().any(|p| p != path && path.starts_with(p))
        {
            return Err(
                "generated producer outputs overlap protected evidence or one another".into(),
            );
        }
    }
    Ok(())
}

impl RetainedArtifact {
    /// Test-only interruption seam for the exact conversion producer. The
    /// public lifecycle entry separately verifies coordinator enrollment/policy.
    #[cfg(feature = "producer-birth-test")]
    #[doc(hidden)]
    pub fn partition_with_observer(
        directory: &Path,
        producer: &str,
        candidate: &str,
        observer: impl FnMut(&str) -> Result<(), String>,
    ) -> Result<Value, String> {
        Store::partition_observed(directory, producer, candidate, observer)
    }

    /// Resolve publication through the same committed policy and coordinator as drain.
    /// This does not release consumers, drain stores, or authorize checkout disposal.
    pub fn registered_store(
        host_root: &Path,
        config_path: &str,
        producer: &str,
    ) -> Result<(PathBuf, Vec<String>), String> {
        Self::resolve_store(host_root, config_path, producer, false)
    }

    fn resolve_store(
        host_root: &Path,
        config_path: &str,
        producer: &str,
        temporary: bool,
    ) -> Result<(PathBuf, Vec<String>), String> {
        let root = fs::canonicalize(host_root).map_err(|e| e.to_string())?;
        let (config, bytes) = crate::configuration(&root, config_path)?;
        let mut selected = None;
        let report = start::with_report(
            &root,
            config_path,
            config,
            &bytes,
            "producer-store",
            "resolved",
            |r, _token, report| {
                let inputs = PolicyInputs::load(r, &root, config_path, &bytes)?
                    .ok_or("retained producer owner is not opted in")?;
                let declaration = inputs
                    .anchor_policy
                    .retained_producers
                    .iter()
                    .find(|p| p.id == producer)
                    .ok_or("retained producer store is not declared by its owner")?;
                if temporary && !declaration.temporary {
                    return Err("temporary producer is not prospectively adopted".into());
                }
                let directory = declaration.directory(&inputs.policy.coordinator_root)?;
                inputs.anchor_stable(r)?;
                report["producer"] = value!(producer);
                report["coordinator_root"] = value!(inputs.policy.coordinator_root);
                report["directory"] = value!(directory);
                report["generated_outputs"] = value!(declaration.generated_outputs);
                selected = Some((directory, declaration.generated_outputs.clone()));
                Ok(())
            },
        )?;
        if report["status"] != "resolved" {
            return Err(report["error"]
                .as_str()
                .unwrap_or("retained producer store resolution failed")
                .into());
        }
        selected.ok_or("retained producer store was not resolved".into())
    }

    pub fn begin(
        directory: &Path,
        producer: &str,
        artifact: &Path,
        generated_outputs: &[String],
        consumer: &str,
        retention_reason: &str,
    ) -> Result<Self, String> {
        Self::begin_observed(
            directory,
            producer,
            artifact,
            generated_outputs,
            consumer,
            retention_reason,
            |_| {},
        )
    }

    /// Test-only scheduling seam; runs the same birth and registration code.
    #[cfg(feature = "producer-birth-test")]
    #[doc(hidden)]
    pub fn begin_with_birth_observer(
        directory: &Path,
        producer: &str,
        artifact: &Path,
        generated_outputs: &[String],
        consumer: &str,
        retention_reason: &str,
        observer: impl FnMut(&str),
    ) -> Result<Self, String> {
        Self::begin_observed(
            directory,
            producer,
            artifact,
            generated_outputs,
            consumer,
            retention_reason,
            observer,
        )
    }

    fn begin_observed(
        directory: &Path,
        producer: &str,
        artifact: &Path,
        generated_outputs: &[String],
        consumer: &str,
        retention_reason: &str,
        mut observer: impl FnMut(&str),
    ) -> Result<Self, String> {
        generated_paths(generated_outputs)?;
        if consumer.is_empty() || retention_reason.is_empty() {
            return Err("producer requires a consumer and retention reason".into());
        }
        let mut store = Store::open_observed(directory, producer, true, &mut observer)?;
        let object = artifact
            .strip_prefix(directory)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("producer path UTF-8")?
            .to_owned();
        relative_path(&object)?;
        if Path::new(&object).components().count() != 1 || store.contains(&object)? {
            return Err("producer artifact must be one new exact child directory".into());
        }
        let identity = directory_id(&no_symlink_parents(directory, &object)?)?;
        store.bind(&object)?;
        if store.contains(&object)? {
            return Err("producer artifact is already registered".into());
        }
        observer("object-record-admitted");
        let intent = store.receipt(
            "producer-intent",
            &value!({
                "schema":"chrono-retained-artifact-intent/v1","producer":producer,
                "path":object,"identity":identity,"generated_outputs":generated_outputs,
                "consumer":consumer,"retention_reason":retention_reason,
                "evidence_disposal":"not-authorized"
            }),
        )?;
        store.state.objects.push(Arc::new(Object {
            path: object.clone(),
            identity,
            intent,
            sealed: false,
            publication: None,
            generated: BTreeMap::new(),
            evidence: BTreeMap::new(),
            consumers: BTreeMap::from([(
                consumer.into(),
                Reference {
                    released: false,
                    reason: retention_reason.into(),
                    receipt: None,
                },
            )]),
            retention_reason: retention_reason.into(),
            disposed: false,
            attempts: vec![],
            cache_lease: None,
            bodies: BTreeMap::new(),
            rolling: None,
        }));
        store.save(store.state.objects.len() - 1)?;
        Ok(Self {
            directory: directory.into(),
            producer: producer.into(),
            object,
        })
    }

    pub fn resume(directory: &Path, producer: &str, artifact: &Path) -> Result<Self, String> {
        let mut store = Store::open(directory, producer, false)?;
        let object = artifact
            .strip_prefix(directory)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("producer path UTF-8")?
            .to_owned();
        store.load(&object)?;
        let record = store
            .state
            .objects
            .iter()
            .find(|o| o.path == object)
            .ok_or("unknown producer artifact; preserve it")?;
        store.stable(record)?;
        Ok(Self {
            directory: directory.into(),
            producer: producer.into(),
            object,
        })
    }

    /// Publish identities only after the producer has retained its original bytes.
    pub fn seal(&self) -> Result<(), String> {
        let mut store = Store::open(&self.directory, &self.producer, false)?;
        store.load(&self.object)?;
        let i = store
            .state
            .objects
            .iter()
            .position(|o| o.path == self.object)
            .ok_or("producer object missing")?;
        let object = &store.state.objects[i];
        store.stable(object)?;
        if object.sealed || object.disposed || object.cache_lease.is_some() {
            return Err("producer artifact is already sealed/disposed".into());
        }
        let intent: Value =
            json(&fs::read(self.directory.join(&object.intent.path)).map_err(|e| e.to_string())?)?;
        let paths: Vec<String> = serde_json::from_value(intent["generated_outputs"].clone())
            .map_err(|e| e.to_string())?;
        let root = no_symlink_parents(&self.directory, &self.object)?;
        let mut generated = BTreeMap::new();
        for path in paths {
            let physical = no_symlink_parents(&root, path.trim_end_matches('/'))?;
            match fs::symlink_metadata(&physical) {
                Ok(m) if m.is_dir() => {
                    generated.insert(path, directory_id(&physical)?);
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(e.to_string()),
                Ok(_) => return Err("generated receipt output is not a physical directory".into()),
            }
        }
        let mut evidence = BTreeMap::new();
        for name in ["stdout.bin", "stderr.bin", "binding.json"] {
            let bytes = fs::read(no_symlink_parents(&root, name)?).map_err(|e| e.to_string())?;
            evidence.insert(
                name.into(),
                value!({"sha256":sha256(&bytes),"length":bytes.len()}),
            );
        }
        let publication = store.receipt(
            "producer-publication",
            &value!({
                "schema":"chrono-retained-artifact-publication/v1","intent":object.intent,
                "generated":generated,"evidence":evidence
            }),
        )?;
        let object = Arc::make_mut(&mut store.state.objects[i]);
        object.publication = Some(publication);
        object.generated = generated;
        object.evidence = evidence;
        object.sealed = true;
        store.save(i)
    }

    pub fn acquire(&self, consumer: &str, reason: &str) -> Result<(), String> {
        self.reference(consumer, reason, false)
    }

    pub fn release(&self, consumer: &str, reason: &str) -> Result<(), String> {
        self.reference(consumer, reason, true)
    }

    fn reference(&self, consumer: &str, reason: &str, released: bool) -> Result<(), String> {
        if consumer.is_empty() || reason.is_empty() {
            return Err("consumer/release reason required".into());
        }
        let mut store = Store::open(&self.directory, &self.producer, false)?;
        store.load(&self.object)?;
        let i = store
            .state
            .objects
            .iter()
            .position(|o| o.path == self.object)
            .ok_or("producer object missing")?;
        let object = &store.state.objects[i];
        store.stable(object)?;
        let _cache = object
            .cache_lease
            .as_ref()
            .map(|binding| {
                Lease::acquire(
                    &no_symlink_parents(&self.directory, &binding.path)?,
                    Some(&binding.id),
                    false,
                    false,
                    None,
                )?
                .ok_or_else(|| {
                    "temporary cache recovery is active; reference not acquired".to_string()
                })
            })
            .transpose()?;
        if (!object.sealed && object.cache_lease.is_none()) || (!released && object.disposed) {
            return Err(
                "unsealed/disposed producer artifact cannot acquire or release consumers".into(),
            );
        }
        if released && !object.consumers.contains_key(consumer) {
            return Err("unknown consumer cannot publish released state".into());
        }
        let receipt = store.receipt(
            "producer-reference",
            &value!({"schema":"chrono-retained-artifact-reference/v1",
            "path":object.path,"identity":object.identity,"intent":object.intent,
            "consumer":consumer,"released":released,"reason":reason}),
        )?;
        Arc::make_mut(&mut store.state.objects[i]).consumers.insert(
            consumer.into(),
            Reference {
                released,
                reason: reason.into(),
                receipt: Some(receipt),
            },
        );
        store.save(i)
    }
}

/// One prospectively enrolled temporary allocation. Source and originals stay
/// in place after Drop or interruption; only its declared outputs enter drain.
/// Native consumers use bind_command; engine consumers inherit the same Scope.
pub struct TemporaryHost {
    path: PathBuf,
    lease: Option<Lease>,
    owner: Option<rolling::Owner>,
}

impl TemporaryHost {
    pub fn registered_store(
        host_root: &Path,
        config_path: &str,
        producer: &str,
    ) -> Result<(PathBuf, Vec<String>), String> {
        RetainedArtifact::resolve_store(host_root, config_path, producer, true)
    }

    pub fn allocate(
        directory: &Path,
        producer: &str,
        generated_outputs: &[String],
        prefix: &str,
    ) -> Result<Self, String> {
        Self::allocate_owned(directory, producer, generated_outputs, prefix, None)
    }

    fn allocate_owned(
        directory: &Path,
        producer: &str,
        generated_outputs: &[String],
        prefix: &str,
        owner: Option<rolling::Owner>,
    ) -> Result<Self, String> {
        generated_paths(generated_outputs)?;
        if prefix.contains(['/', '\\', '\0']) {
            return Err("temporary allocation prefix must be one literal name".into());
        }
        fs::create_dir_all(directory).map_err(|e| e.to_string())?;
        let mut store = Store::open(directory, producer, true)?;
        // Keep immediately: even a failed/interrupted enrollment is an unknown
        // neighbor, never an implicitly reclaimable source or evidence object.
        let path = tempfile::Builder::new()
            .prefix(prefix)
            .tempdir_in(directory)
            .map_err(|e| e.to_string())?
            .keep();
        let object = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("allocation UTF-8")?
            .to_owned();
        let identity = directory_id(&path)?;
        let rolling = if owner.is_some() {
            let metadata = format!("rolling-metadata/{}", sha256(object.as_bytes()));
            let physical = no_symlink_parents(directory, &metadata)?;
            fs::create_dir_all(physical.parent().unwrap()).map_err(|e| e.to_string())?;
            fs::create_dir(&physical).map_err(|e| e.to_string())?;
            store.object_guard = Some(DirectoryGate::acquire(
                &physical,
                store.began,
                std::time::Duration::from_secs(30),
                &mut |_| {},
            )?);
            Some(rolling::Directory {
                metadata,
                metadata_identity: directory_id(&physical)?,
                born_at: rolling::now()?,
                diagnostics: false,
                terminal: None,
                disposal_intent: None,
                disposed: false,
                disposed_at: None,
            })
        } else {
            store.bind(&object)?;
            None
        };
        let lease_path = rolling
            .as_ref()
            .map(|r| format!("{}/allocation.lease", r.metadata))
            .unwrap_or_else(|| format!("allocation-{}.lease", sha256(object.as_bytes())));
        let lease = Lease::acquire(
            &no_symlink_parents(directory, &lease_path)?,
            None,
            true,
            false,
            None,
        )?
        .ok_or("allocation lease unavailable")?;
        let binding = CacheLease {
            path: lease_path,
            id: lease.id().into(),
        };
        let intent = store.receipt(
            &rolling.as_ref().map(|r| format!("{}/producer-intent", r.metadata))
                .unwrap_or_else(|| "producer-intent".into()),
            &value!({
                "schema":"chrono-temporary-host-intent/v1","producer":producer,
                "path":object,"identity":identity,"generated_outputs":generated_outputs,
                "cache_lease":binding,"evidence_disposal":if rolling.is_some() { "terminal-handoff-and-retention" } else { "not-authorized" },
                "rolling_directory":rolling.is_some(),
                "born_at":rolling.as_ref().map(|r| r.born_at),
                "producer_outcome":"not-established","task_completion":"not-claimed"
            }),
        )?;
        store.state.objects.push(Arc::new(Object {
            path: object,
            identity,
            intent,
            sealed: false,
            publication: None,
            generated: BTreeMap::new(),
            evidence: BTreeMap::new(),
            consumers: BTreeMap::new(),
            retention_reason:
                "prospective temporary source and original evidence; cache-only recovery".into(),
            disposed: false,
            attempts: vec![],
            cache_lease: Some(binding),
            bodies: BTreeMap::new(),
            rolling,
        }));
        store.save(store.state.objects.len() - 1)?;
        Ok(Self {
            path,
            lease: Some(lease),
            owner,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// A declared cross-thread engine consumer enters its own process_fds Scope
    /// with this owned copy. It shares the existing kernel open description.
    pub fn capability(&self) -> Result<fs::File, String> {
        let lease = self
            .lease
            .as_ref()
            .ok_or("temporary allocation was handed off")?;
        lease.stable()?;
        lease.capability()
    }

    pub fn bind_command(&self, command: &mut std::process::Command) -> Result<(), String> {
        self.lease
            .as_ref()
            .ok_or("temporary allocation was handed off")?
            .stable()?;
        // The same exact allocation also holds nested default tempfile outputs.
        // It is retained source/evidence, never a disposable mixed TMPDIR root.
        command.env("TMPDIR", &self.path);
        // Lease is !Send and already owns a scope in this thread. Forward it
        // once; adding the explicit lease here duplicates that same capability.
        chrono_harness::process_fds::forward_command(command, &[])
    }

    /// Native fixture output is written before join, independently of Drop.
    /// Captures remain evidence; a missing result never establishes completion.
    pub fn capture_output(
        &self,
        command: &mut std::process::Command,
    ) -> Result<std::process::Output, String> {
        self.bind_command(command)?;
        Self::capture_bound_output(&self.path, command)
    }

    /// Capture an already bound native route in its explicit retained host.
    /// The caller supplies that host's held capabilities and environment; this
    /// adapter records IO and actual join, and establishes no lifecycle custody.
    pub fn capture_bound_output(
        root: &Path,
        command: &mut std::process::Command,
    ) -> Result<std::process::Output, String> {
        Self::capture_native(root, command, None)
    }

    /// Preserve an actual native protocol request before spawn as well as its
    /// original streams. A regular input file provides the same bytes and EOF.
    pub fn capture_bound_input(
        root: &Path,
        command: &mut std::process::Command,
        input: &[u8],
    ) -> Result<std::process::Output, String> {
        Self::capture_native(root, command, Some(input))
    }

    fn capture_native(
        root: &Path,
        command: &mut std::process::Command,
        input: Option<&[u8]>,
    ) -> Result<std::process::Output, String> {
        let directory = root.join(".chrono-harness/state/command-captures");
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let capture = tempfile::Builder::new()
            .prefix("native-")
            .tempdir_in(&directory)
            .map_err(|e| e.to_string())?
            .keep();
        let stdout = capture.join("stdout.bytes");
        let stderr = capture.join("stderr.bytes");
        let out = fs::File::create_new(&stdout).map_err(|e| e.to_string())?;
        let err = fs::File::create_new(&stderr).map_err(|e| e.to_string())?;
        let stdin = if let Some(bytes) = input {
            let path = capture.join("stdin.bytes");
            let mut file = fs::File::create_new(&path).map_err(|e| e.to_string())?;
            use std::io::Write;
            file.write_all(bytes).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
            std::process::Stdio::from(fs::File::open(path).map_err(|e| e.to_string())?)
        } else {
            std::process::Stdio::null()
        };
        let environment: BTreeMap<_, _> = command
            .get_envs()
            .map(|(k, v)| {
                (
                    k.to_string_lossy().into_owned(),
                    v.map(|v| sha256(v.as_encoded_bytes())),
                )
            })
            .collect();
        write_json(
            &capture.join("intent.json"),
            &value!({
                "schema":"chrono-temporary-native-capture/v1", "program":command.get_program().to_string_lossy(),
                "argv":command.get_args().map(|v| v.to_string_lossy()).collect::<Vec<_>>(),
                "cwd":command.get_current_dir(),"explicit_environment_sha256":environment,
                "stdout":"stdout.bytes","stderr":"stderr.bytes","producer_outcome":"not-established",
                "stdin":input.map(|bytes| value!({"path":"stdin.bytes","sha256":sha256(bytes),"length":bytes.len()})),
                "task_completion":"not-claimed"
            }),
            true,
        )?;
        command.stdin(stdin).stdout(out).stderr(err);
        let span = tracing::info_span!("fixture.native", capture = %capture.display(),
            program = %command.get_program().to_string_lossy(),
            argv = %value!(command.get_args().map(|v| v.to_string_lossy()).collect::<Vec<_>>()),
            launcher_pid = std::process::id(), child_pid = tracing::field::Empty);
        let _entered = span.enter();
        tracing::info!(
            event_id = "fixture.native_spawn",
            "original intent and streams created"
        );
        let mut child = command.spawn().map_err(|e| e.to_string())?;
        span.record("child_pid", child.id());
        let started = write_json(
            &capture.join("started.json"),
            &value!({"child_pid":child.id()}),
            true,
        );
        tracing::info!(
            event_id = "fixture.native_wait",
            "joining actual native child"
        );
        let status = child.wait().map_err(|e| e.to_string())?;
        tracing::info!(event_id = "fixture.native_joined", status = %status, "native child joined");
        started?;
        write_json(
            &capture.join("result.json"),
            &value!({"child_pid":child.id(),
            "status":status.to_string(),"exit_code":status.code(),"joined":true,
            "task_completion":"not-claimed"}),
            true,
        )?;
        let result = std::process::Output {
            status,
            stdout: fs::read(stdout).map_err(|e| e.to_string())?,
            stderr: fs::read(stderr).map_err(|e| e.to_string())?,
        };
        tracing::info!(
            event_id = "fixture.native_return",
            "original result and streams available"
        );
        Ok(result)
    }
}

impl ProducerPolicy {
    pub(super) fn directory(&self, coordinator_root: &Path) -> Result<PathBuf, String> {
        no_symlink_parents(coordinator_root, self.directory.trim_end_matches('/'))
    }

    pub(super) fn validate(&self, policy: &Policy) -> Result<(), String> {
        if self.rolling_directory.is_some() && !self.temporary {
            return Err("rolling directory requires prospective temporary ownership".into());
        }
        if !self.operations.is_empty() && self.rolling_directory.is_none() {
            return Err("operation artifact directories require rolling ownership".into());
        }
        let mut operations = BTreeSet::new();
        for operation in &self.operations {
            if operation.trim().is_empty()
                || !operations.insert(operation)
                || policy
                    .retained_producers
                    .iter()
                    .filter(|producer| producer.operations.contains(operation))
                    .count()
                    != 1
            {
                return Err("artifact operation requires one exact producer".into());
            }
        }
        relative_path(self.directory.trim_end_matches('/'))?;
        if self.id.is_empty()
            || !self.directory.ends_with('/')
            || !self.directory.starts_with(".chrono-harness/state/")
            || self.directory == ".chrono-harness/state/"
            || Path::new(&self.directory).starts_with(&policy.state_directory)
            || Path::new(&policy.state_directory).starts_with(&self.directory)
            || !policy.artifacts.iter().any(|a| {
                a.disposition == Disposition::EvidenceRetain && self.directory.starts_with(&a.path)
            })
        {
            return Err("producer requires a distinct explicit retained evidence directory".into());
        }
        generated_paths(&self.generated_outputs)?;
        let mut roots = BTreeSet::new();
        for root in &self.source_roots {
            if root != "." {
                relative_path(root.trim_end_matches('/'))?;
                if !root.ends_with('/') {
                    return Err("producer source root requires a literal directory".into());
                }
            }
            if !roots.insert(root) {
                return Err("duplicate producer source root".into());
            }
        }
        Ok(())
    }

    fn protect_sources(
        &self,
        r: &mut Runner,
        allocation: &Path,
        outputs: &[String],
    ) -> Result<(), String> {
        for declared in &self.source_roots {
            let names: Vec<_> = outputs
                .iter()
                .filter_map(|p| {
                    if declared == "." {
                        Some(p.clone())
                    } else {
                        p.strip_prefix(declared).map(str::to_owned)
                    }
                })
                .filter(|p| {
                    allocation
                        .join(if declared == "." { "" } else { declared })
                        .join(p)
                        .exists()
                })
                .collect();
            if names.is_empty() {
                continue;
            }
            let root = if declared == "." {
                allocation.to_owned()
            } else {
                no_symlink_parents(allocation, declared.trim_end_matches('/'))?
            };
            // A missing nested repository must not fall through to a parent Git
            // root. Only the explicit native consumer's root supplies source facts.
            if !root.join(".git").exists() {
                return Err(format!(
                    "declared producer source root is unavailable: {declared}"
                ));
            }
            let observed = PathBuf::from(
                String::from_utf8(r.git(&root, &["rev-parse", "--show-toplevel"])?)
                    .map_err(|e| e.to_string())?
                    .trim(),
            );
            if fs::canonicalize(&observed).map_err(|e| e.to_string())?
                != fs::canonicalize(&root).map_err(|e| e.to_string())?
            {
                return Err("producer source root identity differs".into());
            }
            let head = r.oid(&root, "HEAD")?;
            artifact_disposal::paths(r, &root, &head, &names)?;
        }
        Ok(())
    }

    pub(super) fn drain(
        &self,
        inputs: &PolicyInputs,
        r: &mut Runner,
        report: &mut Value,
    ) -> Result<(), String> {
        self.drain_selected(inputs, r, report, None)
    }

    fn drain_selected(
        &self,
        inputs: &PolicyInputs,
        r: &mut Runner,
        report: &mut Value,
        selected: Option<&str>,
    ) -> Result<(), String> {
        let root = &inputs.policy.coordinator_root;
        let directory = self.directory(root)?;
        if !directory.exists() || !no_symlink_parents(&directory, "objects.json")?.exists() {
            report["producer_objects"].as_array_mut().unwrap().push(value!({"producer":self.id,
                "status":"protected-unknown","reason":"no explicit producer registry; historical objects remain untouched"}));
            return Ok(());
        }
        if selected.is_none() {
            rolling::recover(self, inputs, r, &directory, report)?;
        }
        let mut opened = Store::open(&directory, &self.id, false)?;
        let (objects, registration_errors) = if let Some(selected) = selected {
            // finish joins its exact allocation's existing bounded admission.
            // Broad maintenance still skips unrelated busy addresses below.
            opened.load(selected)?;
            (opened.state.objects.clone(), vec![])
        } else {
            opened.records()?
        };
        let registry_id = opened.state.directory_id.clone();
        let mut store = Some(opened);
        let mut failures = vec![];
        for error in registration_errors {
            failures.push(error["error"].to_string());
            report["producer_objects"]
                .as_array_mut()
                .unwrap()
                .push(error);
        }
        let mut originals = BTreeMap::new();
        for snapshot in &objects {
            let excluded = (|| -> Result<Option<Lease>, String> {
                if let Some(binding) = &snapshot.cache_lease {
                    let path = no_symlink_parents(&directory, &binding.path)?;
                    Lease::acquire(&path, Some(&binding.id), false, true, None)
                } else {
                    Ok(None)
                }
            })();
            let cache_guard = match excluded {
                Ok(guard) => guard,
                Err(error) => {
                    let path = format!("{}{}", self.directory, snapshot.path);
                    failures.push(format!("{path}: {error}"));
                    report["producer_objects"]
                        .as_array_mut()
                        .unwrap()
                        .push(value!({
                            "producer":self.id,"path":path,"status":"failed","error":error,
                            "evidence":"retained","consumers":snapshot.consumers
                        }));
                    drop(store.take());
                    continue;
                }
            };
            let owned = if !snapshot.disposed
                && (snapshot.cache_lease.is_none() || cache_guard.is_some())
            {
                if store.is_none() {
                    store = Some(Store::open(&directory, &self.id, false)?);
                }
                let current = store.as_mut().unwrap();
                let loaded = current.load_available(&snapshot.path);
                if matches!(loaded, Ok(false)) {
                    report["producer_objects"]
                        .as_array_mut()
                        .unwrap()
                        .push(value!({
                            "producer":self.id,"path":format!("{}{}",self.directory,snapshot.path),
                            "status":"protected","reason":"live allocation metadata transaction"
                        }));
                    drop(store.take());
                    continue;
                }
                if let Err(error) = loaded {
                    failures.push(format!("{}: {error}", snapshot.path));
                    report["producer_objects"]
                        .as_array_mut()
                        .unwrap()
                        .push(value!({
                            "producer":self.id,"path":format!("{}{}",self.directory,snapshot.path),
                            "status":"failed","error":error,"evidence":"retained"
                        }));
                    drop(store.take());
                    continue;
                }
                let object = current
                    .state
                    .objects
                    .iter()
                    .find(|o| o.path == snapshot.path)
                    .ok_or("producer object missing during recovery")?;
                if object.identity != snapshot.identity
                    || value!(object.intent) != value!(snapshot.intent)
                    || value!(object.cache_lease) != value!(snapshot.cache_lease)
                {
                    return Err("producer allocation binding changed; preserve originals".into());
                }
                if let Err(error) = current.stable(object) {
                    // This unavailable allocation cannot establish exclusion or
                    // disposal. Preserve its registration and original outcome,
                    // but do not veto checked siblings in the same producer.
                    let path = format!("{}{}", self.directory, object.path);
                    failures.push(format!("{path}: {error}"));
                    report["producer_objects"].as_array_mut().unwrap().push(value!({
                        "producer":self.id,"path":path,"status":"failed","error":error,
                        "retention_reason":object.retention_reason,"consumers":object.consumers,
                        "registered_generated_objects":object.generated.len(),"evidence":"retained"
                    }));
                    if snapshot.cache_lease.is_some() {
                        drop(store.take());
                    }
                    continue;
                }
                object.clone()
            } else {
                snapshot.clone()
            };
            let object = &owned;
            let mut row = value!({"producer":self.id,"path":format!("{}{}",self.directory,object.path),
                "retention_reason":object.retention_reason,"consumers":object.consumers,
                "registered_generated_objects":object.generated.len(),"evidence":"retained"});
            if object.rolling.is_some() {
                drop(store.take());
                match rolling::drain(self, inputs, r, &directory, object, cache_guard.as_ref()) {
                    Ok(result) => {
                        if result["status"] == "failed" {
                            failures.push(format!("{}: {}", object.path, result["error"]));
                        }
                        row["rolling_directory"] = result;
                    }
                    Err(error) => {
                        failures.push(format!("{}: {error}", object.path));
                        row["rolling_directory"] = value!({"status":"failed","error":error});
                    }
                }
                row["status"] = row["rolling_directory"]["status"].clone();
                report["producer_objects"].as_array_mut().unwrap().push(row);
                continue;
            }
            // Body recovery has its own membership and receipts. A historical
            // Cargo disposal bit cannot settle a subsequently adopted body.
            if !object.bodies.is_empty() {
                if let Some(lease) = cache_guard
                    .as_ref()
                    .filter(|_| object.consumers.values().all(|consumer| consumer.released))
                {
                    drop(store.take());
                    let bodies = body::drain(
                        self,
                        inputs,
                        r,
                        &directory,
                        object,
                        &registry_id,
                        lease,
                        &mut originals,
                    )?;
                    if bodies["status"] == "failed" {
                        failures.push(format!("{}: {}", object.path, bodies["failures"]));
                    }
                    row["fixture_bodies"] = bodies;
                } else {
                    row["fixture_bodies"] = value!({"status":"protected",
                        "reason":"live supported holder or unreleased actual consumer"});
                }
            }
            if (!object.sealed && object.cache_lease.is_none())
                || object.consumers.values().any(|c| !c.released)
                || (object.cache_lease.is_some() && cache_guard.is_none())
            {
                row["status"] = value!("protected");
                row["reason"] = value!(
                    "live kernel holder, unreleased diagnostic consumer, or interrupted retained publication"
                );
            } else if object.disposed {
                row["status"] = value!("already-disposed");
            } else {
                let mut run = || -> Result<Option<Value>, String> {
                    Store::stable_in(&directory, &self.id, &registry_id, object)?;
                    let prospective: Vec<String> = if object.cache_lease.is_some() {
                        if !self.temporary {
                            return Err(
                                "temporary producer is no longer adopted; preserve outputs".into(),
                            );
                        }
                        let intent = json(
                            &fs::read(directory.join(&object.intent.path))
                                .map_err(|e| e.to_string())?,
                        )?;
                        serde_json::from_value(intent["generated_outputs"].clone())
                            .map_err(|e| e.to_string())?
                    } else {
                        object.generated.keys().cloned().collect()
                    };
                    generated_paths(&prospective)?;
                    if prospective
                        .iter()
                        .any(|p| !self.generated_outputs.contains(p))
                    {
                        return Err("producer output is not in the current exact whitelist".into());
                    }
                    if object.cache_lease.is_some() {
                        let mut absent = true;
                        for output in &prospective {
                            let path = no_symlink_parents(
                                &directory.join(&object.path),
                                output.trim_end_matches('/'),
                            )?;
                            match fs::symlink_metadata(path) {
                                Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                                Err(error) => return Err(error.to_string()),
                                Ok(_) => absent = false,
                            }
                        }
                        if absent {
                            // No producer state or success publication is needed for
                            // a complete, excluded output set that is actually absent.
                            // Keep the short registry read transaction for the next
                            // observation instead of reloading it for each empty row.
                            row["absent_outputs"] = value!(prospective);
                            return Ok(None);
                        }
                        // The exact allocation EX lease excludes reference acquisition;
                        // unrelated allocations may publish during source checks/removal.
                        drop(store.take());
                    }
                    let names: Vec<_> = prospective
                        .iter()
                        .map(|p| format!("{}{}/{p}", self.directory, object.path))
                        .collect();
                    let allocation = directory.join(&object.path);
                    let mut final_original_guards = Vec::new();
                    for git_root in &self.source_roots {
                        let host = if git_root == "." {
                            allocation.clone()
                        } else {
                            no_symlink_parents(&allocation, git_root)?
                        };
                        if !host.exists() {
                            continue;
                        }
                        let paths = prospective
                            .iter()
                            .filter_map(|output| {
                                let destination = allocation.join(output.trim_end_matches('/'));
                                destination
                                    .strip_prefix(&host)
                                    .ok()
                                    .and_then(|relative| relative.to_str())
                                    .filter(|relative| !relative.is_empty())
                                    .map(str::to_owned)
                            })
                            .collect::<Vec<_>>();
                        final_original_guards.push(
                            chrono_harness::retained_artifacts::retention::external_removal_guards(
                                &host, &paths,
                            )?,
                        );
                    }
                    self.protect_sources(r, &allocation, &prospective)?;
                    let head = if names.iter().all(|p| !root.join(p).exists()) {
                        None
                    } else {
                        Some(r.oid(root, "HEAD")?)
                    };
                    let paths = if let Some(head) = &head {
                        artifact_disposal::paths(r, root, head, &names)?
                    } else {
                        names
                            .iter()
                            .map(|p| no_symlink_parents(root, p.trim_end_matches('/')))
                            .collect::<Result<Vec<_>, _>>()?
                    };
                    let intent = Store::receipt_in(
                        &directory,
                        "producer-cleanup-intent",
                        &value!({
                            "schema":"chrono-retained-artifact-cleanup-intent/v1","path":object.path,
                            "identity":object.identity,"producer_intent":object.intent,"generated":object.generated,
                            "policy_sha256":sha256(&inputs.anchor_policy_bytes),"released_consumers":object.consumers,
                            "cache_lease":object.cache_lease,"prospective_outputs":prospective,
                            "producer_outcome":"not-established-by-cache-recovery"
                        }),
                    )?;
                    let mut attempt = value!({"automatic_cleanup":true,"intent":intent,"artifact_disposals":[],"status":"failed"});
                    let outcome =
                        artifact_disposal::dispose(root, &names, &paths, &mut attempt, || {
                            inputs.anchor_stable(r)?;
                            Store::stable_in(&directory, &self.id, &registry_id, object)?;
                            if let Some(lease) = &cache_guard {
                                lease.stable()?;
                            }
                            if let Some(head) = &head {
                                artifact_disposal::paths(r, root, head, &names)?;
                            }
                            self.protect_sources(r, &allocation, &prospective)?;
                            for guard in final_original_guards.iter().flatten() {
                                guard.stable()?;
                            }
                            for (path, expected) in &object.generated {
                                let physical = no_symlink_parents(
                                    &directory.join(&object.path),
                                    path.trim_end_matches('/'),
                                )?;
                                match fs::symlink_metadata(&physical) {
                                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                                    Err(e) => return Err(e.to_string()),
                                    Ok(_) if directory_id(&physical)? == *expected => (),
                                    Ok(_) => {
                                        return Err(
                                            "generated producer artifact identity changed".into()
                                        );
                                    }
                                }
                            }
                            Ok(())
                        });
                    match outcome {
                        Ok(()) => attempt["status"] = value!("cleaned"),
                        Err(error) => attempt["error"] = value!(error),
                    }
                    let receipt =
                        Store::receipt_in(&directory, "producer-cleanup-result", &attempt)?;
                    attempt["receipt"] = value!(receipt);
                    Ok(Some(attempt))
                };
                match run() {
                    Ok(None) => row["status"] = value!("already-absent"),
                    Ok(Some(attempt)) => {
                        if store.is_none() {
                            store = Some(Store::open(&directory, &self.id, false)?);
                        }
                        let current = store.as_mut().unwrap();
                        current.load(&object.path)?;
                        let i = current
                            .state
                            .objects
                            .iter()
                            .position(|o| o.path == object.path)
                            .ok_or("producer object missing during recovery")?;
                        if value!(current.state.objects[i]) != value!(object) {
                            return Err(
                                "producer registration changed during recovery; preserve originals"
                                    .into(),
                            );
                        }
                        let updated = Arc::make_mut(&mut current.state.objects[i]);
                        updated.attempts.push(
                            serde_json::from_value(attempt["receipt"].clone())
                                .map_err(|e| e.to_string())?,
                        );
                        updated.disposed = attempt["status"] == "cleaned";
                        if attempt["status"] != "cleaned" {
                            failures.push(attempt["error"].to_string());
                        }
                        row["status"] = attempt["status"].clone();
                        row["cleanup"] = attempt;
                        current.save(i)?;
                        if object.cache_lease.is_some() {
                            drop(store.take());
                        }
                    }
                    Err(error) => {
                        failures.push(error.clone());
                        row["status"] = value!("failed");
                        row["error"] = value!(error);
                    }
                }
            }
            if object.cache_lease.is_some()
                && row["status"] != "already-absent"
                && row["status"] != "already-disposed"
            {
                drop(store.take());
            }
            drop(store.take());
            report["producer_objects"].as_array_mut().unwrap().push(row);
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures.join("; "))
        }
    }
}
