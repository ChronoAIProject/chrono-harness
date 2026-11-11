//! Opt-in lifecycle cleanup with explicit terminal and kernel-quiescent cache phases.
mod age;
mod cache_adoption;
mod migration;
mod owner_deployment;
mod producer;
mod retirement;
use crate::verified_lifecycle::{Disposition, select_artifact};
// Explicit file custody uses these same admission/enrollment capabilities, without
// draining unrelated entries or migrating a historical checkout's policy.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EvidenceExclusion {
    coordinator_root: PathBuf,
    config_path: String,
    state_directory: String,
}
pub(crate) struct EvidenceGuard {
    namespace: Lease,
    admission: Lease,
    enrollment: Lease,
    ledger_path: PathBuf,
    ledger_bytes: Vec<u8>,
}
impl EvidenceGuard {
    pub(crate) fn capabilities_stable(&self) -> Result<(), String> {
        self.namespace.stable()?;
        self.admission.stable()?;
        self.enrollment.stable()
    }
    pub(crate) fn stable(&self) -> Result<(), String> {
        self.capabilities_stable()?;
        if fs::read(&self.ledger_path).map_err(|e| e.to_string())? != self.ledger_bytes {
            return Err("evidence custody enrollment changed; preserve remaining objects".into());
        }
        Ok(())
    }
}
impl EvidenceExclusion {
    pub(crate) fn acquire(&self, r: &mut Runner, target: &Path) -> Result<EvidenceGuard, String> {
        let coordinator = absolute(&self.coordinator_root, false)?;
        let inventory = r.inventory(target)?;
        if inventory
            .first()
            .and_then(|row| row.get("worktree"))
            .map(PathBuf::from)
            .as_ref()
            != Some(&coordinator)
        {
            return Err("evidence coordinator must be this repository's actual Git main".into());
        }
        let (config, config_bytes) = crate::configuration(&coordinator, &self.config_path)?;
        let head = r.oid(&coordinator, "HEAD")?;
        if r.blob(&coordinator, &head, &self.config_path)? != config_bytes {
            return Err("evidence coordinator configuration differs from committed bytes".into());
        }
        let policy_path = config
            .automatic_cleanup
            .ok_or("evidence exclusion requires adopted cleanup")?;
        let policy_bytes =
            fs::read(no_symlink_parents(&coordinator, &policy_path)?).map_err(|e| e.to_string())?;
        let policy: Policy = decode(&policy_bytes)?;
        if r.blob(&coordinator, &head, &policy_path)? != policy_bytes
            || policy.schema != "chrono-worktree-automatic-cleanup/v2"
            || policy.state_directory != self.state_directory
            || !(policy.coordinator_root == coordinator
                || policy.coordinator_root == Path::new("git-main-worktree"))
        {
            return Err("evidence exclusion must bind the current committed kernel owner".into());
        }
        relative_path(self.state_directory.trim_end_matches('/'))?;
        if !self.state_directory.starts_with(".chrono-harness/state/")
            || !self.state_directory.ends_with('/')
        {
            return Err("invalid evidence owner state directory".into());
        }
        let namespace =
            no_symlink_parents(&coordinator, self.state_directory.trim_end_matches('/'))?;
        let declaration =
            json(&fs::read(namespace.join("namespace.json")).map_err(|e| e.to_string())?)?;
        let deployment = Lease::acquire(
            &namespace.join("deployment.lease"),
            Some(
                declaration["deployment_id"]
                    .as_str()
                    .ok_or("missing executable deployment identity")?,
            ),
            false,
            false,
            None,
        )?
        .ok_or("executable deployment excludes evidence custody")?;
        let directory = record_directory(&namespace, target)?;
        let owner = json(&fs::read(directory.join("owner.json")).map_err(|e| e.to_string())?)?;
        let common = absolute(&r.checkout_identity(&coordinator)?.common, false)?;
        if owner["schema"] != "chrono-worktree-enrollment-owner/v2"
            || owner["coordinator_root"] != value!(coordinator)
            || owner["common"] != value!(common)
            || owner["path"] != value!(target)
        {
            return Err("evidence coordinator ownership mismatch".into());
        }
        let admission = Lease::acquire(
            &directory.join("metadata.lease"),
            Some(
                owner["admission_id"]
                    .as_str()
                    .ok_or("missing admission identity")?,
            ),
            false,
            true,
            None,
        )?
        .ok_or("evidence admission has live consumers")?;
        let ledger_path = directory.join("ledger.json");
        let ledger_bytes = fs::read(&ledger_path).map_err(|e| e.to_string())?;
        let ledger: Ledger = decode(&ledger_bytes)?;
        if ledger.schema != "chrono-worktree-enrollment-state/v2"
            || ledger.coordinator_root != coordinator
            || ledger.common != common
            || ledger.admission_id.as_deref() != Some(admission.id())
        {
            return Err("evidence ledger ownership mismatch".into());
        }
        let entries: Vec<_> = ledger
            .entries
            .iter()
            .filter(|e| e.path == target && e.status != "disposed")
            .collect();
        if entries.len() != 1 {
            return Err("evidence custody requires one current enrollment".into());
        }
        let e = entries[0];
        let rows: Vec<_> = inventory
            .iter()
            .filter(|row| row.get("worktree").is_some_and(|p| Path::new(p) == target))
            .collect();
        if rows.len() != 1 || rows[0].contains_key("locked") || rows[0].contains_key("prunable") {
            return Err("evidence checkout has foreign or unknown Git ownership".into());
        }
        if e.status != "active" || e.terminal.is_some() || !e.uses.is_empty() {
            return Err(
                "evidence custody requires active enrollment without outstanding use tokens".into(),
            );
        }
        let observed = r.checkout_identity(target)?;
        let branch = if e.cache_only {
            observed.branch.clone()
        } else {
            maintenance::checkout_reference(e.current_branch())
        };
        for lock in [
            observed.metadata.join("index.lock"),
            observed.metadata.join("HEAD.lock"),
            common.join("packed-refs.lock"),
            common.join("config.lock"),
            common.join(format!("{branch}.lock")),
        ] {
            match fs::symlink_metadata(lock) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                Err(error) => return Err(error.to_string()),
                Ok(_) => return Err("evidence checkout has an active or unknown Git lock".into()),
            }
        }
        let actual = if e.cache_only {
            main_attachment(target, &observed.metadata)?
        } else {
            attachment(r, target)?
        };
        if actual != e.attachment
            || observed.branch != branch
            || absolute(&observed.common, false)? != common
        {
            return Err("evidence enrollment attachment changed".into());
        }
        let ownership = e
            .ownership
            .as_ref()
            .ok_or("evidence enrollment lacks kernel protection")?;
        let enrollment = Lease::acquire(
            &no_symlink_parents(&coordinator, &ownership.path)?,
            Some(&ownership.id),
            false,
            true,
            None,
        )?
        .ok_or("evidence enrollment has live consumers")?;
        let guard = EvidenceGuard {
            namespace: deployment,
            admission,
            enrollment,
            ledger_path,
            ledger_bytes,
        };
        guard.stable()?;
        Ok(guard)
    }
}
use crate::verified_owner_publication::{PolicyNamespace, policy_namespace_matches};
use crate::{
    Start, artifact_disposal,
    maintenance::{self, Cleanup, Retention},
    ownership::Lease,
    start::{self, Runner},
};
use chrono_harness::verified_identifiers::{equal_bytes, equal_text};
use chrono_harness::{CommandSpec, decode, facts, json, no_symlink_parents, relative_path, sha256};
use chrono_judge_registration::Registrations;
pub use producer::{RetainedArtifact, TemporaryHost};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json as value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    schema: String,
    coordinator_root: PathBuf,
    state_directory: String,
    retained_ref: String,
    retention: Retention,
    remove_branch: bool,
    allow_evidence_disposal: bool,
    #[serde(default)]
    main_cache_only: bool,
    #[serde(default)]
    retained_producers: Vec<producer::ProducerPolicy>,
    #[serde(default)]
    terminal_history: Option<TerminalHistory>,
    #[serde(default)]
    cache_history: Option<TerminalHistory>,
    #[serde(default)]
    age_policy: Option<age::AgePolicy>,
    artifacts: Vec<Artifact>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct TerminalHistory {
    keep_seconds: u64,
    max_entries: usize,
    max_bytes: u64,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    path: String,
    disposition: Disposition,
}
#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Attachment {
    gitfile_sha256: String,
    gitfile_id: String,
    metadata: PathBuf,
    metadata_id: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    path: String,
    sha256: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Attempt {
    path: String,
    token: String,
    receipt: Option<Receipt>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Terminal {
    head: String,
    retained_commit: String,
    dispose_evidence: bool,
    artifacts_only: bool,
    receipt: Receipt,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Ownership {
    path: String,
    id: String,
    cache_pending: bool,
    generation: u64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CacheAttempt {
    path: String,
    intent: Receipt,
    generation: u64,
    receipt: Option<Receipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    completed_at: Option<u64>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    path: PathBuf,
    #[serde(default)]
    creator_agent: Option<crate::AgentIdentity>,
    #[serde(default)]
    custodian_agent: Option<crate::AgentIdentity>,
    branch: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    checkout_branch: Option<String>,
    attachment: Attachment,
    policy_sha256: String,
    status: String,
    enrollment: Value,
    uses: Vec<String>,
    terminal: Option<Terminal>,
    attempts: Vec<Attempt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ownership: Option<Ownership>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    cache_attempts: Vec<CacheAttempt>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    policy_migrations: Vec<Receipt>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    main_cache_adoptions: Vec<Receipt>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    cache_only: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cleanup: Option<CleanupProgress>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    disposed_at: Option<u64>,
}
impl Entry {
    // Creation evidence keeps `branch`; only explicit migrate adopts a new checkout.
    fn current_branch(&self) -> &str {
        self.checkout_branch.as_deref().unwrap_or(&self.branch)
    }
    fn report_handoff_pending(&self) -> bool {
        ["completed_uses", "main_cache_releases"]
            .iter()
            .any(|field| {
                !self.enrollment[*field].is_null()
                    && self.enrollment[*field]
                        .as_array()
                        .is_none_or(|rows| !rows.is_empty())
            })
    }
    fn check_agents(&self) -> Result<(), String> {
        if self.creator_agent.as_ref().is_some_and(|a| !a.valid())
            || self.custodian_agent.as_ref().is_some_and(|a| !a.valid())
        {
            return Err("invalid recorded agent identity".into());
        }
        if self.enrollment["kind"] == "birth"
            && self.creator_agent
                != serde_json::from_value::<Option<crate::AgentIdentity>>(
                    self.enrollment["receipt"]["creator_agent"].clone(),
                )
                .map_err(|error| format!("invalid recorded creator identity: {error}"))?
        {
            return Err("enrollment creator differs from its original creation input".into());
        }
        if self.enrollment["kind"] != "birth" && self.creator_agent.is_some() {
            return Err("imported worktree cannot claim a historical creator".into());
        }
        Ok(())
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CleanupProgress {
    stage: crate::verified_lifecycle::CleanupStage,
    detail: String,
    report_path: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Ledger {
    schema: String,
    coordinator_root: PathBuf,
    common: PathBuf,
    entries: Vec<Entry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    admission_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    retired_leases: Vec<retirement::RetiredLease>,
}
struct Gate {
    path: PathBuf,
    token: String,
    lease: Option<Lease>,
    directory: Option<chrono_harness::ownership::DirectoryLease>,
}
impl Gate {
    fn stable(&self) -> Result<(), String> {
        if let Some(directory) = &self.directory {
            directory.stable()?;
        }
        if let Some(lease) = &self.lease {
            lease.stable()?;
        }
        Ok(())
    }
}
impl Drop for Gate {
    fn drop(&mut self) {
        if self.lease.is_none()
            && fs::read(self.path.join("owner")).ok().as_deref() == Some(self.token.as_bytes())
        {
            let _ = fs::remove_file(self.path.join("owner"));
            let _ = fs::remove_dir(&self.path);
        }
    }
}
struct PolicyInputs {
    policy: Policy,
    policy_path: String,
    policy_bytes: Vec<u8>,
    config_path: String,
    config_bytes: Vec<u8>,
    source_root: PathBuf,
    source_head: String,
    anchor_head: String,
    anchor_config_bytes: Vec<u8>,
    anchor_policy_path: String,
    anchor_policy_bytes: Vec<u8>,
    anchor_policy: Policy,
    anchor_filemap_digest: String,
    registrations: Arc<Registrations>,
    registry_digest: String,
    common: PathBuf,
}
struct Manager {
    inputs: PolicyInputs,
    ledger: Ledger,
    directory: PathBuf,
    scope: PathBuf,
    record_directory: PathBuf,
    // Shared by readers; exclusive only while replacing the executable used
    // by every enrollment. It never serializes independent metadata updates.
    _namespace: Lease,
    _gate: Gate,
}
fn record_directory(directory: &Path, target: &Path) -> Result<PathBuf, String> {
    let address = target.to_str().ok_or("worktree address is not UTF-8")?;
    no_symlink_parents(
        directory,
        &format!("enrollments/{}", sha256(address.as_bytes())),
    )
}
fn absolute(path: &Path, absent: bool) -> Result<PathBuf, String> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| !matches!(c, Component::RootDir | Component::Normal(_)))
    {
        return Err("automatic cleanup requires a literal absolute physical path".into());
    }
    let text = path
        .strip_prefix("/")
        .map_err(|e| e.to_string())?
        .to_str()
        .ok_or("path UTF-8")?;
    let p = no_symlink_parents(Path::new("/"), text)?;
    if !absent && fs::canonicalize(&p).map_err(|e| e.to_string())? != p {
        return Err("physical path changed".into());
    }
    Ok(p)
}
fn directory_id(path: &Path) -> Result<String, String> {
    let m = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if m.file_type().is_symlink() {
        return Err("attachment must not be a symlink".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(format!("{}:{}", m.dev(), m.ino()))
    }
    #[cfg(not(unix))]
    {
        Ok(format!(
            "{:?}:{}",
            m.created().map_err(|e| e.to_string())?,
            m.len()
        ))
    }
}
fn attachment(r: &mut Runner, target: &Path) -> Result<Attachment, String> {
    let metadata = r.text(target, &["rev-parse", "--absolute-git-dir"])?;
    attachment_at(target, Path::new(metadata.trim()))
}
fn attachment_at(target: &Path, metadata: &Path) -> Result<Attachment, String> {
    absolute(target, false)?;
    let gitfile = target.join(".git");
    if !fs::symlink_metadata(&gitfile)
        .map_err(|e| e.to_string())?
        .is_file()
    {
        return Err("only an existing linked worktree can enroll".into());
    }
    let metadata = absolute(metadata, false)?;
    Ok(Attachment {
        gitfile_sha256: sha256(&fs::read(&gitfile).map_err(|e| e.to_string())?),
        gitfile_id: directory_id(&gitfile)?,
        metadata_id: directory_id(&metadata)?,
        metadata,
    })
}
fn main_attachment(target: &Path, metadata: &Path) -> Result<Attachment, String> {
    let git = target.join(".git");
    if !fs::symlink_metadata(&git)
        .map_err(|e| e.to_string())?
        .is_dir()
        || absolute(&git, false)? != absolute(metadata, false)?
    {
        return Err("main cache ownership requires the physical Git main directory".into());
    }
    Ok(Attachment {
        gitfile_sha256: "physical-main-directory".into(),
        gitfile_id: directory_id(&git)?,
        metadata_id: directory_id(metadata)?,
        metadata: metadata.into(),
    })
}
fn write_json(path: &Path, v: &impl Serialize, immutable: bool) -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec_pretty(v).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    write_bytes(path, &bytes, immutable)?;
    Ok(bytes)
}
fn write_bytes(path: &Path, bytes: &[u8], immutable: bool) -> Result<(), String> {
    write_bytes_staged(
        path,
        bytes,
        immutable,
        path.parent().ok_or("publication parent")?,
    )
}
fn write_bytes_staged(
    path: &Path,
    bytes: &[u8],
    immutable: bool,
    staging_directory: &Path,
) -> Result<(), String> {
    let mut out = tempfile::Builder::new()
        .prefix(".publish-")
        .tempfile_in(staging_directory)
        .map_err(|e| e.to_string())?;
    out.write_all(bytes).map_err(|e| e.to_string())?;
    out.as_file().sync_all().map_err(|e| e.to_string())?;
    if immutable {
        out.persist_noclobber(path).map_err(|e| e.to_string())?;
    } else {
        out.persist(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn load_policy(
    r: &mut Runner,
    root: &Path,
    config_path: &str,
    bytes: &[u8],
) -> Result<Option<(Policy, String, Vec<u8>)>, String> {
    let config: crate::Config = decode(bytes)?;
    let Some(path) = config.automatic_cleanup else {
        return Ok(None);
    };
    relative_path(&path)?;
    if !path.starts_with(".chrono-harness/") || path.starts_with(".chrono-harness/state/") {
        return Err("automatic cleanup policy must be a tracked .chrono-harness input".into());
    }
    let head = r.oid(root, "HEAD")?;
    if r.blob(root, &head, config_path)? != bytes {
        return Err("worktree configuration differs from source commit".into());
    }
    let policy_bytes = fs::read(no_symlink_parents(root, &path)?).map_err(|e| e.to_string())?;
    if r.blob(root, &head, &path)? != policy_bytes {
        return Err("automatic cleanup policy differs from source commit".into());
    }
    let p = parse_policy(r, root, &policy_bytes)?;
    Ok(Some((p, path, policy_bytes)))
}
fn parse_policy(r: &mut Runner, root: &Path, policy_bytes: &[u8]) -> Result<Policy, String> {
    let mut p: Policy = decode(policy_bytes)?;
    if !matches!(
        p.schema.as_str(),
        "chrono-worktree-automatic-cleanup/v1" | "chrono-worktree-automatic-cleanup/v2"
    ) || !p.state_directory.starts_with(".chrono-harness/state/")
        || !p.state_directory.ends_with('/')
        || !p.retained_ref.starts_with("refs/heads/")
    {
        return Err("invalid automatic cleanup policy".into());
    }
    relative_path(p.state_directory.trim_end_matches('/'))?;
    if let Some(age) = &p.age_policy {
        age.validate(p.schema == "chrono-worktree-automatic-cleanup/v2")?;
    }
    if p.main_cache_only && p.schema != "chrono-worktree-automatic-cleanup/v2" {
        return Err("main cache ownership requires kernel cleanup v2".into());
    }
    if !p.retained_producers.is_empty() && p.schema != "chrono-worktree-automatic-cleanup/v2" {
        return Err("retained producer references require kernel cleanup v2".into());
    }
    for history in [&p.terminal_history, &p.cache_history]
        .into_iter()
        .flatten()
    {
        if p.schema != "chrono-worktree-automatic-cleanup/v2"
            || history.max_entries == 0
            || history.max_bytes == 0
        {
            return Err(
                "lifecycle history requires kernel cleanup v2 and positive record/byte limits"
                    .into(),
            );
        }
    }
    if p.coordinator_root == Path::new("git-main-worktree") {
        // Explicit host selector delegates physical identity to Git's existing owner inventory.
        p.coordinator_root = PathBuf::from(
            r.inventory(root)?
                .first()
                .and_then(|row| row.get("worktree"))
                .ok_or("Git main worktree identity is missing")?,
        );
    }
    absolute(&p.coordinator_root, false)?;
    let mut seen = BTreeSet::new();
    for a in &p.artifacts {
        if !a.path.ends_with('/') || !seen.insert(&a.path) {
            return Err("invalid or repeated automatic artifact path".into());
        }
        relative_path(a.path.trim_end_matches('/'))?;
    }
    for a in &p.artifacts {
        for b in &p.artifacts {
            if a.path != b.path && a.path.starts_with(&b.path) {
                return Err("overlapping automatic artifact selections".into());
            }
        }
    }
    r.git(root, &["check-ref-format", &p.retained_ref])?;
    if r.command(root, &["symbolic-ref", "--quiet", &p.retained_ref])?
        .exit_code
        != 1
    {
        return Err("automatic retention requires a direct registered local branch".into());
    }
    Ok(p)
}
fn policy_namespace(policy: &Policy) -> PolicyNamespace {
    PolicyNamespace {
        schema: policy.schema.clone(),
        coordinator: policy
            .coordinator_root
            .as_os_str()
            .as_encoded_bytes()
            .to_vec(),
        state_directory: policy.state_directory.clone(),
    }
}
impl PolicyInputs {
    fn report_source(&self, report: &mut Value) {
        report["source_root"] = value!(self.policy.coordinator_root);
        report["policy_source_root"] = value!(self.source_root);
        report["policy_source_commit"] = value!(self.source_head);
        report["lifecycle_coordinator_root"] = value!(self.policy.coordinator_root);
        report["source_commit"] = value!(self.anchor_head);
    }
    fn load(
        r: &mut Runner,
        root: &Path,
        config_path: &str,
        bytes: &[u8],
    ) -> Result<Option<Self>, String> {
        let Some((policy, policy_path, policy_bytes)) = load_policy(r, root, config_path, bytes)?
        else {
            return Ok(None);
        };
        let anchor = &policy.coordinator_root;
        let observed = r.checkout_identity(anchor)?;
        if absolute(&observed.top, false)? != *anchor {
            return Err("coordinator is not a Git checkout root".into());
        }
        let common = absolute(&observed.common, false)?;
        // One live checkout observation supplies both roles when the caller is
        // the coordinator. Distinct checkouts still establish their shared owner.
        if anchor != root {
            let root_common = fs::canonicalize(
                root.join(r.text(root, &["rev-parse", "--git-common-dir"])?.trim()),
            )
            .map_err(|e| e.to_string())?;
            if common != root_common {
                return Err("coordinator belongs to a different repository".into());
            }
        }
        let anchor_head = observed.head;
        let config: crate::Config = decode(bytes)?;
        let (
            anchor_config_bytes,
            anchor_policy,
            anchor_policy_path,
            anchor_policy_bytes,
            anchor_host_config,
        ) = if anchor == root {
            (
                bytes.to_vec(),
                policy.clone(),
                policy_path.clone(),
                policy_bytes.clone(),
                config.host_config.clone(),
            )
        } else {
            let (anchor_config, anchor_bytes) = crate::configuration(anchor, config_path)?;
            let (anchor_policy, anchor_path, anchor_policy_bytes) =
                load_policy(r, anchor, config_path, &anchor_bytes)?
                    .ok_or("coordinator cleanup ownership is not adopted")?;
            (
                anchor_bytes,
                anchor_policy,
                anchor_path,
                anchor_policy_bytes,
                anchor_config.host_config,
            )
        };
        if !policy_namespace_matches(
            &policy_namespace(&policy),
            &policy_namespace(&anchor_policy),
        ) {
            return Err("coordinator policy/configuration ownership namespace mismatch".into());
        }
        let (anchor_registrations, anchor_registry_digest) =
            start::registrations(r, anchor, &anchor_head, &anchor_host_config)?;
        let anchor_registrations = Arc::new(anchor_registrations);
        let anchor_filemap_digest =
            chrono_harness::wire::digest(&value!(anchor_registrations.filemap()))?;
        let source_head = if root == anchor {
            anchor_head.clone()
        } else {
            r.oid(root, "HEAD")?
        };
        let (registrations, registry_digest) = if root == anchor {
            (Arc::clone(&anchor_registrations), anchor_registry_digest)
        } else {
            let (regs, digest) = start::registrations(r, root, &source_head, &config.host_config)?;
            (Arc::new(regs), digest)
        };
        for path in [config_path, &anchor_policy_path] {
            start::registered_policy(&anchor_registrations, path, &anchor_policy.state_directory)?;
        }
        for path in [config_path, &policy_path] {
            start::registered_policy(&registrations, path, &policy.state_directory)?;
        }
        artifact_disposal::registered(
            &policy
                .artifacts
                .iter()
                .map(|a| a.path.clone())
                .collect::<Vec<_>>(),
            &[registrations.config()],
        )?;
        let mut producers = BTreeSet::new();
        for producer in &anchor_policy.retained_producers {
            producer.validate(&anchor_policy)?;
            if !producers.insert(&producer.id)
                || anchor_policy.retained_producers.iter().any(|other| {
                    other.id != producer.id
                        && (other.directory.starts_with(&producer.directory)
                            || producer.directory.starts_with(&other.directory))
                })
            {
                return Err("repeated or overlapping retained producer registrations".into());
            }
        }
        start::registered_policy(&registrations, config_path, &config.report_directory)?;
        Ok(Some(Self {
            policy,
            policy_path,
            policy_bytes,
            config_path: config_path.into(),
            config_bytes: bytes.into(),
            source_root: root.into(),
            source_head,
            anchor_head,
            anchor_config_bytes,
            anchor_policy_path,
            anchor_policy_bytes,
            anchor_policy,
            anchor_filemap_digest,
            registrations,
            registry_digest,
            common,
        }))
    }
    fn stable(&self, r: &mut Runner) -> Result<(), String> {
        let anchor = &self.policy.coordinator_root;
        let head = r.oid(anchor, "HEAD")?;
        let (config, bytes) = crate::configuration(anchor, &self.config_path)?;
        if !equal_bytes(&r.blob(anchor, &head, &self.config_path)?, &bytes) {
            return Err("coordinator configuration differs from committed bytes".into());
        }
        let path = config
            .automatic_cleanup
            .ok_or("coordinator cleanup ownership disappeared")?;
        let bytes = fs::read(no_symlink_parents(anchor, &path)?).map_err(|e| e.to_string())?;
        if !equal_bytes(&r.blob(anchor, &head, &path)?, &bytes) {
            return Err("coordinator cleanup policy differs from committed bytes".into());
        }
        let mut observed: Policy = decode(&bytes)?;
        if observed.coordinator_root == Path::new("git-main-worktree") {
            observed.coordinator_root = anchor.clone();
        }
        if !policy_namespace_matches(
            &policy_namespace(&self.anchor_policy),
            &policy_namespace(&observed),
        ) {
            return Err("coordinator ownership namespace changed; preserve pending work".into());
        }
        Ok(())
    }
    fn anchor_stable(&self, r: &mut Runner) -> Result<(), String> {
        let anchor = &self.policy.coordinator_root;
        if r.oid(anchor, "HEAD")? != self.anchor_head
            || fs::read(no_symlink_parents(anchor, &self.config_path)?)
                .map_err(|e| e.to_string())?
                != self.anchor_config_bytes
            || fs::read(no_symlink_parents(anchor, &self.anchor_policy_path)?)
                .map_err(|e| e.to_string())?
                != self.anchor_policy_bytes
        {
            return Err(
                "coordinator HEAD/configuration/policy changed; preserve pending work".into(),
            );
        }
        Ok(())
    }
}
struct Observation<'a> {
    inputs: &'a PolicyInputs,
    directory: &'a Path,
    scope: &'a Path,
}

fn observe_status(
    r: &mut Runner,
    inputs: &PolicyInputs,
    scope: &Path,
    report: &mut Value,
    all: bool,
) -> Result<(), String> {
    let anchor = &inputs.policy.coordinator_root;
    let directory =
        no_symlink_parents(anchor, inputs.policy.state_directory.trim_end_matches('/'))?;
    let namespace_path = no_symlink_parents(&directory, "namespace.json")?;
    let existing = match fs::symlink_metadata(&directory) {
        Ok(_) => true,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => return Err(e.to_string()),
    };
    if directory.join("ledger.json").exists() || directory.join("owner.json").exists() {
        return Err("aggregate cleanup state is retired; preserve originals and partition the current enrollments before adoption".into());
    }
    let namespace_bytes = if existing {
        Some(
            fs::read(&namespace_path)
                .map_err(|e| format!("unreadable enrollment namespace: {e}"))?,
        )
    } else {
        None
    };
    let namespace = if let Some(bytes) = &namespace_bytes {
        let declaration = json(bytes)?;
        let lease = Lease::acquire(
            &no_symlink_parents(&directory, "deployment.lease")?,
            Some(
                declaration["deployment_id"]
                    .as_str()
                    .ok_or("missing namespace deployment identity")?,
            ),
            false,
            false,
            Some(std::time::Duration::from_secs(r.config.timeout_seconds)),
        )?
        .ok_or("owner deployment is using the enrollment namespace")?;
        if declaration
            != value!({"schema":"chrono-worktree-enrollments/v1",
            "coordinator_root":anchor,"common":inputs.common,"deployment_id":lease.id()})
        {
            return Err(
                "enrollment namespace deployment identity changed; preserve records".into(),
            );
        }
        Some(lease)
    } else {
        None
    };
    let stable = |r: &mut Runner| -> Result<(), String> {
        inputs.anchor_stable(r)?;
        if inputs.source_root != inputs.policy.coordinator_root
            && (r.oid(&inputs.source_root, "HEAD")? != inputs.source_head
                || fs::read(no_symlink_parents(
                    &inputs.source_root,
                    &inputs.config_path,
                )?)
                .map_err(|e| e.to_string())?
                    != inputs.config_bytes
                || fs::read(no_symlink_parents(
                    &inputs.source_root,
                    &inputs.policy_path,
                )?)
                .map_err(|e| e.to_string())?
                    != inputs.policy_bytes)
        {
            return Err("status policy inputs changed during observation; preserve records".into());
        }
        if let Some(lease) = &namespace {
            lease.stable()?;
            if fs::read(&namespace_path).map_err(|e| e.to_string())?
                != *namespace_bytes.as_ref().unwrap()
            {
                return Err(
                    "enrollment namespace changed during observation; preserve records".into(),
                );
            }
        } else {
            match fs::symlink_metadata(&directory) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                _ => {
                    return Err(
                        "enrollment namespace appeared during observation; retry status".into(),
                    );
                }
            }
        }
        Ok(())
    };
    stable(r)?;
    Observation {
        inputs,
        directory: &directory,
        scope,
    }
    .observe_attention(r, report, all)?;
    stable(r)
}

impl Manager {
    fn observation(&self) -> Observation<'_> {
        Observation {
            inputs: &self.inputs,
            directory: &self.directory,
            scope: &self.scope,
        }
    }
    fn observe_inventory(
        &self,
        r: &mut Runner,
        exclude: &[PathBuf],
        report: &mut Value,
        all: bool,
        selected: &BTreeSet<PathBuf>,
    ) -> Result<BTreeSet<PathBuf>, String> {
        self.observation()
            .observe_inventory(r, exclude, report, all, selected)
    }
    fn enrollment_paths(&self) -> Result<Vec<PathBuf>, String> {
        self.observation().enrollment_paths()
    }
    fn observe_attention(
        &self,
        r: &mut Runner,
        report: &mut Value,
        all: bool,
    ) -> Result<(), String> {
        self.observation().observe_attention(r, report, all)
    }
}

impl Observation<'_> {
    /// Observe Git's explicit owner inventory without enrolling or leasing rows.
    /// This is diagnostic input, never authority for a later cleanup effect.
    fn observe_inventory(
        &self,
        r: &mut Runner,
        exclude: &[PathBuf],
        report: &mut Value,
        all: bool,
        selected: &BTreeSet<PathBuf>,
    ) -> Result<BTreeSet<PathBuf>, String> {
        report["worktree_inventory"] = value!([]);
        let mut addresses = BTreeSet::new();
        for git in r.inventory(&self.inputs.policy.coordinator_root)? {
            let path = PathBuf::from(&git["worktree"]);
            addresses.insert(path.clone());
            if !all && path != self.scope && !exclude.contains(&path) && !selected.contains(&path) {
                continue;
            }
            let snapshot = self.enrollment_snapshot(&path);
            let entry = snapshot
                .as_ref()
                .ok()
                .and_then(Option::as_ref)
                .and_then(|ledger| {
                    ledger
                        .entries
                        .iter()
                        .find(|entry| entry.status != "disposed")
                });
            let mut row = value!({"path":path,"head":git.get("HEAD"),"branch":git.get("branch"),
                "locked":git.contains_key("locked"),"prunable":git.contains_key("prunable"),
                "git":git,"enrollment_match":false});
            if let Some(e) = entry {
                row["enrollment_status"] = value!(e.status);
            }
            let observed = (|| {
                if let Err(error) = &snapshot {
                    return Err(error.clone());
                }
                absolute(&path, false)?;
                let checkout = r.checkout_identity(&path)?;
                row["checkout_identity"] = value!({"top":checkout.top,"common":checkout.common,
                    "metadata":checkout.metadata,"head":checkout.head,"branch":checkout.branch});
                if checkout.top != path || absolute(&checkout.common, false)? != self.inputs.common
                {
                    return Err(
                        "checkout attachment belongs to a different physical repository".into(),
                    );
                }
                let attached = if path == self.inputs.policy.coordinator_root {
                    main_attachment(&path, &checkout.metadata)?
                } else {
                    attachment_at(&path, &checkout.metadata)?
                };
                row["attachment"] = value!(attached);
                if let Some(e) = entry {
                    if attached != e.attachment {
                        return Err("current enrollment attachment changed".into());
                    }
                    // HEAD and branch are mutable checkout state, not birth identity.
                    row["enrollment_match"] = value!(true);
                }
                Ok::<(), String>(())
            })();
            if let Err(error) = &observed {
                row["attachment_error"] = value!(error);
            }
            let reason = if path == self.inputs.policy.coordinator_root {
                Some("coordinator worktree".to_owned())
            } else if exclude.contains(&path) {
                Some("invoking source or destination".to_owned())
            } else if entry.is_none() {
                Some(
                    "Git-registered worktree has no current enrollment; explicit import required"
                        .to_owned(),
                )
            } else if let Err(error) = &observed {
                Some(format!("unverified checkout attachment: {error}"))
            } else if git.contains_key("locked") || git.contains_key("prunable") {
                Some("Git worktree is locked or prunable; preserve its attachment".to_owned())
            } else {
                None
            };
            row["status"] = value!(if observed.is_err()
                && path != self.inputs.policy.coordinator_root
                && !exclude.contains(&path)
            {
                "blocked"
            } else if reason.is_some() {
                "preserved"
            } else {
                "enrolled"
            });
            if let Some(reason) = reason {
                row["preserved_reason"] = value!(reason);
            }
            report["worktree_inventory"]
                .as_array_mut()
                .unwrap()
                .push(row);
        }
        Ok(addresses)
    }
    fn enrollment_paths(&self) -> Result<Vec<PathBuf>, String> {
        let namespace = no_symlink_parents(&self.directory, "enrollments")?;
        let mut paths = Vec::new();
        let items = match fs::read_dir(namespace) {
            Ok(items) => items,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(paths),
            Err(e) => return Err(e.to_string()),
        };
        for item in items {
            let item = item.map_err(|e| e.to_string())?;
            if !item.file_type().map_err(|e| e.to_string())?.is_dir() {
                return Err("enrollment namespace contains an unknown member".into());
            }
            let marker = item.path().join("owner.json");
            let raw = match fs::read(&marker) {
                Ok(raw) => raw,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e.to_string()),
            };
            let owner = json(&raw)?;
            let path = PathBuf::from(
                owner["path"]
                    .as_str()
                    .ok_or("enrollment declaration lacks its worktree address")?,
            );
            if record_directory(&self.directory, &path)? != item.path() {
                return Err("enrollment declaration address differs from its namespace".into());
            }
            paths.push(path);
        }
        paths.sort();
        Ok(paths)
    }
    fn enrollment_snapshot(&self, path: &Path) -> Result<Option<Ledger>, String> {
        use crate::verified_attention::{
            SnapshotProtocol, snapshot_owned, snapshot_protocol, snapshot_stable,
        };
        let record = record_directory(self.directory, path)?;
        match fs::symlink_metadata(&record) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
            Ok(_) => (),
        }
        let identity = directory_id(&record)?;
        let marker = no_symlink_parents(&record, "owner.json")?;
        let file = no_symlink_parents(&record, "ledger.json")?;
        let owner_bytes =
            fs::read(&marker).map_err(|e| format!("unreadable enrollment owner: {e}"))?;
        let bytes = fs::read(&file).map_err(|e| format!("unreadable enrollment ledger: {e}"))?;
        let ledger: Ledger = decode(&bytes)?;
        let protocol = snapshot_protocol(&ledger.schema, ledger.admission_id.is_some());
        let (expected_owner, admission_matches) = match &protocol {
            SnapshotProtocol::Unleased => (
                value!({"schema":"chrono-worktree-enrollment-owner/v1",
                    "coordinator_root":self.inputs.policy.coordinator_root,"common":self.inputs.common,"path":path}),
                true,
            ),
            SnapshotProtocol::Kernel => {
                let admission = ledger
                    .admission_id
                    .as_deref()
                    .ok_or("missing enrollment metadata identity")?;
                let observed = chrono_harness::ownership::identity(&no_symlink_parents(
                    &record,
                    "metadata.lease",
                )?)?;
                (
                    value!({"schema":"chrono-worktree-enrollment-owner/v2",
                    "coordinator_root":self.inputs.policy.coordinator_root,"common":self.inputs.common,
                    "path":path,"admission_id":admission}),
                    observed == admission,
                )
            }
            SnapshotProtocol::Invalid => {
                return Err("unknown enrollment snapshot schema or admission identity".into());
            }
        };
        if !snapshot_owned(
            &protocol,
            json(&owner_bytes)? == expected_owner,
            ledger.coordinator_root == self.inputs.policy.coordinator_root,
            ledger.common == self.inputs.common,
            ledger.entries.iter().all(|entry| entry.path == path),
            admission_matches,
        ) {
            return Err("enrollment snapshot belongs to a different owner".into());
        }
        for entry in &ledger.entries {
            entry.check_agents()?;
        }
        for pending in &ledger.retired_leases {
            pending.check_agent()?;
        }
        if !snapshot_stable(
            directory_id(&record)? == identity,
            fs::read(&marker).map_err(|e| e.to_string())? == owner_bytes,
            fs::read(&file).map_err(|e| e.to_string())? == bytes,
        ) {
            return Err("enrollment snapshot changed during observation; retry status".into());
        }
        Ok(Some(ledger))
    }
    /// Project existing local state and Git inventory; never enroll or dispose.
    fn observe_attention(
        &self,
        r: &mut Runner,
        report: &mut Value,
        all: bool,
    ) -> Result<(), String> {
        use crate::verified_attention::{Attention, classify, needs_attention, pending_for_agent};
        let current = crate::AgentIdentity::from_report(report)?;
        let mut selected = BTreeSet::new();
        let mut unknown_creators = BTreeSet::new();
        let mut declared = BTreeSet::new();
        let mut empty_records = BTreeSet::new();
        let mut unreadable_records = 0;
        let mut namespace_error = None;
        // Read only the declared enrollment metadata to select this agent's work.
        // Other agents' physical checkouts are observed only by explicit all-status.
        if all || current.is_some() {
            match self.enrollment_paths() {
                Ok(paths) => {
                    for path in paths {
                        declared.insert(path.clone());
                        match self.enrollment_snapshot(&path) {
                            Ok(Some(ledger)) => {
                                for pending in &ledger.retired_leases {
                                    if pending.creator_agent.is_none() {
                                        unknown_creators.insert(path.clone());
                                    }
                                    if all
                                        || pending_for_agent(
                                            pending
                                                .creator_agent
                                                .as_ref()
                                                .map(|a| a.id.as_str())
                                                .unwrap_or(""),
                                            current.as_ref().map(|a| a.id.as_str()).unwrap_or(""),
                                            true,
                                        )
                                    {
                                        selected.insert(path.clone());
                                    }
                                }
                                let entry = ledger
                                    .entries
                                    .iter()
                                    .find(|e| e.status != "disposed")
                                    .or_else(|| ledger.entries.last());
                                if let Some(entry) = entry {
                                    if path != self.inputs.policy.coordinator_root
                                        && entry.status != "disposed"
                                        && entry.creator_agent.is_none()
                                    {
                                        unknown_creators.insert(path.clone());
                                    }
                                    if all
                                        || pending_for_agent(
                                            entry
                                                .creator_agent
                                                .as_ref()
                                                .map(|a| a.id.as_str())
                                                .unwrap_or(""),
                                            current.as_ref().map(|a| a.id.as_str()).unwrap_or(""),
                                            true,
                                        )
                                    {
                                        selected.insert(path);
                                    }
                                } else {
                                    if all {
                                        selected.insert(path.clone());
                                    }
                                    empty_records.insert(path);
                                }
                            }
                            Ok(None) => {
                                unknown_creators.insert(path.clone());
                                if all {
                                    selected.insert(path);
                                }
                            }
                            Err(_) => {
                                unreadable_records += 1;
                                if all {
                                    selected.insert(path);
                                }
                            }
                        }
                    }
                }
                Err(error) => namespace_error = Some(error),
            }
        }
        let mut inventory = value!({});
        let git_addresses = self.observe_inventory(r, &[], &mut inventory, all, &selected)?;
        for path in git_addresses {
            if path != self.inputs.policy.coordinator_root
                && (!declared.contains(&path) || empty_records.contains(&path))
            {
                unknown_creators.insert(path);
            }
        }
        let git_rows: BTreeMap<PathBuf, Value> = inventory["worktree_inventory"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| (PathBuf::from(row["path"].as_str().unwrap()), row.clone()))
            .collect();
        let mut paths: BTreeSet<PathBuf> = git_rows.keys().cloned().collect();
        let mut rows = Vec::new();
        paths.extend(selected);
        paths.insert(self.scope.to_path_buf());
        if let Some(error) = &namespace_error {
            rows.push(value!({"path":null,"work_state":"unknown",
                "cleanup_state":"unknown","attention_required":true,
                "agent_attention_required":false,
                "next_action":"inspect-enrollment-namespace","error":error}));
        }
        for path in paths {
            let snapshot = self.enrollment_snapshot(&path);
            let entry = snapshot
                .as_ref()
                .ok()
                .and_then(Option::as_ref)
                .and_then(|ledger| {
                    ledger
                        .entries
                        .iter()
                        .find(|e| e.status != "disposed")
                        .or_else(|| ledger.entries.last())
                });
            let physical = fs::symlink_metadata(&path);
            let physical_readable = physical
                .as_ref()
                .map(|_| true)
                .unwrap_or_else(|e| e.kind() == std::io::ErrorKind::NotFound);
            let git = git_rows.get(&path);
            let error = snapshot
                .as_ref()
                .err()
                .cloned()
                .or_else(|| {
                    physical
                        .as_ref()
                        .err()
                        .filter(|e| e.kind() != std::io::ErrorKind::NotFound)
                        .map(ToString::to_string)
                })
                .or_else(|| {
                    git.and_then(|g| g["attachment_error"].as_str())
                        .map(str::to_owned)
                });
            let cleanup = entry.map(|e| e.status.as_str()).unwrap_or("unregistered");
            let view = classify(
                cleanup,
                path == self.inputs.policy.coordinator_root,
                snapshot.is_ok() && physical_readable && error.is_none(),
                entry.is_some(),
                physical.is_ok(),
                git.is_some(),
                git.is_some_and(|g| g["enrollment_match"] == true),
                entry.is_some_and(|e| e.terminal.is_some()),
            );
            // This match only encodes the proved classification for JSON/CLI.
            let (work, action) = match &view {
                Attention::Coordinator => ("coordinator", "none"),
                Attention::Active => ("active", "complete-work-and-finish"),
                Attention::TerminalPending => ("terminal", "maintain"),
                Attention::Retained => ("terminal", "resolve-retention-and-finish"),
                Attention::Disposed => ("terminal", "none"),
                Attention::Unregistered => ("unregistered", "inspect-and-import"),
                Attention::Unknown => ("unknown", "inspect-original-state"),
            };
            let creator = entry.and_then(|e| e.creator_agent.as_ref());
            let report_handoff_pending = entry.is_some_and(Entry::report_handoff_pending);
            let metadata_retirement_pending = path != self.inputs.policy.coordinator_root
                && entry.is_none()
                && physical
                    .as_ref()
                    .err()
                    .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound)
                && git.is_none()
                && snapshot.as_ref().ok().and_then(Option::as_ref).is_some();
            if metadata_retirement_pending
                && snapshot
                    .as_ref()
                    .ok()
                    .and_then(Option::as_ref)
                    .is_some_and(|ledger| ledger.retired_leases.is_empty())
            {
                unknown_creators.insert(path.clone());
            }
            let attention_required = needs_attention(&view) || report_handoff_pending;
            let action = if report_handoff_pending {
                "recover-result-handoff"
            } else if metadata_retirement_pending {
                "maintain-metadata"
            } else {
                action
            };
            let agent_attention = pending_for_agent(
                creator.map(|a| a.id.as_str()).unwrap_or(""),
                current.as_ref().map(|a| a.id.as_str()).unwrap_or(""),
                attention_required,
            );
            rows.push(value!({"path":path,"work_state":work,"cleanup_state":cleanup,
                "branch":entry.map(Entry::current_branch),"creator_agent":creator,
                "custodian_agent":entry.and_then(|e| e.custodian_agent.as_ref()),
                "cleanup_progress":entry.and_then(|e| e.cleanup.as_ref()),
                "agent_attention_required":agent_attention,
                "attention_required":attention_required,"next_action":action,
                "report_handoff_pending":report_handoff_pending,
                "metadata_retirement_pending":metadata_retirement_pending,
                "ledger_path":record_directory(&self.directory, &path)?.join("ledger.json"),
                "physical_present":physical.is_ok(),"git_registered":git.is_some(),
                "terminal_receipt":entry.and_then(|e| e.terminal.as_ref()).map(|t| &t.receipt),
                "cleanup_receipt":entry.and_then(|e| e.attempts.last()).and_then(|a| a.receipt.as_ref()),
                "error":error}));
            if let Some(ledger) = snapshot.as_ref().ok().and_then(Option::as_ref) {
                for pending in &ledger.retired_leases {
                    let agent_attention = pending_for_agent(
                        pending
                            .creator_agent
                            .as_ref()
                            .map(|a| a.id.as_str())
                            .unwrap_or(""),
                        current.as_ref().map(|a| a.id.as_str()).unwrap_or(""),
                        true,
                    );
                    rows.push(value!({"path":path,"record_kind":"retired-lease",
                        "work_state":"terminal","cleanup_state":"capability-retirement-pending",
                        "branch":pending.branch,"creator_agent":pending.creator_agent,
                        "retired_lease_path":pending.ownership.path,"error":pending.error,
                        "attention_required":true,"agent_attention_required":agent_attention,
                        "next_action":"maintain-retired-lease"}));
                }
            }
        }
        report["worktree_status"] = value!({"schema":"chrono-worktree-status/v1",
            "scope":if all {"coordinator"} else if current.is_some() {"target-and-current-agent"} else {"target"},
            "agent":report["agent"],"unknown_creator_records":unknown_creators.len(),
            "unreadable_records":unreadable_records,"namespace_error":namespace_error,
            "agent_observation_complete":current.is_some() && unknown_creators.is_empty()
                && unreadable_records == 0 && namespace_error.is_none(),"worktrees":rows});
        Ok(())
    }
}

