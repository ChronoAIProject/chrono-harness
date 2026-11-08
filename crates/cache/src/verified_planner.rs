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

verus! {

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
