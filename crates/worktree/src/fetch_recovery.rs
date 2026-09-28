//! A pre-fetch identity does not assert that fetching completed or identify its result.
use crate::recovery::{self, Evidence, ExpectedResult};
use chrono_harness::{decode, facts};
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Intent {
    schema: String,
    operation: String,
    source_root: String,
    source_commit: String,
    source_registry_digest: String,
    config_path: String,
    config_sha256: String,
    fetch_ref: String,
    remote: String,
    target_ref: String,
    lock_reason: String,
    report_path: String,
}

pub(crate) fn publish(root: &Path, report: &mut Value) -> Result<(), String> {
    recovery::publish_fields(
        root,
        report,
        "chrono-worktree-fetch-intent/v1",
        ".fetch-intent.json",
        "fetch_intent",
        &[
            "operation",
            "source_root",
            "source_commit",
            "source_registry_digest",
            "config_path",
            "config_sha256",
            "fetch_ref",
            "remote",
            "target_ref",
            "lock_reason",
            "report_path",
        ],
    )
}

pub(crate) fn inspect(
    root: &Path,
    config_path: &str,
    config_bytes: &[u8],
    path: &str,
    digest: &str,
    expected: &ExpectedResult,
    report: &mut Value,
) -> Result<Evidence, String> {
    let (bytes, descriptor) = recovery::read_intent(root, path, digest, report)?;
    let intent: Intent = decode(&bytes)?;
    let config: crate::Config = decode(config_bytes)?;
    if intent.schema != "chrono-worktree-fetch-intent/v1"
        || !matches!(intent.operation.as_str(), "start" | "reconstruct")
        || intent.source_root != root.to_str().ok_or("source root is not UTF-8")?
        || intent.config_path != config_path
        || intent.config_sha256 != chrono_harness::sha256(config_bytes)
        || intent.source_registry_digest.is_empty()
        || intent.remote != config.remote
        || !intent.target_ref.starts_with("refs/heads/")
        || !intent
            .lock_reason
            .starts_with(&format!("{}-", intent.operation))
        || intent.fetch_ref != format!("refs/chrono-harness/fetch/{}", intent.lock_reason)
        || intent.report_path != format!("{}{}.json", config.report_directory, intent.lock_reason)
        || path != format!("{}.fetch-intent.json", intent.report_path)
    {
        return Err("interrupted-fetch/configuration identity mismatch".into());
    }
    facts::full_oid(&intent.source_commit)?;
    recovery::observe_result(
        root,
        path,
        bytes,
        descriptor,
        &intent.report_path,
        expected,
        report,
    )
}
