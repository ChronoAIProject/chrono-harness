//! Discard registered host outputs after unusable native restores, before producers run.
#![allow(unexpected_cfgs)]

#[cfg(verus_keep_ghost)]
use super as planner;
#[cfg(verus_keep_ghost)]
use super::identifiers;
#[cfg(not(verus_keep_ghost))]
use crate::verified_planner as planner;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{verified_identifiers as identifiers, verus};
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

pub enum RecoveryOption {
    HostRoot,
    Config,
    Plan,
    StepsEnvironment,
}

impl RecoveryOption {
    pub open spec fn slot(&self) -> int {
        match self {
            Self::HostRoot => 0,
            Self::Config => 1,
            Self::Plan => 2,
            Self::StepsEnvironment => 3,
        }
    }

    pub open spec fn missing(&self) -> Seq<char> {
        match self {
            Self::HostRoot => "E_CACHE_RECOVERY: missing --host-root"@,
            Self::Config => "E_CACHE_RECOVERY: missing --config"@,
            Self::Plan => "E_CACHE_RECOVERY: missing --plan"@,
            Self::StepsEnvironment => "E_CACHE_RECOVERY: missing --steps-env"@,
        }
    }
}

pub open spec fn recovery_arg_slot(text: Seq<char>) -> int {
    if text == "--host-root"@ { 0 }
    else if text == "--config"@ { 1 }
    else if text == "--plan"@ { 2 }
    else if text == "--steps-env"@ { 3 }
    else { 4 }
}

fn recovery_option(text: &str) -> (out: Option<RecoveryOption>)
    ensures match out {
        Some(key) => key.slot() == recovery_arg_slot(text@),
        None => recovery_arg_slot(text@) == 4,
    },
{
    if identifiers::equal_text(text, "--host-root") { Some(RecoveryOption::HostRoot) }
    else if identifiers::equal_text(text, "--config") { Some(RecoveryOption::Config) }
    else if identifiers::equal_text(text, "--plan") { Some(RecoveryOption::Plan) }
    else if identifiers::equal_text(text, "--steps-env") { Some(RecoveryOption::StepsEnvironment) }
    else { None }
}

pub open spec fn recovery_text(value: Option<&str>) -> Option<Seq<char>> {
    match value { None => None, Some(text) => Some(text@) }
}

pub struct RecoveryOptions<'a> {
    host_root: Option<&'a str>,
    config: Option<&'a str>,
    plan: Option<&'a str>,
    steps_environment: Option<&'a str>,
}

impl<'a> RecoveryOptions<'a> {
    pub closed spec fn value(&self, slot: int) -> Option<Seq<char>> {
        if slot == 0 { recovery_text(self.host_root) }
        else if slot == 1 { recovery_text(self.config) }
        else if slot == 2 { recovery_text(self.plan) }
        else if slot == 3 { recovery_text(self.steps_environment) }
        else { None }
    }

    fn empty() -> (out: Self)
        ensures forall|slot: int| 0 <= slot < 4 ==> out.value(slot).is_none(),
    {
        Self { host_root: None, config: None, plan: None, steps_environment: None }
    }

