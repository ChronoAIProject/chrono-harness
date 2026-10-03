//! Generic external judge transport. Host policy belongs to the registered judge.
mod canonical_request;
mod checkout;
pub mod facts;
mod facts_binding;
mod facts_configs;
#[cfg(unix)]
mod process_ownership;
/// Existing selector owner resolves a host policy for provider consumers.
pub fn load_selected_config(root: &Path, path: &str) -> Result<Value, String> {
    facts_configs::load(root, path).map(|(_, _, config, _)| config)
}
mod facts_inputs;
pub mod full;
pub mod initial;
pub mod input_file;
pub mod observation;
pub mod parity;
pub mod prepared;
#[cfg(unix)]
pub mod process_fds;
pub mod retained_artifacts;
mod short_console;
pub mod units;
pub mod wire;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const PROTOCOL: &str = "chrono-ci-judge/v1";
#[derive(Debug, PartialEq, Eq)]
pub struct CliOutput {
    pub exit_code: u8,
    pub stdout: String,
    pub stderr: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandSpec {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::BTreeMap<String, String>,
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckConfig {
    pub schema: String,
    pub judge: CommandSpec,
    pub policy: Value,
    pub report_path: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<units::Scope>,
    #[serde(default)]
    pub observations: Value,
    pub protocol: String,
    pub request_id: String,
    pub host_root: PathBuf,
    pub config_path: String,
    pub config_sha256: String,
    pub base: Option<String>,
    pub candidate: String,
    pub initial: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Passed,
    Failed,
    NotRequired,
    Blocked,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckResult {
    pub id: String,
    pub status: Status,
    pub cause: String,
    pub exit_code: Option<i32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub protocol: String,
    pub request_id: String,
    pub status: Status,
    pub results: Vec<CheckResult>,
    pub evidence: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessResult {
    pub argv: Vec<String>,
    pub cwd: PathBuf,
    pub environment: std::collections::BTreeMap<String, String>,
    pub environment_digest: String,
    pub stdin_sha256: String,
    pub stdout_bytes: Vec<u8>,
    pub stderr_bytes: Vec<u8>,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    pub failure: Option<String>,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub executable: PathBuf,
    pub sha256: String,
}

pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
/// Transport bytes with bounded memory, returning their observed identity.
pub fn copy_hashed(mut input: impl Read, mut output: impl Write) -> Result<(String, u64), String> {
    let mut hash = Sha256::new();
    let mut length = 0_u64;
    let mut buffer = [0_u8; 65536];
    loop {
        let count = input.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        output
            .write_all(&buffer[..count])
            .map_err(|e| e.to_string())?;
        hash.update(&buffer[..count]);
        length = length
            .checked_add(count as u64)
            .ok_or("input length overflow")?;
    }
    Ok((format!("{:x}", hash.finalize()), length))
}
pub fn file_identity(path: &Path) -> Result<(String, u64), String> {
    if !fs::metadata(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .is_file()
    {
        return Err(format!("not a regular input file: {}", path.display()));
    }
    let file = fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err(format!("not a regular input file: {}", path.display()));
    }
    copy_hashed(file, std::io::sink())
}
pub fn relative_path(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.contains(['\0', '\n', '\r', '\\'])
        || Path::new(value)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(format!("invalid relative path: {value:?}"));
    }
    Ok(())
}
pub fn no_symlink_parents(root: &Path, relative: &str) -> Result<PathBuf, String> {
    relative_path(relative)?;
    let mut p = root.to_path_buf();
    for c in Path::new(relative).components() {
        p.push(c);
        match fs::symlink_metadata(&p) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(format!("symlink path is not allowed: {}", p.display()));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(p)
}

/// Validate and prepare one registered publication destination before any
/// business or judge process is launched. Missing parent directories are part
/// of publication preparation; existing files at the final address may be
/// replaced, while symlinks and type-conflicting parents are rejected.
pub fn prepare_publication(root: &Path, relative: &str) -> Result<PathBuf, String> {
    relative_path(relative)?;
    let components: Vec<_> = Path::new(relative).components().collect();
    let target = root.join(relative);
    let mut current = root.to_path_buf();
    for (index, component) in components.iter().enumerate() {
        current.push(component);
        let final_component = index + 1 == components.len();
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(format!(
                    "symlink path is not allowed: {}",
                    current.display()
                ));
            }
            Ok(metadata) if !final_component && !metadata.is_dir() => {
                return Err(format!(
                    "publication parent is not a directory: {}",
                    current.display()
                ));
            }
            Ok(metadata) if final_component && !metadata.is_file() => {
                return Err(format!(
                    "publication destination is not a file: {}",
                    current.display()
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(error.to_string()),
        }
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    no_symlink_parents(root, relative)
}

fn invocation_template(template: &[String], binding: &Value) -> Vec<String> {
    template
        .iter()
        .map(|value| match value.as_str() {
            "{base}" => binding["base"].as_str().unwrap_or("").into(),
            "{candidate}" => binding["candidate"].as_str().unwrap_or("").into(),
            _ => value.clone(),
        })
        .collect()
}

fn invocation_path(index: usize, expected: &[String]) -> bool {
    index == 0 || index > 0 && matches!(expected[index - 1].as_str(), "--config" | "--context")
}

/// Resolve the canonical path identities used by the live route validator.
/// Raw argv/cwd remain in the entry; this value is an additional retained
/// observation for portable offline comparison after the original root is gone.
pub fn resolve_invocation(
    root: &Path,
    entry: &Value,
    template: &[String],
    binding: &Value,
) -> Result<Value, String> {
    let argv: Vec<String> = serde_json::from_value(entry["argv"].clone())
        .map_err(|_| "E_ROUTE_MISSING: observed entry argv")?;
    let cwd = Path::new(
        entry["cwd"]
            .as_str()
            .ok_or("E_ROUTE_MISSING: observed cwd")?,
    );
    let expected = invocation_template(template, binding);
    if argv.len() != expected.len() || argv.is_empty() {
        return Err("E_ROUTE_MISSING: canonical argv differs".into());
    }
    let mut paths = vec![Value::Null; argv.len()];
    for (index, (actual, want)) in argv.iter().zip(&expected).enumerate() {
        if invocation_path(index, &expected) {
            let actual_path =
                fs::canonicalize(if index > 0 && expected[index - 1] == "--context" {
                    root.join(actual)
                } else {
                    cwd.join(actual)
                })
                .map_err(|error| format!("E_ROUTE_MISSING: {error}"))?;
            let expected_path = fs::canonicalize(root.join(want))
                .map_err(|error| format!("E_ROUTE_MISSING: {error}"))?;
            if actual_path != expected_path {
                return Err("E_ROUTE_MISSING: canonical path differs".into());
            }
            paths[index] = serde_json::json!({
                "actual": actual_path,
                "expected": expected_path,
            });
        } else if actual != want {
            return Err("E_ROUTE_MISSING: canonical argv differs".into());
        }
    }
    let raw = serde_json::json!({"argv": argv, "cwd": cwd});
    Ok(serde_json::json!({
        "raw_digest": wire::digest(&raw)?,
        "paths": paths,
    }))
}

/// Validate a retained entry's live path identities without touching the
/// original checkout. The raw argv/cwd digest and non-path arguments remain
/// checked, while path equality comes from the identities captured live.
pub fn validate_invocation_observation(
    entry: &Value,
    template: &[String],
    binding: &Value,
) -> Result<(), String> {
    let argv: Vec<String> = serde_json::from_value(entry["argv"].clone())
        .map_err(|_| "E_ROUTE_MISSING: observed entry argv")?;
    let cwd = entry["cwd"]
        .as_str()
        .ok_or("E_ROUTE_MISSING: observed cwd")?;
    let expected = invocation_template(template, binding);
    let resolved = entry["resolved_paths"]
        .as_object()
        .ok_or("E_ROUTE_MISSING: resolved entry paths")?;
    let raw = serde_json::json!({"argv": argv, "cwd": cwd});
    if *resolved
        .get("raw_digest")
        .ok_or("E_ROUTE_MISSING: raw entry digest")?
        != wire::digest(&raw)?
    {
        return Err("E_ROUTE_MISSING: raw entry identity differs".into());
    }
    let paths = resolved
        .get("paths")
        .ok_or("E_ROUTE_MISSING: resolved entry paths")?
        .as_array()
        .ok_or("E_ROUTE_MISSING: resolved entry paths")?;
    if argv.len() != expected.len() || paths.len() != argv.len() || argv.is_empty() {
        return Err("E_ROUTE_MISSING: canonical argv differs".into());
    }
    for (index, (actual, want)) in argv.iter().zip(&expected).enumerate() {
        if invocation_path(index, &expected) {
            let identity = paths[index]
                .as_object()
                .ok_or("E_ROUTE_MISSING: resolved path identity")?;
            let actual_path = identity
                .get("actual")
                .ok_or("E_ROUTE_MISSING: actual path identity")?
                .as_str()
                .ok_or("E_ROUTE_MISSING: resolved path identity")?;
            let expected_path = identity
                .get("expected")
                .ok_or("E_ROUTE_MISSING: expected path identity")?
                .as_str()
                .ok_or("E_ROUTE_MISSING: resolved path identity")?;
            if actual_path != expected_path || actual_path.is_empty() {
                return Err("E_ROUTE_MISSING: canonical path differs".into());
            }
        } else if !paths[index].is_null() || actual != want {
            return Err("E_ROUTE_MISSING: canonical argv differs".into());
        }
    }
    Ok(())
}
/// Reject duplicate object members before typed decoding, including opaque policy data.
pub fn json(bytes: &[u8]) -> Result<Value, String> {
    struct Strict;
    impl<'de> serde::de::Visitor<'de> for Strict {
        type Value = Value;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("JSON without duplicate members")
        }
        fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Value, E> {
            Ok(v.into())
        }
        fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Value, E> {
            Ok(v.into())
        }
        fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Value, E> {
            Ok(v.into())
        }
        fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Value, E> {
            serde_json::Number::from_f64(v)
                .map(Value::Number)
                .ok_or_else(|| E::custom("nonfinite number"))
        }
        fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Value, E> {
            Ok(v.into())
        }
        fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Value, E> {
            Ok(v.into())
        }
        fn visit_unit<E: serde::de::Error>(self) -> Result<Value, E> {
            Ok(Value::Null)
        }
        fn visit_none<E: serde::de::Error>(self) -> Result<Value, E> {
            Ok(Value::Null)
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut a: A) -> Result<Value, A::Error> {
            let mut out = vec![];
            while let Some(v) = a.next_element::<StrictValue>()? {
                out.push(v.0)
            }
            Ok(out.into())
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(self, mut a: A) -> Result<Value, A::Error> {
            let mut out = serde_json::Map::new();
            while let Some(k) = a.next_key::<String>()? {
                if out.contains_key(&k) {
                    return Err(serde::de::Error::custom(format!(
                        "duplicate JSON member {k}"
                    )));
                }
                out.insert(k, a.next_value::<StrictValue>()?.0);
            }
            Ok(Value::Object(out))
        }
    }
    struct StrictValue(Value);
    impl<'de> Deserialize<'de> for StrictValue {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            d.deserialize_any(Strict).map(StrictValue)
        }
    }
    let mut d = serde_json::Deserializer::from_slice(bytes);
    let v = StrictValue::deserialize(&mut d).map_err(|e| e.to_string())?;
    d.end().map_err(|e| e.to_string())?;
    Ok(v.0)
}
pub fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    serde_json::from_value(json(bytes)?).map_err(|e| e.to_string())
}
pub fn load_config(path: &Path) -> Result<CheckConfig, String> {
    let c: CheckConfig = decode(&fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?)?;
    if !matches!(
        c.schema.as_str(),
        "chrono-ci-check/v1" | "chrono-ci-check/v2" | "chrono-ci-check/v3"
    ) {
        return Err("unsupported check profile; full governance remains not implemented".into());
    }
    validate_command(&c.judge)?;
    relative_path(&c.report_path)?;
    if !c.report_path.starts_with(".chrono-harness/state/") {
        return Err("report_path must reside in .chrono-harness/state/".into());
    }
    Ok(c)
}
pub fn validate_command(s: &CommandSpec) -> Result<(), String> {
    if s.program.is_empty()
        || s.program.contains('\0')
        || s.args.iter().any(|v| v.contains('\0'))
        || s.env
            .iter()
            .any(|(k, v)| k.is_empty() || k.contains(['=', '\0']) || v.contains('\0'))
        || s.timeout_seconds == 0
        || s.output_limit_bytes == 0
        || s.output_limit_bytes > 64 * 1024 * 1024
    {
        return Err("invalid command/bounds".into());
    }
    Ok(())
}
/// Ordered lexical candidates for live resolution and historical invocation verification.
/// Passing a PATH override avoids consulting the collector's ambient PATH.
pub fn program_candidates(
    root: &Path,
    program: &str,
    path_override: Option<&str>,
) -> Result<Vec<PathBuf>, String> {
    let root = if root.is_absolute() {
        root.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(root)
    };
    let raw = Path::new(program);
    let paths = if raw.is_absolute() {
        vec![raw.to_path_buf()]
    } else if program.contains('/') {
        vec![root.join(raw)]
    } else {
        let path = path_override
            .map(std::ffi::OsString::from)
            .or_else(|| std::env::var_os("PATH"))
            .ok_or("PATH is missing")?;
        std::env::split_paths(&path)
            .map(|p| {
                if p.is_absolute() {
                    p.join(raw)
                } else {
                    root.join(p).join(raw)
                }
            })
            .collect()
    };
    Ok(paths)
}
pub fn resolve_program(
    root: &Path,
    program: &str,
    path_override: Option<&str>,
) -> Result<PathBuf, String> {
    for p in program_candidates(root, program, path_override)? {
        if p.is_file() {
            // Preserve the invocation basename: rustup/cargo and other multicall tools
            // select behavior from argv[0]. Reading p still hashes the target bytes.
            return Ok(p);
        }
    }
    Err(format!("executable not found: {program}"))
}
pub fn run_process(root: &Path, s: &CommandSpec, input: &[u8]) -> Result<ProcessResult, String> {
    finish_process(run_process_inner(
        root,
        s,
        input,
        None,
        Duration::from_secs(s.timeout_seconds),
    )?)
}
/// v1 uses the same bounded engine with a cleared environment and prelaunch binding.
pub fn run_process_bound(
    root: &Path,
    s: &CommandSpec,
    input: &[u8],
    digest: &str,
) -> Result<ProcessResult, String> {
    finish_process(run_process_inner(
        root,
        s,
        input,
        Some(digest),
        Duration::from_secs(s.timeout_seconds),
    )?)
}
fn finish_process(p: ProcessResult) -> Result<ProcessResult, String> {
    if let Some(error) = &p.failure {
        return Err(error.clone());
    }
    Ok(p)
}
/// Same engine, retaining partial output and true exit even when a process bound fires.
pub fn run_process_observed(
    root: &Path,
    s: &CommandSpec,
    input: &[u8],
    digest: &str,
) -> Result<ProcessResult, String> {
    run_process_inner(
        root,
        s,
        input,
        Some(digest),
        Duration::from_secs(s.timeout_seconds),
    )
}
/// Preserve a caller's finer acquisition deadline through the same engine.
pub fn run_process_observed_for(
    root: &Path,
    s: &CommandSpec,
    input: &[u8],
    digest: &str,
    timeout: Duration,
) -> Result<ProcessResult, String> {
    run_process_inner(root, s, input, Some(digest), timeout)
}
// The existing process engine owns termination on errors and unwinding too.
struct OwnedProcess {
    child: std::process::Child,
    #[cfg(unix)]
    ownership: process_ownership::Launch,
    joined: bool,
}
impl std::ops::Deref for OwnedProcess {
    type Target = std::process::Child;
    fn deref(&self) -> &Self::Target {
        &self.child
    }
}
impl std::ops::DerefMut for OwnedProcess {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.child
    }
}
impl Drop for OwnedProcess {
    fn drop(&mut self) {
        if self.joined {
            return;
        }
        #[cfg(unix)]
        {
            self.ownership.drain();
            unsafe {
                libc::kill(-(self.child.id() as i32), libc::SIGKILL);
            }
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn run_process_inner(
    root: &Path,
    s: &CommandSpec,
    input: &[u8],
    expected: Option<&str>,
    timeout: Duration,
) -> Result<ProcessResult, String> {
    validate_command(s)?;
    if expected.is_some() && !Path::new(&s.program).is_absolute() {
        return Err("bound executable must be absolute; no ambient PATH resolution".into());
    }
    let executable = resolve_program(root, &s.program, s.env.get("PATH").map(String::as_str))?;
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let mut environment = if expected.is_some() {
        s.env.clone()
    } else {
        let mut env: std::collections::BTreeMap<String, String> = std::env::vars().collect();
        env.extend(s.env.clone());
        env
    };
    let stdin_sha256 = sha256(input);
    let hash = sha256(&fs::read(&executable).map_err(|e| e.to_string())?);
    if expected.is_some_and(|v| v != hash) {
        return Err("prelaunch executable digest mismatch".into());
    }
    let mut command = Command::new(&executable);
    if expected.is_some() {
        command.env_clear();
    }
    command
        .args(&s.args)
        .envs(&s.env)
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(unix)]
    let _transfer = process_fds::Transfer::prepare(&mut command)?;
    #[cfg(unix)]
    if let Some(value) = &_transfer.value {
        environment.insert(process_fds::ENV.into(), value.clone());
    } else {
        environment.remove(process_fds::ENV);
    }
    #[cfg(unix)]
    let ownership = process_ownership::Launch::prepare()?;
    #[cfg(unix)]
    ownership.configure(&mut command);
    std::thread::scope(|scope| {
        let child = command.spawn().map_err(|e| {
            #[cfg(unix)]
            let e = ownership.spawn_error(e);
            format!("{}: {e}", executable.display())
        })?;
        let mut child = OwnedProcess {
            child,
            #[cfg(unix)]
            ownership,
            joined: false,
        };
        let pid = child.id();
        let stdin = child.stdin.take().ok_or("missing process stdin")?;
        let input = input.to_vec();
        let writer = scope.spawn(move || {
            let mut stdin = stdin;
            stdin.write_all(&input)
        });
        let exceeded = Arc::new(AtomicBool::new(false));
        let limit = s.output_limit_bytes;
        #[cfg(unix)]
        let wake = child.ownership.wake();
        #[cfg(not(unix))]
        let wake = std::thread::current();
        let monitor = move || {
            #[cfg(unix)]
            wake.notify();
            #[cfg(not(unix))]
            wake.unpark();
        };
        fn reader<'scope, R: Read + Send + 'scope, W: Fn() + Send + 'scope>(
            scope: &'scope std::thread::Scope<'scope, '_>,
            mut r: R,
            limit: usize,
            flag: Arc<AtomicBool>,
            monitor: W,
        ) -> std::thread::ScopedJoinHandle<'scope, std::io::Result<Vec<u8>>> {
            scope.spawn(move || {
                let result = (|| {
                    let mut out = Vec::new();
                    let mut buf = [0u8; 8192];
                    loop {
                        let n = r.read(&mut buf)?;
                        if n == 0 {
                            break;
                        }
                        if out.len() + n > limit && !flag.swap(true, Ordering::Relaxed) {
                            monitor();
                        }
                        let keep = n.min(limit.saturating_sub(out.len()));
                        out.extend_from_slice(&buf[..keep]);
                    }
                    Ok(out)
                })();
                // EOF or an IO error prompts another real child-status probe;
                // neither is treated as evidence that the process has exited.
                monitor();
                result
            })
        }
        let stdout = reader(
            scope,
            child.stdout.take().ok_or("missing stdout")?,
            limit,
            exceeded.clone(),
            monitor.clone(),
        );
        let stderr = reader(
            scope,
            child.stderr.take().ok_or("missing stderr")?,
            limit,
            exceeded.clone(),
            monitor,
        );
        let start = Instant::now();
        let mut failure = None;
        let mut status = loop {
            #[cfg(unix)]
            if child.ownership.cancelled() {
                failure = Some("process cancelled by enclosing owner".to_string());
                break None;
            }
            if exceeded.load(Ordering::Relaxed) || start.elapsed() >= timeout {
                failure = Some(
                    if exceeded.load(Ordering::Relaxed) {
                        "process output limit exceeded"
                    } else {
                        "process timed out"
                    }
                    .to_string(),
                );
                break None;
            }
            match child.try_wait() {
                Ok(Some(v)) => break Some(v),
                Ok(None) => {}
                Err(e) => {
                    failure = Some(format!("process wait: {e}"));
                    break None;
                }
            }
            #[cfg(unix)]
            child.ownership.wait_for_wake(Duration::from_millis(10));
            #[cfg(not(unix))]
            std::thread::park_timeout(Duration::from_millis(10));
        };
        #[cfg(unix)]
        if !child.ownership.drain() {
            let original = failure
                .take()
                .unwrap_or_else(|| "process ownership cleanup failed".into());
            failure = Some(format!(
                "{original}; nested owner did not acknowledge joined completion"
            ));
        }
        // Kill lingering descendants even when their direct parent has exited; pipe readers remain bounded.
        #[cfg(unix)]
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
        if status.is_none() {
            let _ = child.kill();
            match child.wait() {
                Ok(v) => status = Some(v),
                Err(e) => {
                    let original = failure.take().unwrap_or_default();
                    failure = Some(format!("{original}; process join: {e}"));
                }
            }
        }
        child.joined = status.is_some();
        let stdout_result = stdout.join();
        let stderr_result = stderr.join();
        let _ = writer.join();
        let a = stdout_result
            .map_err(|_| "stdout reader panicked")?
            .map_err(|e| e.to_string())?;
        let b = stderr_result
            .map_err(|_| "stderr reader panicked")?
            .map_err(|e| e.to_string())?;
        if exceeded.load(Ordering::Relaxed)
            && failure.as_deref() != Some("process output limit exceeded")
        {
            failure = Some(match failure {
                Some(original) => format!("{original}; process output limit exceeded"),
                None => "process output limit exceeded".into(),
            });
        }
        Ok(ProcessResult {
            argv: std::iter::once(executable.to_string_lossy().into_owned())
                .chain(s.args.clone())
                .collect(),
            cwd: root,
            environment_digest: wire::digest(&environment)?,
            environment,
            stdin_sha256,
            stdout_sha256: sha256(&a),
            stderr_sha256: sha256(&b),
            stdout: String::from_utf8_lossy(&a).into_owned(),
            stderr: String::from_utf8_lossy(&b).into_owned(),
            stdout_bytes: a,
            stderr_bytes: b,
            failure,
            exit_code: status.and_then(|s| s.code()).unwrap_or(-1),
            executable,
            sha256: hash,
        })
    })
}
pub fn validate_response(r: &Response, id: &str, exit: i32) -> Result<(), String> {
    validate_response_protocol(r, id, exit, PROTOCOL)
}
pub fn validate_response_protocol(
    r: &Response,
    id: &str,
    exit: i32,
    protocol: &str,
) -> Result<(), String> {
    if r.protocol != protocol
        || r.request_id != id
        || r.results.is_empty()
        || !r.evidence.is_object()
        || r.evidence.as_object().is_none_or(|o| o.is_empty())
    {
        return Err("empty or mismatched judge response".into());
    }
    let mut ids = BTreeSet::new();
    for result in &r.results {
        if result.id.is_empty() || result.cause.is_empty() || !ids.insert(&result.id) {
            return Err("invalid/duplicate judge result".into());
        }
        if result.status == Status::Passed && result.exit_code.is_some_and(|c| c != 0) {
            return Err("passed result has failing exit".into());
        }
        if result.status == Status::Failed && result.exit_code == Some(0) {
            return Err("failed result has successful exit".into());
        }
        if result.status == Status::NotRequired && result.exit_code.is_some() {
            return Err("not-required result must have null exit_code".into());
        }
    }
    let failed = r
        .results
        .iter()
        .any(|v| matches!(v.status, Status::Failed | Status::Blocked));
    let all_unused = r.results.iter().all(|v| v.status == Status::NotRequired);
    if (failed && !matches!(r.status, Status::Failed | Status::Blocked))
        || (!failed && matches!(r.status, Status::Failed | Status::Blocked))
        || (r.status == Status::NotRequired) != all_unused
        || (exit == 0) != (!failed)
    {
        return Err("judge exit/status/results disagreement".into());
    }
    Ok(())
}
pub fn canonical_argv(
    runner: &str,
    config: &str,
    base: Option<&str>,
    candidate: &str,
    initial: bool,
) -> Vec<String> {
    let mut v = vec![
        runner.into(),
        "check".into(),
        "--config".into(),
        config.into(),
    ];
    if let Some(b) = base {
        v.extend(["--base".into(), b.into()])
    }
    v.extend(["--candidate".into(), candidate.into()]);
    if initial {
        v.push("--initial".into())
    }
    v
}
fn root_for_config(
    path: &Path,
    canonical_host: Option<&Path>,
) -> Result<(PathBuf, String), String> {
    let full = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    if let Some(host) = canonical_host {
        // Resolve only caller-to-host traversal. Below-root entries must be
        // checked before a link or a following parent component can erase them.
        let components: Vec<_> = full.components().collect();
        for (index, component) in components.iter().enumerate() {
            if *component != Component::Normal(std::ffi::OsStr::new(".chrono-harness")) {
                continue;
            }
            let prefix: PathBuf = components[..index].iter().collect();
            let root = fs::canonicalize(prefix).map_err(|e| e.to_string())?;
            if root != host {
                continue;
            }
            let mut rel = PathBuf::from(".chrono-harness");
            no_symlink_parents(&root, rel.to_str().ok_or("non UTF-8 config path")?)?;
            for component in &components[index + 1..] {
                match component {
                    Component::CurDir => {}
                    Component::Normal(value) => {
                        rel.push(value);
                        no_symlink_parents(&root, rel.to_str().ok_or("non UTF-8 config path")?)?;
                    }
                    Component::ParentDir => {
                        if rel == Path::new(".chrono-harness") || !rel.pop() {
                            return Err("config path escaped host policy root".into());
                        }
                    }
                    _ => return Err("config path escaped host root".into()),
                }
            }
            return Ok((root, rel.to_str().ok_or("non UTF-8 config path")?.into()));
        }
        return Err("config must reside beneath .chrono-harness".into());
    }
    let canonical = fs::canonicalize(&full).map_err(|e| e.to_string())?;
    for p in canonical.ancestors().skip(1) {
        if p.file_name().is_some_and(|n| n == ".chrono-harness") {
            let root = p.parent().ok_or("no host root")?.to_path_buf();
            let rel = canonical
                .strip_prefix(&root)
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("non UTF-8 config path")?
                .into();
            return Ok((root, rel));
        }
    }
    Err("config must reside beneath .chrono-harness".into())
}
pub fn dispatch(args: &[&str]) -> CliOutput {
    let mut argv = vec![std::env::args().next().unwrap_or_default()];
    argv.extend(args.iter().map(|s| s.to_string()));
    dispatch_observed(
        args,
        serde_json::json!({"argv":argv,"cwd":std::env::current_dir().ok()}),
    )
}
pub fn dispatch_observed(args: &[&str], entry: Value) -> CliOutput {
    match args{
    []|["help"]|["--help"]|["-h"]=>CliOutput{exit_code:0,stdout:"chrono-harness check\nchrono-harness check --unit ID\nchrono-harness check --collect\nConfigured short checks read .chrono-harness/config.json and produce their inputs automatically. Legacy explicit spelling is accepted only by legacy registered contracts. Full independent scopes require registered execution_units.\nchrono-harness parity --host-root H --report P --compared-report P\nUse --initial without --base for a parentless candidate. Use --context P for chrono-judge/v1 external judges. The parity command adds fail-closed evidence to two completed full reports; it never changes the canonical check command. Use an explicit chrono-initial-check/v1 profile for root registry inventory. Seven-judge governance NOT IMPLEMENTED.\n".into(),stderr:String::new()},
    ["--version"]|["-V"]=>CliOutput{exit_code:0,stdout:format!("chrono-harness {}\n",env!("CARGO_PKG_VERSION")),stderr:String::new()},
    ["spec","status"]=>CliOutput{exit_code:0,stdout:"SPEC_STATUS=draft\nENFORCEMENT=not-implemented\nHOST_REGISTRIES=proposed\nCI_CHECK=chrono-ci-check/v1\nV1_TRANSPORT=implemented\nREGISTRATION=implemented\nCONTRACT=SPEC.md\n".into(),stderr:String::new()},
    ["check",rest @ ..]=>match check(rest, entry){Ok(output)=>output,Err(e)=>CliOutput{exit_code:2,stdout:String::new(),stderr:format!("E_CHECK: {e}\n")}},
    ["parity",rest @ ..]=>parity_command(rest),
    _=>CliOutput{exit_code:2,stdout:String::new(),stderr:"E_USAGE: use --help\n".into()}
}
}
fn check(args: &[&str], entry: Value) -> Result<CliOutput, String> {
    if args.is_empty() || args == ["--collect"] || matches!(args, ["--unit", _]) {
        let root = fs::canonicalize(std::env::current_dir().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        if let Some(result) = prepared::participate(&root, args)? {
            return Ok(result);
        }
    }
    check_unmanaged(args, entry).map(|(exit_code, stdout)| CliOutput {
        exit_code,
        stdout,
        stderr: String::new(),
    })
}
fn check_unmanaged(args: &[&str], entry: Value) -> Result<(u8, String), String> {
    if args.is_empty() || args == ["--collect"] || matches!(args, ["--unit", _]) {
        let root = fs::canonicalize(std::env::current_dir().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let selection = match args {
            [] => prepared::Selection::All,
            ["--collect"] => prepared::Selection::Collect,
            ["--unit", unit] => prepared::Selection::Unit {
                unit: (*unit).into(),
            },
            _ => unreachable!(),
        };
        let (req, p, binding) = prepared::prepare(&root, selection)?;
        let retention = prepared::retention_directory(&req);
        let result = execute_check(
            root.clone(),
            req.profile,
            p.base,
            p.candidate,
            p.initial,
            p.context.map(|c| c.path),
            p.scope,
            entry,
            Some(binding),
        )
        .map(|(code, report)| {
            let console = match short_console::completed(code, &report) {
                Ok(console) => console,
                Err(projection_error) => {
                    short_console::projection_fallback(code, &report, &projection_error)
                }
            };
            (code, console)
        });
        return result.map_err(|error| {
            match prepared::retain_original(&root, &retention, "check-error", error.as_bytes()) {
                Ok(original) => format!(
                    "{}\nOriginal check error: {} (sha256 {})",
                    short_console::error_cause(&error),
                    original.path,
                    original.sha256
                ),
                // Retention failure cannot justify discarding the original error
                // or advertising a completed report or a nonexistent artifact.
                Err(retention_error) => {
                    format!("{error}\nOriginal check error retention failed: {retention_error}")
                }
            }
        });
    }
    if !args.contains(&"--config") {
        return Err("short check accepts only check, check --unit ID, or check --collect".into());
    }

    let mut config = None;
    let mut base = None;
    let mut candidate = None;
    let mut initial = false;
    let mut context = None;
    let mut scope = None;
    let mut i = 0;
    let mut seen = BTreeSet::new();
    while i < args.len() {
        let k = args[i];
        if !seen.insert(k) {
            return Err(format!("duplicate argument {k}"));
        }
        if k == "--initial" {
            initial = true;
            i += 1;
            continue;
        }
        let value = *args.get(i + 1).ok_or("missing argument value")?;
        match k {
            "--config" => config = Some(value),
            "--base" => base = Some(value.to_owned()),
            "--candidate" => candidate = Some(value.to_owned()),
            "--context" => context = Some(value),
            "--unit" | "--collect" => {
                if scope.is_some() {
                    return Err("unit and collect are mutually exclusive".into());
                }
                scope = Some(if k == "--unit" {
                    units::Scope::Unit { unit: value.into() }
                } else {
                    units::Scope::Collect {
                        manifest: value.into(),
                    }
                });
            }
            _ => return Err(format!("unknown argument {k}")),
        }
        i += 2;
    }
    if initial == base.is_some() {
        return Err("supply either --base or --initial".into());
    }
    let candidate = candidate.ok_or("missing --candidate")?;
    let config = Path::new(config.ok_or("missing --config")?);
    let (root, entry_path) = root_for_config(config, None)?;
    let entry_profile = json(&fs::read(root.join(&entry_path)).map_err(|e| e.to_string())?)?;
    // Resolve a native selector before admitting full-unit scope.  The request
    // keeps the literal entry path so route identity and registry provenance
    // still bind the caller's selector; policy checks use the stable selected
    // direct full-v3 profile.
    let (root, config_path, _profile) = if entry_profile["schema"] == "chrono-git-configs/v1" {
        let (root, entry_path) = root_for_config(config, Some(&root))?;
        let (_, _, selected, _) = facts_configs::load(&root, &entry_path)?;
        (root, entry_path, selected)
    } else {
        (root, entry_path, entry_profile)
    };
    execute_check(
        root,
        config_path,
        base,
        candidate,
        initial,
        context.map(str::to_owned),
        scope,
        entry,
        None,
    )
}
#[allow(clippy::too_many_arguments)]
fn execute_check(
    root: PathBuf,
    config_path: String,
    base: Option<String>,
    candidate: String,
    initial: bool,
    context: Option<String>,
    scope: Option<units::Scope>,
    entry: Value,
    preparation: Option<Value>,
) -> Result<(u8, String), String> {
    let (_, _, profile, _) = facts_configs::load(&root, &config_path)?;
    let full_units = if scope.is_some() && profile["schema"] != units::PROFILE {
        units::full_execution_units(&profile)?
            .ok_or("unit/collect selection requires registered full-v3/v4 execution units")
            .map(Some)?
    } else {
        None
    };
    if scope.is_some()
        && profile["schema"] == units::PROFILE
        && profile["policy"].get("units").is_none()
    {
        return Err("unit/collect selection requires registered execution units".into());
    }
    if let (Some(units::Scope::Collect { manifest }), Some(block)) = (&scope, &full_units) {
        let definitions: std::collections::BTreeMap<String, units::Unit> =
            serde_json::from_value(block["units"].clone()).map_err(|e| e.to_string())?;
        units::validate_collection_paths(
            manifest,
            block["report_path"]
                .as_str()
                .ok_or("collection report path")?,
            &definitions
                .values()
                .map(|u| u.report_path.clone())
                .collect::<Vec<_>>(),
        )?;
    }
    let mut entry = entry;
    if let Some(template) = profile["canonical_check"]["argv"].as_array() {
        let mut template: Vec<String> =
            serde_json::from_value(Value::Array(template.clone())).map_err(|e| e.to_string())?;
        if let Some(scope) = &scope {
            template.extend(scope.contract_argv(&profile));
        }
        let binding = serde_json::json!({"base":base,"candidate":candidate});
        let resolved = resolve_invocation(&root, &entry, &template, &binding)?;
        entry
            .as_object_mut()
            .ok_or("observed entry object")?
            .insert("resolved_paths".into(), resolved);
    }
    if profile.get("schema").and_then(Value::as_str) == Some(initial::PROFILE) {
        if !initial || context.is_some() {
            return Err("initial inventory requires --initial without --base or --context".into());
        }
        return initial::check(&root, &config_path, &candidate, entry);
    }
    if !matches!(
        profile.get("schema").and_then(Value::as_str),
        Some("chrono-ci-check/v1" | "chrono-ci-check/v2" | "chrono-ci-check/v3")
    ) {
        return full::check_prepared(
            &root,
            &config_path,
            base.as_deref()
                .ok_or("full check requires --base; initial mode has no governance success")?,
            &candidate,
            Path::new(&context.ok_or("full check requires --context")?),
            entry,
            scope,
            preparation,
        );
    }
    if context.is_some() {
        return Err("CI profile does not accept --context".into());
    }
    let c = load_config(&root.join(&config_path))?;
    let output_path = scope
        .as_ref()
        .map(|s| s.report_path(&c))
        .transpose()?
        .unwrap_or(c.report_path.clone());
    let protocol = if scope.is_some() {
        units::PROTOCOL
    } else {
        PROTOCOL
    };
    let bytes = fs::read(root.join(&config_path)).map_err(|e| e.to_string())?;
    let request_id = sha256(
        format!(
            "{}:{:?}:{candidate}:{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH),
            sha256(&bytes)
        )
        .as_bytes(),
    );
    let req = Request {
        observations: serde_json::json!({"entry":entry,"preparation":preparation}),
        scope,
        protocol: protocol.into(),
        request_id: request_id.clone(),
        host_root: root.clone(),
        config_path,
        config_sha256: sha256(&bytes),
        base,
        candidate,
        initial,
    };
    let report_path = prepare_publication(&root, &output_path)?;
    let input = serde_json::to_vec(&req).map_err(|e| e.to_string())?;
    let proc = if req.observations["preparation"].is_object() {
        let (_, _, policy, _) = facts_configs::load(&root, ".chrono-harness/config.json")?;
        let credentials = prepared::credential_environment(&policy)?;
        let mut judge = c.judge.clone();
        let overrides = judge.env.clone();
        judge.env.clear();
        for key in policy["environment"]["inherit"]
            .as_array()
            .ok_or("environment inherit")?
        {
            let key = key.as_str().ok_or("environment key")?;
            if !credentials.contains(key) {
                if let Ok(value) = std::env::var(key) {
                    judge.env.insert(key.into(), value);
                }
            }
        }
        for (key, value) in policy["environment"]["values"]
            .as_object()
            .ok_or("environment values")?
        {
            judge.env.insert(
                key.clone(),
                value.as_str().ok_or("environment value")?.into(),
            );
        }
        if overrides.keys().any(|key| credentials.contains(key)) {
            return Err("acquisition credentials cannot be forwarded to the check judge".into());
        }
        judge.env.extend(overrides);
        let path = resolve_program(
            &root,
            &judge.program,
            Some(judge.env.get("PATH").map(String::as_str).unwrap_or("")),
        )?;
        judge.program = path.to_str().ok_or("judge path UTF-8")?.into();
        run_process_observed(&root, &judge, &input, &file_identity(&path)?.0)
    } else {
        run_process(&root, &c.judge, &input)
    };
    let runner_executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let runner_identity = serde_json::json!({"path":runner_executable,"sha256":sha256(&fs::read(&runner_executable).map_err(|e|e.to_string())?),"version":env!("CARGO_PKG_VERSION")});
    let mut report = match proc {
        Ok(p) => {
            let response: Result<Response, String> = match &p.failure {
                Some(failure) => Err(failure.clone()),
                None => decode(&p.stdout_bytes),
            };
            match response.and_then(|r| {
                validate_response_protocol(&r, &request_id, p.exit_code, protocol)?;
                Ok(r)
            }) {
                Ok(r) => {
                    serde_json::json!({"schema":"chrono-check-report/v1","request":req,"judge":p,"response":r,"parity":"unestablished"})
                }
                Err(e) => {
                    serde_json::json!({"schema":"chrono-check-report/v1","request":req,"transport_failure":e,"judge":p})
                }
            }
        }
        Err(e) => {
            serde_json::json!({"schema":"chrono-check-report/v1","request":req,"transport_failure":e})
        }
    };
    report["runner"] = runner_identity;
    let code = if report.get("transport_failure").is_some() {
        2
    } else if matches!(
        report["response"]["status"].as_str(),
        Some("passed" | "not-required")
    ) {
        0
    } else {
        1
    };
    let retained_report = if report["request"]["observations"]["preparation"].is_object() {
        let prepared: prepared::InputRequest = serde_json::from_value(
            report["request"]["observations"]["preparation"]["request"].clone(),
        )
        .map_err(|e| e.to_string())?;
        let path = format!(
            "{}check-{request_id}.json",
            prepared::retention_directory(&prepared)
        );
        report["retained_report"] = serde_json::json!(path);
        Some(no_symlink_parents(&root, &path)?)
    } else {
        None
    };
    let text = if c.schema == units::PROFILE {
        serde_json::to_string(&report)
    } else {
        serde_json::to_string_pretty(&report)
    }
    .map_err(|e| e.to_string())?
        + "\n";
    fs::create_dir_all(report_path.parent().ok_or("report parent missing")?)
        .map_err(|e| e.to_string())?;
    if let Some(path) = retained_report {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        file.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    }
    fs::write(report_path, &text).map_err(|e| e.to_string())?;
    Ok((code, text))
}

fn parity_command(args: &[&str]) -> CliOutput {
    let parsed =
        (|| -> Result<(std::path::PathBuf, std::path::PathBuf, std::path::PathBuf), String> {
            let mut root = None;
            let mut report = None;
            let mut compared = None;
            let mut seen = BTreeSet::new();
            let mut i = 0;
            while i < args.len() {
                let key = args[i];
                if !seen.insert(key) {
                    return Err(format!("duplicate argument {key}"));
                }
                if !matches!(key, "--host-root" | "--report" | "--compared-report") {
                    return Err(format!("unknown argument {key}"));
                }
                let value = args.get(i + 1).ok_or("missing argument value")?;
                match key {
                    "--host-root" => root = Some(PathBuf::from(value)),
                    "--report" => report = Some(PathBuf::from(value)),
                    "--compared-report" => compared = Some(PathBuf::from(value)),
                    _ => unreachable!(),
                }
                i += 2;
            }
            let root = fs::canonicalize(root.ok_or("missing --host-root")?)
                .map_err(|e| format!("E_PARITY_INPUT: {e}"))?;
            let report = report.ok_or("missing --report")?;
            let compared = compared.ok_or("missing --compared-report")?;
            if report.is_absolute() || compared.is_absolute() {
                return Err("E_PARITY_INPUT: report paths must be relative to host root".into());
            }
            let report_text = report.to_str().ok_or("non UTF-8 report path")?;
            let compared_text = compared.to_str().ok_or("non UTF-8 compared report path")?;
            let report_resolved = no_symlink_parents(&root, report_text)?;
            let compared_resolved = no_symlink_parents(&root, compared_text)?;
            if report_resolved == compared_resolved {
                return Err("E_PARITY_INPUT: reports must be distinct".into());
            }
            Ok((root, report, compared))
        })();
    let (root, report_path, compared_path) = match parsed {
        Ok(v) => v,
        Err(e) => {
            return CliOutput {
                exit_code: 2,
                stdout: String::new(),
                stderr: format!("E_PARITY: {e}\n"),
            };
        }
    };
    match parity::update_files(&root, &report_path, &compared_path) {
        Ok((current, established)) => CliOutput {
            exit_code: if established { 0 } else { 2 },
            stdout: serde_json::to_string_pretty(&current).unwrap_or_default() + "\n",
            stderr: if established {
                String::new()
            } else {
                current["unresolved"]["/parity"]
                    .as_str()
                    .unwrap_or("E_PARITY_UNESTABLISHED")
                    .to_owned()
                    + "\n"
            },
        },
        Err(error) => parity_error(format!("E_PARITY_OUTPUT: {error}")),
    }
}

fn parity_error(error: String) -> CliOutput {
    CliOutput {
        exit_code: 2,
        stdout: String::new(),
        stderr: format!("E_PARITY: {error}\n"),
    }
}
