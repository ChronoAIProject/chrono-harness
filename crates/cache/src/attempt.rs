//! Cache planning owns its original probe attempts; retention owns custody and disposal.
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
use super::identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{verified_identifiers as identifiers, verus};
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
/// Request digests and original addresses use complete text, including absence.
pub fn same_attempt(expected: &str, original: Option<&str>) -> (bound: bool)
    ensures bound == match original { Some(value) => expected@ == value@, None => false },
{
    match original { Some(value) => identifiers::equal_text(expected, value), None => false }
}

pub fn accept_original(bound: bool, terminal: bool, sealed: bool) -> (accepted: bool)
    ensures accepted == (bound && terminal && sealed),
{ bound && terminal && sealed }

pub fn release_custody(bound: bool, terminal: bool, sealed: bool, consumer_finished: bool) -> (release: bool)
    ensures release == (bound && terminal && sealed && consumer_finished),
{ accept_original(bound, terminal, sealed) && consumer_finished }

/// A result can be replayed only for the exact request, after confirmed work.
pub fn replay_attempt(bound: bool, terminal: bool) -> (allowed: bool)
    ensures allowed == (bound && terminal),
{ bound && terminal }

/// A process receipt alone does not establish joined, matched terminal work.
/// Engine errors do not erase an actually joined tree or establish a missing join.
pub fn confirmed_probe(launch_matches: bool, exit_code: i32, exit_signal: Option<i32>, owned_work_joined: bool) -> (terminal: bool)
    ensures terminal == (launch_matches && (exit_code >= 0 || exit_signal.is_some()) && owned_work_joined),
{ launch_matches && (exit_code >= 0 || exit_signal.is_some()) && owned_work_joined }
}

#[cfg(not(verus_keep_ghost))]
pub(crate) use runtime::{Attempt, prepare, publish_plan, release_plan, validate};

// JSON, filesystem and retention capabilities are the explicit IO boundary.
// The strict root compiles and proves the same judgments used below; it does
// not claim a proof of the transaction's IO or process engine.
#[cfg(not(verus_keep_ghost))]
mod runtime {
    use super as decision;
    use chrono_harness::retained_artifacts::retention::{Inventory, Publication, unique_path};
    use chrono_harness::{artifact_disposal::identity, decode, no_symlink_parents, sha256, wire};
    use serde_json::{Value, json};
    use std::{
        cell::Cell,
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    const PRODUCER: &str = "cache-probe-attempt";
    const STORE: &str = ".chrono-harness/state/cache-probes";

    fn inventory(root: &Path) -> Result<Inventory, String> {
        let state = Inventory::adopted(root)?
            .ok_or("E_CACHE_ATTEMPT: host must adopt retention/v2 for cache probe originals")?;
        if state.terminal_limits(PRODUCER).is_none()
            || state.terminal_limits("cache-plan").is_none()
        {
            return Err("E_CACHE_ATTEMPT: adopted policy requires cache-probe-attempt and cache-plan terminal_limits".into());
        }
        Ok(state)
    }
    fn timestamp() -> Result<u64, String> {
        Ok(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs())
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
            // write_file synchronizes these bytes before returning. The link
            // below still needs the directory sync, without a second file sync.
            self.publication.write_file(&temporary, &raw)?;
            fs::hard_link(&temporary, path).map_err(|e| e.to_string())?;
            fs::remove_file(&temporary).map_err(|e| e.to_string())?;
            fs::File::open(path.parent().ok_or("E_CACHE_ATTEMPT: record parent")?)
                .and_then(|f| f.sync_all())
                .map_err(|e| e.to_string())
        }
    }

