//! 独立边界预期直接消费生产集合判断，Cargo 与 Verus 共用源码。
#![allow(unexpected_cfgs)]
#[path = "../../judge-ci/src/verified_collection.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
use policy::result_types::{CheckResult, Status};
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

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
