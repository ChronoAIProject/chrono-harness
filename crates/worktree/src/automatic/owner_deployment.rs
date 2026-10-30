//! Selected lifecycle bytes change only under the original admission + EX lease.
use super::*;

fn executable_binding(path: &Path) -> Result<Value, String> {
    let id = crate::ownership::identity(path)?;
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    if crate::ownership::identity(path)? != id {
        return Err("deployment executable identity changed".into());
    }
    Ok(value!({"identity":id,"sha256":sha256(&bytes),"size":bytes.len()}))
}

impl Manager {
    pub(super) fn deploy_owner(
        &mut self,
        r: &mut Runner,
        root: &Path,
        profile: &str,
        stage_path: &str,
        stage_sha: &str,
        report: &mut Value,
    ) -> Result<(), String> {
        if root != self.policy.coordinator_root || self._gate.lease.is_none() {
            return Err("owner deployment requires the original kernel Git-main owner".into());
        }
        let i = self
            .current_entry(root)
            .ok_or("owner deployment requires enrollment")?;
        let e = self.ledger.entries[i].clone();
        if !e.cache_only || e.status != "active" || e.terminal.is_some() {
            return Err("owner deployment preserves unknown or terminal enrollment".into());
        }
        let lease = self
            .lease(&e, true)?
            .ok_or("owner deployment has live inherited consumers; preserve selected bytes")?;
        self.birth_ready(&e)?;
        let head = r.oid(root, "HEAD")?;
        self.check_entry(r, &e, &head, None)?;
        if !e.uses.is_empty() {
            return Err("owner deployment has unjoined use inputs; preserve selected bytes".into());
        }
        let mut command = bootstrap_command(r, root, &self.registrations, profile)?;
        let profile_bytes =
            fs::read(no_symlink_parents(root, profile)?).map_err(|e| e.to_string())?;
        let policy = json(&profile_bytes)?;
        let declaration = &policy["owner_stage"];
        // A stage receipt is not a proof about unmanaged consumers. The current
        // adopted host has not qualified this input; it must remain a refusal.
        // Registered-only fixtures exercise the positive kernel domain. A host
        // can select it only after its actual caller-owned consumer handoff.
        if declaration["consumer_scope"] != "registered-enrollment-only" {
            return Err("owner deployment unmanaged consumer handoff is unqualified; preserve selected bytes".into());
        }
        if !stage_path.starts_with(".chrono-harness/state/bootstrap-") {
            return Err("owner deployment requires an original retained bootstrap report".into());
        }
        let raw = crate::recovery::state_bytes(root, stage_path)?;
        if sha256(&raw) != stage_sha {
            return Err("owner deployment stage report changed".into());
        }
        let stage = json(&raw)?;
        if stage["schema"] != "chrono-bootstrap-result/v1"
            || stage["status"] != "passed"
            || stage["config"] != profile
            || stage["source"]["state"] != "clean"
            || stage["source"]["commit"] != head
            || stage["host_root"] != value!(root)
        {
            return Err("owner deployment stage source/configuration differs".into());
        }
        let record = &stage["owner_stage"];
        if record["selected"] != declaration["selected"] {
            return Err("owner deployment destination differs from declaration".into());
        }
        let staged_path = record["staged"]["path"]
            .as_str()
            .ok_or("staged executable path")?;
        if !staged_path.starts_with(".chrono-harness/bin/owner-stages/") {
            return Err("owner deployment stage must survive cache disposal".into());
        }
        let staged = no_symlink_parents(root, staged_path)?;
        let mut bound = executable_binding(&staged)?;
        bound["path"] = value!(staged_path);
        if bound != record["staged"]
            || fs::canonicalize(std::env::current_exe().map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?
                != staged
        {
            return Err("owner deployment must execute its exact retained candidate bytes".into());
        }
        let selected = no_symlink_parents(
            root,
            declaration["selected"]
                .as_str()
                .ok_or("selected owner path")?,
        )?;
        let owner_identity = value!({"attachment":e.attachment,
            "admission":self._gate.lease.as_ref().unwrap().id(),
            "enrollment":e.ownership.as_ref().ok_or("kernel ownership")?.id});
        report["owner_deployment_identity"] = owner_identity.clone();
        command.args.extend([
            "--owner-publication".into(),
            stage_path.into(),
            stage_sha.into(),
        ]);
        command
            .env
            .insert("CHRONO_WORKTREE_OWNER_DEPLOYMENT".into(), stage_sha.into());
        lease.stable()?;
        self.check_entry(r, &e, &head, None)?;
        let digest = chrono_harness::file_identity(Path::new(&command.program))?.0;
        let process = chrono_harness::prepared::process_identities(
            chrono_harness::run_process_observed(root, &command, &[], &digest)?,
        );
        report["managed_process"] = value!(process);
        lease.stable()?;
        self.check_entry(r, &e, &head, None)?;
        if process.failure.is_some() || process.exit_code != 0 {
            report["managed_command_failed"] = value!(true);
            return Err(format!(
                "owner publication failed with original exit {}: {:?}",
                process.exit_code, process.failure
            ));
        }
        let publication = json(&process.stdout_bytes)?;
        if publication["schema"] != "chrono-bootstrap-owner-publication/v1"
            || publication["stage"] != *record
            || publication["installed"] != executable_binding(&selected)?
            || publication["installed"]["sha256"] != record["staged"]["sha256"]
            || executable_binding(
                &staged
                    .parent()
                    .ok_or("stage directory")?
                    .join("previous-owner"),
            )? != record["previous"]
        {
            return Err(
                "owner publication original/result identity mismatch; adoption not claimed".into(),
            );
        }
        let mut adoption = value!({"schema":"chrono-worktree-owner-adoption/v1",
            "source_commit":head,"stage_result":stage_path,"stage_sha256":stage_sha,
            "installed":publication["installed"],"original_owner":record["previous"],
            "original_protection":owner_identity,"publication":publication,
            "consumer_scope":"registered-enrollment-only","terminal_handoff":"not-claimed",
            "task_completion":"not-claimed"});
        let receipt = self.immutable_reconciled(
            &format!(
                "owner-adoption-{}.json",
                chrono_harness::wire::digest(&adoption)?
            ),
            &adoption,
        )?;
        lease.stable()?;
        self.check_entry(r, &e, &head, None)?;
        if executable_binding(&selected)? != adoption["installed"] {
            return Err("installed owner changed before adoption".into());
        }
        adoption["receipt"] = value!(receipt);
        let index = no_symlink_parents(root, ".chrono-harness/state/bootstrap-owner-current.json")?;
        write_json(&index, &adoption, false)?;
        lease.stable()?;
        self.check_entry(r, &e, &head, None)?;
        report["owner_deployment"] = adoption;
        Ok(())
    }
}
