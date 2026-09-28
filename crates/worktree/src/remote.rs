//! Explicit remote branch disposal with saved-work checks and an exact push lease.
use crate::{
    maintenance::{Retention, saved_commit},
    start::Runner,
};
use chrono_harness::facts;
use chrono_judge_registration::Registrations;
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

pub(crate) struct Cleanup {
    pub(crate) head: String,
    pub(crate) retained_commit: String,
    pub(crate) branch: String,
    pub(crate) expected_url: String,
    pub(crate) retained_ref: String,
    pub(crate) retention: Retention,
    pub(crate) allow_absent_ref: bool,
}

impl Cleanup {
    fn endpoint(&self, r: &mut Runner, root: &Path) -> Result<(), String> {
        let remote = r.config.remote.clone();
        let urls = r.text(root, &["remote", "get-url", "--push", "--all", &remote])?;
        if urls.lines().collect::<Vec<_>>() != [self.expected_url.as_str()] {
            return Err("remote must have exactly the expected push URL".into());
        }
        Ok(())
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

    fn observed(
        &self,
        r: &mut Runner,
        root: &Path,
        branch_ref: &str,
    ) -> Result<Option<String>, String> {
        // One endpoint supplies both the disposal and retention observations.
        let text = r.text(
            root,
            &[
                "ls-remote",
                "--symref",
                "--",
                &self.expected_url,
                branch_ref,
                &self.retained_ref,
            ],
        )?;
        let mut refs = BTreeMap::new();
        for line in text.lines() {
            let (oid, name) = line
                .split_once('\t')
                .ok_or("invalid remote ref observation")?;
            facts::full_oid(oid)?; // Also rejects a symbolic-ref advertisement.
            if ![branch_ref, &self.retained_ref].contains(&name) || refs.insert(name, oid).is_some()
            {
                return Err("unexpected or duplicate remote ref observation".into());
            }
        }
        if refs.get(self.retained_ref.as_str()).copied() != Some(&self.retained_commit) {
            return Err("remote target does not match retained commit".into());
        }
        Ok(refs.get(branch_ref).map(|s| (*s).to_string()))
    }

    pub(crate) fn execute(
        &self,
        r: &mut Runner,
        root: &Path,
        registrations: &Registrations,
        report: &mut Value,
        stable: impl Fn() -> Result<(), String>,
    ) -> Result<(), String> {
        report["remote_branch_removed"] = json!(false);
        report["remote_branch_removal"] = json!("not-attempted");
        let branch_ref = format!("refs/heads/{}", self.branch);
        r.git(root, &["check-ref-format", &branch_ref])?;
        r.git(root, &["check-ref-format", &self.retained_ref])?;
        let workflow = registrations.workflow();
        let target = format!(
            "refs/heads/{}",
            workflow["target_branch"]
                .as_str()
                .ok_or("missing workflow target")?
        );
        if self.retained_ref != target
            || branch_ref == target
            || !["feature_prefix", "integration_prefix"].iter().any(|k| {
                workflow[k]
                    .as_str()
                    .is_some_and(|p| self.branch.starts_with(p))
            })
        {
            return Err("remote cleanup requires a registered work branch and the distinct configured target".into());
        }
        if self.expected_url.is_empty()
            || self.expected_url.starts_with('-')
            || self.expected_url.contains(['\0', '\n', '\r'])
        {
            return Err("invalid expected remote push URL".into());
        }
        report["remote"] = json!(r.config.remote);
        report["remote_url"] = json!(self.expected_url);
        report["branch_ref"] = json!(self.branch);
        report["head"] = json!(self.head);
        report["retained_ref"] = json!(self.retained_ref);
        report["retained_commit"] = json!(self.retained_commit);
        self.endpoint(r, root)?;
        self.saved(r, root)?;
        let observed = self.observed(r, root, &branch_ref)?;
        let absent = match observed {
            None if self.allow_absent_ref => true,
            Some(ref oid) if oid == &self.head => false,
            _ => {
                return Err("remote branch changed or absent without an explicit retry plan".into());
            }
        };
        stable()?;
        self.saved(r, root)?;
        self.endpoint(r, root)?;
        if !absent {
            // Explicit URL and refspec avoid configured push refspecs/multiple push URLs.
            // Only the branch's expected OID is leased, not the remote target's future state.
            report["remote_branch_removal"] = json!("attempted-unverified");
            r.git(
                root,
                &[
                    "push",
                    "--porcelain",
                    "--no-follow-tags",
                    "--recurse-submodules=no",
                    &format!("--force-with-lease={branch_ref}:{}", self.head),
                    "--",
                    &self.expected_url,
                    &format!(":{branch_ref}"),
                ],
            )?;
        }
        if self.observed(r, root, &branch_ref)?.is_some() {
            return Err("remote branch absence was not verified".into());
        }
        self.saved(r, root)?;
        self.endpoint(r, root)?;
        stable()?;
        report["remote_branch_removed"] = json!(!absent);
        report["remote_branch_removal"] = json!(if absent {
            "already-absent"
        } else {
            "verified-absent"
        });
        Ok(())
    }
}