    /// An explicit plan-output names the exact attempt. Calls without an output
    /// allocate independent attempts, including callers of the public Rust API.
    pub(crate) fn prepare(
        root: &Path,
        c: &crate::Config,
        consumer: &str,
        output: Option<&str>,
    ) -> Result<Value, String> {
        crate::validate(c)?;
        crate::name(consumer)?;
        let has_probe = c
            .caches
            .values()
            .filter(|cache| cache.consumers.iter().any(|v| v == consumer))
            .flat_map(|cache| {
                cache
                    .compatibility_inputs
                    .iter()
                    .chain(&cache.source_inputs)
            })
            .any(|id| matches!(c.inputs[id], crate::Input::Command { .. }));
        if !has_probe {
            return crate::prepare_inputs(root, c, consumer, None, None);
        }
        let began = std::time::Instant::now();
        let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
        let store = no_symlink_parents(&root, STORE)?;
        fs::create_dir_all(&store).map_err(|e| e.to_string())?;
        let directory = match output {
            Some(output) => store.join(format!("plan-{}", sha256(output.as_bytes()))),
            None => unique_path(&store, "attempt-", "")?,
        };
        let relative = directory
            .strip_prefix(&root)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("E_CACHE_ATTEMPT: UTF-8 path")?
            .to_owned();
        let span = tracing::debug_span!("cache_attempt", attempt = relative);
        let _entered = span.enter();
        let role = format!("cache-plan:{}", sha256(relative.as_bytes()));
        let request = json!({"host":root,"host_identity":identity(&root)?,"attempt":relative,
        "consumer":consumer,"registration":c,"plan_output":output});
        let state = inventory(&root)?;
        let rooted = state.rooted_original(PRODUCER, &role)?;
        let existing = match rooted {
            Some(record) => Some(record),
            None if state.enrolled(&relative)? => {
                let status = state.producer_status(PRODUCER, Some(&relative))?;
                status["records"]
                    .as_array()
                    .and_then(|rows| rows.first())
                    .map(|record| {
                        (
                            record["id"].as_str().unwrap_or_default().to_owned(),
                            relative.clone(),
                            record["phase"].as_str().unwrap_or_default().to_owned(),
                            record["outcome"].clone(),
                        )
                    })
            }
            None => None,
        };
        if let Some((id, original, mut phase, retained)) = existing {
            let mut declared = &retained;
            for _ in 0..8 {
                if declared.get("request_sha256").is_some() {
                    break;
                }
                declared = declared.get("original").unwrap_or(declared);
            }
            if !decision::same_attempt(&relative, Some(&original))
                || !decision::same_attempt(
                    &wire::digest(&request)?,
                    declared["request_sha256"].as_str(),
                )
            {
                return Err(format!(
                    "E_CACHE_ATTEMPT: original request differs; preserve {relative}"
                ));
            }
            let path = directory.join("result.json");
            let raw = fs::read(&path).map_err(|e| {
            format!(
                "E_CACHE_ATTEMPT: unknown original result ({e}); no redispatch; original {relative}"
            )
        })?;
            let result: Value = decode(&raw)?;
            if !decision::replay_attempt(
                decision::same_attempt(&wire::digest(&request)?, result["request_sha256"].as_str()),
                result["terminal"] == true,
            ) {
                return Err(format!(
                    "E_CACHE_ATTEMPT: original result unknown or unbound; no redispatch; original {relative}"
                ));
            }
            if phase == "sealing" {
                state.maintain_originals(std::slice::from_ref(&id))?;
                let status = state.producer_status(PRODUCER, Some(&relative))?;
                phase = status["records"][0]["phase"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned();
                if phase == "sealing" {
                    return Err(format!(
                        "E_CACHE_ATTEMPT: original sealing unfinished; no redispatch; original {relative}"
                    ));
                }
            }
            if phase == "publishing" {
                inventory(&root)?.resume_unfinished_directory(PRODUCER, &relative)?
                .ok_or("E_CACHE_ATTEMPT: unfinished publication unavailable")?
                .complete_sealed(json!({"request_sha256":wire::digest(&request)?,"result":"result.json","completion":"confirmed"}))
                .map_err(|error| publication_error(&relative, &error, result["error"].as_str()))?;
            }
            if !inventory(&root)?.published_tree_member(
                PRODUCER,
                &relative,
                &format!("{relative}/result.json"),
                &sha256(&raw),
                raw.len() as u64,
            )? {
                return Err(format!(
                    "E_CACHE_ATTEMPT: original publication unverified; preserve {relative}"
                ));
            }
            let _use = state.consume(&id)?;
            // A terminal failed attempt has no downstream plan consumer. If
            // the first execution was interrupted after sealing its result but
            // before releasing the plan root, replay resumes that same
            // producer-owned release obligation after the exact result is
            // verified and consumed.
            if result["error"].is_string() {
                inventory(&root)?.reference_paths(&role, &[])?;
            }
            return replay(&result);
        }
        // Retired/unrooted addresses cannot be used to repeat the same attempt.
        if state.enrolled(&relative)? || directory.exists() {
            return Err(format!(
                "E_CACHE_ATTEMPT: original unavailable or released; no redispatch; original {relative}"
            ));
        }
        tracing::debug!(
            event_id = "cache.attempt.admitted",
            elapsed_seconds = began.elapsed().as_secs_f64(),
            "new probe attempt admission returned"
        );
        let intent = json!({"schema":"chrono-cache-attempt/v1","request":request,
        "request_sha256":wire::digest(&request)?,"updated":timestamp()?,
        "task_identity":null,"result":null,"completion":"unknown",
        "wait":"original process engine joins its owned work; missing result cannot be reacquired by redispatch",
        "consumer_role":role});
        let publication = state.plan_rooted(PRODUCER, &relative, json!({"request_sha256":wire::digest(&request)?,"intent":"intent.json","completion":"not-established"}), &role)?;
        publication.materialize()?;
        tracing::debug!(
            event_id = "cache.attempt.materialized",
            elapsed_seconds = began.elapsed().as_secs_f64(),
            "new probe attempt materialization returned"
        );
        let attempt = Attempt {
            directory,
            relative: relative.clone(),
            publication,
            unknown: Cell::new(false),
        };
        attempt.record(&attempt.directory.join("intent.json"), &intent)?;
        let result = crate::prepare_inputs(&root, c, consumer, None, Some(&attempt));
        tracing::debug!(
            event_id = "cache.attempt.inputs_returned",
            elapsed_seconds = began.elapsed().as_secs_f64(),
            succeeded = result.is_ok(),
            "cache input preparation returned"
        );
        let terminal = !attempt.unknown.get();
        let receipt = json!({"schema":"chrono-cache-attempt-result/v1","request_sha256":wire::digest(&request)?,
        "updated":timestamp()?,"terminal":terminal,"plan":result.as_ref().ok(),"error":result.as_ref().err()});
        attempt
            .record(&attempt.directory.join("result.json"), &receipt)
            .map_err(|publication| match &result {
                Ok(_) => {
                    format!(
                        "E_CACHE_ATTEMPT: result publication: {publication}; original {relative}"
                    )
                }
                Err(error) => {
                    format!("{error}; result publication: {publication}; attempt {relative}")
                }
            })?;
        tracing::debug!(
            event_id = "cache.attempt.result_recorded",
            elapsed_seconds = began.elapsed().as_secs_f64(),
            terminal,
            "original attempt result was recorded"
        );
        // Unknown work retains its original unfinished capability and recovery root.
        if terminal {
            // The immediate plan consumer requires a sealed tree, not just a
            // completed accounting round. Keep the same capability across rounds.
            attempt.publication.complete_sealed(json!({"request_sha256":wire::digest(&request)?,"result":"result.json","completion":"confirmed"}))
                .map_err(|error| publication_error(&relative, &error, result.as_ref().err().map(String::as_str)))?;
            tracing::debug!(
                event_id = "cache.attempt.sealed",
                elapsed_seconds = began.elapsed().as_secs_f64(),
                "terminal attempt sealing returned"
            );
            // A failed plan has no downstream restore/seal caller. Its original
            // diagnostic result stays under the adopted finite terminal policy.
            if result.is_err() {
                inventory(&root)?.reference_paths(&role, &[])?;
            }
        }
        result
    }
    fn publication_error(relative: &str, publication: &str, original: Option<&str>) -> String {
        let publication = format!(
            "E_CACHE_ATTEMPT: terminal publication: {publication}; original {relative}/result.json"
        );
        match original {
            Some(original) => format!("{original}; {publication}"),
            None => publication,
        }
    }
    fn replay(result: &Value) -> Result<Value, String> {
        match (
            result.get("plan").filter(|p| !p.is_null()),
            result["error"].as_str(),
        ) {
            (Some(plan), None) => Ok(plan.clone()),
            (None, Some(error)) => Err(error.to_owned()),
            _ => Err("E_CACHE_ATTEMPT: invalid original result; no redispatch".into()),
        }
    }

    /// Derive custody only from current command originals and verify the unchanged
    /// original plan. Recovery and seal never execute the command again.
    pub(crate) fn validate(root: &Path, plan: &Value, release: bool) -> Result<(), String> {
        let mut attempts = std::collections::BTreeSet::new();
        for input in plan["inputs"]
            .as_object()
            .ok_or("E_CACHE_ATTEMPT: inputs")?
            .values()
        {
            if input["declaration"]["kind"] != "command" {
                continue;
            }
            let original = input["original"]["path"]
                .as_str()
                .ok_or("E_CACHE_ATTEMPT: missing process original")?;
            let path = Path::new(original);
            let directory = path
                .parent()
                .and_then(Path::parent)
                .ok_or("E_CACHE_ATTEMPT: original address")?;
            let relative = directory
                .to_str()
                .ok_or("E_CACHE_ATTEMPT: original UTF-8")?;
            if !relative.starts_with(&format!("{STORE}/")) {
                return Err("E_CACHE_ATTEMPT: original outside attempt".into());
            }
            let state = inventory(root)?;
            let status = state.producer_status(PRODUCER, Some(relative))?;
            let id = status["records"][0]["id"]
                .as_str()
                .ok_or("E_CACHE_ATTEMPT: original identity missing")?;
            let _use = state.consume(id)?;
            let raw = fs::read(no_symlink_parents(root, original)?).map_err(|e| e.to_string())?;
            if !decision::same_attempt(&sha256(&raw), input["original"]["sha256"].as_str())
                || !decision::same_attempt(id, input["original"]["attempt_id"].as_str())
            {
                return Err("E_CACHE_ATTEMPT: original digest or attempt identity differs".into());
            }
            if !state.published_tree_member(
                PRODUCER,
                relative,
                original,
                &sha256(&raw),
                raw.len() as u64,
            )? {
                return Err("E_CACHE_ATTEMPT: process original is not sealed and bound".into());
            }
            let process: chrono_harness::ProcessResult = decode(&raw)?;
            chrono_harness::validate_launch_evidence(
                &root.join(path.with_file_name("launch.json")),
                &process,
            )?;
            attempts.insert(relative.to_owned());
        }
        for relative in attempts {
            let state = inventory(root)?;
            let result_path = format!("{relative}/result.json");
            let raw =
                fs::read(no_symlink_parents(root, &result_path)?).map_err(|e| e.to_string())?;
            let result: Value = decode(&raw)?;
            let bound = result["plan"] == *plan;
            let terminal = result["terminal"] == true;
            let sealed = state.published_tree_member(
                PRODUCER,
                &relative,
                &result_path,
                &sha256(&raw),
                raw.len() as u64,
            )?;
            if !decision::accept_original(bound, terminal, sealed) {
                return Err(
                    "E_CACHE_ATTEMPT: original plan publication differs or is unfinished".into(),
                );
            }
            let status = state.producer_status(PRODUCER, Some(&relative))?;
            let id = status["records"][0]["id"]
                .as_str()
                .ok_or("E_CACHE_ATTEMPT: original owner identity missing")?;
            let _use = state.consume(id)?;
            if decision::release_custody(bound, terminal, sealed, release) {
                state
                    .reference_paths(&format!("cache-plan:{}", sha256(relative.as_bytes())), &[])?;
            }
        }
        Ok(())
    }

    /// Publish the existing external plan slot through the same retention owner.
    /// Its pending handoff root protects the attempt dependency until seal returns.
    pub(crate) fn publish_plan(root: &Path, path: &str, plan: &Value) -> Result<bool, String> {
        let originals: Vec<String> = plan["inputs"]
            .as_object()
            .ok_or("E_CACHE_ATTEMPT: inputs")?
            .values()
            .filter(|input| input["declaration"]["kind"] == "command")
            .map(|input| {
                input["original"]["path"]
                    .as_str()
                    .ok_or("E_CACHE_ATTEMPT: original missing")
                    .map(str::to_owned)
            })
            .collect::<Result<_, _>>()?;
        if originals.is_empty() {
            return Ok(false);
        }
        let began = std::time::Instant::now();
        validate(root, plan, false)?;
        tracing::debug!(
            event_id = "cache.plan.originals_validated",
            elapsed_seconds = began.elapsed().as_secs_f64(),
            "plan original qualification returned"
        );
        let raw = serde_json::to_vec(plan).map_err(|e| e.to_string())?;
        if !chrono_harness::retained_artifacts::retention::publish_final_original(
            root,
            "cache-plan",
            path,
            &raw,
            json!({"kind":"cache-plan","completion":"confirmed","plan_sha256":sha256(&raw)}),
            true,
        )? {
            return Err("E_CACHE_ATTEMPT: cache-plan scope must be adopted".into());
        }
        tracing::debug!(
            event_id = "cache.plan.original_published",
            elapsed_seconds = began.elapsed().as_secs_f64(),
            "plan original publication returned"
        );
        let state = inventory(root)?;
        if !state.published_original(path, &sha256(&raw), raw.len() as u64)? {
            return Err("E_CACHE_ATTEMPT: plan publication unfinished; original attempt retained; no redispatch".into());
        }
        state.dependencies_paths(path, &originals)?;
        tracing::debug!(
            event_id = "cache.plan.dependencies_bound",
            elapsed_seconds = began.elapsed().as_secs_f64(),
            "published plan dependencies were bound"
        );
        Ok(true)
    }

    pub(crate) fn release_plan(root: &Path, path: &str, raw: &[u8]) -> Result<(), String> {
        let plan: Value = decode(raw)?;
        if !plan["inputs"]
            .as_object()
            .ok_or("E_CACHE_ATTEMPT: inputs")?
            .values()
            .any(|input| input["declaration"]["kind"] == "command")
        {
            return Ok(());
        }
        let state = inventory(root)?;
        if !state.published_original(path, &sha256(raw), raw.len() as u64)? {
            return Err("E_CACHE_ATTEMPT: consumer plan publication not bound".into());
        }
        let status = state.producer_status("cache-plan", Some(path))?;
        let id = status["records"][0]["id"]
            .as_str()
            .ok_or("E_CACHE_ATTEMPT: plan owner missing")?;
        let _use = state.consume(id)?;
        state.reference(&format!("consumer:publication-recheck:{id}"), &[])
    }
}
