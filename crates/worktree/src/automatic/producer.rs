//! Explicit producer records consumed by the existing lifecycle drain. No discovery.
use super::*;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProducerPolicy {
    pub(super) id: String,
    pub(super) directory: String,
    pub(super) generated_outputs: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reference {
    released: bool,
    reason: String,
    receipt: Option<Receipt>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Object {
    path: String,
    identity: String,
    intent: Receipt,
    sealed: bool,
    publication: Option<Receipt>,
    generated: BTreeMap<String, String>,
    evidence: BTreeMap<String, Value>,
    consumers: BTreeMap<String, Reference>,
    retention_reason: String,
    disposed: bool,
    attempts: Vec<Receipt>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    schema: String,
    producer: String,
    directory_id: String,
    lease_id: String,
    objects: Vec<Object>,
}

struct Store {
    directory: PathBuf,
    registry: Registry,
    lease: Lease,
}

// The physical store already exists. Lock it before examining birth state, so
// publishing the named lease cannot expose an unlocked, unexplained birth to
// another ordinary publisher. This gate never creates or adopts an owner file.
struct BirthGate {
    _file: fs::File,
    #[cfg(unix)]
    _scope: chrono_harness::process_fds::Scope,
}

impl BirthGate {
    fn acquire(
        directory: &Path,
        began: std::time::Instant,
        bound: std::time::Duration,
        observer: &mut impl FnMut(&str),
    ) -> Result<Self, String> {
        #[cfg(not(unix))]
        {
            let _ = (directory, began, bound, observer);
            Err("kernel ownership is unsupported on this platform".into())
        }
        #[cfg(unix)]
        {
            use std::os::{
                fd::{AsFd, AsRawFd},
                unix::fs::{MetadataExt, OpenOptionsExt},
            };
            let file = fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(directory)
                .map_err(|e| format!("producer birth gate unavailable; preserve objects: {e}"))?;
            let metadata = file.metadata().map_err(|e| e.to_string())?;
            let id = format!("{}:{}", metadata.dev(), metadata.ino());
            loop {
                if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
                    break;
                }
                let error = std::io::Error::last_os_error();
                if error.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                if error.kind() != std::io::ErrorKind::WouldBlock {
                    return Err(error.to_string());
                }
                observer("birth-gate-busy");
                if began.elapsed() >= bound {
                    return Err("producer registry birth remains busy; preserve objects".into());
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            if directory_id(directory)? != id {
                return Err("producer store replaced during birth admission".into());
            }
            let scope = chrono_harness::process_fds::Scope::new(&[file.as_fd()])?;
            Ok(Self {
                _file: file,
                _scope: scope,
            })
        }
    }
}

impl Store {
    fn open(directory: &Path, producer: &str, create: bool) -> Result<Self, String> {
        Self::open_observed(directory, producer, create, |_| {})
    }

    fn open_observed(
        directory: &Path,
        producer: &str,
        create: bool,
        mut observer: impl FnMut(&str),
    ) -> Result<Self, String> {
        absolute(directory, false)?;
        if producer.is_empty() {
            return Err("producer identity is required".into());
        }
        let began = std::time::Instant::now();
        let bound = std::time::Duration::from_secs(30);
        let _birth = BirthGate::acquire(directory, began, bound, &mut observer)?;
        let path = no_symlink_parents(directory, "objects.json")?;
        let prior: Option<Registry> = match fs::read(&path) {
            Ok(bytes) => Some(decode(&bytes)?),
            Err(e) if create && e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => {
                return Err(format!(
                    "producer registry unavailable; preserve objects: {e}"
                ));
            }
        };
        let lease_path = no_symlink_parents(directory, "registry.lease")?;
        let lease = Lease::acquire_observed(
            &lease_path,
            prior.as_ref().map(|r| r.lease_id.as_str()),
            prior.is_none(),
            true,
            Some(bound.saturating_sub(began.elapsed())),
            || observer("lease-created-before-lock"),
        )?
        .ok_or("producer registry busy")?;
        // Re-read after exclusion: reference acquisition/release and drain use this gate.
        let registry: Registry = match fs::read(&path) {
            Ok(bytes) => decode(&bytes)?,
            Err(e)
                if prior.is_none()
                    && lease.newly_created()
                    && e.kind() == std::io::ErrorKind::NotFound =>
            {
                let registry = Registry {
                    schema: "chrono-retained-producer/v1".into(),
                    producer: producer.into(),
                    directory_id: directory_id(directory)?,
                    lease_id: lease.id().into(),
                    objects: vec![],
                };
                write_json(&path, &registry, true)?;
                registry
            }
            Err(e) => {
                return Err(format!(
                    "producer registry birth is unknown; preserve original lease: {e}"
                ));
            }
        };
        if registry.schema != "chrono-retained-producer/v1"
            || registry.producer != producer
            || registry.directory_id != directory_id(directory)?
            || registry.lease_id != lease.id()
        {
            return Err("producer registry identity differs; preserve objects".into());
        }
        let mut seen = BTreeSet::new();
        for object in &registry.objects {
            relative_path(&object.path)?;
            if Path::new(&object.path).components().count() != 1
                || !seen.insert(&object.path)
                || object.consumers.is_empty()
            {
                return Err("invalid exact producer object registration".into());
            }
        }
        Ok(Self {
            directory: directory.into(),
            registry,
            lease,
        })
    }

    fn stable(&self, object: &Object) -> Result<(), String> {
        self.lease.stable()?;
        if directory_id(&self.directory)? != self.registry.directory_id
            || directory_id(&no_symlink_parents(&self.directory, &object.path)?)? != object.identity
        {
            return Err("producer artifact identity changed; preserve object".into());
        }
        let intent = no_symlink_parents(&self.directory, &object.intent.path)?;
        let bytes = fs::read(intent).map_err(|e| e.to_string())?;
        let intent = json(&bytes)?;
        if sha256(&bytes) != object.intent.sha256
            || intent["path"] != object.path
            || intent["identity"] != object.identity
            || intent["producer"] != self.registry.producer
        {
            return Err("producer intent changed; preserve object".into());
        }
        if object.sealed {
            let publication = object
                .publication
                .as_ref()
                .ok_or("producer publication is missing")?;
            let bytes = fs::read(no_symlink_parents(&self.directory, &publication.path)?)
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
                let bytes = fs::read(no_symlink_parents(&self.directory, &receipt.path)?)
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
            let bytes = fs::read(no_symlink_parents(
                &self.directory.join(&object.path),
                path,
            )?)
            .map_err(|e| e.to_string())?;
            if observed != &value!({"sha256":sha256(&bytes),"length":bytes.len()}) {
                return Err("producer original evidence changed; preserve object".into());
            }
        }
        Ok(())
    }

    fn save(&self) -> Result<(), String> {
        self.lease.stable()?;
        write_json(
            &no_symlink_parents(&self.directory, "objects.json")?,
            &self.registry,
            false,
        )?;
        Ok(())
    }

    fn receipt(&self, prefix: &str, value: &Value) -> Result<Receipt, String> {
        let name = format!(
            "{prefix}-{}.json",
            sha256(&serde_json::to_vec(value).map_err(|e| e.to_string())?)
        );
        let physical = no_symlink_parents(&self.directory, &name)?;
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
        if ["stdout.bin", "stderr.bin", "binding.json", "configuration"]
            .iter()
            .any(|p| {
                Path::new(p).starts_with(path.trim_end_matches('/'))
                    || Path::new(path).starts_with(p)
            })
            || paths.iter().any(|p| p != path && path.starts_with(p))
        {
            return Err(
                "generated producer outputs overlap protected evidence or one another".into(),
            );
        }
    }
    Ok(())
}

impl RetainedArtifact {
    /// Resolve publication through the same committed policy and coordinator as drain.
    /// This does not release consumers, drain stores, or authorize checkout disposal.
    pub fn registered_store(
        host_root: &Path,
        config_path: &str,
        producer: &str,
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
            |r, token, report| {
                let manager = Manager::open(r, &root, config_path, &bytes, token)?
                    .ok_or("retained producer owner is not opted in")?;
                let declaration = manager
                    .policy
                    .retained_producers
                    .iter()
                    .find(|p| p.id == producer)
                    .ok_or("retained producer store is not declared by its owner")?;
                let directory = declaration.directory(&manager.policy.coordinator_root)?;
                manager.stable(r)?;
                report["producer"] = value!(producer);
                report["coordinator_root"] = value!(manager.policy.coordinator_root);
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
        observer: impl FnMut(&str),
    ) -> Result<Self, String> {
        generated_paths(generated_outputs)?;
        if consumer.is_empty() || retention_reason.is_empty() {
            return Err("producer requires a consumer and retention reason".into());
        }
        let mut store = Store::open_observed(directory, producer, true, observer)?;
        let object = artifact
            .strip_prefix(directory)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("producer path UTF-8")?
            .to_owned();
        relative_path(&object)?;
        if Path::new(&object).components().count() != 1
            || store.registry.objects.iter().any(|o| o.path == object)
        {
            return Err("producer artifact must be one new exact child directory".into());
        }
        let identity = directory_id(&no_symlink_parents(directory, &object)?)?;
        let intent = store.receipt(
            "producer-intent",
            &value!({
                "schema":"chrono-retained-artifact-intent/v1","producer":producer,
                "path":object,"identity":identity,"generated_outputs":generated_outputs,
                "consumer":consumer,"retention_reason":retention_reason,
                "evidence_disposal":"not-authorized"
            }),
        )?;
        store.registry.objects.push(Object {
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
        });
        store.save()?;
        Ok(Self {
            directory: directory.into(),
            producer: producer.into(),
            object,
        })
    }

    pub fn resume(directory: &Path, producer: &str, artifact: &Path) -> Result<Self, String> {
        let store = Store::open(directory, producer, false)?;
        let object = artifact
            .strip_prefix(directory)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("producer path UTF-8")?
            .to_owned();
        let record = store
            .registry
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
        let i = store
            .registry
            .objects
            .iter()
            .position(|o| o.path == self.object)
            .ok_or("producer object missing")?;
        let object = &store.registry.objects[i];
        store.stable(object)?;
        if object.sealed || object.disposed {
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
        store.registry.objects[i].publication = Some(publication);
        store.registry.objects[i].generated = generated;
        store.registry.objects[i].evidence = evidence;
        store.registry.objects[i].sealed = true;
        store.save()
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
        let i = store
            .registry
            .objects
            .iter()
            .position(|o| o.path == self.object)
            .ok_or("producer object missing")?;
        let object = &store.registry.objects[i];
        store.stable(object)?;
        if !object.sealed || (!released && object.disposed) {
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
        store.registry.objects[i].consumers.insert(
            consumer.into(),
            Reference {
                released,
                reason: reason.into(),
                receipt: Some(receipt),
            },
        );
        store.save()
    }
}

impl ProducerPolicy {
    fn directory(&self, coordinator_root: &Path) -> Result<PathBuf, String> {
        no_symlink_parents(coordinator_root, self.directory.trim_end_matches('/'))
    }

    pub(super) fn validate(&self, policy: &Policy) -> Result<(), String> {
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
        Ok(())
    }

    pub(super) fn drain(
        &self,
        manager: &Manager,
        r: &mut Runner,
        report: &mut Value,
    ) -> Result<(), String> {
        let root = &manager.policy.coordinator_root;
        let directory = self.directory(root)?;
        if !directory.exists() || !no_symlink_parents(&directory, "objects.json")?.exists() {
            report["producer_objects"].as_array_mut().unwrap().push(value!({"producer":self.id,
                "status":"protected-unknown","reason":"no explicit producer registry; historical objects remain untouched"}));
            return Ok(());
        }
        let mut store = Store::open(&directory, &self.id, false)?;
        let mut failures = vec![];
        for i in 0..store.registry.objects.len() {
            let object = &store.registry.objects[i];
            let mut row = value!({"producer":self.id,"path":format!("{}{}",self.directory,object.path),
                "retention_reason":object.retention_reason,"consumers":object.consumers,
                "registered_generated_objects":object.generated.len(),"evidence":"retained"});
            if !object.sealed || object.consumers.values().any(|c| !c.released) {
                row["status"] = value!("protected");
                row["reason"] = value!(
                    "producer has not explicitly released all consumers or publication is interrupted"
                );
            } else if object.disposed {
                row["status"] = value!("already-disposed");
            } else {
                let mut run = || -> Result<Value, String> {
                    store.stable(object)?;
                    if object
                        .generated
                        .keys()
                        .any(|p| !self.generated_outputs.contains(p))
                    {
                        return Err("producer output is not in the current exact whitelist".into());
                    }
                    let names: Vec<_> = object
                        .generated
                        .keys()
                        .map(|p| format!("{}{}/{p}", self.directory, object.path))
                        .collect();
                    let head = r.oid(root, "HEAD")?;
                    let paths = artifact_disposal::paths(r, root, &head, &names)?;
                    let intent = store.receipt("producer-cleanup-intent", &value!({
                        "schema":"chrono-retained-artifact-cleanup-intent/v1","path":object.path,
                        "identity":object.identity,"producer_intent":object.intent,"generated":object.generated,
                        "policy_sha256":sha256(&manager.policy_bytes),"released_consumers":object.consumers
                    }))?;
                    let mut attempt = value!({"automatic_cleanup":true,"intent":intent,"artifact_disposals":[],"status":"failed"});
                    let outcome =
                        artifact_disposal::dispose(root, &names, &paths, &mut attempt, || {
                            manager.stable(r)?;
                            store.stable(object)?;
                            artifact_disposal::paths(r, root, &head, &names)?;
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
                    let receipt = store.receipt("producer-cleanup-result", &attempt)?;
                    attempt["receipt"] = value!(receipt);
                    Ok(attempt)
                };
                match run() {
                    Ok(attempt) => {
                        store.registry.objects[i].attempts.push(
                            serde_json::from_value(attempt["receipt"].clone())
                                .map_err(|e| e.to_string())?,
                        );
                        store.registry.objects[i].disposed = attempt["status"] == "cleaned";
                        if attempt["status"] != "cleaned" {
                            failures.push(attempt["error"].to_string());
                        }
                        row["status"] = attempt["status"].clone();
                        row["cleanup"] = attempt;
                        store.save()?;
                    }
                    Err(error) => {
                        failures.push(error.clone());
                        row["status"] = value!("failed");
                        row["error"] = value!(error);
                    }
                }
            }
            report["producer_objects"].as_array_mut().unwrap().push(row);
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures.join("; "))
        }
    }
}
