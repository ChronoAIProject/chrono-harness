//! Independent boundary expectations for workflow execution admission.
#![allow(unexpected_cfgs)]

#[path = "../../judge-workflow/src/verified_execution.rs"]
mod policy;

#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

fn expect(actual: bool)
    requires actual,
    ensures actual,
{
    if !actual { panic!(); }
}

fn expect_code(actual: &str, expected: &str)
    requires actual.spec_bytes() == expected.spec_bytes(),
    ensures actual@ == expected@,
{
    proof {
        vstd::utf8::encode_utf8_decode_utf8(actual@);
        vstd::utf8::encode_utf8_decode_utf8(expected@);
    }
    let same = policy::response::identifiers::equal_text(actual, expected);
    expect(same);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn all_five_domain_rejections_have_fail_status() {
    proof {
        reveal_strlit("E_BRANCH_STALE: stale"); reveal_strlit("E_BRANCH_STALE");
        reveal_strlit("E_RETIREMENT");
        reveal_strlit("E_REQUIRED_TEST_REMOVED: test:a:b"); reveal_strlit("E_REQUIRED_TEST_REMOVED");
        reveal_strlit("E_INTEGRATION_REQUIRED:"); reveal_strlit("E_INTEGRATION_REQUIRED");
        reveal_strlit("E_INTEGRATION_MISMATCH: mismatch"); reveal_strlit("E_INTEGRATION_MISMATCH");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("E_BRANCH_STALE: stale") && vstd::string::is_ascii("E_BRANCH_STALE"));
        assert(vstd::string::is_ascii("E_RETIREMENT"));
        assert(vstd::string::is_ascii("E_REQUIRED_TEST_REMOVED: test:a:b") && vstd::string::is_ascii("E_REQUIRED_TEST_REMOVED"));
        assert(vstd::string::is_ascii("E_INTEGRATION_REQUIRED:") && vstd::string::is_ascii("E_INTEGRATION_REQUIRED"));
        assert(vstd::string::is_ascii("E_INTEGRATION_MISMATCH: mismatch") && vstd::string::is_ascii("E_INTEGRATION_MISMATCH"));
        policy::response::identifiers::ascii_separator_end_is("E_BRANCH_STALE: stale".spec_bytes(), 58, 14);
        policy::response::identifiers::ascii_separator_end_is("E_RETIREMENT".spec_bytes(), 58, 12);
        policy::response::identifiers::ascii_separator_end_is("E_REQUIRED_TEST_REMOVED: test:a:b".spec_bytes(), 58, 23);
        policy::response::identifiers::ascii_separator_end_is("E_INTEGRATION_REQUIRED:".spec_bytes(), 58, 22);
        policy::response::identifiers::ascii_separator_end_is("E_INTEGRATION_MISMATCH: mismatch".spec_bytes(), 58, 22);
    }
    let (code, status) = policy::response::classify_error("E_BRANCH_STALE: stale");
    expect_code(code, "E_BRANCH_STALE");
    expect(matches!(status, policy::response::Status::Fail));
    let (code, status) = policy::response::classify_error("E_RETIREMENT");
    expect_code(code, "E_RETIREMENT");
    expect(matches!(status, policy::response::Status::Fail));
    let (code, status) = policy::response::classify_error("E_REQUIRED_TEST_REMOVED: test:a:b");
    expect_code(code, "E_REQUIRED_TEST_REMOVED");
    expect(matches!(status, policy::response::Status::Fail));
    let (code, status) = policy::response::classify_error("E_INTEGRATION_REQUIRED:");
    expect_code(code, "E_INTEGRATION_REQUIRED");
    expect(matches!(status, policy::response::Status::Fail));
    let (code, status) = policy::response::classify_error("E_INTEGRATION_MISMATCH: mismatch");
    expect_code(code, "E_INTEGRATION_MISMATCH");
    expect(matches!(status, policy::response::Status::Fail));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn invalid_prefixes_preserve_the_workflow_fallback() {
    proof {
        reveal_strlit("E_BRANCH_STALE"); reveal_strlit("E_RETIREMENT");
        reveal_strlit("E_REQUIRED_TEST_REMOVED"); reveal_strlit("E_INTEGRATION_REQUIRED");
        reveal_strlit("E_INTEGRATION_MISMATCH");
        reveal_strlit(""); reveal_strlit(":E_BRANCH_STALE");
        reveal_strlit("e_BRANCH_STALE: lower"); reveal_strlit(" E_BRANCH_STALE: space");
        reveal_strlit("E: short"); reveal_strlit("E_WORKFLOW_INPUT");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("") && vstd::string::is_ascii(":E_BRANCH_STALE"));
        assert(vstd::string::is_ascii("e_BRANCH_STALE: lower") && vstd::string::is_ascii(" E_BRANCH_STALE: space"));
        assert(vstd::string::is_ascii("E: short") && vstd::string::is_ascii("E_WORKFLOW_INPUT"));
        policy::response::identifiers::ascii_separator_end_is("".spec_bytes(), 58, 0);
        policy::response::identifiers::ascii_separator_end_is(":E_BRANCH_STALE".spec_bytes(), 58, 0);
        policy::response::identifiers::ascii_separator_end_is("e_BRANCH_STALE: lower".spec_bytes(), 58, 14);
        policy::response::identifiers::ascii_separator_end_is(" E_BRANCH_STALE: space".spec_bytes(), 58, 15);
        policy::response::identifiers::ascii_separator_end_is("E: short".spec_bytes(), 58, 1);
    }
    let (code, status) = policy::response::classify_error("");
    expect_code(code, "E_WORKFLOW_INPUT");
    expect(matches!(status, policy::response::Status::Error));
    let (code, status) = policy::response::classify_error(":E_BRANCH_STALE");
    expect_code(code, "E_WORKFLOW_INPUT");
    expect(matches!(status, policy::response::Status::Error));
    let (code, status) = policy::response::classify_error("e_BRANCH_STALE: lower");
    expect_code(code, "E_WORKFLOW_INPUT");
    expect(matches!(status, policy::response::Status::Error));
    let (code, status) = policy::response::classify_error(" E_BRANCH_STALE: space");
    expect_code(code, "E_WORKFLOW_INPUT");
    expect(matches!(status, policy::response::Status::Error));
    let (code, status) = policy::response::classify_error("E: short");
    expect_code(code, "E_WORKFLOW_INPUT");
    expect(matches!(status, policy::response::Status::Error));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn complete_code_identity_is_required_for_a_rejection() {
    proof {
        reveal_strlit("E_BRANCH_STALE"); reveal_strlit("E_RETIREMENT");
        reveal_strlit("E_REQUIRED_TEST_REMOVED"); reveal_strlit("E_INTEGRATION_REQUIRED");
        reveal_strlit("E_INTEGRATION_MISMATCH");
        reveal_strlit("E_BRANCH_STALE_SUFFIX: detail"); reveal_strlit("E_BRANCH_STALE_SUFFIX");
        reveal_strlit("E_BRANCH_STALE\0: detail"); reveal_strlit("E_BRANCH_STALE\0");
        reveal_strlit("E_雪\0:后:文"); reveal_strlit("E_雪\0");
        reveal_strlit("E_:"); reveal_strlit("E_");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("E_BRANCH_STALE_SUFFIX: detail") && vstd::string::is_ascii("E_BRANCH_STALE_SUFFIX"));
        assert(vstd::string::is_ascii("E_BRANCH_STALE\0: detail") && vstd::string::is_ascii("E_BRANCH_STALE\0"));
        assert(vstd::string::is_ascii("E_:") && vstd::string::is_ascii("E_"));
        policy::response::identifiers::ascii_separator_end_is("E_BRANCH_STALE_SUFFIX: detail".spec_bytes(), 58, 21);
        policy::response::identifiers::ascii_separator_end_is("E_BRANCH_STALE\0: detail".spec_bytes(), 58, 15);
        policy::response::identifiers::ascii_separator_end_is("E_:".spec_bytes(), 58, 2);
        assert('雪' as u32 == 38634 && '后' as u32 == 21518 && '文' as u32 == 25991);
        assert(vstd::utf8::encode_scalar(38634) == seq![233u8, 155, 170]) by (compute);
        assert(vstd::utf8::encode_scalar(21518) == seq![229u8, 144, 142]) by (compute);
        assert(vstd::utf8::encode_scalar(25991) == seq![230u8, 150, 135]) by (compute);
        assert(vstd::utf8::encode_scalar(69) == seq![69u8]) by (compute);
        assert(vstd::utf8::encode_scalar(95) == seq![95u8]) by (compute);
        assert(vstd::utf8::encode_scalar(0) == seq![0u8]) by (compute);
        assert(vstd::utf8::encode_scalar(58) == seq![58u8]) by (compute);
        reveal_with_fuel(vstd::utf8::encode_utf8, 9);
        assert("E_雪\0:后:文".spec_bytes() =~= seq![69u8, 95, 233, 155, 170, 0, 58, 229, 144, 142, 58, 230, 150, 135]);
        assert("E_雪\0".spec_bytes() =~= seq![69u8, 95, 233, 155, 170, 0]);
        policy::response::identifiers::ascii_separator_end_is("E_雪\0:后:文".spec_bytes(), 58, 6);
    }
    let (code, status) = policy::response::classify_error("E_BRANCH_STALE_SUFFIX: detail");
    expect_code(code, "E_BRANCH_STALE_SUFFIX");
    expect(matches!(status, policy::response::Status::Error));
    let (code, status) = policy::response::classify_error("E_BRANCH_STALE\0: detail");
    expect_code(code, "E_BRANCH_STALE\0");
    expect(matches!(status, policy::response::Status::Error));
    let (code, status) = policy::response::classify_error("E_雪\0:后:文");
    expect_code(code, "E_雪\0");
    expect(matches!(status, policy::response::Status::Error));
    let (code, status) = policy::response::classify_error("E_:");
    expect_code(code, "E_");
    expect(matches!(status, policy::response::Status::Error));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn collected_execution_requires_every_fact() {
    let admitted = policy::admit_collected(true, true, true, true, true, true, true, true, true);
    assert(admitted);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn each_collected_execution_fact_is_required() {
    let schema = policy::admit_collected(false, true, true, true, true, true, true, true, true);
    let status = policy::admit_collected(true, false, true, true, true, true, true, true, true);
    let scope = policy::admit_collected(true, true, false, true, true, true, true, true, true);
    let selected = policy::admit_collected(true, true, true, false, true, true, true, true, true);
    let executed = policy::admit_collected(true, true, true, true, false, true, true, true, true);
    let blocked = policy::admit_collected(true, true, true, true, true, false, true, true, true);
    let plan = policy::admit_collected(true, true, true, true, true, true, false, true, true);
    let result_selected = policy::admit_collected(true, true, true, true, true, true, true, false, true);
    let tests = policy::admit_collected(true, true, true, true, true, true, true, true, false);
    assert(!schema && !status && !scope && !selected && !executed);
    assert(!blocked && !plan && !result_selected && !tests);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn ordinary_execution_requires_every_fact() {
    let admitted = policy::admit_execution(true, true, true, true, true, true);
    let plan = policy::admit_execution(false, true, true, true, true, true);
    let scope = policy::admit_execution(true, false, true, true, true, true);
    let selected = policy::admit_execution(true, true, false, true, true, true);
    let tests = policy::admit_execution(true, true, true, false, true, true);
    let passed = policy::admit_execution(true, true, true, true, false, true);
    let count = policy::admit_execution(true, true, true, true, true, false);
    assert(admitted && !plan && !scope && !selected && !tests && !passed && !count);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn all_false_execution_facts_are_rejected() {
    let collected = policy::admit_collected(false, false, false, false, false, false, false, false, false);
    let ordinary = policy::admit_execution(false, false, false, false, false, false);
    assert(!collected && !ordinary);
}

}
