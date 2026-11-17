//! One explicit maintain's diagnostic original, never a membership or deletion authority.
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
/// Preserve each matching action's original index, including repeated addresses.
pub fn append_action_index(mut indices: Vec<usize>, matches: bool, index: usize)
    -> (out: Vec<usize>)
    ensures out@ == if matches { indices@.push(index) } else { indices@ },
{
    if matches { indices.push(index); }
    indices
}
}

#[cfg(not(verus_keep_ghost))]
use super::*;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::retained_artifacts::retention::{self, Publication};
#[cfg(not(verus_keep_ghost))]
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[cfg(not(verus_keep_ghost))]
fn timestamp() -> Result<u64, String> {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis(),
    )
    .map_err(|e| e.to_string())
}

/// Construct the output used by the complete maintenance inventory.
#[doc(hidden)]
#[cfg(not(verus_keep_ghost))]
pub fn inventory_writer<W: Write>(output: W) -> std::io::BufWriter<W> {
    std::io::BufWriter::with_capacity(64 * 1024, output)
}

/// Flush before syncing, and never let Drop retry a failed buffered write.
#[doc(hidden)]
#[cfg(not(verus_keep_ghost))]
pub fn finish_inventory_writer<W: Write>(
    mut output: std::io::BufWriter<W>,
    sync: impl FnOnce(&mut W) -> Result<(), String>,
) -> Result<(), String> {
    let flushed = output.flush().map_err(|e| format!("inventory flush: {e}"));
    let (mut output, _unwritten) = output.into_parts();
    flushed?;
    sync(&mut output).map_err(|e| format!("inventory sync: {e}"))
}

/// The actual report write boundary; callers retain partial bytes on IO failure.
#[doc(hidden)]
#[cfg(not(verus_keep_ghost))]
pub fn write_record(
    output: &mut impl Write,
    row: &Value,
    bytes_written: &mut u64,
    rows_written: &mut u64,
    within_limit: impl FnOnce(u64) -> Result<(), String>,
) -> Result<(), String> {
    let mut bytes = serde_json::to_vec(row).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    let total = bytes_written
        .checked_add(bytes.len() as u64)
        .ok_or("inventory byte overflow")?;
    within_limit(total)?;
    output.write_all(&bytes).map_err(|e| e.to_string())?;
    *bytes_written = total;
    *rows_written += 1;
    Ok(())
}

/// Keep every observation, sharing only identical declaration bodies within
/// this original. References are one-based JSONL line numbers, never authority.
#[doc(hidden)]
#[cfg(not(verus_keep_ghost))]
pub fn compact_retention_record(
    row: &mut Value,
    first_records: &mut BTreeMap<String, u64>,
    line: u64,
) -> Result<(), String> {
    if row["kind"] != "declaration-state" || !row["record"].is_object() {
        return Ok(());
    }
    let digest = sha256(&serde_json::to_vec(&row["record"]).map_err(|e| e.to_string())?);
    if let Some(first) = first_records.get(&digest) {
        row.as_object_mut().unwrap().remove("record");
        row["record_ref"] = value!(first);
    } else {
        first_records.insert(digest, line);
    }
    Ok(())
}

#[cfg(not(verus_keep_ghost))]
pub(super) struct Report {
    root: PathBuf,
    path: String,
    output: std::io::BufWriter<fs::File>,
    publication: Option<Publication>,
    bytes: u64,
    rows: u64,
    began: Instant,
    started: u64,
    enumeration_complete: bool,
    measurements_complete: bool,
    errors: Vec<String>,
    io_error: Option<String>,
    artifact_addresses: BTreeSet<PathBuf>,
    worktree_addresses: BTreeSet<PathBuf>,
    retention_records: BTreeMap<String, u64>,
}

