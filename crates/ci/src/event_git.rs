//! Bound event acquisition; the selected reader owns process evidence.
use crate::{Config, full_oid};
use chrono_harness::facts::Reader;
use serde_json::{Value, json};
use std::path::Path;

pub(crate) struct EventGit<'a> {
    root: &'a Path,
    reader: Reader,
}
impl<'a> EventGit<'a> {
    pub(crate) fn new(root: &'a Path, c: &Config, candidate: &str) -> Result<Self, String> {
        if !full_oid(candidate) {
            return Err("invalid full candidate OID".into());
        }
        let path = c
            .facts_config
            .as_deref()
            .ok_or("CI requires facts_config")?;
        let reader = Reader::for_config(root, path)?;
        if !reader.is_bound() {
            return Err("CI requires a full v3 facts_config".into());
        }
        reader.verify_config(root, candidate)?;
        let facts = Self { root, reader };
        if facts
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
        String::from_utf8(self.reader.git(self.root, args)?)
            .map_err(|_| "git output not UTF-8".into())
    }
    pub(crate) fn require_commit(&self, oid: &str) -> Result<(), String> {
        if !self.reader.commit_available(self.root, oid)? {
            self.git(&[
                "fetch",
                "--no-tags",
                "--depth=2",
                "--filter=blob:none",
                "origin",
                oid,
            ])?;
            if !self.reader.commit_available(self.root, oid)? {
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
        let candidate = self.reader.tree(self.root, candidate)?;
        Ok(match base {
            Some(base) => {
                chrono_harness::facts::delta(&self.reader.tree(self.root, base)?, &candidate)
                    .into_iter()
                    .map(|d| d.path)
                    .collect()
            }
            None => candidate.keys().cloned().collect(),
        })
    }
    pub(crate) fn parents(&self, candidate: &str) -> Result<Vec<String>, String> {
        self.reader.parents(self.root, candidate)
    }
    pub(crate) fn record(&self, context: &mut Value) {
        context["git_facts"] = self.reader.observation();
    }
    pub(crate) fn error(&self, message: String) -> String {
        format!(
            "E_CI_GIT: {}",
            json!({"message":message,"git_facts":self.reader.observation()})
        )
    }
}