impl Manager {
    fn protect_reports(&self, r: &mut Runner, invoking: &Path) -> Result<(), String> {
        let coordinator = &self.inputs.policy.coordinator_root;
        if coordinator != invoking && r.coordinator_inputs.is_none() {
            // Maintain a distinct coordinator once and retain only this
            // invocation's explicitly read originals until publication.
            r.coordinator_inputs = Some(chrono_harness::retained_artifacts::retention::read_scope(
                coordinator,
            )?);
        }
        Ok(())
    }

    fn open(
        r: &mut Runner,
        root: &Path,
        config_path: &str,
        bytes: &[u8],
        token: &str,
        scope: &Path,
        exclusive_namespace: bool,
        wait: bool,
    ) -> Result<Option<Self>, String> {
        let Some(mut manager) = Self::open_metadata(
            r,
            root,
            config_path,
            bytes,
            token,
            scope,
            exclusive_namespace,
            wait,
        )?
        else {
            return Ok(None);
        };
        manager.restore_absent_policy(r)?;
        Ok(Some(manager))
    }
    // Metadata ownership and its durable consumers do not depend on whether an
    // absent checkout's adopted policy can still be interpreted.
    fn open_metadata(
        r: &mut Runner,
        root: &Path,
        config_path: &str,
        bytes: &[u8],
        token: &str,
        scope: &Path,
        exclusive_namespace: bool,
        wait: bool,
    ) -> Result<Option<Self>, String> {
        let Some(inputs) = PolicyInputs::load(r, root, config_path, bytes)? else {
            return Ok(None);
        };
        Self::from_inputs(r, inputs, token, scope, exclusive_namespace, wait).map(Some)
    }
    fn from_inputs(
        r: &mut Runner,
        inputs: PolicyInputs,
        token: &str,
        scope: &Path,
        exclusive_namespace: bool,
        wait: bool,
    ) -> Result<Self, String> {
        let scope = absolute(scope, true)?;
        let policy = &inputs.policy;
        let anchor = &policy.coordinator_root;
        let common = inputs.common.clone();
        let directory = no_symlink_parents(anchor, policy.state_directory.trim_end_matches('/'))?;
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        if directory.join("ledger.json").exists() || directory.join("owner.json").exists() {
            return Err("aggregate cleanup state is retired; preserve originals and partition the current enrollments before adoption".into());
        }
        let namespace = Lease::acquire(
            &directory.join("deployment.lease"),
            None,
            true,
            exclusive_namespace,
            wait.then_some(std::time::Duration::from_secs(r.config.timeout_seconds)),
        )?
        .ok_or("owner deployment is using the enrollment namespace")?;
        let declaration = value!({"schema":"chrono-worktree-enrollments/v1",
            "coordinator_root":anchor,"common":common,"deployment_id":namespace.id()});
        let namespace_path = no_symlink_parents(&directory, "namespace.json")?;
        if !namespace_path.exists() {
            if let Err(error) = write_json(&namespace_path, &declaration, true) {
                if fs::read(&namespace_path)
                    .ok()
                    .and_then(|raw| json(&raw).ok())
                    .as_ref()
                    != Some(&declaration)
                {
                    return Err(error);
                }
            }
        }
        if json(&fs::read(&namespace_path).map_err(|e| e.to_string())?)? != declaration {
            return Err(
                "enrollment namespace deployment identity changed; preserve records".into(),
            );
        }
        let record_directory = record_directory(&directory, &scope)?;
        fs::create_dir_all(&record_directory).map_err(|e| e.to_string())?;
        let kernel = policy.schema == "chrono-worktree-automatic-cleanup/v2";
        let directory_lease = if kernel {
            Some(
                chrono_harness::ownership::DirectoryLease::acquire(
                    &record_directory,
                    true,
                    wait.then_some(std::time::Duration::from_secs(r.config.timeout_seconds)),
                )?
                .ok_or("enrollment metadata is busy")?,
            )
        } else {
            None
        };
        if record_directory.join("retirement.json").exists() {
            return Err("enrollment metadata retirement is pending; resume maintain".into());
        }
        let ledger_path = no_symlink_parents(&record_directory, "ledger.json")?;
        let marker = no_symlink_parents(&record_directory, "owner.json")?;
        let gate_path = record_directory.join("gate.lock");
        let gate = if kernel {
            if gate_path.exists() {
                return Err("unknown enrollment gate; preserve its records".into());
            }
            let owner = if marker.exists() {
                Some(json(&fs::read(&marker).map_err(|e| e.to_string())?)?)
            } else {
                None
            };
            let expected = owner
                .as_ref()
                .map(|v| {
                    v["admission_id"]
                        .as_str()
                        .ok_or("missing enrollment metadata identity")
                })
                .transpose()?;
            let path = record_directory.join("metadata.lease");
            let lease = Lease::acquire(
                &path,
                expected,
                expected.is_none(),
                true,
                wait.then_some(std::time::Duration::from_secs(r.config.timeout_seconds)),
            )?
            .ok_or("enrollment metadata is busy")?;
            Gate {
                path,
                token: token.into(),
                lease: Some(lease),
                directory: directory_lease,
            }
        } else {
            let waiting = std::time::Instant::now();
            loop {
                match fs::create_dir(&gate_path) {
                    Ok(()) => break,
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                        if !wait
                            || waiting.elapsed()
                                >= std::time::Duration::from_secs(r.config.timeout_seconds)
                        {
                            return Err("enrollment metadata remains busy or interrupted; no lock was expired".into());
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }
            let gate = Gate {
                path: gate_path,
                token: token.into(),
                lease: None,
                directory: None,
            };
            fs::write(gate.path.join("owner"), token).map_err(|e| e.to_string())?;
            gate
        };
        let schema = if kernel {
            "chrono-worktree-enrollment-state/v2"
        } else {
            "chrono-worktree-enrollment-state/v1"
        };
        let admission_id = gate.lease.as_ref().map(|l| l.id().to_owned());
        let expected_owner = if kernel {
            value!({"schema":"chrono-worktree-enrollment-owner/v2","coordinator_root":anchor,"common":common,"path":scope,"admission_id":admission_id})
        } else {
            value!({"schema":"chrono-worktree-enrollment-owner/v1","coordinator_root":anchor,"common":common,"path":scope})
        };
        let ledger: Ledger = match fs::read(&ledger_path) {
            Ok(b) => decode(&b)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && !marker.exists() => {
                let ledger = Ledger {
                    schema: schema.into(),
                    coordinator_root: anchor.clone(),
                    common: common.clone(),
                    entries: vec![],
                    admission_id: admission_id.clone(),
                    retired_leases: vec![],
                };
                if !kernel {
                    write_json(&marker, &expected_owner, true)?;
                }
                write_json(&ledger_path, &ledger, true)?;
                ledger
            }
            Err(e) => {
                return Err(format!(
                    "missing or unreadable cleanup ledger; preserve enrolled work: {e}"
                ));
            }
        };
        if ledger.schema != schema
            || ledger.coordinator_root != *anchor
            || ledger.common != common
            || ledger.admission_id != admission_id
        {
            return Err(
                "cleanup ledger/ownership identity mismatch; preserve legacy records".into(),
            );
        }
        if kernel && !marker.exists() && ledger.entries.is_empty() {
            write_json(&marker, &expected_owner, true)?;
        }
        if json(&fs::read(&marker).map_err(|e| e.to_string())?)? != expected_owner {
            return Err("cleanup owner identity mismatch".into());
        }
        // Entries are successive physical attachments, with disposed generations
        // retained in order. A pathname may have only one unresolved generation.
        let mut paths = BTreeMap::new();
        for pending in &ledger.retired_leases {
            pending.check_agent()?;
            if !kernel {
                return Err("retired capabilities require kernel ownership".into());
            }
        }
        for e in &ledger.entries {
            if e.path != scope {
                return Err("enrollment record contains another worktree".into());
            }
            e.check_agents()?;
            if e.cache_only && (e.path != *anchor || e.status != "active" || e.terminal.is_some()) {
                return Err(
                    "main cache enrollment cannot authorize terminal worktree disposal".into(),
                );
            }
            if paths
                .insert(&e.path, e.status.as_str())
                .is_some_and(|previous| previous != "disposed")
                || !matches!(
                    e.status.as_str(),
                    "active" | "terminal" | "disposed" | "retained"
                )
                || (e.status == "active") != e.terminal.is_none()
                || (e.status == "disposed" && !e.uses.is_empty())
            {
                return Err("invalid cleanup enrollment state".into());
            }
            absolute(&e.path, true)?;
        }
        let manager = Self {
            inputs,
            ledger,
            directory,
            scope,
            record_directory,
            _namespace: namespace,
            _gate: gate,
        };
        manager.protect_originals()?;
        Ok(manager)
    }
    /// The enrollment is an actual durable consumer, independently of any
    /// invocation's read scope. These are the fields owned by this schema;
    /// neither JSON traversal nor a directory scan defines dependencies.
    fn original_paths(&self, ledger: &Ledger) -> Result<BTreeSet<String>, String> {
        let mut paths = BTreeSet::new();
        for e in &ledger.entries {
            if e.enrollment["receipt"]["source_root"].is_null()
                || e.enrollment["receipt"]["source_root"]
                    == value!(self.inputs.policy.coordinator_root)
            {
                if let Some(path) = e.enrollment["receipt"]["report_path"].as_str() {
                    paths.insert(path.into());
                }
            }
            for field in ["sealed_receipt", "recovered_receipt", "cache_release"] {
                if !e.enrollment[field].is_null() {
                    let receipt: Receipt = serde_json::from_value(e.enrollment[field].clone())
                        .map_err(|error| format!("invalid enrollment {field}: {error}"))?;
                    paths.insert(receipt.path);
                }
            }
            for field in [
                "interrupted_uses",
                "completed_uses",
                "main_cache_releases",
                "terminal_history",
            ] {
                if !e.enrollment[field].is_null() {
                    let receipts: Vec<Receipt> =
                        serde_json::from_value(e.enrollment[field].clone())
                            .map_err(|error| format!("invalid enrollment {field}: {error}"))?;
                    paths.extend(receipts.into_iter().map(|receipt| receipt.path));
                }
            }
            if let Some(terminal) = &e.terminal {
                paths.insert(terminal.receipt.path.clone());
            }
            for attempt in &e.attempts {
                paths.insert(attempt.path.clone());
            }
            for attempt in &e.cache_attempts {
                paths.insert(attempt.intent.path.clone());
                paths.insert(attempt.path.clone());
            }
            paths.extend(e.policy_migrations.iter().map(|r| r.path.clone()));
            paths.extend(e.main_cache_adoptions.iter().map(|r| r.path.clone()));
            if let Some(path) = e.cleanup.as_ref().and_then(|p| p.report_path.as_ref()) {
                paths.insert(path.clone());
            }
            for token in &e.uses {
                relative_path(token)?;
                for prefix in ["use-intent-", "use-"] {
                    paths.insert(format!(
                        "{}{prefix}{token}.json",
                        self.inputs.policy.state_directory
                    ));
                }
            }
        }
        Ok(paths)
    }
    fn original_role(&self) -> Result<String, String> {
        Ok(format!(
            "worktree-enrollment:{}",
            sha256(
                self.scope
                    .to_str()
                    .ok_or("enrollment address UTF-8")?
                    .as_bytes()
            )
        ))
    }
    fn protect_originals(&self) -> Result<(), String> {
        if let Some(inventory) = chrono_harness::retained_artifacts::retention::Inventory::adopted(
            &self.inputs.policy.coordinator_root,
        )? {
            inventory.reference_paths(
                &self.original_role()?,
                &self
                    .original_paths(&self.ledger)?
                    .into_iter()
                    .collect::<Vec<_>>(),
            )?;
        }
        Ok(())
    }
    /// These are pending report handoffs, not an additional command history.
    /// A full original accepting this receipt ends only its interim ledger
    /// reference. The report's own consumers and dependency edges remain.
    fn reconcile_published_results(&mut self) -> Result<(), String> {
        let mut changed = false;
        for i in 0..self.ledger.entries.len() {
            for (field, binding) in [
                ("completed_uses", "managed_use_receipt"),
                ("main_cache_releases", "main_cache_release"),
            ] {
                if self.ledger.entries[i].enrollment[field].is_null() {
                    continue;
                }
                let receipts: Vec<Receipt> =
                    serde_json::from_value(self.ledger.entries[i].enrollment[field].clone())
                        .map_err(|e| format!("invalid pending result handoff: {e}"))?;
                let mut pending = Vec::new();
                for receipt in receipts {
                    let original = json(&self.receipt(&receipt)?)?;
                    let accepted = if let Some(path) = original["report_path"].as_str() {
                        let physical =
                            no_symlink_parents(&self.inputs.policy.coordinator_root, path)?;
                        let _input = chrono_harness::retained_artifacts::retention::read_guard(
                            &self.inputs.policy.coordinator_root,
                            path,
                        )?;
                        match fs::read(physical) {
                            Ok(bytes) => {
                                if let Ok(report) = json(&bytes) {
                                    let mut matches = report["schema"]
                                        == "chrono-worktree-report/v1"
                                        && report["report_path"] == path
                                        && matches!(
                                            report["status"].as_str(),
                                            Some("used" | "finished" | "failed")
                                        )
                                        && report[binding] == value!(receipt);
                                    if matches {
                                        if let Some(inventory) = chrono_harness::retained_artifacts::retention::Inventory::adopted(&self.inputs.policy.coordinator_root)? {
                                        matches = inventory.published_original(path, &sha256(&bytes), bytes.len() as u64)?;
                                    }
                                    }
                                    if matches {
                                        crate::recovery::bind_report_inputs(
                                            &self.inputs.policy.coordinator_root,
                                            &report,
                                        )?;
                                    }
                                    matches
                                } else {
                                    false
                                }
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
                            Err(e) => return Err(e.to_string()),
                        }
                    } else {
                        false
                    };
                    if accepted {
                        changed = true;
                    } else {
                        pending.push(receipt);
                    }
                }
                if pending.is_empty() {
                    self.ledger.entries[i]
                        .enrollment
                        .as_object_mut()
                        .ok_or("enrollment object")?
                        .remove(field);
                } else {
                    self.ledger.entries[i].enrollment[field] = value!(pending);
                }
            }
        }
        if changed {
            self.save()?;
        }
        Ok(())
    }

    fn adopt_live_policy(&mut self, r: &mut Runner) -> Result<(), String> {
        if self.scope == self.inputs.source_root {
            return Ok(());
        }
        // A target without its own cleanup policy keeps the invoking policy.
        let path = no_symlink_parents(&self.scope, &self.inputs.config_path)?;
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.to_string()),
        };
        if let Some(inputs) = PolicyInputs::load(r, &self.scope, &self.inputs.config_path, &bytes)?
        {
            if !policy_namespace_matches(
                &policy_namespace(&self.inputs.policy),
                &policy_namespace(&inputs.policy),
            ) {
                return Err(
                    "selected worktree ownership namespace differs from invoking host".into(),
                );
            }
            self.inputs = inputs;
        }
        Ok(())
    }

    /// Interrupted removal still uses this object's fixed adopted inputs, even
    /// after its physical checkout has gone. Only candidate code executes.
    fn restore_absent_policy(&mut self, r: &mut Runner) -> Result<(), String> {
        if self.scope.exists() {
            return Ok(());
        }
        let Some(i) = self.current_entry(&self.scope) else {
            return Ok(());
        };
        let entry = &self.ledger.entries[i];
        let mut binding = self.policy_binding(entry)?;
        let root = self.inputs.policy.coordinator_root.clone();
        // An import adopts the caller's policy while retaining the target's raw
        // input bindings. Those are separate sources until an explicit migration.
        let mut _original_guard = None;
        match crate::verified_lifecycle::absent_policy_source(
            entry.enrollment["kind"].as_str().unwrap_or(""),
            !entry.policy_migrations.is_empty(),
            &binding.head,
            &self.inputs.source_head,
            &binding.policy_sha256,
            &sha256(&self.inputs.policy_bytes),
        ) {
            crate::verified_lifecycle::AbsentPolicySource::Current => return Ok(()),
            crate::verified_lifecycle::AbsentPolicySource::RetainedTarget => (),
            crate::verified_lifecycle::AbsentPolicySource::ImportedOriginal => {
                let path = entry.enrollment["receipt"]["report_path"]
                    .as_str()
                    .ok_or("import enrollment lacks its original report path")?;
                _original_guard =
                    chrono_harness::retained_artifacts::retention::read_guard(&root, path)?;
                let bytes = crate::recovery::state_bytes(&root, path)?;
                let inventory =
                    chrono_harness::retained_artifacts::retention::Inventory::adopted(&root)?
                        .ok_or("import policy restoration requires its published original")?;
                let published =
                    inventory.published_original(path, &sha256(&bytes), bytes.len() as u64)?;
                let report = json(&bytes)?;
                let fields = crate::verified_lifecycle::ImportedPolicyReceipt {
                    schema: report["schema"].as_str(),
                    operation: report["operation"].as_str(),
                    status: report["status"].as_str(),
                    source_root: report["source_root"].as_str(),
                    report_path: report["report_path"].as_str(),
                    config_path: report["config_path"].as_str(),
                };
                if !crate::verified_lifecycle::imported_policy_receipt_valid(
                    &fields,
                    published,
                    root.to_str().ok_or("non-UTF8 coordinator root")?,
                    path,
                    &self.inputs.config_path,
                ) {
                    return Err("import policy original does not bind this adoption".into());
                }
                self.verify_inputs(&report)?;
                binding.head = report["policy_source_commit"]
                    .as_str()
                    .ok_or("import policy original lacks its fixed policy source commit")?
                    .into();
                facts::full_oid(&binding.head)?;
                let config_sha256 = report["config_sha256"]
                    .as_str()
                    .ok_or("import policy original lacks its configuration digest")?;
                binding.inputs = BTreeMap::from([
                    (self.inputs.config_path.clone(), Some(config_sha256.into())),
                    (
                        self.inputs.policy_path.clone(),
                        Some(binding.policy_sha256.clone()),
                    ),
                ]);
            }
        }
        let config_bytes = r.blob(&root, &binding.head, &self.inputs.config_path)?;
        let policy_bytes = r.blob(&root, &binding.head, &self.inputs.policy_path)?;
        let expected = BTreeMap::from([
            (self.inputs.config_path.clone(), Some(sha256(&config_bytes))),
            (self.inputs.policy_path.clone(), Some(sha256(&policy_bytes))),
        ]);
        if !equal_bytes(
            &serde_json::to_vec(&expected).map_err(|e| e.to_string())?,
            &serde_json::to_vec(&binding.inputs).map_err(|e| e.to_string())?,
        ) || !equal_text(&sha256(&policy_bytes), &binding.policy_sha256)
        {
            return Err("absent worktree policy does not match its retained adoption".into());
        }
        // These bound bytes supply addresses, never an executable configuration.
        #[derive(Deserialize)]
        struct AdoptedPolicyAddresses {
            host_config: String,
            automatic_cleanup: Option<String>,
        }
        let config: AdoptedPolicyAddresses = decode(&config_bytes)?;
        if !equal_bytes(
            &serde_json::to_vec(&config.automatic_cleanup).map_err(|e| e.to_string())?,
            &serde_json::to_vec(&Some(&self.inputs.policy_path)).map_err(|e| e.to_string())?,
        ) {
            return Err("absent worktree policy input address changed".into());
        }
        // Coordinator producers execute from anchor_policy, not from this
        // absent checkout's retained data. Keep its original bytes bound.
        let mut cleanup_policy = json(&policy_bytes)?;
        if let Some(object) = cleanup_policy.as_object_mut() {
            object.remove("retained_producers");
        }
        let policy = parse_policy(
            r,
            &root,
            &serde_json::to_vec(&cleanup_policy).map_err(|e| e.to_string())?,
        )?;
        if !policy_namespace_matches(
            &policy_namespace(&policy),
            &policy_namespace(&self.inputs.anchor_policy),
        ) {
            return Err("absent worktree ownership namespace changed".into());
        }
        let (registrations, registry_digest) =
            start::registrations(r, &root, &binding.head, &config.host_config)?;
        artifact_disposal::registered(
            &policy
                .artifacts
                .iter()
                .map(|a| a.path.clone())
                .collect::<Vec<_>>(),
            &[registrations.config()],
        )?;
        for path in expected.keys() {
            start::registered_policy(&registrations, path, &policy.state_directory)?;
        }
        self.inputs.policy = policy;
        self.inputs.policy_bytes = policy_bytes;
        self.inputs.config_bytes = config_bytes;
        self.inputs.source_root = root;
        self.inputs.source_head = binding.head;
        self.inputs.registrations = Arc::new(registrations);
        self.inputs.registry_digest = registry_digest;
        Ok(())
    }
    fn save(&self) -> Result<(), String> {
        if self.inputs.policy.schema == "chrono-worktree-automatic-cleanup/v2"
            && self._gate.lease.is_none()
        {
            return Err("enrollment update requires its current admission capability".into());
        }
        if self
            .ledger
            .entries
            .iter()
            .any(|entry| entry.path != self.scope)
        {
            return Err("enrollment update cannot write another worktree".into());
        }
        self._namespace.stable()?;
        self._gate.stable()?;
        let path = no_symlink_parents(&self.record_directory, "ledger.json")?;
        let inventory = chrono_harness::retained_artifacts::retention::Inventory::adopted(
            &self.inputs.policy.coordinator_root,
        )?;
        if let Some(inventory) = &inventory {
            let previous: Ledger = decode(&fs::read(&path).map_err(|e| e.to_string())?)?;
            let mut paths = self.original_paths(&previous)?;
            paths.extend(self.original_paths(&self.ledger)?);
            // Protect both possible durable records before publication. A
            // failed ledger write leaves the previous recovery inputs rooted.
            inventory.reference_paths(
                &self.original_role()?,
                &paths.into_iter().collect::<Vec<_>>(),
            )?;
        }
        write_json(&path, &self.ledger, false)?;
        if let Some(inventory) = inventory {
            inventory.reference_paths(
                &self.original_role()?,
                &self
                    .original_paths(&self.ledger)?
                    .into_iter()
                    .collect::<Vec<_>>(),
            )?;
        }
        Ok(())
    }
    fn stable(&self, r: &mut Runner) -> Result<(), String> {
        self._namespace.stable()?;
        self._gate.stable()?;
        self.inputs.stable(r)
    }
    // Only this worktree's metadata is excluded. Independent report owners
    // and worktrees use their own capabilities; no global release/reload join
    // is required around physical IO or an independent owner's failure.
    fn outside_enrollment<T>(
        &mut self,
        r: &mut Runner,
        activity: impl FnOnce(&PolicyInputs, &mut Runner) -> Result<T, String>,
    ) -> Result<T, String> {
        self.stable(r)?;
        activity(&self.inputs, r)
    }
    fn outside_enrollment_readonly<T>(
        &mut self,
        r: &mut Runner,
        activity: impl FnOnce(&Self, &mut Runner) -> Result<T, String>,
    ) -> Result<T, String> {
        self.stable(r)?;
        activity(self, r)
    }
    fn pinned_retention(&self, r: &mut Runner, t: &Terminal) -> Result<(), String> {
        if r.command(
            &self.inputs.policy.coordinator_root,
            &["symbolic-ref", "--quiet", &self.inputs.policy.retained_ref],
        )?
        .exit_code
            != 1
            || r.oid(
                &self.inputs.policy.coordinator_root,
                &self.inputs.policy.retained_ref,
            )? != t.retained_commit
        {
            return Err(
                "retained reference changed after terminal handoff; preserve checkout".into(),
            );
        }
        Ok(())
    }
    fn target_policy_inputs(
        &self,
        r: &mut Runner,
        target: &Path,
        head: &str,
    ) -> Result<BTreeMap<String, Option<String>>, String> {
        let tree = r.tree(target, head)?;
        let mut inputs = BTreeMap::new();
        for path in [&self.inputs.config_path, &self.inputs.policy_path] {
            let physical = no_symlink_parents(target, path)?;
            let digest = if tree.contains_key(path) {
                let committed = r.blob(target, head, path)?;
                if fs::read(&physical).map_err(|e| e.to_string())? != committed {
                    return Err(format!(
                        "enrolled target policy input differs from commit: {path}"
                    ));
                }
                Some(sha256(&committed))
            } else {
                match fs::symlink_metadata(&physical) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                    Err(e) => return Err(e.to_string()),
                    Ok(_) => {
                        return Err(format!(
                            "uncommitted target policy input must be preserved: {path}"
                        ));
                    }
                }
                None
            };
            inputs.insert(path.clone(), digest);
        }
        Ok(inputs)
    }
    fn check_entry(
        &self,
        r: &mut Runner,
        e: &Entry,
        head: &str,
        lock: Option<&str>,
    ) -> Result<(), String> {
        let binding = self.policy_binding(e)?;
        if binding.policy_sha256 != sha256(&self.inputs.policy_bytes) {
            return Err("enrolled policy changed; explicit migration is required".into());
        }
        self.check_attachment(r, e, head, lock)?;
        if self.target_policy_inputs(r, &e.path, head)? != binding.inputs {
            return Err("enrolled target policy/configuration changed; preserve work for explicit migration".into());
        }
        if e.cache_only {
            self.main_cache_owned(e)?;
            let owned: Vec<String> = serde_json::from_value(
                e.enrollment["main_cache_artifacts"].clone(),
            )
            .map_err(|_| "main cache artifact declarations missing or invalid; preserve outputs")?;
            let protected = e.enrollment["protected_historical_artifacts"]
                .as_array()
                .ok_or("main historical ownership declarations missing; preserve outputs")?;
            let mut declared: BTreeSet<_> = owned.iter().map(String::as_str).collect();
            for artifact in protected {
                if !declared.insert(
                    artifact["path"]
                        .as_str()
                        .ok_or("main historical artifact path missing")?,
                ) {
                    return Err("ambiguous main cache ownership; preserve outputs".into());
                }
            }
            let registered: BTreeSet<_> = self
                .inputs
                .policy
                .artifacts
                .iter()
                .filter(|a| select_artifact(&a.disposition, true, false))
                .map(|a| a.path.as_str())
                .collect();
            if owned.iter().collect::<BTreeSet<_>>().len() != owned.len()
                || (e.policy_migrations.is_empty() && declared != registered)
            {
                return Err(
                    "main cache ownership does not match its exact artifact registration".into(),
                );
            }
        }
        Ok(())
    }
    fn check_attachment(
        &self,
        r: &mut Runner,
        e: &Entry,
        head: &str,
        lock: Option<&str>,
    ) -> Result<(), String> {
        let kernel = self.inputs.policy.schema == "chrono-worktree-automatic-cleanup/v2";
        if kernel && e.ownership.is_none() {
            return Err(
                "kernel enrollment ownership is missing; preserve legacy/unknown work".into(),
            );
        }
        if let Some(owner) = &e.ownership {
            if !kernel || !owner.path.starts_with(&self.inputs.policy.state_directory) {
                return Err("enrollment ownership policy mismatch".into());
            }
            let path = no_symlink_parents(&self.inputs.policy.coordinator_root, &owner.path)?;
            if crate::ownership::identity(&path)? != owner.id {
                return Err("enrollment lease identity changed".into());
            }
        }
        absolute(&e.path, false)?;
        if e.cache_only {
            if !self.inputs.policy.main_cache_only || e.path != self.inputs.policy.coordinator_root
            {
                return Err("main cache ownership is not adopted at this coordinator".into());
            }
            let observed = r.checkout_identity(&e.path)?;
            if observed.top != e.path
                || observed.common != self.ledger.common
                || observed.head != head
                || main_attachment(&e.path, &observed.metadata)? != e.attachment
            {
                return Err("main cache attachment changed; preserve outputs".into());
            }
            self.git_locks(e, Some(&observed.branch))?;
            return self.stable(r);
        }
        let observed = maintenance::identity(
            r,
            &self.inputs.policy.coordinator_root,
            &e.path,
            e.current_branch(),
            head,
            lock,
        )?;
        if attachment_at(&e.path, &observed.metadata)? != e.attachment {
            return Err("enrolled checkout attachment changed".into());
        }
        self.git_locks(e, None)?;
        self.stable(r)
    }
    fn git_locks(&self, e: &Entry, main_branch: Option<&str>) -> Result<(), String> {
        let mut locks = vec![
            e.attachment.metadata.join("index.lock"),
            e.attachment.metadata.join("HEAD.lock"),
            self.ledger.common.join("packed-refs.lock"),
            self.ledger.common.join("config.lock"),
        ];
        let mut references = vec![self.inputs.policy.retained_ref.clone()];
        match main_branch {
            Some("HEAD") => (), // Detached main has no branch-removal authority.
            Some(reference) => references.push(reference.into()),
            None if e.current_branch() != "HEAD" => {
                references.push(maintenance::checkout_reference(e.current_branch()))
            }
            None => (),
        }
        for reference in references {
            relative_path(&reference)?;
            locks.push(self.ledger.common.join(format!("{reference}.lock")));
        }
        for lock in locks {
            match fs::symlink_metadata(&lock) {
                Err(x) if x.kind() == std::io::ErrorKind::NotFound => (),
                Err(x) => return Err(x.to_string()),
                Ok(_) => {
                    return Err(format!(
                        "Git lock remains owned or unknown: {}",
                        lock.display()
                    ));
                }
            }
        }
        Ok(())
    }
    fn current_entry(&self, target: &Path) -> Option<usize> {
        self.ledger
            .entries
            .iter()
            .position(|e| e.path == target && e.status != "disposed")
    }
    fn enrollment_available(&self, target: &Path) -> Result<(), String> {
        if self.current_entry(target).is_some() {
            return Err("path is already enrolled; preserve its original state".into());
        }
        Ok(())
    }
    fn preflight_birth(&self, r: &mut Runner, target: &Path) -> Result<(), String> {
        self.enrollment_available(target)?;
        match fs::symlink_metadata(target) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.to_string()),
            Ok(_) => return Err("birth destination must be absent before enrollment".into()),
        }
        if r.inventory(&self.inputs.policy.coordinator_root)?
            .iter()
            .any(|row| row.get("worktree").is_some_and(|p| Path::new(p) == target))
        {
            return Err("birth destination still has a Git attachment; preserve it".into());
        }
        Ok(())
    }
    fn enroll(
        &mut self,
        r: &mut Runner,
        target: &Path,
        kind: &str,
        receipt: Value,
    ) -> Result<(), String> {
        // Explicit import observes existing work under the invoking host's
        // current policy. Its original inputs are bound below as raw committed
        // bytes; adopting the target's policy is a separate migrate operation.
        if kind != "import" {
            self.adopt_live_policy(r)?;
        }
        let target = absolute(target, false)?;
        self.enrollment_available(&target)?;
        let cache_only = kind == "main-cache";
        let observed = r.checkout_identity(&target)?;
        let branch = if cache_only || (kind == "import" && observed.branch == "HEAD") {
            observed.branch
        } else {
            observed
                .branch
                .strip_prefix("refs/heads/")
                .ok_or("enrollment requires a local branch or explicit detached import")?
                .to_owned()
        };
        if cache_only
            && (!self.inputs.policy.main_cache_only
                || self._gate.lease.is_none()
                || target != self.inputs.policy.coordinator_root)
        {
            return Err("main cache ownership is not explicitly adopted".into());
        }
        if !cache_only
            && !crate::verified_lifecycle::accepts_enrollment_branch(
                kind,
                ["feature_prefix", "integration_prefix"].iter().any(|k| {
                    self.inputs.registrations.workflow()[k]
                        .as_str()
                        .is_some_and(|p| branch.starts_with(p))
                }),
            )
        {
            return Err("enrollment branch is not a registered work branch".into());
        }
        let head = r.oid(&target, "HEAD")?;
        let attached = if cache_only {
            let observed = r.checkout_identity(&target)?;
            main_attachment(&target, &observed.metadata)?
        } else {
            attachment(r, &target)?
        };
        let policy_inputs = self.target_policy_inputs(r, &target, &head)?;
        if kind == "birth"
            && (policy_inputs.get(&self.inputs.config_path)
                != Some(&Some(sha256(&self.inputs.config_bytes)))
                || policy_inputs.get(&self.inputs.policy_path)
                    != Some(&Some(sha256(&self.inputs.policy_bytes))))
        {
            return Err("new birth did not adopt the coordinator policy/configuration".into());
        }
        let mut birth_lease = None;
        let ownership = if self._gate.lease.is_some() {
            let token = receipt["report_path"]
                .as_str()
                .ok_or("enrollment receipt identity")?;
            let path = format!(
                "{}enrollment-{}.lease",
                self.inputs.policy.state_directory,
                sha256(token.as_bytes())
            );
            let physical = no_symlink_parents(&self.inputs.policy.coordinator_root, &path)?;
            if physical.exists() {
                return Err("enrollment lease identity already exists".into());
            }
            let lease = Lease::acquire(&physical, None, true, true, None)?
                .ok_or("new enrollment lease unavailable")?;
            let owner = Ownership {
                path,
                id: lease.id().into(),
                cache_pending: !cache_only,
                generation: 1,
            };
            if kind == "birth" {
                birth_lease = Some(lease);
            }
            Some(owner)
        } else {
            None
        };
        // Existing main outputs have no adopted release history. Enrollment can
        // own absent registered outputs before its managed producer starts; it
        // cannot silently migrate historical output or unknown consumers.
        let mut main_cache_artifacts = vec![];
        let mut historical_artifacts = vec![];
        if cache_only {
            for artifact in self
                .inputs
                .policy
                .artifacts
                .iter()
                .filter(|a| select_artifact(&a.disposition, true, false))
            {
                let path = no_symlink_parents(&target, artifact.path.trim_end_matches('/'))?;
                match fs::symlink_metadata(path) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => main_cache_artifacts.push(artifact.path.clone()),
                    Err(e) => return Err(e.to_string()),
                    Ok(_) => historical_artifacts.push(value!({"path":artifact.path,
                        "reason":"pre-existing output has no adopted consumer release; explicit migration required"})),
                }
            }
        }
        let mut enrollment = value!({"kind":kind,"observed_head":head,"receipt":receipt,"policy_inputs":policy_inputs});
        if cache_only {
            enrollment["main_cache_artifacts"] = value!(main_cache_artifacts);
            enrollment["protected_historical_artifacts"] = value!(historical_artifacts);
        }
        let entry = Entry {
            path: target,
            creator_agent: if kind == "birth" {
                serde_json::from_value(receipt["creator_agent"].clone())
                    .map_err(|error| format!("invalid birth creator identity: {error}"))?
            } else {
                None
            },
            custodian_agent: if kind == "import" {
                serde_json::from_value(receipt["custodian_agent"].clone())
                    .map_err(|error| format!("invalid import custodian identity: {error}"))?
            } else {
                None
            },
            branch,
            checkout_branch: None,
            attachment: attached,
            policy_sha256: sha256(&self.inputs.policy_bytes),
            status: "active".into(),
            enrollment,
            uses: vec![],
            terminal: None,
            attempts: vec![],
            ownership,
            cache_attempts: vec![],
            policy_migrations: vec![],
            main_cache_adoptions: vec![],
            cache_only,
            cleanup: None,
            disposed_at: None,
        };
        self.check_entry(r, &entry, &head, None)?;
        let host_config = decode::<crate::Config>(&self.inputs.config_bytes)?.host_config;
        let target_snapshot = if head == self.inputs.source_head
            && host_config == self.inputs.registrations.entry_path()
        {
            None
        } else {
            Some(start::registrations(r, &entry.path, &head, &host_config)?)
        };
        let target_registrations = target_snapshot
            .as_ref()
            .map_or(self.inputs.registrations.as_ref(), |(registrations, _)| {
                registrations
            });
        if kind != "import" {
            artifact_disposal::registered(
                &self
                    .inputs
                    .policy
                    .artifacts
                    .iter()
                    .map(|a| a.path.clone())
                    .collect::<Vec<_>>(),
                &[target_registrations.config()],
            )?;
        }
        for (path, digest) in &policy_inputs {
            if digest.is_some() {
                start::registered_policy(
                    &target_registrations,
                    path,
                    &self.inputs.policy.state_directory,
                )?;
            }
        }
        self.ledger.entries.push(entry);
        self.save()?;
        r.birth_lease = birth_lease;
        Ok(())
    }
    fn birth_binding(e: &Entry) -> Value {
        let mut binding = value!({"path":e.path,"branch":e.branch,"attachment":e.attachment,
            "policy_sha256":e.policy_sha256,"observed_head":e.enrollment["observed_head"],
            "policy_inputs":e.enrollment["policy_inputs"],"original_receipt":e.enrollment["receipt"],
            "ownership":e.ownership.as_ref().map(|o| value!({"path":o.path,"id":o.id}))});
        if let Some(agent) = &e.creator_agent {
            binding["creator_agent"] = value!(agent);
        }
        binding
    }
    fn birth_ready(&self, e: &Entry) -> Result<(), String> {
        if e.enrollment["kind"] == "birth" {
            if e.enrollment["sealed_receipt"].is_null()
                && !e.enrollment["recovered_receipt"].is_null()
            {
                let receipt: Receipt =
                    serde_json::from_value(e.enrollment["recovered_receipt"].clone())
                        .map_err(|_| "invalid recovered birth receipt")?;
                let recovered = json(&self.receipt(&receipt)?)?;
                if recovered["schema"] != "chrono-worktree-interrupted-birth/v2"
                    || recovered["binding"] != Self::birth_binding(e)
                    || recovered["original_outcome"] != "not-established"
                    || recovered["terminal_handoff"] != "not-claimed"
                {
                    return Err("recovered birth receipt does not establish this enrollment".into());
                }
                return Ok(());
            }
            let receipt: Receipt =
                serde_json::from_value(e.enrollment["sealed_receipt"].clone())
                    .map_err(|_| "birth has no completed original receipt; preserve it")?;
            let original = json(&self.receipt(&receipt)?)?;
            if !matches!(
                original["status"].as_str(),
                Some("created" | "reconstructed")
            ) || original["destination"] != value!(e.path)
                || original["branch_ref"] != e.branch
            {
                return Err("birth receipt does not establish this enrollment".into());
            }
        }
        Ok(())
    }
    fn reconcile_birth(&mut self, r: &mut Runner, i: usize, lease: &Lease) -> Result<(), String> {
        let e = self.ledger.entries[i].clone();
        if e.enrollment["kind"] != "birth"
            || !e.enrollment["sealed_receipt"].is_null()
            || !e.enrollment["recovered_receipt"].is_null()
        {
            return self.birth_ready(&e);
        }
        lease.stable()?;
        let head = r.oid(&e.path, "HEAD")?;
        self.check_entry(r, &e, &head, None)?;
        let original_head = e.enrollment["observed_head"]
            .as_str()
            .ok_or("birth original HEAD")?;
        let inputs: BTreeMap<String, Option<String>> =
            serde_json::from_value(e.enrollment["policy_inputs"].clone())
                .map_err(|_| "birth original policy inputs")?;
        if self.target_policy_inputs(r, &e.path, original_head)? != inputs {
            return Err("birth original policy bindings changed; preserve enrollment".into());
        }
        let report_path = e.enrollment["receipt"]["report_path"]
            .as_str()
            .ok_or("birth original report path")?;
        let name = format!("interrupted-birth-{}.json", sha256(report_path.as_bytes()));
        let physical = no_symlink_parents(
            &self.inputs.policy.coordinator_root,
            &format!("{}{name}", self.inputs.policy.state_directory),
        )?;
        let binding = Self::birth_binding(&e);
        // Reuse an already-published reconciliation on repeated interruption.
        // Its original missing/partial outcome is immutable, even if another
        // report becomes available later.
        let receipt = match fs::read(&physical) {
            Ok(bytes) => {
                let original = json(&bytes)?;
                if original["schema"] != "chrono-worktree-interrupted-birth/v2"
                    || original["binding"] != binding
                {
                    return Err("interrupted birth reconciliation binding changed".into());
                }
                self.verify_inputs(&original)?;
                Receipt {
                    path: format!("{}{name}", self.inputs.policy.state_directory),
                    sha256: sha256(&bytes),
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                relative_path(report_path)?;
                if !report_path.starts_with(&r.config.report_directory) {
                    return Err(
                        "birth original report is outside registered lifecycle evidence".into(),
                    );
                }
                let source = Path::new(
                    e.enrollment["receipt"]["source_root"]
                        .as_str()
                        .ok_or("birth original source root")?,
                );
                let original_path = no_symlink_parents(source, report_path)?;
                let original_result = match fs::read(&original_path) {
                    Ok(bytes) => {
                        value!({"presence":"present","sha256":sha256(&bytes),"input":self.retain_bytes(&bytes)?})
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        value!({"presence":"absent"})
                    }
                    Err(error) => return Err(error.to_string()),
                };
                let token = e.enrollment["receipt"]["token"].as_str();
                if let Some(token) = token {
                    relative_path(token)?;
                }
                let sealed_original = token
                    .map(|t| {
                        self.result_input(
                            &format!("{}birth-{t}.json", self.inputs.policy.state_directory),
                            None,
                        )
                    })
                    .transpose()?;
                self.immutable(&name, &value!({"schema":"chrono-worktree-interrupted-birth/v2","binding":binding,
                    "original_report":{"source_root":source,"path":report_path,"result":original_result},
                    "unreferenced_original_receipt":sealed_original,"original_outcome":"not-established",
                    "state":"registered-consumers-quiescent","terminal_handoff":"not-claimed","task_completion":"not-claimed"}))?
            }
            Err(error) => return Err(error.to_string()),
        };
        // Publication and ledger attachment can themselves be interrupted.
        self.check_entry(r, &e, &head, None)?;
        lease.stable()?;
        self.ledger.entries[i].enrollment["recovered_receipt"] = value!(receipt);
        self.save()
    }
    fn immutable(&self, name: &str, v: &Value) -> Result<Receipt, String> {
        relative_path(name)?;
        let path = format!("{}{name}", self.inputs.policy.state_directory);
        let mut b = serde_json::to_vec_pretty(v).map_err(|error| error.to_string())?;
        b.push(b'\n');
        self.publish_original(&path, &b, Some(v), false)?;
        Ok(Receipt {
            path,
            sha256: sha256(&b),
        })
    }
    fn publish_original(
        &self,
        path: &str,
        bytes: &[u8],
        receipt: Option<&Value>,
        reuse: bool,
    ) -> Result<(), String> {
        self._namespace.stable()?;
        if let Some(lease) = &self._gate.lease {
            lease.stable()?;
        }
        let root = &self.inputs.policy.coordinator_root;
        let target = no_symlink_parents(root, path)?;
        if reuse && target.exists() {
            if crate::recovery::state_bytes(root, path)? != bytes {
                return Err("retained lifecycle input changed".into());
            }
            return Ok(());
        }
        let inventory = chrono_harness::retained_artifacts::retention::Inventory::adopted(root)?;
        if let Some(inventory) = inventory.filter(|i| i.accepts("lifecycle-original", path)) {
            let dependencies = receipt
                .map(|v| crate::recovery::original_paths(root, v))
                .transpose()?
                .unwrap_or_default();
            let publication = inventory.plan(
                "lifecycle-original",
                path,
                false,
                value!({"kind":"automatic-original","completion":"not-established"}),
            )?;
            publication.materialize_for_reference(&self.original_role()?)?;
            publication.replace_file(&target, bytes)?;
            chrono_harness::retained_artifacts::retention::Inventory::adopted(root)?
                .ok_or("automatic original retention disappeared")?
                .dependencies_paths(path, &dependencies)?;
            publication.complete(value!({"kind":"automatic-original","publication":"complete"}))?;
        } else {
            write_bytes(&target, bytes, true)?;
        }
        Ok(())
    }
    fn immutable_reconciled(&self, name: &str, value: &Value) -> Result<Receipt, String> {
        let path = format!("{}{name}", self.inputs.policy.state_directory);
        let physical = no_symlink_parents(&self.inputs.policy.coordinator_root, &path)?;
        let _input = chrono_harness::retained_artifacts::retention::read_guard(
            &self.inputs.policy.coordinator_root,
            &path,
        )?;
        match fs::read(&physical) {
            Ok(bytes) => {
                if json(&bytes)? != *value {
                    return Err("interrupted use reconciliation receipt changed".into());
                }
                if let Some(inventory) =
                    chrono_harness::retained_artifacts::retention::Inventory::adopted(
                        &self.inputs.policy.coordinator_root,
                    )?
                {
                    inventory.dependencies_paths(
                        &path,
                        &crate::recovery::original_paths(
                            &self.inputs.policy.coordinator_root,
                            value,
                        )?,
                    )?;
                }
                Ok(Receipt {
                    path,
                    sha256: sha256(&bytes),
                })
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => self.immutable(name, value),
            Err(e) => Err(e.to_string()),
        }
    }
    fn receipt(&self, receipt: &Receipt) -> Result<Vec<u8>, String> {
        let b = crate::recovery::state_bytes(&self.inputs.policy.coordinator_root, &receipt.path)?;
        if sha256(&b) != receipt.sha256 {
            return Err("original lifecycle receipt changed".into());
        }
        self.verify_inputs(&json(&b)?)?;
        Ok(b)
    }
    fn retained_input(&self, path: &str, bytes: &[u8], format: &str) -> Value {
        crate::recovery::retained_input(&self.inputs.policy.coordinator_root, path, bytes, format)
    }
    /// Preserve partial/unsealed bytes independently of their mutable source.
    fn retain_bytes(&self, bytes: &[u8]) -> Result<Value, String> {
        let path = format!(
            "{}input-{}.bin",
            self.inputs.policy.state_directory,
            sha256(bytes)
        );
        self.publish_original(&path, bytes, None, true)?;
        Ok(self.retained_input(&path, bytes, "bytes"))
    }
    fn verify_inputs(&self, report: &Value) -> Result<(), String> {
        crate::recovery::verify_inputs(&self.inputs.policy.coordinator_root, report)
    }
    fn lease(&self, e: &Entry, exclusive: bool) -> Result<Option<Lease>, String> {
        let owner = e
            .ownership
            .as_ref()
            .ok_or("enrollment has no kernel ownership; preserve legacy state")?;
        if self._gate.lease.is_none()
            || !owner.path.starts_with(&self.inputs.policy.state_directory)
        {
            return Err("enrollment lease policy mismatch".into());
        }
        let path = no_symlink_parents(&self.inputs.policy.coordinator_root, &owner.path)?;
        Lease::acquire(&path, Some(&owner.id), false, exclusive, None)
    }
    fn reconcile_uses(&mut self, i: usize, lease: &Lease) -> Result<(), String> {
        lease.stable()?;
        let e = self.ledger.entries[i].clone();
        for token in &e.uses {
            relative_path(token)?;
            let intent_path = format!(
                "{}use-intent-{token}.json",
                self.inputs.policy.state_directory
            );
            let intent_bytes =
                crate::recovery::state_bytes(&self.inputs.policy.coordinator_root, &intent_path)?;
            let intent = json(&intent_bytes)?;
            if intent["path"] != value!(e.path)
                || intent["attachment"] != value!(e.attachment)
                || intent["lease_path"]
                    != value!(e.ownership.as_ref().ok_or("missing use ownership")?.path)
                || intent["lease_id"] != value!(e.ownership.as_ref().unwrap().id)
            {
                return Err("interrupted use intent binding changed; preserve record".into());
            }
            let path = format!("{}use-{token}.json", self.inputs.policy.state_directory);
            let original = self.result_input(&path, None)?;
            let receipt = self.immutable_reconciled(&format!("interrupted-use-{token}.json"), &value!({
                "schema":"chrono-worktree-interrupted-use/v2", "path":e.path, "token":token,
                "intent":{"path":intent_path,"sha256":sha256(&intent_bytes)}, "original_result":original,
                "state":"registered-consumers-quiescent", "command_outcome":"result-unavailable-unless-original-proves-it",
                "task_completion":"not-claimed", "terminal_handoff":"not-claimed"
            }))?;
            let history = self.ledger.entries[i]
                .enrollment
                .as_object_mut()
                .ok_or("enrollment object")?
                .entry("interrupted_uses")
                .or_insert(value!([]));
            history
                .as_array_mut()
                .ok_or("interrupted use history")?
                .push(value!(receipt));
            self.ledger.entries[i].uses.retain(|s| s != token);
            self.save()?;
        }
        Ok(())
    }
    fn result_input(&self, path: &str, receipt: Option<&Receipt>) -> Result<Value, String> {
        if let Some(receipt) = receipt {
            if receipt.path != path {
                return Err("original lifecycle result path changed".into());
            }
            let bytes = self.receipt(receipt)?;
            return Ok(
                value!({"presence":"present","sha256":sha256(&bytes),"input":self.retained_input(path, &bytes, "receipt"),"receipt":receipt}),
            );
        }
        let physical = no_symlink_parents(&self.inputs.policy.coordinator_root, path)?;
        match fs::read(physical) {
            Ok(bytes) => Ok(
                value!({"presence":"present","sha256":sha256(&bytes),"input":self.retain_bytes(&bytes)?,"original_outcome":"unknown"}),
            ),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Ok(value!({"presence":"absent","original_outcome":"unknown"}))
            }
            Err(e) => Err(e.to_string()),
        }
    }
    fn execute_cache(
        &mut self,
        r: &mut Runner,
        i: usize,
        token: &str,
        report: &mut Value,
    ) -> Result<(), String> {
        let e = self.ledger.entries[i].clone();
        let owner = e
            .ownership
            .as_ref()
            .ok_or("cache disposal requires kernel ownership")?;
        let head = r.oid(&e.path, "HEAD")?;
        self.check_entry(r, &e, &head, None)?;
        let release_reusable = crate::verified_lifecycle::reusable_release(
            e.terminal.is_some(),
            owner.generation,
            self.cache_release_generation(&e)?,
        );
        let owned = if e.cache_only {
            Some(self.main_cache_owned(&e)?)
        } else {
            None
        };
        let names: Vec<_> = self
            .inputs
            .policy
            .artifacts
            .iter()
            .filter(|a| {
                select_artifact(&a.disposition, release_reusable, false)
                    && owned.as_ref().is_none_or(|paths| paths.contains(&a.path))
            })
            .map(|a| a.path.clone())
            .collect();
        let host_config = decode::<crate::Config>(&self.inputs.config_bytes)?.host_config;
        // check_entry just verified the live attachment and common repository.
        // Equal immutable commits use the already observed coordinator registry;
        // different commits acquire their own snapshot. No live check is reused.
        let target_snapshot = if head == self.inputs.source_head
            && host_config == self.inputs.registrations.entry_path()
        {
            None
        } else {
            Some(start::registrations(r, &e.path, &head, &host_config)?)
        };
        let (target_regs, target_digest) = target_snapshot.as_ref().map_or(
            (
                self.inputs.registrations.as_ref(),
                &self.inputs.registry_digest,
            ),
            |(r, d)| (r, d),
        );
        artifact_disposal::registered(
            &names,
            &[self.inputs.registrations.config(), target_regs.config()],
        )?;
        let paths = artifact_disposal::paths(r, &e.path, &head, &names)?;
        let binding = value!({"path":e.path,"branch":e.current_branch(),"attachment":e.attachment,"lease_path":owner.path,"lease_id":owner.id,
            "head":head,"policy_sha256":self.policy_binding(&e)?.policy_sha256,"config_sha256":sha256(&self.inputs.config_bytes),
            "anchor_registry_digest":self.inputs.anchor_filemap_digest,
            "policy_source_root":self.inputs.source_root,"policy_source_commit":self.inputs.source_head,
            "policy_registry_digest":self.inputs.registry_digest,
            "target_registry_digest":target_digest,"artifacts":names,"generation":owner.generation,
            "release_reusable":release_reusable});
        if let Some(prior) = e
            .cache_attempts
            .last()
            .filter(|a| a.generation == owner.generation)
        {
            let bytes = self.receipt(&prior.intent)?;
            let intent = json(&bytes)?;
            if intent["binding"] != binding {
                return Err("interrupted cache disposal bindings changed; preserve original intent and work".into());
            }
            report["prior_cache_attempt"] = value!({"intent":prior.intent,"intent_input":self.retained_input(&prior.intent.path, &bytes, "receipt"),"result":self.result_input(&prior.path, prior.receipt.as_ref())?});
        }
        let intent = self.immutable(&format!("cache-intent-{token}.json"), &value!({"schema":"chrono-worktree-cache-intent/v2","binding":binding,"report_path":report["report_path"],"terminal_handoff":"not-claimed"}))?;
        self.ledger.entries[i].cache_attempts.push(CacheAttempt {
            path: report["report_path"]
                .as_str()
                .ok_or("cache report path")?
                .into(),
            intent: intent.clone(),
            generation: owner.generation,
            receipt: None,
            completed_at: None,
        });
        self.save()?;
        report["cache_intent"] = value!(intent);
        report["cache_generation"] = value!(owner.generation);
        report["destination"] = value!(e.path);
        report["head"] = value!(head);
        report["worktree_removal"] = value!("preserved");
        report["branch_removal"] = value!("not-requested");
        report["protected_historical_artifacts"] = if e.cache_only {
            self.protected_main_history(&e)?
        } else {
            Value::Null
        };
        report["preserved_reason"] =
            value!("unfinished resumable enrollment; only registered disposable outputs selected");
        report["artifact_disposals"] = value!([]);
        self.outside_enrollment_readonly(r, |manager, r| {
            artifact_disposal::dispose(&e.path, &names, &paths, report, || {
                manager.check_entry(r, &e, &head, None)?;
                artifact_disposal::paths(r, &e.path, &head, &names)?;
                Ok(())
            })?;
            manager.check_entry(r, &e, &head, None)
        })
    }
    fn finish(
        &mut self,
        r: &mut Runner,
        target: &Path,
        dispose_evidence: bool,
        artifacts_only: bool,
        pin: Option<&str>,
        token: &str,
        report: &mut Value,
    ) -> Result<(), String> {
        let target = absolute(target, false)?;
        let i = self
            .current_entry(&target)
            .ok_or("worktree is not enrolled; use exact import for existing work")?;
        let _lease = if self.ledger.entries[i].ownership.is_some() {
            let lease = self
                .lease(&self.ledger.entries[i], true)?
                .ok_or("worktree has live registered consumers; preserve it")?;
            self.reconcile_uses(i, &lease)?;
            self.reconcile_birth(r, i, &lease)?;
            Some(lease)
        } else {
            None
        };
        let e = self.ledger.entries[i].clone();
        if e.cache_only {
            if dispose_evidence || pin.is_some() {
                return Err(
                    "main finish only releases caches; evidence/landing disposal is not authorized"
                        .into(),
                );
            }
            let head = r.oid(&target, "HEAD")?;
            self.check_entry(r, &e, &head, None)?;
            let generation = e
                .ownership
                .as_ref()
                .ok_or("main cache ownership missing")?
                .generation
                .checked_add(1)
                .ok_or("main cache release generation overflow")?;
            let receipt = self.immutable(
                &format!("main-release-{token}.json"),
                &value!({
                    "schema":"chrono-main-cache-release/v1","path":target,"head":head,"report_path":report["report_path"],
                    "attachment":e.attachment,"ownership":e.ownership,"cache_generation":generation,
                    "worktree_removal":"not-authorized","task_completion":"not-claimed"
                }),
            )?;
            report["main_cache_release"] = value!(receipt);
            self.ledger.entries[i]
                .enrollment
                .as_object_mut()
                .ok_or("enrollment object")?
                .entry("main_cache_releases")
                .or_insert(value!([]))
                .as_array_mut()
                .ok_or("main cache release history")?
                .push(value!(receipt));
            self.ledger.entries[i].enrollment["cache_release"] = value!(receipt);
            let owner = self.ledger.entries[i].ownership.as_mut().unwrap();
            owner.generation = generation;
            owner.cache_pending = true;
            self.save()?;
            crate::recovery::bind_report_inputs(&self.inputs.policy.coordinator_root, report)?;
            report["worktree_removal"] = value!("preserved");
            return Ok(());
        }
        if !matches!(e.status.as_str(), "active" | "retained") || !e.uses.is_empty() {
            return Err(
                "worktree is terminal or has active/unknown managed use; preserve it".into(),
            );
        }
        if dispose_evidence && !self.inputs.policy.allow_evidence_disposal {
            return Err("policy does not authorize terminal evidence disposal".into());
        }
        self.birth_ready(&e)?;
        let head = r.oid(&target, "HEAD")?;
        self.check_entry(r, &e, &head, None)?;
        let retained_commit = r.oid(
            &self.inputs.policy.coordinator_root,
            &self.inputs.policy.retained_ref,
        )?;
        if let Some(pin) = pin {
            facts::full_oid(pin)?;
            if pin != retained_commit {
                return Err("verified landing pin differs from retained reference".into());
            }
        }
        let receipt=self.immutable(&format!("terminal-{token}.json"),&value!({
            "schema":"chrono-worktree-terminal/v1","path":target,"branch":e.current_branch(),"head":head,"attachment":e.attachment,
            "policy_sha256":self.policy_binding(&e)?.policy_sha256,"retained_ref":self.inputs.policy.retained_ref,"retained_commit":retained_commit,
            "dispose_evidence":dispose_evidence,"artifacts_only":artifacts_only,
            "handoff":"caller-joined-owned-jobs-and-established-completion","report_path":report["report_path"]
        }))?;
        self.stable(r)?;
        if let Some(old) = &e.terminal {
            let history = self.ledger.entries[i]
                .enrollment
                .as_object_mut()
                .ok_or("enrollment object")?
                .entry("terminal_history")
                .or_insert(value!([]));
            history
                .as_array_mut()
                .ok_or("terminal history")?
                .push(value!(old.receipt));
        }
        self.ledger.entries[i].terminal = Some(Terminal {
            head,
            retained_commit,
            dispose_evidence,
            artifacts_only,
            receipt: receipt.clone(),
        });
        self.ledger.entries[i].status = "terminal".into();
        self.progress(
            i,
            crate::verified_lifecycle::CleanupStage::Pending,
            "terminal handoff recorded; cleanup not completed",
            report,
        )?;
        self.save()?;
        report["terminal_receipt"] = value!(receipt);
        Ok(())
    }
    fn drain(
        &mut self,
        r: &mut Runner,
        exclude: &[PathBuf],
        report: &mut Value,
        all: bool,
    ) -> Result<(), String> {
        report["drain"] = value!([]);
        report["cleanup_failures"] = value!([]);
        report["producer_drains"] = value!([]);
        report["output_retention"] = Value::Null;
        report["maintenance_scope"] = if all {
            value!({"kind":"declared-enrollments"})
        } else {
            value!({"kind":"worktree","path":self.scope})
        };
        let mut failures = if all {
            self.resume_retired_addresses(r, report)?
        } else {
            vec![]
        };
        self.observe_inventory(r, exclude, report, all, &BTreeSet::new())?;
        failures.extend(self.drain_entries(r, exclude, report, all)?);
        if !all {
            report["cleanup_failures"] = value!(failures);
            return if failures.is_empty() {
                Ok(())
            } else {
                Err(format!(
                    "automatic cleanup pending failures: {}",
                    failures.join("; ")
                ))
            };
        }
        let (independent, unprotected) = self.outside_enrollment_readonly(r, |manager, r| {
            let mut failures = Vec::new();
            let mut unprotected = Vec::new();
            for path in manager.enrollment_paths()? {
                if path == manager.scope {
                    continue;
                }
                let opened = Self::open_metadata(
                    r,
                    &manager.inputs.policy.coordinator_root,
                    &manager.inputs.config_path,
                    &manager.inputs.anchor_config_bytes,
                    &manager._gate.token,
                    &path,
                    false,
                    false,
                );
                match opened {
                    Ok(Some(mut owner)) => {
                        if let Err(error) = owner.restore_absent_policy(r) {
                            failures.push(format!("{}: {error}", path.display()));
                            report["drain"]
                                .as_array_mut()
                                .unwrap()
                                .push(value!({"path":path,"status":"failed","error":error}));
                            continue;
                        }
                        match owner.drain_entries(r, exclude, report, true) {
                            Ok(pending) => failures.extend(pending),
                            Err(error) => {
                                failures.push(format!("{}: {error}", path.display()));
                                continue;
                            }
                        }
                        if let Err(error) = owner.retire_empty_address(r, report) {
                            failures.push(format!("{}: {error}", path.display()));
                        }
                    }
                    Ok(None) => return Err("cleanup policy disappeared during maintenance".into()),
                    Err(error)
                        if error.contains("enrollment metadata is busy")
                            || error.contains("enrollment metadata remains busy") =>
                    {
                        unprotected.push(path.clone());
                        report["drain"].as_array_mut().unwrap().push(
                            value!({"path":path,"status":"preserved","preserved_reason":error}),
                        );
                    }
                    Err(error) => {
                        unprotected.push(path.clone());
                        failures.push(format!("{}: {error}", path.display()));
                        report["drain"]
                            .as_array_mut()
                            .unwrap()
                            .push(value!({"path":path,"status":"failed","error":error}));
                    }
                }
            }
            Ok((failures, unprotected))
        })?;
        failures.extend(independent);
        let producer_failures = self.outside_enrollment(r, |inputs, _| {
        let mut failures = vec![];
        for producer in inputs.anchor_policy.retained_producers.clone() {
            let (config, bytes) = crate::configuration(
                &inputs.policy.coordinator_root,
                &inputs.config_path,
            )?;
            let root = &inputs.policy.coordinator_root;
            let result = start::with_report(
                root,
                &inputs.config_path,
                config,
                &bytes,
                "producer-cleanup",
                "cleaned",
                |r, _, attempt| {
                    attempt["producer_objects"] = value!([]);
                    producer.drain(inputs, r, attempt)
                },
            )?;
            if result["status"] != "cleaned" {
                failures.push(format!("producer {}: {}", producer.id, result["error"]));
            }
            let path = result["report_path"]
                .as_str()
                .ok_or("producer drain report path")?;
            let bytes = crate::recovery::state_bytes(root, path)?;
            report["producer_drains"].as_array_mut().unwrap().push(value!({"producer":producer.id,
                "status":result["status"],"error":result["error"],"receipt":{"path":path,"sha256":sha256(&bytes)},
                "input":crate::recovery::retained_input(root, path, &bytes, "receipt")}));
        }
        Ok(failures)
        })?;
        failures.extend(producer_failures);
        // All declared ledgers must establish their original consumers before
        // this cross-object pass. Unknown metadata defers only that dependent
        // pass; independent worktree and producer maintenance has already run.
        report["output_retention"] = if unprotected.is_empty() {
            value!(self.outside_enrollment(r, |inputs, _| {
                chrono_harness::retained_artifacts::retention::maintain_adopted(
                    &inputs.policy.coordinator_root,
                )
            })?)
        } else {
            value!({"status":"preserved","reason":"enrollment original protection unavailable",
                "unprotected_enrollments":unprotected})
        };
        report["cleanup_failures"] = value!(failures);
        if failures.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "automatic cleanup pending failures: {}",
                failures.join("; ")
            ))
        }
    }
    fn drain_entries(
        &mut self,
        r: &mut Runner,
        exclude: &[PathBuf],
        report: &mut Value,
        maintenance: bool,
    ) -> Result<Vec<String>, String> {
        // Selection and execution must use this checkout's committed choices;
        // the coordinator's thresholds can differ or be absent.
        self.adopt_live_policy(r)?;
        self.reconcile_published_results()?;
        let mut failures = self.rotate_history(r, report)?;
        failures.extend(self.rotate_cache_history(r, report)?);
        failures.extend(self.retire_leases(r, report)?);
        for i in 0..self.ledger.entries.len() {
            let e = self.ledger.entries[i].clone();
            let cache =
                e.status == "active" && e.ownership.as_ref().is_some_and(|o| o.cache_pending);
            let age = if matches!(e.status.as_str(), "active" | "retained" | "terminal")
                && e.enrollment["age_retirement"].is_null()
            {
                match self.age_eligible(&e, false) {
                    Ok(eligible) => eligible,
                    Err(error) => {
                        report["drain"].as_array_mut().unwrap().push(value!({"path":e.path,
                            "status":"preserved","age_eligibility":"unknown","preserved_reason":error}));
                        false
                    }
                }
            } else {
                false
            };
            if !crate::verified_lifecycle::select_cleanup(
                &e.status,
                e.ownership.as_ref().is_some_and(|o| o.cache_pending),
                maintenance,
                e.terminal.as_ref().is_some_and(|t| t.artifacts_only),
            ) && !age
            {
                continue;
            }
            if self._gate.lease.is_some() && e.ownership.is_none() {
                report["drain"].as_array_mut().unwrap().push(value!({"path":e.path,"status":"preserved","preserved_reason":"legacy/unknown enrollment has no kernel ownership"}));
                continue;
            }
            if exclude.contains(&e.path) && (!cache || !e.cache_only) {
                report["drain"].as_array_mut().unwrap().push(value!({"path":e.path,"status":"preserved","preserved_reason":"invoking source or destination"}));
                continue;
            }
            let _lease = if e.ownership.is_some() {
                match self.lease(&e, true) {
                    Ok(Some(lease)) => Some(lease),
                    Ok(None) => {
                        self.progress(i, crate::verified_lifecycle::CleanupStage::WaitingForConsumers,
                            "live registered consumers hold enrollment lease; retry after their release", report)?;
                        report["drain"].as_array_mut().unwrap().push(value!({"path":e.path,"status":"preserved","preserved_reason":"live registered consumers hold enrollment lease"}));
                        continue;
                    }
                    Err(error) => {
                        self.progress(
                            i,
                            crate::verified_lifecycle::CleanupStage::Failed,
                            &error,
                            report,
                        )?;
                        failures.push(format!("{}: {error}", e.path.display()));
                        report["drain"]
                            .as_array_mut()
                            .unwrap()
                            .push(value!({"path":e.path,"status":"failed","error":error}));
                        continue;
                    }
                }
            } else {
                None
            };
            if let Some(lease) = &_lease {
                if let Err(error) = self.reconcile_uses(i, lease) {
                    self.progress(
                        i,
                        crate::verified_lifecycle::CleanupStage::Failed,
                        &error,
                        report,
                    )?;
                    failures.push(format!("{}: {error}", e.path.display()));
                    report["drain"]
                        .as_array_mut()
                        .unwrap()
                        .push(value!({"path":e.path,"status":"failed","error":error}));
                    continue;
                }
            }
            if !self.ledger.entries[i].uses.is_empty() {
                self.progress(i, crate::verified_lifecycle::CleanupStage::WaitingForConsumers,
                    "active/unknown managed use; establish original result or reconcile under exclusive lease", report)?;
                failures.push(format!("{}: active/unknown managed use", e.path.display()));
                report["drain"].as_array_mut().unwrap().push(
                    value!({"path":e.path,"status":"failed","error":"active/unknown managed use"}),
                );
                continue;
            }
            let (config, bytes) = crate::configuration(
                &self.inputs.policy.coordinator_root,
                &self.inputs.config_path,
            )?;
            let root = self.inputs.policy.coordinator_root.clone();
            let config_path = self.inputs.config_path.clone();
            self.progress(
                i,
                crate::verified_lifecycle::CleanupStage::InProgress,
                "cleanup started; retain original attempt across interruption",
                report,
            )?;
            let outcome = start::with_report(
                &root,
                &config_path,
                config,
                &bytes,
                "cleanup",
                "cleaned",
                |r, token, attempt| {
                    self.adopt_live_policy(r)?;
                    attempt["source_commit"] = value!(self.inputs.anchor_head);
                    attempt["policy_source_root"] = value!(self.inputs.source_root);
                    attempt["policy_source_commit"] = value!(self.inputs.source_head);
                    attempt["registry_digest"] = value!(chrono_harness::wire::digest(&value!(
                        self.inputs.registrations.filemap()
                    ))?);
                    attempt["automatic_cleanup"] = value!(true);
                    if let Some(lease) = &_lease {
                        self.reconcile_birth(r, i, lease)?;
                    }
                    if age {
                        self.execute_age(r, i, token, attempt)
                    } else if cache {
                        self.execute_cache(r, i, token, attempt)
                    } else {
                        self.execute_entry(r, i, token, attempt)
                    }
                },
            );
            // Reattach the exact physical enrollment. Other worktrees have
            // separate records and cannot be overwritten by this result.
            let i = self
                .ledger
                .entries
                .iter()
                .position(|current| {
                    current.path == e.path
                        && current.attachment == e.attachment
                        && current.enrollment["receipt"] == e.enrollment["receipt"]
                })
                .ok_or("cleanup enrollment changed while its physical effect ran")?;
            match outcome {
                Ok(result) => {
                    let path = result["report_path"]
                        .as_str()
                        .ok_or("cleanup report path")?
                        .to_owned();
                    let b = crate::recovery::state_bytes(&root, &path)?;
                    let cache_outcome = result["age_outcome"] == "reusable-source-preserved"
                        || (cache
                            && result["age_outcome"]
                                != "expired-checkout-verified-absent-source-branches-retained");
                    let stage = crate::verified_lifecycle::cleanup_result(
                        cache_outcome,
                        result["status"].as_str().unwrap_or(""),
                        result["worktree_removal"].as_str().unwrap_or(""),
                    );
                    if cache_outcome {
                        if let Some(a) = self.ledger.entries[i]
                            .cache_attempts
                            .last_mut()
                            .filter(|a| a.path == path)
                        {
                            a.receipt = Some(Receipt {
                                path: path.clone(),
                                sha256: sha256(&b),
                            });
                            if result["status"] == "cleaned" {
                                a.completed_at = Some(
                                    std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .map_err(|error| error.to_string())?
                                        .as_secs(),
                                );
                                self.ledger.entries[i]
                                    .ownership
                                    .as_mut()
                                    .unwrap()
                                    .cache_pending = false;
                                self.ledger.entries[i]
                                    .enrollment
                                    .as_object_mut()
                                    .ok_or("enrollment object")?
                                    .remove("cache_release");
                            }
                        }
                    }
                    if let Some(a) = self.ledger.entries[i].attempts.last_mut() {
                        if a.path == path {
                            a.receipt = Some(Receipt {
                                path: path.clone(),
                                sha256: sha256(&b),
                            });
                        }
                    }
                    match &stage {
                        crate::verified_lifecycle::CleanupStage::Disposed => {
                            self.ledger.entries[i].status = "disposed".into();
                            if let Some(owner) = &mut self.ledger.entries[i].ownership {
                                owner.cache_pending = false;
                            }
                            self.ledger.entries[i].disposed_at = Some(
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map_err(|error| error.to_string())?
                                    .as_secs(),
                            );
                        }
                        crate::verified_lifecycle::CleanupStage::Retained => {
                            self.ledger.entries[i].status = "retained".into()
                        }
                        _ => {}
                    }
                    if result["status"] != "cleaned" {
                        failures.push(format!("{}: {}", e.path.display(), result["error"]));
                    }
                    self.progress(
                        i,
                        stage,
                        result["error"]
                            .as_str()
                            .or_else(|| result["preserved_reason"].as_str())
                            .unwrap_or("original cleanup result recorded"),
                        &result,
                    )?;
                    report["drain"].as_array_mut().unwrap().push(value!({
                        "path":e.path,"status":result["status"],"error":result["error"],
                        "receipt":{"path":path,"sha256":sha256(&b)},
                        "input":self.retained_input(&path, &b, "receipt")
                    }));
                }
                Err(error) => {
                    self.progress(
                        i,
                        crate::verified_lifecycle::CleanupStage::Failed,
                        &error,
                        report,
                    )?;
                    failures.push(format!("{}: {error}", e.path.display()));
                    report["drain"]
                        .as_array_mut()
                        .unwrap()
                        .push(value!({"path":e.path,"status":"failed","error":error}));
                }
            }
        }
        Ok(failures)
    }
    fn rotate_cache_history(
        &mut self,
        r: &mut Runner,
        report: &mut Value,
    ) -> Result<Vec<String>, String> {
        let Some(history) = self.inputs.policy.cache_history.clone() else {
            return Ok(vec![]);
        };
        let observed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_secs();
        let mut failures = vec![];
        for i in 0..self.ledger.entries.len() {
            let entry = self.ledger.entries[i].clone();
            let Some(owner) = entry
                .ownership
                .as_ref()
                .filter(|_| entry.status == "active")
            else {
                continue;
            };
            if !entry.uses.is_empty() || entry.cache_attempts.is_empty() {
                continue;
            }
            let lease = match self.lease(&entry, true) {
                Ok(Some(lease)) => lease,
                Ok(None) => continue,
                Err(error) => {
                    failures.push(format!("{}: {error}", entry.path.display()));
                    continue;
                }
            };
            let mut count = entry.cache_attempts.len();
            let sizes = entry
                .cache_attempts
                .iter()
                .map(|attempt| serde_json::to_vec(attempt).map(|bytes| bytes.len() as u64))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?;
            let mut bytes = sizes
                .iter()
                .try_fold(0u64, |sum, size| sum.checked_add(*size))
                .ok_or("cache history byte count overflow")?;
            let mut removed = BTreeSet::new();
            for (index, attempt) in entry.cache_attempts.iter().enumerate() {
                if !crate::verified_lifecycle::retire_cache_history(
                    attempt.receipt.is_some(),
                    attempt.generation,
                    owner.generation,
                    true,
                    attempt.completed_at,
                    observed,
                    history.keep_seconds,
                    count > history.max_entries || bytes > history.max_bytes,
                ) {
                    continue;
                }
                let verified = (|| {
                    let receipt = attempt
                        .receipt
                        .as_ref()
                        .ok_or("cache history result missing")?;
                    let result = json(&self.receipt(receipt)?)?;
                    let intent = json(&self.receipt(&attempt.intent)?)?;
                    if receipt.path != attempt.path
                        || result["status"] != "cleaned"
                        || result["destination"] != value!(entry.path)
                        || result["cache_generation"] != attempt.generation
                        || result["cache_intent"] != value!(attempt.intent)
                        || intent["binding"]["path"] != value!(entry.path)
                        || intent["binding"]["attachment"] != value!(entry.attachment)
                        || intent["binding"]["generation"] != attempt.generation
                        || intent["binding"]["lease_id"] != owner.id
                    {
                        return Err(
                            "cache history original does not establish completed generation".into(),
                        );
                    }
                    self.stable(r)?;
                    lease.stable()
                })();
                match verified {
                    Ok(()) => {
                        removed.insert(index);
                        count -= 1;
                        bytes -= sizes[index];
                    }
                    Err(error) => failures.push(format!("{}: {error}", entry.path.display())),
                }
            }
            if !removed.is_empty() {
                let mut index = 0;
                self.ledger.entries[i].cache_attempts.retain(|_| {
                    let keep = !removed.contains(&index);
                    index += 1;
                    keep
                });
                // save publishes the replacement ledger before releasing its
                // old original references. Physical disposal stays with retention.
                self.save()?;
            }
            if report["cache_history_rotation"].is_null() {
                report["cache_history_rotation"] = value!([]);
            }
            report["cache_history_rotation"]
                .as_array_mut()
                .ok_or("cache history rotation report is not an array")?
                .push(value!({"path":entry.path,"removed_records":removed.len(),
                    "remaining_records":count,"remaining_bytes":bytes,
                    "over_limit":count > history.max_entries || bytes > history.max_bytes,
                    "original_files":"retention-owner","task_completion":"not-claimed"}));
        }
        Ok(failures)
    }
    fn rotate_history(
        &mut self,
        r: &mut Runner,
        report: &mut Value,
    ) -> Result<Vec<String>, String> {
        let Some(history) = self.inputs.policy.terminal_history.clone() else {
            return Ok(vec![]);
        };
        let observed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_secs();
        let mut count = 0usize;
        let mut bytes = 0u64;
        let mut sizes = Vec::with_capacity(self.ledger.entries.len());
        for entry in &self.ledger.entries {
            let size = serde_json::to_vec(entry)
                .map_err(|error| error.to_string())?
                .len() as u64;
            if entry.status == "disposed" {
                count = count
                    .checked_add(1)
                    .ok_or("terminal history count overflow")?;
                bytes = bytes
                    .checked_add(size)
                    .ok_or("terminal history byte count overflow")?;
            }
            sizes.push(size);
        }
        let mut removed = BTreeSet::new();
        let mut leases = Vec::new();
        let mut failures = vec![];
        for (i, entry) in self.ledger.entries.iter().enumerate() {
            let over_limit = count > history.max_entries || bytes > history.max_bytes;
            if !crate::verified_lifecycle::retire_history(
                entry.status == "disposed",
                entry.uses.is_empty(),
                entry.disposed_at,
                observed,
                history.keep_seconds,
                over_limit,
            ) {
                continue;
            }
            let verified = (|| {
                let terminal = entry
                    .terminal
                    .as_ref()
                    .ok_or("disposed record lacks its terminal handoff")?;
                if !entry.uses.is_empty()
                    || entry.report_handoff_pending()
                    || entry.cache_only
                    || terminal.artifacts_only
                    || entry
                        .ownership
                        .as_ref()
                        .is_none_or(|owner| owner.cache_pending)
                {
                    return Err("disposed history still has unresolved responsibilities".into());
                }
                let lease = self
                    .lease(entry, true)?
                    .ok_or("terminal history has live consumers")?;
                let receipt = entry
                    .attempts
                    .last()
                    .and_then(|attempt| attempt.receipt.as_ref())
                    .ok_or("disposed history lacks its original cleanup result")?;
                let original = json(&self.receipt(receipt)?)?;
                let handoff = json(&self.receipt(&terminal.receipt)?)?;
                if original["terminal_receipt"] != value!(terminal.receipt)
                    || original["destination"] != value!(entry.path)
                    || original["head"] != terminal.head
                    || handoff["path"] != value!(entry.path)
                    || handoff["attachment"] != value!(entry.attachment)
                    || handoff["head"] != terminal.head
                    || entry.cleanup.as_ref().is_none_or(|progress| {
                        progress.stage != crate::verified_lifecycle::CleanupStage::Disposed
                            || progress.report_path.as_deref() != Some(receipt.path.as_str())
                    })
                    || crate::verified_lifecycle::cleanup_result(
                        false,
                        original["status"].as_str().unwrap_or(""),
                        original["worktree_removal"].as_str().unwrap_or(""),
                    ) != crate::verified_lifecycle::CleanupStage::Disposed
                {
                    return Err(
                        "disposed history original does not establish this attachment's removal"
                            .into(),
                    );
                }
                // A present/reused address remains protected. This method releases
                // history only; it never removes a checkout or Git registration.
                match fs::symlink_metadata(&entry.path) {
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.to_string()),
                    Ok(_) => {
                        return Err(
                            "terminal history address is present; preserve its record".into()
                        );
                    }
                }
                if r.inventory(&self.inputs.policy.coordinator_root)?
                    .iter()
                    .any(|row| {
                        row.get("worktree")
                            .is_some_and(|path| Path::new(path) == entry.path)
                    })
                {
                    return Err("terminal history address remains Git-registered".into());
                }
                self.stable(r)?;
                lease.stable()?;
                Ok::<_, String>(lease)
            })();
            match verified {
                Ok(lease) => {
                    removed.insert(i);
                    self.ledger.retired_leases.push(retirement::RetiredLease {
                        ownership: entry.ownership.clone().unwrap(),
                        creator_agent: entry.creator_agent.clone(),
                        branch: entry.current_branch().into(),
                        error: None,
                    });
                    count -= 1;
                    bytes -= sizes[i];
                    // Keep exclusion through publication of the replacement ledger.
                    leases.push(lease);
                }
                Err(error) => failures.push(format!("{}: {error}", entry.path.display())),
            }
        }
        if !removed.is_empty() {
            let mut i = 0usize;
            self.ledger.entries.retain(|_| {
                let keep = !removed.contains(&i);
                i += 1;
                keep
            });
            self.save()?;
        }
        if report["history_rotation"].is_null() {
            report["history_rotation"] = value!([]);
        }
        report["history_rotation"]
            .as_array_mut()
            .ok_or("history rotation report is not an array")?
            .push(value!({"path":self.scope,"removed_records":removed.len(),
                "remaining_disposed_records":count,"remaining_disposed_bytes":bytes,
                "over_limit":count > history.max_entries || bytes > history.max_bytes,
                "original_files":"not-disposed","checkout_disposal":"not-attempted"}));
        Ok(failures)
    }
    fn progress(
        &mut self,
        i: usize,
        stage: crate::verified_lifecycle::CleanupStage,
        detail: &str,
        report: &Value,
    ) -> Result<(), String> {
        self.ledger.entries[i].cleanup = Some(CleanupProgress {
            stage,
            detail: detail.into(),
            report_path: report["report_path"].as_str().map(str::to_owned),
        });
        self.save()
    }
    /// Entry-local refusals remain in the drain report and immutable receipts.
    /// Coordinator/state publication failures still prevent admission.
    fn drain_for_admission(
        &mut self,
        r: &mut Runner,
        exclude: &[PathBuf],
        report: &mut Value,
    ) -> Result<(), String> {
        match self.drain(r, exclude, report, false) {
            Err(_)
                if report["cleanup_failures"]
                    .as_array()
                    .is_some_and(|f| !f.is_empty()) =>
            {
                Ok(())
            }
            result => result,
        }
    }
    fn execute_entry(
        &mut self,
        r: &mut Runner,
        i: usize,
        token: &str,
        report: &mut Value,
    ) -> Result<(), String> {
        if !self.ledger.entries[i].enrollment["age_retirement"].is_null() {
            return self.execute_age_retirement(r, i, token, report);
        }
        if self.ledger.entries[i].cache_only
            || self.ledger.entries[i].path == self.inputs.policy.coordinator_root
        {
            return Err("main checkout has no worktree deletion authority".into());
        }
        let e = self.ledger.entries[i].clone();
        let t = e
            .terminal
            .as_ref()
            .ok_or("missing terminal handoff")?
            .clone();
        report["terminal_receipt"] = value!(t.receipt);
        report["terminal_input"] =
            self.retained_input(&t.receipt.path, &self.receipt(&t.receipt)?, "receipt");
        if self.policy_binding(&e)?.policy_sha256 != sha256(&self.inputs.policy_bytes) {
            return Err("enrolled policy changed; preserve terminal work".into());
        }
        self.stable(r)?;
        self.pinned_retention(r, &t)?;
        if let Some(prior) = e.attempts.last() {
            let receipt = prior
                .receipt
                .as_ref()
                .ok_or("interrupted cleanup has no original result; explicit recovery required")?;
            let b = self.receipt(receipt)?;
            let prior_report = json(&b)?;
            report["prior_report"] = value!({"receipt":receipt,"input":self.retained_input(&receipt.path, &b, "receipt")});
            if e.path.exists() {
                let locked = r
                    .inventory(&self.inputs.policy.coordinator_root)?
                    .into_iter()
                    .find(|row| row.get("worktree").is_some_and(|p| Path::new(p) == e.path))
                    .and_then(|row| row.get("locked").cloned());
                if locked.as_deref() == Some(prior.token.as_str()) {
                    self.check_entry(r, &e, &t.head, Some(&prior.token))?;
                    let tree = r.oid(&e.path, &format!("{}^{{tree}}", t.head))?;
                    maintenance::release_reconciled(
                        r,
                        &self.inputs.policy.coordinator_root,
                        report,
                        &prior_report,
                        &t.head,
                        &tree,
                        || self.receipt(receipt).map(|_| ()),
                    )?;
                }
            }
        }
        self.stable(r)?;
        self.git_locks(&e, None)?;
        self.pinned_retention(r, &t)?;
        let names: Vec<_> = self
            .inputs
            .policy
            .artifacts
            .iter()
            .filter(|a| select_artifact(&a.disposition, true, t.dispose_evidence))
            .map(|a| a.path.clone())
            .collect();
        let mut preserved = None;
        if t.artifacts_only {
            preserved = Some("terminal owner requested artifacts only".to_owned());
        } else if crate::verified_lifecycle::retain_evidence(
            t.dispose_evidence,
            self.evidence_present(&e.path)?,
        ) {
            preserved =
                Some("needed evidence remains retained by policy and terminal owner".to_owned());
        }
        if e.path.exists() {
            self.check_entry(r, &e, &t.head, None)?;
            if preserved.is_none() {
                if let Err(error) = maintenance::saved_commit(
                    r,
                    &self.inputs.policy.coordinator_root,
                    &t.head,
                    &self.inputs.policy.retained_ref,
                    &t.retained_commit,
                    &self.inputs.policy.retention,
                ) {
                    preserved = Some(format!("commit retention unresolved: {error}"));
                }
                let host_config = decode::<crate::Config>(&self.inputs.config_bytes)?.host_config;
                let (target_regs, _) = start::registrations(r, &e.path, &t.head, &host_config)?;
                let mut disposal = target_regs.config().clone();
                disposal["artifacts"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|a| names.iter().any(|p| a["path"] == *p));
                if preserved.is_none() {
                    if let Err(error) = start::cleanliness(r, &e.path, &disposal) {
                        preserved = Some(format!(
                            "checkout work or retained artifacts remain: {error}"
                        ));
                    }
                }
            }
        } else if e.attempts.is_empty() {
            return Err(
                "enrolled checkout disappeared without an original disposal attempt".into(),
            );
        }
        self.ledger.entries[i].attempts.push(Attempt {
            path: report["report_path"].as_str().ok_or("report path")?.into(),
            token: token.into(),
            receipt: None,
        });
        self.save()?;
        self.outside_enrollment_readonly(r, |manager, r| {
            if let Some(reason) = preserved {
                report["preserved_reason"] = value!(reason);
                report["worktree_removal"] = value!("preserved");
                report["branch_removal"] = value!("not-requested");
                report["artifact_disposals"] = value!([]);
                manager.check_entry(r, &e, &t.head, None)?;
                let host_config =
                    decode::<crate::Config>(&manager.inputs.config_bytes)?.host_config;
                let (target_regs, _) = start::registrations(r, &e.path, &t.head, &host_config)?;
                artifact_disposal::registered(
                    &names,
                    &[manager.inputs.registrations.config(), target_regs.config()],
                )?;
                artifact_disposal::paths(r, &e.path, &t.head, &names)?;
                report["destination"] = value!(e.path);
                report["branch_ref"] = value!(e.current_branch());
                report["head"] = value!(t.head);
                report["base"] = value!(t.head);
                crate::recovery::publish(&manager.inputs.policy.coordinator_root, report)?;
                let path = e.path.to_str().ok_or("path UTF-8")?;
                r.git(
                    &manager.inputs.policy.coordinator_root,
                    &["worktree", "lock", "--reason", token, "--", path],
                )?;
                manager.check_entry(r, &e, &t.head, Some(token))?;
                let paths = artifact_disposal::paths(r, &e.path, &t.head, &names)?;
                artifact_disposal::dispose(&e.path, &names, &paths, report, || {
                    manager.check_entry(r, &e, &t.head, Some(token))?;
                    manager.pinned_retention(r, &t)
                })?;
                manager.check_entry(r, &e, &t.head, Some(token))?;
                manager.pinned_retention(r, &t)?;
                start::unlock(
                    r,
                    &manager.inputs.policy.coordinator_root,
                    path,
                    &maintenance::checkout_reference(e.current_branch()),
                    &t.head,
                )?;
                manager.check_entry(r, &e, &t.head, None)?;
                manager.pinned_retention(r, &t)
            } else {
                Cleanup {
                    path: e.path.clone(),
                    branch: e.current_branch().into(),
                    head: t.head.clone(),
                    retained_ref: manager.inputs.policy.retained_ref.clone(),
                    retained_commit: t.retained_commit.clone(),
                    retention: manager.inputs.policy.retention.clone(),
                    discard_artifacts: names,
                    remove_branch: manager.inputs.policy.remove_branch,
                    allow_absent_worktree: !e.attempts.is_empty(),
                }
                .execute_checked(
                    r,
                    &manager.inputs.policy.coordinator_root,
                    &manager.inputs.registrations,
                    token,
                    report,
                    e.enrollment["kind"].as_str().unwrap_or(""),
                    |r| {
                        manager.stable(r)?;
                        manager.git_locks(&e, None)?;
                        manager.pinned_retention(r, &t)?;
                        if fs::symlink_metadata(&e.path).is_ok() {
                            let row = r
                                .inventory(&manager.inputs.policy.coordinator_root)?
                                .into_iter()
                                .find(|row| {
                                    row.get("worktree").is_some_and(|p| Path::new(p) == e.path)
                                })
                                .ok_or("enrolled checkout inventory disappeared")?;
                            let lock = row.get("locked").map(String::as_str);
                            if lock.is_some() && lock != Some(token) {
                                return Err("another worktree lock owns pending disposal".into());
                            }
                            manager.check_entry(r, &e, &t.head, lock)?;
                        }
                        Ok(())
                    },
                )
            }
        })
    }
    fn evidence_present(&self, target: &Path) -> Result<bool, String> {
        for artifact in self
            .inputs
            .policy
            .artifacts
            .iter()
            .filter(|a| a.disposition == Disposition::EvidenceRetain)
        {
            let path = no_symlink_parents(target, artifact.path.trim_end_matches('/'))?;
            match fs::symlink_metadata(path) {
                Ok(_) => return Ok(true),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(format!("cannot establish evidence presence: {error}")),
            }
        }
        Ok(false)
    }
}

