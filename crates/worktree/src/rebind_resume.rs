//! Resume only the explicit old plan and preserved pre-effect identity.
use crate::{
    maintenance,
    rebind::Binding,
    recovery::{self, Evidence, ExpectedResult},
    start::Runner,
};
use chrono_harness::{decode, facts, sha256, wire};
use chrono_judge_registration::Registrations;
use serde::Deserialize;
use serde_json::{Value, json as value};
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Intent {
    schema: String,
    operation: String,
    source_root: String,
    source_commit: String,
    config_path: String,
    config_sha256: String,
    registry_digest: String,
    destination: String,
    branch_ref: String,
    head: String,
    index_tree: String,
    metadata_path: String,
    backup_path: String,
    donor_path: String,
    lock_reason: String,
    report_path: String,
    visible_before: String,
    plan_path: String,
    plan_sha256: String,
    expected_inputs_digest: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OriginalPlan {
    schema: String,
    operation: String,
    head: String,
    binding: Binding,
}

pub(crate) struct Continuation {
    evidence: Evidence,
    root: PathBuf,
    invocation_path: String,
    invocation_bytes: Vec<u8>,
}
impl Continuation {
    pub(crate) fn stable(&self) -> Result<(), String> {
        self.evidence.stable()?;
        if recovery::state_bytes(&self.root, &self.invocation_path)? != self.invocation_bytes {
            return Err("rebind continuation plan changed".into());
        }
        Ok(())
    }
    pub(crate) fn matches(&self, report: &Value) -> Result<(), String> {
        for field in [
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
        ] {
            if report[field] != self.evidence.descriptor[field] {
                return Err(format!("rebind continuation differs from retained {field}"));
            }
        }
        self.stable()
    }
}
pub(crate) fn execute(
    r: &mut Runner,
    root: &Path,
    registrations: &Registrations,
    report: &mut Value,
    head: &str,
    intent_path: &str,
    intent_digest: &str,
    result: &ExpectedResult,
    invocation_path: &str,
    invocation_bytes: &[u8],
) -> Result<(), String> {
    maintenance::artifacts(registrations, &[intent_path])?;
    let (bytes, descriptor) = recovery::read_intent(root, intent_path, intent_digest, report)?;
    let intent: Intent = decode(&bytes)?;
    if intent.schema != "chrono-worktree-rebind-intent/v1"
        || intent.operation != "rebind"
        || intent.source_root != root.to_str().ok_or("source root is not UTF-8")?
        || intent.source_commit != report["source_commit"]
        || intent.config_path != report["config_path"]
        || intent.config_sha256 != report["config_sha256"]
        || intent.registry_digest != report["registry_digest"]
        || intent.head != head
        || !intent.lock_reason.starts_with("rebind-")
        || intent.report_path != format!("{}{}.json", r.config.report_directory, intent.lock_reason)
        || intent_path != format!("{}.rebind-intent.json", intent.report_path)
        || ![
            &intent.destination,
            &intent.metadata_path,
            &intent.backup_path,
            &intent.donor_path,
        ]
        .iter()
        .all(|s| Path::new(s).is_absolute())
        || intent.branch_ref.is_empty()
    {
        return Err("rebind intent/source/configuration identity mismatch".into());
    }
    facts::full_oid(&intent.source_commit)?;
    facts::full_oid(&intent.head)?;
    facts::full_oid(&intent.index_tree)?;
    maintenance::artifacts(registrations, &[&intent.plan_path, &intent.report_path])?;
    if invocation_path == intent.plan_path
        || invocation_path == intent.report_path
        || invocation_path == intent_path
    {
        return Err("continuation plan must have its own path".into());
    }
    let original_bytes = recovery::state_bytes(root, &intent.plan_path)?;
    if sha256(&original_bytes) != intent.plan_sha256 {
        return Err("original rebind plan digest mismatch".into());
    }
    let original: OriginalPlan = decode(&original_bytes)?;
    let expected = original
        .binding
        .expected
        .as_ref()
        .ok_or("original rebind plan has no expected identities")?;
    if original.schema != "chrono-worktree-maintenance/v1"
        || original.operation != "rebind"
        || original.head != head
        || expected.visible != intent.visible_before
        || wire::digest(&value!(expected))? != intent.expected_inputs_digest
    {
        return Err("original rebind plan differs from retained intent".into());
    }
    let evidence = recovery::observe_rebind_result(
        root,
        intent_path,
        bytes,
        descriptor,
        &intent.report_path,
        result,
        report,
    )?;
    report["original_plan"] =
        value!({"path":intent.plan_path,"sha256":intent.plan_sha256,"input_bytes":original_bytes});
    report["rebind_intent"] = value!({"path":intent_path,"sha256":intent_digest});
    report["invocation_lock_reason"] = report["lock_reason"].clone();
    report["lock_reason"] = value!(intent.lock_reason);
    let continuation = Continuation {
        evidence,
        root: root.into(),
        invocation_path: invocation_path.into(),
        invocation_bytes: invocation_bytes.into(),
    };
    let outcome = report["original_outcome"].clone();
    let completed = original.binding.execute(
        r,
        root,
        registrations,
        &intent.lock_reason,
        report,
        head,
        &intent.plan_path,
        &original_bytes,
        false,
        Some(&continuation),
    );
    report["original_outcome"] = outcome;
    completed
}
