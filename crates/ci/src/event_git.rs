//! Versioned event acquisition; the selected reader owns process evidence.
use crate::{Config, full_oid};
use chrono_harness::facts::Reader;
use serde_json::{Value, json};
use std::{path::Path, process::Command};

pub(crate) struct EventGit<'a> {
    root: &'a Path,
    reader: Option<Reader>,
}
impl<'a> EventGit<'a> {
    pub(crate) fn new(root: &'a Path, c: &Config, candidate: &str) -> Result<Self, String> {
        let reader = if let Some(path) = &c.facts_config {
            if !full_oid(candidate) {
                return Err("invalid full candidate OID".into());
            }
            let reader = Reader::for_config(root, path)?;
            if !reader.is_bound() {
                return Err("CI v3 requires a full v3 facts_config".into());
            }
            reader.verify_config(root, candidate)?;
            Some(reader)
        } else {
            None
        };
        let facts = Self { root, reader };
        if facts.reader.is_some()
            && facts
                .git(&["rev-parse", "HEAD"])
                .map_err(|e| facts.error(e))?
                .trim()
                != candidate
        {
            return Err(facts.error("checkout is not exact event candidate".into()));
        }
        Ok(facts)
    }
    pub(crate) fn git(&self, args: &[&str]) -> Result<String, String> {
        if let Some(reader) = &self.reader {
            return String::from_utf8(reader.git(self.root, args)?)
                .map_err(|_| "git output not UTF-8".into());
        }
        legacy_git(self.root, args)
    }
    pub(crate) fn require_commit(&self, oid: &str) -> Result<(), String> {
        if let Some(reader) = &self.reader {
            if !reader.commit_available(self.root, oid)? {
                self.git(&[
                    "fetch",
                    "--no-tags",
                    "--depth=2",
                    "--filter=blob:none",
                    "origin",
                    oid,
                ])?;
                if !reader.commit_available(self.root, oid)? {
                    return Err("fetched commit remains missing".into());
                }
            }
            if self
                .git(&["rev-parse", &format!("{oid}^{{commit}}")])?
                .trim()
                != oid
            {
                return Err("commit identity mismatch".into());
            }
            Ok(())
        } else {
            legacy_require_commit(self.root, oid)
        }
    }
    pub(crate) fn remote_baseline(&self, reference: &str) -> Result<String, String> {
        self.git(&["check-ref-format", reference])?;
        let observed = self.git(&["ls-remote", "--refs", "origin", reference])?;
        let fields: Vec<_> = observed.split_whitespace().collect();
        if fields.len() != 2 || fields[1] != reference || !full_oid(fields[0]) {
            return Err(format!(
                "missing or ambiguous remote baseline ref: {reference}"
            ));
        }
        let base = fields[0].to_owned();
        self.require_commit(&base)?;
        Ok(base)
    }
    pub(crate) fn changed_paths(
        &self,
        base: Option<&str>,
        candidate: &str,
    ) -> Result<Vec<String>, String> {
        let tree = |oid| match &self.reader {
            Some(reader) => reader.tree(self.root, oid),
            None => chrono_harness::facts::tree(self.root, oid),
        };
        let candidate = tree(candidate)?;
        Ok(match base {
            Some(base) => chrono_harness::facts::delta(&tree(base)?, &candidate)
                .into_iter()
                .map(|d| d.path)
                .collect(),
            None => candidate.keys().cloned().collect(),
        })
    }
    pub(crate) fn parents(&self, candidate: &str) -> Result<Vec<String>, String> {
        match &self.reader {
            Some(reader) => reader.parents(self.root, candidate),
            None => chrono_harness::facts::parents(self.root, candidate),
        }
    }
    pub(crate) fn record(&self, context: &mut Value) {
        if let Some(reader) = &self.reader {
            context["git_facts"] = reader.observation();
        }
    }
    pub(crate) fn error(&self, message: String) -> String {
        match &self.reader {
            Some(reader) => format!(
                "E_CI_GIT: {}",
                json!({"message":message,"git_facts":reader.observation()})
            ),
            None => message,
        }
    }
}

fn legacy_git(root: &Path, args: &[&str]) -> Result<String, String> {
    let o = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if !o.status.success() {
        return Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&o.stderr)
        ));
    }
    String::from_utf8(o.stdout).map_err(|_| "git output not UTF-8".into())
}
fn legacy_require_commit(root: &Path, oid: &str) -> Result<(), String> {
    if !full_oid(oid) {
        return Err(format!("invalid full OID: {oid}"));
    }
    if legacy_git(root, &["cat-file", "-e", &format!("{oid}^{{commit}}")]).is_err() {
        legacy_git(
            root,
            &[
                "fetch",
                "--no-tags",
                "--depth=2",
                "--filter=blob:none",
                "origin",
                oid,
            ],
        )?;
    }
    if legacy_git(root, &["rev-parse", &format!("{oid}^{{commit}}")])?.trim() != oid {
        return Err("commit identity mismatch".into());
    }
    Ok(())
}