/// Runs while the admission gate is held through drain, creation and enrollment.
pub(crate) fn create(
    r: &mut Runner,
    o: &Start,
    bytes: &[u8],
    token: &str,
    report: &mut Value,
    create: impl FnOnce(&mut Runner, &mut Value) -> Result<(), String>,
) -> Result<(), String> {
    let destination = absolute(
        &fs::canonicalize(o.destination.parent().ok_or("destination parent")?)
            .map_err(|e| e.to_string())?
            .join(o.destination.file_name().ok_or("destination name")?),
        true,
    )?;
    let Some(mut manager) = Manager::open(
        r,
        &o.root,
        &o.config_path,
        bytes,
        token,
        &destination,
        false,
        true,
    )?
    else {
        return create(r, report);
    };
    manager.protect_reports(r, &o.root)?;
    manager.preflight_birth(r, &destination)?;
    manager.drain_for_admission(r, &[o.root.clone(), destination.clone()], report)?;
    manager.stable(r)?;
    manager.preflight_birth(r, &destination)?;
    create(r, report)?;
    manager.stable(r)?;
    manager.enroll(r,Path::new(report["destination"].as_str().ok_or("birth destination")?),"birth",value!({"source_root":o.root,"report_path":report["report_path"],"operation":report["operation"],"base":report["base"],"token":report["lock_reason"],"creator_agent":crate::AgentIdentity::from_report(report)?}))?;
    report["cleanup_enrollment"] = value!({"coordinator_root":manager.inputs.policy.coordinator_root,"state_directory":manager.inputs.policy.state_directory,"kind":"birth"});
    manager.observe_attention(r, report, false)?;
    Ok(())
}

