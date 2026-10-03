//! Opt-in lifecycle cleanup. Eligibility is an explicit terminal handoff, never age or discovery.
use crate::{
    Start, artifact_disposal,
    maintenance::{self, Cleanup, Retention},
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
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Ledger {
    schema: String,
    coordinator_root: PathBuf,
    common: PathBuf,
    entries: Vec<Entry>,
}
struct Gate {
    path: PathBuf,
    token: String,
}
impl Drop for Gate {
    fn drop(&mut self) {
        if fs::read(self.path.join("owner")).ok().as_deref() == Some(self.token.as_bytes()) {
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
    absolute(target, false)?;
    let gitfile = target.join(".git");
    if !fs::symlink_metadata(&gitfile)
        .map_err(|e| e.to_string())?
        .is_file()
    {
        return Err("only an existing linked worktree can enroll".into());
    }
    let metadata = absolute(
        Path::new(r.text(target, &["rev-parse", "--absolute-git-dir"])?.trim()),
        false,
    )?;
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
    let mut out = tempfile::Builder::new()
        .prefix(".publish-")
        .tempfile_in(path.parent().ok_or("publication parent")?)
        .map_err(|e| e.to_string())?;
    out.write_all(&bytes).map_err(|e| e.to_string())?;
    out.as_file().sync_all().map_err(|e| e.to_string())?;
    if immutable {
        out.persist_noclobber(path).map_err(|e| e.to_string())?;
    } else {
        out.persist(path).map_err(|e| e.to_string())?;
    }
    Ok(bytes)
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
    if p.schema != "chrono-worktree-automatic-cleanup/v1"
        || !p.state_directory.starts_with(".chrono-harness/state/")
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
        if absolute(
            Path::new(r.text(anchor, &["rev-parse", "--show-toplevel"])?.trim()),
            false,
        )? != *anchor
        {
            return Err("coordinator is not a Git checkout root".into());
        }
        let common = absolute(
            &anchor.join(r.text(anchor, &["rev-parse", "--git-common-dir"])?.trim()),
            false,
        )?;
        let root_common =
            fs::canonicalize(root.join(r.text(root, &["rev-parse", "--git-common-dir"])?.trim()))
                .map_err(|e| e.to_string())?;
        if common != root_common {
            return Err("coordinator belongs to a different repository".into());
        }
        let anchor_head = r.oid(anchor, "HEAD")?;
        if fs::read(no_symlink_parents(anchor, config_path)?).map_err(|e| e.to_string())? != bytes
            || r.blob(anchor, &anchor_head, config_path)? != bytes
            || fs::read(no_symlink_parents(anchor, &policy_path)?).map_err(|e| e.to_string())?
                != policy_bytes
            || r.blob(anchor, &anchor_head, &policy_path)? != policy_bytes
        {
            return Err("coordinator policy/configuration identity mismatch".into());
        }
        let host_config = r.config.host_config.clone();
        let (registrations, _) = start::registrations(r, anchor, &anchor_head, &host_config)?;
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
        let gate_path =
            no_symlink_parents(anchor, &format!("{}gate.lock", policy.state_directory))?;
        let waiting = std::time::Instant::now();
        loop {
            match fs::create_dir(&gate_path) {
                Ok(()) => break,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    // The owner file is published/removed inside the atomic directory
                    // lease. Its transient absence must not shorten a live handoff.
                    let bound = std::time::Duration::from_secs(r.config.timeout_seconds);
                    if waiting.elapsed() >= bound {
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
        };
        fs::write(gate.path.join("owner"), token).map_err(|e| e.to_string())?;
        let ledger_path =
            no_symlink_parents(anchor, &format!("{}ledger.json", policy.state_directory))?;
        let marker = no_symlink_parents(anchor, &format!("{}owner.json", policy.state_directory))?;
        let ledger: Ledger = match fs::read(&ledger_path) {
            Ok(b) => decode(&b)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && !marker.exists() => {
                let ledger = Ledger {
                    schema: "chrono-worktree-cleanup-state/v1".into(),
                    coordinator_root: anchor.clone(),
                    common: common.clone(),
                    entries: vec![],
                };
                write_json(
                    &marker,
                    &value!({"schema":"chrono-worktree-cleanup-owner/v1","coordinator_root":anchor,"common":common}),
                    true,
                )?;
                write_json(&ledger_path, &ledger, true)?;
                ledger
            }
            Err(e) => {
                return Err(format!(
                    "missing or unreadable cleanup ledger; preserve enrolled work: {e}"
                ));
            }
        };
        let owner = json(&fs::read(&marker).map_err(|e| e.to_string())?)?;
        if ledger.schema != "chrono-worktree-cleanup-state/v1"
            || ledger.coordinator_root != *anchor
            || ledger.common != common
            || owner
                != value!({"schema":"chrono-worktree-cleanup-owner/v1","coordinator_root":anchor,"common":common})
        {
            return Err("cleanup ledger/owner identity mismatch".into());
        }
        let mut paths = BTreeSet::new();
        for e in &ledger.entries {
            if !paths.insert(&e.path)
                || !matches!(
                    e.status.as_str(),
                    "active" | "terminal" | "disposed" | "retained"
                )
                || (e.status == "active") != e.terminal.is_none()
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
        let tree = facts::parse_tree(&r.git(target, &["ls-tree", "-rz", "--full-tree", head])?)?;
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
        if e.policy_sha256 != sha256(&self.policy_bytes) {
            return Err("enrolled policy changed; explicit migration is required".into());
        }
        absolute(&e.path, false)?;
        maintenance::identity(
            r,
            &self.policy.coordinator_root,
            &e.path,
            &e.branch,
            head,
            lock,
        )?;
        let inputs: BTreeMap<String, Option<String>> =
            serde_json::from_value(e.enrollment["policy_inputs"].clone())
                .map_err(|_| "enrollment lacks original target policy input bindings")?;
        if self.target_policy_inputs(r, &e.path, head)? != inputs {
            return Err("enrolled target policy/configuration changed; preserve work for explicit migration".into());
        }
        if attachment(r, &e.path)? != e.attachment {
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
    fn enroll(
        &mut self,
        r: &mut Runner,
        target: &Path,
        kind: &str,
        receipt: Value,
    ) -> Result<(), String> {
        let target = absolute(target, false)?;
        if self.ledger.entries.iter().any(|e| e.path == target) {
            return Err("path is already enrolled; preserve its original state".into());
        }
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
        };
        self.check_entry(r, &entry, &head, None)?;
        let host_config = r.config.host_config.clone();
        let (target_registrations, _) = start::registrations(r, &entry.path, &head, &host_config)?;
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
        self.save()
    }
    fn birth_ready(&self, e: &Entry) -> Result<(), String> {
        if e.enrollment["kind"] == "birth" {
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
    fn receipt(&self, receipt: &Receipt) -> Result<Vec<u8>, String> {
        let b = crate::recovery::state_bytes(&self.policy.coordinator_root, &receipt.path)?;
        if sha256(&b) != receipt.sha256 {
            return Err("original lifecycle receipt changed".into());
        }
        Ok(b)
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
            .ledger
            .entries
            .iter()
            .position(|e| e.path == target)
            .ok_or("worktree is not enrolled; use exact import for existing work")?;
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
            "policy_sha256":e.policy_sha256,"retained_ref":self.policy.retained_ref,"retained_commit":retained_commit,
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
        let mut failures = vec![];
        for i in 0..self.ledger.entries.len() {
            let e = self.ledger.entries[i].clone();
            if e.status != "terminal" {
                continue;
            }
            if exclude.contains(&e.path) {
                report["drain"].as_array_mut().unwrap().push(value!({"path":e.path,"status":"preserved","preserved_reason":"invoking source or destination"}));
                continue;
            }
            if !e.uses.is_empty() {
                failures.push(format!("{}: active/unknown managed use", e.path.display()));
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
                    self.execute_entry(r, i, token, attempt)
                },
            );
            match outcome {
                Ok(result) => {
                    let path = result["report_path"]
                        .as_str()
                        .ok_or("cleanup report path")?
                        .to_owned();
                    let b = crate::recovery::state_bytes(&root, &path)?;
                    if let Some(a) = self.ledger.entries[i].attempts.last_mut() {
                        if a.path == path {
                            a.receipt = Some(Receipt {
                                path: path.clone(),
                                sha256: sha256(&b),
                            });
                        }
                    }
                    if result["status"] == "cleaned"
                        && matches!(
                            result["worktree_removal"].as_str(),
                            Some("verified-absent" | "already-absent")
                        )
                    {
                        self.ledger.entries[i].status = "disposed".into();
                    } else if result["status"] == "cleaned"
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
        if failures.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "automatic cleanup pending failures: {}",
                failures.join("; ")
            ))
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
        report["terminal_input_bytes"] = value!(self.receipt(&t.receipt)?);
        if e.policy_sha256 != sha256(&self.policy_bytes) {
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
            report["prior_report"] =
                value!({"receipt":receipt,"input_bytes":b,"report":prior_report});
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
    manager.drain(&[o.root.clone(), destination], report)?;
    manager.stable(r)?;
    create(r, report)?;
    manager.stable(r)?;
    manager.enroll(r,Path::new(report["destination"].as_str().ok_or("birth destination")?),"birth",value!({"source_root":o.root,"report_path":report["report_path"],"operation":report["operation"],"base":report["base"]}))?;
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
        .ledger
        .entries
        .iter()
        .position(|e| e.path == target)
        .ok_or("birth enrollment missing")?;
    if manager.ledger.entries[i].status != "active"
        || manager.ledger.entries[i].enrollment["kind"] != "birth"
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
    if (values.contains_key("--operation") && operation != "use")
        || (operation == "use" && !values.contains_key("--operation"))
        || (values.contains_key("--retained-commit") && operation != "finish")
        || (operation == "maintain" && values.contains_key("--path"))
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
        "use" => "used",
        _ => return Err("unknown lifecycle operation".into()),
    };
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
                "use" => {
                    let target = target.as_ref().unwrap();
                    let i = manager
                        .ledger
                        .entries
                        .iter()
                        .position(|e| &e.path == target)
                        .ok_or("managed use requires an enrolled worktree")?;
                    let e = manager.ledger.entries[i].clone();
                    if e.status != "active" {
                        return Err("terminal worktree refuses new managed use".into());
                    }
                    manager.birth_ready(&e)?;
                    let head = r.oid(target, "HEAD")?;
                    manager.check_entry(r, &e, &head, None)?;
                    let host_config = r.config.host_config.clone();
                    let (target_regs, _) = start::registrations(r, target, &head, &host_config)?;
                    let command = registered_command(target, &target_regs, values["--operation"])?;
                    manager.ledger.entries[i].uses.push(token.into());
                    manager.save()?;
                    let coordinator = manager.policy.coordinator_root.clone();
                    drop(manager);
                    let digest = chrono_harness::file_identity(Path::new(&command.program))?.0;
                    let process =
                        chrono_harness::run_process_observed(target, &command, &[], &digest)?;
                    report["managed_process"] = value!(process);
                    let mut manager = Manager::open(r, &coordinator, config_path, &bytes, token)?
                        .ok_or("cleanup policy disappeared during managed use")?;
                    let receipt=manager.immutable(&format!("use-{token}.json"),&value!({"schema":"chrono-worktree-managed-use-result/v1","path":target,"operation":values["--operation"],"process":process}))?;
                    report["managed_use_receipt"] = value!(receipt);
                    let i = manager
                        .ledger
                        .entries
                        .iter()
                        .position(|e| &e.path == target)
                        .ok_or("managed enrollment disappeared")?;
                    if !manager.ledger.entries[i].uses.iter().any(|s| s == token) {
                        return Err("managed use identity disappeared".into());
                    }
                    manager.ledger.entries[i].uses.retain(|s| s != token);
                    manager.save()?;
                    if process.failure.is_some() || process.exit_code != 0 {
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
    let a = selected[0];
    let config = registrations.config();
    let tool = config["tools"]
        .as_array()
        .ok_or("missing registered tools")?
        .iter()
        .find(|t| t["id"] == a["tool"])
        .ok_or("consuming tool is not registered")?;
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
        tool["program"].as_str().ok_or("registered program")?,
        environment.get("PATH").map(String::as_str),
    )?;
    let command = CommandSpec {
        program: program.to_str().ok_or("command path UTF-8")?.into(),
        args: serde_json::from_value(a["argv"].clone()).map_err(|e| e.to_string())?,
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
