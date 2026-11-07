//! 缓存报告和封存共用的当前 planner 身份判断。
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "../../runner/src/verified_identifiers.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;

verus! {

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
