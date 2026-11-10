//! Read existing enrollment state without acquiring metadata mutation capabilities.
use super::*;

struct Observation<'a> {
    inputs: &'a PolicyInputs,
    directory: &'a Path,
    scope: &'a Path,
}

pub(super) fn status(
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
    pub(super) fn observe_inventory(
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
    pub(super) fn enrollment_paths(&self) -> Result<Vec<PathBuf>, String> {
        self.observation().enrollment_paths()
    }
    pub(super) fn observe_attention(
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
