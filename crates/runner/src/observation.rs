//! Factual process/tool observations, without expected-version policy or work selection.
use crate::{
    CommandSpec, ProcessEvidence, ProcessResult, resolve_program, run_process_observed, sha256,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

/// An owner's unbuffered, unique temporal original. Rows are observations, not
/// receipts: missing/truncated rows and absence of a terminal remain incomplete.
/// The caller supplies the evidence namespace and accepted source/input identity.
pub struct Progress {
    file: fs::File,
    path: String,
    began: Instant,
    sequence: u64,
    failed: bool,
}
impl Progress {
    pub fn open(
        root: &Path,
        directory: &str,
        prefix: &str,
        identity: serde_json::Value,
    ) -> Result<Self, String> {
        let dir = crate::no_symlink_parents(root, directory.trim_end_matches('/'))?;
        fs::create_dir_all(&dir).map_err(|e| format!("progress directory: {e}"))?;
        let original = tempfile::Builder::new()
            .prefix(prefix)
            .suffix(".jsonl")
            .tempfile_in(&dir)
            .map_err(|e| format!("progress original: {e}"))?;
        let (file, path) = original
            .keep()
            .map_err(|e| format!("progress original: {e}"))?;
        let path = path
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("progress path UTF-8")?
            .to_owned();
        let mut progress = Self {
            file,
            path,
            began: Instant::now(),
            sequence: 0,
            failed: false,
        };
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        progress.record(
            "accepted",
            serde_json::json!({"identity":identity,
            "producer":{"pid":std::process::id(),"executable":executable,
                "sha256":crate::file_identity(&executable)?.0}}),
        )?;
        Ok(progress)
    }
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn record(&mut self, phase: &str, observation: serde_json::Value) -> Result<(), String> {
        if self.failed {
            return Err("progress original has a prior publication failure".into());
        }
        let row = serde_json::json!({"schema":"chrono-progress/v1","sequence":self.sequence,
            "producer_pid":std::process::id(),"phase":phase,"observation":observation,
            "at_unix_millis":SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_millis(),
            "elapsed_millis":self.began.elapsed().as_millis()});
        let mut bytes = serde_json::to_vec(&row).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        if let Err(error) = self.file.write_all(&bytes).and_then(|_| self.file.flush()) {
            self.failed = true;
            return Err(format!(
                "progress original {} publication: {error}",
                self.path
            ));
        }
        self.sequence += 1;
        Ok(())
    }
}
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
    tool_with_evidence(root, program, version_argv, env, timeout, limit, None)
}
pub fn tool_retained(
    root: &Path,
    program: &str,
    version_argv: &[String],
    env: &BTreeMap<String, String>,
    timeout: u64,
    limit: usize,
    evidence: &ProcessEvidence<'_>,
) -> Result<Tool, String> {
    tool_with_evidence(
        root,
        program,
        version_argv,
        env,
        timeout,
        limit,
        Some(evidence),
    )
}
fn tool_with_evidence(
    root: &Path,
    program: &str,
    version_argv: &[String],
    env: &BTreeMap<String, String>,
    timeout: u64,
    limit: usize,
    evidence: Option<&ProcessEvidence<'_>>,
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
    let version = match evidence {
        Some(paths) => crate::run_process_observed_retained(root, &spec, &[], &hash, paths)?,
        None => run_process_observed(root, &spec, &[], &hash)?,
    };
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