#[cfg(not(verus_keep_ghost))]
impl Report {
    pub(super) fn begin(root: &Path, parent: &mut Value) -> Result<Self, String> {
        let parent_path = parent["report_path"]
            .as_str()
            .ok_or("maintenance report path")?
            .to_owned();
        let path = format!("{parent_path}.inventory.jsonl");
        let physical = no_symlink_parents(root, &path)?;
        let publication = retention::create_adopted(
            root,
            "lifecycle-original",
            &physical,
            false,
            value!({"operation":"maintenance-inventory","status":"not-established"}),
        )?;
        parent["complete_inventory"] = value!({"schema":"chrono-maintenance-inventory/v2",
            "path":path,"format":"jsonl","status":"incomplete",
            "member_enumeration_complete":false,"measurements_complete":false,
            "publication_complete":false});
        let mut report = Self {
            root: root.into(),
            path,
            output: inventory_writer(
                fs::OpenOptions::new()
                    .write(true)
                    .open(physical)
                    .map_err(|e| e.to_string())?,
            ),
            publication,
            bytes: 0,
            rows: 0,
            began: Instant::now(),
            started: timestamp()?,
            enumeration_complete: true,
            measurements_complete: true,
            errors: vec![],
            io_error: None,
            artifact_addresses: BTreeSet::new(),
            worktree_addresses: BTreeSet::new(),
            retention_records: BTreeMap::new(),
        };
        report.record(value!({"kind":"scope","source":"maintenance-round",
            "trigger":parent["maintenance_trigger"],
            "coordinator_root":root,"parent_report":parent_path,
            "window_started_unix_millis":report.started,"atomic":false,
            "physical_release_bytes":null,"aggregation":"overlapping scopes; do not sum",
            "authority":"observation-only"}));
        Ok(report)
    }

    // Keep independent cleanup running after report IO failure. The final producer
    // result remains failed, and the partially written original stays protected.
    pub(super) fn record(&mut self, row: Value) {
        if self.io_error.is_some() {
            return;
        }
        let publication = &self.publication;
        let written = write_record(
            &mut self.output,
            &row,
            &mut self.bytes,
            &mut self.rows,
            |total| {
                if let Some(publication) = publication {
                    publication.check_length(total)?;
                }
                Ok(())
            },
        );
        if let Err(error) = written {
            self.io_error = Some(error);
        }
    }

    pub(super) fn error(&mut self, scope: &str, error: impl ToString) {
        self.enumeration_complete = false;
        self.measurements_complete = false;
        let error = error.to_string();
        self.record(
            value!({"kind":"enumeration-error","scope":scope,"error":error,
            "members":"unknown","action":"not-established"}),
        );
        self.errors.push(format!("{scope}: {error}"));
    }

    pub(super) fn unknown(&mut self, scope: &str, reason: &str) {
        self.enumeration_complete = false;
        self.measurements_complete = false;
        self.record(
            value!({"kind":"unknown-declarations","scope":scope,"reason":reason,
            "members":null,"action":"not-established"}),
        );
    }

    pub(super) fn measure(&mut self, path: &Path) -> Value {
        let started = timestamp();
        let footprint = (|| {
            // Validate parents, including a worktree address whose attachment is
            // unknown. Never follow a linked root or an external linked ancestor.
            let parent = path.parent().ok_or("inventory measurement parent")?;
            absolute(parent, true)?;
            let before = fs::symlink_metadata(path);
            let presence = match before {
                Ok(ref m) if m.file_type().is_symlink() => {
                    return Err("inventory root is a symbolic link".into());
                }
                Ok(_) => "present",
                Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => "absent",
                Err(e) => return Err(e.to_string()),
            };
            super::service::observe_boundary("footprint", path);
            let (footprint, sockets) =
                chrono_harness::artifact_disposal::observed_artifact_footprint(path)?;
            Ok::<_, String>((presence, footprint, sockets))
        })();
        let finished = timestamp();
        let mut measurement = value!({"unit":"bytes","path":path,
            "logical_method":"literal-entry-lengths",
            "allocated_method":"lstat-blocks-times-512-including-directories",
            "window_started_unix_millis":started.as_ref().ok(),
            "window_finished_unix_millis":finished.as_ref().ok(),
            "freshness":"this-round-non-atomic","physical_release_bytes":null});
        match (footprint, started, finished) {
            (Ok((presence, footprint, sockets)), Ok(_), Ok(_)) => {
                measurement["presence"] = value!(presence);
                measurement["footprint"] = value!(footprint);
                measurement["retained_socket_entries"] = value!(sockets);
            }
            (result, start, end) => {
                let error = result
                    .err()
                    .or_else(|| start.err())
                    .or_else(|| end.err())
                    .unwrap();
                self.measurements_complete = false;
                self.errors.push(format!("{}: {error}", path.display()));
                measurement["freshness"] = value!("unknown");
                measurement["error"] = value!(error);
                measurement["footprint"] = Value::Null;
            }
        }
        measurement
    }

