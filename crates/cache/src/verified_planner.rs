//! 当前缓存格式及报告和封存共用的 planner 身份判断。
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

#[path = "verified_content.rs"]
pub mod content_preflight;

#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;

#[cfg(verus_keep_ghost)]
#[path = "attempt.rs"]
mod attempt;
#[cfg(verus_keep_ghost)]
#[path = "recovery.rs"]
mod recovery_source;
#[cfg(not(verus_keep_ghost))]
pub use crate::attempt::{
    accept_original, confirmed_probe, release_custody, replay_attempt, same_attempt,
};
#[cfg(not(verus_keep_ghost))]
pub use crate::recovery::{
    RecoveryOption, RecoveryOptions, RecoveryReplay, RecoverySummary, admit_recovery_paths,
    admit_recovery_replay, complete_recovery_cache, parse_recovery_options,
};
#[cfg(verus_keep_ghost)]
pub use attempt::{
    accept_original, confirmed_probe, release_custody, replay_attempt, same_attempt,
};
#[cfg(verus_keep_ghost)]
pub use recovery_source::*;

verus! {

pub open spec fn restore_prefix_matches(key: &str, prefix: Option<&str>) -> bool {
    match prefix {
        Some(prefix) => prefix.spec_bytes().len() > 0
            && prefix.spec_bytes().len() <= key.spec_bytes().len()
            && forall|i: int| 0 <= i < prefix.spec_bytes().len()
                ==> key.spec_bytes()[i] == prefix.spec_bytes()[i],
        None => false,
    }
}

pub fn restore_prefix(key: &str, prefix: Option<&str>) -> (matched: bool)
    ensures matched == restore_prefix_matches(key, prefix),
{
    let prefix_text = match prefix {
        Some(prefix_text) => prefix_text,
        None => return false,
    };
    let key_bytes = key.as_bytes();
    let prefix_bytes = prefix_text.as_bytes();
    if prefix_bytes.len() == 0 || prefix_bytes.len() > key_bytes.len() {
        return false;
    }
    let mut i = 0;
    while i < prefix_bytes.len()
        invariant i <= prefix_bytes.len(), prefix_bytes.len() <= key_bytes.len(),
            prefix_bytes.len() > 0,
            prefix == Some(prefix_text),
            key_bytes@ == key.spec_bytes(), prefix_bytes@ == prefix_text.spec_bytes(),
            forall|j: int| 0 <= j < i ==> key_bytes[j] == prefix_bytes[j],
        decreases prefix_bytes.len() - i,
    {
        if key_bytes[i] != prefix_bytes[i] {
            proof {
                assert(key.spec_bytes()[i as int] != prefix_text.spec_bytes()[i as int]);
                assert(!restore_prefix_matches(key, prefix));
            }
            return false;
        }
        i += 1;
    }
    true
}

pub open spec fn compatible_restore_key(key: &str, prefixes: Seq<Option<&str>>) -> bool {
    exists|i: int| 0 <= i < prefixes.len()
        && restore_prefix_matches(key, #[trigger] prefixes[i])
}

fn compatible_restore(key: &str, prefixes: &[Option<&str>]) -> (matched: bool)
    ensures matched == compatible_restore_key(key, prefixes@),
{
    let mut i = 0;
    while i < prefixes.len()
        invariant i <= prefixes.len(),
            forall|j: int| 0 <= j < i
                ==> !restore_prefix_matches(key, #[trigger] prefixes[j]),
        decreases prefixes.len() - i,
    {
        if restore_prefix(key, prefixes[i]) {
            return true;
        }
        i += 1;
    }
    false
}

pub open spec fn restore_status_spec(
    outcome: Option<&str>, requested: Option<&str>, matched: Option<&str>,
    prefixes: Seq<Option<&str>>,
) -> Seq<char> {
    match outcome {
        Some(outcome) => if outcome@ == "success"@ {
            match matched {
                Some(key) => if key.spec_bytes().len() == 0 {
                    "miss-or-unavailable"@
                } else if match requested { Some(requested) => requested@ == key@, None => false } {
                    "exact"@
                } else if compatible_restore_key(key, prefixes) {
                    "compatible"@
                } else {
                    "incompatible"@
                },
                None => "miss-or-unavailable"@,
            }
        } else if outcome@ == "failure"@ || outcome@ == "cancelled"@ {
            "error"@
        } else if outcome@ == "skipped"@ {
            "not-attempted"@
        } else {
            "unavailable"@
        },
        None => "unavailable"@,
    }
}

/// 原生恢复结果的完整文本和全部前缀共同决定原因；空／非文本前缀从不授权恢复。
pub fn restore_status(
    outcome: Option<&str>, requested: Option<&str>, matched: Option<&str>,
    prefixes: &[Option<&str>],
) -> (status: &'static str)
    ensures status@ == restore_status_spec(outcome, requested, matched, prefixes@),
{
    let outcome = match outcome {
        Some(outcome) => outcome,
        None => return "unavailable",
    };
    if identifiers::equal_text(outcome, "success") {
        let key = match matched {
            Some(key) => key,
            None => return "miss-or-unavailable",
        };
        if key.as_bytes().len() == 0 { return "miss-or-unavailable"; }
        if let Some(requested) = requested {
            if identifiers::equal_text(requested, key) { return "exact"; }
        }
        if compatible_restore(key, prefixes) { "compatible" } else { "incompatible" }
    } else if identifiers::equal_text(outcome, "failure")
        || identifiers::equal_text(outcome, "cancelled") {
        "error"
    } else if identifiers::equal_text(outcome, "skipped") {
        "not-attempted"
    } else {
        "unavailable"
    }
}

pub fn producer_can_seal(save_on_failure: bool, outcome: Option<&str>) -> (allowed: bool)
    ensures allowed <==> match outcome {
        Some(value) => value@ == "success"@ || (save_on_failure && value@ == "failure"@),
        None => false,
    },
{
    match outcome {
        Some(value) => identifiers::equal_text(value, "success")
            || (save_on_failure && identifiers::equal_text(value, "failure")),
        None => false,
    }
}

pub fn refresh_sources(enabled: bool, reason: &str) -> (refresh: bool)
    ensures refresh <==> (enabled && reason@ == "compatible"@),
{
    enabled && identifiers::equal_text(reason, "compatible")
}

/// The filesystem adapter reports observations; this function owns the
/// freshness state transition after each observation. It does not interpret
/// timestamps or filesystem errors.
pub enum FreshnessDecision { AlreadyNewer, Updated, Retry, Failed }

pub open spec fn freshness_decision_spec(
    current_newer: bool, first_newer: bool, second_newer: bool,
    second_attempted: bool,
) -> FreshnessDecision {
    if current_newer {
        FreshnessDecision::AlreadyNewer
    } else if first_newer {
        FreshnessDecision::Updated
    } else if second_attempted {
        if second_newer {
            FreshnessDecision::Updated
        } else {
            FreshnessDecision::Failed
        }
    } else {
        FreshnessDecision::Retry
    }
}

pub fn freshness_decision(
    current_newer: bool, first_newer: bool, second_newer: bool,
    second_attempted: bool,
) -> (decision: FreshnessDecision)
    ensures decision == freshness_decision_spec(
        current_newer, first_newer, second_newer, second_attempted),
{
    if current_newer {
        FreshnessDecision::AlreadyNewer
    } else if first_newer {
        FreshnessDecision::Updated
    } else if second_attempted {
        if second_newer { FreshnessDecision::Updated } else { FreshnessDecision::Failed }
    } else {
        FreshnessDecision::Retry
    }
}

pub enum RecoveryAction { Keep, Discard, Refresh }

pub enum RecoveryDecisionError { UnknownOutcome, MalformedOutputs }

pub open spec fn recovery_action_spec(
    outcome: Option<&str>, reason: &str, unconfirmed: bool,
    outputs_is_null_or_object: bool, matched_key_is_null_or_string: bool,
    content_method: Option<&str>, content_status: Option<&str>, refresh_enabled: bool,
) -> Result<RecoveryAction, RecoveryDecisionError> {
    let base = match outcome {
        Some(value) => if value@ == "failure"@ {
            Ok(true)
        } else if value@ == "success"@ {
            if unconfirmed {
                if !outputs_is_null_or_object || !matched_key_is_null_or_string {
                    Err(RecoveryDecisionError::MalformedOutputs)
                } else {
                    Ok(reason@ == "miss-or-unavailable"@ || reason@ == "incompatible"@)
                }
            } else {
                Ok(false)
            }
        } else if value@ == "skipped"@ {
            Ok(false)
        } else {
            Err(RecoveryDecisionError::UnknownOutcome)
        },
        None => Err(RecoveryDecisionError::UnknownOutcome),
    };
    match base {
        Err(error) => Err(error),
        Ok(true) => Ok(RecoveryAction::Discard),
        Ok(false) => {
            let verify_content = match content_method {
                Some(method) => method@ == "producer-manifest/v1"@
                    && (reason@ == "exact"@ || reason@ == "compatible"@),
                None => false,
            };
            let content_matched = match content_status {
                Some(status) => status@ == "matched"@,
                None => false,
            };
            if verify_content && !content_matched {
                Ok(RecoveryAction::Discard)
            } else if refresh_enabled && reason@ == "compatible"@ {
                Ok(RecoveryAction::Refresh)
            } else {
                Ok(RecoveryAction::Keep)
            }
        }
    }
}

pub fn recovery_action(
    outcome: Option<&str>, reason: &str, unconfirmed: bool,
    outputs_is_null_or_object: bool, matched_key_is_null_or_string: bool,
    content_method: Option<&str>, content_status: Option<&str>, refresh_enabled: bool,
) -> (out: Result<RecoveryAction, RecoveryDecisionError>)
    ensures out == recovery_action_spec(
        outcome, reason, unconfirmed, outputs_is_null_or_object,
        matched_key_is_null_or_string, content_method, content_status, refresh_enabled),
{
    let discard = match outcome {
        Some(value) if identifiers::equal_text(value, "failure") => true,
        Some(value) if identifiers::equal_text(value, "success") => {
            if unconfirmed {
                if !outputs_is_null_or_object || !matched_key_is_null_or_string {
                    return Err(RecoveryDecisionError::MalformedOutputs);
                }
                identifiers::equal_text(reason, "miss-or-unavailable")
                    || identifiers::equal_text(reason, "incompatible")
            } else {
                false
            }
        }
        Some(value) if identifiers::equal_text(value, "skipped") => false,
        Some(_) | None => return Err(RecoveryDecisionError::UnknownOutcome),
    };
    if discard {
        return Ok(RecoveryAction::Discard);
    }
    let verify_content = match content_method {
        Some(method) => identifiers::equal_text(method, "producer-manifest/v1")
            && (identifiers::equal_text(reason, "exact")
                || identifiers::equal_text(reason, "compatible")),
        None => false,
    };
    if verify_content && !matches!(content_status, Some(status) if identifiers::equal_text(status, "matched")) {
        return Ok(RecoveryAction::Discard);
    }
    if refresh_enabled && identifiers::equal_text(reason, "compatible") {
        Ok(RecoveryAction::Refresh)
    } else {
        Ok(RecoveryAction::Keep)
    }
}

pub fn content_verification_applies(method: Option<&str>, reason: &str) -> (applies: bool)
    ensures applies <==> match method {
        Some(method) => method@ == "producer-manifest/v1"@
            && (reason@ == "exact"@ || reason@ == "compatible"@),
        None => false,
    },
{
    match method {
        Some(method) => identifiers::equal_text(method, "producer-manifest/v1")
            && (identifiers::equal_text(reason, "exact")
                || identifiers::equal_text(reason, "compatible")),
        None => false,
    }
}

pub fn current_registration(schema: &str) -> (valid: bool)
    ensures valid <==> schema@ == "chrono-cache/v2"@,
{
    identifiers::equal_text(schema, "chrono-cache/v2")
}

pub open spec fn current_plan_schema(schema: Option<&str>) -> bool {
    match schema {
        Some(schema) => schema@ == "chrono-cache-plan/v2"@,
        None => false,
    }
}

pub fn current_plan(schema: Option<&str>) -> (valid: bool)
    ensures valid <==> current_plan_schema(schema),
{
    match schema {
        Some(schema) => identifiers::equal_text(schema, "chrono-cache-plan/v2"),
        None => false,
    }
}

pub open spec fn planner_identity_matches(
    declared_sha256: Option<&str>, declared_version: Option<&str>,
    observed_sha256: Option<&str>, observed_version: Option<&str>,
) -> bool {
    match (declared_sha256, declared_version, observed_sha256, observed_version) {
        (Some(ds), Some(dv), Some(os), Some(ov)) => ds@ == os@ && dv@ == ov@,
        _ => false,
    }
}

pub fn matches_planner_identity(
    declared_sha256: Option<&str>, declared_version: Option<&str>,
    observed_sha256: Option<&str>, observed_version: Option<&str>,
) -> (matched: bool)
    ensures matched <==> planner_identity_matches(
        declared_sha256, declared_version, observed_sha256, observed_version),
{
    match (declared_sha256, declared_version, observed_sha256, observed_version) {
        (Some(ds), Some(dv), Some(os), Some(ov)) =>
            identifiers::equal_text(ds, os) && identifiers::equal_text(dv, ov),
        _ => false,
    }
}

}
