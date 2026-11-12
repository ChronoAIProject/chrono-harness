//! 当前缓存格式与 planner 身份的独立成功、拒绝与完整文本边界预期。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../cache/src/verified_planner.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_cache::verified_planner as policy;
use policy::content_preflight as content;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

#[cfg(verus_keep_ghost)]
#[path = "continuity.rs"]
mod continuity;

verus! {

fn artifact_trace(mut state: Vec<u8>, event: u8) -> (out: Vec<u8>)
    ensures out@ == state@.push(event),
{
    state.push(event);
    state
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn artifact_error_renderer_receives_every_original_field() {
    let prefix = |parts: [&str; 4]| -> (out: String) ensures out@ == parts[0]@,
        { parts[0].to_owned() };
    let path = |parts: [&str; 4]| -> (out: String) ensures out@ == parts[1]@,
        { parts[1].to_owned() };
    let separator = |parts: [&str; 4]| -> (out: String) ensures out@ == parts[2]@,
        { parts[2].to_owned() };
    let detail = |parts: [&str; 4]| -> (out: String) ensures out@ == parts[3]@,
        { parts[3].to_owned() };
    expect_restore_text(content::content_error("缓存/λ\0\n", "原错\0\n", &prefix).as_str(), "E_CACHE_CONTENT: ");
    expect_restore_text(content::content_error("缓存/λ\0\n", "原错\0\n", &path).as_str(), "缓存/λ\0\n");
    expect_restore_text(content::content_error("缓存/λ\0\n", "原错\0\n", &separator).as_str(), ": ");
    expect_restore_text(content::content_error("缓存/λ\0\n", "原错\0\n", &detail).as_str(), "原错\0\n");
}

fn artifact_observation_case(raw: Result<bool, (u32, &str)>, expected: Result<bool, &str>)
    requires vstd::laws_cmp::obeys_cmp::<&str>(),
        match raw {
            Ok(true) => expected == Ok::<bool, &str>(true),
            Ok(false) => expected == Err::<bool, &str>("cache artifact must be a physical directory"),
            Err((tag, detail)) => if tag == 404 { expected == Ok::<bool, &str>(false) }
                else { expected == Err::<bool, &str>(detail) },
        },
{
    let resolve = |state: Vec<u8>, _path: &str| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == state@.push(1), out.1 == Ok::<(), String>(()),
        { (artifact_trace(state, 1), Ok(())) };
    let observe = |state: Vec<u8>, _target: &()| -> (out: (Vec<u8>, Result<bool, (u32, &str)>))
        ensures out.0@ == state@.push(2), out.1 == raw,
        { (artifact_trace(state, 2), raw) };
    let error_text = |text: &str| -> (out: String) ensures out@ == text@,
        { text.to_owned() };
    let render = |parts: [&str; 4]| -> (out: String) ensures out@ == parts[3]@,
        { parts[3].to_owned() };
    let io = content::ArtifactIo { resolve, observe, error_text, render };
    let rows = [("缓存\0\n", Some("路径/λ\0\n"))];
    let initial = Vec::<u8>::new();
    let ghost initial_state = initial;
    let out = content::preflight(initial, &rows, 404, &io);
    let _ = (out, expected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn artifact_raw_tags_keep_directories_absence_wrong_type_and_original_errors_distinct()
    requires vstd::laws_cmp::obeys_cmp::<&str>(),
{
    artifact_observation_case(Ok(true), Ok(true));
    artifact_observation_case(Ok(false), Err("cache artifact must be a physical directory"));
    artifact_observation_case(Err((404, "ignored missing detail")), Ok(false));
    artifact_observation_case(Err((0, "")), Err(""));
    artifact_observation_case(Err((u32::MAX, "原错\0\n")), Err("原错\0\n"));
}

fn artifact_early_error_case(no_artifacts: bool, no_path: bool)
    requires vstd::laws_cmp::obeys_cmp::<&str>(),
{
    let resolve = |state: Vec<u8>, _path: &str| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == state@.push(1), match out.1 {
            Err(error) => error@ == "原路径错误\0\n"@, _ => false },
        { (artifact_trace(state, 1), Err("原路径错误\0\n".to_owned())) };
    let observe = |state: Vec<u8>, _target: &()| -> (out: (Vec<u8>, Result<bool, (u32, &str)>))
        ensures out.0@ == state@.push(2), out.1 == Ok::<bool, (u32, &str)>(true),
        { (artifact_trace(state, 2), Ok(true)) };
    let error_text = |text: &str| -> (out: String) ensures out@ == text@,
        { text.to_owned() };
    let render = |parts: [&str; 4]| -> (out: String) ensures out@ == parts[3]@,
        { parts[3].to_owned() };
    let io = content::ArtifactIo { resolve, observe, error_text, render };
    let rows = [("a", if no_path { None } else { Some("root") })];
    let unavailable = |state: Vec<u8>, _absent: content::MissingArtifacts| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == state@.push(3), out.1 == Ok::<(), String>(()),
        { (artifact_trace(state, 3), Ok(())) };
    let available = |state: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == state@.push(4), out.1 == Ok::<(), String>(()),
        { (artifact_trace(state, 4), Ok(())) };
    let initial = Vec::<u8>::new();
    let ghost initial_state = initial;
    let out = content::with_artifacts(initial, if no_artifacts { None } else { Some(&rows) },
        404, &io, unavailable, available);
    let _ = (out, no_artifacts, no_path);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn artifact_input_and_path_failures_do_not_observe_or_publish()
    requires vstd::laws_cmp::obeys_cmp::<&str>(),
{
    artifact_early_error_case(true, false);
    artifact_early_error_case(true, true);
    artifact_early_error_case(false, true);
    artifact_early_error_case(false, false);
}

fn expect_restore_text(actual: &str, expected: &str)
    requires actual@ == expected@,
{
    expect_cache_true(actual.to_owned() == expected.to_owned());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn restore_outcome_precedes_any_key_or_prefix() {
    proof {
        reveal_strlit("success"); reveal_strlit("failure"); reveal_strlit("cancelled");
        reveal_strlit("skipped"); reveal_strlit("Success"); reveal_strlit("success\n");
        reveal_strlit("");
        assert("success"@[0] != "failure"@[0]);
        assert("success"@.len() != "cancelled"@.len());
        assert("success"@[1] != "skipped"@[1]);
        assert("failure"@.len() != "cancelled"@.len());
        assert("failure"@[0] != "skipped"@[0]);
        assert("cancelled"@.len() != "skipped"@.len());
        assert("Success"@[0] != "success"@[0]);
        assert("Success"@[0] != "failure"@[0]);
        assert("Success"@.len() != "cancelled"@.len());
        assert("Success"@[0] != "skipped"@[0]);
        assert("success\n"@.len() != "success"@.len());
        assert("success\n"@.len() != "failure"@.len());
        assert("success\n"@.len() != "cancelled"@.len());
        assert("success\n"@.len() != "skipped"@.len());
    }
    let prefixes = [Some("key")];
    expect_restore_text(policy::restore_status(Some("failure"), Some("key"), Some("key"), &prefixes), "error");
    expect_restore_text(policy::restore_status(Some("cancelled"), Some("key"), Some("key"), &prefixes), "error");
    expect_restore_text(policy::restore_status(Some("skipped"), Some("key"), Some("key"), &prefixes), "not-attempted");
    expect_restore_text(policy::restore_status(None, Some("key"), Some("key"), &prefixes), "unavailable");
    expect_restore_text(policy::restore_status(Some("Success"), Some("key"), Some("key"), &prefixes), "unavailable");
    expect_restore_text(policy::restore_status(Some("success\n"), Some("key"), Some("key"), &prefixes), "unavailable");
    expect_restore_text(policy::restore_status(Some(""), Some("key"), Some("key"), &prefixes), "unavailable");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn successful_restore_needs_a_nonempty_observed_key_and_exact_match_wins() {
    proof {
        reveal_strlit(""); reveal_strlit("key");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("key") && vstd::string::is_ascii(""));
        assert("key".spec_bytes().len() == 3);
        assert("".spec_bytes().len() == 0);
    }
    expect_restore_text(policy::restore_status(Some("success"), Some("key"), None, &[]), "miss-or-unavailable");
    expect_restore_text(policy::restore_status(Some("success"), Some(""), Some(""), &[Some("")]), "miss-or-unavailable");
    expect_restore_text(policy::restore_status(Some("success"), Some("key"), Some("key"), &[]), "exact");
    expect_restore_text(policy::restore_status(Some("success"), Some("key"), Some("key"), &[Some("key")]), "exact");
    expect_restore_text(policy::restore_status(Some("success"), None, Some("key"), &[]), "incompatible");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn every_prefix_is_examined_but_missing_empty_case_and_longer_prefixes_do_not_match() {
    proof {
        reveal_strlit(""); reveal_strlit("cache/revision"); reveal_strlit("other/");
        reveal_strlit("cache/"); reveal_strlit("Cache/");
        reveal_strlit("cache/revision-extra"); reveal_strlit("cache/\0");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("") && vstd::string::is_ascii("cache/revision")
            && vstd::string::is_ascii("other/") && vstd::string::is_ascii("cache/")
            && vstd::string::is_ascii("Cache/") && vstd::string::is_ascii("cache/revision-extra")
            && vstd::string::is_ascii("cache/\0"));
        assert("cache/revision".spec_bytes()[0] != "Cache/".spec_bytes()[0]);
        assert("cache/revision".spec_bytes()[6] != "cache/\0".spec_bytes()[6]);
        assert("cache/revision".spec_bytes()[0] != "other/".spec_bytes()[0]);
        assert("cache/revision-extra".spec_bytes().len() > "cache/revision".spec_bytes().len());
        assert(forall|i: int| 0 <= i < "cache/".spec_bytes().len()
            ==> "cache/revision".spec_bytes()[i] == "cache/".spec_bytes()[i]);
    }
    let key = Some("cache/revision");
    let prefixes = [None, Some(""), Some("other/"), Some("cache/")];
    proof { assert(policy::restore_prefix_matches("cache/revision", prefixes@[3])); }
    expect_restore_text(policy::restore_status(Some("success"), None, key, &prefixes), "compatible");
    expect_restore_text(policy::restore_status(Some("success"), None, key, &[None, Some("")]), "incompatible");
    expect_restore_text(policy::restore_status(Some("success"), None, key, &[Some("Cache/")]), "incompatible");
    expect_restore_text(policy::restore_status(Some("success"), None, key, &[Some("cache/revision-extra")]), "incompatible");
    expect_restore_text(policy::restore_status(Some("success"), None, key, &[Some("cache/\0")]), "incompatible");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn restore_identity_and_full_prefix_keep_unicode_nul_and_newline() {
    let key = "缓存/λ\0\n";
    proof {
        reveal_strlit("缓存/λ\0\n");
        vstd::utf8::encode_utf8_valid_utf8(key@);
        vstd::utf8::encode_utf8_decode_utf8(key@);
        assert(key.spec_bytes().len() > 0) by {
            if key.spec_bytes().len() == 0 {
                assert(key.spec_bytes() == Seq::<u8>::empty());
                reveal(vstd::utf8::decode_utf8);
            }
        }
        assert(forall|i: int| 0 <= i < key.spec_bytes().len()
            ==> key.spec_bytes()[i] == key.spec_bytes()[i]);
    }
    expect_restore_text(policy::restore_status(Some("success"), Some(key), Some(key), &[]), "exact");
    expect_restore_text(policy::restore_status(Some("success"), None, Some(key), &[Some(key)]), "compatible");
}

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

fn recovery_action_tag(
    outcome: Option<&str>, reason: &str, unconfirmed: bool,
    outputs_is_null_or_object: bool, matched_key_is_null_or_string: bool,
    content_method: Option<&str>, content_status: Option<&str>, refresh_enabled: bool,
    expected: u8,
)
    requires match policy::recovery_action_spec(
        outcome, reason, unconfirmed, outputs_is_null_or_object,
        matched_key_is_null_or_string, content_method, content_status, refresh_enabled,
    ) {
        Ok(policy::RecoveryAction::Keep) => expected == 0,
        Ok(policy::RecoveryAction::Discard) => expected == 1,
        Ok(policy::RecoveryAction::Refresh) => expected == 2,
        Err(policy::RecoveryDecisionError::UnknownOutcome) => expected == 3,
        Err(policy::RecoveryDecisionError::MalformedOutputs) => expected == 4,
    },
{
    let actual = policy::recovery_action(
        outcome, reason, unconfirmed, outputs_is_null_or_object,
        matched_key_is_null_or_string, content_method, content_status, refresh_enabled,
    );
    let actual_tag = match actual {
        Ok(policy::RecoveryAction::Keep) => 0,
        Ok(policy::RecoveryAction::Discard) => 1,
        Ok(policy::RecoveryAction::Refresh) => 2,
        Err(policy::RecoveryDecisionError::UnknownOutcome) => 3,
        Err(policy::RecoveryDecisionError::MalformedOutputs) => 4,
    };
    assert(actual_tag == expected);
    expect_cache_true(actual_tag == expected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_decision_covers_outcomes_content_and_refresh_boundaries() {
    proof {
        reveal_strlit("failure"); reveal_strlit("success"); reveal_strlit("skipped");
        reveal_strlit("miss-or-unavailable"); reveal_strlit("incompatible");
        reveal_strlit("exact"); reveal_strlit("compatible");
        reveal_strlit("producer-manifest/v1"); reveal_strlit("matched");
        reveal_strlit("unusable"); reveal_strlit("unknown"); reveal_strlit("");
        assert("failure"@[0] != "success"@[0]);
        assert("success"@[1] != "skipped"@[1]);
        assert("exact"@.len() != "compatible"@.len());
        assert("matched"@[0] != "unusable"@[0]);
        assert("producer-manifest/v1"@.len() != "matched"@.len());
        assert("unknown"@[0] != "success"@[0]);
    }
    // A failed provider always discards, regardless of optional content metadata.
    recovery_action_tag(
        Some("failure"), "error", false, true, true,
        Some("producer-manifest/v1"), Some("matched"), true, 1,
    );
    // An opted-in unconfirmed restore rejects malformed output shape first.
    recovery_action_tag(
        Some("success"), "exact", true, false, true,
        None, None, false, 4,
    );
    // Misses and incompatible keys discard before content verification.
    recovery_action_tag(
        Some("success"), "miss-or-unavailable", true, true, true,
        None, None, false, 1,
    );
    recovery_action_tag(
        Some("success"), "incompatible", true, true, true,
        None, None, false, 1,
    );
    // Exact content verification keeps a matched producer manifest and discards an unusable one.
    recovery_action_tag(
        Some("success"), "exact", false, true, true,
        Some("producer-manifest/v1"), Some("matched"), false, 0,
    );
    recovery_action_tag(
        Some("success"), "exact", false, true, true,
        Some("producer-manifest/v1"), Some("unusable"), false, 1,
    );
    // Compatible restores refresh only when the policy enables source refresh.
    recovery_action_tag(
        Some("success"), "compatible", false, true, true,
        None, None, false, 0,
    );
    recovery_action_tag(
        Some("success"), "compatible", false, true, true,
        None, None, true, 2,
    );
    // Skipped is retained; missing and unknown outcomes are rejected.
    recovery_action_tag(
        Some("skipped"), "not-attempted", false, true, true,
        None, None, false, 0,
    );
    recovery_action_tag(
        None, "unavailable", false, true, true,
        None, None, false, 3,
    );
    recovery_action_tag(
        Some("unknown"), "unavailable", false, true, true,
        None, None, false, 3,
    );
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
