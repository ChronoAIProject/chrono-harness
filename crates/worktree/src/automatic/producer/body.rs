//! Copied fixture inputs have a cache lifetime; their original bytes have evidence custody.
//! Membership comes from the actual copy producer or a finite current-custodian plan.
use super::*;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Body {
    intent: Receipt,
    original: String,
    sha256: String,
    length: u64,
    git_root: Option<String>,
    attempts: Vec<Receipt>,
}

fn input(path: &Path) -> Result<(String, u64), String> {
    let m = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !m.is_file() {
        return Err("copied fixture input must be a regular file; preserve it".into());
    }
    let (digest, _) = chrono_harness::file_identity(path)?;
    Ok((digest, m.len()))
}

fn original(store: &Store, source: &Path) -> Result<(String, String, u64), String> {
    let (digest, length) = input(source)?;
    let name = format!("body-inputs/{digest}.bin");
    let path = no_symlink_parents(&store.directory, &name)?;
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    if !path.exists() {
        let temporary =
            tempfile::NamedTempFile::new_in(path.parent().unwrap()).map_err(|e| e.to_string())?;
        fs::copy(source, temporary.path()).map_err(|e| e.to_string())?;
        if input(source)? != (digest.clone(), length)
            || input(temporary.path())? != (digest.clone(), length)
        {
            return Err("copied input changed during original custody transfer".into());
        }
        temporary.as_file().sync_all().map_err(|e| e.to_string())?;
        temporary
            .persist_noclobber(&path)
            .map_err(|e| e.to_string())?;
    }
    if input(&path)? != (digest.clone(), length) {
        return Err("retained copied-input original differs; preserve bodies".into());
    }
    Ok((name, digest, length))
}

fn register(
    store: &mut Store,
    i: usize,
    member: &str,
    source: &Path,
    git_root: Option<String>,
    provenance: Value,
) -> Result<PathBuf, String> {
    relative_path(member)?;
    if provenance.is_null() {
        return Err("copied body requires real producer provenance".into());
    }
    let object = &store.registry.objects[i];
    store.stable(object)?;
    if object.cache_lease.is_none() || object.evidence.contains_key(member) {
        return Err(
            "copied body needs temporary ownership and cannot replace original evidence".into(),
        );
    }
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
        if relative.starts_with(".git/") || relative == ".git" {
            return Err("Git identity and unretained commits cannot be fixture bodies".into());
        }
    }
    let (name, digest, length) = original(store, source)?;
    if let Some(existing) = object.bodies.get(member) {
        if existing.original != name
            || existing.sha256 != digest
            || existing.length != length
            || existing.git_root != git_root
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
            "git_root":git_root, "provenance":provenance,
            "lifetime":"last-supported-holder-and-current-consumer-release",
            "original_custody":"retained", "historical_producer_outcome":"unchanged"
        }),
    )?;
    store.registry.objects[i].bodies.insert(
        member.into(),
        Body {
            intent,
            original: name.clone(),
            sha256: digest,
            length,
            git_root,
            attempts: vec![],
        },
    );
    store.save()?;
    Ok(store.directory.join(name))
}

