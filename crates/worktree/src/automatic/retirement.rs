//! Retire only identity-bound capabilities and declared empty enrollment metadata.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn retire_history(disposed: bool, consumers_released: bool,
    confirmed_at: Option<u64>, observed: u64, keep_seconds: u64, over_limit: bool) -> (retire: bool)
    ensures retire <==> (disposed && consumers_released
        && confirmed_at.is_some()
        && confirmed_at.unwrap() <= observed
        && (over_limit || observed - confirmed_at.unwrap() >= keep_seconds)),
{
    match confirmed_at {
        Some(confirmed) => disposed && consumers_released
            && confirmed <= observed && (over_limit || observed - confirmed >= keep_seconds),
        None => false,
    }
}

pub fn retire_cache_history(completed: bool, generation: u64, current_generation: u64,
    consumers_released: bool, confirmed_at: Option<u64>, observed: u64,
    keep_seconds: u64, over_limit: bool) -> (retire: bool)
    ensures retire <==> (completed && generation < current_generation && consumers_released
        && confirmed_at.is_some() && confirmed_at.unwrap() <= observed
        && (over_limit || observed - confirmed_at.unwrap() >= keep_seconds)),
{
    retire_history(completed && generation < current_generation, consumers_released,
        confirmed_at, observed, keep_seconds, over_limit)
}
}

#[cfg(not(verus_keep_ghost))]
use super::*;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{artifact_disposal as disposal, ownership::DirectoryLease};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
struct Member {
    name: String,
    identity: String,
    sha256: String,
    length: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
struct Retirement {
    schema: String,
    coordinator_root: PathBuf,
    common: PathBuf,
    path: PathBuf,
    directory_identity: String,
    admission_id: String,
    members: Vec<Member>,
}
#[cfg(not(verus_keep_ghost))]
const MEMBERS: [&str; 3] = ["ledger.json", "owner.json", "metadata.lease"];

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
pub(super) struct RetiredLease {
    pub(super) ownership: Ownership,
    pub(super) creator_agent: Option<crate::AgentIdentity>,
    pub(super) branch: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<String>,
}
#[cfg(not(verus_keep_ghost))]
impl RetiredLease {
    pub(super) fn check_agent(&self) -> Result<(), String> {
        if self
            .creator_agent
            .as_ref()
            .is_some_and(|agent| !agent.valid())
        {
            return Err("invalid retired capability creator identity".into());
        }
        Ok(())
    }
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn absent(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(error.to_string()),
        Ok(_) => Ok(false),
    }
}
#[cfg(not(verus_keep_ghost))]
fn names(directory: &Path) -> Result<BTreeSet<String>, String> {
    fs::read_dir(directory)
        .map_err(|error| error.to_string())?
        .map(|entry| {
            entry
                .map_err(|error| error.to_string())?
                .file_name()
                .into_string()
                .map_err(|_| "enrollment member is not UTF-8".into())
        })
        .collect()
}
#[cfg(not(verus_keep_ghost))]
fn observe(directory: &Path, name: &str) -> Result<Member, String> {
    let path = no_symlink_parents(directory, name)?;
    let identity = crate::ownership::identity(&path)?;
    let bytes = fs::read(&path).map_err(|error| error.to_string())?;
    if crate::ownership::identity(&path)? != identity {
        return Err("enrollment retirement member changed".into());
    }
    Ok(Member {
        name: name.into(),
        identity,
        sha256: sha256(&bytes),
        length: bytes.len() as u64,
    })
}
#[cfg(not(verus_keep_ghost))]
fn check_member(directory: &Path, member: &Member) -> Result<bool, String> {
    let path = no_symlink_parents(directory, &member.name)?;
    if absent(&path)? {
        return Ok(false);
    }
    if fs::symlink_metadata(&path)
        .map_err(|error| error.to_string())?
        .len()
        != member.length
    {
        return Err(format!(
            "retirement member length changed; preserve {}",
            member.name
        ));
    }
    let actual = observe(directory, &member.name)?;
    if actual.identity != member.identity
        || actual.sha256 != member.sha256
        || actual.length != member.length
    {
        return Err(format!(
            "retirement member changed; preserve {}",
            member.name
        ));
    }
    Ok(true)
}
#[cfg(not(verus_keep_ghost))]
fn note(report: &mut Value, field: &str, value: Value) {
    if report[field].is_null() {
        report[field] = value!([]);
    }
    report[field].as_array_mut().unwrap().push(value);
}

#[cfg(not(verus_keep_ghost))]
impl Manager {
    // Transfer to this field and publish the ledger BEFORE dropping the final
    // history reference. An interrupted unlink therefore retains its owner.
    pub(super) fn retire_leases(
        &mut self,
        r: &mut Runner,
        report: &mut Value,
    ) -> Result<Vec<String>, String> {
        let mut failures = Vec::new();
        let mut index = 0;
        while index < self.ledger.retired_leases.len() {
            let pending = self.ledger.retired_leases[index].clone();
            let owner = &pending.ownership;
            let result: Result<&str, String> = (|| {
                let suffix = owner
                    .path
                    .strip_prefix(&format!(
                        "{}enrollment-",
                        self.inputs.policy.state_directory
                    ))
                    .and_then(|name| name.strip_suffix(".lease"))
                    .ok_or("retired capability is outside its declared owner")?;
                if suffix.len() != 64
                    || !suffix
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                    || owner.cache_pending
                {
                    return Err("retired capability has unresolved or invalid ownership".into());
                }
                let path = no_symlink_parents(&self.inputs.policy.coordinator_root, &owner.path)?;
                if absent(&path)? {
                    return Ok("already-absent");
                }
                let lease = Lease::acquire(&path, Some(&owner.id), false, true, None)?
                    .ok_or("retired capability still has live consumers")?;
                if fs::metadata(&path)
                    .map_err(|error| error.to_string())?
                    .len()
                    != 0
                {
                    return Err("retired capability contains unknown bytes".into());
                }
                disposal::dispose_entry(
                    &self.inputs.policy.coordinator_root,
                    &owner.path,
                    &format!("{}:file", owner.id),
                    || {
                        self.stable(r)?;
                        lease.stable()
                    },
                )?;
                if !absent(&path)? {
                    return Err("retired capability removal is unverified".into());
                }
                Ok("removed")
            })();
            match result {
                Ok(status) => {
                    self.ledger.retired_leases.remove(index);
                    self.save()?;
                    note(
                        report,
                        "lease_retirement",
                        value!({"path":owner.path,"status":status}),
                    );
                }
                Err(error) => {
                    self.ledger.retired_leases[index].error = Some(error.clone());
                    self.save()?;
                    failures.push(format!("{}: {error}", owner.path));
                    note(
                        report,
                        "lease_retirement",
                        value!({"path":owner.path,"status":"preserved","error":error}),
                    );
                    index += 1;
                }
            }
        }
        Ok(failures)
    }

