//! Opt-in lifecycle cleanup with explicit terminal and kernel-quiescent cache phases.
mod migration;
use crate::{
    Start, artifact_disposal,
    maintenance::{self, Cleanup, Retention},
    ownership::Lease,
    start::{self, Runner},
};
use chrono_harness::{CommandSpec, decode, facts, json, no_symlink_parents, relative_path, sha256};
use chrono_judge_registration::Registrations;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json as value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
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
    artifacts: Vec<Artifact>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    path: String,
    disposition: Disposition,
}
#[derive(Clone, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum Disposition {
    Dispose,
    Retain,
    EvidenceRetain,
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
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RetainedInput {
    schema: String,
    source_root: PathBuf,
    path: String,
    sha256: String,
    byte_length: usize,
    format: String,
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
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    path: PathBuf,
    branch: String,
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
}
struct Gate {
    path: PathBuf,
    token: String,
    lease: Option<Lease>,
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
struct Manager {
    policy: Policy,
    policy_path: String,
    policy_bytes: Vec<u8>,
    config_path: String,
    config_bytes: Vec<u8>,
    anchor_head: String,
    registrations: Registrations,
    registry_digest: String,
    ledger: Ledger,
    directory: PathBuf,
    _gate: Gate,
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
fn write_json(path: &Path, v: &impl Serialize, immutable: bool) -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec_pretty(v).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    write_bytes(path, &bytes, immutable)?;
    Ok(bytes)
}
fn write_bytes(path: &Path, bytes: &[u8], immutable: bool) -> Result<(), String> {
    let mut out = tempfile::Builder::new()
        .prefix(".publish-")
        .tempfile_in(path.parent().ok_or("publication parent")?)
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
    let Some(path) = r.config.automatic_cleanup.clone() else {
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
    let mut p: Policy = decode(&policy_bytes)?;
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
    Ok(Some((p, path, policy_bytes)))
}
impl Manager {
    fn open(
        r: &mut Runner,
        root: &Path,
        config_path: &str,
        bytes: &[u8],
        token: &str,
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
        if fs::read(no_symlink_parents(anchor, config_path)?).map_err(|e| e.to_string())? != bytes
            || r.blob(anchor, &anchor_head, config_path)? != bytes
            || fs::read(no_symlink_parents(anchor, &policy_path)?).map_err(|e| e.to_string())?
                != policy_bytes
            || r.blob(anchor, &anchor_head, &policy_path)? != policy_bytes
        {
            return Err("coordinator policy/configuration identity mismatch".into());
        }
        let host_config = r.config.host_config.clone();
        let (registrations, registry_digest) =
            start::registrations(r, anchor, &anchor_head, &host_config)?;
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
        start::registered_policy(&registrations, config_path, &r.config.report_directory)?;
        let directory = no_symlink_parents(anchor, policy.state_directory.trim_end_matches('/'))?;
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let kernel = policy.schema == "chrono-worktree-automatic-cleanup/v2";
        let ledger_path =
            no_symlink_parents(anchor, &format!("{}ledger.json", policy.state_directory))?;
        let marker = no_symlink_parents(anchor, &format!("{}owner.json", policy.state_directory))?;
        let gate_path =
            no_symlink_parents(anchor, &format!("{}gate.lock", policy.state_directory))?;
        let gate = if kernel {
            if gate_path.exists() {
                return Err(
                    "legacy admission ownership is unknown; preserve its gate and records".into(),
                );
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
                        .ok_or("missing admission identity")
                })
                .transpose()?;
            let path = no_symlink_parents(
                anchor,
                &format!("{}admission.lease", policy.state_directory),
            )?;
            let lease = Lease::acquire(
                &path,
                expected,
                expected.is_none(),
                true,
                Some(std::time::Duration::from_secs(r.config.timeout_seconds)),
            )?
            .ok_or("admission unavailable")?;
            Gate {
                path,
                token: token.into(),
                lease: Some(lease),
            }
        } else {
            let waiting = std::time::Instant::now();
            loop {
                match fs::create_dir(&gate_path) {
                    Ok(()) => break,
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                        if waiting.elapsed()
                            >= std::time::Duration::from_secs(r.config.timeout_seconds)
                        {
                            return Err("automatic cleanup admission remains busy or interrupted; no lock was expired".into());
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
            };
            fs::write(gate.path.join("owner"), token).map_err(|e| e.to_string())?;
            gate
        };
        let schema = if kernel {
            "chrono-worktree-cleanup-state/v2"
        } else {
            "chrono-worktree-cleanup-state/v1"
        };
        let admission_id = gate.lease.as_ref().map(|l| l.id().to_owned());
        let expected_owner = if kernel {
            value!({"schema":"chrono-worktree-cleanup-owner/v2","coordinator_root":anchor,"common":common,"admission_id":admission_id})
        } else {
            value!({"schema":"chrono-worktree-cleanup-owner/v1","coordinator_root":anchor,"common":common})
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
        for e in &ledger.entries {
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
        Ok(Some(Self {
            policy,
            policy_path,
            policy_bytes,
            config_path: config_path.into(),
            config_bytes: bytes.into(),
            anchor_head,
            registrations,
            registry_digest,
            ledger,
            directory,
            _gate: gate,
        }))
    }
    fn save(&self) -> Result<(), String> {
        no_symlink_parents(
            &self.policy.coordinator_root,
            &format!("{}ledger.json", self.policy.state_directory),
        )?;
        write_json(&self.directory.join("ledger.json"), &self.ledger, false)?;
        Ok(())
    }
    fn stable(&self, r: &mut Runner) -> Result<(), String> {
        if let Some(lease) = &self._gate.lease {
            lease.stable()?;
        }
        let anchor = &self.policy.coordinator_root;
        if r.oid(anchor, "HEAD")? != self.anchor_head
            || fs::read(no_symlink_parents(anchor, &self.config_path)?)
                .map_err(|e| e.to_string())?
                != self.config_bytes
            || fs::read(no_symlink_parents(anchor, &self.policy_path)?)
                .map_err(|e| e.to_string())?
                != self.policy_bytes
        {
            return Err(
                "coordinator HEAD/configuration/policy changed; preserve pending work".into(),
            );
        }
        Ok(())
    }
    fn pinned_retention(&self, r: &mut Runner, t: &Terminal) -> Result<(), String> {
        if r.command(
            &self.policy.coordinator_root,
            &["symbolic-ref", "--quiet", &self.policy.retained_ref],
        )?
        .exit_code
            != 1
            || r.oid(&self.policy.coordinator_root, &self.policy.retained_ref)? != t.retained_commit
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
        for path in [&self.config_path, &self.policy_path] {
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
        if binding.policy_sha256 != sha256(&self.policy_bytes) {
            return Err("enrolled policy changed; explicit migration is required".into());
        }
        self.check_attachment(r, e, head, lock)?;
        if self.target_policy_inputs(r, &e.path, head)? != binding.inputs {
            return Err("enrolled target policy/configuration changed; preserve work for explicit migration".into());
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
        if self._gate.lease.is_some() && e.ownership.is_none() {
            return Err(
                "kernel enrollment ownership is missing; preserve legacy/unknown work".into(),
            );
        }
        if let Some(owner) = &e.ownership {
            if self._gate.lease.is_none() || !owner.path.starts_with(&self.policy.state_directory) {
                return Err("enrollment ownership policy mismatch".into());
            }
            let path = no_symlink_parents(&self.policy.coordinator_root, &owner.path)?;
            if crate::ownership::identity(&path)? != owner.id {
                return Err("enrollment lease identity changed".into());
            }
        }
        absolute(&e.path, false)?;
        let observed = maintenance::identity(
            r,
            &self.policy.coordinator_root,
            &e.path,
            &e.branch,
            head,
            lock,
        )?;
        if attachment_at(&e.path, &observed.metadata)? != e.attachment {
            return Err("enrolled checkout attachment changed".into());
        }
        self.git_locks(e)?;
        self.stable(r)
    }
    fn git_locks(&self, e: &Entry) -> Result<(), String> {
        let mut locks = vec![
            e.attachment.metadata.join("index.lock"),
            e.attachment.metadata.join("HEAD.lock"),
            self.ledger.common.join("packed-refs.lock"),
            self.ledger.common.join("config.lock"),
        ];
        for reference in [
            format!("refs/heads/{}", e.branch),
            self.policy.retained_ref.clone(),
        ] {
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
        if r.inventory(&self.policy.coordinator_root)?
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
        let target = absolute(target, false)?;
        self.enrollment_available(&target)?;
        let branch = r
            .text(&target, &["symbolic-ref", "--short", "HEAD"])?
            .trim()
            .to_owned();
        if !["feature_prefix", "integration_prefix"].iter().any(|k| {
            self.registrations.workflow()[k]
                .as_str()
                .is_some_and(|p| branch.starts_with(p))
        }) {
            return Err("enrollment branch is not a registered work branch".into());
        }
        let head = r.oid(&target, "HEAD")?;
        let attached = attachment(r, &target)?;
        let policy_inputs = self.target_policy_inputs(r, &target, &head)?;
        if kind == "birth"
            && (policy_inputs.get(&self.config_path) != Some(&Some(sha256(&self.config_bytes)))
                || policy_inputs.get(&self.policy_path) != Some(&Some(sha256(&self.policy_bytes))))
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
                self.policy.state_directory,
                sha256(token.as_bytes())
            );
            let physical = no_symlink_parents(&self.policy.coordinator_root, &path)?;
            if physical.exists() {
                return Err("enrollment lease identity already exists".into());
            }
            let lease = Lease::acquire(&physical, None, true, true, None)?
                .ok_or("new enrollment lease unavailable")?;
            let owner = Ownership {
                path,
                id: lease.id().into(),
                cache_pending: true,
                generation: 1,
            };
            if kind == "birth" {
                birth_lease = Some(lease);
            }
            Some(owner)
        } else {
            None
        };
        let entry = Entry {
            path: target,
            branch,
            attachment: attached,
            policy_sha256: sha256(&self.policy_bytes),
            status: "active".into(),
            enrollment: value!({"kind":kind,"observed_head":head,"receipt":receipt,"policy_inputs":policy_inputs}),
            uses: vec![],
            terminal: None,
            attempts: vec![],
            ownership,
            cache_attempts: vec![],
            policy_migrations: vec![],
        };
        self.check_entry(r, &entry, &head, None)?;
        let host_config = r.config.host_config.clone();
        let target_snapshot =
            if head == self.anchor_head && host_config == self.registrations.entry_path() {
                None
            } else {
                Some(start::registrations(r, &entry.path, &head, &host_config)?)
            };
        let target_registrations = target_snapshot
            .as_ref()
            .map_or(&self.registrations, |(registrations, _)| registrations);
        artifact_disposal::registered(
            &self
                .policy
                .artifacts
                .iter()
                .map(|a| a.path.clone())
                .collect::<Vec<_>>(),
            &[target_registrations.config()],
        )?;
        for (path, digest) in &policy_inputs {
            if digest.is_some() {
                start::registered_policy(
                    &target_registrations,
                    path,
                    &self.policy.state_directory,
                )?;
            }
        }
        self.ledger.entries.push(entry);
        self.save()?;
        r.birth_lease = birth_lease;
        Ok(())
    }
    fn birth_binding(e: &Entry) -> Value {
        value!({"path":e.path,"branch":e.branch,"attachment":e.attachment,
            "policy_sha256":e.policy_sha256,"observed_head":e.enrollment["observed_head"],
            "policy_inputs":e.enrollment["policy_inputs"],"original_receipt":e.enrollment["receipt"],
            "ownership":e.ownership.as_ref().map(|o| value!({"path":o.path,"id":o.id}))})
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
            &self.policy.coordinator_root,
            &format!("{}{name}", self.policy.state_directory),
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
                    path: format!("{}{name}", self.policy.state_directory),
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
                            &format!("{}birth-{t}.json", self.policy.state_directory),
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
        let path = format!("{}{name}", self.policy.state_directory);
        let target = no_symlink_parents(&self.policy.coordinator_root, &path)?;
        let b = write_json(&target, v, true)?;
        Ok(Receipt {
            path,
            sha256: sha256(&b),
        })
    }
    fn immutable_reconciled(&self, name: &str, value: &Value) -> Result<Receipt, String> {
        let path = format!("{}{name}", self.policy.state_directory);
        let physical = no_symlink_parents(&self.policy.coordinator_root, &path)?;
        match fs::read(&physical) {
            Ok(bytes) => {
                if json(&bytes)? != *value {
                    return Err("interrupted use reconciliation receipt changed".into());
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
        let b = crate::recovery::state_bytes(&self.policy.coordinator_root, &receipt.path)?;
        if sha256(&b) != receipt.sha256 {
            return Err("original lifecycle receipt changed".into());
        }
        self.verify_inputs(&json(&b)?)?;
        Ok(b)
    }
    fn retained_input(&self, path: &str, bytes: &[u8], format: &str) -> Value {
        value!(RetainedInput {
            schema: "chrono-worktree-retained-input/v1".into(),
            source_root: self.policy.coordinator_root.clone(),
            path: path.into(),
            sha256: sha256(bytes),
            byte_length: bytes.len(),
            format: format.into(),
        })
    }
    /// Preserve partial/unsealed bytes independently of their mutable source.
    fn retain_bytes(&self, bytes: &[u8]) -> Result<Value, String> {
        let path = format!("{}input-{}.bin", self.policy.state_directory, sha256(bytes));
        let physical = no_symlink_parents(&self.policy.coordinator_root, &path)?;
        match fs::symlink_metadata(&physical) {
            Ok(_) => {
                if crate::recovery::state_bytes(&self.policy.coordinator_root, &path)? != bytes {
                    return Err("retained lifecycle input changed".into());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                write_bytes(&physical, bytes, true)?;
            }
            Err(e) => return Err(e.to_string()),
        }
        Ok(self.retained_input(&path, bytes, "bytes"))
    }
    /// Follow only the reference-bearing fields of the lifecycle contract.
    /// Historical inline receipts keep their original representation.
    fn collect_inputs(report: &Value, inputs: &mut Vec<RetainedInput>) -> Result<(), String> {
        let mut reports = vec![report];
        while let Some(report) = reports.pop() {
            for pointer in [
                "/terminal_input",
                "/prior_report/input",
                "/prior_cache_attempt/intent_input",
                "/prior_cache_attempt/result/input",
                "/original_report/result/input",
                "/unreferenced_original_receipt/input",
                "/original_result/input",
            ] {
                if let Some(input) = report.pointer(pointer) {
                    inputs.push(serde_json::from_value(input.clone()).map_err(|e| {
                        format!("invalid retained lifecycle input at {pointer}: {e}")
                    })?);
                }
            }
            if let Some(rows) = report.get("migration_inputs").and_then(Value::as_array) {
                for row in rows {
                    if let Some(input) = row.get("input") {
                        inputs.push(
                            serde_json::from_value(input.clone())
                                .map_err(|e| format!("invalid retained migration input: {e}"))?,
                        );
                    }
                }
            }
            if let Some(drain) = report.get("drain").and_then(Value::as_array) {
                reports.extend(drain.iter().filter_map(|entry| entry.get("report")));
            }
        }
        Ok(())
    }
    fn verify_inputs(&self, report: &Value) -> Result<(), String> {
        let mut inputs = Vec::new();
        let mut checked = BTreeSet::new();
        Self::collect_inputs(report, &mut inputs)?;
        while let Some(input) = inputs.pop() {
            if input.schema != "chrono-worktree-retained-input/v1"
                || input.source_root != self.policy.coordinator_root
                || !matches!(input.format.as_str(), "bytes" | "receipt")
            {
                return Err("invalid retained lifecycle input binding".into());
            }
            if !checked.insert((
                input.path.clone(),
                input.sha256.clone(),
                input.byte_length,
                input.format.clone(),
            )) {
                continue;
            }
            let bytes = crate::recovery::state_bytes(&input.source_root, &input.path)?;
            if bytes.len() != input.byte_length || sha256(&bytes) != input.sha256 {
                return Err("original lifecycle receipt changed: retained input mismatch".into());
            }
            if input.format == "receipt" {
                Self::collect_inputs(&json(&bytes)?, &mut inputs)?;
            }
        }
        Ok(())
    }
    fn lease(&self, e: &Entry, exclusive: bool) -> Result<Option<Lease>, String> {
        let owner = e
            .ownership
            .as_ref()
            .ok_or("enrollment has no kernel ownership; preserve legacy state")?;
        if self._gate.lease.is_none() || !owner.path.starts_with(&self.policy.state_directory) {
            return Err("enrollment lease policy mismatch".into());
        }
        let path = no_symlink_parents(&self.policy.coordinator_root, &owner.path)?;
        Lease::acquire(&path, Some(&owner.id), false, exclusive, None)
    }
    fn reconcile_uses(&mut self, i: usize, lease: &Lease) -> Result<(), String> {
        lease.stable()?;
        let e = self.ledger.entries[i].clone();
        for token in &e.uses {
            relative_path(token)?;
            let intent_path = format!("{}use-intent-{token}.json", self.policy.state_directory);
            let intent_bytes =
                crate::recovery::state_bytes(&self.policy.coordinator_root, &intent_path)?;
            let intent = json(&intent_bytes)?;
            if intent["path"] != value!(e.path)
                || intent["attachment"] != value!(e.attachment)
                || intent["lease_path"]
                    != value!(e.ownership.as_ref().ok_or("missing use ownership")?.path)
                || intent["lease_id"] != value!(e.ownership.as_ref().unwrap().id)
            {
                return Err("interrupted use intent binding changed; preserve record".into());
            }
            let path = format!("{}use-{token}.json", self.policy.state_directory);
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
        let physical = no_symlink_parents(&self.policy.coordinator_root, path)?;
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
        let names: Vec<_> = self
            .policy
            .artifacts
            .iter()
            .filter(|a| a.disposition == Disposition::Dispose)
            .map(|a| a.path.clone())
            .collect();
        let host_config = r.config.host_config.clone();
        // check_entry just verified the live attachment and common repository.
        // Equal immutable commits use the already observed coordinator registry;
        // different commits acquire their own snapshot. No live check is reused.
        let target_snapshot =
            if head == self.anchor_head && host_config == self.registrations.entry_path() {
                None
            } else {
                Some(start::registrations(r, &e.path, &head, &host_config)?)
            };
        let (target_regs, target_digest) = target_snapshot
            .as_ref()
            .map_or((&self.registrations, &self.registry_digest), |(r, d)| {
                (r, d)
            });
        artifact_disposal::registered(
            &names,
            &[self.registrations.config(), target_regs.config()],
        )?;
        let paths = artifact_disposal::paths(r, &e.path, &head, &names)?;
        let binding = value!({"path":e.path,"branch":e.branch,"attachment":e.attachment,"lease_path":owner.path,"lease_id":owner.id,
            "head":head,"policy_sha256":self.policy_binding(&e)?.policy_sha256,"config_sha256":sha256(&self.config_bytes),
            "anchor_registry_digest":chrono_harness::wire::digest(&value!(self.registrations.filemap()))?,
            "target_registry_digest":target_digest,"artifacts":names,"generation":owner.generation});
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
        });
        self.save()?;
        report["cache_intent"] = value!(intent);
        report["cache_generation"] = value!(owner.generation);
        report["destination"] = value!(e.path);
        report["head"] = value!(head);
        report["worktree_removal"] = value!("preserved");
        report["branch_removal"] = value!("not-requested");
        report["preserved_reason"] =
            value!("unfinished resumable enrollment; only registered disposable outputs selected");
        report["artifact_disposals"] = value!([]);
        artifact_disposal::dispose(&e.path, &names, &paths, report, || {
            self.check_entry(r, &e, &head, None)
        })?;
        self.check_entry(r, &e, &head, None)
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
        if !matches!(e.status.as_str(), "active" | "retained") || !e.uses.is_empty() {
            return Err(
                "worktree is terminal or has active/unknown managed use; preserve it".into(),
            );
        }
        if dispose_evidence && !self.policy.allow_evidence_disposal {
            return Err("policy does not authorize terminal evidence disposal".into());
        }
        self.birth_ready(&e)?;
        let head = r.oid(&target, "HEAD")?;
        self.check_entry(r, &e, &head, None)?;
        let retained_commit = r.oid(&self.policy.coordinator_root, &self.policy.retained_ref)?;
        if let Some(pin) = pin {
            facts::full_oid(pin)?;
            if pin != retained_commit {
                return Err("verified landing pin differs from retained reference".into());
            }
        }
        let receipt=self.immutable(&format!("terminal-{token}.json"),&value!({
            "schema":"chrono-worktree-terminal/v1","path":target,"branch":e.branch,"head":head,"attachment":e.attachment,
            "policy_sha256":self.policy_binding(&e)?.policy_sha256,"retained_ref":self.policy.retained_ref,"retained_commit":retained_commit,
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
        self.save()?;
        report["terminal_receipt"] = value!(receipt);
        Ok(())
    }
    fn drain(&mut self, exclude: &[PathBuf], report: &mut Value) -> Result<(), String> {
        report["drain"] = value!([]);
        report["cleanup_failures"] = value!([]);
        let mut failures = vec![];
        for i in 0..self.ledger.entries.len() {
            let e = self.ledger.entries[i].clone();
            let cache =
                e.status == "active" && e.ownership.as_ref().is_some_and(|o| o.cache_pending);
            if e.status != "terminal" && !cache {
                continue;
            }
            if self._gate.lease.is_some() && e.ownership.is_none() {
                report["drain"].as_array_mut().unwrap().push(value!({"path":e.path,"status":"preserved","preserved_reason":"legacy/unknown enrollment has no kernel ownership"}));
                continue;
            }
            if exclude.contains(&e.path) {
                report["drain"].as_array_mut().unwrap().push(value!({"path":e.path,"status":"preserved","preserved_reason":"invoking source or destination"}));
                continue;
            }
            let _lease = if e.ownership.is_some() {
                match self.lease(&e, true) {
                    Ok(Some(lease)) => Some(lease),
                    Ok(None) => {
                        report["drain"].as_array_mut().unwrap().push(value!({"path":e.path,"status":"preserved","preserved_reason":"live registered consumers hold enrollment lease"}));
                        continue;
                    }
                    Err(error) => {
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
                    failures.push(format!("{}: {error}", e.path.display()));
                    report["drain"]
                        .as_array_mut()
                        .unwrap()
                        .push(value!({"path":e.path,"status":"failed","error":error}));
                    continue;
                }
            }
            if !self.ledger.entries[i].uses.is_empty() {
                failures.push(format!("{}: active/unknown managed use", e.path.display()));
                report["drain"].as_array_mut().unwrap().push(
                    value!({"path":e.path,"status":"failed","error":"active/unknown managed use"}),
                );
                continue;
            }
            let (config, bytes) =
                crate::configuration(&self.policy.coordinator_root, &self.config_path)?;
            let root = self.policy.coordinator_root.clone();
            let config_path = self.config_path.clone();
            let outcome = start::with_report(
                &root,
                &config_path,
                config,
                &bytes,
                "cleanup",
                "cleaned",
                |r, token, attempt| {
                    attempt["source_commit"] = value!(self.anchor_head);
                    attempt["registry_digest"] = value!(chrono_harness::wire::digest(&value!(
                        self.registrations.filemap()
                    ))?);
                    attempt["automatic_cleanup"] = value!(true);
                    if let Some(lease) = &_lease {
                        self.reconcile_birth(r, i, lease)?;
                    }
                    if cache {
                        self.execute_cache(r, i, token, attempt)
                    } else {
                        self.execute_entry(r, i, token, attempt)
                    }
                },
            );
            match outcome {
                Ok(result) => {
                    let path = result["report_path"]
                        .as_str()
                        .ok_or("cleanup report path")?
                        .to_owned();
                    let b = crate::recovery::state_bytes(&root, &path)?;
                    if cache {
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
                                self.ledger.entries[i]
                                    .ownership
                                    .as_mut()
                                    .unwrap()
                                    .cache_pending = false;
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
                    if !cache
                        && result["status"] == "cleaned"
                        && matches!(
                            result["worktree_removal"].as_str(),
                            Some("verified-absent" | "already-absent")
                        )
                    {
                        self.ledger.entries[i].status = "disposed".into();
                    } else if !cache
                        && result["status"] == "cleaned"
                        && result["worktree_removal"] == "preserved"
                    {
                        self.ledger.entries[i].status = "retained".into();
                    }
                    if result["status"] != "cleaned" {
                        failures.push(format!("{}: {}", e.path.display(), result["error"]));
                    }
                    self.save()?;
                    report["drain"].as_array_mut().unwrap().push(value!({"path":e.path,"receipt":{"path":path,"sha256":sha256(&b)},"report":result}));
                }
                Err(error) => {
                    failures.push(format!("{}: {error}", e.path.display()));
                    report["drain"]
                        .as_array_mut()
                        .unwrap()
                        .push(value!({"path":e.path,"status":"failed","error":error}));
                }
            }
        }
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
    /// Entry-local refusals remain in the drain report and immutable receipts.
    /// Coordinator/state publication failures still prevent admission.
    fn drain_for_admission(
        &mut self,
        exclude: &[PathBuf],
        report: &mut Value,
    ) -> Result<(), String> {
        match self.drain(exclude, report) {
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
        let e = self.ledger.entries[i].clone();
        let t = e
            .terminal
            .as_ref()
            .ok_or("missing terminal handoff")?
            .clone();
        report["terminal_receipt"] = value!(t.receipt);
        report["terminal_input"] =
            self.retained_input(&t.receipt.path, &self.receipt(&t.receipt)?, "receipt");
        if self.policy_binding(&e)?.policy_sha256 != sha256(&self.policy_bytes) {
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
                    .inventory(&self.policy.coordinator_root)?
                    .into_iter()
                    .find(|row| row.get("worktree").is_some_and(|p| Path::new(p) == e.path))
                    .and_then(|row| row.get("locked").cloned());
                if locked.as_deref() == Some(prior.token.as_str()) {
                    self.check_entry(r, &e, &t.head, Some(&prior.token))?;
                    let tree = r.oid(&e.path, &format!("{}^{{tree}}", t.head))?;
                    maintenance::release_reconciled(
                        r,
                        &self.policy.coordinator_root,
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
        self.git_locks(&e)?;
        self.pinned_retention(r, &t)?;
        let names: Vec<_> = self
            .policy
            .artifacts
            .iter()
            .filter(|a| {
                a.disposition == Disposition::Dispose
                    || (a.disposition == Disposition::EvidenceRetain && t.dispose_evidence)
            })
            .map(|a| a.path.clone())
            .collect();
        let mut preserved = None;
        if t.artifacts_only {
            preserved = Some("terminal owner requested artifacts only".to_owned());
        } else if self
            .policy
            .artifacts
            .iter()
            .any(|a| a.disposition == Disposition::EvidenceRetain)
            && !t.dispose_evidence
        {
            preserved =
                Some("needed evidence remains retained by policy and terminal owner".to_owned());
        }
        if e.path.exists() {
            self.check_entry(r, &e, &t.head, None)?;
            if preserved.is_none() {
                if let Err(error) = maintenance::saved_commit(
                    r,
                    &self.policy.coordinator_root,
                    &t.head,
                    &self.policy.retained_ref,
                    &t.retained_commit,
                    &self.policy.retention,
                ) {
                    preserved = Some(format!("commit retention unresolved: {error}"));
                }
                let host_config = r.config.host_config.clone();
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
        if let Some(reason) = preserved {
            report["preserved_reason"] = value!(reason);
            report["worktree_removal"] = value!("preserved");
            report["branch_removal"] = value!("not-requested");
            report["artifact_disposals"] = value!([]);
            self.check_entry(r, &e, &t.head, None)?;
            let host_config = r.config.host_config.clone();
            let (target_regs, _) = start::registrations(r, &e.path, &t.head, &host_config)?;
            artifact_disposal::registered(
                &names,
                &[self.registrations.config(), target_regs.config()],
            )?;
            artifact_disposal::paths(r, &e.path, &t.head, &names)?;
            report["destination"] = value!(e.path);
            report["branch_ref"] = value!(e.branch);
            report["head"] = value!(t.head);
            report["base"] = value!(t.head);
            crate::recovery::publish(&self.policy.coordinator_root, report)?;
            let path = e.path.to_str().ok_or("path UTF-8")?;
            r.git(
                &self.policy.coordinator_root,
                &["worktree", "lock", "--reason", token, "--", path],
            )?;
            self.check_entry(r, &e, &t.head, Some(token))?;
            let paths = artifact_disposal::paths(r, &e.path, &t.head, &names)?;
            artifact_disposal::dispose(&e.path, &names, &paths, report, || {
                self.check_entry(r, &e, &t.head, Some(token))?;
                self.pinned_retention(r, &t)
            })?;
            self.check_entry(r, &e, &t.head, Some(token))?;
            self.pinned_retention(r, &t)?;
            start::unlock(
                r,
                &self.policy.coordinator_root,
                path,
                &format!("refs/heads/{}", e.branch),
                &t.head,
            )?;
            self.check_entry(r, &e, &t.head, None)?;
            self.pinned_retention(r, &t)
        } else {
            Cleanup {
                path: e.path.clone(),
                branch: e.branch.clone(),
                head: t.head.clone(),
                retained_ref: self.policy.retained_ref.clone(),
                retained_commit: t.retained_commit.clone(),
                retention: self.policy.retention.clone(),
                discard_artifacts: names,
                remove_branch: self.policy.remove_branch,
                allow_absent_worktree: !e.attempts.is_empty(),
            }
            .execute_checked(
                r,
                &self.policy.coordinator_root,
                &self.registrations,
                token,
                report,
                |r| {
                    self.stable(r)?;
                    self.git_locks(&e)?;
                    self.pinned_retention(r, &t)?;
                    if fs::symlink_metadata(&e.path).is_ok() {
                        let row = r
                            .inventory(&self.policy.coordinator_root)?
                            .into_iter()
                            .find(|row| row.get("worktree").is_some_and(|p| Path::new(p) == e.path))
                            .ok_or("enrolled checkout inventory disappeared")?;
                        let lock = row.get("locked").map(String::as_str);
                        if lock.is_some() && lock != Some(token) {
                            return Err("another worktree lock owns pending disposal".into());
                        }
                        self.check_entry(r, &e, &t.head, lock)?;
                    }
                    Ok(())
                },
            )
        }
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
    let Some(mut manager) = Manager::open(r, &o.root, &o.config_path, bytes, token)? else {
        return create(r, report);
    };
    let destination = absolute(
        &fs::canonicalize(o.destination.parent().ok_or("destination parent")?)
            .map_err(|e| e.to_string())?
            .join(o.destination.file_name().ok_or("destination name")?),
        true,
    )?;
    manager.preflight_birth(r, &destination)?;
    manager.drain_for_admission(&[o.root.clone(), destination.clone()], report)?;
    manager.stable(r)?;
    manager.preflight_birth(r, &destination)?;
    create(r, report)?;
    manager.stable(r)?;
    manager.enroll(r,Path::new(report["destination"].as_str().ok_or("birth destination")?),"birth",value!({"source_root":o.root,"report_path":report["report_path"],"operation":report["operation"],"base":report["base"],"token":report["lock_reason"]}))?;
    report["cleanup_enrollment"] = value!({"coordinator_root":manager.policy.coordinator_root,"state_directory":manager.policy.state_directory,"kind":"birth"});
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
    let Some(mut manager) = Manager::open(r, root, config_path, bytes, token)? else {
        return Ok(());
    };
    let target = Path::new(report["destination"].as_str().ok_or("birth destination")?);
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
    if (values.contains_key("--bootstrap-config") && operation != "bootstrap")
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
    {
        return Err("invalid lifecycle command arguments".into());
    }
    let invoking = fs::canonicalize(values.get("--host-root").copied().unwrap_or("."))
        .map_err(|e| e.to_string())?;
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
    let target = if operation == "maintain" {
        None
    } else {
        Some(absolute(&target, false)?)
    };
    let success = match operation.as_str() {
        "finish" => "finished",
        "maintain" => "maintained",
        "import" => "imported",
        "migrate" => "migrated",
        "use" | "check" | "bootstrap" => "used",
        _ => return Err("unknown lifecycle operation".into()),
    };
    let mut use_guard = None;
    start::with_report(
        &invoking,
        config_path,
        config,
        &bytes,
        operation,
        success,
        |r, token, report| {
            report["invoking_root"] = value!(invoking);
            let mut manager = Manager::open(r, &invoking, config_path, &bytes, token)?
                .ok_or("automatic cleanup is not opted in")?;
            report["source_root"] = value!(manager.policy.coordinator_root);
            report["lifecycle_coordinator_root"] = value!(manager.policy.coordinator_root);
            report["source_commit"] = value!(manager.anchor_head);
            match operation.as_str() {
                "maintain" => manager.drain(&[invoking.clone()], report),
                "import" => manager.enroll(
                    r,
                    target.as_ref().unwrap(),
                    "import",
                    value!({"report_path":report["report_path"],"historical_birth":"not-claimed"}),
                ),
                "migrate" => manager.migrate(r, target.as_ref().unwrap(), report),
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
                    manager.drain(&[invoking.clone()], report)
                }
                "use" | "check" | "bootstrap" => {
                    let target = target.as_ref().unwrap();
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
                            && target == &manager.policy.coordinator_root =>
                        {
                            manager.drain_for_admission(&[invoking.clone()], report)?;
                            let command = if operation == "bootstrap" {
                                bootstrap_command(
                                    r,
                                    target,
                                    &manager.registrations,
                                    values["--bootstrap-config"],
                                )?
                            } else {
                                canonical_command(target, &manager.registrations, &selection)?
                            };
                            drop(manager);
                            let digest =
                                chrono_harness::file_identity(Path::new(&command.program))?.0;
                            let process = chrono_harness::run_process_observed(
                                target,
                                &command,
                                &[],
                                &digest,
                            )?;
                            report["managed_process"] = value!(process);
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
                    let host_config = r.config.host_config.clone();
                    let target_snapshot = if head == manager.anchor_head
                        && host_config == manager.registrations.entry_path()
                    {
                        None
                    } else {
                        Some(start::registrations(r, target, &head, &host_config)?)
                    };
                    let target_regs = target_snapshot
                        .as_ref()
                        .map_or(&manager.registrations, |(registrations, _)| registrations);
                    let command = if operation == "check" {
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
                    if e.ownership.is_some() {
                        use_guard = Some(
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
                    manager.drain_for_admission(&[invoking.clone(), target.clone()], report)?;
                    let coordinator = manager.policy.coordinator_root.clone();
                    drop(manager);
                    let digest = chrono_harness::file_identity(Path::new(&command.program))?.0;
                    let process =
                        chrono_harness::run_process_observed(target, &command, &[], &digest)?;
                    report["managed_process"] = value!(process);
                    let mut manager = Manager::open(r, &coordinator, config_path, &bytes, token)?
                        .ok_or("cleanup policy disappeared during managed use")?;
                    let receipt=manager.immutable(&format!("use-{token}.json"),&value!({"schema":"chrono-worktree-managed-use-result/v1","path":target,"operation":use_operation,"process":process}))?;
                    report["managed_use_receipt"] = value!(receipt);
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
                    manager.ledger.entries[i].uses.retain(|s| s != token);
                    manager.save()?;
                    if process.failure.is_some() || process.exit_code != 0 {
                        report["managed_command_failed"] = value!(true);
                        return Err(format!(
                            "registered managed command failed with exit {}: {:?}",
                            process.exit_code, process.failure
                        ));
                    }
                    Ok(())
                }
                _ => unreachable!(),
            }
        },
    )
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

fn bootstrap_command(
    r: &mut Runner,
    root: &Path,
    registrations: &Registrations,
    path: &str,
) -> Result<CommandSpec, String> {
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

// Lifecycle wrapper publication continues only at the validated surviving owner,
// after managed use of the disposable checkout has been released.
pub(crate) fn publish_result(root: &Path, report: &Value) -> Result<(), String> {
    let root = absolute(root, false)?;
    let path = report["report_path"]
        .as_str()
        .ok_or("lifecycle report path")?;
    relative_path(path)?;
    if !path.starts_with(".chrono-harness/state/") {
        return Err("lifecycle report must remain registered state".into());
    }
    let target = no_symlink_parents(&root, path)?;
    fs::create_dir_all(target.parent().ok_or("lifecycle report parent")?)
        .map_err(|e| e.to_string())?;
    write_json(&target, report, true)?;
    Ok(())
}
