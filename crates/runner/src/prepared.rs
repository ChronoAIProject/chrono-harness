//! Fixed check acquisition using registered producer actions and the existing transport.
use crate::{
    CommandSpec, ProcessResult, decode, facts, file_identity, no_symlink_parents, observation,
    relative_path, run_process_observed, sha256, units, wire,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

#[path = "prepared_binding.rs"]
mod binding;
pub use binding::validate_binding;
#[path = "prepared_artifacts.rs"]
mod artifacts;
pub use artifacts::*;

pub const SOURCE: &str = "CHRONO_CHECK_SOURCE";
pub const REQUEST: &str = "chrono-check-input-request/v1";
pub const RESPONSE: &str = "chrono-check-inputs/v1";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub operation: String,
    pub tool: String,
    pub argv: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inputs {
    pub local: Action,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ci: Option<Action>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Canonical {
    pub operation: String,
    pub argv: Vec<String>,
    pub profile: String,
    pub inputs: Inputs,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Selection {
    All,
    Unit { unit: String },
    Collect,
}
impl Selection {
    pub fn argv(&self) -> Vec<String> {
        match self {
            Self::All => vec![],
            Self::Unit { unit } => vec!["--unit".into(), unit.clone()],
            Self::Collect => vec!["--collect".into()],
        }
    }
    pub fn matches(&self, scope: &Option<units::Scope>) -> bool {
        match (self, scope) {
            (Self::All, None) | (Self::Collect, Some(units::Scope::Collect { .. })) => true,
            (Self::Unit { unit: a }, Some(units::Scope::Unit { unit: b })) => a == b,
            _ => false,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputRequest {
    pub schema: String,
    pub host_root: PathBuf,
    pub source: String,
    pub host_config: String,
    pub host_config_sha256: String,
    pub effective_config: String,
    pub effective_config_sha256: String,
    pub profile: String,
    pub profile_sha256: String,
    pub selection: Selection,
    pub native_artifacts: Option<NativeArtifacts>,
    // Local collection uses the CI collection owner after local endpoints were observed.
    pub prepared: Option<PreparedCheck>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub path: String,
    pub raw: Vec<u8>,
    pub sha256: String,
    pub semantic_digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedCheck {
    pub schema: String,
    pub request_sha256: String,
    pub source: String,
    pub profile: String,
    pub base: Option<String>,
    pub candidate: String,
    pub initial: bool,
    pub context: Option<Context>,
    pub scope: Option<units::Scope>,
    pub evidence: Value,
    pub originals: Vec<Original>,
}
impl InputRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != REQUEST
            || !matches!(self.source.as_str(), "local" | "ci")
            || !self.host_root.is_absolute()
        {
            return Err("invalid check input request/source/root".into());
        }
        for p in [&self.host_config, &self.effective_config, &self.profile] {
            relative_path(p)?;
            if !p.starts_with(".chrono-harness/") || p.starts_with(".chrono-harness/state/") {
                return Err("check input policy must be beneath .chrono-harness".into());
            }
        }
        for (p, h) in [
            (&self.host_config, &self.host_config_sha256),
            (&self.effective_config, &self.effective_config_sha256),
            (&self.profile, &self.profile_sha256),
        ] {
            if file_identity(&no_symlink_parents(&self.host_root, p)?)?.0 != *h {
                return Err(format!("check input policy drift: {p}"));
            }
        }
        let (effective, bytes, _, _) =
            crate::facts_configs::load(&self.host_root, &self.host_config)?;
        if effective != self.effective_config || sha256(&bytes) != self.effective_config_sha256 {
            return Err("check input effective selector identity mismatch".into());
        }
        let cfg = crate::json(
            &fs::read(no_symlink_parents(&self.host_root, &self.effective_config)?)
                .map_err(|e| e.to_string())?,
        )?;
        let c = declaration(&cfg)?;
        if c.profile != self.profile {
            return Err("check input profile disagrees with binding".into());
        }
        let dir = retention_directory(self);
        if !cfg["artifacts"]
            .as_array()
            .ok_or("artifacts missing")?
            .iter()
            .any(|a| {
                a["tracked"] == false && a["path"].as_str().is_some_and(|p| dir.starts_with(p))
            })
        {
            return Err("short retention directory lacks declared artifact ownership".into());
        }
        if native_artifacts(&self.host_root, &cfg, &self.source, &self.selection)?
            != self.native_artifacts
        {
            return Err("check input producer/upload contract mismatch".into());
        }
        if let Selection::Unit { unit } = &self.selection {
            units::id(unit)?;
        }
        Ok(())
    }
}
pub fn declaration(config: &Value) -> Result<Canonical, String> {
    if config["schema_version"] != 4 {
        return Err("short check requires config schema_version 4".into());
    }
    let c: Canonical =
        serde_json::from_value(config["canonical_check"].clone()).map_err(|e| e.to_string())?;
    if c.operation != "validate.delta"
        || c.argv
            != vec![
                config["runner"]["path"]
                    .as_str()
                    .ok_or("runner path missing")?
                    .to_owned(),
                "check".into(),
            ]
    {
        return Err("short canonical_check must declare runner check".into());
    }
    relative_path(&c.profile)?;
    if !c.profile.starts_with(".chrono-harness/") || c.profile.starts_with(".chrono-harness/state/")
    {
        return Err("short profile must be a host policy".into());
    }
    let inherit: Vec<String> = serde_json::from_value(config["environment"]["inherit"].clone())
        .map_err(|e| e.to_string())?;
    if !inherit.iter().any(|s| s == SOURCE) || config["environment"]["values"].get(SOURCE).is_some()
    {
        return Err("short check must inherit CHRONO_CHECK_SOURCE without a fixed override".into());
    }
    credential_environment(config)?;
    if !config["artifacts"]
        .as_array()
        .ok_or("artifacts missing")?
        .iter()
        .any(|a| {
            a["tracked"] == false
                && a["path"].as_str().is_some_and(|p| {
                    ".chrono-harness/state/preparation/".starts_with(p)
                        || p.starts_with(".chrono-harness/state/")
                })
        })
    {
        return Err("short preparation lacks declared artifact ownership".into());
    }
    let mut operations = BTreeSet::new();
    for a in std::iter::once(&c.inputs.local).chain(c.inputs.ci.iter()) {
        if !operations.insert(&a.operation)
            || a.operation.is_empty()
            || a.tool.is_empty()
            || a.argv.is_empty()
            || a.argv.iter().any(|s| s.contains('\0'))
        {
            return Err("invalid check input action".into());
        }
        let hits = config["tools"]
            .as_array()
            .ok_or("missing tools")?
            .iter()
            .filter(|t| t["id"] == a.tool)
            .count();
        if hits != 1 {
            return Err(format!("missing/ambiguous input producer tool {}", a.tool));
        }
    }
    Ok(c)
}
/// Acquisition credentials are declared in the existing environment policy and never forwarded to judges.
pub fn credential_environment(cfg: &Value) -> Result<BTreeSet<String>, String> {
    let names: Vec<String> = match cfg["environment"].get("credential_environment") {
        Some(v) => serde_json::from_value(v.clone()).map_err(|e| e.to_string())?,
        None => vec![],
    };
    let mut out = BTreeSet::new();
    for name in names {
        if name == SOURCE
            || name.is_empty()
            || name.contains(['=', '\0'])
            || !out.insert(name.clone())
            || !cfg["environment"]["inherit"]
                .as_array()
                .is_some_and(|a| a.iter().any(|k| k == &name))
            || cfg["environment"]["values"].get(&name).is_some()
        {
            return Err("invalid acquisition credential environment binding".into());
        }
    }
    Ok(out)
}
fn environment_hashes(env: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    env.iter()
        .map(|(k, v)| (k.clone(), sha256(v.as_bytes())))
        .collect()
}
fn process_identities(mut process: ProcessResult) -> ProcessResult {
    process.environment = environment_hashes(&process.environment);
    process
}
fn environment(cfg: &Value) -> Result<(BTreeMap<String, String>, Value), String> {
    let mut env = BTreeMap::new();
    let mut observed = serde_json::Map::new();
    let mut seen = BTreeSet::new();
    for k in cfg["environment"]["inherit"]
        .as_array()
        .ok_or("environment inherit missing")?
    {
        let k = k.as_str().ok_or("environment key")?;
        if !seen.insert(k) || k.is_empty() || k.contains(['=', '\0']) {
            return Err("invalid/duplicate environment inheritance".into());
        }
        match std::env::var(k) {
            Ok(v) => {
                observed.insert(k.into(), json!(sha256(v.as_bytes())));
                env.insert(k.into(), v);
            }
            Err(std::env::VarError::NotPresent) => {
                observed.insert(k.into(), Value::Null);
            }
            Err(_) => return Err(format!("non UTF-8 environment {k}")),
        }
    }
    for (k, v) in cfg["environment"]["values"]
        .as_object()
        .ok_or("environment values missing")?
    {
        env.insert(k.clone(), v.as_str().ok_or("environment value")?.into());
    }
    Ok((env, Value::Object(observed)))
}
fn invoke(
    root: &Path,
    cfg: &Value,
    a: &Action,
    env: &BTreeMap<String, String>,
    req: &InputRequest,
) -> Result<(PreparedCheck, Original), String> {
    let t = cfg["tools"]
        .as_array()
        .ok_or("missing tools")?
        .iter()
        .find(|t| t["id"] == a.tool)
        .ok_or("missing producer tool")?;
    let timeout = cfg["protocol"]["timeout_seconds"]
        .as_u64()
        .ok_or("producer timeout missing")?;
    let limit = cfg["protocol"]["stdout_limit_bytes"]
        .as_u64()
        .ok_or("producer bound missing")? as usize;
    let version: Vec<String> =
        serde_json::from_value(t["version_argv"].clone()).map_err(|e| e.to_string())?;
    let mut tool = observation::tool(
        root,
        t["program"].as_str().ok_or("producer program missing")?,
        &version,
        env,
        timeout,
        limit,
    )?;
    tool.version = process_identities(tool.version);
    if tool.version.failure.is_some()
        || tool.version.exit_code != 0
        || t["expected_version"]
            .as_str()
            .is_some_and(|v| tool.version.stdout.trim() != v)
    {
        return Err(format!(
            "input producer version mismatch: {}",
            serde_json::to_string(&tool).map_err(|e| e.to_string())?
        ));
    }
    let input = serde_json::to_vec(req).map_err(|e| e.to_string())?;
    let spec = CommandSpec {
        program: tool.path.to_str().ok_or("producer path UTF-8")?.into(),
        args: a.argv.clone(),
        env: env.clone(),
        timeout_seconds: timeout,
        output_limit_bytes: limit,
    };
    let process = process_identities(run_process_observed(root, &spec, &input, &tool.sha256)?);
    let receipt = json!({"action":a,"tool":tool,"request":req,"process":process,"environment_representation":"sha256"});
    // Preserve failed original transport before interpreting output.
    let original = retain_original(
        root,
        &retention_directory(req),
        "acquisition",
        &serde_json::to_vec(&receipt).map_err(|e| e.to_string())?,
    )?;
    if process.failure.is_some() || process.exit_code != 0 {
        return Err(format!(
            "input producer failed: {}",
            json!({"exit":process.exit_code,"failure":process.failure,"stdout":process.stdout,"stderr":process.stderr})
        ));
    }
    let p: PreparedCheck = decode(&process.stdout_bytes)?;
    validate_result(req, &p)?;
    Ok((p, original))
}
pub fn retain(root: &Path, prefix: &str, v: &Value) -> Result<String, String> {
    retain_bytes(
        root,
        prefix,
        &serde_json::to_vec(v).map_err(|e| e.to_string())?,
    )
}
pub fn retain_directory(root: &Path, directory: &str, prefix: &str) -> Result<PathBuf, String> {
    let dir = no_symlink_parents(root, directory)?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in(dir)
        .map_err(|e| e.to_string())?
        .keep())
}
pub fn retain_bytes(root: &Path, prefix: &str, bytes: &[u8]) -> Result<String, String> {
    use std::io::Write;
    let dir = no_symlink_parents(root, ".chrono-harness/state/preparation")?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut f = tempfile::Builder::new()
        .prefix(prefix)
        .suffix(".json")
        .tempfile_in(&dir)
        .map_err(|e| e.to_string())?;
    f.write_all(bytes).map_err(|e| e.to_string())?;
    let (_, path) = f.keep().map_err(|e| e.to_string())?;
    Ok(path
        .strip_prefix(root)
        .map_err(|e| e.to_string())?
        .to_str()
        .ok_or("evidence path UTF-8")?
        .into())
}
pub fn validate_result_identity(req: &InputRequest, p: &PreparedCheck) -> Result<(), String> {
    if p.schema != RESPONSE
        || p.request_sha256 != sha256(&serde_json::to_vec(req).map_err(|e| e.to_string())?)
        || p.source != req.source
        || p.profile != req.profile
        || p.initial == p.base.is_some()
        || !req.selection.matches(&p.scope)
        || !p.evidence.is_object()
        || p.evidence.as_object().is_none_or(|o| o.is_empty())
    {
        return Err("prepared check identity/scope mismatch".into());
    }
    facts::full_oid(&p.candidate)?;
    if let Some(base) = &p.base {
        facts::full_oid(base)?;
    }
    Ok(())
}
pub fn validate_result(req: &InputRequest, p: &PreparedCheck) -> Result<(), String> {
    validate_result_identity(req, p)?;
    validate_originals(&req.host_root, p, None)?;
    if let Some(a) = &req.native_artifacts {
        if p.originals
            .iter()
            .any(|o| !o.path.starts_with(&a.directory))
        {
            return Err("original evidence outside selected native upload root".into());
        }
    }
    facts::full_oid(&p.candidate)?;
    if let Some(b) = &p.base {
        facts::full_oid(b)?;
    }
    Ok(())
}
pub fn prepare(
    root: &Path,
    selection: Selection,
) -> Result<(InputRequest, PreparedCheck, Value), String> {
    let (effective, bytes, cfg, sel) =
        crate::facts_configs::load(root, ".chrono-harness/config.json")?;
    let c = declaration(&cfg)?;
    facts::git_declaration(&cfg)?;
    let (env, inherited) = environment(&cfg)?;
    let source = env.get(SOURCE).map(String::as_str).unwrap_or("local");
    if !matches!(source, "local" | "ci") {
        return Err("CHRONO_CHECK_SOURCE must be local or ci".into());
    }
    let mut req = InputRequest {
        schema: REQUEST.into(),
        host_root: root.into(),
        source: source.into(),
        host_config: ".chrono-harness/config.json".into(),
        host_config_sha256: file_identity(&no_symlink_parents(
            root,
            ".chrono-harness/config.json",
        )?)?
        .0,
        effective_config: effective,
        effective_config_sha256: sha256(&bytes),
        profile: c.profile.clone(),
        profile_sha256: file_identity(&no_symlink_parents(root, &c.profile)?)?.0,
        native_artifacts: native_artifacts(root, &cfg, source, &selection)?,
        selection,
        prepared: None,
    };
    let profile = crate::json(
        &fs::read(no_symlink_parents(root, &req.profile)?).map_err(|e| e.to_string())?,
    )?;
    if matches!(
        profile["schema"].as_str(),
        Some("chrono-ci-check/v1" | "chrono-ci-check/v2" | "chrono-ci-check/v3")
    ) && (profile["policy"]["registration_config"] != req.host_config
        || profile["policy"]["facts_config"] != req.host_config)
    {
        return Err(
            "short scoped profile requires matching registration_config and bound facts_config"
                .into(),
        );
    }
    if req.selection != Selection::All && profile["schema"] != units::PROFILE {
        return Err("full independent scopes are not supported by short check".into());
    }
    req.validate()?;
    let action = if source == "local" {
        &c.inputs.local
    } else {
        c.inputs.ci.as_ref().ok_or("CI input binding missing")?
    };
    let (mut p, first) = invoke(root, &cfg, action, &env, &req)?;
    let mut receipts = vec![first];
    if source == "local" && matches!(req.selection, Selection::Collect) {
        req.prepared = Some(p);
        let (next, receipt) = invoke(
            root,
            &cfg,
            c.inputs
                .ci
                .as_ref()
                .ok_or("collection requires the registered CI producer binding")?,
            &env,
            &req,
        )?;
        p = next;
        receipts.push(receipt);
    }
    req.validate()?;
    let binding = json!({"schema":"chrono-prepared-check/v1","request":req,"result":p,"selection":sel.as_ref().map(|s|&s.observation),"environment":{"representation":"sha256","source":source,"inherited":inherited,"effective":environment_hashes(&env),"effective_digest":wire::digest(&env)?},"receipts":receipts});
    Ok((req, p, binding))
}