    pub(super) fn retire_empty_address(
        &self,
        r: &mut Runner,
        report: &mut Value,
    ) -> Result<(), String> {
        if self._gate.directory.is_none()
            || !self.ledger.entries.is_empty()
            || !self.ledger.retired_leases.is_empty()
            || self.scope == self.inputs.policy.coordinator_root
            || !absent(&self.scope)?
        {
            return Ok(());
        }
        if r.inventory(&self.inputs.policy.coordinator_root)?
            .iter()
            .any(|row| {
                row.get("worktree")
                    .is_some_and(|path| Path::new(path) == self.scope)
            })
        {
            return Ok(());
        }
        self.stable(r)?;
        if names(&self.record_directory)? != MEMBERS.iter().map(|name| (*name).into()).collect() {
            return Err("empty enrollment has unknown members; preserve its metadata".into());
        }
        let retirement = Retirement {
            schema: "chrono-worktree-address-retirement/v1".into(),
            coordinator_root: self.inputs.policy.coordinator_root.clone(),
            common: self.inputs.common.clone(),
            path: self.scope.clone(),
            directory_identity: disposal::identity(&self.record_directory)?,
            admission_id: self
                .ledger
                .admission_id
                .clone()
                .ok_or("retirement lacks admission identity")?,
            members: MEMBERS
                .iter()
                .map(|name| observe(&self.record_directory, name))
                .collect::<Result<_, _>>()?,
        };
        write_json(
            &self.record_directory.join("retirement.json"),
            &retirement,
            true,
        )?;
        self.resume_address(
            r,
            &self.record_directory,
            self._gate.directory.as_ref().unwrap(),
            self._gate.lease.as_ref(),
            report,
        )
    }