    pub(super) fn worktrees(
        &mut self,
        observation: &Observation<'_>,
        r: &mut Runner,
        parent: &mut Value,
        phase: &str,
    ) {
        let observation_owner = observation;
        let mut observation = value!({"agent":parent["agent"]});
        if let Err(error) = observation_owner.observe_attention(r, &mut observation, true) {
            self.error("worktree-status", error);
            if observation["worktree_status"].is_null() {
                return;
            }
        }
        let status = &observation["worktree_status"];
        parent["worktree_status"] = status.clone();
        if !status["namespace_error"].is_null() {
            self.error("enrollment-namespace", &status["namespace_error"]);
        }
        for issue in status["namespace_issues"].as_array().into_iter().flatten() {
            self.error("enrollment-namespace", issue);
        }
        for state in status["worktrees"].as_array().into_iter().flatten() {
            let mut row = value!({"kind":"worktree","phase":phase,"state":state,
                "reporting_owner":"worktree-lifecycle",
                "owner":if state["git_observation"]["enrollment_match"] == true
                    || state["work_state"] == "coordinator" {"worktree-lifecycle"} else {"unknown"},
                "qualification":"see original drain; no permission inferred"});
            if let Some(path) = state["path"].as_str() {
                self.worktree_addresses.insert(path.into());
                if state["record_kind"].is_null() && phase == "before" {
                    row["measurement"] = self.measure(Path::new(path));
                }
            } else {
                self.enumeration_complete = false;
            }
            self.record(row);
        }
    }

    pub(super) fn artifacts(&mut self, snapshot: &ArtifactSnapshot) {
        let entry = snapshot.entry.as_ref();
        let root = &snapshot.root;
        self.artifact_addresses.insert(root.into());
        for artifact in &snapshot.artifacts {
            let path = root.join(artifact.path.trim_end_matches('/'));
            let measurement = self.measure(&path);
            let disposition = match &artifact.disposition {
                Disposition::Dispose => "dispose",
                Disposition::FinishDispose => "finish-dispose",
                Disposition::Retain => "retain",
                Disposition::EvidenceRetain => "evidence-retain",
            };
            self.record(value!({"kind":"worktree-artifact","worktree":root,"path":path,
                "source":{"policy_sha256":snapshot.policy_sha256,
                    "policy_path":snapshot.policy_path,"source_head":snapshot.source_head},
                "owner":"worktree-lifecycle","disposition":disposition,
                "cleanup_state":entry.map(|e| &e.status),"cleanup_progress":entry.and_then(|e| e.cleanup.as_ref()),
                "uses":entry.map(|e| &e.uses),"ownership":entry.and_then(|e| e.ownership.as_ref()),
                "terminal_source":entry.and_then(|e| e.terminal.as_ref()).map(|t| &t.receipt),
                "terminal_established":entry.and_then(|e| e.terminal.as_ref()).and_then(|t| t.established.as_ref()),
                "qualification":"not-evaluated-by-report","action":"see worktree-action",
                "measurement":measurement}));
        }
    }