    pub fn get(&self, key: &RecoveryOption) -> (out: Option<&'a str>)
        ensures recovery_text(out) == self.value(key.slot()),
    {
        match key {
            RecoveryOption::HostRoot => self.host_root,
            RecoveryOption::Config => self.config,
            RecoveryOption::Plan => self.plan,
            RecoveryOption::StepsEnvironment => self.steps_environment,
        }
    }

    /// 真实 CLI 在原有 IO 边界逐项读取必需值。
    pub fn required(&self, key: &RecoveryOption) -> (out: Result<&'a str, &'static str>)
        ensures match out {
            Ok(text) => self.value(key.slot()) == Some(text@),
            Err(error) => self.value(key.slot()).is_none() && error@ == key.missing(),
        },
    {
        match self.get(key) {
            Some(text) => Ok(text),
            None => Err(match key {
                RecoveryOption::HostRoot => "E_CACHE_RECOVERY: missing --host-root",
                RecoveryOption::Config => "E_CACHE_RECOVERY: missing --config",
                RecoveryOption::Plan => "E_CACHE_RECOVERY: missing --plan",
                RecoveryOption::StepsEnvironment => "E_CACHE_RECOVERY: missing --steps-env",
            }),
        }
    }

    fn insert(self, key: &RecoveryOption, text: &'a str)
        -> (out: Result<Self, &'static str>)
        ensures match out {
            Ok(next) => self.value(key.slot()).is_none()
                && forall|slot: int| 0 <= slot < 4 ==> next.value(slot)
                    == if slot == key.slot() { Some(text@) } else { self.value(slot) },
            Err(error) => self.value(key.slot()).is_some()
                && error@ == "E_CACHE_RECOVERY: invalid or duplicate recovery option"@,
        },
    {
        if self.get(key).is_some() {
            return Err("E_CACHE_RECOVERY: invalid or duplicate recovery option");
        }
        let mut next = self;
        match key {
            RecoveryOption::HostRoot => next.host_root = Some(text),
            RecoveryOption::Config => next.config = Some(text),
            RecoveryOption::Plan => next.plan = Some(text),
            RecoveryOption::StepsEnvironment => next.steps_environment = Some(text),
        }
        Ok(next)
    }
}

pub open spec fn recovery_pair_slot(args: Seq<String>, pair: int) -> int {
    recovery_arg_slot(args[2 * pair]@)
}

pub open spec fn recovery_prefix_valid(args: Seq<String>, pairs: int) -> bool {
    0 <= pairs && 2 * pairs <= args.len()
        && (forall|i: int| 0 <= i < pairs ==> recovery_pair_slot(args, i) < 4)
        && (forall|i: int, j: int| 0 <= i < j < pairs
            ==> recovery_pair_slot(args, i) != recovery_pair_slot(args, j))
}

pub open spec fn recovery_options_bound(args: Seq<String>, pairs: int,
    options: &RecoveryOptions) -> bool
{
    forall|slot: int| 0 <= slot < 4 ==> (
        (options.value(slot).is_some() <==> exists|i: int| 0 <= i < pairs
            && recovery_pair_slot(args, i) == slot)
        && (forall|i: int| 0 <= i < pairs && recovery_pair_slot(args, i) == slot
            ==> options.value(slot) == Some(args[2 * i + 1]@))
    )
}

pub open spec fn recovery_args_valid(args: Seq<String>) -> bool {
    args.len() % 2 == 0 && recovery_prefix_valid(args, (args.len() / 2) as int)
}

/// IO 前解析完整键值对，拒绝未知／重复键，原样借用每个值。
pub fn parse_recovery_options<'a>(args: &'a [String])
    -> (out: Result<RecoveryOptions<'a>, &'static str>)
    ensures match out {
        Ok(options) => recovery_args_valid(args@)
            && recovery_options_bound(args@, (args.len() / 2) as int, &options),
        Err(error) => !recovery_args_valid(args@)
            && error@ == "E_CACHE_RECOVERY: invalid or duplicate recovery option"@,
    },
{
    if args.len() % 2 != 0 {
        return Err("E_CACHE_RECOVERY: invalid or duplicate recovery option");
    }
    let pairs = args.len() / 2;
    let mut options = RecoveryOptions::empty();
    let mut i = 0usize;
    while i < pairs
        invariant i <= pairs, pairs == args.len() / 2, 2 * pairs == args.len(),
            recovery_prefix_valid(args@, i as int),
            recovery_options_bound(args@, i as int, &options),
        decreases pairs - i,
    {
        let key = match recovery_option(args[2 * i].as_str()) {
            Some(key) => key,
            None => {
                proof {
                    assert(recovery_pair_slot(args@, i as int) == 4);
                    assert(!recovery_args_valid(args@)) by {
                        if recovery_args_valid(args@) {
                            assert(recovery_prefix_valid(args@, pairs as int));
                            assert(recovery_pair_slot(args@, i as int) < 4);
                        }
                    }
                }
                return Err("E_CACHE_RECOVERY: invalid or duplicate recovery option");
            },
        };
        let ghost before = options;
        let ghost slot = key.slot();
        match options.insert(&key, args[2 * i + 1].as_str()) {
            Err(error) => {
                proof {
                    let prior = choose|j: int| 0 <= j < i
                        && recovery_pair_slot(args@, j) == slot;
                    assert(recovery_pair_slot(args@, prior)
                        == recovery_pair_slot(args@, i as int));
                    assert(!recovery_args_valid(args@));
                }
                return Err(error);
            },
            Ok(next) => {
                proof {
                    assert forall|p: int, q: int| 0 <= p < q < i + 1
                        implies recovery_pair_slot(args@, p)
                            != recovery_pair_slot(args@, q) by {
                        if q == i {
                            if recovery_pair_slot(args@, p) == slot {
                                assert(before.value(slot).is_some());
                            }
                        }
                    }
                    assert forall|s: int| 0 <= s < 4 implies (
                        (next.value(s).is_some() <==> exists|p: int| 0 <= p < i + 1
                            && recovery_pair_slot(args@, p) == s)
                        && (forall|p: int| 0 <= p < i + 1
                            && recovery_pair_slot(args@, p) == s
                            ==> next.value(s) == Some(args[2 * p + 1]@))
                    ) by {
                        if s == slot {
                            assert(exists|p: int| 0 <= p < i + 1
                                && recovery_pair_slot(args@, p) == s) by {
                                assert(recovery_pair_slot(args@, i as int) == s);
                            }
                        }
                    }
                }
                options = next;
            },
        }
        i += 1;
    }
    Ok(options)
}

pub fn admit_recovery_paths(config: &str, plan: &str) -> (out: Result<(), &'static str>)
    ensures match out {
        Ok(()) => planner::restore_prefix_matches(config, Some(".chrono-harness/"))
            && planner::restore_prefix_matches(plan, Some(".chrono-harness/state/")),
        Err(error) => !(planner::restore_prefix_matches(config, Some(".chrono-harness/"))
            && planner::restore_prefix_matches(plan, Some(".chrono-harness/state/")))
            && error@ == "E_CACHE_RECOVERY: expected host config and retained state plan"@,
    },
{
    if !planner::restore_prefix(config, Some(".chrono-harness/"))
        || !planner::restore_prefix(plan, Some(".chrono-harness/state/")) {
        return Err("E_CACHE_RECOVERY: expected host config and retained state plan");
    }
    Ok(())
}

}

#[cfg(verus_keep_ghost)]
verus! {
    /// Bind the runtime adapter's decision call to the planner's proved specification.
    pub fn recovery_action_binding(
        outcome: Option<&str>, reason: &str, unconfirmed: bool,
        outputs_is_null_or_object: bool, matched_key_is_null_or_string: bool,
        content_method: Option<&str>, content_status: Option<&str>, refresh_enabled: bool,
    ) -> (out: Result<super::RecoveryAction, super::RecoveryDecisionError>)
        ensures out == super::recovery_action_spec(
            outcome, reason, unconfirmed, outputs_is_null_or_object,
            matched_key_is_null_or_string, content_method, content_status, refresh_enabled),
    {
        super::recovery_action(
            outcome, reason, unconfirmed, outputs_is_null_or_object,
            matched_key_is_null_or_string, content_method, content_status, refresh_enabled,
        )
    }
}

#[cfg(not(verus_keep_ghost))]
use chrono_harness::{decode, no_symlink_parents, prepared, sha256};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{collections::BTreeMap, fs, io::Write, path::Path};

#[cfg(not(verus_keep_ghost))]
fn restore_status(cache: &Value, restore: &Value) -> &'static str {
    let prefixes: Vec<_> = cache["restore_keys"]
        .as_array()
        .into_iter()
        .flatten()
        .map(Value::as_str)
        .collect();
    super::verified_planner::restore_status(
        restore["outcome"].as_str(),
        cache["key"].as_str(),
        restore["outputs"]["cache-matched-key"].as_str(),
        &prefixes,
    )
}

#[cfg(not(verus_keep_ghost))]
fn publish_once(root: &Path, path: &str, bytes: &[u8]) -> Result<(), String> {
    let destination = chrono_harness::prepare_publication(root, path)?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .and_then(|mut file| file.write_all(bytes))
        .map_err(|e| format!("E_CACHE_RECOVERY: {path}: {e}"))
}

#[cfg(not(verus_keep_ghost))]
fn retained(root: &Path, plan_path: &str) -> Result<(prepared::Original, Value), String> {
    let directory = plan_path
        .rsplit_once('/')
        .ok_or("E_CACHE_RECOVERY: plan directory")?
        .0;
    let pointer = fs::read(no_symlink_parents(
        root,
        &format!("{directory}/recovery.json"),
    )?)
    .map_err(|e| format!("E_CACHE_RECOVERY: missing recovery result: {e}"))?;
    let original: prepared::Original = decode(&pointer)?;
    let raw = prepared::read_original(root, &original, None)?;
    Ok((original, decode(&raw)?))
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn dispatch(args: &[String]) -> Result<String, String> {
    let options = parse_recovery_options(args)?;
    let root = fs::canonicalize(options.required(&RecoveryOption::HostRoot)?)
        .map_err(|e| e.to_string())?;
    let _activity = chrono_harness::retained_artifacts::retention::read_scope(&root)?;
    let config_path = options.required(&RecoveryOption::Config)?;
    let plan_path = options.required(&RecoveryOption::Plan)?;
    admit_recovery_paths(config_path, plan_path)?;
    let plan_raw = fs::read(no_symlink_parents(&root, plan_path)?).map_err(|e| e.to_string())?;
    let plan: Value = decode(&plan_raw)?;
    let steps_raw = std::env::var(options.required(&RecoveryOption::StepsEnvironment)?)
        .map_err(|e| e.to_string())?;
    let directory = plan_path
        .rsplit_once('/')
        .ok_or("E_CACHE_RECOVERY: plan directory")?
        .0
        .to_owned()
        + "/";
    let output = format!("{directory}recovery.json");
    let intent_path = format!("{directory}recovery-intent.json");
    let binding = json!({"path":plan_path,"sha256":sha256(&plan_raw)});
    let steps_hash = sha256(steps_raw.as_bytes());
    let output_file = no_symlink_parents(&root, &output)?;
    match fs::symlink_metadata(&output_file) {
        Ok(_) => {
            let (original, report) = retained(&root, plan_path)?;
            if report["schema"] != "chrono-cache-recovery/v1"
                || report["plan"] != binding
                || report["native_steps"]["sha256"] != steps_hash
            {
                return Err(
                    "E_CACHE_RECOVERY: previous recovery belongs to different inputs".into(),
                );
            }
            if !matches!(
                report["status"].as_str(),
                Some("recovered" | "not-required")
            ) {
                return Err(format!(
                    "E_CACHE_RECOVERY: previous attempt failed; original {}",
                    original.path
                ));
            }
            // This is the previous operation's result, never a second deletion of
            // outputs that a later producer may already have rebuilt.
            return Ok(serde_json::to_string(
                &json!({"report":original,"observation":report,"replayed":true}),
            )
            .map_err(|e| e.to_string())?
                + "\n");
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("E_CACHE_RECOVERY: result metadata: {e}")),
    }
    match fs::symlink_metadata(no_symlink_parents(&root, &intent_path)?) {
        Ok(_) => {
            return Err(format!(
                "E_CACHE_RECOVERY: unfinished original intent {intent_path}; no automatic repeat"
            ));
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("E_CACHE_RECOVERY: intent metadata: {e}")),
    }
    let steps_original =
        super::retain_original(&root, &directory, "restore-steps", steps_raw.as_bytes())?;
    let mut report = json!({"schema":"chrono-cache-recovery/v1","plan":binding,"native_steps":steps_original,
        "status":"not-required","caches":{},"error":null,
        "limits":["isolated provider restore phase, before producers", "registered native observations only; no archive integrity or build verdict", "missing matched keys do not distinguish miss from backend or extraction failure", "linked worktree recovery unsupported"]});
    let result = (|| -> Result<(), String> {
        let config: super::Config =
            decode(&fs::read(no_symlink_parents(&root, config_path)?).map_err(|e| e.to_string())?)?;
        if !config.recover_failed_restores {
            return Err("E_CACHE_RECOVERY: recovery is not registered".into());
        }
        let consumer = plan["consumer"]
            .as_str()
            .ok_or("E_CACHE_RECOVERY: consumer missing")?;
        let expected = super::prepare_with_inputs(&root, &config, consumer, Some(&plan["inputs"]))?;
        // Recovery may use an installed binary after a bootstrap-produced plan.
        // Revalidate registration, host and selected observations while retaining
        // the original planner identity. Seal checks the current executable.
        let mut expected_binding = expected;
        expected_binding["planner"] = plan["planner"].clone();
        if expected_binding != plan {
            return Err(
                "E_CACHE_RECOVERY: original plan differs from current registration or host".into(),
            );
        }
        super::attempt::validate(&root, &plan, false)?;
        let steps: Value = decode(steps_raw.as_bytes())?;
        if !steps.is_object() {
            return Err("E_CACHE_RECOVERY: native step observations missing".into());
        }
        let caches: BTreeMap<_, _> = plan["caches"]
            .as_object()
            .ok_or("E_CACHE_RECOVERY: caches missing")?
            .iter()
            .collect();
        let mut paths = vec![];
        let mut refreshes = vec![];
        for (index, (id, cache)) in caches.into_iter().enumerate() {
            let restore = &steps[format!("cache_{index}_restore")];
            let reason = restore_status(cache, restore);
            let outputs = &restore["outputs"];
            let outputs_is_null_or_object = outputs.is_null() || outputs.is_object();
            let matched_key_is_null_or_string = outputs
                .get("cache-matched-key")
                .is_none_or(|key| key.is_null() || key.is_string());
            let outcome = restore["outcome"].as_str();
            let base_action = super::verified_planner::recovery_action(
                outcome,
                reason,
                config.recover_unconfirmed_restores,
                outputs_is_null_or_object,
                matched_key_is_null_or_string,
                None,
                None,
                false,
            );
            let base_action = match base_action {
                Ok(action) => action,
                Err(super::verified_planner::RecoveryDecisionError::MalformedOutputs) => {
                    return Err(format!(
                        "E_CACHE_RECOVERY: malformed restore outputs for {id}"
                    ));
                }
                Err(super::verified_planner::RecoveryDecisionError::UnknownOutcome) => {
                    return Err(format!(
                        "E_CACHE_RECOVERY: missing, cancelled or unknown restore outcome for {id}"
                    ));
                }
            };
            let content_method = cache["content_verification"].as_str();
            let mut verification = None;
            if matches!(base_action, super::verified_planner::RecoveryAction::Keep)
                && super::verified_planner::content_verification_applies(content_method, reason)
            {
                let verified = super::content::verify(
                    &root,
                    &directory,
                    id,
                    cache,
                    restore["outputs"]["cache-matched-key"].as_str().unwrap(),
                )?;
                verification = Some(verified);
            }
            let content_status = verification
                .as_ref()
                .and_then(|value| value["status"].as_str());
            let action = super::verified_planner::recovery_action(
                outcome,
                reason,
                config.recover_unconfirmed_restores,
                outputs_is_null_or_object,
                matched_key_is_null_or_string,
                content_method,
                content_status,
                config.caches[id].refresh_sources,
            );
            let action = match action {
                Ok(action) => action,
                Err(super::verified_planner::RecoveryDecisionError::MalformedOutputs) => {
                    return Err(format!(
                        "E_CACHE_RECOVERY: malformed restore outputs for {id}"
                    ));
                }
                Err(super::verified_planner::RecoveryDecisionError::UnknownOutcome) => {
                    return Err(format!(
                        "E_CACHE_RECOVERY: missing, cancelled or unknown restore outcome for {id}"
                    ));
                }
            };
            let discard = matches!(action, super::verified_planner::RecoveryAction::Discard);
            report["caches"][id] = json!({"restore":restore,"reason":reason,"status":if discard {"pending"} else {"not-required"},"artifacts":[]});
            if let Some(verification) = verification {
                report["caches"][id]["verification"] = verification;
                if discard {
                    report["caches"][id]["status"] = json!("pending");
                }
            }
            if !discard {
                if matches!(action, super::verified_planner::RecoveryAction::Refresh) {
                    let refresh = super::freshness::prepare(
                        &root,
                        &config.caches[id],
                        cache,
                        &plan["inputs"],
                    )?;
                    report["caches"][id]["source_freshness"] =
                        json!({"method":super::freshness::METHOD,"status":"pending"});
                    refreshes.push((id.clone(), refresh));
                }
                continue;
            }
            let artifacts: BTreeMap<_, _> = cache["artifacts"]
                .as_object()
                .ok_or("E_CACHE_RECOVERY: artifacts missing")?
                .iter()
                .collect();
            for (_, artifact) in artifacts {
                let path = artifact["path"]
                    .as_str()
                    .ok_or("E_CACHE_RECOVERY: artifact path")?;
                let target = super::path(&root, path)?;
                if !target.starts_with(&root) {
                    return Err(
                        "E_CACHE_RECOVERY: only registered host artifacts may be discarded".into(),
                    );
                }
                let position = report["caches"][id]["artifacts"].as_array().unwrap().len();
                report["caches"][id]["artifacts"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"path":path,"status":"pending"}));
                paths.push((id.clone(), position, path.to_owned()));
            }
        }
        // Validate the complete selected set before the first removal, and
        // publish the original plan/restore binding before touching any output.
        publish_once(
            &root,
            &intent_path,
            &serde_json::to_vec(&report).map_err(|e| e.to_string())?,
        )?;
        for (id, position, path) in paths {
            let removed = (|| -> Result<&str, String> {
                let target = super::path(&root, &path)?;
                match fs::symlink_metadata(&target) {
                    Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(target),
                    Ok(metadata) if metadata.is_file() => fs::remove_file(target),
                    Ok(_) => {
                        return Err(
                            "cache artifact is neither a regular file nor a directory".into()
                        );
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok("absent"),
                    Err(e) => return Err(e.to_string()),
                }
                .map_err(|e| e.to_string())?;
                Ok("removed")
            })();
            let row = &mut report["caches"][&id]["artifacts"][position];
            match removed {
                Ok(status) => row["status"] = json!(status),
                Err(error) => {
                    row["status"] = json!("failed");
                    row["error"] = json!(error);
                    return Err(format!("E_CACHE_RECOVERY: {path}: {error}"));
                }
            }
        }
        for cache in report["caches"].as_object_mut().unwrap().values_mut() {
            if cache["status"] == "pending" {
                cache["status"] = json!("discarded");
            }
        }
        for (id, refresh) in refreshes {
            super::freshness::apply(
                &root,
                refresh,
                &mut report["caches"][&id]["source_freshness"],
            )?;
            report["caches"][&id]["status"] = json!("source-times-refreshed");
        }
        if report["caches"]
            .as_object()
            .unwrap()
            .values()
            .any(|v| v["status"] == "discarded" || v["status"] == "source-times-refreshed")
        {
            report["status"] = json!("recovered");
        }
        Ok(())
    })();
    if let Err(error) = &result {
        report["status"] = json!("failed");
        report["error"] = json!(error);
    }
    let raw = serde_json::to_vec(&report).map_err(|e| e.to_string())?;
    let publication = (|| {
        let original = super::retain_original(&root, &directory, "cache-recovery", &raw)?;
        publish_once(
            &root,
            &output,
            &serde_json::to_vec(&original).map_err(|e| e.to_string())?,
        )?;
        Ok::<_, String>(original)
    })();
    let original = publication.map_err(|publication| {
        format!(
            "E_CACHE_RECOVERY: recovery {:?}; publication {publication}",
            result.as_ref().err()
        )
    })?;
    if let Err(error) = result {
        return Err(format!("{error}; original {}", original.path));
    }
    let changed_or_failed = report["caches"].as_object().unwrap().values().any(|cache| {
        cache["restore"]["outcome"] == "failure"
            || cache["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|artifact| artifact["status"] == "removed")
    });
    if report["status"] == "recovered" && changed_or_failed {
        eprintln!(
            "W_CACHE_RESTORE_RECOVERED: unusable restore outputs cleared before registered build; original {}",
            original.path
        );
    }
    Ok(
        serde_json::to_string(&json!({"report":original,"observation":report,"replayed":false}))
            .map_err(|e| e.to_string())?
            + "\n",
    )
}
