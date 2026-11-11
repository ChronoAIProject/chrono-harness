//! Cache planning owns its original probe attempts; retention owns custody and disposal.
use chrono_harness::{artifact_disposal::identity, decode, no_symlink_parents, sha256, wire};
use chrono_harness::retained_artifacts::retention::{Inventory, Publication, unique_path};
use serde_json::{Value, json};
use std::{cell::Cell, fs, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};

pub(crate) const PRODUCER: &str = "cache-probe-attempt";
const STORE: &str = ".chrono-harness/state/cache-probes";
const POLICY: &[u8] = include_bytes!("../../../assets/cache-retention.json");

fn inventory(root: &Path) -> Result<Inventory, String> {
    let state = Inventory::with_standalone_policy(root, POLICY)?;
    if state.terminal_limits(PRODUCER).is_none() {
        return Err("E_CACHE_ATTEMPT: adopted policy requires cache-probe-attempt terminal_limits".into());
    }
    Ok(state)
}
fn timestamp() -> Result<u64, String> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_secs())
}

pub(crate) struct Attempt {
    pub directory: PathBuf,
    pub relative: String,
    pub publication: Publication,
    pub unknown: Cell<bool>,
}
impl Attempt {
    pub fn record(&self, path: &Path, value: &Value) -> Result<(), String> {
        let raw = serde_json::to_vec(value).map_err(|e| e.to_string())?;
        let temporary = path.with_extension("json.writing");
        self.publication.write_file(&temporary, &raw)?;
        fs::File::open(&temporary).and_then(|f| f.sync_all()).map_err(|e| e.to_string())?;
        fs::hard_link(&temporary, path).map_err(|e| e.to_string())?;
        fs::remove_file(&temporary).map_err(|e| e.to_string())?;
        fs::File::open(path.parent().ok_or("E_CACHE_ATTEMPT: record parent")?)
            .and_then(|f| f.sync_all()).map_err(|e| e.to_string())
    }
}