/// Preserve the original producer report at the surviving anchor before admitting use/finish.
pub(crate) fn seal_birth(
    r: &mut Runner,
    root: &Path,
    config_path: &str,
    bytes: &[u8],
    report: &Value,
) -> Result<(), String> {
    let token = report["lock_reason"].as_str().ok_or("birth token")?;
    let target = Path::new(report["destination"].as_str().ok_or("birth destination")?);
    let Some(mut manager) = Manager::open(r, root, config_path, bytes, token, target, false, true)?
    else {
        return Ok(());
    };
    let i = manager
        .current_entry(target)
        .ok_or("birth enrollment missing")?;
    if manager.ledger.entries[i].status != "active"
        || manager.ledger.entries[i].enrollment["kind"] != "birth"
        || manager.ledger.entries[i].enrollment["receipt"]["report_path"] != report["report_path"]
    {
        return Err("birth enrollment changed before receipt publication".into());
    }
    let receipt = manager.immutable(&format!("birth-{token}.json"), report)?;
    manager.ledger.entries[i].enrollment["sealed_receipt"] = value!(receipt);
    manager.save()
}

pub(crate) fn dispatch(args: &[String]) -> Result<Value, String> {
    let operation = args.first().ok_or("missing lifecycle command")?;
    let mut values = BTreeMap::new();
    let mut flags = BTreeSet::new();
    let mut n = 1;
    while n < args.len() {
        let key = &args[n];
        if operation == "check" && key == "--collect" {
            if !flags.insert("--collect") {
                return Err("repeated check selection".into());
            }
            n += 1;
            continue;
        }
        if ["--dispose-evidence", "--artifacts-only"].contains(&key.as_str()) {
            if operation != "finish" || !flags.insert(key.as_str()) {
                return Err("invalid or repeated terminal flag".into());
            }
            n += 1;
            continue;
        }
        if ![
            "--host-root",
            "--config",
            "--path",
            "--operation",
            "--retained-commit",
            "--unit",
            "--bootstrap-config",
            "--adopt-cache",
            "--adopt-fixtures",
            "--partition-producer",
            "--stage-result",
            "--stage-sha256",
            "--report-consumer",
        ]
        .contains(&key.as_str())
            || n + 1 >= args.len()
            || args[n + 1].is_empty()
            || values.insert(key.as_str(), args[n + 1].as_str()).is_some()
        {
            return Err("unknown, repeated or missing lifecycle argument".into());
        }
        n += 2;
    }
    if (values.contains_key("--bootstrap-config")
        && !matches!(operation.as_str(), "bootstrap" | "deploy-owner"))
        || (operation == "bootstrap" && !values.contains_key("--bootstrap-config"))
        || (values.contains_key("--path") && operation == "bootstrap")
        || (values.contains_key("--unit") && operation != "check")
        || (values.contains_key("--unit") && flags.contains("--collect"))
        || (values.contains_key("--path") && operation == "check")
        || (values.contains_key("--operation") && operation != "use")
        || (operation == "use" && !values.contains_key("--operation"))
        || (values.contains_key("--retained-commit") && operation != "finish")
        || (operation == "maintain" && values.contains_key("--path"))
        || (operation == "migrate" && !values.contains_key("--path"))
        || (values.contains_key("--adopt-cache") && operation != "migrate")
        || (values.contains_key("--adopt-fixtures") && operation != "migrate")
        || (values.contains_key("--partition-producer") && operation != "migrate")
        || (values.contains_key("--stage-result") && operation != "deploy-owner")
        || (values.contains_key("--stage-sha256") && operation != "deploy-owner")
        || (operation == "deploy-owner"
            && (!values.contains_key("--bootstrap-config")
                || !values.contains_key("--stage-result")
                || !values.contains_key("--stage-sha256")
                || values.contains_key("--path")))
    {
        return Err("invalid lifecycle command arguments".into());
    }
    let invoking = fs::canonicalize(values.get("--host-root").copied().unwrap_or("."))
        .map_err(|e| e.to_string())?;
    if values
        .get("--report-consumer")
        .is_some_and(|name| name.trim().is_empty() || name.len() > 247)
    {
        return Err("invalid bounded report consumer identity".into());
    }
    let config_path = values
        .get("--config")
        .copied()
        .unwrap_or(".chrono-harness/worktree.json");
    let (config, bytes) = crate::configuration(&invoking, config_path)?;
    if config.automatic_cleanup.is_none() {
        return Err("automatic cleanup is not opted in".into());
    }
    let target = values
        .get("--path")
        .map(|p| invoking.join(p))
        .unwrap_or_else(|| invoking.clone());
    let target =
        if operation == "maintain" || (operation == "status" && !values.contains_key("--path")) {
            None
        } else {
            Some(absolute(&target, operation == "status")?)
        };
    let success = match operation.as_str() {
        "finish" => "finished",
        "maintain" => "maintained",
        "status" => "observed",
        "import" => "imported",
        "migrate" => "migrated",
        "deploy-owner" => "owner-deployed",
        "use" | "check" | "bootstrap" => "used",
        _ => return Err("unknown lifecycle operation".into()),
    };
    let mut _use_guard = None;
    start::with_report_for_consumer(
        &invoking,
        config_path,
        config,
        &bytes,
        operation,
        success,
        values.get("--report-consumer").copied(),
        |r, report| {
            report["invoking_root"] = value!(invoking);
            let inputs = PolicyInputs::load(r, &invoking, config_path, &bytes)?
                .ok_or("automatic cleanup is not opted in")?;
            Ok((inputs.policy.coordinator_root.clone(), inputs))
        },
        |r, token, report, inputs| {
            if operation == "status" {
                inputs.report_source(report);
                return observe_status(
                    r,
                    &inputs,
                    target.as_deref().unwrap_or(&invoking),
                    report,
                    target.is_none(),
                );
            }
            let mut manager = Manager::from_inputs(
                r,
                inputs,
                token,
                target.as_deref().unwrap_or(&invoking),
                operation == "deploy-owner",
                true,
            )?;
            manager.restore_absent_policy(r)?;
            manager.protect_reports(r, &invoking)?;
            if !matches!(operation.as_str(), "maintain" | "import") {
                manager.adopt_live_policy(r)?;
            }
            manager.inputs.report_source(report);
            match operation.as_str() {
                "deploy-owner" => manager.deploy_owner(
                    r,
                    &invoking,
                    values["--bootstrap-config"],
                    values["--stage-result"],
                    values["--stage-sha256"],
                    report,
                ),
                "maintain" => {
                    let result = manager.drain(r, &[invoking.clone()], report, true);
                    manager.observe_attention(r, report, true)?;
                    result
                }
                "import" => {
                    manager.enroll(
                    r,
                    target.as_ref().unwrap(),
                    if manager.inputs.policy.main_cache_only
                        && target.as_ref().unwrap() == &manager.inputs.policy.coordinator_root
                    {
                        "main-cache"
                    } else {
                        "import"
                    },
                    value!({"report_path":report["report_path"],"historical_birth":"not-claimed",
                        "custodian_agent":crate::AgentIdentity::from_report(report)?}),
                    )?;
                    manager.observe_attention(r, report, false)
                }
                "migrate" => {
                    let target = target.as_ref().unwrap();
                    manager.migrate(r, target, report)?;
                    if let Some(producer) = values.get("--partition-producer") {
                        manager.partition_producer(r, target, producer, report)?;
                    }
                    if let Some(plan) = values.get("--adopt-cache") {
                        manager.adopt_main_cache(r, target, plan, report)?;
                    }
                    if let Some(plan) = values.get("--adopt-fixtures") {
                        manager.adopt_fixture_bodies(r, target, plan, report)?;
                    }
                    manager.observe_attention(r, report, false)
                }
                "finish" => {
                    manager.finish(
                        r,
                        target.as_ref().unwrap(),
                        flags.contains("--dispose-evidence"),
                        flags.contains("--artifacts-only"),
                        values.get("--retained-commit").copied(),
                        token,
                        report,
                    )?;
                    let result = manager.drain(r, &[invoking.clone()], report, false);
                    manager.observe_attention(r, report, false)?;
                    result
                }
                "use" | "check" | "bootstrap" => {
                    let target = target.as_ref().unwrap();
                    manager.drain_for_admission(r, &[invoking.clone(), target.clone()], report)?;
                    if manager.current_entry(target).is_none()
                        && target == &manager.inputs.policy.coordinator_root
                        && manager.inputs.policy.main_cache_only
                    {
                        manager.enroll(
                            r,
                            target,
                            "main-cache",
                            value!({"report_path":report["report_path"]}),
                        )?;
                    }
                    let selection = if flags.contains("--collect") {
                        vec!["--collect".to_owned()]
                    } else if let Some(unit) = values.get("--unit") {
                        vec!["--unit".to_owned(), (*unit).to_owned()]
                    } else {
                        vec![]
                    };
                    let i = match manager.current_entry(target) {
                        Some(i) => i,
                        None if matches!(operation.as_str(), "check" | "bootstrap")
                            && target == &manager.inputs.policy.coordinator_root =>
                        {
                            let command = if operation == "bootstrap" {
                                bootstrap_command(
                                    r,
                                    target,
                                    &manager.inputs.registrations,
                                    values["--bootstrap-config"],
                                )?
                            } else {
                                canonical_command(
                                    target,
                                    &manager.inputs.registrations,
                                    &selection,
                                )?
                            };
                            drop(manager);
                            let digest =
                                chrono_harness::file_identity(Path::new(&command.program))?.0;
                            let process = chrono_harness::prepared::process_identities(
                                chrono_harness::run_process_observed(
                                    target,
                                    &command,
                                    &[],
                                    &digest,
                                )?,
                            );
                            report["managed_process"] =
                                chrono_harness::full::compact_process(&value!(&process))?;
                            report["managed_process"]["environment_representation"] =
                                value!("sha256");
                            // This main checkout has no cache enrollment, but
                            // its caller still consumes the agent's pending work.
                            let observer = Manager::open(
                                r,
                                &invoking,
                                config_path,
                                &bytes,
                                token,
                                target,
                                false,
                                true,
                            )?
                            .ok_or("cleanup policy disappeared during main check")?;
                            observer.observe_attention(r, report, false)?;
                            if process.failure.is_some() || process.exit_code != 0 {
                                report["managed_command_failed"] = value!(true);
                                return Err(format!(
                                    "check failed with exit {}",
                                    process.exit_code
                                ));
                            }
                            return Ok(());
                        }
                        None => return Err("managed use requires an enrolled worktree".into()),
                    };
                    let e = manager.ledger.entries[i].clone();
                    if e.status != "active" {
                        return Err("terminal worktree refuses new managed use".into());
                    }
                    if e.enrollment["kind"] == "birth"
                        && e.enrollment["sealed_receipt"].is_null()
                        && e.enrollment["recovered_receipt"].is_null()
                    {
                        let lease = manager
                            .lease(&e, true)?
                            .ok_or("birth still has live registered consumers; preserve it")?;
                        manager.reconcile_birth(r, i, &lease)?;
                    }
                    let e = manager.ledger.entries[i].clone();
                    manager.birth_ready(&e)?;
                    let head = r.oid(target, "HEAD")?;
                    manager.check_entry(r, &e, &head, None)?;
                    let host_config =
                        decode::<crate::Config>(&manager.inputs.config_bytes)?.host_config;
                    let target_snapshot = if head == manager.inputs.source_head
                        && host_config == manager.inputs.registrations.entry_path()
                    {
                        None
                    } else {
                        Some(start::registrations(r, target, &head, &host_config)?)
                    };
                    let target_regs = target_snapshot.as_ref().map_or(
                        manager.inputs.registrations.as_ref(),
                        |(registrations, _)| registrations,
                    );
                    let mut command = if operation == "check" {
                        canonical_command(target, target_regs, &selection)?
                    } else if operation == "bootstrap" {
                        bootstrap_command(r, target, target_regs, values["--bootstrap-config"])?
                    } else {
                        registered_command(target, target_regs, values["--operation"])?
                    };
                    let use_operation = if operation == "check" {
                        "validate.delta"
                    } else if operation == "bootstrap" {
                        "bootstrap.build"
                    } else {
                        values["--operation"]
                    };
                    let artifact_producer = manager
                        .inputs
                        .policy
                        .retained_producers
                        .iter()
                        .find(|producer| {
                            producer.operations.iter().any(|name| name == use_operation)
                        })
                        .map(|producer| producer.id.clone());
                    if e.ownership.is_some() {
                        _use_guard = Some(
                            manager
                                .lease(&e, false)?
                                .ok_or("enrollment lease is busy")?,
                        );
                        let published_attempt = manager.ledger.entries[i]
                            .cache_attempts
                            .iter()
                            .any(|a| a.generation == e.ownership.as_ref().unwrap().generation);
                        let owner = manager.ledger.entries[i].ownership.as_mut().unwrap();
                        if !owner.cache_pending || published_attempt {
                            owner.generation = owner
                                .generation
                                .checked_add(1)
                                .ok_or("cache generation overflow")?;
                        }
                        owner.cache_pending = true;
                        let owner = owner.clone();
                        manager.immutable(&format!("use-intent-{token}.json"), &value!({"schema":"chrono-worktree-managed-use-intent/v2","path":target,"attachment":e.attachment,"lease_path":owner.path,"lease_id":owner.id,"operation":use_operation,"report_path":report["report_path"]}))?;
                    }
                    manager.ledger.entries[i].uses.push(token.into());
                    manager.save()?;
                    let coordinator = manager.inputs.policy.coordinator_root.clone();
                    manager.observe_attention(r, report, false)?;
                    let coordinator_bytes = manager.inputs.anchor_config_bytes.clone();
                    drop(manager);
                    let mut artifact = artifact_producer
                        .as_deref()
                        .map(|producer| {
                            TemporaryHost::registered_allocate(
                                &coordinator,
                                config_path,
                                producer,
                                "run-",
                            )
                        })
                        .transpose()?;
                    if let Some(artifact) = &artifact {
                        let directory = artifact
                            .path()
                            .to_str()
                            .ok_or("artifact directory UTF8")?
                            .to_owned();
                        command
                            .env
                            .insert("CHRONO_ARTIFACT_DIRECTORY".into(), directory.clone());
                        command.env.insert("TMPDIR".into(), directory);
                        report["artifact_directory"] =
                            value!({"producer":artifact_producer,"path":artifact.path()});
                    }
                    let digest = chrono_harness::file_identity(Path::new(&command.program))?.0;
                    let process = chrono_harness::prepared::process_identities(
                        chrono_harness::run_process_observed(target, &command, &[], &digest)?,
                    );
                    report["managed_process"] =
                        chrono_harness::full::compact_process(&value!(&process))?;
                    report["managed_process"]["environment_representation"] = value!("sha256");
                    let artifact_error = if let Some(artifact) = &mut artifact {
                        match artifact.finish(process.failure.is_some() || process.exit_code != 0) {
                            Ok(handoff) => {
                                report["artifact_directory"]["handoff"] = value!({"status":handoff["status"],"report_path":handoff["report_path"],"error":handoff["error"]});
                                (handoff["status"] != "finished").then(|| {
                                    format!(
                                        "artifact directory handoff failed: {}",
                                        handoff["error"]
                                    )
                                })
                            }
                            Err(error) => {
                                report["artifact_directory"]["error"] = value!(error);
                                Some(error)
                            }
                        }
                    } else {
                        None
                    };
                    let mut manager = Manager::open(
                        r,
                        &coordinator,
                        config_path,
                        &coordinator_bytes,
                        token,
                        target,
                        false,
                        true,
                    )?
                    .ok_or("cleanup policy disappeared during managed use")?;
                    let receipt=manager.immutable(&format!("use-{token}.json"),&value!({"schema":"chrono-worktree-managed-use-result/v1","path":target,"operation":use_operation,"report_path":report["report_path"],"process":report["managed_process"]}))?;
                    report["managed_use_receipt"] = value!(receipt);
                    crate::recovery::bind_report_inputs(&coordinator, report)?;
                    let i = manager
                        .current_entry(target)
                        .ok_or("managed enrollment disappeared")?;
                    if manager.ledger.entries[i].attachment != e.attachment
                        || manager.ledger.entries[i].enrollment["receipt"]
                            != e.enrollment["receipt"]
                        || !manager.ledger.entries[i].uses.iter().any(|s| s == token)
                    {
                        return Err("managed use identity disappeared".into());
                    }
                    manager.ledger.entries[i]
                        .enrollment
                        .as_object_mut()
                        .ok_or("enrollment object")?
                        .entry("completed_uses")
                        .or_insert(value!([]))
                        .as_array_mut()
                        .ok_or("completed use history")?
                        .push(value!(receipt));
                    manager.ledger.entries[i].uses.retain(|s| s != token);
                    manager.save()?;
                    if process.failure.is_some() || process.exit_code != 0 {
                        report["managed_command_failed"] = value!(true);
                        return Err(format!(
                            "registered managed command failed with exit {}: {:?}",
                            process.exit_code, process.failure
                        ));
                    }
                    if let Some(error) = artifact_error {
                        return Err(error);
                    }
                    let mut post_use_cleanup = value!({});
                    let cleanup = manager.drain_for_admission(
                        r,
                        &[invoking.clone(), target.clone()],
                        &mut post_use_cleanup,
                    );
                    report["post_use_cleanup"] = post_use_cleanup;
                    cleanup?;
                    Ok(())
                }
                _ => unreachable!(),
            }
        },
    )
}

