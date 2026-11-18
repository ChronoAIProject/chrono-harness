//! Creation-age eligibility belongs to the existing addressed lifecycle owner.
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn age_preservation_required(disposable_present: bool, retained_file_present: bool)
    -> (required: bool)
    ensures required == (disposable_present || !retained_file_present),
{
    disposable_present || !retained_file_present
}

pub fn reusable_release(terminal_handoff: bool, expired_source_preserved: bool, generation: u64,
    requested_generation: Option<u64>) -> (release: bool)
    ensures release <==> (terminal_handoff || expired_source_preserved || (requested_generation.is_some()
        && requested_generation.unwrap() == generation)),
{
    terminal_handoff || expired_source_preserved || match requested_generation {
        Some(requested) => requested == generation,
        None => false,
    }
}
}

#[cfg(not(verus_keep_ghost))]
use super::*;
#[cfg(not(verus_keep_ghost))]
use std::time::{Duration, SystemTime};

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AgePolicy {
    reuse_after_seconds: u64,
    delete_after_seconds: u64,
    preserved_branch_prefix: String,
}
#[cfg(not(verus_keep_ghost))]
impl AgePolicy {
    pub(super) fn validate(&self, kernel: bool) -> Result<(), String> {
        if !kernel
            || self.reuse_after_seconds == 0
            || self.delete_after_seconds <= self.reuse_after_seconds
            || !self.preserved_branch_prefix.ends_with('/')
        {
            return Err(
                "age policy requires kernel ownership, 0 < reuse < delete and a branch prefix"
                    .into(),
            );
        }
        relative_path(self.preserved_branch_prefix.trim_end_matches('/'))
    }
    fn eligible(&self, age: Duration, deleting: bool) -> bool {
        age > Duration::from_secs(if deleting {
            self.delete_after_seconds
        } else {
            self.reuse_after_seconds
        })
    }
}
#[cfg(not(verus_keep_ghost))]
impl Manager {
    // Sufficient evidence that this attempt has no physical effect. Probe only
    // declared roots and one immediate retained entry per root; other layouts
    // continue through the original preservation and retirement implementation.
    fn age_has_no_effect(&self, e: &Entry) -> Result<bool, String> {
        if e.cache_attempts.last().is_some_and(|a| a.receipt.is_none()) {
            return Ok(false);
        }
        let mut retained_file = false;
        for artifact in &self.inputs.policy.artifacts {
            let path = no_symlink_parents(&e.path, artifact.path.trim_end_matches('/'))?;
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.to_string()),
            };
            if select_artifact(&artifact.disposition, true, false) {
                return Ok(!age_preservation_required(true, retained_file));
            }
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(
                    "retained age artifact is not a physical directory; preserve it".into(),
                );
            }
            if let Some(entry) = fs::read_dir(&path)
                .map_err(|error| error.to_string())?
                .next()
                .transpose()
                .map_err(|error| error.to_string())?
            {
                retained_file |= entry
                    .file_type()
                    .map_err(|error| error.to_string())?
                    .is_file();
            }
        }
        Ok(!age_preservation_required(false, retained_file))
    }

    // A previous clean pass does not cover a later change to the checkout.
    // Recheck current roots and the same attachment under its enrollment lease.
    pub(super) fn observe_settled_age(
        &mut self,
        r: &mut Runner,
        i: usize,
        lease: &Lease,
    ) -> Result<bool, String> {
        use crate::verified_lifecycle::CleanupStage;
        let e = &self.ledger.entries[i];
        if !matches!(e.status.as_str(), "active" | "retained")
            || e.ownership.as_ref().is_none_or(|owner| owner.cache_pending)
            || !e.cleanup.as_ref().is_some_and(|progress| {
                matches!(
                    progress.stage,
                    CleanupStage::CachesReleased | CleanupStage::Retained
                )
            })
            || e.attempts.iter().any(|attempt| attempt.receipt.is_none())
            || !self.age_has_no_effect(e)?
        {
            return Ok(false);
        }
        self.reconcile_birth(r, i, lease)?;
        let e = &self.ledger.entries[i];
        let head = r.oid(&e.path, "HEAD")?;
        self.check_entry(r, e, &head, None)?;
        lease.stable()?;
        Ok(true)
    }

    pub(super) fn age_eligible(&self, e: &Entry, deleting: bool) -> Result<bool, String> {
        let Some(policy) = &self.inputs.policy.age_policy else {
            return Ok(false);
        };
        if e.cache_only || e.path == self.inputs.policy.coordinator_root {
            return Ok(false);
        }
        // Neither last modification nor enrollment/import time establishes birth.
        let created = fs::symlink_metadata(&e.path)
            .map_err(|e| e.to_string())?
            .created()
            .map_err(|e| format!("checkout creation age unknown: {e}"))?;
        let age = SystemTime::now()
            .duration_since(created)
            .map_err(|_| "checkout creation time is in the future; preserve it")?;
        Ok(policy.eligible(age, deleting))
    }

    fn retained_branch(&self, r: &mut Runner, reference: &str, commit: &str) -> Result<(), String> {
        let root = &self.inputs.policy.coordinator_root;
        r.git(root, &["check-ref-format", reference])?;
        // Never overwrite another owner's branch or follow a symbolic ref.
        if r.command(root, &["symbolic-ref", "--quiet", reference])?
            .exit_code
            != 1
        {
            return Err("source custody requires a direct retained branch".into());
        }
        let found = r.command(root, &["show-ref", "--verify", "--quiet", reference])?;
        match found.exit_code {
            1 => {
                r.git(
                    root,
                    &[
                        "update-ref",
                        "--no-deref",
                        reference,
                        commit,
                        &"0".repeat(commit.len()),
                    ],
                )?;
            }
            0 => (),
            n => return Err(format!("source branch lookup exited {n}")),
        }
        if r.oid(root, reference)? != commit {
            return Err("retained source branch changed".into());
        }
        Ok(())
    }

    /// Preserve HEAD, the original index tree and current source bytes as actual
    /// branch commits. Generated directories never enter the source commit.
    fn preserve_age_source(
        &self,
        r: &mut Runner,
        e: &Entry,
        report: &mut Value,
    ) -> Result<(String, String), String> {
        let head = r.oid(&e.path, "HEAD")?;
        self.check_entry(r, e, &head, None)?;
        let policy = self
            .inputs
            .policy
            .age_policy
            .as_ref()
            .ok_or("age policy missing")?;
        let prefix = format!(
            "refs/heads/{}{}/",
            policy.preserved_branch_prefix,
            sha256(
                e.ownership
                    .as_ref()
                    .ok_or("age cleanup requires kernel ownership")?
                    .id
                    .as_bytes()
            )
        );
        let original_ref = format!("{prefix}head-{head}");
        self.retained_branch(r, &original_ref, &head)?;
        let reflog = r.text(&e.path, &["reflog", "show", "--format=%H", "HEAD"])?;
        let commits: Vec<_> = reflog
            .lines()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .filter(|commit| *commit != head)
            .collect();
        for commit in &commits {
            facts::full_oid(commit)?;
        }
        let mut reflog_refs = BTreeMap::new();
        // The retained HEAD already covers its ancestors. Keep argv below 9 KiB
        // even for SHA-256 OIDs instead of passing an unbounded reflog to Git.
        for batch in commits.chunks(128) {
            let mut args = vec!["merge-base", "--independent", &head];
            args.extend_from_slice(batch);
            for commit in r.text(&e.path, &args)?.lines() {
                facts::full_oid(commit)?;
                if commit != head {
                    let reference = format!("{prefix}reflog-{commit}");
                    reflog_refs.insert(reference, commit.to_owned());
                }
            }
        }
        if !reflog_refs.is_empty() {
            let retained = r.text(
                &self.inputs.policy.coordinator_root,
                &[
                    "for-each-ref",
                    "--format=%(refname) %(objectname) %(symref)",
                    &prefix,
                ],
            )?;
            let mut observed = BTreeSet::new();
            for line in retained.lines() {
                let (reference, identity) = line
                    .split_once(' ')
                    .ok_or("invalid retained branch inventory")?;
                let Some(commit) = reflog_refs.get(reference) else {
                    continue;
                };
                let (oid, symbolic) = identity
                    .split_once(' ')
                    .ok_or("invalid retained branch identity")?;
                if !symbolic.is_empty() {
                    return Err("source custody requires a direct retained branch".into());
                }
                if oid != commit {
                    return Err("retained source branch changed".into());
                }
                observed.insert(reference);
            }
            // Git omits dangling symbolic refs from this inventory. New refs
            // still need the original direct-ref check and compare-and-create.
            for (reference, commit) in &reflog_refs {
                if !observed.contains(reference.as_str()) {
                    self.retained_branch(r, reference, commit)?;
                }
            }
        }
        let index = r.text(&e.path, &["write-tree"])?.trim().to_owned();
        facts::full_oid(&index)?;
        let commit_tree =
            |r: &mut Runner, tree: &str, parent: &str, message: &[u8]| -> Result<String, String> {
                let result = r.input(&e.path, &["commit-tree", tree, "-p", parent], message)?;
                if result.exit_code != 0 {
                    return Err(format!(
                        "source preservation commit failed: {}",
                        result.stderr
                    ));
                }
                let commit = result.stdout.trim().to_owned();
                facts::full_oid(&commit)?;
                Ok(commit)
            };
        let head_tree = r.oid(&e.path, &format!("{head}^{{tree}}"))?;
        let (index_commit, index_ref) = if head_tree == index {
            (head.clone(), original_ref.clone())
        } else {
            let commit = commit_tree(
                r,
                &index,
                &head,
                b"Preserve lifecycle index before age reuse\n",
            )?;
            let reference = format!("{prefix}index-{commit}");
            self.retained_branch(r, &reference, &commit)?;
            (commit, reference)
        };
        let host_config = r.config.host_config.clone();
        let (regs, _) = start::registrations(r, &e.path, &head, &host_config)?;
        let tracked = start::paths(r.git(&e.path, &["ls-files", "-z"])?)?;
        let tracked_paths: Vec<_> = tracked.iter().cloned().collect();
        if chrono_judge_registration::nonartifact_paths(regs.config(), &tracked_paths).len()
            != tracked.len()
        {
            return Err(
                "age preservation refuses tracked source inside generated artifacts".into(),
            );
        }
        // Explicit Git inventory, including ignored source. Registered artifacts
        // are excluded by their existing owner; no path/language inference.
        let mut paths = tracked;
        let exclusions = facts::artifact_exclusions(&facts::artifact_directories(regs.config())?);
        let mut args = vec!["ls-files", "--others", "-z", "--", "."];
        args.extend(exclusions.iter().map(String::as_str));
        paths.extend(start::paths(r.literal_inventory(&e.path, &args)?)?);
        let selected = chrono_judge_registration::nonartifact_paths(
            regs.config(),
            &paths.into_iter().collect::<Vec<_>>(),
        );
        let mut input = Vec::new();
        for path in selected {
            input.extend_from_slice(path.as_bytes());
            input.push(0);
        }
        self.check_entry(r, e, &head, None)?;
        if !input.is_empty() {
            let add = r.input(
                &e.path,
                &[
                    "--literal-pathspecs",
                    "add",
                    "--force",
                    "--all",
                    "--pathspec-from-file=-",
                    "--pathspec-file-nul",
                ],
                &input,
            )?;
            if add.exit_code != 0 {
                return Err(format!(
                    "source staging failed; original index branch retained: {}",
                    add.stderr
                ));
            }
        }
        let tree = r.text(&e.path, &["write-tree"])?.trim().to_owned();
        let (saved, reference) = if head_tree == tree {
            (head.clone(), original_ref.clone())
        } else if index == tree {
            (index_commit.clone(), index_ref.clone())
        } else {
            let commit = commit_tree(
                r,
                &tree,
                &head,
                b"Preserve source before automatic worktree age reuse\n",
            )?;
            let reference = format!("{prefix}source-{commit}");
            self.retained_branch(r, &reference, &commit)?;
            (commit, reference)
        };
        self.check_entry(r, e, &head, None)?;
        if saved != head {
            r.git(
                &e.path,
                &[
                    "update-ref",
                    "--no-deref",
                    &maintenance::checkout_reference(e.current_branch()),
                    &saved,
                    &head,
                ],
            )?;
        }
        // The committed index already describes current working source. No
        // reset/checkout of working bytes and no overwrite of the index original.
        self.check_entry(r, e, &saved, None)?;
        start::cleanliness(r, &e.path, regs.config())?;
        report["source_preservation"] = value!({"original_head":head,"original_ref":original_ref,"reflog_refs":reflog_refs,
            "index_tree":index,"index_ref":index_ref,"index_commit":index_commit,
            "head":saved,"retained_ref":reference,"working_source":"committed","original_index":"retained-as-commit"});
        Ok((saved, reference))
    }

    pub(super) fn execute_age(
        &mut self,
        r: &mut Runner,
        i: usize,
        token: &str,
        report: &mut Value,
    ) -> Result<(), String> {
        let e = self.ledger.entries[i].clone();
        if !self.age_eligible(&e, false)? {
            return Err("checkout has not passed its reuse age".into());
        }
        self.birth_ready(&e)?;
        report["age_policy"] = value!({"reuse_after_seconds":self.inputs.policy.age_policy.as_ref().unwrap().reuse_after_seconds,
            "delete_after_seconds":self.inputs.policy.age_policy.as_ref().unwrap().delete_after_seconds,
            "clock":"physical-checkout-creation","task_completion":"not-claimed"});
        if self.age_has_no_effect(&e)? {
            let head = r.oid(&e.path, "HEAD")?;
            self.check_entry(r, &e, &head, None)?;
            report["source_preservation"] = value!({"status":"not-needed",
                "reason":"all disposable roots absent; retained file prevents checkout retirement"});
            report["age_outcome"] = value!("no-disposable-output-retained-checkout");
            if let Some(owner) = &mut self.ledger.entries[i].ownership {
                owner.cache_pending = false;
            }
            return self.save();
        }
        let (head, reference) = self.preserve_age_source(r, &e, report)?;
        let expired_source_preserved = self.age_eligible(&e, true)?;
        if expired_source_preserved {
            let names: Vec<_> = self
                .inputs
                .policy
                .artifacts
                .iter()
                .filter(|a| select_artifact(&a.disposition, true, false))
                .map(|a| a.path.clone())
                .collect();
            let host_config = r.config.host_config.clone();
            let (regs, _) = start::registrations(r, &e.path, &head, &host_config)?;
            let mut disposal = regs.config().clone();
            disposal["artifacts"]
                .as_array_mut()
                .unwrap()
                .retain(|a| names.iter().any(|n| a["path"] == *n));
            if let Err(error) = start::cleanliness(r, &e.path, &disposal) {
                report["age_deletion_deferred"] = value!(error);
            } else {
                let mut record = value!({
                    "schema":"chrono-worktree-age-retirement/v1","binding":Self::birth_binding(&e),
                    "path":e.path,"attachment":e.attachment,
                    "head":head,"retained_ref":reference,"source_preservation":report["source_preservation"],
                    "task_completion":"not-claimed","age_policy":report["age_policy"],"prior_terminal":e.terminal
                });
                if let Some(t) = &e.terminal {
                    if let Some(handoff) = e.terminal_handoff()? {
                        record["prior_terminal_handoff"] = handoff;
                    } else {
                        record["prior_terminal_input"] = self.retained_input(
                            &t.receipt.path,
                            &self.receipt(&t.receipt)?,
                            "receipt",
                        );
                    }
                }
                let intent = self.immutable(&format!("age-retirement-{token}.json"), &record)?;
                self.ledger.entries[i].enrollment["age_retirement"] = value!(intent);
                self.ledger.entries[i].terminal = Some(Terminal {
                    head: head.clone(),
                    retained_commit: head,
                    dispose_evidence: false,
                    artifacts_only: false,
                    receipt: intent,
                    established: None,
                });
                self.ledger.entries[i].status = "terminal".into();
                self.save()?;
                return self.execute_age_retirement(r, i, token, report);
            }
        }
        let owner = self.ledger.entries[i]
            .ownership
            .as_mut()
            .ok_or("age reuse ownership missing")?;
        owner.generation = owner
            .generation
            .checked_add(1)
            .ok_or("age cache generation overflow")?;
        owner.cache_pending = true;
        self.save()?;
        self.execute_cache(r, i, token, report, expired_source_preserved)?;
        report["age_outcome"] = value!("reusable-source-preserved");
        Ok(())
    }

    pub(super) fn execute_age_retirement(
        &mut self,
        r: &mut Runner,
        i: usize,
        token: &str,
        report: &mut Value,
    ) -> Result<(), String> {
        let e = self.ledger.entries[i].clone();
        let receipt: Receipt = serde_json::from_value(e.enrollment["age_retirement"].clone())
            .map_err(|e| e.to_string())?;
        let bytes = self.receipt(&receipt)?;
        let intent = json(&bytes)?;
        let head = intent["head"].as_str().ok_or("age head missing")?;
        let reference = intent["retained_ref"]
            .as_str()
            .ok_or("age retained branch missing")?;
        if intent["schema"] != "chrono-worktree-age-retirement/v1"
            || intent["binding"] != Self::birth_binding(&e)
            || e.terminal
                .as_ref()
                .is_none_or(|t| t.head != head || t.receipt.path != receipt.path)
        {
            return Err("age retirement identity changed".into());
        }
        self.retained_branch(r, reference, head)?;
        report["terminal_receipt"] = value!(receipt);
        report["age_retirement_input"] = self.retained_input(&receipt.path, &bytes, "receipt");
        report["source_preservation"] = intent["source_preservation"].clone();
        report["age_policy"] = intent["age_policy"].clone();
        if let Some(prior) = e.attempts.last() {
            report["prior_age_result"] = self.result_input(&prior.path, prior.receipt.as_ref())?;
            if e.path.exists() {
                let lock = r
                    .inventory(&self.inputs.policy.coordinator_root)?
                    .into_iter()
                    .find(|row| row.get("worktree").is_some_and(|p| Path::new(p) == e.path))
                    .and_then(|row| row.get("locked").cloned());
                if lock.as_deref() == Some(&prior.token) {
                    let raw = crate::recovery::retained_bytes(
                        &self.inputs.policy.coordinator_root,
                        report["prior_age_result"]
                            .get("input")
                            .ok_or("interrupted age lock needs its original report")?,
                    )?;
                    let tree = r.oid(&e.path, &format!("{head}^{{tree}}"))?;
                    maintenance::release_reconciled(
                        r,
                        &self.inputs.policy.coordinator_root,
                        report,
                        &json(&raw)?,
                        head,
                        &tree,
                        || self.receipt(&receipt).map(|_| ()),
                    )?;
                }
            }
        }
        self.ledger.entries[i].attempts.push(Attempt {
            path: report["report_path"]
                .as_str()
                .ok_or("age report path")?
                .into(),
            token: token.into(),
            receipt: None,
        });
        self.save()?;
        let names = self
            .inputs
            .policy
            .artifacts
            .iter()
            .filter(|a| select_artifact(&a.disposition, true, false))
            .map(|a| a.path.clone())
            .collect();
        Cleanup {
            path: e.path.clone(),
            branch: e.current_branch().into(),
            head: head.into(),
            retained_ref: reference.into(),
            retained_commit: head.into(),
            retention: Retention::Ancestor,
            allow_retained_ref_advance: false,
            discard_artifacts: names,
            remove_branch: false,
            allow_absent_worktree: !e.attempts.is_empty(),
        }
        .execute_checked(
            r,
            &self.inputs.policy.coordinator_root,
            &self.inputs.registrations,
            token,
            report,
            e.enrollment["kind"].as_str().unwrap_or(""),
            |r| {
                self.stable(r)?;
                self.git_locks(&e, None)?;
                if self.receipt(&receipt)? != bytes {
                    return Err("age intent changed".into());
                }
                maintenance::saved_commit(
                    r,
                    &self.inputs.policy.coordinator_root,
                    head,
                    reference,
                    head,
                    &Retention::Ancestor,
                )?;
                if e.path.exists() {
                    if !self.age_eligible(&e, true)? {
                        return Err("checkout no longer has verified deletion age".into());
                    }
                    let lock = r
                        .inventory(&self.inputs.policy.coordinator_root)?
                        .into_iter()
                        .find(|row| row.get("worktree").is_some_and(|p| Path::new(p) == e.path))
                        .and_then(|row| row.get("locked").cloned());
                    self.check_entry(r, &e, head, lock.as_deref())?;
                }
                Ok(())
            },
        )?;
        report["age_outcome"] = value!("expired-checkout-verified-absent-source-branches-retained");
        Ok(())
    }
}
