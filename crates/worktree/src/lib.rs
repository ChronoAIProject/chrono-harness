//! Produce real Git worktrees and observations; governance remains with the judges.
mod start;
use chrono_harness::{CliOutput, decode, no_symlink_parents, relative_path};
use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::PathBuf};

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

fn arguments(args: &[String]) -> Result<Start, String> {
    if args.first().map(String::as_str) != Some("start") || args.len() != 11 {
        return Err("usage: chrono-worktree start --host-root ROOT --config PATH --kind feature|integration --name TASK --path DESTINATION".into());
    }
    let mut values = BTreeMap::new();
    for pair in args[1..].chunks_exact(2) {
        if !["--host-root", "--config", "--kind", "--name", "--path"].contains(&pair[0].as_str())
            || pair[1].is_empty()
            || values.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("E_WORKTREE_ARGUMENT: unknown, repeated or empty argument".into());
        }
    }
    let root = fs::canonicalize(values["--host-root"]).map_err(|e| e.to_string())?;
    Ok(Start {
        destination: root.join(values["--path"]),
        root,
        config_path: values["--config"].into(),
        kind: values["--kind"].into(),
        name: values["--name"].into(),
    })
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
        return CliOutput { exit_code: 0, stdout: "chrono-worktree start --host-root ROOT --config PATH --kind feature|integration --name TASK --path DESTINATION\nCreates from a fetched registered target; does not certify governance or perform PR/merge/cleanup.\n".into(), stderr: String::new() };
    }
    match arguments(args).and_then(start) {
        Ok(report) => CliOutput {
            exit_code: if report["status"] == "created" { 0 } else { 2 },
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
pub fn start(options: Start) -> Result<serde_json::Value, String> {
    relative_path(&options.config_path)?;
    if !options.config_path.starts_with(".chrono-harness/") {
        return Err("worktree configuration must be under .chrono-harness".into());
    }
    let bytes = fs::read(no_symlink_parents(&options.root, &options.config_path)?)
        .map_err(|e| e.to_string())?;
    let config: Config = decode(&bytes)?;
    if config.schema != "chrono-worktree-config/v1"
        || config.remote.is_empty()
        || config.remote.starts_with('-')
        || config.remote.contains(['\0', '\n', '\r'])
        || !matches!(options.kind.as_str(), "feature" | "integration")
        || options.name.is_empty()
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
    start::create(options, config, bytes)
}
