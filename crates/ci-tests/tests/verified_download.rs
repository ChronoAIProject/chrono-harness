//! 下载状态与停止条件的独立预期，同时由 Verus 和普通 Rust 测试执行。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../ci/src/verified_download.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_ci::verified_download as policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

fn expect_true(actual: bool)
    requires actual,
{
    if !actual { panic!(); }
}

proof fn parsed_result_is_unique(bytes: Seq<u8>, left: Option<u16>, right: Option<u16>)
    requires policy::parsed(bytes, left), policy::parsed(bytes, right),
    ensures left == right,
    decreases if left.is_none() { 1nat } else { 0nat },
{
    match (left, right) {
        (Some(a), Some(b)) => {
            let i = choose|i: int| 0 <= i < bytes.len()
                && policy::token_at(bytes, i) && policy::code_at(bytes, i) == a;
            assert(policy::code_at(bytes, i) == b);
        },
        (Some(a), None) => {
            let i = choose|i: int| 0 <= i < bytes.len()
                && policy::token_at(bytes, i) && policy::code_at(bytes, i) == a;
            assert(!policy::prefix(bytes, bytes.len() as int, None));
            let (j, k) = choose|j: int, k: int| policy::token_at(bytes, j)
                && policy::token_at(bytes, k) && policy::code_at(bytes, j) != policy::code_at(bytes, k);
            assert(policy::code_at(bytes, j) == a && policy::code_at(bytes, k) == a);
        },
        (None, Some(b)) => {
            parsed_result_is_unique(bytes, right, left);
        },
        _ => {},
    }
}

fn expect(bytes: &[u8], expected: Option<u16>)
    requires policy::parsed(bytes@, expected),
{
    let actual = policy::http_status(bytes);
    proof { parsed_result_is_unique(bytes@, actual, expected); }
    expect_true(actual == expected);
}

#[cfg(not(verus_keep_ghost))]
#[test]
#[should_panic]
fn runtime_http_expectation_keeps_its_failure_branch() {
    expect(&[], Some(503));
}

#[cfg(not(verus_keep_ghost))]
#[test]
#[should_panic]
fn runtime_retry_expectation_keeps_its_failure_branch() {
    expect_true(false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn only_complete_ascii_status_tokens_are_accepted() {
    expect(&[], None);
    expect(&[72u8, 84, 84, 80, 32, 53, 48, 51], None);
    let valid: &[u8] = &[72u8, 84, 84, 80, 32, 53, 48, 51, 58];
    proof {
        assert(policy::token_at(valid@, 0));
        assert(policy::code_at(valid@, 0) == 503);
    }
    expect(valid, Some(503));
    expect(&[104u8, 84, 84, 80, 32, 53, 48, 51, 58], None);
    expect(&[72u8, 84, 84, 80, 32, 53, 255, 51, 58], None);
    expect(&[72u8, 84, 84, 80, 32, 53, 48, 51, 59], None);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn repeated_statuses_are_unambiguous_but_conflicting_statuses_are_rejected() {
    let repeated: &[u8] = &[
        72u8, 84, 84, 80, 32, 53, 48, 51, 58,
        72, 84, 84, 80, 32, 53, 48, 51, 58,
    ];
    proof {
        assert(policy::token_at(repeated@, 0) && policy::token_at(repeated@, 9));
        assert(policy::code_at(repeated@, 0) == 503 && policy::code_at(repeated@, 9) == 503);
        assert forall|i: int| policy::token_at(repeated@, i) implies i == 0 || i == 9 by {}
    }
    expect(repeated, Some(503));
    let conflict: &[u8] = &[
        72u8, 84, 84, 80, 32, 53, 48, 51, 58,
        72, 84, 84, 80, 32, 52, 48, 49, 58,
    ];
    proof {
        assert(policy::token_at(conflict@, 0) && policy::token_at(conflict@, 9));
        assert(policy::code_at(conflict@, 0) == 503 && policy::code_at(conflict@, 9) == 401);
    }
    expect(conflict, None);
    let embedded: &[u8] = &[255u8, 72, 84, 84, 80, 32, 48, 48, 48, 58, 0];
    proof {
        assert(policy::token_at(embedded@, 1));
        assert(policy::code_at(embedded@, 1) == 0);
        assert forall|i: int| policy::token_at(embedded@, i) implies i == 1 by {}
    }
    expect(embedded, Some(0));
    let maximum: &[u8] = &[72u8, 84, 84, 80, 32, 57, 57, 57, 58];
    proof {
        assert(policy::token_at(maximum@, 0));
        assert(policy::code_at(maximum@, 0) == 999);
    }
    expect(maximum, Some(999));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn every_missing_retry_obligation_stops_another_download() {
    let allowed = policy::retry(false, false, true, true, 1, 1, true);
    let successful = policy::retry(true, false, true, true, 1, 1, true);
    let failed_process = policy::retry(false, true, true, true, 1, 1, true);
    let exhausted = policy::retry(false, false, false, true, 1, 1, true);
    let unlisted = policy::retry(false, false, true, false, 1, 1, true);
    let no_stdout = policy::retry(false, false, true, true, 0, 1, true);
    let no_stderr = policy::retry(false, false, true, true, 1, 0, true);
    let deadline = policy::retry(false, false, true, true, 1, 1, false);
    expect_true(allowed && !successful && !failed_process && !exhausted && !unlisted
        && !no_stdout && !no_stderr && !deadline);
}

}
