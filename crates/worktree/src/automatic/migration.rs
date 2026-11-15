//! Explicit policy and checkout adoption preserve original enrollment evidence.
#[cfg(verus_keep_ghost)]
use super::super::identifiers::equal_text;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{verified_identifiers::equal_text, verus};
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
// The caller holds the original exclusive lease. Cleanup observations are
// diagnostic history; actual unjoined work still prevents checkout adoption.
pub fn checkout_migration_ready(status: &str, terminal: bool, uses_pending: bool,
    handoff_pending: bool) -> (ready: bool)
    ensures ready <==> (status@ == "active"@ && !terminal
        && !uses_pending && !handoff_pending),
{
    equal_text(status, "active") && !terminal && !uses_pending && !handoff_pending
}
}

#[cfg(not(verus_keep_ghost))]
use super::*;

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct PolicySource {
    pub(super) head: String,
    pub(super) inputs: BTreeMap<String, Option<String>>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct PolicyBinding {
    pub(super) policy_sha256: String,
    pub(super) head: String,
    pub(super) inputs: BTreeMap<String, Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) policy_source: Option<PolicySource>,
}

#[cfg(not(verus_keep_ghost))]
impl Manager {
    fn committed_policy_source(&self, r: &mut Runner) -> Result<PolicySource, String> {
        let inputs = BTreeMap::from([
            (
                self.inputs.config_path.clone(),
                Some(sha256(&self.inputs.config_bytes)),
            ),
            (
                self.inputs.policy_path.clone(),
                Some(sha256(&self.inputs.policy_bytes)),
            ),
        ]);
        if r.oid(&self.inputs.source_root, "HEAD")? != self.inputs.source_head
            || self.target_policy_inputs(r, &self.inputs.source_root, &self.inputs.source_head)?
                != inputs
        {
            return Err("cleanup policy source changed before adoption".into());
        }
        Ok(PolicySource {
            head: self.inputs.source_head.clone(),
            inputs,
        })
    }

    pub(super) fn policy_binding(&self, e: &Entry) -> Result<PolicyBinding, String> {
        let mut binding = PolicyBinding {
            policy_sha256: e.policy_sha256.clone(),
            head: e.enrollment["observed_head"]
                .as_str()
                .ok_or("enrollment lacks original observed HEAD")?
                .into(),
            inputs: serde_json::from_value(e.enrollment["policy_inputs"].clone())
                .map_err(|_| "enrollment lacks original target policy input bindings")?,
            policy_source: None,
        };
        for receipt in &e.policy_migrations {
            let report = json(&self.receipt(receipt)?)?;
            if report["schema"] != "chrono-worktree-policy-migration/v1"
                || report["binding"] != Self::birth_binding(e)
                || report["previous"] != value!(binding)
                || report["terminal_handoff"] != "not-claimed"
                || report["disposal"] != "not-performed"
            {
                return Err("policy migration chain does not bind this enrollment".into());
            }
            binding = serde_json::from_value(report["next"].clone())
                .map_err(|e| format!("invalid migrated policy binding: {e}"))?;
            facts::full_oid(&binding.head)?;
            if let Some(source) = &binding.policy_source {
                facts::full_oid(&source.head)?;
                if source.inputs.keys().ne(binding.inputs.keys())
                    || source.inputs.get(&self.inputs.policy_path)
                        != Some(&Some(binding.policy_sha256.clone()))
                {
                    return Err("adopted cleanup source does not bind policy inputs".into());
                }
            }
        }
        Ok(binding)
    }

    fn migration_inputs(
        &self,
        r: &mut Runner,
        target: &Path,
        binding: &PolicyBinding,
        phase: &str,
    ) -> Result<Vec<Value>, String> {
        let tree = r.tree(target, &binding.head)?;
        let mut retained = vec![];
        for (path, expected) in &binding.inputs {
            relative_path(path)?;
            let bytes = if tree.contains_key(path) {
                Some(r.blob(target, &binding.head, path)?)
            } else {
                None
            };
            if bytes.as_ref().map(|b| sha256(b)) != *expected {
                return Err("migration source does not match retained policy inputs".into());
            }
            let mut row = value!({"phase":phase,"path":path,"head":binding.head,
                "presence":if bytes.is_some() {"present"} else {"absent"}});
            if let Some(bytes) = bytes {
                row["input"] = self.retain_bytes(&bytes)?;
            }
            retained.push(row);
        }
        if let Some(source) = &binding.policy_source {
            retained.extend(self.migration_inputs(
                r,
                target,
                &PolicyBinding {
                    policy_sha256: binding.policy_sha256.clone(),
                    head: source.head.clone(),
                    inputs: source.inputs.clone(),
                    policy_source: None,
                },
                &format!("{phase}-policy-source"),
            )?);
        }
        Ok(retained)
    }

