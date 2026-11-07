//! 当前 planner 身份的独立成功、拒绝与完整文本边界预期。
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

verus! {

fn expect(
    ds: Option<&str>, dv: Option<&str>, os: Option<&str>, ov: Option<&str>,
    expected: bool,
)
    requires expected == policy::planner_identity_matches(ds, dv, os, ov),
{
    let actual = policy::matches_planner_identity(ds, dv, os, ov);
    assert(actual == expected);
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