/// Warnings are a projection of the already-persisted report, never new state.
pub(crate) fn attention_reminder(report: &Value) -> String {
    let mut text = String::new();
    let status = &report["worktree_status"];
    let known = status["agent"]["status"] == "known";
    if let Some(rows) = status["worktrees"].as_array() {
        let mut total = 0;
        for row in rows {
            let required = if known {
                row["agent_attention_required"] == true
            } else {
                row["attention_required"] == true
            };
            if required {
                total += 1;
                if total <= 64 {
                    let detail: String = row["cleanup_progress"]["detail"]
                        .as_str()
                        .unwrap_or("")
                        .chars()
                        .take(512)
                        .collect();
                    text.push_str(&format!(
                        "W_WORKTREE_ATTENTION: creator={} path={} branch={} work_state={} cleanup_state={} detail={} next_action={}; inspect chrono-worktree status and maintain unfinished work.\n",
                        row["creator_agent"], row["path"], row["branch"], row["work_state"],
                        row["cleanup_state"], value!(detail), row["next_action"]
                    ));
                }
            }
        }
        if total > 64 {
            text.push_str(&format!("W_WORKTREE_ATTENTION: {total} pending objects; showing 64; inspect chrono-worktree status for all rows.\n"));
        }
        if status["agent_observation_complete"] != true {
            text.push_str(&format!("W_WORKTREE_AGENT_SCOPE: agent={} unknown_creator_records={} unreadable_records={} namespace_error={}; observation is incomplete; unknown ownership is preserved.\n",
                status["agent"], status["unknown_creator_records"], status["unreadable_records"], status["namespace_error"]));
        }
    }
    text
}

