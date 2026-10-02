//! Full-format orchestration: fixed facts, declared DAG, external processes and aggregation.
use crate::{
    facts, json, no_symlink_parents, sha256,
    wire::{self, Binding, Context, Endpoint, Executable, Registries, Request, Response, Status},
};
use serde::Deserialize;
use serde_json::{Value, json as value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

pub fn schedule(bindings: &[Binding]) -> Result<Vec<Binding>, String> {
    schedule_mode(bindings, "every-delta", "evaluate")
}
pub(crate) fn schedule_mode(
    bindings: &[Binding],
    selector: &str,
    mode: &str,
) -> Result<Vec<Binding>, String> {
    let mut pending = BTreeMap::new();
    for b in bindings {
        if b.id.is_empty() || pending.insert(b.id.clone(), b.clone()).is_some() {
            return Err("duplicate/empty judge ID".into());
        }
        if b.selector != selector || b.modes != [mode] {
            return Err("unsupported judge transport selection/mode".into());
        }
        if b.after.iter().collect::<BTreeSet<_>>().len() != b.after.len() {
            return Err("duplicate predecessor".into());
        }
    }
    if pending.is_empty() {
        return Err("no configured judges".into());
    }
    for b in bindings {
        for id in &b.after {
            if !pending.contains_key(id) {
                return Err(format!("missing predecessor {id}"));
            }
        }
    }
    let mut ordered = vec![];
    let mut done = BTreeSet::new();
    while !pending.is_empty() {
        let id = pending
            .iter()
            .find(|(_, b)| b.after.iter().all(|id| done.contains(id)))
            .map(|(id, _)| id.clone())
            .ok_or("judge dependency cycle")?;
        ordered.push(pending.remove(&id).ok_or("missing scheduled judge")?);
        done.insert(id);
    }
    Ok(ordered)
}
/// The single original stdin construction rule. History never contains requests.
pub fn predecessor(record: &Value) -> Value {
    let mut value = record.clone();
    if let Some(object) = value.as_object_mut() {
        object.remove("request");
        object.remove("executable_index");
    }
    value
}
pub fn judge_request(
    template: &Request,
    binding: &Binding,
    history: &[Value],
) -> Result<Request, String> {
    let observations = history
        .iter()
        .map(|record| predecessor_observation(record, template.scope.is_some()))
        .collect::<Result<Vec<_>, String>>()?;
    judge_request_from_observations(template, binding, &observations)
}
fn predecessor_observation(record: &Value, scoped: bool) -> Result<Value, String> {
    let mut record = predecessor(record);
    // Preserve the existing lossless scoped stdin encoding. Prepare it once
    // per original process in an invocation, then reuse it for later judges.
    if scoped {
        compact_record(&mut record)?;
    }
    Ok(record)
}
fn judge_request_from_observations(
    template: &Request,
    binding: &Binding,
    observations: &[Value],
) -> Result<Request, String> {
    let mut req = request_from_observations(template, binding, observations)?;
    req.seal()?;
    Ok(req)
}
fn request_from_observations(
    template: &Request,
    binding: &Binding,
    observations: &[Value],
) -> Result<Request, String> {
    let mut req = template.clone();
    req.judge_id = binding.id.clone();
    req.prior_results = binding
        .after
        .iter()
        .map(|id| {
            let row = observations
                .iter()
                .find(|row| row["id"] == *id)
                .ok_or_else(|| format!("missing predecessor {id}"))?;
            let response = Response::deserialize(&row["response"]).map_err(|e| e.to_string())?;
            if response.status >= Status::Fail {
                return Err(format!("failed predecessor {id}"));
            }
            Ok(response)
        })
        .collect::<Result<_, String>>()?;
    req.observations["judges"] = Value::Array(observations.to_vec());
    let impacts: Vec<_> = req
        .prior_results
        .iter()
        .filter_map(|r| r.outputs.get("impact"))
        .collect();
    if impacts.windows(2).any(|p| p[0] != p[1]) {
        return Err("conflicting predecessor impact outputs".into());
    }
    req.impact = impacts.first().map(|v| (*v).clone()).unwrap_or(Value::Null);
    Ok(req)
}
/// Lossless process encoding for scoped reports and predecessor observations.
/// Unscoped stdin and direct legacy reports retain their original shape and bytes.
pub fn expand_process(value: &Value) -> Result<Value, String> {
    if value.get("encoding").is_none() {
        return Ok(value.clone());
    }
    if value["encoding"] != "chrono-retained-process/v1" {
        return Err("unsupported retained process encoding".into());
    }
    let mut value = value.clone();
    for field in ["stdout", "stderr"] {
        let key = format!("{field}_hex");
        let bytes = artifact_bytes(
            &value!({"bytes": {"hex":value[&key],"sha256":value[format!("{field}_sha256")],"length":value[&key].as_str().ok_or("process encoding")?.len()/2}}),
            "bytes",
        )?;
        value[format!("{field}_bytes")] = value!(&bytes);
        value[field] = value!(String::from_utf8_lossy(&bytes));
        value.as_object_mut().unwrap().remove(&key);
    }
    value.as_object_mut().unwrap().remove("encoding");
    Ok(value)
}
pub fn expand_record(record: &Value) -> Result<Value, String> {
    let mut record = record.clone();
    record["process"] = expand_process(&record["process"])?;
    Ok(record)
}
/// Pure original DAG/process validation, shared by unit admission and collected proof consumption.
pub fn retained_judges(
    template: &Request,
    bindings: &[Binding],
    records: &[Value],
) -> Result<BTreeMap<String, (Request, Binding, Response, crate::ProcessResult)>, String> {
    let ordered = schedule(bindings)?;
    if records.len() != ordered.len()
        || records
            .iter()
            .zip(&ordered)
            .any(|(row, b)| row["id"] != b.id)
    {
        return Err("E_RETAINED_JUDGES: original scheduling/coverage".into());
    }
    let first = judge_request(template, &ordered[0], &[])?;
    if serde_json::to_value(&first).map_err(|e| e.to_string())?
        != serde_json::to_value(template).map_err(|e| e.to_string())?
    {
        return Err("E_RETAINED_JUDGES: template is not original first stdin".into());
    }
    let environment: BTreeMap<String, String> =
        serde_json::from_value(template.observations["environment"]["effective"].clone())
            .map_err(|e| e.to_string())?;
    let mut history = vec![];
    let mut parsed = BTreeMap::new();
    for (compact, binding) in records.iter().zip(ordered) {
        let record = expand_record(compact)?;
        let request = judge_request_from_observations(template, &binding, &history)?;
        let digest = wire::digest(&request)?;
        let response = Response::deserialize(&record["response"]).map_err(|e| e.to_string())?;
        let warnings = response.findings.iter().any(|f| f.level == "warning");
        if response.findings.iter().any(|f| f.level == "error")
            || response.status == Status::Pass && warnings
            || response.status == Status::Warn && !warnings
        {
            return Err("E_RETAINED_JUDGES: response status/findings differ".into());
        }
        let process =
            crate::ProcessResult::deserialize(&record["process"]).map_err(|e| e.to_string())?;
        crate::observation::process_success(&process)?;
        let executable = template.candidate.root.join(&binding.executable);
        let argv: Vec<String> = std::iter::once(
            executable
                .to_str()
                .ok_or("judge executable path")?
                .to_owned(),
        )
        .chain(binding.argv.iter().map(|arg| match arg.as_str() {
            "{base}" => template.base.commit.clone(),
            "{candidate}" => template.candidate.commit.clone(),
            _ => arg.clone(),
        }))
        .collect();
        if record["binding"] != serde_json::to_value(&binding).map_err(|e| e.to_string())?
            || record["state"] != "executed"
            || record.get("transport_failure").is_some()
            || record["request_id"] != request.request_id
            || record["request_digest"] != digest
            || record["exit_code"] != process.exit_code
            || process.stdin_sha256 != digest
            || process.cwd != template.candidate.root
            || process.executable != executable
            || binding.sha256.as_deref() != Some(process.sha256.as_str())
            || process.argv != argv
            || process.environment != environment
            || response.protocol != wire::PROTOCOL
            || response.judge_id != binding.id
            || response.request_id != request.request_id
            || response.status >= Status::Fail
            || response.status.exit_code() != process.exit_code
            || json(&process.stdout_bytes)?
                != serde_json::to_value(&response).map_err(|e| e.to_string())?
            || record
                .get("request")
                .is_some_and(|v| *v != serde_json::to_value(&request).unwrap())
        {
            return Err(format!(
                "E_RETAINED_JUDGES: original request/process/response differs: {}",
                binding.id
            ));
        }
        history.push(predecessor_observation(&record, template.scope.is_some())?);
        parsed.insert(binding.id.clone(), (request, binding, response, process));
    }
    Ok(parsed)
}
fn compact_record(record: &mut Value) -> Result<(), String> {
    if !record["process"].is_object() {
        return Ok(());
    }
    let process = record["process"].as_object_mut().unwrap();
    for field in ["stdout", "stderr"] {
        let bytes: Vec<u8> = serde_json::from_value(
            process
                .remove(&format!("{field}_bytes"))
                .ok_or("process bytes")?,
        )
        .map_err(|e| e.to_string())?;
        process.remove(field);
        process.insert(format!("{field}_hex"), value!(hex_bytes(&bytes)));
    }
    process.insert("encoding".into(), value!("chrono-retained-process/v1"));
    Ok(())
}
fn hex_bytes(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut hex = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        hex.push(DIGITS[(byte >> 4) as usize] as char);
        hex.push(DIGITS[(byte & 15) as usize] as char);
    }
    hex
}
/// Bounded addressed bytes. Original addresses remain intact during transport.
pub fn artifact(bytes: &[u8]) -> Value {
    value!({"sha256":sha256(bytes),"length":bytes.len(),"hex":hex_bytes(bytes)})
}
pub fn artifact_bytes(artifacts: &Value, address: &str) -> Result<Vec<u8>, String> {
    let a = artifacts
        .get(address)
        .ok_or_else(|| format!("missing original artifact {address}"))?;
    let hex = a["hex"].as_str().ok_or("artifact encoding")?;
    if hex.len() > 128 * 1024 * 1024 || hex.len() % 2 != 0 {
        return Err("artifact bound/encoding".into());
    }
    let bytes = hex
        .as_bytes()
        .chunks_exact(2)
        .map(|p| {
            let p = std::str::from_utf8(p).map_err(|e| e.to_string())?;
            u8::from_str_radix(p, 16).map_err(|e| e.to_string())
        })
        .collect::<Result<Vec<_>, String>>()?;
    if a["sha256"] != sha256(&bytes) || a["length"] != bytes.len() {
        return Err("original artifact digest/length".into());
    }
    Ok(bytes)
}
/// The current invocation's existing evidence owner also owns external blobs.
pub fn artifact_directory(req: &Request) -> Result<String, String> {
    let report = req.observations["report_path"]
        .as_str()
        .ok_or("full report path")?;
    let dir = report.rsplit_once('/').ok_or("full report directory")?.0;
    Ok(format!("{dir}/blobs/"))
}
fn blob_original(a: &Value) -> Result<crate::prepared::Original, String> {
    let object = a.as_object().ok_or("artifact encoding")?;
    if object.len() != 3 || object.contains_key("hex") {
        return Err("artifact encoding".into());
    }
    let path = a["blob"].as_str().ok_or("artifact blob path")?;
    crate::units::artifact_path(path)?;
    let digest = a["sha256"]
        .as_str()
        .filter(|s| wire::is_digest(s))
        .ok_or("artifact digest")?;
    a["length"].as_u64().ok_or("artifact length")?;
    if !path.ends_with(&format!("/blob-{digest}.bin")) {
        return Err("original artifact blob address differs".into());
    }
    Ok(crate::prepared::Original {
        path: path.into(),
        sha256: digest.into(),
    })
}
/// Resolve only declared transport edges. A finalized parent report retains
/// each child's external blob by its original address, even after download removal.
/// This creates a reader view; it never rewrites the original unit-report bytes.
pub fn resolve_artifacts(
    artifacts: &Value,
    transport: Option<&crate::prepared::ArtifactTransport>,
    parent: Option<&Value>,
) -> Result<Value, String> {
    let mut resolved = artifacts
        .as_object()
        .ok_or("original artifacts map")?
        .clone();
    for a in resolved.values_mut() {
        if a.get("blob").is_none() {
            continue;
        }
        let original = blob_original(a)?;
        if let Some(parent) = parent {
            let retained = parent
                .get(&original.path)
                .ok_or_else(|| format!("missing original artifact {}", original.path))?;
            if retained["sha256"] != a["sha256"] || retained["length"] != a["length"] {
                return Err("original artifact transport identity differs".into());
            }
            *a = retained.clone();
        } else {
            a["blob"] = value!(crate::prepared::original_path(&original, transport)?);
        }
    }
    Ok(Value::Object(resolved))
}
/// Stream validation for large original inputs; callers that need JSON bytes
/// use artifact_bytes_at after this same digest/length/address validation.
pub fn artifact_identity_at(
    root: &Path,
    artifacts: &Value,
    address: &str,
) -> Result<(String, u64), String> {
    let a = artifacts
        .get(address)
        .ok_or_else(|| format!("missing original artifact {address}"))?;
    if a.get("blob").is_none() {
        let bytes = artifact_bytes(artifacts, address)?;
        return Ok((sha256(&bytes), bytes.len() as u64));
    }
    let original = blob_original(a)?;
    let identity = crate::file_identity(&no_symlink_parents(root, &original.path)?)?;
    if identity.0 != original.sha256 || a["length"] != identity.1 {
        return Err("original artifact digest/length".into());
    }
    Ok(identity)
}
pub fn artifact_bytes_at(root: &Path, artifacts: &Value, address: &str) -> Result<Vec<u8>, String> {
    let a = artifacts
        .get(address)
        .ok_or_else(|| format!("missing original artifact {address}"))?;
    if a.get("blob").is_none() {
        return artifact_bytes(artifacts, address);
    }
    artifact_identity_at(root, artifacts, address)?;
    // This reader materializes bounded JSON/evidence, like the old inline form.
    // Large declared inputs use artifact_identity_at and remain streamed.
    let raw = crate::units::read_bounded(
        root,
        a["blob"].as_str().ok_or("artifact blob path")?,
        64 * 1024 * 1024,
    )?;
    if a["sha256"] != sha256(&raw) || a["length"] != raw.len() {
        return Err("original artifact digest/length".into());
    }
    Ok(raw)
}

