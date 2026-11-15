//! 独立边界预期直接消费生产集合判断，Cargo 与 Verus 共用源码。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../judge-ci/src/verified_collection.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as source_identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_verifier as source_verifier;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(not(verus_keep_ghost))]
use chrono_judge_ci::verified_collection as policy;
use policy::result_types::{CheckResult, Status};
#[cfg(verus_keep_ghost)]
use policy::source_verifier;
#[cfg(verus_keep_ghost)]
use policy::source_verifier::identifiers as source_identifiers;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn registered_action_references_require_the_real_tool_with_no_profile_fallback() {
    let missing = policy::action_tool_allowed(true, false, false);
    let stale_copy = policy::action_tool_allowed(true, false, true);
    let registered_only = policy::action_tool_allowed(true, true, false);
    let both = policy::action_tool_allowed(true, true, true);
    assert(!missing && !stale_copy && registered_only && both);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn standalone_action_references_use_the_explicit_profile_tools() {
    let missing = policy::action_tool_allowed(false, false, false);
    let unrelated_registration = policy::action_tool_allowed(false, true, false);
    let profile_only = policy::action_tool_allowed(false, false, true);
    let both = policy::action_tool_allowed(false, true, true);
    assert(!missing && !unrelated_registration && profile_only && both);
}

fn expect_operation(
    state: &str, error: &Option<String>, exit: Option<i32>,
    expected_state: Status, expected_exit: Option<i32>,
)
    requires expected_state == policy::operation_status(state@),
        expected_exit == policy::operation_exit(state@, exit),
{
    let row = policy::operation_result("é\0operation", state, error, exit);
    assert(row.id@ == "é\0operation"@);
    assert(row.status == expected_state);
    assert(row.exit_code == expected_exit);
    assert(row.cause@ == policy::operation_cause(error));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn operation_state_and_exit_boundaries_use_the_original_fields() {
    proof {
        reveal_strlit("passed"); reveal_strlit("blocked"); reveal_strlit("failed");
        reveal_strlit("error"); reveal_strlit("Passed"); reveal_strlit("passed\0");
        assert("passed"@[0] != "blocked"@[0]);
        assert("passed"@[0] != "failed"@[0]);
        assert("passed"@[0] != "error"@[0]);
        assert("blocked"@.len() != "failed"@.len());
        assert("blocked"@.len() != "error"@.len());
        assert("passed"@[0] != "Passed"@[0]);
        assert("blocked"@.len() != "Passed"@.len());
        assert("passed"@.len() != "passed\0"@.len());
        assert("blocked"@[0] != "passed\0"@[0]);
    }
    let error = Some("original error\0é".to_string());
    let no_error = None;
    expect_operation("passed", &no_error, Some(0), Status::Passed, Some(0));
    expect_operation("passed", &error, Some(-7), Status::Passed, Some(-7));
    expect_operation("passed", &no_error, None, Status::Passed, None);
    expect_operation("blocked", &error, Some(0), Status::Blocked, None);
    expect_operation("blocked", &no_error, Some(7), Status::Blocked, Some(7));
    expect_operation("blocked", &no_error, None, Status::Blocked, None);
    expect_operation("failed", &error, Some(0), Status::Failed, None);
    expect_operation("failed", &error, Some(i32::MIN), Status::Failed, Some(i32::MIN));
    expect_operation("error", &error, Some(i32::MAX), Status::Failed, Some(i32::MAX));
    expect_operation("Passed", &no_error, Some(0), Status::Failed, None);
    expect_operation("passed\0", &no_error, Some(1), Status::Failed, Some(1));
}

/// 任意未知完整名称都失败；错误文本和退出码不反向改写原执行状态。
fn arbitrary_operation_states_are_not_inferred_from_error_or_exit(state: &str, exit: i32)
    requires state@ != "passed"@, state@ != "blocked"@,
{
    let row = policy::operation_result("operation", state, &None, Some(exit));
    assert(row.status == Status::Failed);
    assert(row.exit_code == if exit == 0 { None } else { Some(exit) });
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn operation_cause_preserves_empty_unicode_nul_and_default() {
    let original = Some("é\0original error".to_string());
    let row = policy::operation_result("operation", "failed", &original, Some(0));
    assert(row.cause@ == "é\0original error"@);
    let empty = Some("".to_string());
    let empty_cause = policy::operation_result("operation", "passed", &empty, Some(0));
    assert(empty_cause.cause@ == ""@);
    let default_cause = policy::operation_result("operation", "passed", &None, Some(0));
    assert(default_cause.cause@ == "registered command completed with matching receipt"@);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn source_binding_failure_cannot_reuse_success_or_fabricate_process_exit() {
    proof { reveal_strlit("failed"); reveal_strlit("passed");
        assert("failed"@[0] != "passed"@[0]); }
    let zero = policy::operation_result_with_sources("proof\0é", "passed", &None, Some(0),
        Err::<(), String>("original source mismatch\0é".to_string()));
    assert(zero.id@ == "proof\0é"@ && zero.status == Status::Failed);
    assert(zero.cause@ == "original source mismatch\0é"@ && zero.exit_code.is_none());
    let nonzero = policy::operation_result_with_sources("proof", "passed", &None, Some(-7),
        Err::<(), String>("".to_string()));
    assert(nonzero.status == Status::Failed && nonzero.exit_code == Some(-7i32));
    assert(nonzero.cause@ == ""@);
    let passed = policy::operation_result_with_sources("proof", "passed", &None, Some(0), Ok(()));
    assert(passed.status == Status::Passed && passed.exit_code == Some(0i32));
    let blocked = policy::operation_result_with_sources("proof", "blocked",
        &Some("original block".to_string()), Some(0), Err::<(), String>("later binding".to_string()));
    proof {
        reveal_strlit("blocked"); reveal_strlit("passed");
        assert("blocked"@[0] != "passed"@[0]);
    }
    assert(blocked.status == Status::Blocked && blocked.exit_code.is_none());
    assert(blocked.cause@ == "original block"@);
    let rows = vec![passed, zero];
    let terminal = policy::response_status(&rows);
    assert(terminal == Status::Failed);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn terminal_status_uses_all_original_result_states() {
    let empty: Vec<CheckResult> = Vec::new();
    let empty_status = policy::response_status(&empty);
    assert(empty_status == Status::Passed);
    let success = vec![
        result("a", Status::NotRequired, Some(-7)),
        result("a", Status::Passed, None),
        result("failed", Status::Passed, Some(1)),
    ];
    let passed = policy::response_status(&success);
    assert(passed == Status::Passed);
    let failed_first = vec![
        result("a", Status::Failed, None), result("a", Status::Passed, Some(0)),
    ];
    let failed = policy::response_status(&failed_first);
    assert(failed == Status::Failed);
    let blocked_last = vec![
        result("a", Status::Passed, Some(0)), result("a", Status::Blocked, None),
    ];
    let blocked = policy::response_status(&blocked_last);
    assert(blocked == Status::Failed);
    let duplicate_failures = vec![
        result("é\0", Status::NotRequired, None),
        result("é\0", Status::Failed, Some(0)),
        result("é\0", Status::Blocked, Some(0)),
    ];
    let duplicates = policy::response_status(&duplicate_failures);
    assert(duplicates == Status::Failed);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn diagnostic_failure_appends_the_real_record_without_replacing_prior_results() {
    let mut rows = vec![result("é\0original", Status::Passed, Some(0))];
    let status = policy::diagnostic_failure(&mut rows);
    assert(status == Status::Failed);
    assert(rows.len() == 2);
    assert(rows[0].id@ == "é\0original"@);
    assert(rows[0].status == Status::Passed);
    assert(rows[0].cause@ == "original report"@);
    assert(rows[0].exit_code == Some(0i32));
    assert(rows[1].id@ == "ci.execution-diagnostics"@);
    assert(rows[1].status == Status::Failed);
    assert(rows[1].exit_code.is_none());
    assert(rows[1].cause@ == "execution diagnostic publication failed; original errors retained"@);
    let mut empty: Vec<CheckResult> = Vec::new();
    let from_empty = policy::diagnostic_failure(&mut empty);
    assert(from_empty == Status::Failed && empty.len() == 1);
    let again = policy::diagnostic_failure(&mut empty);
    assert(again == Status::Failed && empty.len() == 2);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn inventory_failure_produces_its_own_failed_terminal_state_and_original_cause() {
    let (status, rows) = policy::inventory_failure("é\0original inventory error".to_string());
    assert(status == Status::Failed);
    assert(rows.len() == 1);
    assert(rows[0].id@ == "ci.inventory"@);
    assert(rows[0].status == Status::Failed);
    assert(rows[0].cause@ == "é\0original inventory error"@);
    assert(rows[0].exit_code.is_none());
}

fn result(id: &str, status: Status, exit_code: Option<i32>) -> (out: CheckResult)
    ensures out.id@ == id@, out.status == status, out.exit_code == exit_code,
        out.cause@ == "original report"@,
{
    CheckResult {
        id: id.to_string(), status, cause: "original report".to_string(), exit_code,
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn operation_success_requires_all_fields_on_one_exact_result() {
    proof {
        reveal_strlit("build"); reveal_strlit("other");
        assert("build"@[0] != "other"@[0]);
    }
    let values = vec![result("build", Status::Passed, Some(0))];
    let accepted = policy::has_result(&values, "build", &Status::Passed, Some(0));
    assert(accepted);
    let wrong_id = policy::has_result(&values, "other", &Status::Passed, Some(0));
    assert(!wrong_id);
    let wrong_status = policy::has_result(&values, "build", &Status::Failed, Some(0));
    assert(!wrong_status);
    let wrong_exit = policy::has_result(&values, "build", &Status::Passed, Some(7));
    assert(!wrong_exit);
    let absent_exit = policy::has_result(&values, "build", &Status::Passed, None);
    assert(!absent_exit);
    let empty = Vec::new();
    let missing = policy::has_result(&empty, "build", &Status::Passed, Some(0));
    assert(!missing);
    let split = vec![
        result("build", Status::Failed, Some(0)),
        result("other", Status::Passed, Some(0)),
        result("build", Status::Passed, Some(7)),
        result("build", Status::NotRequired, Some(0)),
        result("build", Status::Blocked, Some(0)),
    ];
    let cannot_combine = policy::has_result(&split, "build", &Status::Passed, Some(0));
    assert(!cannot_combine);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn inventory_and_unrequired_states_preserve_their_distinct_exit_contracts() {
    proof {
        reveal_strlit("ci.inventory"); reveal_strlit("test");
        assert("ci.inventory"@.len() != "test"@.len());
    }
    let unrequired = vec!["test"];
    let valid = vec![
        result("test", Status::NotRequired, Some(7)),
        result("ci.inventory", Status::Passed, None),
    ];
    let accepted = policy::matches_result_states(&valid, &unrequired);
    assert(accepted);
    let no_inventory = vec![result("test", Status::NotRequired, None)];
    let missing = policy::matches_result_states(&no_inventory, &unrequired);
    assert(!missing);
    let exited_inventory = vec![result("ci.inventory", Status::Passed, Some(0))];
    let forbidden_exit = policy::matches_result_states(&exited_inventory, &unrequired);
    assert(!forbidden_exit);
    let failed_inventory = vec![result("ci.inventory", Status::Failed, None)];
    let failed = policy::matches_result_states(&failed_inventory, &unrequired);
    assert(!failed);
    let wrong_unrequired = vec![
        result("ci.inventory", Status::Passed, None),
        result("test", Status::Passed, None),
    ];
    let wrong_status = policy::matches_result_states(&wrong_unrequired, &unrequired);
    assert(!wrong_status);
    let empty = Vec::new();
    let absent = policy::matches_result_states(&empty, &unrequired);
    assert(!absent);
    // ID 的完整覆盖由独立集合合同承担；状态合同不补造缺少的结果。
    let marker_only = vec![result("ci.inventory", Status::Passed, None)];
    let states_only = policy::matches_result_states(&marker_only, &unrequired);
    assert(states_only);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn cloning_the_actual_report_result_preserves_every_field() {
    let original = result("build", Status::Blocked, Some(-7));
    let cloned = original.clone();
    assert(cloned.id@ == "build"@);
    assert(cloned.status == Status::Blocked);
    assert(cloned.exit_code == Some(-7i32));
    assert(cloned.cause@ == "original report"@);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn borrowed_registered_ids_preserve_order_duplicates_unicode_nul_and_empty() {
    let values = vec!["é".to_string(), "a\0b".to_string(), "é".to_string()];
    let texts = policy::borrow_texts(&values);
    assert(texts.len() == 3);
    assert(texts[0]@ == "é"@);
    assert(texts[1]@ == "a\0b"@);
    assert(texts[2]@ == "é"@);
    let empty = Vec::new();
    let no_texts = policy::borrow_texts(&empty);
    assert(no_texts.len() == 0);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn actual_report_id_projection_preserves_order_duplicates_unicode_and_nul() {
    let values = vec![
        result("é", Status::Blocked, Some(-7)),
        result("a\0b", Status::Passed, None),
        result("é", Status::NotRequired, Some(0)),
    ];
    let ids = policy::result_ids(&values);
    assert(ids.len() == 3);
    assert(ids[0]@ == "é"@);
    assert(ids[1]@ == "a\0b"@);
    assert(ids[2]@ == "é"@);
    let empty = Vec::new();
    let no_ids = policy::result_ids(&empty);
    assert(no_ids.len() == 0);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn actual_report_coverage_uses_the_original_record_ids() {
    proof {
        reveal_strlit("ci.inventory"); reveal_strlit("build"); reveal_strlit("test");
        assert("ci.inventory"@.len() != "build"@.len());
        assert("ci.inventory"@.len() != "test"@.len());
        assert("build"@.len() != "test"@.len());
    }
    let operation_names = vec!["build".to_string()];
    let unrequired_names = vec!["test".to_string()];
    let operations = policy::borrow_texts(&operation_names);
    let unrequired = policy::borrow_texts(&unrequired_names);
    assert(policy::text_values(operations@) =~= seq!["build"@]);
    assert(policy::text_values(unrequired@) =~= seq!["test"@]);
    assert(policy::expected_result_ids(operations@, unrequired@)
        =~= seq!["ci.inventory"@, "build"@, "test"@]);
    let values = vec![
        result("test", Status::NotRequired, None),
        result("ci.inventory", Status::Passed, None),
        result("build", Status::Passed, Some(0)),
    ];
    let accepted = policy::matches_report_result_ids(&values, &operations, &unrequired);
    assert(accepted);
    let missing_operation = vec![
        result("test", Status::NotRequired, None),
        result("ci.inventory", Status::Passed, None),
    ];
    let missing = policy::matches_report_result_ids(&missing_operation, &operations, &unrequired);
    assert(!missing);
    let undeclared = policy::matches_report_result_ids(&values, &operations, &Vec::new());
    assert(!undeclared);
    let empty = Vec::new();
    let absent = policy::matches_report_result_ids(&empty, &operations, &unrequired);
    assert(!absent);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn result_membership_preserves_permutations_and_rejects_missing_or_extra_ids() {
    proof {
        reveal_strlit("ci.inventory"); reveal_strlit("build"); reveal_strlit("test");
        reveal_strlit("other");
        assert("ci.inventory"@.len() != "build"@.len());
        assert("ci.inventory"@.len() != "test"@.len());
        assert("build"@[0] != "other"@[0]);
        assert("build"@.len() != "test"@.len());
        assert("test"@.len() != "other"@.len());
        assert("ci.inventory"@.len() != "other"@.len());
    }
    let expected = vec!["ci.inventory", "build", "test"];
    let reordered = vec!["test", "ci.inventory", "build"];
    let accepted = policy::matches_text_membership(&reordered, &expected);
    assert(accepted);
    let missing = vec!["ci.inventory", "build"];
    let absent = policy::matches_text_membership(&missing, &expected);
    assert(!absent);
    let extra = vec!["ci.inventory", "build", "test", "other"];
    let unexpected = policy::matches_text_membership(&extra, &expected);
    assert(!unexpected);
    let replaced = vec!["ci.inventory", "build", "other"];
    let changed = policy::matches_text_membership(&replaced, &expected);
    assert(!changed);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn result_membership_keeps_empty_and_duplicate_set_semantics() {
    proof { reveal_strlit("test"); }
    let empty: Vec<&str> = Vec::new();
    let same_empty = policy::matches_text_membership(&empty, &empty);
    assert(same_empty);
    let one = vec!["test"];
    let missing = policy::matches_text_membership(&empty, &one);
    assert(!missing);
    let extra = policy::matches_text_membership(&one, &empty);
    assert(!extra);
    let duplicates = vec!["test", "test"];
    let same_set = policy::matches_text_membership(&duplicates, &one);
    assert(same_set);
    let reverse = policy::matches_text_membership(&one, &duplicates);
    assert(reverse);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn result_membership_preserves_exact_unicode_and_nul_identity() {
    proof {
        reveal_strlit("é"); reveal_strlit("é"); reveal_strlit("a\0b");
        reveal_strlit("a");
        assert("é"@.len() != "é"@.len());
        assert("a\0b"@.len() != "a"@.len());
    }
    let composed = vec!["é"];
    let decomposed = vec!["é"];
    let unicode_changed = policy::matches_text_membership(&composed, &decomposed);
    assert(!unicode_changed);
    let nul = vec!["a\0b"];
    let prefix = vec!["a"];
    let truncated = policy::matches_text_membership(&nul, &prefix);
    assert(!truncated);
    let exact = policy::matches_text_membership(&nul, &nul);
    assert(exact);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registered_inventory_requires_the_marker_and_both_declared_lists() {
    proof {
        reveal_strlit("ci.inventory"); reveal_strlit("build"); reveal_strlit("test:other");
        assert("ci.inventory"@.len() != "build"@.len());
        assert("ci.inventory"@.len() != "test:other"@.len());
        assert("build"@.len() != "test:other"@.len());
    }
    let operations = vec!["build"];
    let not_required = vec!["test:other"];
    let reordered = vec!["test:other", "ci.inventory", "build"];
    assert(policy::text_values(operations@) =~= seq!["build"@]);
    assert(policy::text_values(not_required@) =~= seq!["test:other"@]);
    assert(policy::expected_result_ids(operations@, not_required@)
        =~= seq!["ci.inventory"@, "build"@, "test:other"@]);
    let accepted = policy::matches_result_ids(&reordered, &operations, &not_required);
    assert(accepted);
    let no_inventory = vec!["build", "test:other"];
    let missing_marker = policy::matches_result_ids(&no_inventory, &operations, &not_required);
    assert(!missing_marker);
    let no_operation = vec!["ci.inventory", "test:other"];
    let missing_operation = policy::matches_result_ids(&no_operation, &operations, &not_required);
    assert(!missing_operation);
    let no_unrequired = vec!["ci.inventory", "build"];
    let missing_unrequired = policy::matches_result_ids(&no_unrequired, &operations, &not_required);
    assert(!missing_unrequired);
    let undeclared = policy::matches_result_ids(&reordered, &operations, &Vec::new());
    assert(!undeclared);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registered_inventory_preserves_empty_lists_and_exact_declared_names() {
    proof {
        reveal_strlit("ci.inventory"); reveal_strlit("é"); reveal_strlit("a\0b");
        assert("ci.inventory"@.len() != "é"@.len());
        assert("ci.inventory"@.len() != "a\0b"@.len());
        assert("é"@.len() != "a\0b"@.len());
    }
    let empty: Vec<&str> = Vec::new();
    let marker = vec!["ci.inventory"];
    assert(marker[0]@ == "ci.inventory"@);
    assert(policy::expected_result_ids(empty@, empty@) =~= seq!["ci.inventory"@]);
    assert(policy::expected_result_ids(empty@, empty@)[0] == "ci.inventory"@);
    let inventory_only = policy::matches_result_ids(&marker, &empty, &empty);
    assert(inventory_only);
    let no_marker = policy::matches_result_ids(&empty, &empty, &empty);
    assert(!no_marker);
    let operations = vec!["é", "é"];
    let not_required = vec!["a\0b", "é"];
    let actual = vec!["a\0b", "ci.inventory", "é"];
    assert(policy::text_values(operations@) =~= seq!["é"@, "é"@]);
    assert(policy::text_values(not_required@) =~= seq!["a\0b"@, "é"@]);
    assert(policy::expected_result_ids(operations@, not_required@)
        =~= seq!["ci.inventory"@, "é"@, "é"@, "a\0b"@, "é"@]);
    let exact_set = policy::matches_result_ids(&actual, &operations, &not_required);
    assert(exact_set);
}

}

verus! {

fn expect_continuity_bool(actual: bool)
    requires actual,
{ if !actual { panic!(); } }

#[cfg_attr(not(verus_keep_ghost), test)]
fn continuity_pipeline_preserves_both_endpoints_paths_and_observed_evidence() {
    let (trace, result) = policy::with_source_continuity(
        Vec::<u8>::new(), 11u8, 22u8, 33u8,
        |mut state: Vec<u8>, previous: u8, candidate: u8, paths: u8| -> (out: (Vec<u8>, Result<(u8,u8,u8), String>))
            ensures out.0@ == state@.push(1u8).push(previous).push(candidate).push(paths),
                out.1 == Ok::<(u8,u8,u8), String>((previous,candidate,paths)),
        {
            state.push(1); state.push(previous); state.push(candidate); state.push(paths);
            (state, Ok((previous,candidate,paths)))
        },
        |mut state: Vec<u8>, evidence: (u8,u8,u8)| -> (out: (Vec<u8>, Result<(u8,u8,u8), String>))
            ensures out.0@ == state@.push(2u8).push(evidence.0).push(evidence.1).push(evidence.2),
                out.1 == Ok::<(u8,u8,u8), String>(evidence),
        {
            state.push(2); state.push(evidence.0); state.push(evidence.1); state.push(evidence.2);
            (state, Ok(evidence))
        },
    );
    expect_continuity_bool(trace.len() == 8);
    expect_continuity_bool(trace[0] == 1 && trace[1] == 11 && trace[2] == 22 && trace[3] == 33);
    expect_continuity_bool(trace[4] == 2 && trace[5] == 11 && trace[6] == 22 && trace[7] == 33);
    let exact = match result { Ok(value) => value.0 == 11 && value.1 == 22 && value.2 == 33, Err(_) => false };
    expect_continuity_bool(exact);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn continuity_observation_failure_keeps_the_first_error_and_skips_validation() {
    let (trace, result) = policy::with_source_continuity(
        Vec::<u8>::new(), (), (), (),
        |mut state: Vec<u8>, _previous: (), _candidate: (), _paths: ()| -> (out: (Vec<u8>, Result<u8, String>))
            ensures out.0@ == state@.push(1u8),
                match out.1 { Err(error) => error@ == "原读取错误\0é"@, Ok(_) => false },
        { state.push(1); (state, Err("原读取错误\0é".to_string())) },
        |mut state: Vec<u8>, evidence: u8| -> (out: (Vec<u8>, Result<u8, String>))
            ensures out.0@ == state@.push(9u8), out.1 == Ok::<u8,String>(evidence),
        { state.push(9); (state, Ok(evidence)) },
    );
    expect_continuity_bool(trace.len() == 1 && trace[0] == 1);
    let exact = match result { Err(error) => source_identifiers::equal_text(error.as_str(), "原读取错误\0é"), Ok(_) => false };
    expect_continuity_bool(exact);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn continuity_validation_failure_keeps_its_full_original_error() {
    let (trace, result) = policy::with_source_continuity(
        Vec::<u8>::new(), (), (), 7u8,
        |mut state: Vec<u8>, _previous: (), _candidate: (), paths: u8| -> (out: (Vec<u8>, Result<u8, String>))
            ensures out.0@ == state@.push(1u8), out.1 == Ok::<u8,String>(paths),
        { state.push(1); (state, Ok(paths)) },
        |mut state: Vec<u8>, evidence: u8| -> (out: (Vec<u8>, Result<u8, String>))
            ensures out.0@ == state@.push(evidence),
                match out.1 { Err(error) => error@ == "原判定错误\0é"@, Ok(_) => false },
        { state.push(evidence); (state, Err("原判定错误\0é".to_string())) },
    );
    expect_continuity_bool(trace.len() == 2 && trace[0] == 1 && trace[1] == 7);
    let exact = match result { Err(error) => source_identifiers::equal_text(error.as_str(), "原判定错误\0é"), Ok(_) => false };
    expect_continuity_bool(exact);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn combined_source_plan_keeps_base_errors_before_affected_file_errors() {
    let registry = source_verifier::sources::source_registry(vec![source_verifier::sources::SourceFile {
        path: "x".to_string(), verification: None,
    }], Some(true));
    let nodes = vec!["file:x".to_string()];
    let empty = source_verifier::sources::continuity::Continuity::empty();
    let result = policy::validate_source_plan(&registry, Err("原基线错误\0é"),
        &nodes, &Vec::new(), &Vec::new(), &empty);
    expect_continuity_bool(result.is_err());
    let exact = match result { Err(error) => source_identifiers::equal_text(error.path, "原基线错误\0é"), Ok(_) => false };
    expect_continuity_bool(exact);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn combined_source_plan_rejects_unknown_after_a_successful_inventory() {
    let registry = source_verifier::sources::source_registry(vec![source_verifier::sources::SourceFile {
        path: "x".to_string(), verification: None,
    }], Some(true));
    let nodes = vec!["file:x".to_string()];
    let empty = source_verifier::sources::continuity::Continuity::empty();
    proof {
        reveal_with_fuel(source_verifier::sources::classifications_error, 3);
        reveal_with_fuel(source_verifier::sources::selected_plan_error, 3);
        reveal(source_verifier::sources::file_plan_error);
        reveal_strlit("file:"); reveal_strlit("x"); reveal_strlit("file:x");
        assert("file:x"@ =~= "file:"@ + "x"@);
        assert(source_verifier::sources::node_affected("file:"@, "x"@, nodes@)) by {
            assert(nodes[0]@ == "file:"@ + "x"@);
        }
    }
    let result = policy::validate_source_plan(&registry, Ok(Some(&registry)),
        &nodes, &Vec::new(), &Vec::new(), &empty);
    expect_continuity_bool(result.is_err());
    let exact = match result { Err(error) => source_identifiers::equal_text(error.message,
        "E_VERUS_COVERAGE: affected file lacks explicit verification classification: "), Ok(_) => false };
    expect_continuity_bool(exact);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn combined_source_plan_accepts_the_complete_empty_initial_inventory() {
    let registry = source_verifier::sources::source_registry(Vec::new(), Some(true));
    let empty = source_verifier::sources::continuity::Continuity::empty();
    let result = policy::validate_source_plan(&registry, Ok(None),
        &Vec::new(), &Vec::new(), &Vec::new(), &empty);
    expect_continuity_bool(result.is_ok());
}


fn reference_form(prefix: &str, base: u8) -> (out: source_verifier::sources::continuity::ReferenceSyntax)
    ensures out.file@ == "x"@, out.before@ == prefix@, out.after@ == prefix@, out.base == base,
{
    source_verifier::sources::continuity::ReferenceSyntax {
        file: "x".to_string(), before: prefix.to_string(), after: prefix.to_string(),
        trailing: String::new(), suffix_before: String::new(), suffix_after: String::new(), base,
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn reference_declarations_validate_actual_prefixes_and_explicit_bases() {
    proof {
        reveal_strlit("x"); reveal_strlit("\""); reveal_strlit("/"); reveal_strlit("");
        vstd::utf8::is_ascii_chars_encode_utf8("x"@);
        vstd::utf8::is_ascii_chars_encode_utf8("\""@);
        vstd::utf8::is_ascii_chars_encode_utf8("/"@);
        vstd::utf8::is_ascii_chars_encode_utf8(""@);
        assert(vstd::utf8::encode_utf8("x"@) =~= seq![120u8]);
        assert(vstd::utf8::encode_utf8("\""@) =~= seq![34u8]);
        assert(vstd::utf8::encode_utf8("/"@) =~= seq![47u8]);
        assert(vstd::utf8::encode_utf8(""@) =~= Seq::<u8>::empty());
        assert(source_verifier::sources::canonical_source_path(seq![120u8])) by (compute);
    }
    let quoted = policy::validate_reference_syntax(&vec![reference_form("\"", 0)]);
    let root = policy::validate_reference_syntax(&vec![reference_form("/", 2)]);
    let wrong_base = policy::validate_reference_syntax(&vec![reference_form("/", 0)]);
    let empty = policy::validate_reference_syntax(&vec![reference_form("", 0)]);
    let unknown_base = policy::validate_reference_syntax(&vec![reference_form("\"", 3)]);
    expect_continuity_bool(quoted.is_ok() && root.is_ok());
    expect_continuity_bool(wrong_base.is_err() && empty.is_err() && unknown_base.is_err());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn object_shape_rejects_nonobjects_before_inspecting_fields() {
    let result = policy::validate_object_fields(None::<u64>, &["missing"], &[], &["unexpected"]);
    match result {
        Err(policy::ObjectFieldError::ExpectedObject) => {},
        _ => { expect_continuity_bool(false); },
    }
}

fn object_shape_keeps_the_exact_successful_value(value: u64) {
    let required = ["blob", "sha256"];
    let optional = ["length", "blob"];
    let keys = ["sha256", "length", "blob"];
    let result = policy::validate_object_fields(Some(value), &required, &optional, &keys);
    match result {
        Ok(actual) => expect_continuity_bool(actual == value),
        Err(_) => { expect_continuity_bool(false); },
    }
    let empty = policy::validate_object_fields(Some(value), &[], &[], &[]);
    match empty {
        Ok(actual) => expect_continuity_bool(actual == value),
        Err(_) => { expect_continuity_bool(false); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn object_shape_accepts_only_complete_known_fields_and_preserves_values() {
    object_shape_keeps_the_exact_successful_value(0);
    object_shape_keeps_the_exact_successful_value(u64::MAX);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn object_shape_reports_the_first_required_field_without_any_keys() {
    let required = ["first", "second"];
    let keys: [&str; 0] = [];
    proof { assert(required@[0]@ == "first"@); }
    let result = policy::validate_object_fields(Some(7u64), &required, &[], &keys);
    match result {
        Err(policy::ObjectFieldError::Missing(name)) => {
            let first = source_identifiers::equal_text(name, "first");
            expect_continuity_bool(first);
        },
        _ => { expect_continuity_bool(false); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn object_shape_missing_required_precedes_an_unknown_key() {
    proof {
        reveal_strlit("required"); reveal_strlit("unknown");
        assert("required"@[0] != "unknown"@[0]);
    }
    let result = policy::validate_object_fields(Some(1u64), &["required"], &[], &["unknown"]);
    match result {
        Err(policy::ObjectFieldError::Missing(name)) => {
            let same = source_identifiers::equal_text(name, "required");
            expect_continuity_bool(same);
        },
        _ => { expect_continuity_bool(false); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn object_shape_reports_the_first_unknown_key_after_all_required_fields() {
    proof {
        reveal_strlit("known"); reveal_strlit("first"); reveal_strlit("second");
        assert("known"@[0] != "first"@[0]);
        assert("known"@[0] != "second"@[0]);
    }
    let result = policy::validate_object_fields(
        Some(9u64), &["known"], &[], &["known", "first", "second"],
    );
    match result {
        Err(policy::ObjectFieldError::Unknown(name)) => {
            let same = source_identifiers::equal_text(name, "first");
            expect_continuity_bool(same);
        },
        _ => { expect_continuity_bool(false); },
    }
}

fn object_shape_errors_preserve_the_complete_field_name(name: &str) {
    let missing = policy::object_error_parts(policy::ObjectFieldError::Missing(name));
    let prefix = source_identifiers::equal_text(missing[0], "missing field ");
    let field = source_identifiers::equal_text(missing[1], name);
    expect_continuity_bool(prefix && field);
    let unknown = policy::object_error_parts(policy::ObjectFieldError::Unknown(name));
    let prefix = source_identifiers::equal_text(unknown[0], "unknown field ");
    let field = source_identifiers::equal_text(unknown[1], name);
    expect_continuity_bool(prefix && field);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn object_shape_error_messages_keep_empty_unicode_nul_and_newline_names() {
    object_shape_errors_preserve_the_complete_field_name("");
    object_shape_errors_preserve_the_complete_field_name("字段\0original\nname");
    let expected = policy::object_error_parts(policy::ObjectFieldError::ExpectedObject);
    let prefix = source_identifiers::equal_text(expected[0], "expected object");
    let suffix = source_identifiers::equal_text(expected[1], "");
    expect_continuity_bool(prefix && suffix);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn retained_absence_checks_fields_before_the_boolean_value() {
    proof {
        reveal_strlit("absent"); reveal_strlit("bytes");
        assert("absent"@[0] != "bytes"@[0]);
        reveal(policy::retained_selection);
    }
    let observed = policy::RetainedObservation {
        object: true, keys: vec!["absent", "bytes"], absent: Some(Some(false)),
        bytes: Some(7u64), blob: None, digest: None, length: None,
    };
    proof {
        assert(policy::retained_mode(observed) == 0);
        assert(policy::retained_field_failure(observed, policy::ObjectFieldError::Unknown("bytes")));
    }
    let result = policy::select_retained(observed);
    match result {
        Err(policy::RetainedError::Fields(policy::ObjectFieldError::Unknown(name))) => {
            let exact = source_identifiers::equal_text(name, "bytes");
            expect_continuity_bool(exact);
        },
        _ => { expect_continuity_bool(false); },
    }
}

fn retained_decode(calls: Vec<u64>, input: u64) -> (out: (Vec<u64>, Result<Vec<u8>, ()>))
    ensures out.0@ == calls@.push(input),
        match out.1 {
            Ok(bytes) => input != 0 && bytes@ == seq![0u8, 255u8],
            Err(()) => input == 0,
        },
{
    let mut calls = calls;
    calls.push(input);
    if input == 0 { (calls, Err(())) } else { (calls, Ok(vec![0u8, 255u8])) }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn retained_absence_requires_true_and_never_decodes_bytes() {
    proof {
        reveal(policy::retained_selection);
        reveal(policy::retained_decoding);
    }
    let valid = policy::RetainedObservation::<u64> {
        object: true, keys: vec!["absent"], absent: Some(Some(true)),
        bytes: None, blob: None, digest: None, length: None,
    };
    proof { assert(policy::retained_fields_ok(valid)); }
    let (calls, result) = policy::parse_retained(Vec::new(), valid, retained_decode);
    expect_continuity_bool(calls.len() == 0);
    match result {
        Ok(policy::Retained::Absent) => {},
        _ => { expect_continuity_bool(false); },
    }
    let invalid = policy::RetainedObservation::<u64> {
        object: true, keys: vec!["absent"], absent: Some(None),
        bytes: None, blob: None, digest: None, length: None,
    };
    proof { assert(policy::retained_fields_ok(invalid)); }
    let (calls, result) = policy::parse_retained(Vec::new(), invalid, retained_decode);
    expect_continuity_bool(calls.len() == 0);
    match result {
        Err(policy::RetainedError::Absent) => {},
        _ => { expect_continuity_bool(false); },
    }
    let false_value = policy::RetainedObservation::<u64> {
        object: true, keys: vec!["absent"], absent: Some(Some(false)),
        bytes: None, blob: None, digest: None, length: None,
    };
    proof { assert(policy::retained_fields_ok(false_value)); }
    match policy::select_retained(false_value) {
        Err(policy::RetainedError::Absent) => {},
        _ => { expect_continuity_bool(false); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn retained_bytes_and_blob_must_have_exactly_one_present_key() {
    let both = policy::RetainedObservation {
        object: true, keys: vec!["bytes", "blob"], absent: None,
        bytes: Some(u64::MAX), blob: Some(None), digest: None, length: None,
    };
    let (calls, result) = policy::parse_retained(Vec::new(), both, retained_decode);
    expect_continuity_bool(calls.len() == 0);
    match result {
        Err(policy::RetainedError::Representation) => {},
        _ => { expect_continuity_bool(false); },
    }
    let neither = policy::RetainedObservation::<u64> {
        object: false, keys: Vec::new(), absent: None,
        bytes: None, blob: None, digest: None, length: None,
    };
    let (calls, result) = policy::parse_retained(Vec::new(), neither, retained_decode);
    expect_continuity_bool(calls.len() == 0);
    match result {
        Err(policy::RetainedError::Representation) => {},
        _ => { expect_continuity_bool(false); },
    }
}

fn retained_bytes_preserve_decoder_calls_and_original_output(input: u64) {
    proof {
        reveal(policy::retained_selection);
        reveal(policy::retained_decoding);
    }
    let observed = policy::RetainedObservation {
        object: true, keys: vec!["bytes"], absent: None,
        bytes: Some(input), blob: None, digest: None, length: None,
    };
    proof {
        assert(policy::retained_mode(observed) == 1);
        assert(policy::retained_fields_ok(observed));
    }
    let (calls, result) = policy::parse_retained(vec![31u64], observed, retained_decode);
    expect_continuity_bool(calls.len() == 2 && calls[0] == 31 && calls[1] == input);
    match result {
        Ok(policy::Retained::Bytes(bytes)) => {
            expect_continuity_bool(input != 0 && bytes.len() == 2 && bytes[0] == 0 && bytes[1] == 255);
        },
        Err(policy::RetainedError::Bytes) => { expect_continuity_bool(input == 0); },
        _ => { expect_continuity_bool(false); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn retained_bytes_decode_once_after_fields_and_keep_success_or_failure() {
    proof {
        reveal(policy::retained_selection);
        reveal(policy::retained_decoding);
    }
    retained_bytes_preserve_decoder_calls_and_original_output(0);
    retained_bytes_preserve_decoder_calls_and_original_output(1);
    retained_bytes_preserve_decoder_calls_and_original_output(u64::MAX);
    proof {
        reveal_strlit("bytes"); reveal_strlit("unexpected");
        assert("bytes"@.len() != "unexpected"@.len());
    }
    let unknown = policy::RetainedObservation {
        object: true, keys: vec!["bytes", "unexpected"], absent: None,
        bytes: Some(0u64), blob: None, digest: None, length: None,
    };
    proof {
        assert(policy::retained_mode(unknown) == 1);
        assert(policy::retained_field_failure(unknown, policy::ObjectFieldError::Unknown("unexpected")));
    }
    let (calls, result) = policy::parse_retained(Vec::new(), unknown, retained_decode);
    expect_continuity_bool(calls.len() == 0);
    match result {
        Err(policy::RetainedError::Fields(policy::ObjectFieldError::Unknown(name))) => {
            let exact = source_identifiers::equal_text(name, "unexpected");
            expect_continuity_bool(exact);
        },
        _ => { expect_continuity_bool(false); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn retained_blob_requires_fields_before_path_digest_or_length_checks() {
    proof {
        reveal_strlit("blob"); reveal_strlit("sha256"); reveal_strlit("length");
        reveal_strlit("unknown");
        assert("blob"@.len() != "sha256"@.len());
        assert("blob"@.len() != "length"@.len());
        assert("sha256"@[0] != "length"@[0]);
        assert("unknown"@.len() != "blob"@.len());
        assert("unknown"@.len() != "sha256"@.len());
        assert("unknown"@.len() != "length"@.len());
        reveal(policy::retained_selection);
        reveal(policy::retained_decoding);
    }
    let missing = policy::RetainedObservation::<u64> {
        object: true, keys: vec!["blob", "unknown"], absent: None,
        bytes: None, blob: Some(None), digest: None, length: None,
    };
    proof {
        assert(policy::retained_mode(missing) == 2);
        assert(policy::retained_field_failure(missing, policy::ObjectFieldError::Missing("sha256")));
    }
    let (calls, result) = policy::parse_retained(Vec::new(), missing, retained_decode);
    expect_continuity_bool(calls.len() == 0);
    match result {
        Err(policy::RetainedError::Fields(policy::ObjectFieldError::Missing(name))) => {
            let exact = source_identifiers::equal_text(name, "sha256");
            expect_continuity_bool(exact);
        },
        _ => { expect_continuity_bool(false); },
    }
    let unknown = policy::RetainedObservation::<u64> {
        object: true, keys: vec!["blob", "sha256", "length", "unknown", ""], absent: None,
        bytes: None, blob: Some(None), digest: None, length: None,
    };
    let result = policy::select_retained(unknown);
    match result {
        Err(policy::RetainedError::Fields(policy::ObjectFieldError::Unknown(name))) => {
            let exact = source_identifiers::equal_text(name, "unknown");
            expect_continuity_bool(exact);
        },
        _ => { expect_continuity_bool(false); },
    }
}

fn retained_blob_boundaries(path: &str, digest: &str, length: u64)
    requires policy::result_types::paths::host_state_path(path.spec_bytes()),
        source_identifiers::digest_format(digest.spec_bytes()),
{
    let success = policy::RetainedObservation::<u64> {
        object: true, keys: vec!["length", "sha256", "blob"], absent: None,
        bytes: None, blob: Some(Some(path)), digest: Some(digest), length: Some(length),
    };
    let (calls, result) = policy::parse_retained(Vec::new(), success, retained_decode);
    expect_continuity_bool(calls.len() == 0);
    match result {
        Ok(policy::Retained::Blob { path: actual_path, digest: actual_digest, length: actual_length }) => {
            let same_path = source_identifiers::equal_text(actual_path, path);
            let same_digest = source_identifiers::equal_text(actual_digest, digest);
            expect_continuity_bool(same_path && same_digest && actual_length == length);
        },
        _ => { expect_continuity_bool(false); },
    }
    let bad_path = policy::RetainedObservation::<u64> {
        object: true, keys: vec!["blob", "sha256", "length"], absent: None,
        bytes: None, blob: Some(None), digest: None, length: None,
    };
    match policy::select_retained(bad_path) {
        Err(policy::RetainedError::Path) => {},
        _ => { expect_continuity_bool(false); },
    }
    let bad_digest = policy::RetainedObservation::<u64> {
        object: true, keys: vec!["blob", "sha256", "length"], absent: None,
        bytes: None, blob: Some(Some(path)), digest: Some(""), length: None,
    };
    proof { reveal_strlit(""); vstd::utf8::is_ascii_chars_encode_utf8(""@); }
    match policy::select_retained(bad_digest) {
        Err(policy::RetainedError::Digest) => {},
        _ => { expect_continuity_bool(false); },
    }
    let bad_length = policy::RetainedObservation::<u64> {
        object: true, keys: vec!["blob", "sha256", "length"], absent: None,
        bytes: None, blob: Some(Some(path)), digest: Some(digest), length: None,
    };
    match policy::select_retained(bad_length) {
        Err(policy::RetainedError::Length) => {},
        _ => { expect_continuity_bool(false); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn retained_blob_checks_path_then_digest_then_length_and_preserves_boundary_values() {
    let path = ".chrono-harness/state/x";
    let digest = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    proof {
        reveal_strlit(".chrono-harness/state/x");
        reveal_strlit("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        vstd::utf8::is_ascii_chars_encode_utf8(path@);
        vstd::utf8::is_ascii_chars_encode_utf8(digest@);
        let path_bytes = seq![46u8,99,104,114,111,110,111,45,104,97,114,110,101,115,115,47,115,116,97,116,101,47,120];
        let digest_bytes = seq![48u8,49,50,51,52,53,54,55,56,57,97,98,99,100,101,102,
            48,49,50,51,52,53,54,55,56,57,97,98,99,100,101,102,
            48,49,50,51,52,53,54,55,56,57,97,98,99,100,101,102,
            48,49,50,51,52,53,54,55,56,57,97,98,99,100,101,102];
        assert(path.spec_bytes() =~= path_bytes);
        assert(digest.spec_bytes() =~= digest_bytes);
        assert(policy::result_types::paths::host_state_path(path_bytes)) by (compute);
        assert(source_identifiers::digest_format(digest_bytes)) by (compute);
    }
    retained_blob_boundaries(path, digest, 0);
    retained_blob_boundaries(path, digest, u64::MAX);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn retained_nonobject_precedes_fields_and_invalid_paths_precede_other_values() {
    let nonobject = policy::RetainedObservation {
        object: false, keys: vec!["unknown"], absent: Some(None),
        bytes: Some(7u64), blob: Some(None), digest: None, length: None,
    };
    let (calls, result) = policy::parse_retained(Vec::new(), nonobject, retained_decode);
    expect_continuity_bool(calls.len() == 0);
    match result {
        Err(policy::RetainedError::Fields(policy::ObjectFieldError::ExpectedObject)) => {},
        _ => { expect_continuity_bool(false); },
    }
    let bad_path = policy::RetainedObservation::<u64> {
        object: true, keys: vec!["blob", "sha256", "length"], absent: None,
        bytes: None, blob: Some(Some("")), digest: None, length: None,
    };
    proof { reveal_strlit(""); vstd::utf8::is_ascii_chars_encode_utf8(""@); }
    match policy::select_retained(bad_path) {
        Err(policy::RetainedError::Path) => {},
        _ => { expect_continuity_bool(false); },
    }
}

fn retained_error_has_exact_parts(error: policy::RetainedError, head: &str, suffix: &str)
    requires head@ == policy::retained_error_head(error),
        suffix@ == policy::retained_error_suffix(error),
{
    let parts = error.parts();
    let prefix = source_identifiers::equal_text(parts[0], "E_INPUT_BLOB: ");
    let body = source_identifiers::equal_text(parts[1], head);
    let field = source_identifiers::equal_text(parts[2], suffix);
    expect_continuity_bool(prefix && body && field);
}

fn retained_error_preserves_any_field_name(name: &str) {
    retained_error_has_exact_parts(policy::RetainedError::Fields(policy::ObjectFieldError::Missing(name)),
        "missing field ", name);
    retained_error_has_exact_parts(policy::RetainedError::Fields(policy::ObjectFieldError::Unknown(name)),
        "unknown field ", name);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn retained_errors_keep_the_prefix_category_and_full_original_field_name() {
    retained_error_preserves_any_field_name("");
    retained_error_preserves_any_field_name("字段\0原文\nfield");
    retained_error_has_exact_parts(policy::RetainedError::Fields(policy::ObjectFieldError::ExpectedObject),
        "expected object", "");
    retained_error_has_exact_parts(policy::RetainedError::Representation,
        "expected exactly one bytes or blob representation", "");
    retained_error_has_exact_parts(policy::RetainedError::Absent, "absent must be true", "");
    retained_error_has_exact_parts(policy::RetainedError::Bytes, "invalid retained byte array", "");
    retained_error_has_exact_parts(policy::RetainedError::Path, "reference must be within host state", "");
    retained_error_has_exact_parts(policy::RetainedError::Digest, "invalid digest", "");
    retained_error_has_exact_parts(policy::RetainedError::Length, "invalid length", "");
}

} // verus!
