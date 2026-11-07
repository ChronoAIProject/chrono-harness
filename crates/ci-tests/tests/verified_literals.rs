//! CI 字面文本判断的独立成功、失败与边界预期。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../ci/src/verified_literals.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_ci::verified_literals as policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

fn expect_cache_registry(schema: Option<&str>, primary_checkout: bool, expected: bool)
    requires expected == policy::cache_registry(schema, primary_checkout),
{
    let actual = policy::valid_cache_registry(schema, primary_checkout);
    assert(actual == expected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn cache_registry_requires_current_schema_and_primary_checkout() {
    proof {
        reveal_strlit("chrono-cache/v2"); reveal_strlit("chrono-cache/v1");
        reveal_strlit("chrono-cache/v20"); reveal_strlit("chrono-cache/v2\n");
        reveal_strlit("");
        assert("chrono-cache/v2"@[14] != "chrono-cache/v1"@[14]);
        assert("chrono-cache/v2"@.len() != "chrono-cache/v20"@.len());
        assert("chrono-cache/v2"@.len() != "chrono-cache/v2\n"@.len());
        assert("chrono-cache/v2"@.len() != ""@.len());
    }
    expect_cache_registry(Some("chrono-cache/v2"), true, true);
    expect_cache_registry(Some("chrono-cache/v2"), false, false);
    expect_cache_registry(Some("chrono-cache/v1"), true, false);
    expect_cache_registry(Some("chrono-cache/v20"), true, false);
    expect_cache_registry(Some("chrono-cache/v2\n"), true, false);
    expect_cache_registry(Some(""), true, false);
    expect_cache_registry(None, true, false);
    expect_cache_registry(None, false, false);
}

fn expect(bytes: &[u8], expected: bool)
    requires expected == policy::literal(bytes@),
{
    let actual = policy::valid_literal(bytes);
    assert(actual == expected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn empty_and_ordinary_utf8_text_are_accepted() {
    expect(&[], true);
    expect(&[97u8, 32, 9, 127], true);
    expect(&[228u8, 184, 173, 230, 150, 135], true);
    expect(&[36u8, 72, 79, 77, 69, 32, 96, 97, 96], true);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn forbidden_bytes_fail_at_both_ends_and_inside_text() {
    expect(&[0u8], false);
    expect(&[10u8], false);
    expect(&[13u8], false);
    expect(&[0u8, 97], false);
    expect(&[97u8, 10, 98], false);
    expect(&[97u8, 13], false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn complete_template_openers_fail_at_every_boundary() {
    expect(&[36u8, 123, 123], false);
    expect(&[36u8, 123, 123, 97], false);
    expect(&[97u8, 36, 123, 123], false);
    expect(&[97u8, 36, 123, 123, 98], false);
    expect(&[36u8, 36, 123, 123], false);
    expect(&[36u8, 123, 36, 123, 123], false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn partial_openers_and_nonadjacent_bytes_are_accepted() {
    expect(&[36u8], true);
    expect(&[36u8, 123], true);
    expect(&[123u8, 123], true);
    expect(&[36u8, 123, 97, 123], true);
    expect(&[36u8, 123, 36, 123], true);
    expect(&[36u8, 123, 32, 123], true);
}

}