/// An explicit plan-output names the exact attempt. Calls without an output
/// allocate independent attempts, including callers of the public Rust API.
pub(crate) fn prepare(root: &Path, c: &super::Config, consumer: &str, output: Option<&str>) -> Result<Value, String> {
    super::validate(c)?;
    super::name(consumer)?;
    let has_probe = c.caches.values().filter(|cache| cache.consumers.iter().any(|v| v == consumer))
        .flat_map(|cache| cache.compatibility_inputs.iter().chain(&cache.source_inputs))
        .any(|id| matches!(c.inputs[id], super::Input::Command { .. }));
    if !has_probe {
        return super::prepare_inputs(root, c, consumer, None, None);
    }
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let store = no_symlink_parents(&root, STORE)?;
    fs::create_dir_all(&store).map_err(|e| e.to_string())?;
    let directory = match output {
        Some(output) => store.join(format!("plan-{}", sha256(output.as_bytes()))),
        None => unique_path(&store, "attempt-", "")?,
    };
    let relative = directory.strip_prefix(&root).map_err(|e| e.to_string())?.to_str().ok_or("E_CACHE_ATTEMPT: UTF-8 path")?.to_owned();
    let role = format!("cache-plan:{}", sha256(relative.as_bytes()));
    let request = json!({"host":root,"host_identity":identity(&root)?,"attempt":relative,
        "consumer":consumer,"registration":c,"plan_output":output});
    let state = inventory(&root)?;
    if let Some((_, original, phase, retained)) = state.rooted_original(PRODUCER, &role)? {
        if original != relative || retained["request_sha256"] != wire::digest(&request)? {
            return Err(format!("E_CACHE_ATTEMPT: original request differs; preserve {relative}"));
        }
        let path = directory.join("result.json");
        let raw = fs::read(&path).map_err(|e| format!("E_CACHE_ATTEMPT: unknown original result ({e}); no redispatch; original {relative}"))?;
        let result: Value = decode(&raw)?;
        if !super::verified_planner::replay_attempt(result["request_sha256"] == wire::digest(&request)?, result["terminal"] == true) {
            return Err(format!("E_CACHE_ATTEMPT: original result unknown or unbound; no redispatch; original {relative}"));
        }
        if phase == "publishing" || phase == "sealing" {
            inventory(&root)?.resume_unfinished_directory(PRODUCER, &relative)?
                .ok_or("E_CACHE_ATTEMPT: unfinished publication unavailable")?
                .complete(json!({"request_sha256":wire::digest(&request)?,"result":"result.json","completion":"confirmed"}))?;
        }
        if !inventory(&root)?.published_tree_member(PRODUCER, &relative, &format!("{relative}/result.json"), &sha256(&raw), raw.len() as u64)? {
            return Err(format!("E_CACHE_ATTEMPT: original publication unverified; preserve {relative}"));
        }
        return replay(&result);
    }
    // Retired/unrooted addresses cannot be used to repeat the same attempt.
    if state.enrolled(&relative)? || directory.exists() {
        return Err(format!("E_CACHE_ATTEMPT: original unavailable or released; no redispatch; original {relative}"));
    }
    let intent = json!({"schema":"chrono-cache-attempt/v1","request":request,
        "request_sha256":wire::digest(&request)?,"updated":timestamp()?,
        "task_identity":null,"result":null,"completion":"unknown",
        "wait":"original process engine joins its owned work; missing result cannot be reacquired by redispatch",
        "consumer_role":role});
    let publication = state.plan_rooted(PRODUCER, &relative, json!({"request_sha256":wire::digest(&request)?,"intent":"intent.json","completion":"not-established"}), &role)?;
    publication.materialize()?;
    let attempt = Attempt {directory, relative:relative.clone(), publication, unknown:Cell::new(false)};
    attempt.record(&attempt.directory.join("intent.json"), &intent)?;
    let result = super::prepare_inputs(&root, c, consumer, None, Some(&attempt));
    let terminal = !attempt.unknown.get();
    let receipt = json!({"schema":"chrono-cache-attempt-result/v1","request_sha256":wire::digest(&request)?,
        "updated":timestamp()?,"terminal":terminal,"plan":result.as_ref().ok(),"error":result.as_ref().err()});
    attempt.record(&attempt.directory.join("result.json"), &receipt).map_err(|publication| match &result {
        Ok(_) => format!("E_CACHE_ATTEMPT: result publication: {publication}; original {relative}"),
        Err(error) => format!("{error}; result publication: {publication}; attempt {relative}"),
    })?;
    // Unknown work retains its original unfinished capability and recovery root.
    if terminal {
        attempt.publication.complete(json!({"request_sha256":wire::digest(&request)?,"result":"result.json","completion":"confirmed"}))?;
        // A failed plan has no downstream restore/seal caller. Its original
        // diagnostic result stays under the adopted finite terminal policy.
        if result.is_err() {
            inventory(&root)?.reference_paths(&role, &[])?;
        }
    }
    result
}
fn replay(result: &Value) -> Result<Value, String> {
    match (result.get("plan").filter(|p| !p.is_null()), result["error"].as_str()) {
        (Some(plan), None) => Ok(plan.clone()),
        (None, Some(error)) => Err(error.to_owned()),
        _ => Err("E_CACHE_ATTEMPT: invalid original result; no redispatch".into()),
    }
}

/// Derive custody only from current command originals and verify the unchanged
/// original plan. Recovery and seal never execute the command again.
pub(crate) fn validate(root: &Path, plan: &Value, release: bool) -> Result<(), String> {
    let mut attempts = std::collections::BTreeSet::new();
    for input in plan["inputs"].as_object().ok_or("E_CACHE_ATTEMPT: inputs")?.values() {
        if input["declaration"]["kind"] != "command" { continue; }
        let original = input["original"].as_str().ok_or("E_CACHE_ATTEMPT: missing process original")?;
        let path = Path::new(original);
        let directory = path.parent().and_then(Path::parent).ok_or("E_CACHE_ATTEMPT: original address")?;
        let relative = directory.to_str().ok_or("E_CACHE_ATTEMPT: original UTF-8")?;
        if !relative.starts_with(&format!("{STORE}/")) { return Err("E_CACHE_ATTEMPT: original outside attempt".into()); }
        attempts.insert(relative.to_owned());
    }
    for relative in attempts {
        let state = inventory(root)?;
        let result_path = format!("{relative}/result.json");
        let raw = fs::read(no_symlink_parents(root, &result_path)?).map_err(|e| e.to_string())?;
        let result: Value = decode(&raw)?;
        if result["plan"] != *plan || result["terminal"] != true || !state.published_tree_member(PRODUCER, &relative, &result_path, &sha256(&raw), raw.len() as u64)? {
            return Err("E_CACHE_ATTEMPT: original plan publication differs or is unfinished".into());
        }
        if release { state.reference_paths(&format!("cache-plan:{}", sha256(relative.as_bytes())), &[])?; }
    }
    Ok(())
}
