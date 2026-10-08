//! Host-selected historical owner migration through the existing retention owner.
use super::retention::{Inventory, OriginalIdentity, POLICY_PATH};
use crate::{CliOutput, decode, no_symlink_parents, ownership::Lease, sha256};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{fs, io::Read, path::Path};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Evidence {
    pub path: String,
    pub sha256: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Release {
    pub receipt: Evidence,
    pub pointer: String,
    pub expected: Value,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Historical {
    pub producer: String,
    pub path: String,
    pub producer_source: Evidence,
    pub receipt: Evidence,
    pub receipt_schema: String,
    pub output_pointer: Option<String>,
    pub outcome_pointer: String,
    pub owner_lease: String,
    pub owner_lease_identity: String,
    pub roots: Vec<String>,
    pub release: Option<Release>,
}
/// A host/consumer declaration, outside its disposable originals. No discovery.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Consumer {
    schema: String,
    consumer: String,
    purpose: String,
    originals: Vec<OriginalIdentity>,
}
#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
enum Request {
    Maintain,
    MigratePolicy {
        previous_policy: Evidence,
    },
    Enroll {
        enrollment: String,
        expected_identity: String,
    },
    Release {
        enrollment: String,
    },
    ProtectConsumer {
        declaration: Evidence,
    },
    ReleaseConsumer {
        declaration: Evidence,
        receipt: Evidence,
        pointer: String,
    },
    AcknowledgeDelivery {
        report: String,
        report_sha256: String,
        artifact_id: u64,
        artifact_digest: String,
    },
}
fn evidence(root: &Path, e: &Evidence) -> Result<Vec<u8>, String> {
    let path = no_symlink_parents(root, &e.path)?;
    let m = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if !m.is_file() || m.len() > 1024 * 1024 {
        return Err("migration evidence must be a bounded regular file".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 1024 * 1024 || sha256(&bytes) != e.sha256 {
        return Err("migration producer/evidence identity differs".into());
    }
    Ok(bytes)
}
fn owner(root: &Path, rule: &Historical) -> Result<Lease, String> {
    let path = no_symlink_parents(root, &rule.owner_lease)?;
    Lease::acquire(&path, Some(&rule.owner_lease_identity), false, true, None)?
        .ok_or("historical producer/readers are live; original preserved".into())
}
fn bounded_json(root: &Path, path: &str, limit: u64) -> Result<(Value, Vec<u8>), String> {
    let path = no_symlink_parents(root, path)?;
    let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err("delivery input must be a bounded regular file".into());
    }
    let mut raw = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    if raw.len() as u64 > limit {
        return Err("delivery input byte bound".into());
    }
    Ok((decode(&raw)?, raw))
}
fn execute(root: &Path, request: Request) -> Result<Value, String> {
    let inventory = Inventory::adopted(root)?.ok_or("host has not adopted output retention")?;
    match request {
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
        Request::Maintain => inventory.maintain(),
        Request::MigratePolicy { previous_policy } => {
            inventory.migrate_policy(&evidence(root, &previous_policy)?)
        }
        Request::Enroll {
            enrollment,
            expected_identity,
        } => {
            let rule = inventory.historical(&enrollment)?.clone();
            for evidence in [&rule.producer_source, &rule.receipt]
                .into_iter()
                .chain(rule.release.as_ref().map(|r| &r.receipt))
            {
                crate::relative_path(&evidence.path)?;
                if evidence.path == rule.path
                    || evidence.path.starts_with(&format!("{}/", rule.path))
                {
                    return Err("historical producer/release evidence must remain outside the disposable output".into());
                }
            }
            let exclusion = owner(root, &rule)?;
            evidence(root, &rule.producer_source)?;
            let receipt_bytes = evidence(root, &rule.receipt)?;
            let receipt: Value = decode(&receipt_bytes)?;
            if receipt["schema"] != rule.receipt_schema
                || rule
                    .output_pointer
                    .as_ref()
                    .is_some_and(|p| receipt.pointer(p) != Some(&json!(rule.path)))
            {
                return Err("historical producer receipt output/schema differs".into());
            }
            let outcome = receipt
                .pointer(&rule.outcome_pointer)
                .ok_or("historical original outcome missing")?
                .clone();
            if crate::artifact_disposal::identity(&no_symlink_parents(root, &rule.path)?)?
                != expected_identity
            {
                return Err("historical output identity changed".into());
            }
            exclusion.stable()?;
            let publication=inventory.begin_rooted(&rule.producer,&rule.path,json!({"enrollment":enrollment,"producer_source":rule.producer_source.path,"producer_source_sha256":rule.producer_source.sha256,"original_receipt":rule.receipt.path,"original_receipt_sha256":rule.receipt.sha256,"original_outcome":outcome}),&rule.roots)?;
            let id = publication.id().to_owned();
            publication.complete(json!({"enrollment":enrollment,"producer_source":rule.producer_source.path,"producer_source_sha256":rule.producer_source.sha256,"original_receipt":rule.receipt.path,"original_receipt_sha256":rule.receipt.sha256,"original_outcome":outcome}))?;
            exclusion.stable()?;
            Ok(
                json!({"schema":"chrono-historical-enrollment/v1","id":id,"path":rule.path,"roots":rule.roots,"original_outcome":outcome,"completion":"not-established","disposal":"not-attempted"}),
            )
        }
        Request::Release { enrollment } => {
            let rule = inventory.historical(&enrollment)?.clone();
            let exclusion = owner(root, &rule)?;
            let release = rule
                .release
                .as_ref()
                .ok_or("historical consumer release evidence is not registered")?;
            let receipt: Value = decode(&evidence(root, &release.receipt)?)?;
            if receipt.pointer(&release.pointer) != Some(&release.expected) {
                return Err("historical consumer release condition not met".into());
            }
            exclusion.stable()?;
            inventory.release_historical(&enrollment, &rule.roots)?;
            Ok(
                json!({"schema":"chrono-historical-release/v1","enrollment":enrollment,"released_roots":rule.roots,"disposal":"not-attempted","completion":"not-established"}),
            )
        }
        Request::AcknowledgeDelivery {
            report,
            report_sha256,
            artifact_id,
            artifact_digest,
        } => inventory.acknowledge_delivery(&report, &report_sha256, artifact_id, &artifact_digest),
    }
}
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
            let _activity = super::retention::report_activity(&root)?;
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
            let (report, _) = bounded_json(&root, path, 64 * 1024 * 1024)?;
            let (original, original_raw) = if report["schema"] == crate::units::REPORT_REFERENCE {
                let reference: crate::units::ReportReference = serde_json::from_value(report)
                    .map_err(|e| format!("delivery report reference: {e}"))?;
                let raw = crate::prepared::read_original(&root, &reference.original, None)?;
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
        if bytes.len() > 65536 {
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
