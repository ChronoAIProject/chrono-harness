//! Explicit saved-state maintenance; this produces observations, never judge verdicts.
use crate::{
    recovery::{self, ExpectedResult, state_bytes},
    start::{self, Runner},
};
use chrono_harness::{decode, facts, json, relative_path, sha256};
use chrono_judge_registration::Registrations;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json as value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    path: String,
    sha256: String,
}
#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "lowercase", deny_unknown_fields)]
enum Plan {
    #[serde(rename = "inspect-rebind")]
    InspectRebind {
        schema: String,
        head: String,
        binding: crate::rebind::Binding,
    },
    Rebind {
        schema: String,
        head: String,
        binding: crate::rebind::Binding,
    },
    #[serde(rename = "resume-rebind")]
    ResumeRebind {
        schema: String,
        head: String,
        intent: Receipt,
        result: ExpectedResult,
    },
    #[serde(rename = "cleanup-remote")]
    CleanupRemote {
        schema: String,
        head: String,
        retained_commit: String,
        branch: String,
        expected_url: String,
        retained_ref: String,
        retention: Retention,
        allow_absent_ref: bool,
    },
    Recover {
        schema: String,
        receipt: Receipt,
        head: String,
        index_tree: String,
    },
    #[serde(rename = "recover-interrupted")]
    RecoverInterrupted {
        schema: String,
        intent: Receipt,
        result: ExpectedResult,
        head: String,
        index_tree: String,
    },
    Cleanup {
        schema: String,
        path: String,
        branch: String,
        head: String,
        retained_ref: String,
        retained_commit: String,
        retention: Retention,
        discard_artifacts: Vec<String>,
        remove_branch: bool,
        allow_absent_worktree: bool,
    },
    #[serde(rename = "cleanup-fetch")]
    CleanupFetch {
        schema: String,
        receipt: Receipt,
        head: String,
        retained_ref: String,
        retained_commit: String,
        allow_absent_ref: bool,
    },
    #[serde(rename = "cleanup-fetch-interrupted")]
    CleanupFetchInterrupted {
        schema: String,
        intent: Receipt,
        result: ExpectedResult,
        head: String,
        retained_ref: String,
        retained_commit: String,
        allow_absent_ref: bool,
    },
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Retention {
    Ancestor,
    SameTree,
}

pub(crate) fn run(args: &[String]) -> Result<Value, String> {
    if args.len() != 7 {
        return Err("usage: chrono-worktree inspect-rebind|rebind|resume-rebind|recover|recover-interrupted|cleanup|cleanup-fetch|cleanup-fetch-interrupted|cleanup-remote --host-root ROOT --config POLICY --plan STATE_PATH".into());
    }
    let mut values = BTreeMap::new();
    for pair in args[1..].chunks_exact(2) {
        if !["--host-root", "--config", "--plan"].contains(&pair[0].as_str())
            || pair[1].is_empty()
            || values.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("unknown, repeated or empty maintenance argument".into());
        }
    }
    let root = fs::canonicalize(values["--host-root"]).map_err(|e| e.to_string())?;
    let config_path = values["--config"];
    let plan_path = values["--plan"];
    let (config, config_bytes) = crate::configuration(&root, config_path)?;
    let bytes = state_bytes(&root, plan_path)?;
    let plan: Plan = decode(&bytes)?;
    let (schema, operation, head) = match &plan {
        Plan::InspectRebind { schema, head, .. } => (schema, "inspect-rebind", head),
        Plan::Rebind { schema, head, .. } => (schema, "rebind", head),
        Plan::ResumeRebind {
            schema,
            head,
            intent,
            ..
        } => {
            if intent.sha256.len() != 64 || !intent.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("invalid rebind intent digest".into());
            }
            (schema, "resume-rebind", head)
        }
        Plan::CleanupRemote {
            schema,
            head,
            retained_commit,
            ..
        } => {
            facts::full_oid(retained_commit)?;
            (schema, "cleanup-remote", head)
        }
        Plan::Recover {
            schema,
            head,
            index_tree,
            receipt,
        } => {
            facts::full_oid(index_tree)?;
            if receipt.sha256.len() != 64 || !receipt.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err("invalid original receipt digest".into());
            }
            (schema, "recover", head)
        }
        Plan::RecoverInterrupted {
            schema,
            head,
            index_tree,
            intent,
            ..
        } => {
            facts::full_oid(index_tree)?;
            if intent.sha256.len() != 64 || !intent.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("invalid recovery intent digest".into());
            }
            (schema, "recover-interrupted", head)
        }
        Plan::Cleanup {
            schema,
            head,
            retained_commit,
            discard_artifacts,
            ..
        } => {
            facts::full_oid(retained_commit)?;
            let mut seen = BTreeSet::new();
            for path in discard_artifacts {
                if !path.ends_with('/') || !seen.insert(path) {
                    return Err("invalid or duplicate artifact directory".into());
                }
                relative_path(path.trim_end_matches('/'))?;
            }
            (schema, "cleanup", head)
        }
        Plan::CleanupFetch {
            schema,
            receipt,
            head,
            retained_commit,
            ..
        } => {
            facts::full_oid(retained_commit)?;
            if receipt.sha256.len() != 64 || !receipt.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err("invalid original receipt digest".into());
            }
            (schema, "cleanup-fetch", head)
        }
        Plan::CleanupFetchInterrupted {
            schema,
            intent,
            head,
            retained_commit,
            ..
        } => {
            facts::full_oid(retained_commit)?;
            if intent.sha256.len() != 64 || !intent.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("invalid fetch intent digest".into());
            }
            (schema, "cleanup-fetch-interrupted", head)
        }
    };
    facts::full_oid(head)?;
    if schema != "chrono-worktree-maintenance/v1" || args[0] != operation {
        return Err("maintenance schema or operation mismatch".into());
    }
    start::with_report(
        &root,
        config_path,
        config,
        &config_bytes,
        operation,
        if operation == "inspect-rebind" {
            "observed"
        } else if matches!(operation, "rebind" | "resume-rebind") {
            "rebound"
        } else if matches!(operation, "recover" | "recover-interrupted") {
            "recovered"
        } else {
            "cleaned"
        },
        |r, token, report| {
            if matches!(operation, "cleanup-fetch" | "cleanup-fetch-interrupted") {
                report["fetch_ref_removal"] = value!("not-attempted");
            }
            report["maintenance_plan"] =
                value!({"path":plan_path,"sha256":sha256(&bytes),"input_bytes":bytes});
            report["recovery"] = value!(
                "Inspect original and new process evidence after partial failure; no original failure or governance verdict is rewritten."
            );
            if physical(r, &root, &["rev-parse", "--show-toplevel"])? != root {
                return Err("host root must be the actual Git checkout root".into());
            }
            let source_head = r.oid(&root, "HEAD")?;
            if r.blob(&root, &source_head, config_path)? != config_bytes {
                return Err("maintenance policy differs from source commit".into());
            }
            let host_config = r.config.host_config.clone();
            let (registrations, digest) =
                start::registrations(r, &root, &source_head, &host_config)?;
            if !registrations.filemap()["files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f["path"] == config_path)
            {
                return Err("maintenance policy is not registered in FILEMAP".into());
            }
            artifacts(
                &registrations,
                &[plan_path, report["report_path"].as_str().unwrap()],
            )?;
            report["source_commit"] = value!(source_head);
            report["registry_digest"] = value!(digest);
            match plan {
                Plan::InspectRebind { head, binding, .. } => binding.execute(
                    r,
                    &root,
                    &registrations,
                    token,
                    report,
                    &head,
                    plan_path,
                    &bytes,
                    true,
                    None,
                ),
                Plan::Rebind { head, binding, .. } => binding.execute(
                    r,
                    &root,
                    &registrations,
                    token,
                    report,
                    &head,
                    plan_path,
                    &bytes,
                    false,
                    None,
                ),
                Plan::ResumeRebind {
                    head,
                    intent,
                    result,
                    ..
                } => crate::rebind_resume::execute(
                    r,
                    &root,
                    &registrations,
                    report,
                    &head,
                    &intent.path,
                    &intent.sha256,
                    &result,
                    plan_path,
                    &bytes,
                ),
                Plan::CleanupRemote {
                    head,
                    retained_commit,
                    branch,
                    expected_url,
                    retained_ref,
                    retention,
                    allow_absent_ref,
                    ..
                } => crate::remote::Cleanup {
                    head,
                    retained_commit,
                    branch,
                    expected_url,
                    retained_ref,
                    retention,
                    allow_absent_ref,
                }
                .execute(r, &root, &registrations, report, || {
                    if state_bytes(&root, plan_path)? != bytes
                        || crate::configuration(&root, config_path)?.1 != config_bytes
                    {
                        return Err("remote cleanup plan or policy changed".into());
                    }
                    Ok(())
                }),
                Plan::Recover {
                    receipt,
                    head,
                    index_tree,
                    ..
                } => recover(
                    r,
                    &root,
                    config_path,
                    &config_bytes,
                    &registrations,
                    report,
                    receipt,
                    &head,
                    &index_tree,
                ),
                Plan::RecoverInterrupted {
                    intent,
                    result,
                    head,
                    index_tree,
                    ..
                } => {
                    artifacts(&registrations, &[&intent.path])?;
                    let evidence = recovery::inspect(
                        &root,
                        config_path,
                        &config_bytes,
                        &intent.path,
                        &intent.sha256,
                        &result,
                        report,
                    )?;
                    artifacts(
                        &registrations,
                        &[field(&evidence.descriptor, "report_path")?],
                    )?;
                    release_reconciled(
                        r,
                        &root,
                        report,
                        &evidence.descriptor,
                        &head,
                        &index_tree,
                        || evidence.stable(),
                    )
                }
                Plan::Cleanup {
                    path,
                    branch,
                    head,
                    retained_ref,
                    retained_commit,
                    retention,
                    discard_artifacts,
                    remove_branch,
                    allow_absent_worktree,
                    ..
                } => {
                    let cleanup = Cleanup {
                        path: root.join(path),
                        branch,
                        head,
                        retained_ref,
                        retained_commit,
                        retention,
                        discard_artifacts,
                        remove_branch,
                        allow_absent_worktree,
                    };
                    cleanup.execute(r, &root, &registrations, token, report)
                }
                Plan::CleanupFetch {
                    receipt,
                    head,
                    retained_ref,
                    retained_commit,
                    allow_absent_ref,
                    ..
                } => cleanup_fetch(
                    r,
                    &root,
                    config_path,
                    &config_bytes,
                    &registrations,
                    report,
                    receipt,
                    &head,
                    &retained_ref,
                    &retained_commit,
                    allow_absent_ref,
                ),
                Plan::CleanupFetchInterrupted {
                    intent,
                    result,
                    head,
                    retained_ref,
                    retained_commit,
                    allow_absent_ref,
                    ..
                } => {
                    artifacts(&registrations, &[&intent.path])?;
                    let evidence = crate::fetch_recovery::inspect(
                        &root,
                        config_path,
                        &config_bytes,
                        &intent.path,
                        &intent.sha256,
                        &result,
                        report,
                    )?;
                    artifacts(
                        &registrations,
                        &[field(&evidence.descriptor, "report_path")?],
                    )?;
                    cleanup_fetch_ref(
                        r,
                        &root,
                        report,
                        field(&evidence.descriptor, "fetch_ref")?,
                        &head,
                        &retained_ref,
                        &retained_commit,
                        allow_absent_ref,
                        || evidence.stable(),
                    )
                }
            }
        },
    )
}
pub(crate) fn artifacts(registrations: &Registrations, paths: &[&str]) -> Result<(), String> {
    let paths: Vec<_> = paths.iter().map(|p| (*p).to_string()).collect();
    if !chrono_judge_registration::nonartifact_paths(registrations.config(), &paths).is_empty() {
        return Err("maintenance input/output lacks a registered artifact owner".into());
    }
    Ok(())
}
fn physical(r: &mut Runner, root: &Path, args: &[&str]) -> Result<PathBuf, String> {
    let path = r.text(root, args)?;
    fs::canonicalize(root.join(path.trim())).map_err(|e| e.to_string())
}
fn common(r: &mut Runner, root: &Path) -> Result<PathBuf, String> {
    physical(r, root, &["rev-parse", "--git-common-dir"])
}
pub(crate) fn identity(
    r: &mut Runner,
    root: &Path,
    target: &Path,
    branch: &str,
    head: &str,
    lock: Option<&str>,
) -> Result<(), String> {
    let inventory = r.inventory(root)?;
    if target == root
        || inventory
            .first()
            .is_some_and(|x| x.get("worktree").is_some_and(|p| Path::new(p) == target))
    {
        return Err("cannot maintain the source or main worktree".into());
    }
    for row in &inventory {
        let path = Path::new(
            row.get("worktree")
                .ok_or("worktree inventory has no path")?,
        );
        if path != target && (path.starts_with(target) || target.starts_with(path)) {
            return Err("nested or overlapping worktree must be preserved".into());
        }
    }
    let rows: Vec<_> = inventory
        .iter()
        .filter(|x| x.get("worktree").is_some_and(|p| Path::new(p) == target))
        .collect();
    if rows.len() != 1
        || rows[0].get("HEAD").map(String::as_str) != Some(head)
        || rows[0].get("branch") != Some(&format!("refs/heads/{branch}"))
        || rows[0].get("locked").map(String::as_str) != lock
        || rows[0].contains_key("prunable")
    {
        return Err("worktree identity or ownership lock mismatch".into());
    }
    let observed = r.checkout_identity(target)?;
    if fs::canonicalize(target).map_err(|e| e.to_string())? != target
        || fs::canonicalize(&observed.top).map_err(|e| e.to_string())? != target
        || fs::canonicalize(&observed.common).map_err(|e| e.to_string())? != common(r, root)?
        || observed.head != head
        || r.text(target, &["symbolic-ref", "HEAD"])?.trim() != format!("refs/heads/{branch}")
    {
        return Err("checkout identity or common repository mismatch".into());
    }
    Ok(())
}
fn field<'a>(v: &'a Value, name: &str) -> Result<&'a str, String> {
    v[name]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("original report lacks {name}"))
}
fn failed_report(
    root: &Path,
    config_path: &str,
    config_bytes: &[u8],
    registrations: &Registrations,
    report: &mut Value,
    receipt: &Receipt,
    operations: &[&str],
) -> Result<(Value, Vec<u8>), String> {
    artifacts(registrations, &[&receipt.path])?;
    let bytes = state_bytes(root, &receipt.path)?;
    if sha256(&bytes) != receipt.sha256 {
        return Err("original report digest mismatch".into());
    }
    let original = json(&bytes)?;
    report["prior_report"] =
        value!({"path":receipt.path,"sha256":receipt.sha256,"input_bytes":bytes,"report":original});
    if original["schema"] != "chrono-worktree-report/v1"
        || original["status"] != "failed"
        || !original["operation"]
            .as_str()
            .is_some_and(|operation| operations.contains(&operation))
        || original["source_root"] != value!(root)
        || original["config_path"] != config_path
        || original["config_sha256"] != sha256(config_bytes)
        || original["report_path"] != receipt.path
    {
        return Err("original failed-operation/configuration identity mismatch".into());
    }
    Ok((original, bytes))
}
fn recover(
    r: &mut Runner,
    root: &Path,
    config_path: &str,
    config_bytes: &[u8],
    registrations: &Registrations,
    report: &mut Value,
    receipt: Receipt,
    head: &str,
    tree: &str,
) -> Result<(), String> {
    let (original, bytes) = failed_report(
        root,
        config_path,
        config_bytes,
        registrations,
        report,
        &receipt,
        &["start", "reconstruct", "cleanup", "rebind"],
    )?;
    release_reconciled(r, root, report, &original, head, tree, || {
        if state_bytes(root, &receipt.path)? != bytes {
            return Err("original report changed during recovery".into());
        }
        Ok(())
    })
}
pub(crate) fn release_reconciled(
    r: &mut Runner,
    root: &Path,
    report: &mut Value,
    original: &Value,
    head: &str,
    tree: &str,
    stable: impl Fn() -> Result<(), String>,
) -> Result<(), String> {
    let target = Path::new(field(original, "destination")?);
    let branch = field(&original, "branch_ref")?;
    let base = field(&original, "base")?;
    let lock = field(&original, "lock_reason")?;
    facts::full_oid(base)?;
    identity(r, root, target, branch, head, Some(lock))?;
    r.git(target, &["merge-base", "--is-ancestor", base, head])?;
    let config_path = r.config.host_config.clone();
    let (target_registrations, _) = start::registrations(r, target, head, &config_path)?;
    start::exact_checkout(r, target, tree)?;
    if r.text(target, &["write-tree"])?.trim() != tree {
        return Err("reconciled index tree mismatch".into());
    }
    start::untracked(r, target, target_registrations.config())?;
    identity(r, root, target, branch, head, Some(lock))?;
    stable()?;
    start::unlock(
        r,
        root,
        target.to_str().ok_or("worktree path is not UTF-8")?,
        &format!("refs/heads/{branch}"),
        head,
    )?;
    if r.text(target, &["write-tree"])?.trim() != tree {
        return Err("index changed while releasing recovery lock".into());
    }
    start::exact_checkout(r, target, tree)?;
    start::untracked(r, target, target_registrations.config())?;
    stable()?;
    report["destination"] = value!(target);
    report["branch_ref"] = value!(branch);
    report["head"] = value!(head);
    report["index_tree"] = value!(tree);
    report["context"] = value!({"base":base,"candidate":null,"branch_ref":branch});
    Ok(())
}
pub(crate) fn saved_commit(
    r: &mut Runner,
    root: &Path,
    head: &str,
    retained_ref: &str,
    retained_commit: &str,
    retention: &Retention,
) -> Result<(), String> {
    if r.oid(root, retained_ref)? != retained_commit {
        return Err("retained reference changed".into());
    }
    if r.oid(root, &format!("{}^{{commit}}", head))? != head {
        return Err("cleanup HEAD is not a commit".into());
    }
    match retention {
        Retention::Ancestor => {
            r.git(
                root,
                &["merge-base", "--is-ancestor", head, retained_commit],
            )?;
        }
        Retention::SameTree => {
            if r.oid(root, &format!("{}^{{tree}}", head))?
                != r.oid(root, &format!("{}^{{tree}}", retained_commit))?
            {
                return Err("retained commit does not preserve the exact worktree tree".into());
            }
        }
    }
    Ok(())
}

