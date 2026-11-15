//! Produce real Git worktrees and observations; governance remains with the judges.
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
use super::identifiers::equal_text;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers::equal_text;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
verus! {
pub fn successful_report_status(status: Option<&str>) -> (successful: bool)
    ensures successful == match status {
        None => false,
        Some(text) => text@ == "created"@
                || text@ == "reconstructed"@
                || text@ == "recovered"@
                || text@ == "cleaned"@
                || text@ == "observed"@
                || text@ == "rebound"@
                || text@ == "finished"@
                || text@ == "maintained"@
                || text@ == "maintenance-requested"@
                || text@ == "maintenance-idle-exit"@
                || text@ == "imported"@
                || text@ == "migrated"@
                || text@ == "owner-deployed"@
                || text@ == "used"@,
    },
{
    match status {
        None => false,
        Some(text) => equal_text(text, "created")
            || equal_text(text, "reconstructed")
            || equal_text(text, "recovered")
            || equal_text(text, "cleaned")
            || equal_text(text, "observed")
            || equal_text(text, "rebound")
            || equal_text(text, "finished")
            || equal_text(text, "maintained")
            || equal_text(text, "maintenance-requested")
            || equal_text(text, "maintenance-idle-exit")
            || equal_text(text, "imported")
            || equal_text(text, "migrated")
            || equal_text(text, "owner-deployed")
            || equal_text(text, "used"),
    }
}
}

#[cfg(not(verus_keep_ghost))]
mod artifact_disposal;
#[cfg(not(verus_keep_ghost))]
pub use artifact_disposal::{ArtifactFootprint, artifact_footprint};
#[cfg(not(verus_keep_ghost))]
mod automatic;
#[cfg(feature = "producer-birth-test")]
#[doc(hidden)]
#[cfg(not(verus_keep_ghost))]
pub use automatic::inventory::worktree_action as inventory_worktree_action;
#[cfg(feature = "producer-birth-test")]
#[doc(hidden)]
#[cfg(not(verus_keep_ghost))]
pub use automatic::inventory::{finish_inventory_writer, inventory_writer};
#[cfg(feature = "producer-birth-test")]
#[doc(hidden)]
#[cfg(not(verus_keep_ghost))]
pub use automatic::write_inventory_record;