    fn resume_address(
        &self,
        r: &mut Runner,
        directory: &Path,
        gate: &DirectoryLease,
        held: Option<&Lease>,
        report: &mut Value,
    ) -> Result<(), String> {
        resume_address(
            &self.inputs,
            &self.directory,
            &self._namespace,
            r,
            directory,
            gate,
            held,
            report,
        )
    }
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn resume_address(
    inputs: &PolicyInputs,
    owner_directory: &Path,
    namespace: &Lease,
    r: &mut Runner,
    directory: &Path,
    gate: &DirectoryLease,
    held: Option<&Lease>,
    report: &mut Value,
) -> Result<(), String> {
    let declaration = directory.join("retirement.json");
    let bytes = fs::read(&declaration).map_err(|error| error.to_string())?;
    let retirement: Retirement = decode(&bytes)?;
    absolute(&retirement.path, true)?;
    if retirement.schema != "chrono-worktree-address-retirement/v1"
        || retirement.coordinator_root != inputs.policy.coordinator_root
        || retirement.common != inputs.common
        || record_directory(owner_directory, &retirement.path)? != directory
        || retirement.path == inputs.policy.coordinator_root
        || disposal::identity(directory)? != retirement.directory_identity
        || retirement
            .members
            .iter()
            .map(|member| member.name.as_str())
            .collect::<Vec<_>>()
            != MEMBERS
        || retirement.members[2].identity != retirement.admission_id
    {
        return Err("enrollment retirement ownership mismatch; preserve metadata".into());
    }
    if !absent(&retirement.path)?
        || r.inventory(&inputs.policy.coordinator_root)?
            .iter()
            .any(|row| {
                row.get("worktree")
                    .is_some_and(|path| Path::new(path) == retirement.path)
            })
    {
        return Err("retiring enrollment address is present or Git registered".into());
    }
    let expected: BTreeSet<_> = MEMBERS
        .iter()
        .copied()
        .chain(["retirement.json"])
        .map(String::from)
        .collect();
    if !names(directory)?.is_subset(&expected) {
        return Err("retiring enrollment has unknown members; preserve metadata".into());
    }
    let metadata = directory.join("metadata.lease");
    let acquired;
    if let Some(held) = held {
        if held.id() != retirement.admission_id {
            return Err("retirement admission changed".into());
        }
        held.stable()?;
    } else if !absent(&metadata)? {
        acquired = Lease::acquire(&metadata, Some(&retirement.admission_id), false, true, None)?
            .ok_or("retiring enrollment still has live metadata consumers")?;
        acquired.stable()?;
    }
    inputs.stable(r)?;
    // Validate ALL remaining members before producing the first effect.
    for member in &retirement.members {
        check_member(directory, member)?;
    }
    let stable = || {
        namespace.stable()?;
        gate.stable()?;
        if fs::read(&declaration).map_err(|error| error.to_string())? != bytes {
            return Err("enrollment retirement declaration changed".into());
        }
        Ok(())
    };
    for member in &retirement.members {
        if !check_member(directory, member)? {
            continue;
        }
        disposal::dispose_entry(
            directory,
            &member.name,
            &format!("{}:file", member.identity),
            || {
                stable()?;
                if let Some(held) = held {
                    held.stable()?;
                }
                check_member(directory, member).and_then(|present| {
                    if present {
                        Ok(())
                    } else {
                        Err("retirement member disappeared".into())
                    }
                })
            },
        )?;
    }
    if names(directory)? != BTreeSet::from(["retirement.json".into()]) {
        return Err("retiring enrollment acquired an unknown member".into());
    }
    disposal::dispose_entry(
        directory,
        "retirement.json",
        &disposal::identity(&declaration)?,
        stable,
    )?;
    let parent = directory.parent().ok_or("retirement directory parent")?;
    disposal::dispose_entry(
        parent,
        directory
            .file_name()
            .unwrap()
            .to_str()
            .ok_or("retirement address UTF-8")?,
        &retirement.directory_identity,
        || {
            namespace.stable()?;
            gate.stable()
        },
    )?;
    if !absent(directory)? {
        return Err("retired enrollment removal is unverified".into());
    }
    note(
        report,
        "address_retirement",
        value!({"path":retirement.path,"record_directory":directory,"status":"removed"}),
    );
    Ok(())
}

// The declared enrollment namespace is the inventory, as in retention's
// transaction owner. No source/output directory discovery supplies members.
#[cfg(not(verus_keep_ghost))]
pub(super) fn resume_retired_addresses(
    inputs: &PolicyInputs,
    owner_directory: &Path,
    namespace: &Lease,
    r: &mut Runner,
    report: &mut Value,
) -> Result<Vec<String>, String> {
    if inputs.policy.schema != "chrono-worktree-automatic-cleanup/v2" {
        return Ok(vec![]);
    }
    let mut failures = vec![];
    let members = match fs::read_dir(owner_directory.join("enrollments")) {
        Ok(members) => members,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(failures),
        Err(error) => return Err(error.to_string()),
    };
    for entry in members {
        let entry = entry.map_err(|error| error.to_string())?;
        let directory = entry.path();
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            continue;
        }
        let members = names(&directory)?;
        if !members.is_empty() && !members.contains("retirement.json") {
            continue;
        }
        let result = (|| {
            let Some(gate) = DirectoryLease::acquire(&directory, true, None)? else {
                return Ok(());
            };
            if names(&directory)?.is_empty() {
                disposal::dispose_entry(
                    &owner_directory.join("enrollments"),
                    entry
                        .file_name()
                        .to_str()
                        .ok_or("enrollment address UTF-8")?,
                    &disposal::identity(&directory)?,
                    || {
                        namespace.stable()?;
                        gate.stable()
                    },
                )?;
                note(
                    report,
                    "address_retirement",
                    value!({"record_directory":directory,"status":"empty-directory-removed"}),
                );
                Ok(())
            } else {
                resume_address(
                    inputs,
                    owner_directory,
                    namespace,
                    r,
                    &directory,
                    &gate,
                    None,
                    report,
                )
            }
        })();
        if let Err(error) = result {
            failures.push(format!("{}: {error}", directory.display()));
            note(
                report,
                "address_retirement",
                value!({"record_directory":directory,"status":"preserved","error":error}),
            );
        }
    }
    Ok(failures)
}