fn retain_artifacts(
    req: &Request,
    judges: &[Value],
    failed: bool,
) -> Result<(Value, Value), String> {
    let mut out = serde_json::Map::new();
    let mut unavailable = serde_json::Map::new();
    // Only unit contributions and collection need a portable original closure.
    // Ordinary full checks keep local blob references, validated by registration
    // through streaming identities, without imposing the unit transport bound.
    if req.scope.is_none() {
        return Ok((Value::Object(out), Value::Object(unavailable)));
    }
    let mut retained_bytes = 0u64;
    let bound = req.observations["execution_units"]["collection_limits"]["report_bytes"]
        .as_u64()
        .unwrap_or(64 * 1024 * 1024);
    let mut retain = |address: &str, path: Result<PathBuf, String>| -> Result<(), String> {
        if out.contains_key(address) {
            return Ok(());
        }
        let result = (|| {
            let path = path?;
            let length = fs::metadata(&path).map_err(|e| e.to_string())?.len();
            // Transport choice only: large originals stay exact external bytes.
            // The registered final report bound is still enforced below.
            let a = if length > 1024 * 1024 {
                let (original, length) = crate::prepared::retain_blob(
                    &req.candidate.root,
                    &artifact_directory(req)?,
                    &path,
                )?;
                value!({"blob":original.path,"sha256":original.sha256,"length":length})
            } else {
                artifact(&fs::read(path).map_err(|e| e.to_string())?)
            };
            let total = retained_bytes
                .checked_add(serde_json::to_vec(&a).map_err(|e| e.to_string())?.len() as u64)
                .ok_or("artifact length overflow")?;
            if total > bound {
                return Err("original artifact closure exceeds registered report bound".into());
            }
            out.insert(address.into(), a);
            retained_bytes = total;
            Ok(())
        })();
        match result {
            Err(error) if failed => {
                // Preserve the already-produced judge failure. Missing originals
                // remain unavailable to collection; no evidence is substituted.
                unavailable.insert(address.into(), Value::String(error));
                Ok(())
            }
            result => result,
        }
    };
    retain(
        req.context.path.to_str().ok_or("context path")?,
        Ok(req.context.path.clone()),
    )?;
    let ctx = json(&fs::read(&req.context.path).map_err(|e| e.to_string())?)?;
    if let Some(p) = ctx["retained_inputs"].as_str() {
        retain(p, no_symlink_parents(&req.candidate.root, p))?;
    }
    if req.observations["preparation"].is_object() {
        let binding = &req.observations["preparation"];
        let mut originals: Vec<crate::prepared::Original> =
            serde_json::from_value(binding["result"]["originals"].clone())
                .map_err(|e| e.to_string())?;
        originals.extend(
            serde_json::from_value::<Vec<crate::prepared::Original>>(binding["receipts"].clone())
                .map_err(|e| e.to_string())?,
        );
        for original in originals {
            retain(
                &original.path,
                no_symlink_parents(&req.candidate.root, &original.path),
            )?;
        }
    }
    for name in ["base", "candidate"] {
        for file in req.observations["retained"][name]["files"]
            .as_object()
            .into_iter()
            .flat_map(|m| m.values())
        {
            if let Some(p) = file["blob"].as_str() {
                retain(p, no_symlink_parents(&req.candidate.root, p))?;
            }
        }
    }
    for row in judges {
        if let Some(evidence) = row["response"]["evidence"].as_array() {
            for e in evidence {
                if let Some(p) = e["path"].as_str() {
                    retain(p, no_symlink_parents(&req.candidate.root, p))?;
                }
            }
        }
        if let Some(reports) =
            row["response"]["outputs"]["tests"]["completion"]["reports"].as_array()
        {
            for report in reports {
                let p = report["retained_path"]
                    .as_str()
                    .ok_or("retained unit path")?;
                retain(p, no_symlink_parents(&req.candidate.root, p))?;
                let unit_report = json(
                    &fs::read(no_symlink_parents(&req.candidate.root, p)?)
                        .map_err(|e| e.to_string())?,
                )?;
                let transport: Option<crate::prepared::ArtifactTransport> =
                    serde_json::from_value(report["artifacts"].clone())
                        .map_err(|e| e.to_string())?;
                for a in unit_report["artifacts"]
                    .as_object()
                    .ok_or("unit original artifacts")?
                    .values()
                {
                    if a.get("blob").is_some() {
                        let original = blob_original(a)?;
                        let path = crate::prepared::original_path(&original, transport.as_ref())?;
                        let resolved = value!({"original":{"blob":path,"sha256":original.sha256,"length":a["length"]}});
                        artifact_identity_at(&req.candidate.root, &resolved, "original")?;
                        retain(
                            &original.path,
                            no_symlink_parents(&req.candidate.root, &path),
                        )?;
                    }
                }
            }
        }
    }
    Ok((Value::Object(out), Value::Object(unavailable)))
}
pub fn execute(
    template: &Request,
    bindings: &[Binding],
    env: &BTreeMap<String, String>,
    timeout: u64,
    limit: usize,
) -> Result<(Status, Vec<Value>), String> {
    let ordered = schedule(bindings)?;
    let mut responses: BTreeMap<String, Response> = BTreeMap::new();
    let mut records = vec![];
    let mut observations = vec![];
    let mut status = Status::Pass;
    for b in ordered {
        let blocked: Vec<_> = b
            .after
            .iter()
            .filter(|id| responses.get(*id).is_none_or(|r| r.status >= Status::Fail))
            .cloned()
            .collect();
        if !blocked.is_empty() {
            records.push(value!({"id":b.id,"binding":b,"state":"blocked","blocked_by":blocked,"exit_code":null}));
            status = status.max(Status::Error);
            continue;
        }
        let req = match (|| {
            for record in &records[observations.len()..] {
                observations.push(predecessor_observation(record, template.scope.is_some())?);
            }
            wire::PreparedRequest::new(request_from_observations(template, &b, &observations)?)
        })() {
            Ok(req) => req,
            Err(message) => {
                records.push(value!({"id":b.id,"binding":b,"state":"blocked","exit_code":null,"transport_failure":message}));
                status = Status::Error;
                continue;
            }
        };
        match wire::invoke_prepared(&req, &b, env, timeout, limit) {
            Ok((r, p)) => {
                status = status.max(r.status.clone());
                // The process already digested the canonical request bytes it consumed.
                // Reuse that observation instead of serializing the same request again.
                records.push(value!({"id":b.id,"binding":b,"state":"executed","request_id":req.request().request_id,"request_digest":p.stdin_sha256,"exit_code":p.exit_code,"response":r,"process":p}));
                responses.insert(b.id, r);
            }
            Err(e) => {
                status = Status::Error;
                let exit = e.process.as_ref().map(|p| p.exit_code);
                records.push(value!({"id":b.id,"binding":b,"state":if e.process.is_some(){"executed"}else{"error"},"request_id":req.request().request_id,"transport_failure":e.message,"exit_code":exit,"process":e.process}));
            }
        }
    }
    Ok((status, records))
}
/// Build fixed full inputs without executing judges or business tools.
#[allow(clippy::too_many_arguments)]
pub fn prepare_request(
    root: &Path,
    config_path: &str,
    base: &str,
    candidate: &str,
    context: &Path,
    entry: Value,
    scope: Option<crate::units::Scope>,
    preparation: Option<Value>,
) -> Result<(Request, Vec<Binding>, facts::Reader, Value), String> {
    let profile = crate::load_selected_config(root, config_path)?;
    if let Some(crate::units::Scope::Collect { manifest }) = &scope {
        let block =
            crate::units::full_execution_units(&profile)?.ok_or("full collection units missing")?;
        let units: BTreeMap<String, crate::units::Unit> =
            serde_json::from_value(block["units"].clone()).map_err(|e| e.to_string())?;
        crate::units::validate_collection_paths(
            manifest,
            block["report_path"]
                .as_str()
                .ok_or("collection output missing")?,
            &units
                .values()
                .map(|u| u.report_path.clone())
                .collect::<Vec<_>>(),
        )?;
    }
    let reader = facts::Reader::for_config(root, config_path)?;
    reader.verify_config(root, candidate)?;
    let base_tree = reader.verify_oid(root, base)?;
    let candidate_tree = reader.verify_oid(root, candidate)?;
    let base_values = reader.registry_values(root, base, config_path)?;
    let candidate_values = reader.registry_values(root, candidate, config_path)?;
    let base_identity = facts::registry_identity(&base_values, config_path)?;
    let candidate_identity = facts::registry_identity(&candidate_values, config_path)?;
    let cfg = candidate_values
        .get(&candidate_identity.effective_path)
        .ok_or("missing effective candidate configuration")?;
    crate::units::full_execution_units(cfg)?;
    let workflow_path = cfg["registries"]["workflow"]
        .as_str()
        .ok_or("workflow registry path")?;
    crate::units::report_paths(
        cfg,
        candidate_values[workflow_path]["integration"]["evidence"].as_str(),
    )?;
    let judges_path = cfg["registries"]["judges"]
        .as_str()
        .ok_or("missing judges path")?;
    let bindings: Vec<Binding> =
        serde_json::from_value(candidate_values[judges_path]["judges"].clone())
            .map_err(|e| e.to_string())?;
    let mut env = BTreeMap::new();
    let mut observed = BTreeMap::new();
    let credentials = if cfg["schema_version"] == 4 {
        crate::prepared::credential_environment(cfg)?
    } else {
        BTreeSet::new()
    };
    for k in cfg["environment"]["inherit"]
        .as_array()
        .ok_or("missing environment.inherit")?
    {
        let k = k.as_str().ok_or("environment key must be string")?;
        let v = std::env::var(k).ok();
        observed.insert(
            k.to_string(),
            if credentials.contains(k) {
                None
            } else {
                v.clone()
            },
        );
        if credentials.contains(k) {
            continue;
        }
        if let Some(v) = v {
            env.insert(k.into(), v);
        }
    }
    for (k, v) in cfg["environment"]["values"]
        .as_object()
        .ok_or("missing environment.values")?
    {
        env.insert(
            k.clone(),
            v.as_str().ok_or("environment value must be string")?.into(),
        );
    }
    if cfg["protocol"]["id"] != wire::PROTOCOL || cfg["protocol"]["encoding"] != "UTF-8" {
        return Err("unsupported protocol/encoding".into());
    }
    let _timeout = cfg["protocol"]["timeout_seconds"]
        .as_u64()
        .filter(|n| *n > 0)
        .ok_or("invalid protocol timeout")?;
    let _limit = cfg["protocol"]["stdout_limit_bytes"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= 64 * 1024 * 1024)
        .ok_or("invalid protocol output bound")? as usize;
    let a = reader.tree(root, base)?;
    let b = reader.tree(root, candidate)?;
    let delta = facts::delta(&a, &b);
    let checkout =
        reader.checkout_excluding(root, candidate, &facts::artifact_directories(cfg)?)?;
    let state = no_symlink_parents(root, ".chrono-harness/state")?;
    fs::create_dir_all(&state).map_err(|e| e.to_string())?;
    let snapshot = tempfile::Builder::new()
        .prefix("base-")
        .tempdir_in(&state)
        .map_err(|e| e.to_string())?;
    reader.export(root, base, &a, snapshot.path())?;
    let snapshot = snapshot.keep();
    let context = fs::canonicalize(if context.is_absolute() {
        context.to_path_buf()
    } else {
        root.join(context)
    })
    .map_err(|e| e.to_string())?;
    let context_value = json(&fs::read(&context).map_err(|e| e.to_string())?)?;
    let context_digest = wire::digest(&context_value)?;
    let retained = match context_value.get("retained_inputs").and_then(Value::as_str) {
        Some(path) => json(&fs::read(no_symlink_parents(root, path)?).map_err(|e| e.to_string())?)?,
        None => Value::Null,
    };
    let runner = std::env::current_exe().map_err(|e| e.to_string())?;
    let runner = Executable {
        path: runner.to_str().ok_or("non UTF-8 runner path")?.into(),
        sha256: sha256(&fs::read(&runner).map_err(|e| e.to_string())?),
        version: env!("CARGO_PKG_VERSION").into(),
    };
    let run = format!("{}:{:?}", std::process::id(), std::time::SystemTime::now());
    let report_directory = preparation
        .as_ref()
        .and_then(|b| b["request"]["native_artifacts"]["directory"].as_str())
        .unwrap_or(".chrono-harness/state/");
    let report_path = format!("{report_directory}run-{}.json", wire::digest(&run)?);
    let mut environment = value!({"inherited":observed,"effective":env});
    if cfg["schema_version"] == 4 {
        environment["omitted_credentials"] = value!(credentials);
    }
    let req = Request {
        scope: scope.clone(),
        observations: value!({"git_facts":reader.observation(),"entry":entry,"preparation":preparation,"run":run,"report_path":report_path,"environment":environment,"retained":retained,
            "execution_units":cfg.get("execution_units"),"registry_bindings":{"base":{"entry_path":base_identity.entry_path,"effective_path":base_identity.effective_path,"selection":base_identity.selection},"candidate":{"entry_path":candidate_identity.entry_path,"effective_path":candidate_identity.effective_path,"selection":candidate_identity.selection}}}),
        protocol: wire::PROTOCOL.into(),
        request_id: String::new(),
        judge_id: String::new(),
        mode: "evaluate".into(),
        base: Endpoint {
            commit: base.into(),
            tree: base_tree,
            root: snapshot.clone(),
        },
        candidate: Endpoint {
            commit: candidate.into(),
            tree: candidate_tree.clone(),
            root: root.into(),
        },
        delta: delta.clone(),
        registries: Registries {
            base: snapshot.join(".chrono-harness"),
            candidate: root.join(".chrono-harness"),
            digest: facts::registry_digest(&base_values, &candidate_values)?,
        },
        context: Context {
            path: context,
            sha256: context_digest.clone(),
        },
        impact: Value::Null,
        prior_results: vec![],
        config_path: config_path.into(),
        checkout,
        runner: runner.clone(),
    };
    Ok((req, bindings, reader, cfg.clone()))
}
pub fn check_observed(
    root: &Path,
    config_path: &str,
    base: &str,
    candidate: &str,
    context: &Path,
    entry: Value,
) -> Result<(u8, String), String> {
    check_observed_scoped(root, config_path, base, candidate, context, entry, None)
}
pub fn check_observed_scoped(
    root: &Path,
    config_path: &str,
    base: &str,
    candidate: &str,
    context: &Path,
    entry: Value,
    scope: Option<crate::units::Scope>,
) -> Result<(u8, String), String> {
    check_prepared(
        root,
        config_path,
        base,
        candidate,
        context,
        entry,
        scope,
        None,
    )
}
#[allow(clippy::too_many_arguments)]
pub fn check_prepared(
    root: &Path,
    config_path: &str,
    base: &str,
    candidate: &str,
    context: &Path,
    entry: Value,
    scope: Option<crate::units::Scope>,
    preparation: Option<Value>,
) -> Result<(u8, String), String> {
    let (mut req, bindings, reader, cfg) = prepare_request(
        root,
        config_path,
        base,
        candidate,
        context,
        entry,
        scope.clone(),
        preparation,
    )?;
    let env: BTreeMap<String, String> =
        serde_json::from_value(req.observations["environment"]["effective"].clone())
            .map_err(|e| e.to_string())?;
    let environment = req.observations["environment"].clone();
    let snapshot = req.base.root.clone();
    let delta = req.delta.clone();
    let candidate_tree = req.candidate.tree.clone();
    let context_digest = req.context.sha256.clone();
    let runner = req.runner.clone();
    let context_value = json(&fs::read(&req.context.path).map_err(|e| e.to_string())?)?;
    let report_path = req.observations["report_path"]
        .as_str()
        .ok_or("full report path")?
        .to_owned();
    let report_directory = report_path
        .rsplit_once('/')
        .ok_or("full report directory")?
        .0
        .to_owned()
        + "/";
    let retained_report = crate::prepare_publication(root, &report_path)?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&retained_report)
        .map_err(|e| e.to_string())?;
    let timeout = cfg["protocol"]["timeout_seconds"]
        .as_u64()
        .ok_or("full timeout")?;
    let limit = cfg["protocol"]["stdout_limit_bytes"]
        .as_u64()
        .ok_or("full limit")? as usize;
    if let Some(units) = cfg.get("execution_units").cloned() {
        req.observations
            .as_object_mut()
            .ok_or("full observations object")?
            .insert("execution_units".into(), units);
    }
    if cfg.get("execution_units").is_some() {
        let mut protected = vec![report_path.clone()];
        if let Some(p) = req
            .context
            .path
            .strip_prefix(root)
            .ok()
            .and_then(|p| p.to_str())
        {
            protected.push(p.into());
        }
        if let Some(p) = context_value["retained_inputs"].as_str() {
            protected.push(p.into());
        }
        for name in ["base", "candidate"] {
            for file in req.observations["retained"][name]["files"]
                .as_object()
                .into_iter()
                .flat_map(|m| m.values())
            {
                if let Some(p) = file["blob"].as_str() {
                    protected.push(p.into());
                }
            }
        }
        let mut paths = vec![
            cfg["execution_units"]["report_path"]
                .as_str()
                .ok_or("collection report")?,
        ];
        let unit_paths: Vec<String> = cfg["execution_units"]["units"]
            .as_object()
            .ok_or("units")?
            .values()
            .map(|unit| {
                unit["report_path"]
                    .as_str()
                    .ok_or("unit report")
                    .map(str::to_owned)
            })
            .collect::<Result<_, _>>()?;
        if let Some(crate::units::Scope::Collect { manifest }) = &scope {
            crate::units::validate_collection_paths(manifest, paths[0], &unit_paths)?;
            paths.push(manifest.as_str());
        }
        paths.extend(unit_paths.iter().map(String::as_str));
        if paths
            .iter()
            .any(|p| protected.iter().any(|a| crate::units::overlap(p, a)))
        {
            return Err("execution report overlaps original retained evidence".into());
        }
    }
    let publication_path = match &scope {
        Some(crate::units::Scope::Unit { unit }) => {
            let units = req.observations["execution_units"]["units"]
                .as_object()
                .ok_or("unit selection requires registered execution_units")?;
            units
                .get(unit)
                .and_then(|definition| definition["report_path"].as_str())
                .ok_or("unregistered CI unit")?
        }
        Some(crate::units::Scope::Collect { .. }) => {
            req.observations["execution_units"]["report_path"]
                .as_str()
                .ok_or("execution unit collection report path missing")?
        }
        None => "",
    };
    let publication_path = if publication_path.is_empty() {
        format!("{report_directory}report.json")
    } else {
        publication_path.into()
    };
    crate::prepare_publication(root, &publication_path)?;
    let (status, mut judges) = execute(&req, &bindings, &env, timeout, limit)?;
    let judge_pins: Vec<_> = bindings
        .iter()
        .map(|binding| value!({"id": binding.id, "sha256": binding.sha256}))
        .collect();
    let judge_sha256 = wire::digest(&judge_pins)?;
    let mut unresolved = BTreeMap::<String, String>::new();
    let mut sources = BTreeMap::<String, Vec<String>>::new();
    let mut executables = vec![value!(runner)];
    let mut findings = vec![];
    let mut missing_responses = vec![];
    let mut missing_processes = vec![];
    for (index, judge) in judges.iter_mut().enumerate() {
        if judge["process"].is_object() {
            let executable_index = executables.len();
            executables.push(value!({
                "path":judge["process"]["executable"],
                "sha256":judge["process"]["sha256"],
                "version":null
            }));
            judge["executable_index"] = value!(executable_index);
            for (field, process_field) in [("path", "executable"), ("sha256", "sha256")] {
                sources.insert(
                    format!("/executables/{executable_index}/{field}"),
                    vec![format!("/judges/{index}/process/{process_field}")],
                );
            }
            unresolved.insert(
                format!("/executables/{executable_index}/version"),
                format!("judge version was not observed; configured metadata is /judges/{index}/binding/version"),
            );
        } else {
            missing_processes.push(judge["id"].clone());
        }
        if let Some(items) = judge["response"]["findings"].as_array() {
            for (finding_index, finding) in items.iter().enumerate() {
                sources.insert(
                    format!("/findings/{}", findings.len()),
                    vec![format!("/judges/{index}/response/findings/{finding_index}")],
                );
                findings.push(finding.clone());
            }
        } else {
            missing_responses.push(judge["id"].clone());
        }
    }
    if !missing_processes.is_empty() {
        unresolved.insert("/executables".into(), format!("only observed executables listed; process observations unavailable for {missing_processes:?}; see judge states/transport failures"));
    }
    if !missing_responses.is_empty() {
        unresolved.insert("/findings".into(), format!("only validated responses aggregated; responses unavailable for {missing_responses:?}; see judge states/transport failures"));
    }
    let findings = if missing_responses.len() == judges.len() {
        Value::Null
    } else {
        value!(findings)
    };
    let mut report = value!({"schema_version":1,"scope":"configured-judges","status":status,"base":base,"candidate":candidate,"candidate_tree":candidate_tree,"context_digest":context_digest,"registry_digest":req.registries.digest,"runner":{"path":req.runner.path,"sha256":req.runner.sha256,"version":req.runner.version},"judge_sha256":judge_sha256,"executables":executables,"environment":environment,"parity":{"status":"unestablished","compared_report":null},"delta":delta,"judges":judges,"findings":findings,"base_snapshot":snapshot});
    let first = schedule(&bindings)?.remove(0);
    let original = judge_request(&req, &first, &[])?;
    report["request"] = value!(original);
    report["request_digest"] = wire::digest(&original)?.into();
    let (artifacts, unavailable) = retain_artifacts(&req, &judges, status.exit_code() != 0)?;
    report["artifacts"] = artifacts;
    if !unavailable
        .as_object()
        .ok_or("unavailable artifacts")?
        .is_empty()
    {
        unresolved.insert(
            "/artifacts".into(),
            "original artifact closure incomplete; see artifact_failures".into(),
        );
        report["artifact_failures"] = unavailable;
    }
    report["git_facts"] = reader.observation();
    report["entry"] = req.observations["entry"].clone();
    report["preparation"] = req.observations["preparation"].clone();
    report["report_path"] = value!(report_path);
    if let Some(scope) = &scope {
        let global_selected = judges
            .iter()
            .find_map(|judge| judge["response"]["outputs"]["global_selected"].as_array())
            .or_else(|| {
                judges
                    .iter()
                    .find_map(|judge| judge["response"]["outputs"]["impact"]["tests"].as_array())
            })
            .cloned()
            .unwrap_or_default();
        let own = judges
            .iter()
            .find_map(|judge| {
                judge["response"]["outputs"]["execution_plan"]["selected"].as_object()
            })
            .map(|selected| selected.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        let all_tests: BTreeSet<String> = req.observations["execution_units"]["units"]
            .as_object()
            .into_iter()
            .flat_map(|units| units.values())
            .flat_map(|unit| unit["tests"].as_array().into_iter().flatten())
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect();
        let global_set: BTreeSet<_> = global_selected
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect();
        let units: BTreeMap<String, crate::units::Unit> =
            serde_json::from_value(req.observations["execution_units"]["units"].clone())
                .map_err(|e| format!("invalid execution units: {e}"))?;
        let required_units = crate::units::required(&units, &global_set);
        report["execution_scope"] = value!(scope);
        report["unit_evidence"] = value!({
            "global_selected":global_selected,
            "own":own,
            "assigned_elsewhere":global_set.difference(&own.iter().cloned().collect()).cloned().collect::<Vec<_>>(),
            "required_units":required_units,
            "not_required":all_tests.difference(&global_set).cloned().collect::<Vec<_>>(),
            "acceptance":if matches!(scope, crate::units::Scope::Unit { .. }) {"contribution-only"} else {"global collection"}
        });
    }
    // Only validated named outputs supply these fields. Configuration and absent
    // producers cannot stand in for observations; conflicting results remain visible.
    for field in ["tools", "effective_inputs", "impact", "tests", "costs"] {
        let outputs: Vec<_> = judges
            .iter()
            .enumerate()
            .filter_map(|(index, judge)| {
                judge["response"]["outputs"]
                    .get(field)
                    .map(|output| (index, output))
            })
            .collect();
        sources.insert(
            format!("/{field}"),
            outputs
                .iter()
                .map(|(index, _)| format!("/judges/{index}/response/outputs/{field}"))
                .collect(),
        );
        let reason = if outputs.is_empty() {
            Some(format!(
                "no validated judge response supplied outputs.{field}; configured metadata is not an observed result"
            ))
        } else if outputs.iter().any(|(_, output)| output.is_null()) {
            Some(format!(
                "a producer supplied null for outputs.{field}; see sources"
            ))
        } else if outputs.windows(2).any(|pair| pair[0].1 != pair[1].1) {
            Some(format!(
                "conflicting outputs.{field}; see sources and original judge responses"
            ))
        } else {
            None
        };
        report[field] = if let Some(reason) = reason {
            unresolved.insert(format!("/{field}"), reason);
            Value::Null
        } else {
            outputs[0].1.clone()
        };
    }
    report["unresolved"] = value!(unresolved);
    report["sources"] = value!(sources);
    if scope.is_some() {
        for record in report["judges"].as_array_mut().ok_or("report judges")? {
            compact_record(record)?;
        }
    }
    let text = serde_json::to_string(&report).map_err(|e| e.to_string())? + "\n";
    if scope.is_some()
        && text.len() as u64
            > req.observations["execution_units"]["collection_limits"]["report_bytes"]
                .as_u64()
                .ok_or("full report bound")?
    {
        return Err("full execution evidence exceeds registered report bound".into());
    }
    let path = match &scope {
        Some(crate::units::Scope::Unit { unit }) => {
            let units = req.observations["execution_units"]["units"]
                .as_object()
                .ok_or("unit selection requires registered execution_units")?;
            let definition = units.get(unit).ok_or("unregistered CI unit")?;
            let path = definition["report_path"]
                .as_str()
                .ok_or("unit report path missing")?;
            crate::units::artifact_path(path)?;
            no_symlink_parents(root, path)?
        }
        Some(crate::units::Scope::Collect { .. }) => {
            let path = req.observations["execution_units"]["report_path"]
                .as_str()
                .ok_or("execution unit collection report path missing")?;
            crate::units::artifact_path(path)?;
            no_symlink_parents(root, path)?
        }
        None => no_symlink_parents(root, &format!("{report_directory}report.json"))?,
    };
    fs::write(retained_report, &text).map_err(|e| e.to_string())?;
    fs::write(path, &text).map_err(|e| e.to_string())?;
    Ok((status.exit_code() as u8, text))
}

/// Library callers supply their own observed entry through check_observed for routes validation.
pub fn check(
    root: &Path,
    config_path: &str,
    base: &str,
    candidate: &str,
    context: &Path,
) -> Result<(u8, String), String> {
    check_observed(root, config_path, base, candidate, context, Value::Null)
}
