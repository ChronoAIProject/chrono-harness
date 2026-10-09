//! Finite current custody of released file aliases; original outcomes remain data.
use crate::{
    automatic::EvidenceExclusion,
    maintenance,
    recovery::state_bytes,
    start::{self, Runner},
};
use chrono_harness::{file_identity, no_symlink_parents, relative_path, sha256};
use chrono_judge_registration::Registrations;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Original {
    path: String,
    sha256: String,
    length: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Alias {
    original: Original,
    survivor: String,
    owner: String,
    reason: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Custody {
    schema: String,
    root: PathBuf,
    branch: String,
    current_consumers_released: bool,
    reason: String,
    authority: Original,
    exclusion: EvidenceExclusion,
    objects: Vec<Alias>,
}

fn state_member(root: &Path, path: &str) -> Result<PathBuf, String> {
    relative_path(path)?;
    if !path.starts_with(".chrono-harness/state/") {
        return Err("file custody selects only declared evidence state members".into());
    }
    no_symlink_parents(root, path)
}
// Keep path identity, content identity and read stability separate from release authority.
fn signature(meta: &fs::Metadata) -> Result<String, String> {
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err("evidence member must be a physical regular file".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(format!(
            "{}:{}:{}:{}:{}:{}:{}:{}",
            meta.dev(),
            meta.ino(),
            meta.len(),
            meta.mtime(),
            meta.mtime_nsec(),
            meta.ctime(),
            meta.ctime_nsec(),
            meta.mode()
        ))
    }
    #[cfg(not(unix))]
    {
        Err("file custody identity unsupported on this platform".into())
    }
}
fn verified(root: &Path, path: &str, expected: &Original) -> Result<(PathBuf, String), String> {
    let physical = state_member(root, path)?;
    let before = signature(&fs::symlink_metadata(&physical).map_err(|e| e.to_string())?)?;
    let file = fs::File::open(&physical).map_err(|e| e.to_string())?;
    if signature(&file.metadata().map_err(|e| e.to_string())?)? != before {
        return Err(format!("evidence file changed before read: {path}"));
    }
    if chrono_harness::copy_hashed(&file, std::io::sink())?
        != (expected.sha256.clone(), expected.length)
        || signature(&file.metadata().map_err(|e| e.to_string())?)? != before
        || signature(&fs::symlink_metadata(&physical).map_err(|e| e.to_string())?)? != before
    {
        return Err(format!(
            "evidence identity or read stability mismatch: {path}"
        ));
    }
    Ok((physical, before))
}
fn journal(file: &mut fs::File, record: &Value) -> Result<(), String> {
    serde_json::to_writer(&mut *file, record).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())
}
impl Custody {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn execute(
        self,
        r: &mut Runner,
        root: &Path,
        registrations: &Registrations,
        report: &mut Value,
        head: &str,
        plan_path: &str,
        bytes: &[u8],
        config_path: &str,
        config_bytes: &[u8],
    ) -> Result<(), String> {
        if self.schema != "chrono-evidence-custody/v1"
            || self.root != root
            || !self.current_consumers_released
            || self.reason.trim().is_empty()
            || self.objects.is_empty()
            || self.branch.is_empty()
        {
            return Err("current evidence custody requires exact root, finite objects and explicit consumer release".into());
        }
        let mut selected = BTreeSet::new();
        for object in &self.objects {
            if !selected.insert(object.original.path.as_str())
                || object.owner.trim().is_empty()
                || object.reason.trim().is_empty()
                || !chrono_harness::wire::is_digest(&object.original.sha256)
            {
                return Err("repeated or incomplete evidence custody member".into());
            }
            state_member(root, &object.original.path)?;
            state_member(root, &object.survivor)?;
            maintenance::artifacts(registrations, &[&object.original.path, &object.survivor])?;
        }
        let report_path = report["report_path"]
            .as_str()
            .ok_or("missing attempt path")?
            .to_owned();
        if selected.contains(self.authority.path.as_str())
            || selected.contains(plan_path)
            || selected.contains(report_path.as_str())
            || self
                .objects
                .iter()
                .any(|o| selected.contains(o.survivor.as_str()))
        {
            return Err("custody would retire a survivor or its own authority/input/output".into());
        }
        maintenance::artifacts(registrations, &[&self.authority.path])?;
        verified(root, &self.authority.path, &self.authority)?;
        let guard = self.exclusion.acquire(r, root)?;
        let check = |r: &mut Runner| -> Result<(), String> {
            let observed = r.checkout_identity(root)?;
            if observed.top != root
                || observed.head != head
                || observed.branch != self.branch
                || state_bytes(root, plan_path)? != bytes
                || crate::configuration(root, config_path)?.1 != config_bytes
            {
                return Err("evidence custody checkout, declaration or policy changed".into());
            }
            start::cleanliness_at(r, root, head, registrations.config())?;
            guard.stable()
        };
        check(r)?;
        let tree = r.tree(root, head)?;
        let index = start::paths(r.git(root, &["ls-files", "-z"])?)?;
        if selected
            .iter()
            .any(|p| tree.contains_key(*p) || index.iter().any(|i| i.as_str() == *p))
        {
            return Err("file custody cannot retire committed or staged source".into());
        }
        let effects_path = format!("{report_path}.effects.jsonl");
        maintenance::artifacts(registrations, &[&effects_path])?;
        let mut effects = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(state_member(root, &effects_path)?)
            .map_err(|e| e.to_string())?;
        report["evidence_retirement"] = json!({"declaration":report["maintenance_plan"],
            "authority":self.authority,"effects_path":effects_path,"selected":self.objects.len(),
            "removed":0,"observed_absent":0,"failed":0,"removed_logical_bytes":0,
            "historical_producer_outcome":"unchanged","terminal_handoff":"not-claimed",
            "physical_release_bytes":null});
        journal(
            &mut effects,
            &json!({"event":"attempt-intent","head":head,
            "declaration":report["maintenance_plan"],"authority":self.authority}),
        )?;
        let mut removed = 0u64;
        let mut absent = 0u64;
        let mut failed = 0u64;
        let mut logical = 0u64;
        for object in self.objects {
            guard.capabilities_stable()?;
            let result = (|| -> Result<&str, String> {
                let (survivor, survivor_id) = verified(root, &object.survivor, &object.original)?;
                let path = state_member(root, &object.original.path)?;
                match fs::symlink_metadata(&path) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        return Ok("observed-absent");
                    }
                    Err(e) => return Err(e.to_string()),
                    Ok(_) => (),
                }
                let (path, alias_id) = verified(root, &object.original.path, &object.original)?;
                journal(
                    &mut effects,
                    &json!({"event":"removal-intent","original":object.original,
                    "survivor":object.survivor,"alias_identity":alias_id,"survivor_identity":survivor_id}),
                )?;
                guard.capabilities_stable()?;
                if signature(&fs::symlink_metadata(&path).map_err(|e| e.to_string())?)? != alias_id
                    || signature(&fs::symlink_metadata(&survivor).map_err(|e| e.to_string())?)?
                        != survivor_id
                {
                    return Err("alias or survivor changed immediately before removal".into());
                }
                fs::remove_file(&path).map_err(|e| e.to_string())?;
                match fs::symlink_metadata(&path) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                    Err(e) => return Err(e.to_string()),
                    Ok(_) => return Err("retired alias remains present".into()),
                }
                if signature(&fs::symlink_metadata(survivor).map_err(|e| e.to_string())?)?
                    != survivor_id
                {
                    return Err(
                        "survivor changed after alias removal; inspect partial effects".into(),
                    );
                }
                Ok("removed-verified-absent")
            })();
            match &result {
                Ok(&"removed-verified-absent") => {
                    removed += 1;
                    logical += object.original.length;
                }
                Ok(_) => absent += 1,
                Err(_) => failed += 1,
            }
            journal(
                &mut effects,
                &json!({"event":"object-result","path":object.original.path,
                "status":result.as_ref().ok(),"error":result.as_ref().err()}),
            )?;
            report["evidence_retirement"]["removed"] = json!(removed);
            report["evidence_retirement"]["observed_absent"] = json!(absent);
            report["evidence_retirement"]["failed"] = json!(failed);
            report["evidence_retirement"]["removed_logical_bytes"] = json!(logical);
        }
        check(r)?;
        if file_identity(&state_member(root, &self.authority.path)?)?
            != (self.authority.sha256, self.authority.length)
        {
            return Err("custody authority changed during disposal".into());
        }
        journal(
            &mut effects,
            &json!({"event":"attempt-complete","removed":removed,
            "observed_absent":absent,"failed":failed,"removed_logical_bytes":logical}),
        )?;
        if failed > 0 {
            return Err(format!(
                "{failed} evidence objects unresolved; independent effects retained in {effects_path}"
            ));
        }
        Ok(())
    }
}