    pub(super) fn retention(&mut self, source_root: &Path, mut row: Value) {
        if row["kind"] == "declaration-state" {
            if row["record"]["output"].is_object() {
                if let Some(path) = row["record"]["path"].as_str() {
                    row["measurement"] = self.measure(&source_root.join(path));
                    row["measurement"]["scope"] =
                        value!("current physical path; declaration identity is separate");
                }
            } else if let Some(path) = row["record"]["metadata_path"].as_str() {
                row["measurement"] = self.measure(Path::new(path));
                row["measurement"]["scope"] = value!("retired output declaration metadata only");
            }
            if !row["error"].is_null() {
                self.error(
                    &format!("retention-declaration:{}", source_root.display()),
                    &row["error"],
                );
            }
        }
        if row["kind"] == "enumeration-error" {
            self.error(
                &format!("retention-namespace:{}", source_root.display()),
                &row["error"],
            );
        }
        if row["kind"] == "round"
            && (!row["error"].is_null()
                || row["original"]["accounting_errors"]
                    .as_array()
                    .is_some_and(|v| !v.is_empty()))
        {
            self.enumeration_complete = false;
            self.measurements_complete = false;
            self.errors.push(format!(
                "retention {} round {}: original error retained",
                source_root.display(),
                row["address"]
            ));
        }
        if row["kind"] == "source-result" {
            for (producer, quota) in row["original"]["terminal_quotas"]
                .as_object()
                .into_iter()
                .flatten()
            {
                for error in quota["errors"].as_array().into_iter().flatten() {
                    self.error(
                        &format!("retention-quota:{}:{producer}", source_root.display()),
                        error,
                    );
                }
            }
        }
        // Measurements remain per observation: identical declarations do not
        // establish that the physical path is unchanged. Only reuse the body.
        // A failed write stops all later writes, so no reference can target a
        // first body that failed publication.
        if let Err(error) =
            compact_retention_record(&mut row, &mut self.retention_records, self.rows + 1)
        {
            self.error("retention-record", error);
            return;
        }
        self.record(value!({"kind":"retention","owner":"runner-retention",
            "source_root":source_root,"observation":row}));
    }