fn registered_command(
    root: &Path,
    registrations: &Registrations,
    operation: &str,
) -> Result<CommandSpec, String> {
    let mut selected = vec![];
    for category in ["projects", "scripts"] {
        for item in registrations.projects()[category]
            .as_array()
            .ok_or("missing consuming command registrations")?
        {
            for action in item["actions"]
                .as_object()
                .ok_or("missing registered actions")?
                .values()
            {
                if action["operation"] == operation {
                    selected.push(action);
                }
            }
        }
    }
    if selected.len() != 1 {
        return Err("managed use requires one exact registered project/script operation".into());
    }
    action_command(root, registrations.config(), selected[0])
}
fn action_command(root: &Path, config: &Value, a: &Value) -> Result<CommandSpec, String> {
    let tool = config["tools"]
        .as_array()
        .ok_or("missing registered tools")?
        .iter()
        .find(|t| t["id"] == a["tool"])
        .ok_or("consuming tool is not registered")?;
    program_command(
        root,
        config,
        tool["program"].as_str().ok_or("registered program")?,
        serde_json::from_value(a["argv"].clone()).map_err(|e| e.to_string())?,
    )
}
fn program_command(
    root: &Path,
    config: &Value,
    program: &str,
    argv: Vec<String>,
) -> Result<CommandSpec, String> {
    let mut environment = BTreeMap::new();
    for key in config["environment"]["inherit"]
        .as_array()
        .ok_or("missing command environment")?
    {
        let key = key.as_str().ok_or("environment name")?;
        match std::env::var(key) {
            Ok(v) => {
                environment.insert(key.into(), v);
            }
            Err(std::env::VarError::NotPresent) => (),
            Err(e) => return Err(e.to_string()),
        }
    }
    for (k, v) in config["environment"]["values"]
        .as_object()
        .ok_or("missing command environment values")?
    {
        environment.insert(k.clone(), v.as_str().ok_or("environment value")?.into());
    }
    let program = chrono_harness::resolve_program(
        root,
        program,
        environment.get("PATH").map(String::as_str),
    )?;
    let command = CommandSpec {
        program: program.to_str().ok_or("command path UTF-8")?.into(),
        args: argv,
        env: environment,
        timeout_seconds: config["protocol"]["timeout_seconds"]
            .as_u64()
            .ok_or("registered command timeout")?,
        output_limit_bytes: config["protocol"]["stdout_limit_bytes"]
            .as_u64()
            .ok_or("registered output limit")? as usize,
    };
    chrono_harness::validate_command(&command)?;
    Ok(command)
}