pub(crate) struct Cleanup {
    pub(crate) path: PathBuf,
    pub(crate) branch: String,
    pub(crate) head: String,
    pub(crate) retained_ref: String,
    pub(crate) retained_commit: String,
    pub(crate) retention: Retention,
    pub(crate) discard_artifacts: Vec<String>,
    pub(crate) remove_branch: bool,
    pub(crate) allow_absent_worktree: bool,
}
impl Cleanup {
    fn disposal_paths(&self, r: &mut Runner, target: &Path) -> Result<Vec<PathBuf>, String> {
        crate::artifact_disposal::paths(r, target, &self.head, &self.discard_artifacts)
    }
    fn saved(&self, r: &mut Runner, root: &Path) -> Result<(), String> {
        saved_commit(
            r,
            root,
            &self.head,
            &self.retained_ref,
            &self.retained_commit,
            &self.retention,
        )
    }
    fn absent(&self, r: &mut Runner, root: &Path) -> Result<(), String> {
        match fs::symlink_metadata(&self.path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            _ => return Err("cleanup destination still exists".into()),
        }
        for row in r.inventory(root)? {
            let path = Path::new(
                row.get("worktree")
                    .ok_or("worktree inventory has no path")?,
            );
            if path.starts_with(&self.path)
                || self.path.starts_with(path)
                || row.get("branch") == Some(&format!("refs/heads/{}", self.branch))
            {
                return Err(
                    "cleanup path or branch remains in use by a registered worktree".into(),
                );
            }
        }
        Ok(())
    }
    pub(crate) fn execute(
        self,
        r: &mut Runner,
        root: &Path,
        registrations: &Registrations,
        token: &str,
        report: &mut Value,
    ) -> Result<(), String> {
        self.execute_checked(r, root, registrations, token, report, |_| Ok(()))
    }
    pub(crate) fn execute_checked(
        mut self,
        r: &mut Runner,
        root: &Path,
        registrations: &Registrations,
        token: &str,
        report: &mut Value,
        mut guard: impl FnMut(&mut Runner) -> Result<(), String>,
    ) -> Result<(), String> {
        guard(r)?;
        report["worktree_removed"] = value!(false);
        report["branch_removed"] = value!(false);
        report["artifact_disposals"] = value!([]);
        report["worktree_removal"] = value!("not-attempted");
        report["branch_removal"] = value!(if self.remove_branch {
            "not-attempted"
        } else {
            "not-requested"
        });
        let branch_ref = format!("refs/heads/{}", self.branch);
        r.git(root, &["check-ref-format", &branch_ref])?;
        r.git(root, &["check-ref-format", &self.retained_ref])?;
        if !self.retained_ref.starts_with("refs/heads/") || self.retained_ref == branch_ref {
            return Err("retention must name a distinct explicit local branch".into());
        }
        let workflow = registrations.workflow();
        if !["feature_prefix", "integration_prefix"].iter().any(|k| {
            workflow[k]
                .as_str()
                .is_some_and(|prefix| self.branch.starts_with(prefix))
        }) {
            return Err("cleanup branch does not match a registered work prefix".into());
        }
        // Parent resolution supports an explicitly allowed absent final component without following it.
        let parent = fs::canonicalize(self.path.parent().ok_or("cleanup path has no parent")?)
            .map_err(|e| e.to_string())?;
        let target = parent.join(self.path.file_name().ok_or("cleanup path has no name")?);
        self.path = target.clone();
        self.saved(r, root)?;
        report["destination"] = value!(target);
        report["branch_ref"] = value!(self.branch);
        report["head"] = value!(self.head);
        report["base"] = value!(self.head);
        report["retained_ref"] = value!(self.retained_ref);
        report["retained_commit"] = value!(self.retained_commit);
        match fs::symlink_metadata(&target) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && self.allow_absent_worktree => {
                self.absent(r, root)?;
                report["worktree_removal"] = value!("already-absent");
            }
            Err(e) => return Err(format!("cleanup worktree unavailable: {e}")),
            Ok(_) => {
                identity(r, root, &target, &self.branch, &self.head, None)?;
                let config_path = r.config.host_config.clone();
                let (target_registrations, _) =
                    start::registrations(r, &target, &self.head, &config_path)?;
                let mut disposal = target_registrations.config().clone();
                crate::artifact_disposal::registered(
                    &self.discard_artifacts,
                    &[registrations.config(), target_registrations.config()],
                )?;
                disposal["artifacts"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|a| self.discard_artifacts.iter().any(|p| a["path"] == *p));
                start::cleanliness(r, &target, &disposal)?;
                self.disposal_paths(r, &target)?;
                let path = target.to_str().ok_or("cleanup path is not UTF-8")?;
                recovery::publish(root, report)?;
                guard(r)?;
                r.git(root, &["worktree", "lock", "--reason", token, "--", path])?;
                identity(r, root, &target, &self.branch, &self.head, Some(token))?;
                self.saved(r, root)?;
                start::cleanliness(r, &target, &disposal)?;
                let artifacts = self.disposal_paths(r, &target)?;
                crate::artifact_disposal::dispose(
                    &target,
                    &self.discard_artifacts,
                    &artifacts,
                    report,
                    || guard(r),
                )?;
                start::cleanliness(r, &target, &disposal)?;
                self.saved(r, root)?;
                start::unlock(r, root, path, &branch_ref, &self.head)?;
                identity(r, root, &target, &self.branch, &self.head, None)?;
                self.saved(r, root)?;
                start::cleanliness(r, &target, &disposal)?;
                guard(r)?;
                report["worktree_removal"] = value!("attempted-unverified");
                r.git(root, &["worktree", "remove", "--force", "--", path])?;
                self.absent(r, root)?;
                report["worktree_removed"] = value!(true);
                report["worktree_removal"] = value!("verified-absent");
            }
        }
        self.saved(r, root)?;
        if self.remove_branch {
            let lookup = r.command(root, &["show-ref", "--verify", "--quiet", &branch_ref])?;
            match lookup.exit_code {
                1 if self.allow_absent_worktree => {
                    report["branch_removal"] = value!("already-absent")
                }
                0 => {
                    self.absent(r, root)?;
                    if r.oid(root, &branch_ref)? != self.head {
                        return Err("cleanup branch changed; preserve its current ref".into());
                    }
                    guard(r)?;
                    report["branch_removal"] = value!("attempted-unverified");
                    r.git(
                        root,
                        &["update-ref", "--no-deref", "-d", &branch_ref, &self.head],
                    )?;
                    let absent =
                        r.command(root, &["show-ref", "--verify", "--quiet", &branch_ref])?;
                    if absent.exit_code != 1 {
                        return Err("branch absence was not verified after deletion".into());
                    }
                    report["branch_removed"] = value!(true);
                    report["branch_removal"] = value!("verified-absent");
                }
                n => return Err(format!("cleanup branch lookup exited {n}")),
            }
        }
        guard(r)?;
        self.saved(r, root)?;
        self.absent(r, root)
    }
}

