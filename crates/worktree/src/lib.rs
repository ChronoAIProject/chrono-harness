//! Produce real Git worktrees and observations; governance remains with the judges.
mod maintenance;
mod reconstruct;
mod recovery;
mod remote;
mod start;
use chrono_harness::{CliOutput, decode, no_symlink_parents, relative_path};
mod fetch_recovery;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Git {
    pub program: String,
    pub expected_version: Option<String>,
    pub sha256: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Environment {
    pub inherit: Vec<String>,
    pub values: BTreeMap<String, String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: String,
    pub host_config: String,
    pub remote: String,
    pub git: Git,
    pub environment: Environment,
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
    pub report_directory: String,
}
pub struct Start {
    pub root: PathBuf,
    pub config_path: String,
    pub kind: String,
    pub name: String,
    pub destination: PathBuf,
}

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
pub fn run(args: &[String]) -> CliOutput {
    if args == ["--version"] {
        return CliOutput {
            exit_code: 0,
            stdout: "chrono-worktree 0.1.0\n".into(),
            stderr: String::new(),
        };
    }
    if args == ["--help"] {
        return CliOutput { exit_code: 0, stdout: "chrono-worktree start --host-root ROOT --config PATH --kind feature|integration --name TASK --path DESTINATION\nchrono-worktree reconstruct --host-root ROOT --config PATH --kind feature|integration --name TASK --path DESTINATION --plan STATE_PATH\nchrono-worktree recover|recover-interrupted|cleanup|cleanup-fetch|cleanup-fetch-interrupted|cleanup-remote --host-root ROOT --config PATH --plan STATE_PATH\nCreates from a fetched target. Maintenance consumes explicit saved-state plans; no governance or PR/merge verdict.\n".into(), stderr: String::new() };
    }
    let result = if matches!(
        args.first().map(String::as_str),
        Some(
            "recover"
                | "recover-interrupted"
                | "cleanup"
                | "cleanup-fetch"
                | "cleanup-fetch-interrupted"
                | "cleanup-remote"
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
            exit_code: if matches!(
                report["status"].as_str(),
                Some("created" | "reconstructed" | "recovered" | "cleaned")
            ) {
                0
            } else {
                2
            },
            stdout: format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
            stderr: String::new(),
        },
        Err(error) => CliOutput {
            exit_code: 2,
            stdout: String::new(),
            stderr: format!("E_WORKTREE: {error}\n"),
        },
    }
}
fn configuration(root: &Path, config_path: &str) -> Result<(Config, Vec<u8>), String> {
    relative_path(config_path)?;
    if !config_path.starts_with(".chrono-harness/") {
        return Err("worktree configuration must be under .chrono-harness".into());
    }
    let bytes = fs::read(no_symlink_parents(root, config_path)?).map_err(|e| e.to_string())?;
    let config: Config = decode(&bytes)?;
    if config.schema != "chrono-worktree-config/v1"
        || config.remote.is_empty()
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
    relative_path(&config.host_config)?;
    relative_path(config.report_directory.trim_end_matches('/'))?;
    Ok((config, bytes))
}
pub fn start(options: Start) -> Result<serde_json::Value, String> {
    if !matches!(options.kind.as_str(), "feature" | "integration") || options.name.is_empty() {
        return Err("invalid worktree branch kind/name".into());
    }
    let (config, bytes) = configuration(&options.root, &options.config_path)?;
    start::create(options, config, bytes, None)
}
pub fn reconstruct(options: Start, plan_path: &str) -> Result<serde_json::Value, String> {
    if !matches!(options.kind.as_str(), "feature" | "integration") || options.name.is_empty() {
        return Err("invalid worktree branch kind/name".into());
    }
    let (config, bytes) = configuration(&options.root, &options.config_path)?;
    let plan = reconstruct::read(&options.root, plan_path)?;
    start::create(options, config, bytes, Some(plan))
}
