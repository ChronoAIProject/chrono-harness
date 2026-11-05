//! Adopt exact externally completed disposal without rewriting its original outcome.
use super::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Adoption {
    schema: String,
    binding: Value,
    original_result: Receipt,
    result_index: usize,
    original_registration: Receipt,
}
impl Manager {
    pub(super) fn adopt_absence(
        &mut self,
        r: &mut Runner,
        target: &Path,
        plan: &str,
        report: &mut Value,
    ) -> Result<(), String> {
        let plan_bytes = crate::recovery::state_bytes(&self.inputs.policy.coordinator_root, plan)?;
        let adoption: Adoption = decode(&plan_bytes)?;
        let i = self
            .current_entry(target)
            .ok_or("absence adoption requires a current enrollment")?;
        let e = self.ledger.entries[i].clone();
        if adoption.schema != "chrono-worktree-absent-adoption/v1"
            || adoption.binding != Self::birth_binding(&e)
            || e.cache_only
            || e.path == self.inputs.policy.coordinator_root
        {
            return Err("absence adoption does not bind this linked enrollment".into());
        }
        let lease = self
            .lease(&e, true)?
            .ok_or("absent enrollment has live registered consumers; preserve it")?;
        self.birth_ready(&e)?;
        self.policy_binding(&e)?;
        let original = self.receipt(&adoption.original_result)?;
        let registration = self.receipt(&adoption.original_registration)?;
        let results = json(&original)?;
        let row = match &results {
            Value::Array(rows) => rows.get(adoption.result_index),
            Value::Object(_) if adoption.result_index == 0 => Some(&results),
            _ => None,
        }
        .ok_or("external disposal result selection missing")?;
        let registered = json(&registration)?;
        let head = row["head"]
            .as_str()
            .ok_or("external disposal has no retained HEAD")?;
        let reference = row["retained_ref"]
            .as_str()
            .ok_or("external disposal has no retained branch")?;
        facts::full_oid(head)?;
        // The actual batch and single-checkout manual producers retain distinct
        // original formats. Decode those bytes here; never normalize the receipt.
        let bound_targets = match (registered.get("targets"), registered.get("target")) {
            (Some(Value::Array(targets)), None) => targets
                .iter()
                .filter(|t| {
                    t["path"] == value!(target)
                        && t["head"] == head
                        && t["owner_git_common"] == value!(self.inputs.common)
                })
                .count(),
            (None, Some(t @ Value::Object(_))) if results.is_object() => {
                let observed = |v: &Value, expected: &str| {
                    v["exit"] == 0
                        && v["stdout"]
                            .as_str()
                            .is_some_and(|s| s.strip_suffix('\n').unwrap_or(s) == expected)
                };
                usize::from(
                    t["path"] == value!(target)
                        && observed(&t["head"], head)
                        && observed(
                            &t["common"],
                            self.inputs
                                .common
                                .to_str()
                                .ok_or("Git common path is not UTF-8")?,
                        ),
                )
            }
            _ => {
                return Err(
                    "external disposal registration has no unambiguous explicit targets".into(),
                );
            }
        };
        if row["path"] != value!(target)
            || row["status"] != "verified-absent"
            || bound_targets != 1
            || !reference.starts_with("refs/heads/")
            || reference == format!("refs/heads/{}", e.branch)
        {
            return Err("external disposal originals do not identify this path, repository and retained source".into());
        }
        let verify = |r: &mut Runner| -> Result<(), String> {
            self.stable(r)?;
            lease.stable()?;
            self.git_locks(&e, None)?;
            if self.receipt(&adoption.original_result)? != original
                || self.receipt(&adoption.original_registration)? != registration
                || crate::recovery::state_bytes(&self.inputs.policy.coordinator_root, plan)?
                    != plan_bytes
            {
                return Err("external disposal originals changed".into());
            }
            for path in [&e.path, &e.attachment.metadata] {
                match fs::symlink_metadata(path) {
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                    _ => return Err("absent enrollment path or original Git attachment still exists; preserve it".into()),
                }
            }
            for row in r.inventory(&self.inputs.policy.coordinator_root)? {
                let path = PathBuf::from(row.get("worktree").ok_or("Git inventory path missing")?);
                if path.starts_with(target)
                    || target.starts_with(&path)
                    || row.get("branch") == Some(&format!("refs/heads/{}", e.branch))
                {
                    return Err(
                        "external disposal path or branch is still attached; preserve it".into(),
                    );
                }
            }
            if r.command(
                &self.inputs.policy.coordinator_root,
                &["symbolic-ref", "--quiet", reference],
            )?
            .exit_code
                != 1
            {
                return Err("external source custody must name a direct retained branch".into());
            }
            maintenance::saved_commit(
                r,
                &self.inputs.policy.coordinator_root,
                head,
                reference,
                head,
                &Retention::Ancestor,
            )
        };
        verify(r)?;
        let receipt = self.immutable_reconciled(&format!("absent-adoption-{}.json", sha256(&plan_bytes)), &value!({
            "schema":"chrono-worktree-absent-adoption-receipt/v1","binding":adoption.binding,
            "original_result":adoption.original_result,"original_registration":adoption.original_registration,
            "original_result_input":self.retained_input(&adoption.original_result.path,&original,"bytes"),
            "original_registration_input":self.retained_input(&adoption.original_registration.path,&registration,"bytes"),
            "result_index":adoption.result_index,"retained_ref":reference,"head":head,
            "observed_worktree":"already-absent","original_outcomes":"unchanged","task_completion":"not-claimed"
        }))?;
        verify(r)?;
        self.reconcile_uses(i, &lease)?;
        self.ledger.entries[i].enrollment["absent_adoption"] = value!(receipt);
        if self.ledger.entries[i].terminal.is_none() {
            self.ledger.entries[i].terminal = Some(Terminal {
                head: head.into(),
                retained_commit: head.into(),
                dispose_evidence: false,
                artifacts_only: false,
                receipt: receipt.clone(),
            });
        }
        self.ledger.entries[i].status = "disposed".into();
        self.save()?;
        report["absent_adoption"] = value!(receipt);
        report["worktree_removal"] = value!("already-absent");
        report["worktree_removed"] = value!(false);
        report["original_outcomes"] = value!("unchanged");
        Ok(())
    }
}