fn cleanup_fetch(
    r: &mut Runner,
    root: &Path,
    config_path: &str,
    config_bytes: &[u8],
    registrations: &Registrations,
    report: &mut Value,
    receipt: Receipt,
    head: &str,
    retained_ref: &str,
    retained_commit: &str,
    allow_absent: bool,
) -> Result<(), String> {
    let (original, bytes) = failed_report(
        root,
        config_path,
        config_bytes,
        registrations,
        report,
        &receipt,
        &["start", "reconstruct"],
    )?;
    let fetch_ref = field(&original, "fetch_ref")?;
    if fetch_ref
        != format!(
            "refs/chrono-harness/fetch/{}",
            field(&original, "lock_reason")?
        )
        || original["fetch_ref_removed"] != false
        || original.get("base").is_some_and(|base| base != head)
    {
        return Err("original report does not identify this unresolved temporary fetch ref".into());
    }
    cleanup_fetch_ref(
        r,
        root,
        report,
        fetch_ref,
        head,
        retained_ref,
        retained_commit,
        allow_absent,
        || {
            if state_bytes(root, &receipt.path)? != bytes {
                return Err("original report changed during fetch-ref cleanup".into());
            }
            Ok(())
        },
    )
}
fn cleanup_fetch_ref(
    r: &mut Runner,
    root: &Path,
    report: &mut Value,
    fetch_ref: &str,
    head: &str,
    retained_ref: &str,
    retained_commit: &str,
    allow_absent: bool,
    stable: impl Fn() -> Result<(), String>,
) -> Result<(), String> {
    r.git(root, &["check-ref-format", fetch_ref])?;
    r.git(root, &["check-ref-format", retained_ref])?;
    if !retained_ref.starts_with("refs/heads/") {
        return Err("fetch retention must name an explicit local branch".into());
    }
    report["fetch_ref"] = value!(fetch_ref);
    report["head"] = value!(head);
    report["retained_ref"] = value!(retained_ref);
    report["retained_commit"] = value!(retained_commit);
    saved_commit(
        r,
        root,
        head,
        retained_ref,
        retained_commit,
        &Retention::Ancestor,
    )?;
    let symbolic = r.command(root, &["symbolic-ref", "--quiet", fetch_ref])?;
    if symbolic.exit_code != 1 {
        return Err("temporary fetch ref must be a direct ref".into());
    }
    let exists = r.command(root, &["show-ref", "--verify", "--quiet", fetch_ref])?;
    match exists.exit_code {
        1 if allow_absent => report["fetch_ref_removal"] = value!("already-absent"),
        0 => {
            if r.oid(root, fetch_ref)? != head {
                return Err("temporary fetch ref changed; preserve its current value".into());
            }
            stable()?;
            saved_commit(
                r,
                root,
                head,
                retained_ref,
                retained_commit,
                &Retention::Ancestor,
            )?;
            report["fetch_ref_removal"] = value!("attempted-unverified");
            r.git(root, &["update-ref", "--no-deref", "-d", fetch_ref, head])?;
            if r.command(root, &["show-ref", "--verify", "--quiet", fetch_ref])?
                .exit_code
                != 1
            {
                return Err("temporary fetch-ref absence was not verified after deletion".into());
            }
            report["fetch_ref_removed"] = value!(true);
            report["fetch_ref_removal"] = value!("verified-absent");
        }
        code => {
            return Err(format!(
                "temporary fetch-ref lookup exited {code}; absence requires an explicit retry plan"
            ));
        }
    }
    saved_commit(
        r,
        root,
        head,
        retained_ref,
        retained_commit,
        &Retention::Ancestor,
    )?;
    stable()
}
