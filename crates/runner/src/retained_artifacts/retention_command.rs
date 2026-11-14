//! Explicit maintenance, migration and consumer requests for the retention owner.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use crate::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub(crate) fn within_bound(length: u64, limit: u64) -> (fits: bool)
    ensures fits == (length <= limit),
{
    length <= limit
}
}

#[cfg(not(verus_keep_ghost))]
use super::retention::{Consumer, Inventory, OriginalIdentity, POLICY_PATH};
#[cfg(not(verus_keep_ghost))]
use crate::{CliOutput, decode, no_symlink_parents, sha256};
#[cfg(not(verus_keep_ghost))]
use serde::Deserialize;
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{fs, io::Read, path::Path};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
pub(super) struct Evidence {
    pub path: String,
    pub sha256: String,
}
#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
enum Request {
    Maintain {
        originals: Option<Vec<String>>,
    },
    MigratePolicy {
        previous_policy: Evidence,
        originals: Option<Vec<String>>,
    },
    ProtectConsumer {
        declaration: Evidence,
    },
    ReleaseConsumer {
        declaration: Evidence,
        receipt: Evidence,
        pointer: String,
    },
    ReleasePreserved {
        original: OriginalIdentity,
        roles: Vec<String>,
        survivor_root: std::path::PathBuf,
        survivor_path: String,
        sha256: String,
        length: u64,
        current_consumers_released: bool,
        reason: String,
    },
    AcknowledgeDelivery {
        report: String,
        report_sha256: String,
        artifact_id: u64,
        artifact_digest: String,
    },
    RecoverReport {
        report: crate::prepared::Original,
    },
}
#[cfg(not(verus_keep_ghost))]
fn evidence(root: &Path, e: &Evidence) -> Result<Vec<u8>, String> {
    let path = no_symlink_parents(root, &e.path)?;
    let m = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if !m.is_file() || !within_bound(m.len(), 1024 * 1024) {
        return Err("migration evidence must be a bounded regular file".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if !within_bound(bytes.len() as u64, 1024 * 1024) || sha256(&bytes) != e.sha256 {
        return Err("migration producer/evidence identity differs".into());
    }
    Ok(bytes)
}
#[cfg(not(verus_keep_ghost))]
fn bounded_json(root: &Path, path: &str, limit: u64) -> Result<(Value, Vec<u8>), String> {
    let path = no_symlink_parents(root, path)?;
    let metadata = fs::symlink_metadata(&path)
        .map_err(|e| format!("delivery input {}: {e}", path.display()))?;
    if !metadata.is_file() || !within_bound(metadata.len(), limit) {
        return Err("delivery input must be a bounded regular file".into());
    }
    let mut raw = Vec::new();
    fs::File::open(&path)
        .map_err(|e| format!("delivery input {}: {e}", path.display()))?
        .take(limit + 1)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    if !within_bound(raw.len() as u64, limit) {
        return Err("delivery input byte bound".into());
    }
    Ok((decode(&raw)?, raw))
}
#[cfg(not(verus_keep_ghost))]
fn execute(root: &Path, request: Request) -> Result<Value, String> {
    let inventory = Inventory::adopted(root)?.ok_or("host has not adopted output retention")?;
    match request {
        Request::RecoverReport { report } => crate::scoped_report::recover(root, &report),
        Request::ProtectConsumer { declaration } => {
            let consumer = consumer(root, &declaration)?;
            inventory.protect_consumer(&consumer.consumer, &consumer.originals)?;
            Ok(
                json!({"schema":"chrono-retention-consumer-protection/v1","consumer":consumer.consumer,"purpose":consumer.purpose,"declaration_sha256":declaration.sha256,"original_count":consumer.originals.len(),"effects":"consumer-root-only","completion":"not-established"}),
            )
        }
        Request::ReleaseConsumer {
            declaration,
            receipt,
            pointer,
        } => {
            let consumer = consumer(root, &declaration)?;
            outside_originals(&receipt.path, &consumer.originals)?;
            let value: Value = decode(&evidence(root, &receipt)?)?;
            if value.pointer(&pointer)
                != Some(
                    &json!({"consumer":consumer.consumer,"declaration_sha256":declaration.sha256,"state":"released"}),
                )
            {
                return Err("responsible consumer has not released this exact declaration".into());
            }
            inventory.release_consumer(
                &consumer.consumer,
                &consumer.originals,
                &declaration.sha256,
                &receipt.sha256,
            )
        }
        Request::Maintain { originals } => match originals {
            Some(originals) => inventory.maintain_originals(&originals),
            None => inventory.maintain_explicit(),
        },
        Request::ReleasePreserved {
            original,
            roles,
            survivor_root,
            survivor_path,
            sha256,
            length,
            current_consumers_released,
            reason,
        } => inventory.release_preserved(
            &original,
            &roles,
            &survivor_root,
            &survivor_path,
            &sha256,
            length,
            current_consumers_released,
            &reason,
        ),
        Request::MigratePolicy {
            previous_policy,
            originals,
        } => inventory.migrate_policy(&evidence(root, &previous_policy)?, originals.as_deref()),
        Request::AcknowledgeDelivery {
            report,
            report_sha256,
            artifact_id,
            artifact_digest,
        } => inventory.acknowledge_delivery(&report, &report_sha256, artifact_id, &artifact_digest),
    }
}
#[cfg(not(verus_keep_ghost))]
fn outside_originals(path: &str, originals: &[OriginalIdentity]) -> Result<(), String> {
    crate::relative_path(path)?;
    if originals
        .iter()
        .any(|o| path == o.path || path.starts_with(&format!("{}/", o.path)))
    {
        return Err(
            "consumer declaration/release evidence must remain outside disposable originals".into(),
        );
    }
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
fn consumer(root: &Path, declaration: &Evidence) -> Result<Consumer, String> {
    let consumer: Consumer = decode(&evidence(root, declaration)?)?;
    if consumer.schema != "chrono-retention-consumer/v1"
        || consumer.purpose.trim().is_empty()
        || consumer.purpose.len() > 4096
    {
        return Err("consumer schema/purpose missing or bounded capacity exceeded".into());
    }
    outside_originals(&declaration.path, &consumer.originals)?;
    Ok(consumer)
}
#[cfg(not(verus_keep_ghost))]
pub fn command(args: &[&str]) -> CliOutput {
    let result = (|| {
        if let [
            "--host-root",
            host,
            "--delivered-check",
            profile,
            "--scope",
            scope,
            "--artifact-id-env",
            id_env,
            "--artifact-digest-env",
            digest_env,
        ] = args
        {
            let root = fs::canonicalize(host).map_err(|e| e.to_string())?;
            let _activity = super::retention::read_scope(&root)?;
            let (config, _) = bounded_json(&root, profile, 1024 * 1024)?;
            let path = if *scope == "collect" {
                config["execution_units"]["report_path"]
                    .as_str()
                    .or(config["report_path"].as_str())
            } else if *scope == "all" {
                config["report_path"].as_str()
            } else {
                config["execution_units"]["units"][scope]["report_path"]
                    .as_str()
                    .or(config["units"][scope]["report_path"].as_str())
                    .or(config["policy"]["units"][scope]["report_path"].as_str())
            }
            .ok_or("delivered report slot not registered")?;
            let (report, _) = bounded_json(&root, path, 64 * 1024 * 1024).map_err(|e| {
                format!("delivered check report unavailable at {path}: {e}; upload does not establish report production; delivery not acknowledged, original outcomes and roots preserved")
            })?;
            let (original, original_raw) = if report["schema"] == crate::units::REPORT_REFERENCE {
                let reference: crate::units::ReportReference = serde_json::from_value(report)
                    .map_err(|e| format!("delivery report reference: {e}"))?;
                let raw = crate::prepared::read_original(&root, &reference.original, None)
                    .map_err(|e| format!("delivery original {} verification failed: {e}; delivery not acknowledged, original outcomes and roots preserved", reference.original.path))?;
                (reference.original.path, raw)
            } else {
                let original = report["retained_report"]
                    .as_str()
                    .or(report["report_path"].as_str())
                    .ok_or("delivered original report address")?
                    .to_owned();
                let (_, raw) = bounded_json(&root, &original, 64 * 1024 * 1024)?;
                (original, raw)
            };
            let artifact_id = std::env::var(id_env)
                .map_err(|_| "upload action artifact id missing")?
                .parse()
                .map_err(|_| "upload action artifact id invalid")?;
            let artifact_digest =
                std::env::var(digest_env).map_err(|_| "upload action artifact digest missing")?;
            return execute(
                &root,
                Request::AcknowledgeDelivery {
                    report: original,
                    report_sha256: sha256(&original_raw),
                    artifact_id,
                    artifact_digest: artifact_digest.trim_start_matches("sha256:").into(),
                },
            );
        }
        let ["--host-root", host, "--request", request] = args else {
            return Err("use retention --host-root H --request P".into());
        };
        let root = fs::canonicalize(host).map_err(|e| e.to_string())?;
        let path = no_symlink_parents(&root, request)?;
        let mut bytes = Vec::new();
        fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(65537)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if !within_bound(bytes.len() as u64, 65536) {
            return Err("retention request bound".into());
        }
        execute(&root, decode(&bytes)?)
    })();
    match result {
        Ok(value) => CliOutput {
            exit_code: 0,
            stdout: format!("{value}\n"),
            stderr: String::new(),
        },
        Err(error) => CliOutput {
            exit_code: 2,
            stdout: String::new(),
            stderr: format!("E_RETENTION: {error}; policy {POLICY_PATH}\n"),
        },
    }
}
