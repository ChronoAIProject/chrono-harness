use crate::{
    Start,
    start::{self, Runner, paths},
};
use chrono_harness::{decode, facts, no_symlink_parents, relative_path, sha256};
use serde::Deserialize;
use serde_json::{Value, json as value};
use std::{collections::BTreeSet, fs, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    schema: String,
    base: String,
    candidate: String,
    changes: Vec<Change>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Change {
    path: String,
    action: Action,
    reason: Option<String>,
}
#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Action {
    Carry,
    Retire,
}
pub(crate) struct Input {
    path: String,
    bytes: Vec<u8>,
    plan: Plan,
}
pub(crate) fn read(root: &Path, path: &str) -> Result<Input, String> {
    relative_path(path)?;
    if !path.starts_with(".chrono-harness/state/") {
        return Err("reconstruction plan must be under .chrono-harness/state".into());
    }
    let bytes = fs::read(no_symlink_parents(root, path)?).map_err(|e| e.to_string())?;
    let plan: Plan = decode(&bytes)?;
    if plan.schema != "chrono-worktree-reconstruction/v1" {
        return Err("unsupported reconstruction plan schema".into());
    }
    facts::full_oid(&plan.base)?;
    facts::full_oid(&plan.candidate)?;
    let mut paths = BTreeSet::new();
    for change in &plan.changes {
        relative_path(&change.path)?;
        if !paths.insert(&change.path)
            || change.action == Action::Retire
                && change.reason.as_ref().is_none_or(|r| r.trim().is_empty())
        {
            return Err("duplicate reconstruction path or retirement without a reason".into());
        }
    }
    Ok(Input {
        path: path.into(),
        bytes,
        plan,
    })
}
pub(crate) fn execute(
    r: &mut Runner,
    o: &Start,
    config_bytes: &[u8],
    token: &str,
    report: &mut Value,
    input: Input,
) -> Result<(), String> {
    let p = &input.plan;
    report["reconstruction"] = value!({"path":input.path,"sha256":sha256(&input.bytes),"input_bytes":input.bytes,"source_base":p.base,"source_candidate":p.candidate});
    if r.oid(&o.root, "HEAD")? != p.candidate {
        return Err("reconstruction candidate differs from source HEAD".into());
    }
    for revision in [&p.base, &p.candidate] {
        if r.oid(&o.root, &format!("{revision}^{{commit}}"))? != *revision {
            return Err("reconstruction endpoint is not a commit".into());
        }
    }
    let ancestry = r.command(
        &o.root,
        &["merge-base", "--is-ancestor", &p.base, &p.candidate],
    )?;
    if ancestry.exit_code != 0 {
        return Err(format!(
            "reconstruction base ancestry check exited {}",
            ancestry.exit_code
        ));
    }
    let config_path = r.config.host_config.clone();
    let (registrations, _) = start::registrations(r, &o.root, &p.candidate, &config_path)?;
    if !chrono_judge_registration::nonartifact_paths(registrations.config(), &[input.path.clone()])
        .is_empty()
    {
        return Err("reconstruction plan lacks a registered artifact owner".into());
    }
    start::cleanliness(r, &o.root, registrations.config())?;
    let changed = paths(r.git(
        &o.root,
        &[
            "diff",
            "--name-only",
            "--no-renames",
            "--no-ext-diff",
            "--no-textconv",
            "-z",
            &p.base,
            &p.candidate,
            "--",
        ],
    )?)?;
    let declared: BTreeSet<_> = p.changes.iter().map(|c| c.path.clone()).collect();
    if changed != declared {
        return Err("reconstruction choices must cover exactly the original DELTA paths".into());
    }
    let carried: BTreeSet<_> = p
        .changes
        .iter()
        .filter(|c| c.action == Action::Carry)
        .map(|c| c.path.clone())
        .collect();
    let patch = if carried.is_empty() {
        vec![]
    } else {
        let mut argv = vec![
            "--literal-pathspecs",
            "diff",
            "--binary",
            "--full-index",
            "--no-renames",
            "--no-ext-diff",
            "--no-textconv",
            "--src-prefix=a/",
            "--dst-prefix=b/",
            &p.base,
            &p.candidate,
            "--",
        ];
        argv.extend(carried.iter().map(String::as_str));
        r.git(&o.root, &argv)?
    };
    report["reconstruction"]["patch_sha256"] = value!(sha256(&patch));
    report["reconstruction"]["carried_paths"] = value!(carried);
    if r.oid(&o.root, "HEAD")? != p.candidate {
        return Err("source HEAD changed during reconstruction preparation".into());
    }
    // Keep the original creation lock until the staged result is verified.
    start::execute(r, o, config_bytes, token, report, true)?;
    let target = report["destination"]
        .as_str()
        .ok_or("missing destination")?
        .to_string();
    let branch = report["branch_ref"]
        .as_str()
        .ok_or("missing branch")?
        .to_string();
    let base = report["base"]
        .as_str()
        .ok_or("missing fetched base")?
        .to_string();
    report["context"]["candidate"] = Value::Null;
    let target_path = Path::new(&target);
    if !patch.is_empty() {
        let applied = r.input(
            target_path,
            &["apply", "--3way", "--index", "--binary", "-"],
            &patch,
        )?;
        if applied.exit_code != 0 {
            return Err(format!(
                "reconstruction apply exited {}; preserve worktree and lock for AI reconciliation",
                applied.exit_code
            ));
        }
    }
    start::ensure_identity(r, &o.root, target_path, &branch, &base, token)?;
    r.git(
        target_path,
        &[
            "diff",
            "--exit-code",
            "--quiet",
            "--no-ext-diff",
            "--no-textconv",
        ],
    )?;
    let staged = paths(r.git(
        target_path,
        &[
            "diff",
            "--cached",
            "--name-only",
            "--no-renames",
            "--no-ext-diff",
            "--no-textconv",
            "-z",
            &base,
            "--",
        ],
    )?)?;
    if !staged.is_subset(&carried) {
        return Err("reconstruction changed an undeclared path; preserve worktree and lock".into());
    }
    let tree = r.text(target_path, &["write-tree"])?.trim().to_string();
    facts::full_oid(&tree)?;
    if r.oid(&o.root, "HEAD")? != p.candidate {
        return Err("source HEAD changed during reconstruction; preserve both worktrees".into());
    }
    start::cleanliness(r, &o.root, registrations.config())?;
    report["reconstruction"]["index_tree"] = value!(tree);
    report["reconstruction"]["staged_paths"] = value!(staged);
    report["recovery"] = value!(
        "Old work is preserved. Review the staged reconstruction, reconcile semantics, commit, then run the registered canonical check with new evidence."
    );
    start::unlock(r, &o.root, &target, &format!("refs/heads/{branch}"), &base)
}
