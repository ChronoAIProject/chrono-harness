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
    let (code, status) = policy::response::classify_error("E_BRANCH_STALE: 过期");
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
    let (code, status) = policy::response::classify_error("E_INTEGRATION_MISMATCH: 不符");
    expect_code(code, "E_INTEGRATION_MISMATCH");
    expect(matches!(status, policy::response::Status::Fail));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn invalid_prefixes_preserve_the_workflow_fallback() {
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
