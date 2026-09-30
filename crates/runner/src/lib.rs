//! Generic external judge transport. Host policy belongs to the registered judge.
mod checkout;
pub mod facts;
mod facts_binding;
mod facts_configs;
mod facts_inputs;
pub mod full;
pub mod initial;
pub mod input_file;
pub mod observation;
pub mod parity;
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
    finish_process(run_process_inner(root, s, input, None)?)
}
/// v1 uses the same bounded engine with a cleared environment and prelaunch binding.
pub fn run_process_bound(
    root: &Path,
    s: &CommandSpec,
    input: &[u8],
    digest: &str,
) -> Result<ProcessResult, String> {
    finish_process(run_process_inner(root, s, input, Some(digest))?)
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
    run_process_inner(root, s, input, Some(digest))
}
fn run_process_inner(
    root: &Path,
    s: &CommandSpec,
    input: &[u8],
    expected: Option<&str>,
) -> Result<ProcessResult, String> {
    validate_command(s)?;
    if expected.is_some() && !Path::new(&s.program).is_absolute() {
        return Err("bound executable must be absolute; no ambient PATH resolution".into());
    }
    let executable = resolve_program(root, &s.program, s.env.get("PATH").map(String::as_str))?;
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let environment = if expected.is_some() {
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
    let mut child = command
        .spawn()
        .map_err(|e| format!("{}: {e}", executable.display()))?;
    let pid = child.id();
    let stdin = child.stdin.take().ok_or("missing process stdin")?;
    let input = input.to_vec();
    let writer = std::thread::spawn(move || {
        let mut stdin = stdin;
        stdin.write_all(&input)
    });
    let exceeded = Arc::new(AtomicBool::new(false));
    let limit = s.output_limit_bytes;
    fn reader<R: Read + Send + 'static>(
        mut r: R,
        limit: usize,
        flag: Arc<AtomicBool>,
    ) -> std::thread::JoinHandle<std::io::Result<Vec<u8>>> {
        std::thread::spawn(move || {
            let mut out = Vec::new();
            let mut buf = [0u8; 8192];
            loop {
                let n = r.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                if out.len() + n > limit {
                    flag.store(true, Ordering::Relaxed);
                }
                let keep = n.min(limit.saturating_sub(out.len()));
                out.extend_from_slice(&buf[..keep]);
            }
            Ok(out)
        })
    }
    let stdout = reader(
        child.stdout.take().ok_or("missing stdout")?,
        limit,
        exceeded.clone(),
    );
    let stderr = reader(
        child.stderr.take().ok_or("missing stderr")?,
        limit,
        exceeded.clone(),
    );
    let start = Instant::now();
    let mut failure = None;
    let mut status = loop {
        if exceeded.load(Ordering::Relaxed)
            || start.elapsed() >= Duration::from_secs(s.timeout_seconds)
        {
            failure = Some(if exceeded.load(Ordering::Relaxed) {
                "process output limit exceeded"
            } else {
                "process timed out"
            });
            break None;
        }
        if let Some(v) = child.try_wait().map_err(|e| e.to_string())? {
            break Some(v);
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    // Kill lingering descendants even when their direct parent has exited; pipe readers remain bounded.
    #[cfg(unix)]
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }
    if status.is_none() {
        let _ = child.kill();
        status = child.wait().ok();
    }
    let a = stdout
        .join()
        .map_err(|_| "stdout reader panicked")?
        .map_err(|e| e.to_string())?;
    let b = stderr
        .join()
        .map_err(|_| "stderr reader panicked")?
        .map_err(|e| e.to_string())?;
    let _ = writer.join();
    if exceeded.load(Ordering::Relaxed) {
        failure = Some("process output limit exceeded");
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
        failure: failure.map(str::to_owned),
        exit_code: status.and_then(|s| s.code()).unwrap_or(-1),
        executable,
        sha256: hash,
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
fn root_for_config(path: &Path) -> Result<(PathBuf, String), String> {
    let full = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    let full = fs::canonicalize(full).map_err(|e| e.to_string())?;
    for p in full.ancestors().skip(1) {
        if p.file_name().is_some_and(|n| n == ".chrono-harness") {
            let root = p.parent().ok_or("no host root")?.to_path_buf();
            let rel = full
                .strip_prefix(&root)
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("non UTF-8 config path")?
                .to_owned();
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
    []|["help"]|["--help"]|["-h"]=>CliOutput{exit_code:0,stdout:"chrono-harness check --config P --base FULL_OID --candidate FULL_OID\nchrono-harness parity --host-root H --report P --compared-report P\nUse --initial without --base for a parentless candidate. Use --context P for chrono-judge/v1 external judges. The parity command adds fail-closed evidence to two completed full reports; it never changes the canonical check command. Use an explicit chrono-initial-check/v1 profile for root registry inventory. Seven-judge governance NOT IMPLEMENTED.\n".into(),stderr:String::new()},
    ["--version"]|["-V"]=>CliOutput{exit_code:0,stdout:format!("chrono-harness {}\n",env!("CARGO_PKG_VERSION")),stderr:String::new()},
    ["spec","status"]=>CliOutput{exit_code:0,stdout:"SPEC_STATUS=draft\nENFORCEMENT=not-implemented\nHOST_REGISTRIES=proposed\nCI_CHECK=chrono-ci-check/v1\nV1_TRANSPORT=implemented\nREGISTRATION=implemented\nCONTRACT=SPEC.md\n".into(),stderr:String::new()},
    ["check",rest @ ..]=>match check(rest, entry){Ok((code,s))=>CliOutput{exit_code:code,stdout:s,stderr:String::new()},Err(e)=>CliOutput{exit_code:2,stdout:String::new(),stderr:format!("E_CHECK: {e}\n")}},
    ["parity",rest @ ..]=>parity_command(rest),
    _=>CliOutput{exit_code:2,stdout:String::new(),stderr:"E_USAGE: use --help\n".into()}
}
}
fn check(args: &[&str], entry: Value) -> Result<(u8, String), String> {
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
    let (root, config_path) = root_for_config(Path::new(config.ok_or("missing --config")?))?;
    let profile = json(&fs::read(root.join(&config_path)).map_err(|e| e.to_string())?)?;
    if scope.is_some() && profile["schema"] != units::PROFILE {
        return Err("unit/collect selection requires chrono-ci-check/v3".into());
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
        return full::check_observed(
            &root,
            &config_path,
            base.as_deref()
                .ok_or("full check requires --base; initial mode has no governance success")?,
            &candidate,
            Path::new(context.ok_or("full check requires --context")?),
            entry,
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
        observations: serde_json::json!({"entry":entry}),
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
    let report_path = no_symlink_parents(&root, &output_path)?;
    let proc = run_process(
        &root,
        &c.judge,
        &serde_json::to_vec(&req).map_err(|e| e.to_string())?,
    );
    let runner_executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let runner_identity = serde_json::json!({"path":runner_executable,"sha256":sha256(&fs::read(&runner_executable).map_err(|e|e.to_string())?),"version":env!("CARGO_PKG_VERSION")});
    let mut report = match proc {
        Ok(p) => {
            let response: Result<Response, String> = decode(&p.stdout_bytes);
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
    let text = if c.schema == units::PROFILE {
        serde_json::to_string(&report)
    } else {
        serde_json::to_string_pretty(&report)
    }
    .map_err(|e| e.to_string())?
        + "\n";
    fs::create_dir_all(report_path.parent().ok_or("report parent missing")?)
        .map_err(|e| e.to_string())?;
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