    pub(super) fn finish(mut self, parent: &mut Value) -> Result<(), String> {
        let worktrees = std::mem::take(&mut self.worktree_addresses);
        for path in worktrees {
            self.record(worktree_action(&path, &parent["drain"]));
        }
        for row in parent["producer_drains"].as_array().into_iter().flatten() {
            self.record(value!({"kind":"producer-result","original":row}));
        }
        // An unavailable adopted policy leaves this address's declarations unknown.
        for row in parent["worktree_status"]["worktrees"]
            .as_array()
            .into_iter()
            .flatten()
        {
            if row["record_kind"].is_null() {
                if let Some(path) = row["path"].as_str() {
                    if !self.artifact_addresses.contains(Path::new(path)) {
                        self.unknown(
                            path,
                            "no observable adopted artifact policy; do not infer declarations",
                        );
                    }
                }
            }
        }
        if let Some(error) = &self.io_error {
            self.errors.push(format!("inventory IO: {error}"));
        }
        let finished = timestamp()?;
        let descriptor = value!({"schema":"chrono-maintenance-inventory/v2",
            "path":self.path,"format":"jsonl","rows":self.rows,"byte_length":self.bytes,
            "status":if self.errors.is_empty() && self.enumeration_complete {"observed"} else {"incomplete"},
            "member_enumeration_complete":self.enumeration_complete && self.io_error.is_none(),
            "measurements_complete":self.measurements_complete && self.io_error.is_none(),
            "window_started_unix_millis":self.started,"window_finished_unix_millis":finished,
            "elapsed_millis":u64::try_from(self.began.elapsed().as_millis()).ok(),
            "publication_complete":false,"errors":self.errors,"physical_release_bytes":null});
        self.record(
            value!({"kind":"completion","data_rows":self.rows,"data_bytes":self.bytes,
            "member_enumeration_complete":descriptor["member_enumeration_complete"],
            "measurements_complete":descriptor["measurements_complete"],
            "window_finished_unix_millis":finished,"errors":self.errors}),
        );
        parent["complete_inventory"] = descriptor;
        parent["complete_inventory"]["rows"] = value!(self.rows);
        let persisted = if let Some(error) = self.io_error {
            // Preserve the actual partial original without retrying a failed
            // record or flushing more bytes after its write boundary failed.
            let _ = self.output.into_parts();
            Err(format!("inventory IO: {error}"))
        } else {
            finish_inventory_writer(self.output, |output| {
                output.sync_all().map_err(|e| e.to_string())
            })
        };
        if let Err(error) = persisted {
            parent["complete_inventory"]["status"] = value!("incomplete");
            parent["complete_inventory"]["member_enumeration_complete"] = value!(false);
            parent["complete_inventory"]["measurements_complete"] = value!(false);
            return Err(error);
        }
        let physical = self.root.join(&self.path);
        let (digest, length) = chrono_harness::file_identity(&physical)?;
        parent["complete_inventory"]["receipt"] = value!({"path":self.path,"sha256":digest});
        parent["complete_inventory"]["byte_length"] = value!(length);
        if let Some(publication) = self.publication {
            publication.complete(value!({"operation":"maintenance-inventory",
                "status":if self.errors.is_empty() {"observed"} else {"failed"}}))?;
        }
        // Hand the original to the actual parent before releasing its recovery root.
        let _guard = retention::read_guard(&self.root, &self.path)?;
        parent["complete_inventory"]["publication_complete"] = value!(true);
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "maintenance inventory incomplete: {}",
                self.errors.join("; ")
            ))
        }
    }
}

/// Copied declarations are observations, never a lease or effect authorization.
#[cfg(not(verus_keep_ghost))]
pub(super) struct ArtifactSnapshot {
    root: PathBuf,
    artifacts: Vec<Artifact>,
    entry: Option<Entry>,
    policy_sha256: String,
    policy_path: String,
    source_head: String,
}
#[cfg(not(verus_keep_ghost))]
impl ArtifactSnapshot {
    pub(super) fn from_owner(manager: &Manager) -> Self {
        let entry = manager
            .ledger
            .entries
            .iter()
            .find(|e| e.status != "disposed")
            .or_else(|| manager.ledger.entries.last())
            .cloned();
        Self {
            root: entry
                .as_ref()
                .map(|e| e.path.clone())
                .unwrap_or_else(|| manager.scope.clone()),
            artifacts: manager.inputs.policy.artifacts.clone(),
            entry,
            policy_sha256: sha256(&manager.inputs.policy_bytes),
            policy_path: manager.inputs.policy_path.clone(),
            source_head: manager.inputs.source_head.clone(),
        }
    }
}

/// Project all original actions; pointers refer to the unchanged published parent.
#[cfg(not(verus_keep_ghost))]
#[doc(hidden)]
pub fn worktree_action(path: &Path, drain: &Value) -> Value {
    let mut indices = Vec::new();
    for (index, row) in drain.as_array().into_iter().flatten().enumerate() {
        indices = append_action_index(
            indices,
            row["path"].as_str().is_some_and(|p| Path::new(p) == path),
            index,
        );
    }
    let results: Vec<_> = indices
        .into_iter()
        .map(|index| format!("/drain/{index}"))
        .collect();
    value!({"kind":"worktree-action","path":path,
        "execution":if results.is_empty() {"not-executed"} else {"see original results"},
        "selection":if results.is_empty() {"not-selected-or-not-reached"} else {"responsible-owner-result"},
        "parent_report_pointers":results})
}