#[doc(hidden)]
#[cfg(feature = "producer-birth-test")]
#[cfg(not(verus_keep_ghost))]
pub use automatic::inventory::compact_retention_record as compact_inventory_retention_record;
#[cfg(not(verus_keep_ghost))]
pub use automatic::{RetainedArtifact, TemporaryHost};
#[cfg(not(verus_keep_ghost))]
mod check_inputs;
#[cfg(not(verus_keep_ghost))]
mod evidence_retirement;
#[cfg(not(verus_keep_ghost))]
mod maintenance;
#[cfg(not(verus_keep_ghost))]
mod ownership;
#[cfg(not(verus_keep_ghost))]
mod rebind;
#[cfg(not(verus_keep_ghost))]
mod rebind_inputs;
#[cfg(not(verus_keep_ghost))]
mod rebind_resume;
#[cfg(not(verus_keep_ghost))]
mod rebind_steps;
#[cfg(not(verus_keep_ghost))]
mod reconstruct;
#[cfg(not(verus_keep_ghost))]
mod recovery;
#[cfg(not(verus_keep_ghost))]
mod remote;
#[cfg(not(verus_keep_ghost))]
mod start;
#[allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
pub mod verified_attention;
#[allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
pub mod verified_lifecycle;
#[allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
pub mod verified_owner_publication;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{CliOutput, decode, no_symlink_parents, relative_path};
#[cfg(not(verus_keep_ghost))]
mod fetch_recovery;
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
pub struct Git {
    pub program: String,
    pub expected_version: Option<String>,
    pub sha256: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
pub struct Environment {
    pub inherit: Vec<String>,
    pub values: BTreeMap<String, String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
pub struct Config {
    pub schema: String,
    #[serde(default)]
    pub automatic_cleanup: Option<String>,
    #[serde(default)]
    pub agent_identity_env: Option<String>,
    #[serde(default)]
    pub check_inputs: Option<check_inputs::Policy>,
    pub host_config: String,
    pub remote: String,
    pub git: Git,
    pub environment: Environment,
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
    #[serde(default)]
    pub immutable_input_limits: BTreeMap<String, usize>,
    pub report_directory: String,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
pub(crate) struct AgentIdentity {
    id: String,
    environment: String,
}
#[cfg(not(verus_keep_ghost))]
impl AgentIdentity {
    fn valid(&self) -> bool {
        !self.id.is_empty()
            && self.id.len() <= 256
            && !self.id.chars().any(char::is_control)
            && !self.environment.is_empty()
            && self.environment.len() <= 128
            && self
                .environment
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    }
    fn context(config: &Config) -> serde_json::Value {
        use serde_json::json;
        let Some(name) = &config.agent_identity_env else {
            return json!({"status":"unknown","reason":"agent identity input is not configured"});
        };
        if name.is_empty()
            || name.len() > 128
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return json!({"status":"unknown","reason":"configured agent environment name is invalid"});
        }
        match std::env::var(name) {
            Ok(id) if !id.is_empty() && id.len() <= 256 && !id.chars().any(char::is_control) => {
                json!({"status":"known","identity":Self {id,environment:name.clone()}})
            }
            _ => {
                json!({"status":"unknown","reason":"configured agent identity is missing or invalid","environment":name})
            }
        }
    }
    pub(crate) fn from_report(report: &serde_json::Value) -> Result<Option<Self>, String> {
        if report["agent"]["status"] != "known" {
            return Ok(None);
        }
        let identity: Self = serde_json::from_value(report["agent"]["identity"].clone())
            .map_err(|error| format!("invalid current agent identity: {error}"))?;
        if !identity.valid() {
            return Err("current agent identity is invalid".into());
        }
        Ok(Some(identity))
    }
}
#[cfg(not(verus_keep_ghost))]
pub struct Start {
    pub root: PathBuf,
    pub config_path: String,
    pub kind: String,
    pub name: String,
    pub destination: PathBuf,
}

#[cfg(not(verus_keep_ghost))]
fn arguments(args: &[String]) -> Result<(Start, Option<String>), String> {
    let reconstruction = args.first().map(String::as_str) == Some("reconstruct");
    if (!reconstruction && args.first().map(String::as_str) != Some("start"))
        || args.len() != if reconstruction { 13 } else { 11 }
    {
        return Err("usage: chrono-worktree start|reconstruct --host-root ROOT --config PATH --kind feature|integration --name TASK --path DESTINATION [--plan STATE_PATH (required for reconstruct)]".into());
    }
    let mut values = BTreeMap::new();
    for pair in args[1..].chunks_exact(2) {
        if !["--host-root", "--config", "--kind", "--name", "--path"].contains(&pair[0].as_str())
            && !(reconstruction && pair[0] == "--plan")
            || pair[1].is_empty()
            || values.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("E_WORKTREE_ARGUMENT: unknown, repeated or empty argument".into());
        }
    }
    let root = fs::canonicalize(values["--host-root"]).map_err(|e| e.to_string())?;
    Ok((
        Start {
            destination: root.join(values["--path"]),
            root,
            config_path: values["--config"].into(),
            kind: values["--kind"].into(),
            name: values["--name"].into(),
        },
        values.get("--plan").map(|v| (*v).to_string()),
    ))
}
#[cfg(not(verus_keep_ghost))]
pub fn run(args: &[String]) -> CliOutput {
    if args == ["--version"] {
        return CliOutput {
            exit_code: 0,
            stdout: "chrono-worktree 0.1.0\n".into(),
            stderr: String::new(),
        };
    }
    if args == ["--help"] {
        return CliOutput { exit_code: 0, stdout: "chrono-worktree start --host-root ROOT --config PATH --kind feature|integration --name TASK --path DESTINATION\nchrono-worktree reconstruct --host-root ROOT --config PATH --kind feature|integration --name TASK --path DESTINATION --plan STATE_PATH\nchrono-worktree inspect-rebind|rebind|resume-rebind|recover|recover-interrupted|cleanup|cleanup-fetch|cleanup-fetch-interrupted|cleanup-remote|retire-evidence --host-root ROOT --config PATH --plan STATE_PATH\nchrono-worktree finish [--path WORKTREE] [--dispose-evidence] [--artifacts-only] [--retained-commit OID]\nchrono-worktree maintain\nchrono-worktree maintenance-service [--host-root COORDINATOR]\nchrono-worktree status [--path WORKTREE]\nchrono-worktree import --path WORKTREE\nchrono-worktree migrate --path WORKTREE [--adopt-cleanup-policy] [--adopt-cache STATE_PATH] [--adopt-fixtures STATE_PATH] [--partition-producer ID]\nchrono-worktree use [--path WORKTREE] --operation REGISTERED_OPERATION\nLifecycle commands default to --host-root . --config .chrono-harness/worktree.json; require explicit automatic-cleanup adoption. Unified maintenance uses adopted OS socket activation; idle/timer policy is in maintenance.json. No PR/merge verdict.\n".into(), stderr: String::new() };
    }
    if args.first().map(String::as_str) == Some("check-inputs") {
        return match check_inputs::dispatch(args) {
            Ok(stdout) => CliOutput {
                exit_code: 0,
                stdout,
                stderr: String::new(),
            },
            Err(e) => CliOutput {
                exit_code: 2,
                stdout: String::new(),
                stderr: format!("E_WORKTREE_INPUTS: {e}\n"),
            },
        };
    }
    let result = if args.first().map(String::as_str) == Some("maintenance-service") {
        automatic::service::dispatch(args)
    } else if matches!(
        args.first().map(String::as_str),
        Some(
            "finish"
                | "maintain"
                | "status"
                | "import"
                | "migrate"
                | "use"
                | "check"
                | "bootstrap"
                | "deploy-owner"
        )
    ) {
        automatic::dispatch(args)
    } else if matches!(
        args.first().map(String::as_str),
        Some(
            "inspect-rebind"
                | "rebind"
                | "resume-rebind"
                | "recover"
                | "recover-interrupted"
                | "cleanup"
                | "cleanup-fetch"
                | "cleanup-fetch-interrupted"
                | "cleanup-remote"
                | "retire-evidence"
        )
    ) {
        maintenance::run(args)
    } else {
        arguments(args).and_then(|(options, plan)| match plan {
            Some(path) => reconstruct(options, &path),
            None => start(options),
        })
    };
    match result {
        Ok(report) => CliOutput {
            exit_code: if successful_report_status(report["status"].as_str()) {
                0
            } else {
                2
            },
            stdout: format!("{}\n", serde_json::to_string(&report).unwrap()),
            stderr: automatic::attention_reminder(&report),
        },
        Err(error) => CliOutput {
            exit_code: 2,
            stdout: String::new(),
            stderr: format!("E_WORKTREE: {error}\n"),
        },
    }
}
#[cfg(not(verus_keep_ghost))]
fn configuration(root: &Path, config_path: &str) -> Result<(Config, Vec<u8>), String> {
    relative_path(config_path)?;
    if !config_path.starts_with(".chrono-harness/") {
        return Err("worktree configuration must be under .chrono-harness".into());
    }
    let bytes = fs::read(no_symlink_parents(root, config_path)?).map_err(|e| e.to_string())?;
    let config: Config = decode(&bytes)?;
    if let Some(p) = &config.check_inputs {
        check_inputs::validate(p)?;
    }
    if (config.schema == "chrono-worktree-config/v1"
        && chrono_harness::json(&bytes)?.get("check_inputs").is_some())
        || (config.schema == "chrono-worktree-config/v2" && config.check_inputs.is_none())
    {
        return Err("worktree v2 requires check_inputs; v1 does not accept it".into());
    }
    if !matches!(
        config.schema.as_str(),
        "chrono-worktree-config/v1" | "chrono-worktree-config/v2"
    ) || config.remote.is_empty()
        || config.remote.starts_with('-')
        || config.remote.contains(['\0', '\n', '\r'])
        || !config.host_config.starts_with(".chrono-harness/")
        || !config
            .report_directory
            .starts_with(".chrono-harness/state/")
        || !config.report_directory.ends_with('/')
    {
        return Err("invalid worktree configuration or branch kind/name".into());
    }
    if let Some(path) = &config.automatic_cleanup {
        relative_path(path)?;
        if !path.starts_with(".chrono-harness/") || path.starts_with(".chrono-harness/state/") {
            return Err("automatic cleanup policy must be a tracked .chrono-harness input".into());
        }
    }
    for (path, limit) in &config.immutable_input_limits {
        relative_path(path)?;
        if *limit == 0 || *limit > 64 * 1024 * 1024 {
            return Err("invalid declared immutable input limit".into());
        }
    }
    relative_path(&config.host_config)?;
    relative_path(config.report_directory.trim_end_matches('/'))?;
    Ok((config, bytes))
}
#[cfg(not(verus_keep_ghost))]
pub fn start(options: Start) -> Result<serde_json::Value, String> {
    if !matches!(options.kind.as_str(), "feature" | "integration") || options.name.is_empty() {
        return Err("invalid worktree branch kind/name".into());
    }
    let (config, bytes) = configuration(&options.root, &options.config_path)?;
    start::create(options, config, bytes, None)
}
#[cfg(not(verus_keep_ghost))]
pub fn reconstruct(options: Start, plan_path: &str) -> Result<serde_json::Value, String> {
    if !matches!(options.kind.as_str(), "feature" | "integration") || options.name.is_empty() {
        return Err("invalid worktree branch kind/name".into());
    }
    let (config, bytes) = configuration(&options.root, &options.config_path)?;
    let plan = reconstruct::read(&options.root, plan_path)?;
    start::create(options, config, bytes, Some(plan))
}

#[doc(hidden)]
#[cfg(unix)]
#[cfg(not(verus_keep_ghost))]
pub use automatic::service::serve as serve_maintenance_socket;

#[doc(hidden)]
#[cfg(feature = "producer-birth-test")]
#[cfg(not(verus_keep_ghost))]
pub use automatic::service::with_observer as with_maintenance_observer;

#[doc(hidden)]
#[cfg(all(unix, feature = "producer-birth-test"))]
#[cfg(not(verus_keep_ghost))]
pub use automatic::service::run_with_socket as run_maintenance_service_socket;

#[doc(hidden)]
#[cfg(all(unix, feature = "producer-birth-test"))]
#[cfg(not(verus_keep_ghost))]
pub use automatic::service::validate_activation as validate_maintenance_activation;