    pub(super) fn migrate(
        &mut self,
        r: &mut Runner,
        target: &Path,
        adopt_cleanup_policy: bool,
        report: &mut Value,
    ) -> Result<(), String> {
        let target = absolute(target, false)?;
        let i = self
            .current_entry(&target)
            .ok_or("policy migration requires an existing enrollment")?;
        let e = self.ledger.entries[i].clone();
        if !matches!(e.status.as_str(), "active" | "retained") {
            return Err(
                "policy migration preserves pending terminal disposal; reconcile it first".into(),
            );
        }
        let lease = self
            .lease(&e, true)?
            .ok_or("worktree has live registered consumers; preserve it")?;
        let head = r.oid(&target, "HEAD")?;
        let mut current = e.clone();
        if !e.cache_only {
            let observed = r.checkout_identity(&target)?;
            let branch = if observed.branch == "HEAD" {
                "HEAD"
            } else {
                observed.branch.strip_prefix("refs/heads/").ok_or(
                    "checkout migration requires a local branch or explicit imported checkout",
                )?
            };
            if branch != e.current_branch() {
                if !crate::verified_lifecycle::checkout_migration_ready(
                    &e.status,
                    e.terminal.is_some(),
                    // Defer interrupted uses and completed-result handoff until
                    // the candidate's own inputs and attachment are qualified.
                    false,
                    false,
                ) {
                    return Err("checkout migration preserves pending work; reconcile its use, result or terminal handoff first".into());
                }
                if (branch == "HEAD" && e.enrollment["kind"] != "import")
                    || !crate::verified_lifecycle::accepts_enrollment_branch(
                        e.enrollment["kind"].as_str().unwrap_or(""),
                        ["feature_prefix", "integration_prefix"].iter().any(|key| {
                            self.inputs.registrations.workflow()[key]
                                .as_str()
                                .is_some_and(|prefix| branch.starts_with(prefix))
                        }),
                    )
                {
                    return Err("checkout migration requires a registered local work branch or explicit imported checkout".into());
                }
                current.checkout_branch = (branch != e.branch).then(|| branch.to_owned());
            }
        }
        let checkout_changed = current.current_branch() != e.current_branch();
        // The original exclusive lease excludes registered descendants. The
        // ordinary attachment check still verifies inventory, HEAD and Git locks
        // against this exact candidate; ordinary use never follows a new branch.
        self.check_attachment(r, &current, &head, None)?;
        self.birth_ready(&e)?;
        let checkout_report = (!e.cache_only).then(|| {
            value!({
                "status":if checkout_changed {"adopted"} else {"already-current"},
                "path":target,"previous_branch":e.current_branch(),
                "branch":current.current_branch(),"head":head,
                "terminal_handoff":"not-claimed","disposal":"not-performed"
            })
        });
        let previous = self.policy_binding(&e)?;
        let adopted = BTreeMap::from([
            (
                self.inputs.config_path.clone(),
                Some(sha256(&self.inputs.config_bytes)),
            ),
            (
                self.inputs.policy_path.clone(),
                Some(sha256(&self.inputs.policy_bytes)),
            ),
        ]);
        let next = PolicyBinding {
            policy_sha256: sha256(&self.inputs.policy_bytes),
            inputs: self.target_policy_inputs(r, &target, &head)?,
            head: head.clone(),
            policy_source: if adopt_cleanup_policy {
                Some(self.committed_policy_source(r)?)
            } else {
                None
            },
        };
        if !adopt_cleanup_policy && next.inputs != adopted {
            return Err(
                "migration target must adopt its own committed policy/configuration".into(),
            );
        }
        if previous.inputs.keys().ne(next.inputs.keys()) {
            return Err(
                "policy input address relocation requires a separate explicit migration".into(),
            );
        }
        let host_config = decode::<crate::Config>(&self.inputs.config_bytes)?.host_config;
        let (target_regs, _) = start::registrations(r, &target, &head, &host_config)?;
        artifact_disposal::registered(
            &self
                .inputs
                .policy
                .artifacts
                .iter()
                .map(|a| a.path.clone())
                .collect::<Vec<_>>(),
            &[self.inputs.registrations.config(), target_regs.config()],
        )?;
        for path in next.inputs.keys() {
            start::registered_policy(&target_regs, path, &self.inputs.policy.state_directory)?;
        }
        let policy_changed = previous.policy_sha256 != next.policy_sha256
            || previous.inputs != next.inputs
            || previous.policy_source != next.policy_source;
        if !policy_changed && !checkout_changed {
            self.check_entry(r, &e, &head, None)?;
            lease.stable()?;
            if let Some(checkout_report) = checkout_report {
                report["checkout_migration"] = checkout_report;
            }
            report["policy_migration"] = value!({"status":"already-current","path":target,
                "terminal_handoff":"not-claimed","disposal":"not-performed"});
            return Ok(());
        }
        let receipt = if policy_changed {
            // Both snapshots supply fixed data, never historical programs.
            let mut inputs = self.migration_inputs(r, &target, &previous, "previous")?;
            inputs.extend(self.migration_inputs(r, &target, &next, "next")?);
            let record = value!({"schema":"chrono-worktree-policy-migration/v1",
                "binding":Self::birth_binding(&e),"previous":previous,"next":next,
                "coordinator_commit":self.inputs.anchor_head,
                "policy_source_root":self.inputs.source_root,
                "policy_source_commit":self.inputs.source_head,"migration_inputs":inputs,
                "terminal_handoff":"not-claimed","disposal":"not-performed"});
            let name = format!(
                "policy-migration-{}.json",
                chrono_harness::wire::digest(&record)?
            );
            Some(self.immutable_reconciled(&name, &record)?)
        } else {
            None
        };
        // A published receipt alone is not adoption. Admission and the original
        // lease remain held until the ledger references the complete transition.
        self.check_attachment(r, &current, &head, None)?;
        if self.target_policy_inputs(r, &target, &head)? != next.inputs {
            return Err("migration target policy changed before adoption".into());
        }
        if let Some(source) = &next.policy_source {
            if self.committed_policy_source(r)? != *source {
                return Err("cleanup policy source changed before adoption".into());
            }
        }
        lease.stable()?;
        if checkout_changed && e.report_handoff_pending() {
            self.reconcile_published_results().map_err(|error| {
                format!("checkout migration preserves pending work; result handoff is not established: {error}")
            })?;
        }
        self.reconcile_uses(i, &lease).map_err(|error| {
            if checkout_changed {
                format!("checkout migration preserves pending work; interrupted use is not reconciled: {error}")
            } else {
                error
            }
        })?;
        if checkout_changed {
            let enrolled = &self.ledger.entries[i];
            if !crate::verified_lifecycle::checkout_migration_ready(
                &enrolled.status,
                enrolled.terminal.is_some(),
                !enrolled.uses.is_empty(),
                enrolled.report_handoff_pending(),
            ) {
                return Err("checkout migration preserves pending work; reconcile its use, result or terminal handoff first".into());
            }
        }
        let mut updated = self.ledger.entries[i].clone();
        updated.checkout_branch = current.checkout_branch;
        if let Some(receipt) = &receipt {
            updated.policy_migrations.push(receipt.clone());
        }
        let owner = updated
            .ownership
            .as_mut()
            .ok_or("migration lost kernel ownership")?;
        owner.generation = owner
            .generation
            .checked_add(1)
            .ok_or("ownership generation overflow")?;
        owner.cache_pending = updated.status == "active";
        self.check_entry(r, &updated, &head, None)?;
        lease.stable()?;
        self.ledger.entries[i] = updated;
        self.save()?;
        if let Some(checkout_report) = checkout_report {
            report["checkout_migration"] = checkout_report;
        }
        report["policy_migration"] = value!({
            "status":if policy_changed {"adopted"} else {"already-current"},"path":target,
            "terminal_handoff":"not-claimed","disposal":"not-performed"});
        if let Some(receipt) = receipt {
            report["policy_migration"]["receipt"] = value!(receipt);
        }
        Ok(())
    }
}
