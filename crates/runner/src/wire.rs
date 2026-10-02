//! Shared chrono-judge/v1 wire types and transport. No host admissibility policy.
use crate::{CommandSpec, ProcessResult, decode, no_symlink_parents, run_process_observed, sha256};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, fs, path::PathBuf};
pub const PROTOCOL: &str = "chrono-judge/v1";
pub fn canonical<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    serde_jcs::to_vec(value).map_err(|e| e.to_string())
}
pub fn digest<T: Serialize>(value: &T) -> Result<String, String> {
    Ok(sha256(&canonical(value)?))
}
pub fn is_digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Pass,
    Warn,
    Fail,
    Error,
}
impl Status {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Pass | Self::Warn => 0,
            Self::Fail => 1,
            Self::Error => 2,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub code: String,
    pub level: String,
    pub message: String,
    pub delta_refs: Vec<String>,
    pub causes: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub path: String,
    pub sha256: String,
    pub kind: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub protocol: String,
    pub request_id: String,
    pub judge_id: String,
    pub status: Status,
    pub findings: Vec<Finding>,
    pub evidence: Vec<Evidence>,
    pub outputs: BTreeMap<String, Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    pub commit: String,
    pub tree: String,
    pub root: PathBuf,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Delta {
    pub kind: String,
    pub path: String,
    pub old_blob: Option<String>,
    pub new_blob: Option<String>,
    pub old_mode: Option<String>,
    pub new_mode: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registries {
    pub base: PathBuf,
    pub candidate: PathBuf,
    pub digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub path: PathBuf,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Checkout {
    pub head: String,
    pub tracked: Vec<String>,
    pub untracked: Vec<String>,
    pub index_flags: Vec<IndexFlag>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct IndexFlag {
    pub path: String,
    pub tag: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Executable {
    pub path: String,
    pub sha256: String,
    pub version: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// Optional full execution selector.  It is omitted from legacy requests so
    /// v1/v2 consumers retain their original wire shape; a selector is carried
    /// in the sealed request and therefore cannot silently become a full run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<crate::units::Scope>,
    #[serde(default)]
    pub observations: Value,
    pub protocol: String,
    pub request_id: String,
    pub judge_id: String,
    pub mode: String,
    pub base: Endpoint,
    pub candidate: Endpoint,
    pub delta: Vec<Delta>,
    pub registries: Registries,
    pub context: Context,
    pub impact: Value,
    pub prior_results: Vec<Response>,
    pub config_path: String,
    pub checkout: Checkout,
    pub runner: Executable,
}
impl Request {
    fn identity(&self) -> Result<String, String> {
        Ok(sha256(&self.canonical_projection(None)?))
    }
    /// Original canonical stdin bytes, without expanding borrowed observations
    /// into another owned JSON tree. This preserves the v1 wire representation.
    pub fn canonical(&self) -> Result<Vec<u8>, String> {
        self.canonical_projection(Some(&self.request_id))
    }
    fn canonical_projection(&self, request_id: Option<&str>) -> Result<Vec<u8>, String> {
        // Keep the large observations borrowed while encoding the typed header.
        // The exhaustive destructuring keeps
        // a new request field from silently escaping the identity contract.
        #[derive(Serialize)]
        struct Projection<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            scope: Option<&'a crate::units::Scope>,
            #[serde(skip_serializing_if = "Option::is_none")]
            request_id: Option<&'a str>,
            protocol: &'a str,
            judge_id: &'a str,
            mode: &'a str,
            base: &'a Endpoint,
            candidate: &'a Endpoint,
            delta: &'a [Delta],
            registries: &'a Registries,
            context: &'a Context,
            impact: &'a Value,
            prior_results: &'a [Response],
            config_path: &'a str,
            checkout: &'a Checkout,
            runner: &'a Executable,
        }
        let Self {
            scope,
            observations,
            protocol,
            request_id: _,
            judge_id,
            mode,
            base,
            candidate,
            delta,
            registries,
            context,
            impact,
            prior_results,
            config_path,
            checkout,
            runner,
        } = self;
        crate::canonical_request::encode(
            &Projection {
                scope: scope.as_ref(),
                request_id,
                protocol,
                judge_id,
                mode,
                base,
                candidate,
                delta,
                registries,
                context,
                impact,
                prior_results,
                config_path,
                checkout,
                runner,
            },
            observations,
        )
    }
    pub fn seal(&mut self) -> Result<(), String> {
        self.request_id = self.identity()?;
        Ok(())
    }
    pub fn validate(&self) -> Result<(), String> {
        self.validate_header()?;
        if self.request_id != self.identity()? {
            return Err("E_PROTOCOL: request identity/mode".into());
        }
        Ok(())
    }
    fn validate_header(&self) -> Result<(), String> {
        if self.protocol != PROTOCOL || self.mode != "evaluate" || self.judge_id.is_empty() {
            return Err("E_PROTOCOL: request identity/mode".into());
        }
        Ok(())
    }
    pub fn response(&self, status: Status) -> Response {
        Response {
            protocol: PROTOCOL.into(),
            request_id: self.request_id.clone(),
            judge_id: self.judge_id.clone(),
            status,
            findings: vec![],
            evidence: vec![],
            outputs: BTreeMap::new(),
        }
    }
}
/// An invocation-owned request whose seal and original stdin were produced
/// together. Neither its request nor its bytes can be mutated by the caller.
pub(crate) struct PreparedRequest {
    request: Request,
    stdin: Vec<u8>,
}
impl PreparedRequest {
    pub(crate) fn new(mut request: Request) -> Result<Self, String> {
        request.seal()?;
        let stdin = request.canonical()?;
        Ok(Self { request, stdin })
    }
    pub(crate) fn request(&self) -> &Request {
        &self.request
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub id: String,
    pub executable: String,
    pub version: String,
    pub sha256: Option<String>,
    pub argv: Vec<String>,
    pub selector: String,
    pub after: Vec<String>,
    pub modes: Vec<String>,
}

pub fn validate_response(req: &Request, r: &Response, exit: i32) -> Result<(), String> {
    validate_response_at(
        &req.candidate.root,
        PROTOCOL,
        &req.request_id,
        &req.judge_id,
        r,
        exit,
        |p| p.starts_with('/') || req.delta.iter().any(|d| d.path == p),
    )
}
/// Shared response contract; the request protocol owns admissible finding references.
pub(crate) fn validate_response_at(
    root: &std::path::Path,
    protocol: &str,
    request_id: &str,
    judge_id: &str,
    r: &Response,
    exit: i32,
    reference: impl Fn(&str) -> bool,
) -> Result<(), String> {
    if r.protocol != protocol
        || r.request_id != request_id
        || r.judge_id != judge_id
        || exit != r.status.exit_code()
    {
        return Err("E_PROTOCOL: response identity/status/exit mismatch".into());
    }
    for f in &r.findings {
        if f.code.is_empty()
            || f.message.is_empty()
            || !matches!(f.level.as_str(), "info" | "warning" | "error")
            || f.delta_refs.is_empty()
            || f.delta_refs.iter().any(|p| !reference(p))
        {
            return Err("E_PROTOCOL: invalid finding".into());
        }
    }
    let errors = r.findings.iter().any(|f| f.level == "error");
    let warnings = r.findings.iter().any(|f| f.level == "warning");
    if (matches!(r.status, Status::Fail | Status::Error) != errors)
        || (r.status == Status::Pass && warnings)
        || (r.status == Status::Warn && !warnings)
    {
        return Err("E_PROTOCOL: finding/status disagreement".into());
    }
    for e in &r.evidence {
        if !e.path.starts_with(".chrono-harness/state/")
            || e.kind.is_empty()
            || !is_digest(&e.sha256)
        {
            return Err("E_PROTOCOL: invalid evidence identity".into());
        }
        let p = no_symlink_parents(root, &e.path)?;
        if !p.is_file()
            || sha256(&fs::read(&p).map_err(|e| format!("E_PROTOCOL: evidence: {e}"))?) != e.sha256
        {
            return Err("E_PROTOCOL: missing/tampered evidence".into());
        }
    }
    Ok(())
}
#[derive(Debug)]
pub struct TransportFailure {
    pub message: String,
    pub process: Option<ProcessResult>,
}
impl From<String> for TransportFailure {
    fn from(message: String) -> Self {
        Self {
            message,
            process: None,
        }
    }
}
impl From<&str> for TransportFailure {
    fn from(message: &str) -> Self {
        message.to_string().into()
    }
}
pub fn invoke(
    req: &Request,
    b: &Binding,
    env: &BTreeMap<String, String>,
    timeout: u64,
    limit: usize,
) -> Result<(Response, ProcessResult), String> {
    invoke_detailed(req, b, env, timeout, limit).map_err(|e| e.message)
}
pub fn invoke_detailed(
    req: &Request,
    b: &Binding,
    env: &BTreeMap<String, String>,
    timeout: u64,
    limit: usize,
) -> Result<(Response, ProcessResult), TransportFailure> {
    req.validate()?;
    invoke_bytes(req, b, env, timeout, limit, &req.canonical()?)
}
pub(crate) fn invoke_prepared(
    req: &PreparedRequest,
    b: &Binding,
    env: &BTreeMap<String, String>,
    timeout: u64,
    limit: usize,
) -> Result<(Response, ProcessResult), TransportFailure> {
    req.request.validate_header()?;
    invoke_bytes(&req.request, b, env, timeout, limit, &req.stdin)
}
fn invoke_bytes(
    req: &Request,
    b: &Binding,
    env: &BTreeMap<String, String>,
    timeout: u64,
    limit: usize,
    stdin: &[u8],
) -> Result<(Response, ProcessResult), TransportFailure> {
    if b.id != req.judge_id {
        return Err("E_PROTOCOL: binding identity".into());
    }
    let hash = b
        .sha256
        .as_deref()
        .filter(|v| is_digest(v))
        .ok_or("E_PROTOCOL: missing executable digest")?;
    let path = no_symlink_parents(&req.candidate.root, &b.executable)?;
    let spec = CommandSpec {
        program: path.to_str().ok_or("non UTF-8 executable")?.into(),
        args: b
            .argv
            .iter()
            .map(|arg| match arg.as_str() {
                "{base}" => req.base.commit.clone(),
                "{candidate}" => req.candidate.commit.clone(),
                _ => arg.clone(),
            })
            .collect(),
        env: env.clone(),
        timeout_seconds: timeout,
        output_limit_bytes: limit,
    };
    let p = run_process_observed(&req.candidate.root, &spec, stdin, hash)?;
    if let Some(message) = p.failure.clone() {
        return Err(TransportFailure {
            message,
            process: Some(p),
        });
    }
    let result = decode::<Response>(&p.stdout_bytes).and_then(|response| {
        validate_response(req, &response, p.exit_code)?;
        Ok(response)
    });
    match result {
        Ok(response) => Ok((response, p)),
        Err(message) => Err(TransportFailure {
            message: format!(
                "E_PROTOCOL: {message}; exit={}; stderr={}",
                p.exit_code, p.stderr
            ),
            process: Some(p),
        }),
    }
}