fn bootstrap_policy(
    r: &mut Runner,
    root: &Path,
    registrations: &Registrations,
    path: &str,
) -> Result<Value, String> {
    relative_path(path)?;
    if !path.starts_with(".chrono-harness/ci/") {
        return Err("bootstrap configuration must be a registered CI policy".into());
    }
    let head = r.oid(root, "HEAD")?;
    let bytes = fs::read(no_symlink_parents(root, path)?).map_err(|e| e.to_string())?;
    if r.blob(root, &head, path)? != bytes {
        return Err("bootstrap configuration differs from commit".into());
    }
    let policy = json(&bytes)?;
    if policy["schema"] != "chrono-bootstrap/v1" || policy["participation"].is_null() {
        return Err("bootstrap has not adopted participation".into());
    }
    let script = policy["entrypoint"]["script"]
        .as_str()
        .ok_or("bootstrap entrypoint script")?;
    relative_path(script)?;
    start::registered_policy(registrations, path, &r.config.report_directory)?;
    start::registered_policy(registrations, script, &r.config.report_directory)?;
    let source = fs::read(no_symlink_parents(root, script)?).map_err(|e| e.to_string())?;
    if r.blob(root, &head, script)? != source {
        return Err("bootstrap entrypoint differs from commit".into());
    }
    Ok(policy)
}

