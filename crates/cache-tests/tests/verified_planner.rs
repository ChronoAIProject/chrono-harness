//! 当前缓存格式与 planner 身份的独立成功、拒绝与完整文本边界预期。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../cache/src/verified_planner.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_cache::verified_planner as policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn only_observed_success_or_opted_in_failure_can_seal() {
    proof {
        reveal_strlit("success"); reveal_strlit("failure");
        reveal_strlit("skipped"); reveal_strlit("cancelled");
        reveal_strlit("success\n"); reveal_strlit("");
        assert("success"@[0] != "failure"@[0]);
        assert("success"@[1] != "skipped"@[1]);
        assert("failure"@[0] != "skipped"@[0]);
        assert("success"@.len() != "cancelled"@.len());
        assert("failure"@.len() != "cancelled"@.len());
        assert("success"@.len() != "success\n"@.len());
        assert("failure"@.len() != "success\n"@.len());
        assert("success"@.len() != ""@.len());
        assert("failure"@.len() != ""@.len());
    }
    expect_cache_true(policy::producer_can_seal(false, Some("success")));
    expect_cache_true(policy::producer_can_seal(true, Some("success")));
    expect_cache_true(!policy::producer_can_seal(false, Some("failure")));
    expect_cache_true(policy::producer_can_seal(true, Some("failure")));
    expect_cache_true(!policy::producer_can_seal(true, Some("skipped")));
    expect_cache_true(!policy::producer_can_seal(true, Some("cancelled")));
    expect_cache_true(!policy::producer_can_seal(true, Some("success\n")));
    expect_cache_true(!policy::producer_can_seal(true, Some("")));
    expect_cache_true(!policy::producer_can_seal(false, None));
    expect_cache_true(!policy::producer_can_seal(true, None));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn only_adopted_compatible_restores_refresh_inputs() {
    proof {
        reveal_strlit("compatible"); reveal_strlit("exact");
        reveal_strlit("error"); reveal_strlit("miss-or-unavailable");
        reveal_strlit("compatible\n"); reveal_strlit("");
        assert("compatible"@.len() != "exact"@.len());
        assert("compatible"@.len() != "error"@.len());
        assert("compatible"@.len() != "miss-or-unavailable"@.len());
        assert("compatible"@.len() != "compatible\n"@.len());
        assert("compatible"@.len() != ""@.len());
    }
    expect_cache_true(policy::refresh_sources(true, "compatible"));
    expect_cache_true(!policy::refresh_sources(false, "compatible"));
    expect_cache_true(!policy::refresh_sources(true, "exact"));
    expect_cache_true(!policy::refresh_sources(true, "error"));
    expect_cache_true(!policy::refresh_sources(true, "miss-or-unavailable"));
    expect_cache_true(!policy::refresh_sources(true, "compatible\n"));
    expect_cache_true(!policy::refresh_sources(true, ""));
}

// 同一预期也在普通 Cargo 中实际执行。
fn expect_cache_true(actual: bool)
    requires actual,
{
    if !actual {
        panic!();
    }
}

fn expect_registration(schema: &str, expected: bool)
    requires expected == (schema@ == "chrono-cache/v2"@),
{
    let actual = policy::current_registration(schema);
    assert(actual == expected);
    expect_cache_true(actual == expected);
}

fn expect_plan(schema: Option<&str>, expected: bool)
    requires expected == policy::current_plan_schema(schema),
{
    let actual = policy::current_plan(schema);
    assert(actual == expected);
    expect_cache_true(actual == expected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn only_current_registration_and_plan_schemas_are_accepted() {
    proof {
        reveal_strlit("chrono-cache/v2"); reveal_strlit("chrono-cache/v1");
        reveal_strlit("chrono-cache/v20"); reveal_strlit("chrono-cache/v2\n");
        reveal_strlit("chrono-cache-plan/v2"); reveal_strlit("chrono-cache-plan/v1");
        reveal_strlit("chrono-cache-plan/v20"); reveal_strlit("chrono-cache-plan/v2\n");
        reveal_strlit("");
        assert("chrono-cache/v2"@[14] != "chrono-cache/v1"@[14]);
        assert("chrono-cache/v2"@.len() != "chrono-cache/v20"@.len());
        assert("chrono-cache/v2"@.len() != "chrono-cache/v2\n"@.len());
        assert("chrono-cache/v2"@.len() != ""@.len());
        assert("chrono-cache-plan/v2"@[19] != "chrono-cache-plan/v1"@[19]);
        assert("chrono-cache-plan/v2"@.len() != "chrono-cache-plan/v20"@.len());
        assert("chrono-cache-plan/v2"@.len() != "chrono-cache-plan/v2\n"@.len());
        assert("chrono-cache-plan/v2"@.len() != ""@.len());
    }
    expect_registration("chrono-cache/v2", true);
    expect_registration("chrono-cache/v1", false);
    expect_registration("chrono-cache/v20", false);
    expect_registration("chrono-cache/v2\n", false);
    expect_registration("", false);
    expect_plan(Some("chrono-cache-plan/v2"), true);
    expect_plan(Some("chrono-cache-plan/v1"), false);
    expect_plan(Some("chrono-cache-plan/v20"), false);
    expect_plan(Some("chrono-cache-plan/v2\n"), false);
    expect_plan(Some(""), false);
    expect_plan(None, false);
}

fn expect(
    ds: Option<&str>, dv: Option<&str>, os: Option<&str>, ov: Option<&str>,
    expected: bool,
)
    requires expected == policy::planner_identity_matches(ds, dv, os, ov),
{
    let actual = policy::matches_planner_identity(ds, dv, os, ov);
    assert(actual == expected);
    expect_cache_true(actual == expected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn matching_fields_accept_complete_text() {
    expect(Some("digest"), Some("1.0"), Some("digest"), Some("1.0"), true);
    expect(Some("摘要"), Some("版本\0\n"), Some("摘要"), Some("版本\0\n"), true);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn every_missing_field_rejects() {
    expect(None, None, None, None, false);
    expect(None, None, Some("digest"), Some("1.0"), false);
    expect(None, Some("1.0"), Some("digest"), Some("1.0"), false);
    expect(Some("digest"), None, Some("digest"), Some("1.0"), false);
    expect(Some("digest"), Some("1.0"), None, Some("1.0"), false);
    expect(Some("digest"), Some("1.0"), Some("digest"), None, false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn mismatches_do_not_accept_case_or_prefixes() {
    proof {
        reveal_strlit("digest"); reveal_strlit("Digest");
        reveal_strlit("digest\0"); reveal_strlit("1.0");
        reveal_strlit("1.1"); reveal_strlit("1.0\n");
        reveal_strlit("版本甲"); reveal_strlit("版本乙");
        assert("digest"@[0] != "Digest"@[0]);
        assert("digest"@.len() != "digest\0"@.len());
        assert("1.0"@[2] != "1.1"@[2]);
        assert("1.0"@.len() != "1.0\n"@.len());
        assert("版本甲"@[2] != "版本乙"@[2]);
    }
    expect(Some("digest"), Some("1.0"), Some("Digest"), Some("1.0"), false);
    expect(Some("digest"), Some("1.0"), Some("digest\0"), Some("1.0"), false);
    expect(Some("digest"), Some("1.0"), Some("digest"), Some("1.1"), false);
    expect(Some("digest"), Some("1.0"), Some("digest"), Some("1.0\n"), false);
    expect(Some("digest"), Some("版本甲"), Some("digest"), Some("版本乙"), false);
}

}
