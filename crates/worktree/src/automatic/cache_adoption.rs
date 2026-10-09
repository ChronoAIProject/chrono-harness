//! Current operator custody of exact historical main outputs, not past producer release.
use super::*;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Custody {
    schema: String,
    path: PathBuf,
    head: String,
    artifacts: Vec<String>,
    current_consumers_released: bool,
    reason: String,
}

impl Manager {
    pub(super) fn main_cache_owned(&self, e: &Entry) -> Result<BTreeSet<String>, String> {
        let initial: Vec<String> = serde_json::from_value(
            e.enrollment["main_cache_artifacts"].clone(),
        )
        .map_err(|_| "main cache artifact declarations missing or invalid; preserve outputs")?;
        let mut owned: BTreeSet<_> = initial.iter().cloned().collect();
        if initial.len() != owned.len() {
            return Err("ambiguous main cache ownership; preserve outputs".into());
        }
        for receipt in &e.main_cache_adoptions {
            let record = json(&self.receipt(receipt)?)?;
            let plan: Custody = serde_json::from_value(record["custody"].clone())
                .map_err(|e| format!("invalid current custody record: {e}"))?;
            let selected: BTreeSet<_> = plan.artifacts.iter().cloned().collect();
            if record["schema"] != "chrono-main-cache-adoption/v1"
                || record["binding"] != Self::birth_binding(e)
                || record["previous"] != value!(owned)
                || record["historical_producer_outcome"] != "unknown"
                || record["disposal"] != "not-performed"
                || record["terminal_handoff"] != "not-claimed"
                || plan.schema != "chrono-main-cache-custody/v1"
                || plan.path != e.path
                || !plan.current_consumers_released
                || plan.reason.trim().is_empty()
                || selected.is_empty()
                || selected.len() != plan.artifacts.len()
                || selected.iter().any(|p| owned.contains(p))
            {
                return Err("main cache adoption chain does not bind current custody".into());
            }
            facts::full_oid(&plan.head)?;
            owned.extend(selected);
            if record["next"] != value!(owned) {
                return Err("main cache adoption has an invalid ownership transition".into());
            }
        }
        Ok(owned)
    }

    pub(super) fn protected_main_history(&self, e: &Entry) -> Result<Value, String> {
        let owned = self.main_cache_owned(e)?;
        Ok(value!(
            e.enrollment["protected_historical_artifacts"]
                .as_array()
                .ok_or("main historical ownership declarations missing")?
                .iter()
                .filter(|row| !row["path"].as_str().is_some_and(|p| owned.contains(p)))
                .collect::<Vec<_>>()
        ))
    }

    pub(super) fn adopt_main_cache(
        &mut self,
        r: &mut Runner,
        target: &Path,
        plan_path: &str,
        report: &mut Value,
    ) -> Result<(), String> {
        let i = self
            .current_entry(target)
            .ok_or("current custody requires main enrollment")?;
        let e = self.ledger.entries[i].clone();
        if !e.cache_only || e.status != "active" || target != self.policy.coordinator_root {
            return Err(
                "current cache custody requires the active adopted Git main checkout".into(),
            );
        }
        let lease = self
            .lease(&e, true)?
            .ok_or("main has live registered consumers; preserve outputs")?;
        let head = r.oid(target, "HEAD")?;
        self.check_entry(r, &e, &head, None)?;
        let bytes = crate::recovery::state_bytes(target, plan_path)?;
        let plan: Custody = decode(&bytes)?;
        if plan.schema != "chrono-main-cache-custody/v1"
            || plan.path != target
            || plan.head != head
            || !plan.current_consumers_released
            || plan.reason.trim().is_empty()
            || plan.artifacts.is_empty()
        {
            return Err(
                "current custody must bind exact main HEAD and explicit consumer release".into(),
            );
        }
        let selected: BTreeSet<_> = plan.artifacts.iter().cloned().collect();
        if selected.len() != plan.artifacts.len()
            || selected.iter().any(|p| {
                !self
                    .policy
                    .artifacts
                    .iter()
                    .any(|a| a.path == *p && a.disposition == Disposition::Dispose)
            })
        {
            return Err("current custody selects repeated or undeclared disposable outputs".into());
        }
        artifact_disposal::registered(&plan.artifacts, &[self.registrations.config()])?;
        artifact_disposal::paths(r, target, &head, &plan.artifacts)?;
        let previous = self.main_cache_owned(&e)?;
        let newly_owned: Vec<_> = selected.difference(&previous).cloned().collect();
        if newly_owned.is_empty() {
            report["cache_adoption"] = value!({"status":"already-managed", "path":target,
                "disposal":"not-performed", "terminal_handoff":"not-claimed"});
            return lease.stable();
        }
        // Preserve the exact current declaration, and record only its new ownership delta.
        // The operator's declaration covers prior unmanaged consumers; kernel exclusion
        // covers adopted concurrent users. Neither proves any historical producer outcome.
        let mut custody = value!(plan);
        custody["artifacts"] = value!(newly_owned);
        let mut next = previous.clone();
        next.extend(newly_owned);
        let record = value!({"schema":"chrono-main-cache-adoption/v1",
            "binding":Self::birth_binding(&e), "previous":previous, "next":next,
            "custody":custody, "declaration_input":self.retain_bytes(&bytes)?,
            "coordinator_commit":self.anchor_head, "policy_sha256":sha256(&self.policy_bytes),
            "registry_digest":self.registry_digest, "historical_producer_outcome":"unknown",
            "disposal":"not-performed", "terminal_handoff":"not-claimed"});
        let name = format!(
            "main-cache-adoption-{}.json",
            chrono_harness::wire::digest(&record)?
        );
        let receipt = self.immutable_reconciled(&name, &record)?;
        self.check_entry(r, &e, &head, None)?;
        artifact_disposal::paths(r, target, &head, &plan.artifacts)?;
        if crate::recovery::state_bytes(target, plan_path)? != bytes {
            return Err("current custody declaration changed before adoption".into());
        }
        lease.stable()?;
        self.reconcile_uses(i, &lease)?;
        let mut updated = self.ledger.entries[i].clone();
        updated.main_cache_adoptions.push(receipt.clone());
        let owner = updated
            .ownership
            .as_mut()
            .ok_or("main custody lost kernel ownership")?;
        owner.generation = owner
            .generation
            .checked_add(1)
            .ok_or("ownership generation overflow")?;
        owner.cache_pending = true;
        self.check_entry(r, &updated, &head, None)?;
        lease.stable()?;
        self.ledger.entries[i] = updated;
        self.save()?;
        report["cache_adoption"] = value!({"status":"adopted", "path":target,
            "receipt":receipt, "selected":custody["artifacts"],
            "historical_producer_outcome":"unknown", "disposal":"not-performed",
            "terminal_handoff":"not-claimed"});
        Ok(())
    }
}
