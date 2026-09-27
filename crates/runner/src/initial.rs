//! Explicit single-endpoint inventory transport. It never constructs a DELTA base.
use crate::{
    CommandSpec, ProcessResult, decode, facts, full, no_symlink_parents, run_process_observed,
    sha256,
    wire::{self, Binding, Checkout, Endpoint, Executable, Response, Status, TransportFailure},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json as value};
use std::{collections::BTreeMap, fs, path::Path};

pub const PROFILE: &str = "chrono-initial-check/v1";
pub const PROTOCOL: &str = "chrono-initial-judge/v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub schema: String,
    pub host_config: String,
    pub judges: Vec<Binding>,
    pub timeout_seconds: u64,
    pub stdout_limit_bytes: usize,
}
impl Profile {
    pub fn validate(&self) -> Result<(), String> {
        crate::relative_path(&self.host_config)?;
        if self.schema != PROFILE
            || !self.host_config.starts_with(".chrono-harness/")
            || self.timeout_seconds == 0
            || !(1..=64 * 1024 * 1024).contains(&self.stdout_limit_bytes)
        {
            return Err("invalid initial profile/config/limits".into());
        }
        for binding in &self.judges {
            crate::relative_path(&binding.executable)?;
            if binding.version.is_empty() || !binding.sha256.as_deref().is_some_and(wire::is_digest)
            {
                return Err(
                    "initial judge requires an explicit version and executable digest".into(),
                );
            }
        }
        full::schedule_mode(&self.judges, "every-initial", "inventory")?;
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub protocol: String,
    pub request_id: String,
    pub judge_id: String,
    pub mode: String,
    pub candidate: Endpoint,
    pub profile_path: String,
    pub profile_sha256: String,
    pub config_path: String,
    pub registry_digest: String,
    pub checkout: Checkout,
    pub runner: Executable,
    pub observations: Value,
    pub prior_results: Vec<Response>,
}
impl Request {
    fn identity(&self) -> Result<String, String> {
        let mut v = serde_json::to_value(self).map_err(|e| e.to_string())?;
        v.as_object_mut()
            .ok_or("request object")?
            .remove("request_id");
        wire::digest(&v)
    }
    pub fn seal(&mut self) -> Result<(), String> {
        self.request_id = self.identity()?;
        Ok(())
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.protocol != PROTOCOL
            || self.mode != "inventory"
            || self.judge_id.is_empty()
            || self.request_id != self.identity()?
        {
            return Err("E_PROTOCOL: initial request identity/mode".into());
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

pub fn invoke(
    req: &Request,
    b: &Binding,
    env: &BTreeMap<String, String>,
    timeout: u64,
    limit: usize,
) -> Result<(Response, ProcessResult), TransportFailure> {
    req.validate()?;
    if b.id != req.judge_id
        || b.selector != "every-initial"
        || b.modes != ["inventory"]
        || b.argv.iter().any(|a| a == "{base}")
    {
        return Err("E_PROTOCOL: initial binding identity/base placeholder".into());
    }
    let hash = b
        .sha256
        .as_deref()
        .filter(|v| wire::is_digest(v))
        .ok_or("E_PROTOCOL: missing executable digest")?;
    let path = no_symlink_parents(&req.candidate.root, &b.executable)?;
    let spec = CommandSpec {
        program: path.to_str().ok_or("executable UTF-8")?.into(),
        args: b
            .argv
            .iter()
            .map(|a| {
                if a == "{candidate}" {
                    req.candidate.commit.clone()
                } else {
                    a.clone()
                }
            })
            .collect(),
        env: env.clone(),
        timeout_seconds: timeout,
        output_limit_bytes: limit,
    };
    let p = run_process_observed(&req.candidate.root, &spec, &wire::canonical(req)?, hash)?;
    if let Some(message) = p.failure.clone() {
        return Err(TransportFailure {
            message,
            process: Some(p),
        });
    }
    let response = decode::<Response>(&p.stdout_bytes).and_then(|r| {
        wire::validate_response_at(
            &req.candidate.root,
            PROTOCOL,
            &req.request_id,
            &req.judge_id,
            &r,
            p.exit_code,
            |s| s.starts_with('/'),
        )?;
        Ok(r)
    });
    match response {
        Ok(r) => Ok((r, p)),
        Err(message) => Err(TransportFailure {
            message,
            process: Some(p),
        }),
    }
}

pub fn check(
    root: &Path,
    profile_path: &str,
    candidate: &str,
    entry: Value,
) -> Result<(u8, String), String> {
    let tree = facts::verify_oid(root, candidate)?;
    if !facts::parents(root, candidate)?.is_empty() {
        return Err("initial inventory requires parentless commit".into());
    }
    let bytes = fs::read(no_symlink_parents(root, profile_path)?).map_err(|e| e.to_string())?;
    let profile: Profile = decode(&bytes)?;
    profile.validate()?;
    let registries = facts::registry_values(root, candidate, &profile.host_config)?;
    let cfg = &registries[&profile.host_config];
    let mut environment = BTreeMap::<String, String>::new();
    let mut inherited = BTreeMap::<String, Option<String>>::new();
    for key in cfg["environment"]["inherit"]
        .as_array()
        .ok_or("environment.inherit")?
    {
        let key = key.as_str().ok_or("environment key")?;
        let value = std::env::var(key).ok();
        inherited.insert(key.into(), value.clone());
        if let Some(value) = value {
            environment.insert(key.into(), value);
        }
    }
    for (key, val) in cfg["environment"]["values"]
        .as_object()
        .ok_or("environment.values")?
    {
        environment.insert(key.clone(), val.as_str().ok_or("environment value")?.into());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let runner = Executable {
        path: exe.to_str().ok_or("runner UTF-8")?.into(),
        sha256: sha256(&fs::read(&exe).map_err(|e| e.to_string())?),
        version: env!("CARGO_PKG_VERSION").into(),
    };
    let mut request = Request {
        protocol: PROTOCOL.into(),
        request_id: String::new(),
        judge_id: String::new(),
        mode: "inventory".into(),
        candidate: Endpoint {
            commit: candidate.into(),
            tree: tree.clone(),
            root: root.into(),
        },
        profile_path: profile_path.into(),
        profile_sha256: sha256(&bytes),
        config_path: profile.host_config.clone(),
        registry_digest: wire::digest(&registries)?,
        checkout: facts::checkout(root, candidate)?,
        runner: runner.clone(),
        observations: value!({"entry":entry,"environment":{"inherited":inherited,"effective":environment}}),
        prior_results: vec![],
    };
    let mut responses = BTreeMap::<String, Response>::new();
    let mut records = vec![];
    let mut status = Status::Pass;
    for binding in full::schedule_mode(&profile.judges, "every-initial", "inventory")? {
        let blocked: Vec<_> = binding
            .after
            .iter()
            .filter(|id| responses.get(*id).is_none_or(|r| r.status >= Status::Fail))
            .cloned()
            .collect();
        if !blocked.is_empty() {
            records.push(value!({"id":binding.id,"binding":binding,"state":"blocked","blocked_by":blocked,"exit_code":null}));
            status = Status::Error;
            continue;
        }
        request.judge_id = binding.id.clone();
        request.prior_results = binding
            .after
            .iter()
            .map(|id| responses[id].clone())
            .collect();
        request.observations["judges"] = value!(records);
        request.seal()?;
        match invoke(
            &request,
            &binding,
            &environment,
            profile.timeout_seconds,
            profile.stdout_limit_bytes,
        ) {
            Ok((response, process)) => {
                status = status.max(response.status.clone());
                records.push(value!({"id":binding.id,"binding":binding,"state":"executed","request_id":request.request_id,
                    "request_digest":wire::digest(&request)?,"exit_code":process.exit_code,"response":response,"process":process}));
                responses.insert(binding.id, response);
            }
            Err(failure) => {
                status = Status::Error;
                records.push(value!({"id":binding.id,"binding":binding,"state":if failure.process.is_some(){"executed"}else{"error"},
                    "request_id":request.request_id,"transport_failure":failure.message,"process":failure.process}));
            }
        }
    }
    let state = no_symlink_parents(root, ".chrono-harness/state")?;
    fs::create_dir_all(&state).map_err(|e| e.to_string())?;
    let report = value!({"schema":"chrono-initial-report/v1","scope":"initial-inventory",
        "status":match status {Status::Pass|Status::Warn=>"complete",Status::Fail=>"failed",Status::Error=>"error"},
        "base":null,"delta":null,"candidate":candidate,"candidate_tree":tree,"governance":"not-evaluated",
        "previous_enforcement":"none","parity":"unestablished","profile_path":profile_path,"profile_sha256":request.profile_sha256,
        "registry_digest":request.registry_digest,"runner":runner,"environment":request.observations["environment"],"judges":records});
    let text = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())? + "\n";
    let unique = no_symlink_parents(
        root,
        &format!(
            ".chrono-harness/state/initial-{}.json",
            wire::digest(&report)?
        ),
    )?;
    if unique.exists() {
        if fs::read(&unique).map_err(|e| e.to_string())? != text.as_bytes() {
            return Err("initial report identity collision".into());
        }
    } else {
        use std::io::Write;
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&unique)
            .map_err(|e| e.to_string())?
            .write_all(text.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    fs::write(
        no_symlink_parents(root, ".chrono-harness/state/initial-report.json")?,
        &text,
    )
    .map_err(|e| e.to_string())?;
    Ok((status.exit_code() as u8, text))
}