impl TemporaryHost {
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
        let i = store
            .registry
            .objects
            .iter()
            .position(|o| o.path == object)
            .ok_or("copy destination has no exact allocation registration")?;
        let binding = store.registry.objects[i]
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
            &mut store,
            i,
            member,
            source,
            git_root,
            value!({"route":"actual-copy-producer", "recipe":recipe,"source":source}),
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
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Member {
    path: String,
    sha256: String,
    length: u64,
    git_root: Option<String>,
    recipe: String,
}

impl Manager {
    pub(in crate::automatic) fn adopt_fixture_bodies(
        &self,
        r: &mut Runner,
        target: &Path,
        plan_path: &str,
        report: &mut Value,
    ) -> Result<(), String> {
        if target != self.policy.coordinator_root {
            return Err("fixture custody requires surviving coordinator".into());
        }
        let bytes = crate::recovery::state_bytes(target, plan_path)?;
        let plan: Custody = decode(&bytes)?;
        if plan.schema != "chrono-fixture-body-custody/v1"
            || plan.head != self.anchor_head
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
                    || selected.members.is_empty()
                {
                    return Err("duplicate or empty fixture custody selection".into());
                }
                let policy = self
                    .policy
                    .retained_producers
                    .iter()
                    .find(|p| p.id == selected.producer && p.fixture_bodies)
                    .ok_or("fixture body class is not adopted for this producer")?;
                let directory = policy.directory(target)?;
                let mut store = Store::open(&directory, &selected.producer, false)?;
                let i = store
                    .registry
                    .objects
                    .iter()
                    .position(|o| o.path == selected.allocation)
                    .ok_or("unknown allocation is never imported by fixture custody")?;
                let object = &store.registry.objects[i];
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
                let allocation = directory.join(&selected.allocation);
                for member in selected.members {
                    let physical = no_symlink_parents(&allocation, &member.path)?;
                    if input(&physical)? != (member.sha256.clone(), member.length) {
                        return Err(format!("custody member differs: {}", member.path));
                    }
                    protect_source(r, &allocation, &member.path, member.git_root.as_deref())?;
                    relative_path(&member.recipe)?;
                    let source = r.blob(target, &self.anchor_head, &member.recipe)?;
                    self.stable(r)?;
                    lease.stable()?;
                    register(
                        &mut store,
                        i,
                        &member.path,
                        &physical,
                        member.git_root,
                        value!({"route":"current-custodian-transition", "custody":custody,
                        "reason":plan.reason, "recipe":member.recipe,"recipe_sha256":sha256(&source),
                        "candidate":self.anchor_head, "past_outcome":"unchanged"}),
                    )?;
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

pub(super) fn drain(
    policy: &ProducerPolicy,
    manager: &Manager,
    r: &mut Runner,
    directory: &Path,
    object: &Object,
    registry_id: &str,
    lease: &Lease,
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
            {
                return Err("copied body intent binding changed".into());
            }
            let original = no_symlink_parents(directory, &body.original)?;
            if input(&original)? != (body.sha256.clone(), body.length) {
                return Err("copied input original unavailable or changed".into());
            }
            let path = no_symlink_parents(&allocation, member)?;
            match fs::symlink_metadata(&path) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(value!({"path":member,"status":"already-absent"}));
                }
                Err(e) => return Err(e.to_string()),
                Ok(_) => (),
            }
            if input(&path)? != (body.sha256.clone(), body.length) {
                return Err("copied body differs or is partial; preserve original member".into());
            }
            protect_source(r, &allocation, member, body.git_root.as_deref())?;
            let footprint = artifact_disposal::artifact_footprint(&path)?;
            let before_id = crate::ownership::identity(&path)?;
            let effect = value!({"schema":"chrono-fixture-body-disposal-intent/v1", "allocation":object.path,
                "identity":object.identity,"member":member,"member_identity":before_id,"body_intent":body.intent,
                "footprint_before":footprint, "physical_release_bytes":Value::Null});
            let intent = Store::receipt_in(directory, "fixture-body-disposal-intent", &effect)?;
            manager.stable(r)?;
            lease.stable()?;
            Store::stable_in(directory, &policy.id, registry_id, object)?;
            if crate::ownership::identity(&path)? != before_id
                || input(&path)? != (body.sha256.clone(), body.length)
                || input(&original)? != (body.sha256.clone(), body.length)
            {
                return Err("copied body or original changed before removal".into());
            }
            protect_source(r, &allocation, member, body.git_root.as_deref())?;
            let mut result = value!({"path":member,"intent":intent,"footprint_before":footprint,"status":"failed","original":body.original,"physical_release_bytes":Value::Null});
            match fs::remove_file(&path) {
                Ok(()) if matches!(fs::symlink_metadata(&path), Err(e) if e.kind() == std::io::ErrorKind::NotFound) =>
                {
                    result["status"] = value!("verified-absent");
                    result["footprint_after"] =
                        value!(artifact_disposal::artifact_footprint(&path)?);
                }
                Ok(()) => result["error"] = value!("removed member absence unverified"),
                Err(e) => result["error"] = value!(e.to_string()),
            }
            result["receipt"] = value!(Store::receipt_in(
                directory,
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
    row["status"] = value!(if failures.is_empty() {
        "recovered"
    } else {
        "failed"
    });
    row["failures"] = value!(failures);
    Ok(row)
}
