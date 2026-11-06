//! Copied fixture inputs have a cache lifetime; their original bytes have evidence custody.
//! Membership comes from the actual copy producer or a finite current-custodian plan.
use super::*;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Body {
    intent: Receipt,
    original: String,
    sha256: String,
    pub(super) length: u64,
    git_root: Option<String>,
    #[serde(default)]
    leases: Vec<CacheLease>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    nested_consumers: Vec<NestedConsumer>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NestedConsumer {
    directory: String,
    producer: String,
    registry_identity: String,
    allocation: String,
    identity: String,
    intent: Receipt,
}

// The exclusive nested lease prevents new reference publication while this
// reads the exact existing registry. No directory/object discovery is involved.
fn nested_released(
    allocation: &Path,
    member: &str,
    bindings: &[NestedConsumer],
    leases: &[CacheLease],
) -> Result<bool, String> {
    if bindings.len() != leases.len() {
        return Err("nested fixture custody requires exact registry reference bindings".into());
    }
    for binding in bindings {
        let directory = no_symlink_parents(allocation, &binding.directory)?;
        let mut store = Store::open(&directory, &binding.producer, false)?;
        store.load(&binding.allocation)?;
        let object = store
            .state
            .objects
            .iter()
            .find(|o| o.path == binding.allocation)
            .ok_or("nested diagnostic allocation is not registered")?;
        if store.state.directory_id != binding.registry_identity
            || object.identity != binding.identity
            || value!(object.intent) != value!(binding.intent)
        {
            return Err("nested diagnostic producer binding changed; preserve member".into());
        }
        let root = Path::new(&binding.directory).join(&binding.allocation);
        if !Path::new(member).starts_with(&root) {
            return Err("copied body lies outside its nested diagnostic allocation".into());
        }
        let lease = object
            .cache_lease
            .as_ref()
            .ok_or("nested diagnostic has no kernel lease")?;
        let lease_path = Path::new(&binding.directory).join(&lease.path);
        if !leases
            .iter()
            .any(|held| Path::new(&held.path) == lease_path && held.id == lease.id)
        {
            return Err("nested diagnostic reference is not bound to its held lease".into());
        }
        store.stable(object)?;
        if object.consumers.values().any(|c| !c.released) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn input(path: &Path) -> Result<(String, u64), String> {
    let m = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !m.is_file() {
        return Err("copied fixture input must be a regular file; preserve it".into());
    }
    let (digest, _) = chrono_harness::file_identity(path)?;
    Ok((digest, m.len()))
}

fn publish_claim(
    directory: &Path,
    allocation: &str,
    allocation_identity: &str,
    producer: &str,
    digest: &str,
    length: u64,
) -> Result<(), String> {
    let claims = no_symlink_parents(directory, &format!("body-input-claims/{digest}"))?;
    fs::create_dir_all(&claims).map_err(|e| e.to_string())?;
    let claim_guard = DirectoryGate::acquire(
        &claims,
        std::time::Instant::now(),
        std::time::Duration::from_secs(30),
        &mut |_| {},
    )?;
    let name = format!("body-inputs/{digest}.bin");
    let claim = value!({"schema":"chrono-fixture-input-claim/v1","producer":producer,
        "allocation":allocation,"identity":allocation_identity,"sha256":digest,"length":length});
    claim_guard.stable()?;
    let claim_path =
        no_symlink_parents(&claims, &format!("{}.json", sha256(allocation.as_bytes())))?;
    let publish = match fs::read(&claim_path) {
        Ok(bytes) if json(&bytes)? == claim => false,
        Ok(_) => return Err("shared copied-input claim differs; preserve original".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
        Err(e) => return Err(e.to_string()),
    };
    let _read = if let Some(context) = Context::for_allocation(&directory.join(allocation))? {
        if let Some(inventory) =
            chrono_harness::retained_artifacts::retention::Inventory::adopted(&context.root)?
        {
            let path = directory.join(&name);
            let relative = path
                .strip_prefix(&context.root)
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("original UTF-8")?;
            let read = match chrono_harness::retained_artifacts::retention::read_guard(
                &context.root,
                relative,
            ) {
                Ok(read) => read,
                Err(original_error) => {
                    // A previous final consumer may have released this same
                    // digest into bounded retirement. Join that exact owner
                    // once before publishing a new claim; never bypass an
                    // unavailable original or retry an unbounded cleanup loop.
                    let recovery =
                        inventory
                            .maintain_paths(&[relative.into()])
                            .map_err(|error| {
                                format!("{original_error}; addressed original retirement: {error}")
                            })?;
                    chrono_harness::retained_artifacts::retention::read_guard(
                        &context.root,
                        relative,
                    )
                    .map_err(|error| {
                        format!("{original_error}; addressed original retirement: {recovery}; original still unavailable: {error}")
                    })?
                }
            };
            if inventory.enrolled(relative)? {
                inventory.reference_paths(
                    &format!("fixture-input:{}:{digest}", sha256(allocation.as_bytes())),
                    &[relative.into()],
                )?;
            }
            read
        } else {
            None
        }
    } else {
        None
    };
    claim_guard.stable()?;
    if publish {
        write_json(&claim_path, &claim, true)?;
    }
    Ok(())
}

fn original(
    directory: &Path,
    source: &Path,
    reserved: Option<u64>,
    observer: &mut impl FnMut(&str),
) -> Result<(String, String, u64, String), String> {
    let (digest, length) = input(source)?;
    if reserved.is_some_and(|bound| length != bound) {
        return Err(
            "copied input length changed after capacity reservation; preserve intent".into(),
        );
    }
    let claims = no_symlink_parents(directory, &format!("body-input-claims/{digest}"))?;
    fs::create_dir_all(&claims).map_err(|e| e.to_string())?;
    let mut claim_guard = None;
    let name = format!("body-inputs/{digest}.bin");
    let path = no_symlink_parents(directory, &name)?;
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    if !path.exists() {
        let temporary =
            tempfile::NamedTempFile::new_in(path.parent().unwrap()).map_err(|e| e.to_string())?;
        use std::io::Read;
        let mut reader = fs::File::open(source).map_err(|e| e.to_string())?;
        let mut writer = temporary.as_file();
        let copied = std::io::copy(&mut (&mut reader).take(length), &mut writer)
            .map_err(|e| e.to_string())?;
        let mut extra = [0];
        if copied != length || reader.read(&mut extra).map_err(|e| e.to_string())? != 0 {
            return Err("copied input changed during bounded custody transfer".into());
        }
        fs::set_permissions(
            temporary.path(),
            fs::metadata(source)
                .map_err(|e| e.to_string())?
                .permissions(),
        )
        .map_err(|e| e.to_string())?;
        if input(source)? != (digest.clone(), length)
            || input(temporary.path())? != (digest.clone(), length)
        {
            return Err("copied input changed during original custody transfer".into());
        }
        temporary.as_file().sync_all().map_err(|e| e.to_string())?;
        observer("copy-original-publish");
        claim_guard = Some(DirectoryGate::acquire(
            &claims,
            std::time::Instant::now(),
            std::time::Duration::from_secs(30),
            &mut |_| {},
        )?);
        match temporary.persist_noclobber(&path) {
            Ok(_) => (),
            // Another protected copy can publish the same content-addressed
            // original. Verify that winner below; never replace its bytes.
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(error) => return Err(error.to_string()),
        }
    }
    let claim_guard = match claim_guard {
        Some(guard) => guard,
        None => DirectoryGate::acquire(
            &claims,
            std::time::Instant::now(),
            std::time::Duration::from_secs(30),
            &mut |_| {},
        )?,
    };
    claim_guard.stable()?;
    let identity = fingerprint(&path)?;
    if input(&path)? != (digest.clone(), length) || fingerprint(&path)? != identity {
        return Err("retained copied-input original differs; preserve bodies".into());
    }
    Ok((name, digest, length, identity))
}

fn register(
    mut store: Store,
    i: usize,
    member: &str,
    source: &Path,
    git_root: Option<String>,
    leases: Vec<CacheLease>,
    nested_consumers: Vec<NestedConsumer>,
    provenance: Value,
    lease: &Lease,
    require_released: bool,
    mut observer: impl FnMut(&str),
) -> Result<PathBuf, String> {
    relative_path(member)?;
    if provenance.is_null() {
        return Err("copied body requires real producer provenance".into());
    }
    let object = &store.state.objects[i];
    if object.original_release.is_some() || object.retiring {
        return Err("fixture original lifetime released; copy refused".into());
    }
    store.stable(object)?;
    let context = Context::for_allocation(&store.directory.join(&object.path))?;
    let reserved = if let Some(context) = context
        .as_ref()
        .filter(|_| !object.bodies.contains_key(member))
    {
        let length = fs::metadata(source).map_err(|e| e.to_string())?.len();
        context.copy_admission(object, member, length)?;
        Some(length)
    } else {
        None
    };
    if object
        .cache_lease
        .as_ref()
        .map(|binding| binding.id.as_str())
        != Some(lease.id())
        || object.disposed
        || object.evidence.contains_key(member)
    {
        return Err(
            "copied body needs temporary ownership and cannot replace original evidence".into(),
        );
    }
    let mut hook_leaf = false;
    if let Some(root) = &git_root {
        if root != "." {
            relative_path(root)?;
        }
        let relative = if root == "." {
            member
        } else {
            member
                .strip_prefix(&format!("{root}/"))
                .ok_or("body lies outside declared Git root")?
        };
        hook_leaf = relative
            .strip_prefix(".git/hooks/")
            .is_some_and(|leaf| !leaf.is_empty() && Path::new(leaf).components().count() == 1);
    }
    if Path::new(member)
        .components()
        .any(|part| part.as_os_str() == ".git")
        && !hook_leaf
    {
        return Err("Git identity and unretained commits cannot be fixture bodies".into());
    }
    if require_released && object.consumers.values().any(|c| !c.released) {
        return Err("fixture allocation has an unreleased diagnostic consumer".into());
    }
    let directory = store.directory.clone();
    let producer = store.state.producer.clone();
    let registry_identity = store.state.directory_id.clone();
    let owner_identity = store.state.owner_id.clone();
    let allocation = object.path.clone();
    let allocation_identity = object.identity.clone();
    let producer_intent = object.intent.clone();
    let cache_lease = object.cache_lease.clone();
    // Allocation ownership protects the copy, while unrelated allocations and
    // references must remain able to use the shared registry during file IO.
    lease.stable()?;
    let (shared_digest, shared_length) = input(source)?;
    Arc::make_mut(&mut store.state.objects[i])
        .shared_originals
        .insert(shared_digest.clone(), shared_length);
    if let Some(length) = reserved {
        Arc::make_mut(&mut store.state.objects[i])
            .copy_intents
            .insert(member.into(), length);
    }
    store.save(i)?;
    publish_claim(
        &directory,
        &allocation,
        &allocation_identity,
        &producer,
        &shared_digest,
        shared_length,
    )?;
    drop(store);
    observer("copy-original-entered");
    let (name, digest, length, original_identity) =
        original(&directory, source, reserved, &mut observer)?;
    observer("copy-original-prepared");
    lease.stable()?;
    let mut store = Store::open(&directory, &producer, false)?;
    store.load(&allocation)?;
    let i = store
        .state
        .objects
        .iter()
        .position(|object| object.path == allocation)
        .ok_or("copy allocation disappeared during original custody transfer")?;
    let object = &store.state.objects[i];
    if store.state.directory_id != registry_identity
        || store.state.owner_id != owner_identity
        || object.identity != allocation_identity
        || value!(object.intent) != value!(producer_intent)
        || value!(object.cache_lease) != value!(cache_lease)
        || object.disposed
        || object.evidence.contains_key(member)
    {
        return Err("copy allocation binding changed during original custody transfer".into());
    }
    store.stable(object)?;
    if let Some(context) = &context {
        context.copy_admission(object, member, length)?;
    }
    if require_released && object.consumers.values().any(|c| !c.released) {
        return Err("fixture allocation has an unreleased diagnostic consumer".into());
    }
    if fingerprint(&no_symlink_parents(&directory, &name)?)? != original_identity {
        return Err("retained copied-input original changed before registration".into());
    }
    if let Some(existing) = object.bodies.get(member) {
        if existing.original != name
            || existing.sha256 != digest
            || existing.length != length
            || existing.git_root != git_root
            || value!(existing.leases) != value!(leases)
            || value!(existing.nested_consumers) != value!(nested_consumers)
        {
            return Err("copied body registration changed; preserve member".into());
        }
        return Ok(store.directory.join(name));
    }
    let intent = store.receipt(
        "fixture-body-intent",
        &value!({
            "schema":"chrono-fixture-copied-input/v1", "allocation":object.path,
            "identity":object.identity, "producer_intent":object.intent,
            "member":member, "original":name, "sha256":digest, "length":length,
            "git_root":git_root, "leases":leases, "nested_consumers":nested_consumers, "provenance":provenance,
            "lifetime":"last-supported-holder-and-current-consumer-release",
            "original_custody":"retained", "historical_producer_outcome":"unchanged"
        }),
    )?;
    Arc::make_mut(&mut store.state.objects[i]).bodies.insert(
        member.into(),
        Body {
            intent,
            original: name.clone(),
            sha256: digest,
            length,
            git_root,
            leases,
            nested_consumers,
        },
    );
    Arc::make_mut(&mut store.state.objects[i])
        .copy_intents
        .remove(member);
    lease.stable()?;
    store.save(i)?;
    Ok(store.directory.join(name))
}

impl TemporaryHost {
    /// A real fixture move carries only its already enrolled copy members.
    /// Publish destination membership before rename; originals and old intents survive.
    pub fn relocate_copies(
        directory: &Path,
        producer: &str,
        old_root: &Path,
        new_root: &Path,
    ) -> Result<(), String> {
        let old = old_root
            .strip_prefix(directory)
            .map_err(|e| e.to_string())?;
        let object = old
            .components()
            .next()
            .and_then(|p| p.as_os_str().to_str())
            .ok_or("copy relocation allocation UTF-8")?;
        let allocation = directory.join(object);
        let old_relative = old_root
            .strip_prefix(&allocation)
            .map_err(|e| e.to_string())?;
        let new_relative = new_root
            .strip_prefix(&allocation)
            .map_err(|e| e.to_string())?;
        let mut store = Store::open(directory, producer, false)?;
        store.load(object)?;
        let record = store
            .state
            .objects
            .iter()
            .find(|o| o.path == object)
            .ok_or("relocation allocation is not enrolled")?;
        store.stable(record)?;
        let bodies = record.bodies.clone();
        drop(store);
        for (member, body) in bodies {
            if !body.leases.is_empty() {
                return Err(
                    "copy relocation with nested custody requires an explicit bound migration"
                        .into(),
                );
            }
            let Ok(tail) = Path::new(&member).strip_prefix(old_relative) else {
                continue;
            };
            let source = allocation.join(&member);
            // Changed/missing inputs retain their prior diagnostics and never
            // acquire reconstructible membership at a new path.
            if !source.exists() || input(&source)? != (body.sha256, body.length) {
                continue;
            }
            let root = body.git_root.map(|root| {
                let relative = if root == "." {
                    PathBuf::new()
                } else {
                    PathBuf::from(root)
                };
                allocation.join(match relative.strip_prefix(old_relative) {
                    Ok(tail) => new_relative.join(tail),
                    Err(_) => relative,
                })
            });
            Self::register_copy(
                directory,
                producer,
                &source,
                &new_root.join(tail),
                root.as_deref(),
                "actual enrolled fixture copy relocation",
            )?;
        }
        Ok(())
    }

    /// Declare a real copy before it is produced. The returned original is retained
    /// independently of this allocation and can be used by the existing copy route.
    /// No outcome or diagnostic consumer is released by this operation.
    pub fn register_copy(
        directory: &Path,
        producer: &str,
        source: &Path,
        destination: &Path,
        git_root: Option<&Path>,
        recipe: &str,
    ) -> Result<PathBuf, String> {
        Self::register_copy_observed(
            directory,
            producer,
            source,
            destination,
            git_root,
            recipe,
            |_| {},
        )
    }

    /// Deterministic scheduling of the real copy transfer in dedicated tests.
    #[cfg(feature = "producer-birth-test")]
    #[doc(hidden)]
    pub fn register_copy_with_observer(
        directory: &Path,
        producer: &str,
        source: &Path,
        destination: &Path,
        git_root: Option<&Path>,
        recipe: &str,
        observer: impl FnMut(&str),
    ) -> Result<PathBuf, String> {
        Self::register_copy_observed(
            directory,
            producer,
            source,
            destination,
            git_root,
            recipe,
            observer,
        )
    }

    fn register_copy_observed(
        directory: &Path,
        producer: &str,
        source: &Path,
        destination: &Path,
        git_root: Option<&Path>,
        recipe: &str,
        mut observer: impl FnMut(&str),
    ) -> Result<PathBuf, String> {
        if recipe.trim().is_empty() {
            return Err("copy producer recipe is required".into());
        }
        let relative = destination
            .strip_prefix(directory)
            .map_err(|e| e.to_string())?;
        let mut parts = relative.components();
        let object = parts
            .next()
            .and_then(|p| p.as_os_str().to_str())
            .ok_or("copy allocation UTF-8")?;
        let member = parts.as_path().to_str().ok_or("copy member UTF-8")?;
        let mut store = Store::open(directory, producer, false)?;
        store.load(object)?;
        let i = store
            .state
            .objects
            .iter()
            .position(|o| o.path == object)
            .ok_or("copy destination has no exact allocation registration")?;
        let binding = store.state.objects[i]
            .cache_lease
            .as_ref()
            .ok_or("copy has no allocation lease")?;
        let lease = Lease::acquire(
            &no_symlink_parents(directory, &binding.path)?,
            Some(&binding.id),
            false,
            false,
            None,
        )?
        .ok_or("copy allocation is being recovered")?;
        let allocation = directory.join(object);
        let git_root = git_root
            .map(|root| -> Result<String, String> {
                if root == allocation {
                    Ok(".".into())
                } else {
                    root.strip_prefix(&allocation)
                        .map_err(|e| e.to_string())?
                        .to_str()
                        .map(str::to_owned)
                        .ok_or("copy Git root UTF-8".into())
                }
            })
            .transpose()?;
        let retained = register(
            store,
            i,
            member,
            source,
            git_root,
            vec![],
            vec![],
            value!({"route":"actual-copy-producer", "recipe":recipe,"source":source}),
            &lease,
            false,
            &mut observer,
        )?;
        lease.stable()?;
        Ok(retained)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Custody {
    schema: String,
    head: String,
    current_consumers_released: bool,
    reason: String,
    objects: Vec<Selection>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    producer: String,
    allocation: String,
    identity: String,
    intent: Receipt,
    members: Vec<Member>,
    #[serde(default)]
    release_originals: bool,
    #[serde(default)]
    adopt_directory: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Member {
    path: String,
    sha256: String,
    length: u64,
    git_root: Option<String>,
    recipe: String,
    #[serde(default)]
    leases: Vec<CacheLease>,
    #[serde(default)]
    nested_consumers: Vec<NestedConsumer>,
}

impl Manager {
    pub(in crate::automatic) fn adopt_fixture_bodies(
        &self,
        r: &mut Runner,
        target: &Path,
        plan_path: &str,
        report: &mut Value,
    ) -> Result<(), String> {
        if target != self.inputs.policy.coordinator_root {
            return Err("fixture custody requires surviving coordinator".into());
        }
        let bytes = crate::recovery::state_bytes(target, plan_path)?;
        let plan: Custody = decode(&bytes)?;
        if plan.schema != "chrono-fixture-body-custody/v1"
            || plan.head != self.inputs.anchor_head
            || !plan.current_consumers_released
            || plan.reason.trim().is_empty()
            || plan.objects.is_empty()
        {
            return Err(
                "fixture custody must bind current HEAD and actual current consumer disposition"
                    .into(),
            );
        }
        let custody = Store::receipt_in(&self.directory, "fixture-custody-input", &json(&bytes)?)?;
        let mut failures = vec![];
        report["fixture_body_adoption"] = value!([]);
        let mut seen = BTreeSet::new();
        for selected in plan.objects {
            let outcome = (|| -> Result<(), String> {
                if !seen.insert((selected.producer.clone(), selected.allocation.clone()))
                    || (selected.members.is_empty() && !selected.release_originals)
                {
                    return Err("duplicate or empty fixture custody selection".into());
                }
                let policy = self
                    .inputs
                    .policy
                    .retained_producers
                    .iter()
                    .find(|p| p.id == selected.producer && p.fixture_bodies)
                    .ok_or("fixture body class is not adopted for this producer")?;
                let directory = policy.directory(target)?;
                let mut store = Store::open(&directory, &selected.producer, false)?;
                store.load(&selected.allocation)?;
                let i = store
                    .state
                    .objects
                    .iter()
                    .position(|o| o.path == selected.allocation)
                    .ok_or("unknown allocation is never imported by fixture custody")?;
                let object = &store.state.objects[i];
                if object.identity != selected.identity
                    || value!(object.intent) != value!(selected.intent)
                {
                    return Err("custody selection does not bind original producer identity".into());
                }
                store.stable(object)?;
                let binding = object
                    .cache_lease
                    .as_ref()
                    .ok_or("fixture custody has no kernel owner")?;
                let lease = Lease::acquire(
                    &no_symlink_parents(&directory, &binding.path)?,
                    Some(&binding.id),
                    false,
                    true,
                    None,
                )?
                .ok_or("fixture allocation still has supported holders")?;
                if object.consumers.values().any(|c| !c.released) {
                    return Err("fixture allocation has an unreleased diagnostic consumer".into());
                }
                if selected.adopt_directory
                    && (!policy.temporary
                        || !policy.disposable_directory
                        || !selected.release_originals
                        || !selected.members.is_empty()
                        || object.disposable_directory
                        || object.retiring)
                {
                    return Err("directory custody requires adopted temporary ownership, exact original release and no member transfer or retirement".into());
                }
                drop(store);
                let allocation = directory.join(&selected.allocation);
                for member in selected.members {
                    let physical = no_symlink_parents(&allocation, &member.path)?;
                    if input(&physical)? != (member.sha256.clone(), member.length) {
                        return Err(format!("custody member differs: {}", member.path));
                    }
                    protect_source(r, &allocation, &member.path, member.git_root.as_deref())?;
                    relative_path(&member.recipe)?;
                    let mut dependencies = vec![];
                    for binding in &member.leases {
                        dependencies.push(
                            Lease::acquire(
                                &no_symlink_parents(&allocation, &binding.path)?,
                                Some(&binding.id),
                                false,
                                true,
                                None,
                            )?
                            .ok_or("nested registered consumer remains live")?,
                        );
                    }
                    if !nested_released(
                        &allocation,
                        &member.path,
                        &member.nested_consumers,
                        &member.leases,
                    )? {
                        return Err(
                            "nested registered diagnostic consumer remains unreleased".into()
                        );
                    }
                    let source = r.blob(target, &self.inputs.anchor_head, &member.recipe)?;
                    self.stable(r)?;
                    lease.stable()?;
                    let mut store = Store::open(&directory, &selected.producer, false)?;
                    store.load(&selected.allocation)?;
                    let i = store
                        .state
                        .objects
                        .iter()
                        .position(|o| o.path == selected.allocation)
                        .ok_or("custody allocation disappeared during original transfer")?;
                    let object = &store.state.objects[i];
                    if object.identity != selected.identity
                        || value!(object.intent) != value!(selected.intent)
                    {
                        return Err("custody selection changed during original transfer".into());
                    }
                    register(
                        store,
                        i,
                        &member.path,
                        &physical,
                        member.git_root,
                        member.leases,
                        member.nested_consumers,
                        value!({"route":"current-custodian-transition", "custody":custody,
                        "reason":plan.reason, "recipe":member.recipe,"recipe_sha256":sha256(&source),
                        "member_identity":crate::ownership::identity(&physical)?,
                        "candidate":self.inputs.anchor_head, "past_outcome":"unchanged"}),
                        &lease,
                        true,
                        |_| {},
                    )?;
                }
                if selected.release_originals {
                    self.stable(r)?;
                    lease.stable()?;
                    let mut store = Store::open(&directory, &selected.producer, false)?;
                    store.load(&selected.allocation)?;
                    let object = &store.state.objects[0];
                    store.stable(object)?;
                    if object.identity != selected.identity
                        || value!(object.intent) != value!(selected.intent)
                        || object.consumers.values().any(|c| !c.released)
                    {
                        return Err("original release binding or consumers changed".into());
                    }
                    if selected.adopt_directory && object.directory_custody.is_none() {
                        let receipt = store.receipt(
                            "producer-directory-custody",
                            &value!({"schema":"chrono-fixture-directory-custody/v1",
                                "producer":selected.producer,"path":object.path,
                                "identity":object.identity,"original_intent":object.intent,
                                "custody":custody,"candidate":self.inputs.anchor_head,
                                "reason":plan.reason,"historical_outcome":"unchanged"}),
                        )?;
                        Arc::make_mut(&mut store.state.objects[0]).directory_custody =
                            Some(receipt);
                    }
                    Arc::make_mut(&mut store.state.objects[0]).original_release = Some(format!(
                        "current custodian released exact originals: {}; custody {}",
                        plan.reason, custody.path
                    ));
                    store.save(0)?;
                    if selected.adopt_directory {
                        retirement::prioritize(&directory, &selected.allocation)?;
                    }
                }
                Ok(())
            })();
            let mut row = value!({"producer":selected.producer,"allocation":selected.allocation,
                "disposal":"not-performed","historical_outcome":"unchanged"});
            match outcome {
                Ok(()) => row["status"] = value!("adopted"),
                Err(error) => {
                    failures.push(error.clone());
                    row["status"] = value!("failed");
                    row["error"] = value!(error);
                }
            }
            report["fixture_body_adoption"]
                .as_array_mut()
                .unwrap()
                .push(row);
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures.join("; "))
        }
    }
}

fn protect_source(
    r: &mut Runner,
    allocation: &Path,
    member: &str,
    root: Option<&str>,
) -> Result<(), String> {
    if let Some(root) = root {
        let directory = if root == "." {
            allocation.to_owned()
        } else {
            no_symlink_parents(allocation, root)?
        };
        let metadata = fs::symlink_metadata(&directory).map_err(|e| e.to_string())?;
        if !metadata.is_dir() {
            return Err("copied body source root is unavailable".into());
        }
        // The producer explicitly names this possible Git root. Absence here
        // does not discover a parent or import any other worktree.
        if !directory.join(".git").exists() {
            return Ok(());
        }
        let observed = r.checkout_identity(&directory)?;
        if observed.top != directory {
            return Err("copied body Git root differs".into());
        }
        let relative = allocation
            .join(member)
            .strip_prefix(&directory)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("copied body source path UTF-8")?
            .to_owned();
        let tree = r.tree(&directory, &observed.head)?;
        let index = start::paths(r.git(&directory, &["ls-files", "-z"])?)?;
        if tree.contains_key(&relative) || index.contains(&relative) {
            return Err("copied body is committed or staged source; preserve it".into());
        }
    }
    Ok(())
}

pub(super) fn directory_guards(directory: &Path, object: &Object) -> Result<Option<Vec<Lease>>, String> {
    let allocation = directory.join(&object.path);
    let mut guards: BTreeMap<String, Lease> = BTreeMap::new();
    for (member, body) in &object.bodies {
        let bytes = fs::read(no_symlink_parents(directory, &body.intent.path)?)
            .map_err(|e| e.to_string())?;
        let intent = json(&bytes)?;
        if sha256(&bytes) != body.intent.sha256
            || intent["producer_intent"] != value!(object.intent)
            || intent["allocation"] != object.path
            || intent["identity"] != object.identity
            || intent["member"] != *member
            || intent["leases"] != value!(body.leases)
            || intent.get("nested_consumers").cloned().unwrap_or_else(|| value!([]))
                != value!(body.nested_consumers)
        {
            return Err("owned directory nested consumer intent differs".into());
        }
        for binding in &body.leases {
            if let Some(held) = guards.get(&binding.path) {
                if held.id() != binding.id {
                    return Err("owned directory nested lease identities conflict".into());
                }
                continue;
            }
            let Some(held) = Lease::acquire(
                &no_symlink_parents(&allocation, &binding.path)?,
                Some(&binding.id), false, true, None,
            )? else { return Ok(None); };
            guards.insert(binding.path.clone(), held);
        }
        if !nested_released(&allocation, member, &body.nested_consumers, &body.leases)? {
            return Ok(None);
        }
    }
    Ok(Some(guards.into_values().collect()))
}

pub(super) fn drain(
    policy: &ProducerPolicy,
    inputs: &PolicyInputs,
    r: &mut Runner,
    directory: &Path,
    object: &Object,
    registry_id: &str,
    lease: &Lease,
    originals: &mut BTreeMap<PathBuf, String>,
) -> Result<Value, String> {
    let allocation = directory.join(&object.path);
    let mut row = value!({"class":"copied-input/v1", "members":[],"original_custody":"retained"});
    let mut failures = vec![];
    for (member, body) in &object.bodies {
        let outcome = (|| -> Result<Value, String> {
            if !policy.fixture_bodies {
                return Err("copied fixture class no longer adopted".into());
            }
            let bytes = fs::read(no_symlink_parents(directory, &body.intent.path)?)
                .map_err(|e| e.to_string())?;
            let intent = json(&bytes)?;
            if sha256(&bytes) != body.intent.sha256
                || intent["schema"] != "chrono-fixture-copied-input/v1"
                || intent["allocation"] != object.path
                || intent["identity"] != object.identity
                || intent["producer_intent"] != value!(object.intent)
                || intent["member"] != *member
                || intent["original"] != body.original
                || intent["sha256"] != body.sha256
                || intent["length"] != body.length
                || intent["git_root"] != value!(body.git_root)
                || intent["leases"] != value!(body.leases)
                || intent
                    .get("nested_consumers")
                    .cloned()
                    .unwrap_or_else(|| value!([]))
                    != value!(body.nested_consumers)
            {
                return Err("copied body intent binding changed".into());
            }
            let original = no_symlink_parents(directory, &body.original)?;
            let original_id = fingerprint(&original)?;
            if !originals
                .get(&original)
                .is_some_and(|prior| prior == &original_id)
            {
                if input(&original)? != (body.sha256.clone(), body.length) {
                    return Err("copied input original unavailable or changed".into());
                }
                originals.insert(original.clone(), original_id.clone());
            }
            let mut dependencies = vec![];
            for binding in &body.leases {
                match Lease::acquire(
                    &no_symlink_parents(&allocation, &binding.path)?,
                    Some(&binding.id),
                    false,
                    true,
                    None,
                )? {
                    Some(held) => dependencies.push(held),
                    None => {
                        return Ok(
                            value!({"path":member,"status":"protected","reason":"live explicitly bound nested consumer","lease":binding.path}),
                        );
                    }
                }
            }
            if !nested_released(&allocation, member, &body.nested_consumers, &body.leases)? {
                return Ok(
                    value!({"path":member,"status":"protected","reason":"unreleased explicitly bound nested diagnostic consumer"}),
                );
            }
            let path = no_symlink_parents(&allocation, member)?;
            match fs::symlink_metadata(&path) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(value!({"path":member,"status":"already-absent"}));
                }
                Err(e) => return Err(e.to_string()),
                Ok(m) if !m.is_file() || m.file_type().is_symlink() => {
                    return Err(
                        "copied body is not a regular file; preserve original member".into(),
                    );
                }
                Ok(_) => (),
            }
            if let Some(expected) = intent["provenance"]["member_identity"].as_str() {
                if crate::ownership::identity(&path)? != expected {
                    return Err(
                        "legacy copied member identity changed; preserve replacement".into(),
                    );
                }
            }
            let footprint = artifact_disposal::artifact_footprint(&path)?;
            let before_id = crate::ownership::identity(&path)?;
            let effect = value!({"schema":"chrono-fixture-body-disposal-intent/v1", "allocation":object.path,
                "identity":object.identity,"member":member,"member_identity":before_id,"body_intent":body.intent,
                "footprint_before":footprint, "physical_release_bytes":Value::Null});
            let intent = Store::receipt_for(
                directory,
                object
                    .metadata_directory
                    .as_ref()
                    .map(|_| object.path.as_str()),
                "fixture-body-disposal-intent",
                &effect,
            )?;
            let mut result = value!({"path":member,"intent":intent,"footprint_before":footprint,"status":"failed","original":body.original,"physical_release_bytes":Value::Null});
            let report_root = body
                .git_root
                .as_ref()
                .map(|root| {
                    if root == "." {
                        allocation.clone()
                    } else {
                        allocation.join(root)
                    }
                })
                .unwrap_or_else(|| allocation.clone());
            let report_member = path
                .strip_prefix(&report_root)
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("report overlap path UTF-8")?;
            let _retention = chrono_harness::retained_artifacts::retention::external_removal_guard(
                &report_root,
                report_member,
            )?;
            match chrono_harness::artifact_disposal::dispose_entry(
                &allocation,
                member,
                // Custody's durable member identity is device:inode. Its
                // already verified regular-file meaning supplies the shared
                // effect owner's type tag; keep the old intent unchanged.
                &format!("{before_id}:file"),
                || {
                    // The shared effect owner calls this immediately before
                    // unlink and rechecks physical identity afterwards. Keep
                    // all live admission here instead of repeating content and
                    // Git reads on both sides of intent publication.
                    if crate::ownership::identity(&path)? != before_id
                        || input(&path)? != (body.sha256.clone(), body.length)
                    {
                        return Err(
                            "copied body differs or is partial; preserve original member".into(),
                        );
                    }
                    if fingerprint(&original)? != original_id {
                        return Err("copied input original changed before removal".into());
                    }
                    protect_source(r, &allocation, member, body.git_root.as_deref())?;
                    inputs.stable(r)?;
                    lease.stable()?;
                    Store::stable_in(directory, &policy.id, registry_id, object)?;
                    for dependency in &dependencies {
                        dependency.stable()?;
                    }
                    if let Some(retention) = &_retention {
                        retention.stable()?;
                    }
                    Ok(())
                },
            ) {
                Ok(()) if matches!(fs::symlink_metadata(&path), Err(e) if e.kind() == std::io::ErrorKind::NotFound) =>
                {
                    result["status"] = value!("verified-absent");
                    result["footprint_after"] =
                        value!(artifact_disposal::artifact_footprint(&path)?);
                }
                Ok(()) => result["error"] = value!("removed member absence unverified"),
                Err(e) => result["error"] = value!(e.to_string()),
            }
            result["receipt"] = value!(Store::receipt_for(
                directory,
                object
                    .metadata_directory
                    .as_ref()
                    .map(|_| object.path.as_str()),
                "fixture-body-disposal-result",
                &result
            )?);
            Ok(result)
        })();
        let result = match outcome {
            Ok(result) => result,
            Err(error) => value!({"path":member,"status":"failed","error":error}),
        };
        if result["status"] == "failed" {
            failures.push(result["error"].clone());
        }
        row["members"].as_array_mut().unwrap().push(result);
    }
    row["status"] = value!(if !failures.is_empty() {
        "failed"
    } else if row["members"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["status"] == "protected")
    {
        "protected"
    } else {
        "recovered"
    });
    row["failures"] = value!(failures);
    Ok(row)
}

pub(super) fn fingerprint(path: &Path) -> Result<String, String> {
    let m = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if !m.is_file() {
            return Err("retained original is not a regular file".into());
        }
        Ok(format!(
            "{}:{}:{}:{}:{}:{}:{}:{}",
            m.dev(),
            m.ino(),
            m.len(),
            m.mode(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec()
        ))
    }
    #[cfg(not(unix))]
    {
        let _ = m;
        Err("copied-input recovery needs kernel ownership".into())
    }
}

pub(super) fn release_original(
    inputs: &PolicyInputs,
    r: &mut Runner,
    directory: &Path,
    object: &Object,
    digest: &str,
    length: u64,
    lease: &Lease,
) -> Result<(), String> {
    let claims = no_symlink_parents(directory, &format!("body-input-claims/{digest}"))?;
    let original = no_symlink_parents(directory, &format!("body-inputs/{digest}.bin"))?;
    if !claims.exists() {
        return match fs::symlink_metadata(&original) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
            Ok(_) => Err("shared original claim namespace unknown; preserve bytes".into()),
        };
    }
    let guard = DirectoryGate::acquire(
        &claims,
        std::time::Instant::now(),
        std::time::Duration::from_secs(30),
        &mut |_| {},
    )?;
    let original_present = match fs::symlink_metadata(&original) {
        Ok(_) => true,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => return Err(e.to_string()),
    };
    let path = no_symlink_parents(&claims, &format!("{}.json", sha256(object.path.as_bytes())))?;
    let expected = value!({"schema":"chrono-fixture-input-claim/v1","producer":inputs.policy.retained_producers.iter().find(|p|p.directory(&inputs.policy.coordinator_root).ok().as_deref()==Some(directory)).ok_or("original producer missing")?.id,
        "allocation":object.path,"identity":object.identity,"sha256":digest,"length":length});
    if path.exists() {
        let bytes = fs::read(&path).map_err(|e| e.to_string())?;
        if json(&bytes)? != expected {
            return Err("original claim identity differs; preserve original".into());
        }
        if original_present && input(&original)? != (digest.into(), length) {
            return Err("shared original content differs; preserve original".into());
        }
        lease.stable()?;
        guard.stable()?;
        inputs.stable(r)?;
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    let inventory = chrono_harness::retained_artifacts::retention::Inventory::adopted(
        &inputs.policy.coordinator_root,
    )?
    .ok_or("shared original retention not adopted")?;
    inventory.reference(
        &format!("fixture-input:{}:{digest}", sha256(object.path.as_bytes())),
        &[],
    )?;
    if fs::read_dir(&claims)
        .map_err(|e| e.to_string())?
        .next()
        .is_some()
    {
        return Ok(());
    }
    let relative = original
        .strip_prefix(&inputs.policy.coordinator_root)
        .map_err(|e| e.to_string())?
        .to_str()
        .ok_or("shared original UTF-8")?;
    lease.stable()?;
    guard.stable()?;
    inputs.stable(r)?;
    if original_present && !inventory.enrolled(relative)? {
        let publication=chrono_harness::retained_artifacts::retention::Inventory::adopted(&inputs.policy.coordinator_root)?.unwrap()
            .begin("fixture-retirement",relative,value!({"original_consumer_release":object.original_release,"historical_outcome":"unchanged"}))?;
        publication.complete(value!({"original_consumer_release":object.original_release,"historical_outcome":"unchanged"}))?;
    }
    guard.stable()?;
    fs::remove_dir(&claims).map_err(|e| e.to_string())?;
    Ok(())
}
