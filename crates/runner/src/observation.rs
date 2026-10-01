//! Factual process/tool observations, without expected-version policy or work selection.
use crate::{CommandSpec, ProcessResult, resolve_program, run_process_observed, sha256};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tool {
    pub program: String,
    pub path: PathBuf,
    pub basename: String,
    pub sha256: String,
    pub version: ProcessResult,
}
pub fn tool(
    root: &Path,
    program: &str,
    version_argv: &[String],
    env: &BTreeMap<String, String>,
    timeout: u64,
    limit: usize,
) -> Result<Tool, String> {
    if !program.contains('/') && !env.contains_key("PATH") {
        return Err("tool resolution requires declared PATH".into());
    }
    // Empty PATH is explicit. resolve_program must not consult ambient PATH here.
    let path = resolve_program(
        root,
        program,
        Some(env.get("PATH").map(String::as_str).unwrap_or("")),
    )?;
    let hash = sha256(&fs::read(&path).map_err(|e| e.to_string())?);
    let spec = CommandSpec {
        program: path.to_str().ok_or("tool path UTF-8")?.into(),
        args: version_argv.to_vec(),
        env: env.clone(),
        timeout_seconds: timeout,
        output_limit_bytes: limit,
    };
    let version = run_process_observed(root, &spec, &[], &hash)?;
    Ok(Tool {
        program: program.into(),
        basename: path
            .file_name()
            .ok_or("tool basename")?
            .to_str()
            .ok_or("tool basename UTF-8")?
            .into(),
        path,
        sha256: hash,
        version,
    })
}

/// Pure success and byte correspondence; no invocation or expected-version policy.
pub fn process_success(p: &ProcessResult) -> Result<(), String> {
    if p.exit_code != 0
        || p.failure.is_some()
        || p.stdout_sha256 != sha256(&p.stdout_bytes)
        || p.stderr_sha256 != sha256(&p.stderr_bytes)
        || p.stdout != String::from_utf8_lossy(&p.stdout_bytes)
        || p.stderr != String::from_utf8_lossy(&p.stderr_bytes)
        || p.environment_digest != crate::wire::digest(&p.environment)?
    {
        return Err("E_PROCESS_EVIDENCE: unsuccessful or inconsistent retained process".into());
    }
    Ok(())
}
