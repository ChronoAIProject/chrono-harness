//! 当前缓存格式及报告和封存共用的 planner 身份判断。
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;

#[cfg(verus_keep_ghost)]
#[path = "attempt.rs"]
mod attempt;
#[cfg(not(verus_keep_ghost))]
pub use crate::attempt::{
    accept_original, confirmed_probe, release_custody, replay_attempt, same_attempt,
};
#[cfg(verus_keep_ghost)]
pub use attempt::{
    accept_original, confirmed_probe, release_custody, replay_attempt, same_attempt,
};

verus! {

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
