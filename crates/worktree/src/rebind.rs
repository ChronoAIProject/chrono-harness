//! Recreate metadata from explicit current identities; never infer a lost index.
use crate::{
    maintenance,
    rebind_inputs::{self, Expected},
    recovery,
    start::{self, Runner},
};
use chrono_harness::{facts, no_symlink_parents, relative_path};
use chrono_judge_registration::Registrations;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json as value};
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Binding {
    path: String,
    branch: String,
    index_tree: String,
    metadata_id: String,
    backup: String,
    donor: String,
    expected: Option<Expected>,
}
fn text(path: &Path) -> Result<&str, String> {
    path.to_str()
        .ok_or_else(|| "rebind path is not UTF-8".into())
}
fn absent(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
        Ok(_) => Err(format!("rebind output already exists: {}", path.display())),
    }
}
fn new_path(root: &Path, path: &str) -> Result<PathBuf, String> {
    let path = root.join(path);
    let parent = fs::canonicalize(path.parent().ok_or("rebind output lacks parent")?)
        .map_err(|e| e.to_string())?;
    let result = parent.join(path.file_name().ok_or("rebind output lacks name")?);
    absent(&result)?;
    Ok(result)
}
fn overlap(a: &Path, b: &Path) -> bool {
    a.starts_with(b) || b.starts_with(a)
}
fn stable_source(
    r: &mut Runner,
    root: &Path,
    report: &Value,
    plan_path: &str,
    bytes: &[u8],
    branch: &str,
    head: &str,
) -> Result<(), String> {
    if recovery::state_bytes(root, plan_path)? != bytes {
        return Err("rebind plan changed".into());
    }
    if r.oid(root, "HEAD")? != report["source_commit"].as_str().unwrap()
        || r.oid(root, branch)? != head
    {
        return Err("rebind source or branch changed".into());
    }
    if fs::read(root.join(report["config_path"].as_str().unwrap())).map_err(|e| e.to_string())?
        != r.blob(
            root,
            report["source_commit"].as_str().unwrap(),
            report["config_path"].as_str().unwrap(),
        )?
    {
        return Err("rebind configuration changed".into());
    }
    Ok(())
}
impl Binding {
    pub(crate) fn execute(
        self,
        r: &mut Runner,
        root: &Path,
        registrations: &Registrations,
        token: &str,
        report: &mut Value,
        head: &str,
        plan_path: &str,
        bytes: &[u8],
        inspect: bool,
    ) -> Result<(), String> {
        facts::full_oid(&self.index_tree)?;
        relative_path(&self.metadata_id)?;
        if Path::new(&self.metadata_id).components().count() != 1 {
            return Err("metadata_id must name one explicit worktrees member".into());
        }
        if inspect == self.expected.is_some() {
            return Err(
                "inspection requires expected:null; rebind requires observed identities".into(),
            );
        }
        let branch = format!("refs/heads/{}", self.branch);
        r.git(root, &["check-ref-format", &branch])?;
        if !["feature_prefix", "integration_prefix"].iter().any(|key| {
            registrations.workflow()[key]
                .as_str()
                .is_some_and(|prefix| self.branch.starts_with(prefix))
        }) {
            return Err("rebind branch lacks a registered work prefix".into());
        }
        if r.oid(root, &branch)? != head
            || r.oid(root, &format!("{head}^{{commit}}"))? != head
            || r.oid(root, &format!("{}^{{tree}}", self.index_tree))? != self.index_tree
        {
            return Err("rebind branch HEAD or explicit index tree mismatch".into());
        }
        let requested = root.join(&self.path);
        if !fs::symlink_metadata(&requested)
            .map_err(|e| e.to_string())?
            .is_dir()
        {
            return Err("rebind target must be a real directory".into());
        }
        let target = fs::canonicalize(&requested).map_err(|e| e.to_string())?;
        let common =
            fs::canonicalize(root.join(r.text(root, &["rev-parse", "--git-common-dir"])?.trim()))
                .map_err(|e| e.to_string())?;
        let parent = common.join("worktrees");
        match fs::symlink_metadata(&parent) {
            Ok(m) if m.is_dir() => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            _ => {
                return Err(
                    "worktree metadata parent must be absent or a physical directory".into(),
                );
            }
        }
        let metadata = parent.join(&self.metadata_id);
        let donor = new_path(root, &self.donor)?;
        if self
            .metadata_id
            .starts_with(text(Path::new(donor.file_name().unwrap()))?)
        {
            return Err(
                "choose a donor basename distinct from the old metadata allocation prefix".into(),
            );
        }
        relative_path(&self.backup)?;
        if !self.backup.starts_with(".chrono-harness/state/") {
            return Err("rebind backup must be under registered state".into());
        }
        maintenance::artifacts(registrations, &[&self.backup])?;
        let backup = no_symlink_parents(root, &self.backup)?;
        absent(&backup)?;
        if overlap(&target, root)
            || overlap(&target, &common)
            || overlap(&target, &donor)
            || overlap(&donor, root)
            || overlap(&donor, &common)
        {
            return Err("rebind paths overlap protected source, metadata or donor".into());
        }
        let inventory = r.inventory(root)?;
        for (i, row) in inventory.iter().enumerate() {
            let path = Path::new(row.get("worktree").ok_or("worktree inventory lacks path")?);
            if i == 0 && path == target {
                return Err("cannot rebind main worktree".into());
            }
            if overlap(path, &donor)
                || (path != target
                    && (overlap(path, &target) || row.get("branch") == Some(&branch)))
            {
                return Err("rebind path or branch belongs to another worktree".into());
            }
        }
        let observed = rebind_inputs::observe(&target, &metadata)?;
        report["destination"] = value!(target);
        report["branch_ref"] = value!(self.branch);
        report["base"] = value!(head);
        report["head"] = value!(head);
        report["index_tree"] = value!(self.index_tree);
        report["index_origin"] = value!("explicit-plan");
        report["original_outcome"] = value!("unknown");
        report["metadata_path"] = value!(metadata);
        report["backup_path"] = value!(backup);
        report["donor_path"] = value!(donor);
        report["observed_inputs"] = value!(observed.expected);
        report["visible_inputs"] = observed.visible["entries"].clone();
        report["visible_before"] = value!(observed.expected.visible);
        report["original_gitfile_bytes"] = value!(observed.gitfile_bytes);
        report["original_metadata"] = value!(observed.metadata);
        // A healthy attached checkout belongs to ordinary maintenance, even with a missing old receipt.
        let attached = r.command(&target, &["rev-parse", "--show-toplevel"])?;
        if attached.exit_code == 0 && Path::new(attached.stdout.trim()) == target {
            let index = r.command(&target, &["ls-files", "--stage", "-z"])?;
            if index.exit_code == 0 {
                return Err("checkout metadata is usable; use ordinary maintenance".into());
            }
        }
        // Available links must agree; an absent old receipt never authorizes another checkout's metadata.
        let mut linked = false;
        if let Some(gitfile) = &observed.gitfile_bytes {
            if let Ok(string) = std::str::from_utf8(gitfile) {
                if let Some(pointer) = string.strip_prefix("gitdir: ") {
                    if rebind_inputs::pointer(&target, pointer.trim_end_matches('\n'))? != metadata
                    {
                        return Err(
                            "original .git pointer differs from explicit metadata member".into(),
                        );
                    }
                    linked = true;
                }
            }
        }
        if observed.metadata.is_some() {
            match fs::read(metadata.join("gitdir")) {
                Ok(bytes) => {
                    if let Ok(pointer) = std::str::from_utf8(&bytes) {
                        if rebind_inputs::pointer(&metadata, pointer.trim_end_matches('\n'))?
                            != target.join(".git")
                        {
                            return Err("selected metadata belongs to another checkout".into());
                        }
                        linked = true;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(e.to_string()),
            }
            if !linked {
                return Err(
                    "existing metadata has no usable relationship to the explicit target".into(),
                );
            }
        }
        stable_source(r, root, report, plan_path, bytes, &branch, head)?;
        if inspect {
            let mut binding = self.clone();
            binding.expected = Some(observed.expected);
            report["proposed_plan"] = value!({"schema":"chrono-worktree-maintenance/v1","operation":"rebind","head":head,"binding":binding});
            return Ok(());
        }
        if self.expected.as_ref() != Some(&observed.expected) {
            return Err("rebind inputs changed since explicit observation".into());
        }
        report["plan_path"] = value!(plan_path);
        report["plan_sha256"] = value!(chrono_harness::sha256(bytes));
        report["expected_inputs_digest"] =
            value!(chrono_harness::wire::digest(&value!(observed.expected))?);
        recovery::publish_fields(
            root,
            report,
            "chrono-worktree-rebind-intent/v1",
            ".rebind-intent.json",
            "rebind_intent",
            &[
                "operation",
                "source_root",
                "source_commit",
                "config_path",
                "config_sha256",
                "registry_digest",
                "destination",
                "branch_ref",
                "head",
                "index_tree",
                "metadata_path",
                "backup_path",
                "donor_path",
                "lock_reason",
                "report_path",
                "visible_before",
                "plan_path",
                "plan_sha256",
                "expected_inputs_digest",
            ],
        )?;
        if rebind_inputs::observe(&target, &metadata)?.expected != observed.expected {
            return Err("rebind inputs changed before preservation".into());
        }
        stable_source(r, root, report, plan_path, bytes, &branch, head)?;
        fs::create_dir(&backup).map_err(|e| format!("rebind backup: {e}"))?;
        report["phase"] = value!("backup-created");
        if let Some(original) = &observed.gitfile_bytes {
            let mut f = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(backup.join("gitfile"))
                .map_err(|e| e.to_string())?;
            f.write_all(original).map_err(|e| e.to_string())?;
            f.set_permissions(fs::Permissions::from_mode(observed.gitfile_mode.unwrap()))
                .map_err(|e| e.to_string())?;
            f.sync_all().map_err(|e| e.to_string())?;
        }
        if observed.metadata.is_some() {
            fs::rename(&metadata, backup.join("metadata"))
                .map_err(|e| format!("preserve original metadata: {e}"))?;
            if rebind_inputs::snapshot(&backup.join("metadata"), false)?
                != observed.metadata.clone().unwrap()
            {
                return Err("preserved metadata identity changed".into());
            }
        }
        report["phase"] = value!("original-metadata-preserved");
        stable_source(r, root, report, plan_path, bytes, &branch, head)?;
        r.git(
            root,
            &[
                "worktree",
                "add",
                "--detach",
                "--no-checkout",
                "--lock",
                "--reason",
                token,
                "--",
                text(&donor)?,
                head,
            ],
        )?;
        report["phase"] = value!("donor-created");
        r.git(&donor, &["symbolic-ref", "HEAD", &branch])?;
        r.git(&donor, &["read-tree", &self.index_tree])?;
        let new_metadata =
            fs::canonicalize(r.text(&donor, &["rev-parse", "--absolute-git-dir"])?.trim())
                .map_err(|e| e.to_string())?;
        if new_metadata.parent() != Some(parent.as_path()) || new_metadata == metadata {
            return Err("donor metadata is not a new common-repository member".into());
        }
        report["new_metadata_path"] = value!(new_metadata);
        let new_gitfile = fs::read(donor.join(".git")).map_err(|e| e.to_string())?;
        let current = rebind_inputs::observe(&target, &metadata)?;
        if current.expected.gitfile != observed.expected.gitfile
            || current.expected.visible != observed.expected.visible
            || current.expected.metadata.is_some()
        {
            return Err("original work changed before attachment".into());
        }
        stable_source(r, root, report, plan_path, bytes, &branch, head)?;
        let mut replacement = tempfile::Builder::new()
            .prefix(".chrono-rebind-")
            .tempfile_in(&target)
            .map_err(|e| e.to_string())?;
        replacement
            .write_all(&new_gitfile)
            .map_err(|e| e.to_string())?;
        replacement
            .as_file()
            .set_permissions(
                fs::metadata(donor.join(".git"))
                    .map_err(|e| e.to_string())?
                    .permissions(),
            )
            .map_err(|e| e.to_string())?;
        replacement
            .as_file()
            .sync_all()
            .map_err(|e| e.to_string())?;
        replacement
            .persist(target.join(".git"))
            .map_err(|e| format!("new Git pointer publication: {e}"))?;
        report["phase"] = value!("pointer-published");
        r.git(root, &["worktree", "repair", "--", text(&target)?])?;
        start::ensure_identity(r, root, &target, &self.branch, head, token)?;
        if r.text(&target, &["write-tree"])?.trim() != self.index_tree {
            return Err("rebound index differs from explicit tree".into());
        }
        if fs::read(donor.join(".git")).map_err(|e| e.to_string())? != new_gitfile {
            return Err("donor pointer changed; preserve it".into());
        }
        fs::remove_file(donor.join(".git")).map_err(|e| e.to_string())?;
        fs::remove_dir(&donor).map_err(|e| format!("preserve nonempty donor: {e}"))?;
        report["phase"] = value!("attached");
        let after = rebind_inputs::snapshot(&target, true)?;
        report["visible_after"] = after["sha256"].clone();
        if after["sha256"] != observed.expected.visible {
            return Err("visible work changed during metadata rebind".into());
        }
        stable_source(r, root, report, plan_path, bytes, &branch, head)?;
        start::unlock(r, root, text(&target)?, &branch, head)?;
        if r.text(&target, &["write-tree"])?.trim() != self.index_tree
            || rebind_inputs::snapshot(&target, true)?["sha256"] != observed.expected.visible
        {
            return Err("rebound work changed after unlock".into());
        }
        stable_source(r, root, report, plan_path, bytes, &branch, head)?;
        rebind_inputs::verify_backup(&backup, &observed)?;
        report["phase"] = value!("verified");
        report["backup_preserved"] = value!(true);
        report["donor_removed"] = value!(true);
        Ok(())
    }
}
