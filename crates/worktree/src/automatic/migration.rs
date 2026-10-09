//! Explicit policy adoption preserves the original enrollment and its evidence.
use super::*;

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct PolicyBinding {
    pub(super) policy_sha256: String,
    head: String,
    pub(super) inputs: BTreeMap<String, Option<String>>,
}

impl Manager {
    pub(super) fn policy_binding(&self, e: &Entry) -> Result<PolicyBinding, String> {
        let mut binding = PolicyBinding {
            policy_sha256: e.policy_sha256.clone(),
            head: e.enrollment["observed_head"]
                .as_str()
                .ok_or("enrollment lacks original observed HEAD")?
                .into(),
            inputs: serde_json::from_value(e.enrollment["policy_inputs"].clone())
                .map_err(|_| "enrollment lacks original target policy input bindings")?,
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
        Ok(retained)
    }

    pub(super) fn migrate(
        &mut self,
        r: &mut Runner,
        target: &Path,
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
        self.check_attachment(r, &e, &head, None)?;
        self.birth_ready(&e)?;
        let previous = self.policy_binding(&e)?;
        let next = PolicyBinding {
            policy_sha256: sha256(&self.policy_bytes),
            inputs: self.target_policy_inputs(r, &target, &head)?,
            head: head.clone(),
        };
        let adopted = BTreeMap::from([
            (self.config_path.clone(), Some(sha256(&self.config_bytes))),
            (self.policy_path.clone(), Some(sha256(&self.policy_bytes))),
        ]);
        if next.inputs != adopted {
            return Err(
                "migration target must adopt the coordinator's committed policy/configuration"
                    .into(),
            );
        }
        if previous.inputs.keys().ne(next.inputs.keys()) {
            return Err(
                "policy input address relocation requires a separate explicit migration".into(),
            );
        }
        let host_config = r.config.host_config.clone();
        let (target_regs, _) = start::registrations(r, &target, &head, &host_config)?;
        artifact_disposal::registered(
            &self
                .policy
                .artifacts
                .iter()
                .map(|a| a.path.clone())
                .collect::<Vec<_>>(),
            &[self.registrations.config(), target_regs.config()],
        )?;
        for path in next.inputs.keys() {
            start::registered_policy(&target_regs, path, &self.policy.state_directory)?;
        }
        if previous.policy_sha256 == next.policy_sha256 && previous.inputs == next.inputs {
            self.check_entry(r, &e, &head, None)?;
            lease.stable()?;
            report["policy_migration"] = value!({"status":"already-current","path":target,
                "terminal_handoff":"not-claimed","disposal":"not-performed"});
            return Ok(());
        }
        // The old Git snapshot supplies old bytes; the current snapshot supplies
        // new bytes. Neither snapshot executes a historical lifecycle program.
        let mut inputs = self.migration_inputs(r, &target, &previous, "previous")?;
        inputs.extend(self.migration_inputs(r, &target, &next, "next")?);
        let record = value!({"schema":"chrono-worktree-policy-migration/v1",
            "binding":Self::birth_binding(&e),"previous":previous,"next":next,
            "coordinator_commit":self.anchor_head,"migration_inputs":inputs,
            "terminal_handoff":"not-claimed","disposal":"not-performed"});
        let name = format!(
            "policy-migration-{}.json",
            chrono_harness::wire::digest(&record)?
        );
        let receipt = self.immutable_reconciled(&name, &record)?;
        // A published receipt alone is not adoption. Admission and the original
        // lease remain held until the ledger references the complete transition.
        self.check_attachment(r, &e, &head, None)?;
        if self.target_policy_inputs(r, &target, &head)? != next.inputs {
            return Err("migration target policy changed before adoption".into());
        }
        lease.stable()?;
        self.reconcile_uses(i, &lease)?;
        let mut updated = self.ledger.entries[i].clone();
        updated.policy_migrations.push(receipt.clone());
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
        report["policy_migration"] = value!({"status":"adopted","path":target,
            "receipt":receipt,"terminal_handoff":"not-claimed","disposal":"not-performed"});
        Ok(())
    }
}