fn bootstrap_command(
    r: &mut Runner,
    root: &Path,
    registrations: &Registrations,
    path: &str,
) -> Result<CommandSpec, String> {
    let policy = bootstrap_policy(r, root, registrations, path)?;
    let script = policy["entrypoint"]["script"]
        .as_str()
        .ok_or("bootstrap entrypoint script")?;
    let action = value!({"tool":policy["entrypoint"]["tool"],"argv":[script,root,path]});
    let mut command = action_command(root, registrations.config(), &action)?;
    command.env.insert(
        "CHRONO_WORKTREE_BOOTSTRAP".into(),
        root.to_str().ok_or("bootstrap root UTF-8")?.into(),
    );
    Ok(command)
}

fn canonical_command(
    root: &Path,
    registrations: &Registrations,
    selection: &[String],
) -> Result<CommandSpec, String> {
    let config = registrations.config();
    let canonical = chrono_harness::prepared::declaration(config)?;
    let participation = canonical
        .participation
        .ok_or("canonical check has not adopted worktree participation")?;
    if participation.operation != "worktree.check" || participation.argv != ["check"] {
        return Err("canonical participation must declare worktree.check with check argv".into());
    }
    let mut argv = vec!["check".to_owned()];
    argv.extend_from_slice(selection);
    let mut command = program_command(
        root,
        config,
        config["runner"]["path"]
            .as_str()
            .ok_or("canonical runner path")?,
        argv,
    )?;
    command.env.insert(
        "CHRONO_WORKTREE_CHECK".into(),
        root.to_str().ok_or("check root UTF-8")?.into(),
    );
    Ok(command)
}
