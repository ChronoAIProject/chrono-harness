//! 交接边界的独立预期，直接消费真实 Verus 实现。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../runner/src/retained_artifacts/verified_release.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_release as policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

#[cfg(not(verus_keep_ghost))]
use chrono_harness::retained_artifacts::retention as accounting;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;
#[cfg(verus_keep_ghost)]
use policy::accounting;
#[cfg(verus_keep_ghost)]
use policy::identifiers;

#[cfg(verus_keep_ghost)]
#[path = "support/retention_frontier.rs"]
mod retention_frontier;

verus! {
#[cfg_attr(not(verus_keep_ghost), test)]
fn archived_tree_must_contain_the_report_and_own_all_its_inputs() {
    let tree = policy::archived_report_binding(true, true, true, false);
    let external = policy::archived_report_binding(true, true, false, true);
    let missing = policy::archived_report_binding(true, false, true, true);
    let file = policy::archived_report_binding(false, false, false, true);
    let unbound = policy::archived_report_binding(false, true, true, false);
    assert(tree && file && !external && !missing && !unbound);
}

proof fn two_creation_ticks_can_fill_the_storage_bound() {
    let live = seq![3nat, 3nat, 0nat];
    policy::rolling_storage_bound(live, 2, 3);
    reveal_with_fuel(policy::temporary_bytes_by_age, 4);
    assert(policy::temporary_bytes_by_age(live) == 6);
}

/// Even if every generation disappears on its next tick, the current
/// generation alone can exceed any proposed capacity without a size bound.
proof fn one_tick_disposal_allows_arbitrary_volume(limit: nat) {
    let live = seq![limit + 1, 0nat];
    reveal_with_fuel(policy::temporary_bytes_by_age, 3);
    assert(live[1] == 0 && policy::temporary_bytes_by_age(live) > limit);
}

/// For every proposed deadline there is a fair trace whose single file is
/// still present at that deadline. The file disappears one second later.
proof fn fairness_allows_arbitrarily_late_disposal(limit: nat) {
    let remaining = |second: nat| if second <= limit { 1nat } else { 0nat };
    assert forall|second: nat| #[trigger] remaining(second) > 0 implies
        exists|later: nat| second < later && #[trigger] remaining(later) < remaining(second) by {
        assert(remaining(limit + 1) == 0);
    }
    assert(policy::fair_retirement(remaining, 0));
    assert(remaining(limit) == 1);
}

/// A one-second service bound with two obligations is attained exactly.
proof fn two_effects_can_use_both_seconds() {
    let remaining = |second: nat| if second < 2 { (2 - second) as nat } else { 0nat };
    policy::retirement_within_service_bound(remaining, 0, 1);
    assert(remaining(1) == 1 && remaining(2) == 0);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn archive_handoff_requires_matching_publication_and_completed_obligations() {
    expect_true(policy::archived_slot_ready(true, true, 2, 2, false, false, false));
    expect_true(!policy::archived_slot_ready(false, true, 2, 2, false, false, false));
    expect_true(!policy::archived_slot_ready(true, false, 2, 2, false, false, false));
    expect_true(!policy::archived_slot_ready(true, true, 2, 0, false, false, false));
    expect_true(!policy::archived_slot_ready(true, true, 2, 2, true, false, false));
    expect_true(!policy::archived_slot_ready(true, true, 2, 2, false, true, false));
    expect_true(!policy::archived_slot_ready(true, true, 2, 2, false, false, true));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn creation_window_and_expiry_clamp_keep_exact_time_boundaries() {
    expect_true(!accounting::within_creation_window(10, 20, 9));
    expect_true(accounting::within_creation_window(10, 20, 10));
    expect_true(accounting::within_creation_window(10, 20, 19));
    expect_true(!accounting::within_creation_window(10, 20, 20));
    expect_true(!accounting::within_creation_window(10, 20, u64::MAX));
    expect_true(!accounting::within_creation_window(0, 0, 0));
    expect_true(!accounting::within_creation_window(u64::MAX, u64::MAX, u64::MAX));
    expect_true(accounting::bounded_expiry(0, 20) == 0);
    expect_true(accounting::bounded_expiry(19, 20) == 19);
    expect_true(accounting::bounded_expiry(20, 20) == 20);
    expect_true(accounting::bounded_expiry(u64::MAX, 20) == 20);
    expect_true(accounting::bounded_expiry(u64::MAX, u64::MAX) == u64::MAX);
}

fn expect_retirement_deferral(metadata_room: bool, nodes: usize, node_limit: usize,
    elapsed_millis: u128, max_millis: u64, deferred: bool)
    requires deferred == (!metadata_room || nodes >= node_limit
        || elapsed_millis >= max_millis as u128),
{
    let reason = policy::retirement_deferral(metadata_room, nodes, node_limit,
        elapsed_millis, max_millis, false);
    match reason {
        Some(text) => {
            expect_true(deferred);
            let same = identifiers::equal_text(text,
                "retirement round budget exhausted; original not attempted");
            expect_true(same);
        },
        None => expect_true(!deferred),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn retirement_admission_keeps_each_original_budget_boundary_and_full_reason() {
    expect_true(policy::retirement_time_available(true, 29, 25));
    expect_true(!policy::retirement_time_available(false, 29, 25));
    let admitted = policy::retirement_deferral(true, 0, 2, 29, 25, true);
    let full = policy::retirement_deferral(true, 2, 2, 29, 25, true);
    let no_metadata = policy::retirement_deferral(false, 0, 2, 29, 25, true);
    expect_true(admitted.is_none() && full.is_some() && no_metadata.is_some());
    expect_retirement_deferral(true, 0, 1, 0, 1, false);
    expect_retirement_deferral(true, 31, 32, 24, 25, false);
    expect_retirement_deferral(false, 0, 1, 0, 1, true);
    expect_retirement_deferral(true, 32, 32, 0, 25, true);
    expect_retirement_deferral(true, 33, 32, 0, 25, true);
    expect_retirement_deferral(true, 0, 32, 25, 25, true);
    expect_retirement_deferral(true, 0, 32, 26, 25, true);
    expect_retirement_deferral(true, 0, 0, 0, 25, true);
    expect_retirement_deferral(true, 0, 32, 0, 0, true);
    expect_retirement_deferral(true, usize::MAX - 1, usize::MAX,
        u64::MAX as u128 - 1, u64::MAX, false);
    expect_retirement_deferral(true, usize::MAX, usize::MAX,
        u128::MAX, u64::MAX, true);
}


fn skipped_disposal_checkpoint_preserves_state(tree: bool, remaining: usize)
    requires !tree || remaining == 0,
{
    let persist = |calls: u8| -> (out: (u8, Result<(), &str>))
        ensures out.0 == 1, out.1 == Err::<(), &str>("unexpected checkpoint"),
    { (1, Err("unexpected checkpoint")) };
    let (calls, result) = policy::checkpoint_disposal(0u8, tree, remaining, &persist);
    expect_true(calls == 0 && result.is_ok());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn disposal_checkpoint_skips_files_and_exhausted_tree_cursors() {
    skipped_disposal_checkpoint_preserves_state(false, 0);
    skipped_disposal_checkpoint_preserves_state(false, 1);
    skipped_disposal_checkpoint_preserves_state(false, usize::MAX);
    skipped_disposal_checkpoint_preserves_state(true, 0);
}

fn partial_tree_checkpoint_preserves_original_result(remaining: usize, error: &str)
    requires remaining > 0,
{
    let persist = |calls: u8| -> (out: (u8, Result<(), &str>))
        ensures out.0 == if calls == 0 { 1u8 } else { 2u8 },
            out.1 == Err::<(), &str>(error),
    { (if calls == 0 { 1 } else { 2 }, Err(error)) };
    let (calls, result) = policy::checkpoint_disposal(0u8, true, remaining, &persist);
    expect_true(calls == 1);
    match result {
        Err(actual) => {
            let same = identifiers::equal_text(actual, error);
            expect_true(same);
        },
        Ok(()) => { assert(false); panic!(); },
    }
    let persist_ok = |calls: u8| -> (out: (u8, Result<(), &str>))
        ensures out.0 == if calls == 0 { 1u8 } else { 2u8 }, out.1.is_ok(),
    { (if calls == 0 { 1 } else { 2 }, Ok(())) };
    let (calls, result) = policy::checkpoint_disposal(0u8, true, remaining, &persist_ok);
    expect_true(calls == 1 && result.is_ok());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn disposal_checkpoint_persists_partial_trees_once_and_preserves_complete_errors() {
    partial_tree_checkpoint_preserves_original_result(1, "");
    partial_tree_checkpoint_preserves_original_result(2, "checkpoint: 中\0original\nerror");
    partial_tree_checkpoint_preserves_original_result(usize::MAX, "original IO failure");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn absent_disposal_recovery_retains_its_original_phase_classification() {
    proof {
        reveal_strlit("disposing"); reveal_strlit("released"); reveal_strlit("");
        reveal_strlit("disposing\0tail");
        assert("released"@.len() != "disposing"@.len());
        assert(""@.len() != "disposing"@.len());
        assert("disposing\0tail"@.len() != "disposing"@.len());
    }
    let partial = policy::absent_disposal_fact("disposing");
    let same = identifiers::equal_text(partial, "absent-after-partial-disposal");
    expect_true(same);
    let released = policy::absent_disposal_fact("released");
    let same = identifiers::equal_text(released, "already-absent-before-disposal");
    expect_true(same);
    let empty = policy::absent_disposal_fact("");
    let same = identifiers::equal_text(empty, "already-absent-before-disposal");
    expect_true(same);
    let unknown = policy::absent_disposal_fact("disposing\0tail");
    let same = identifiers::equal_text(unknown, "already-absent-before-disposal");
    expect_true(same);
}

spec fn fixture_edge(rows: Seq<(String, Vec<String>)>, source: Seq<char>, target: Seq<char>) -> bool {
    exists|i: int, j: int| 0 <= i < rows.len() && 0 <= j < rows[i].1@.len()
        && rows[i].0@ == source && (#[trigger] rows[i].1@[j])@ == target
}

fn reference_fixture<'a>(rows: &'a [(String, Vec<String>)], source: &str, target: &str)
    -> (found: bool)
    ensures found == fixture_edge(rows@, source@, target@),
{
    let observe = |row: &'a (String, Vec<String>)| -> (out: (&'a str, &'a [String]))
        ensures out.0@ == row.0@, out.1@ == row.1@,
    { (row.0.as_str(), row.1.as_slice()) };
    let found = policy::objects_reference(rows, source, target, observe);
    proof {
        let trace = choose|trace: Seq<(&str, &[String])>| policy::output_reference_trace(
            rows, observe, source@, target@, found, trace);
        assert forall|i: int| 0 <= i < trace.len() implies
            trace[i].0@ == rows@[i].0@ && trace[i].1@ == rows@[i].1@ by {
            assert(observe.ensures((&rows@[i],), trace[i]));
        }
        if found {
            let i = trace.len() - 1;
            let j = choose|j: int| 0 <= j < trace[i].1@.len()
                && trace[i].1@[j]@ == target@;
            assert(rows@[i].0@ == source@ && rows@[i].1@[j]@ == target@);
            assert(fixture_edge(rows@, source@, target@));
        } else {
            assert forall|i: int, j: int| 0 <= i < rows.len() && 0 <= j < rows@[i].1@.len()
                implies rows@[i].0@ != source@ || rows@[i].1@[j]@ != target@ by {
                assert(!policy::output_edge_matches(trace[i], source@, target@));
            }
        }
    }
    found
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn output_references_require_both_complete_source_and_complete_target() {
    proof {
        reveal_strlit("source"); reveal_strlit("other"); reveal_strlit("target");
        reveal_strlit("absent");
        assert("source"@[0] != "other"@[0]);
        assert("source"@[0] != "absent"@[0]);
        assert("target"@[0] != "absent"@[0]);
    }
    let empty = reference_fixture(&Vec::new(), "source", "target");
    expect_true(!empty);
    let rows = vec![
        ("other".to_owned(), vec!["target".to_owned()]),
        ("source".to_owned(), vec!["absent".to_owned()]),
    ];
    let crossed = reference_fixture(&rows, "source", "target");
    let present = reference_fixture(&rows, "other", "target");
    let missing_source = reference_fixture(&rows, "absent", "target");
    assert(rows@[0].0@ == "other"@ && rows@[0].1@[0]@ == "target"@);
    assert(fixture_edge(rows@, "other"@, "target"@));
    assert(!crossed);
    assert(!missing_source);
    expect_true(!crossed && present && !missing_source);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn output_references_search_every_duplicate_source_and_reference_position() {
    proof {
        reveal_strlit("source"); reveal_strlit("other"); reveal_strlit("target");
        reveal_strlit("before"); reveal_strlit("after");
        assert("target"@[0] != "before"@[0] && "target"@[0] != "after"@[0]);
        assert("source"@[0] != "other"@[0]);
    }
    let rows = vec![
        ("source".to_owned(), Vec::new()),
        ("other".to_owned(), vec!["target".to_owned()]),
        ("source".to_owned(), vec!["before".to_owned(), "target".to_owned(), "after".to_owned()]),
        ("source".to_owned(), vec!["target".to_owned(), "target".to_owned()]),
    ];
    let target = reference_fixture(&rows, "source", "target");
    let final_reference = reference_fixture(&rows, "source", "after");
    let empty_row = vec![("source".to_owned(), Vec::new())];
    let empty_first = reference_fixture(&empty_row, "source", "target");
    assert(rows@[2].0@ == "source"@ && rows@[2].1@[1]@ == "target"@);
    assert(fixture_edge(rows@, "source"@, "target"@));
    assert(rows@[2].0@ == "source"@ && rows@[2].1@[2]@ == "after"@);
    assert(fixture_edge(rows@, "source"@, "after"@));
    expect_true(target && final_reference && !empty_first);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn output_references_preserve_empty_unicode_nul_and_case_sensitive_ids() {
    proof {
        reveal_strlit(""); reveal_strlit("source: é\0full\nend");
        reveal_strlit("target: 中\0tail\nend"); reveal_strlit("Source: é\0full\nend");
        reveal_strlit("target: 中\0tail\nEnd");
        assert("source: é\0full\nend"@[0] != "Source: é\0full\nend"@[0]);
        assert("target: 中\0tail\nend"@[15] != "target: 中\0tail\nEnd"@[15]);
    }
    let rows = vec![
        ("".to_owned(), vec!["".to_owned()]),
        ("source: é\0full\nend".to_owned(), vec!["target: 中\0tail\nend".to_owned()]),
    ];
    let empty_ids = reference_fixture(&rows, "", "");
    let full_ids = reference_fixture(&rows, "source: é\0full\nend", "target: 中\0tail\nend");
    let source_case = reference_fixture(&rows, "Source: é\0full\nend", "target: 中\0tail\nend");
    let target_case = reference_fixture(&rows, "source: é\0full\nend", "target: 中\0tail\nEnd");
    assert(rows@[0].0@ == ""@ && rows@[0].1@[0]@ == ""@);
    assert(fixture_edge(rows@, ""@, ""@));
    assert(rows@[1].0@ == "source: é\0full\nend"@ && rows@[1].1@[0]@ == "target: 中\0tail\nend"@);
    assert(fixture_edge(rows@, "source: é\0full\nend"@, "target: 中\0tail\nend"@));
    expect_true(empty_ids && full_ids && !source_case && !target_case);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_accounting_never_substitutes_for_domain_completion() {
    expect_true(!accounting::accounting_releases(false, false));
    expect_true(!accounting::accounting_releases(false, true));
    expect_true(!accounting::accounting_releases(true, false));
    expect_true(accounting::accounting_releases(true, true));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn migration_custody_excludes_existing_capabilities_without_attaching_reservations() {
    proof {
        reveal_strlit("reserving"); reveal_strlit("creating");
        reveal_strlit("released"); reveal_strlit(""); reveal_strlit("inode:original");
        assert("creating"@.len() != "reserving"@.len());
    }
    let absent = policy::migration_custody("reserving", "", true);
    expect_true(absent.admitted && !absent.acquire && absent.expected.is_none());
    let unbound = policy::migration_custody("reserving", "", false);
    expect_true(unbound.admitted && unbound.acquire && unbound.expected.is_none());
    let unknown = policy::migration_custody("creating", "", false);
    expect_true(!unknown.admitted && unknown.acquire && unknown.expected.is_none());
    let missing_unknown = policy::migration_custody("creating", "", true);
    expect_true(!missing_unknown.admitted && !missing_unknown.acquire
        && missing_unknown.expected.is_none());
    let attached = policy::migration_custody("creating", "inode:original", false);
    expect_true(attached.admitted && attached.acquire);
    expect_true(policy::contains(&vec![attached.expected.unwrap().to_owned()], "inode:original"));
    let missing_attached = policy::migration_custody("creating", "inode:original", true);
    expect_true(missing_attached.admitted && missing_attached.acquire);
    expect_true(policy::contains(&vec![missing_attached.expected.unwrap().to_owned()], "inode:original"));
    let reserved_attached = policy::migration_custody("reserving", "inode:original", true);
    expect_true(reserved_attached.admitted && reserved_attached.acquire);
    expect_true(policy::contains(&vec![reserved_attached.expected.unwrap().to_owned()], "inode:original"));
    let completed = policy::migration_custody("released", "inode:original", false);
    expect_true(completed.admitted && completed.acquire);
    expect_true(policy::contains(&vec![completed.expected.unwrap().to_owned()], "inode:original"));
}

#[cfg(verus_keep_ghost)]
fn bounded_inputs_accept_the_limit_and_reject_the_next_byte() {
    expect_true(policy::commands::within_bound(0, 0));
    expect_true(!policy::commands::within_bound(1, 0));
    expect_true(policy::commands::within_bound(65536, 65536));
    expect_true(!policy::commands::within_bound(65537, 65536));
    expect_true(policy::commands::within_bound(1048576, 1048576));
    expect_true(!policy::commands::within_bound(1048577, 1048576));
    expect_true(policy::commands::within_bound(u64::MAX, u64::MAX));
    expect_true(!policy::commands::within_bound(u64::MAX, u64::MAX - 1));
}

fn expect_retirement_id(actual: &String, expected: &str)
    requires actual@ == expected@,
{
    let one = vec![actual.clone()];
    expect_true(policy::contains(&one, expected));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn retirement_selection_has_no_work_for_empty_or_terminal_records() {
    proof { reveal_with_fuel(policy::pending_retirement_values, 6); }
    let empty = policy::pending_retirements(&Vec::new());
    expect_true(empty.is_empty());
    let terminal = vec![("retired".to_owned(), true), ("also retired".to_owned(), true)];
    let done = policy::pending_retirements(&terminal);
    expect_true(done.is_empty());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn retirement_selection_preserves_every_active_record_in_order() {
    proof { reveal_with_fuel(policy::pending_retirement_values, 6); }
    let records = vec![("first".to_owned(), false), ("second".to_owned(), false)];
    let pending = policy::pending_retirements(&records);
    expect_true(pending.len() == 2);
    expect_retirement_id(&pending[0], "first");
    expect_retirement_id(&pending[1], "second");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn retirement_selection_keeps_byte_absent_records_until_state_is_terminal() {
    proof { reveal_with_fuel(policy::pending_retirement_values, 6); }
    let records = vec![
        ("finished".to_owned(), true),
        ("bytes removed; edges pending".to_owned(), false),
        ("unvisited".to_owned(), false),
        ("finished after".to_owned(), true),
    ];
    let pending = policy::pending_retirements(&records);
    expect_true(pending.len() == 2);
    expect_retirement_id(&pending[0], "bytes removed; edges pending");
    expect_retirement_id(&pending[1], "unvisited");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn retirement_selection_preserves_complete_ids_and_duplicate_observations() {
    proof { reveal_with_fuel(policy::pending_retirement_values, 6); }
    let records = vec![
        ("original: é\0full\nend".to_owned(), false),
        ("original: é\0full\nend".to_owned(), true),
        ("original: é\0full\nend".to_owned(), false),
        ("".to_owned(), false),
    ];
    let pending = policy::pending_retirements(&records);
    expect_true(pending.len() == 3);
    expect_retirement_id(&pending[0], "original: é\0full\nend");
    expect_retirement_id(&pending[1], "original: é\0full\nend");
    expect_retirement_id(&pending[2], "");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn report_recovery_checks_current_formats_and_exit_boundaries() {
    proof {
        reveal_strlit("chrono-report-publication/v1"); reveal_strlit("wrong");
        reveal_strlit("chrono-check-report/v1"); reveal_strlit("chrono-check-report/v2");
        reveal_strlit("original"); reveal_strlit("other");
        assert("original"@[1] != "other"@[1]);
        assert("wrong"@[0] != "chrono-report-publication/v1"@[0]);
        assert("wrong"@[0] != "chrono-check-report/v1"@[0]);
        assert("wrong"@[0] != "chrono-check-report/v2"@[0]);
        assert("chrono-check-report/v1"@[21] != "chrono-check-report/v2"@[21]);
    }
    expect_true(policy::report_publication("chrono-report-publication/v1", 0));
    expect_true(policy::report_publication("chrono-report-publication/v1", 1));
    expect_true(policy::report_publication("chrono-report-publication/v1", 2));
    expect_true(!policy::report_publication("chrono-report-publication/v1", 3));
    expect_true(!policy::report_publication("chrono-report-publication/v1", 255));
    expect_true(!policy::report_publication("wrong", 0));
    expect_true(policy::recovered_report_kind(Some("chrono-check-report/v1"), None, Some("original"), None, "original") == 1);
    expect_true(policy::recovered_report_kind(Some("chrono-check-report/v2"), None, Some("original"), None, "original") == 2);
    expect_true(policy::recovered_report_kind(None, Some(1), None, Some("original"), "original") == 3);
    expect_true(policy::recovered_report_kind(Some("chrono-check-report/v2"), Some(1), Some("other"), Some("original"), "original") == 0);
    expect_true(policy::recovered_report_kind(None, Some(2), None, Some("original"), "original") == 0);
    expect_true(policy::recovered_report_kind(None, None, None, Some("original"), "original") == 0);
    expect_true(policy::recovered_report_kind(None, Some(1), None, None, "original") == 0);
    expect_true(policy::recovered_report_kind(Some("wrong"), Some(1), Some("original"), Some("original"), "original") == 0);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn report_recovery_requires_one_sealed_matching_operation_outcome() {
    proof {
        reveal_strlit("request"); reveal_strlit("requesT");
        assert("request"@[6] != "requesT"@[6]);
    }
    expect_true(policy::report_outcome(1, Some("request"), Some(0), "request", 0));
    expect_true(policy::report_outcome(1, Some("request"), Some(1), "request", 1));
    expect_true(policy::report_outcome(1, Some("request"), Some(2), "request", 2));
    expect_true(!policy::report_outcome(0, Some("request"), Some(0), "request", 0));
    expect_true(!policy::report_outcome(2, Some("request"), Some(0), "request", 0));
    expect_true(!policy::report_outcome(1, Some("requesT"), Some(0), "request", 0));
    expect_true(!policy::report_outcome(1, None, Some(0), "request", 0));
    expect_true(!policy::report_outcome(1, Some("request"), None, "request", 0));
    expect_true(!policy::report_outcome(1, Some("request"), Some(1), "request", 0));
    expect_true(!policy::report_outcome(1, Some("request"), Some(u64::MAX), "request", 2));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn report_slot_recovery_preserves_newer_reports_and_reuses_its_original() {
    proof {
        reveal_strlit("old"); reveal_strlit("new"); reveal_strlit("else");
        assert("old"@[0] != "new"@[0]);
        assert("old"@[0] != "else"@[0]);
        assert("new"@[0] != "else"@[0]);
    }
    let empty = policy::report_slot(None, None, "new");
    expect_true(empty.allowed && !empty.already_published);
    let unchanged = policy::report_slot(Some("old"), Some("old"), "new");
    expect_true(unchanged.allowed && !unchanged.already_published);
    let repeated = policy::report_slot(Some("new"), Some("old"), "new");
    expect_true(repeated.allowed && repeated.already_published);
    let initially_same = policy::report_slot(Some("new"), Some("new"), "new");
    expect_true(initially_same.allowed && initially_same.already_published);
    let newer = policy::report_slot(Some("else"), Some("old"), "new");
    expect_true(!newer.allowed && !newer.already_published);
    let removed = policy::report_slot(None, Some("old"), "new");
    expect_true(!removed.allowed && !removed.already_published);
    let replaced_empty = policy::report_slot(Some("else"), None, "new");
    expect_true(!replaced_empty.allowed && !replaced_empty.already_published);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn pending_domain_and_resumed_seal_remain_distinct_from_terminal_eligibility() {
    let pending = policy::domain_recovery(true, true);
    let absent = policy::domain_recovery(true, false);
    let unnamed = policy::domain_recovery(false, true);
    let terminal = policy::terminal_eligible(true, true);
    let unknown = policy::terminal_eligible(false, true);
    let unsealed = policy::terminal_eligible(true, false);
    let completed = policy::resumed_seal(true, true, false);
    let interrupted = policy::resumed_seal(true, true, true);
    let partial = policy::resumed_seal(true, false, false);
    expect_true(pending); expect_true(!absent); expect_true(!unnamed);
    expect_true(terminal); expect_true(!unknown); expect_true(!unsealed);
    expect_true(completed); expect_true(!interrupted); expect_true(!partial);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn current_roots_distinguish_unread_absent_nonmember_and_member_roles() {
    expect_true(!policy::current_roles_protect(&vec![]));
    expect_true(!policy::current_roles_protect(&vec![None, Some(Some(false))]));
    expect_true(policy::current_roles_protect(&vec![Some(None)]));
    expect_true(policy::current_roles_protect(&vec![Some(Some(true))]));
    expect_true(policy::current_roles_protect(&vec![None, Some(Some(false)), Some(None)]));
    expect_true(policy::current_roles_protect(&vec![Some(Some(false)), Some(Some(true)), None]));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn finalized_requires_sealed_released_or_disposing_output_and_no_retired_leases() {
    proof {
        reveal_strlit("released"); reveal_strlit("disposing");
        reveal_strlit("publishing"); reveal_strlit("released\0");
        assert("publishing"@[0] != "released"@[0]);
        assert("publishing"@[0] != "disposing"@[0]);
        assert("released\0"@.len() != "released"@.len());
        assert("released\0"@[0] != "disposing"@[0]);
    }
    expect_true(!policy::original_finalized(0, None));
    expect_true(policy::original_finalized(0, Some((true, "released"))));
    expect_true(policy::original_finalized(0, Some((true, "disposing"))));
    expect_true(!policy::original_finalized(0, Some((false, "released"))));
    expect_true(!policy::original_finalized(0, Some((true, "publishing"))));
    expect_true(!policy::original_finalized(0, Some((true, "released\0"))));
    expect_true(!policy::original_finalized(1, Some((true, "released"))));
    expect_true(!policy::original_finalized(usize::MAX, Some((true, "disposing"))));
    expect_true(policy::defer_declared_original(true, true));
    expect_true(!policy::defer_declared_original(false, true));
    expect_true(!policy::defer_declared_original(true, false));
    expect_true(!policy::defer_declared_original(false, false));
}

fn expect_expansion(actual: policy::FinalizedExpansion, expected: u8)
    requires match actual {
        policy::FinalizedExpansion::RetainOnly => expected == 0,
        policy::FinalizedExpansion::RootFrontier => expected == 1,
        policy::FinalizedExpansion::RetireFrontier => expected == 2,
        policy::FinalizedExpansion::Related => expected == 3,
    },
{
    let observed = match actual {
        policy::FinalizedExpansion::RetainOnly => 0u8,
        policy::FinalizedExpansion::RootFrontier => 1u8,
        policy::FinalizedExpansion::RetireFrontier => 2u8,
        policy::FinalizedExpansion::Related => 3u8,
    };
    expect_true(observed == expected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn only_actual_roots_stop_expansion_and_rootless_cycles_remain_related() {
    proof {
        reveal_strlit("initial"); reveal_strlit("parent");
        reveal_strlit("initial\0"); reveal_strlit("é"); reveal_strlit("e\u{301}");
        assert("initial"@[0] != "parent"@[0]);
        assert("initial"@.len() != "initial\0"@.len());
        assert("é"@[0] != "e\u{301}"@[0]);
    }
    expect_expansion(policy::original_expansion(true, true, Some("initial"), "initial", 1, 1), 0);
    expect_expansion(policy::original_expansion(true, true, Some("initial"), "parent", 0, 1), 1);
    expect_expansion(policy::original_expansion(true, true, None, "parent", 0, 1), 1);
    expect_expansion(policy::original_expansion(false, true, Some("initial"), "initial", 0, 0), 2);
    expect_expansion(policy::original_expansion(false, true, Some("initial"), "initial", 1, 0), 3);
    expect_expansion(policy::original_expansion(false, true, Some("initial"), "initial", usize::MAX, 0), 3);
    expect_expansion(policy::original_expansion(false, true, Some("initial"), "initial", 0, 1), 3);
    expect_expansion(policy::original_expansion(false, true, Some("initial"), "parent", 0, 0), 2);
    expect_expansion(policy::original_expansion(true, true, Some("initial"), "initial\0", 0, 1), 1);
    expect_expansion(policy::original_expansion(false, true, Some("é"), "e\u{301}", 0, 0), 2);
    expect_expansion(policy::original_expansion(true, false, Some("initial"), "initial", 0, 1), 0);
    expect_expansion(policy::original_expansion(true, false, Some("initial"), "parent", 0, 1), 1);
    expect_expansion(policy::original_expansion(false, false, Some("initial"), "initial", 0, 0), 3);
    expect_expansion(policy::original_expansion(false, false, Some("initial"), "parent", 1, 0), 3);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn deferred_declarations_keep_exact_identity_and_are_consumed_once() {
    let mut deferred = policy::DeferredOriginal::new();
    expect_true(deferred.take().is_none());
    deferred.defer("parent λ\0\n".to_owned());
    match deferred.take() {
        Some(id) => expect_retirement_id(&id, "parent λ\0\n"),
        None => panic!("deferred identity was lost"),
    }
    expect_true(deferred.take().is_none());
    deferred.defer("next".to_owned());
    match deferred.take() {
        Some(id) => expect_retirement_id(&id, "next"),
        None => panic!("next identity was lost"),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn discovered_frontier_keeps_actual_members_dependencies_and_retiring_originals() {
    expect_true(!policy::preserve_discovered_original(false, false, false));
    expect_true(policy::preserve_discovered_original(true, false, false));
    expect_true(policy::preserve_discovered_original(false, true, false));
    expect_true(policy::preserve_discovered_original(false, false, true));
    expect_true(policy::preserve_discovered_original(true, true, false));
    expect_true(policy::preserve_discovered_original(true, false, true));
    expect_true(policy::preserve_discovered_original(false, true, true));
    expect_true(policy::preserve_discovered_original(true, true, true));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn maintenance_paths_preserve_initial_roots_and_exclude_only_the_retiring_report() {
    expect_true(!policy::maintenance_path_required(false, false, false, false));
    expect_true(!policy::maintenance_path_required(false, true, false, false));
    expect_true(!policy::maintenance_path_required(false, false, true, false));
    expect_true(!policy::maintenance_path_required(false, true, true, false));
    expect_true(policy::maintenance_path_required(true, false, false, false));
    expect_true(policy::maintenance_path_required(true, true, false, false));
    expect_true(!policy::maintenance_path_required(true, false, true, false));
    expect_true(policy::maintenance_path_required(true, true, true, false));
    expect_true(!policy::maintenance_path_required(false, false, false, true));
    expect_true(!policy::maintenance_path_required(false, true, false, true));
    expect_true(!policy::maintenance_path_required(false, false, true, true));
    expect_true(!policy::maintenance_path_required(false, true, true, true));
    expect_true(policy::maintenance_path_required(true, false, false, true));
    expect_true(policy::maintenance_path_required(true, true, false, true));
    expect_true(!policy::maintenance_path_required(true, false, true, true));
    expect_true(!policy::maintenance_path_required(true, true, true, true));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn only_empty_retiring_reports_with_pending_edges_can_skip_path_exclusion() {
    expect_true(policy::metadata_only_retirement(true, true, true, true, true));
    expect_true(!policy::metadata_only_retirement(false, true, true, true, true));
    expect_true(!policy::metadata_only_retirement(true, false, true, true, true));
    expect_true(!policy::metadata_only_retirement(true, true, false, true, true));
    expect_true(!policy::metadata_only_retirement(true, true, true, false, true));
    expect_true(!policy::metadata_only_retirement(true, true, true, true, false));
    expect_true(!policy::metadata_only_retirement(false, false, false, false, false));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn metadata_path_readiness_preserves_empty_vacuity_and_the_retirement_guard()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    let empty = Vec::<String>::new();
    let loaded = std::collections::BTreeSet::<String>::new();
    expect_true(policy::metadata_path_ready(true, &empty, &loaded));
    expect_true(!policy::metadata_path_ready(false, &empty, &loaded));
    let pending = vec!["pending".to_owned()];
    expect_true(!policy::metadata_path_ready(false, &pending, &loaded));
    expect_true(!policy::metadata_path_ready(true, &pending, &loaded));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn metadata_path_readiness_rejects_missing_first_middle_and_last_references()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    let first = "first".to_owned();
    let middle = "middle".to_owned();
    let last = "last".to_owned();
    proof {
        reveal_strlit("first"); reveal_strlit("middle"); reveal_strlit("last");
        assert(first@.len() != middle@.len());
        assert(first@.len() != last@.len());
        assert(middle@.len() != last@.len());
    }
    let references = vec![first.clone(), middle.clone(), last.clone()];
    let mut suffix = std::collections::BTreeSet::new();
    suffix.insert(middle.clone());
    suffix.insert(last.clone());
    expect_true(!policy::metadata_path_ready(true, &references, &suffix));
    let mut edges = std::collections::BTreeSet::new();
    edges.insert(first.clone());
    edges.insert(last.clone());
    expect_true(!policy::metadata_path_ready(true, &references, &edges));
    let mut prefix = std::collections::BTreeSet::new();
    prefix.insert(first.clone());
    prefix.insert(middle.clone());
    expect_true(!policy::metadata_path_ready(true, &references, &prefix));
    suffix.insert(first.clone());
    expect_true(policy::metadata_path_ready(true, &references, &suffix));
    expect_true(!policy::metadata_path_ready(false, &references, &suffix));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn metadata_path_readiness_keeps_full_text_duplicates_and_unrelated_loaded_edges()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    let empty = "".to_owned();
    let embedded = "a\0\n".to_owned();
    let prefix = "a".to_owned();
    let unicode = "中é".to_owned();
    proof {
        reveal_strlit(""); reveal_strlit("a\0\n");
        reveal_strlit("a"); reveal_strlit("中é");
        assert(empty@.len() == 0); assert(embedded@.len() == 3);
        assert(prefix@.len() == 1); assert(unicode@.len() == 2);
    }
    let references = vec![unicode.clone(), embedded.clone(), empty.clone(),
        embedded.clone(), prefix.clone()];
    let mut loaded = std::collections::BTreeSet::new();
    loaded.insert(empty.clone());
    loaded.insert(embedded.clone());
    loaded.insert(unicode.clone());
    expect_true(!policy::metadata_path_ready(true, &references, &loaded));
    loaded.insert(prefix.clone());
    expect_true(policy::metadata_path_ready(true, &references, &loaded));
    loaded.insert("unrelated".to_owned());
    expect_true(policy::metadata_path_ready(true, &references, &loaded));
    let reordered = vec![prefix.clone(), empty.clone(), unicode.clone(), embedded.clone()];
    expect_true(policy::metadata_path_ready(true, &reordered, &loaded));
    let repeated = vec![embedded.clone(), embedded.clone(), embedded.clone()];
    expect_true(policy::metadata_path_ready(true, &repeated, &loaded));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn terminal_limits_keep_equal_bytes_and_reject_count_and_overflow() {
    let first = policy::terminal_budget(0, 0, 10, 1, 10);
    let full_count = policy::terminal_budget(1, 0, 1, 1, 10);
    let too_large = policy::terminal_budget(0, 0, 11, 1, 10);
    let full_bytes = policy::terminal_budget(0, 10, 1, 1, 10);
    let overflow = policy::terminal_budget(0, u64::MAX, 1, 2, u64::MAX);
    let zero = policy::terminal_budget(0, 0, 0, 0, 0);
    expect_true(first); expect_true(!full_count); expect_true(!too_large);
    expect_true(!full_bytes); expect_true(!overflow); expect_true(!zero);
}


fn expect_true(actual: bool)
    requires actual,
{
    if !actual { panic!(); }
}

fn expect_discovery(plan: policy::GateDiscovery, object: bool, role: bool, member: bool)
    requires plan.object_excluded == object, plan.role_excluded == role,
        plan.retain_member == member,
{
    expect_true(plan.object_excluded == object);
    expect_true(plan.role_excluded == role);
    expect_true(plan.retain_member == member);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn ordinary_discovery_excludes_before_reading_only_for_nonblocking_maintenance() {
    expect_discovery(policy::gate_discovery(false, false, false, false), false, false, false);
    expect_discovery(policy::gate_discovery(false, false, false, true), false, false, false);
    expect_discovery(policy::gate_discovery(false, false, true, false), false, false, false);
    expect_discovery(policy::gate_discovery(false, false, true, true), false, false, false);
    expect_discovery(policy::gate_discovery(false, true, false, false), true, true, false);
    expect_discovery(policy::gate_discovery(false, true, false, true), false, false, false);
    expect_discovery(policy::gate_discovery(false, true, true, false), false, false, false);
    expect_discovery(policy::gate_discovery(false, true, true, true), false, false, false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn observation_discovery_always_excludes_the_role_and_preserves_member_dependencies() {
    expect_discovery(policy::gate_discovery(true, false, false, false), false, true, true);
    expect_discovery(policy::gate_discovery(true, false, false, true), false, true, true);
    expect_discovery(policy::gate_discovery(true, false, true, false), false, true, true);
    expect_discovery(policy::gate_discovery(true, false, true, true), false, true, true);
    expect_discovery(policy::gate_discovery(true, true, false, false), true, true, true);
    expect_discovery(policy::gate_discovery(true, true, false, true), false, true, true);
    expect_discovery(policy::gate_discovery(true, true, true, false), false, true, true);
    expect_discovery(policy::gate_discovery(true, true, true, true), false, true, true);
}

#[cfg(not(verus_keep_ghost))]
#[test]
#[should_panic]
fn runtime_expectations_keep_their_failure_branch() {
    expect_true(false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn explicit_selection_is_nonempty_unique_and_bounded() {
    broadcast use vstd::array::group_array_axioms;
    proof { reveal_strlit("output-a"); reveal_strlit("output-b"); }
    let empty: Vec<String> = Vec::new();
    let one = vec!["output-a".to_owned()];
    let duplicate = vec!["output-a".to_owned(), "output-a".to_owned()];
    let two = vec!["output-a".to_owned(), "output-b".to_owned()];
    proof {
        assert("output-a"@[7] != "output-b"@[7]);
        assert(two[0]@ == "output-a"@ && two[1]@ == "output-b"@);
        assert forall|i: int, j: int| 0 <= i < j < two.len()
            implies two[i]@ != two[j]@ by { assert(i == 0 && j == 1); }
    }
    let e = policy::unique_selection(&empty, 512);
    let a = policy::unique_selection(&one, 1);
    let z = policy::unique_selection(&one, 0);
    let d = policy::unique_selection(&duplicate, 512);
    let b = policy::unique_selection(&two, 2);
    let over = policy::unique_selection(&two, 1);
    expect_true(!e); expect_true(a); expect_true(!z); expect_true(!d); expect_true(b); expect_true(!over);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn roles_are_complete_and_release_preserves_other_members() {
    broadcast use vstd::array::group_array_axioms;
    proof {
        reveal_strlit("recovery"); reveal_strlit("current");
        reveal_strlit("kept"); reveal_strlit("retired"); reveal_strlit("last");
        reveal_with_fuel(policy::removed, 4);
    }
    let roles = vec!["recovery".to_owned(), "current".to_owned()];
    let reversed = vec!["current".to_owned(), "recovery".to_owned()];
    let incomplete = vec!["recovery".to_owned()];
    proof {
        assert(roles[0]@ == "recovery"@ && roles[1]@ == "current"@);
        assert(incomplete[0]@ == "recovery"@);
        assert(policy::present(roles@, "current"@));
        assert(!policy::present(incomplete@, "current"@));
    }
    let complete = policy::same_members(&roles, &reversed);
    let short = policy::same_members(&roles, &incomplete);
    let extra = policy::same_members(&incomplete, &roles);
    expect_true(complete); expect_true(!short); expect_true(!extra);
    let members = vec!["kept".to_owned(), "retired".to_owned(), "last".to_owned()];
    let remaining = policy::without(&members, "retired");
    proof {
        reveal_strlit("kept"); reveal_strlit("retired"); reveal_strlit("last");
        assert("kept"@[0] != "retired"@[0] && "last"@[0] != "retired"@[0]);
        assert(members[0]@ == "kept"@ && members[1]@ == "retired"@ && members[2]@ == "last"@);
        reveal_with_fuel(policy::removed, 4);
        assert(members@.take(1).drop_last() =~= Seq::<String>::empty());
        assert(members@.take(1).last() == members@[0]);
        assert(members@.take(2).drop_last() =~= members@.take(1));
        assert(members@.drop_last() =~= members@.take(2));
        assert("kept"@ != "retired"@ && "last"@ != "retired"@);
        assert(policy::removed(Seq::<String>::empty(), "retired"@) =~= Seq::<String>::empty());
        assert(policy::removed(members@.take(1), "retired"@)
            == policy::removed(members@.take(1).drop_last(), "retired"@).push(members@.take(1).last()));
        assert(policy::removed(members@.take(1), "retired"@) =~= seq![members@[0]]);
        assert(policy::removed(members@.take(2), "retired"@) =~= seq![members@[0]]);
        assert(remaining@ =~= seq![members@[0], members@[2]]);
    }
    expect_true(remaining.len() == 2);
    assert(remaining[0]@ == "kept"@ && remaining[1]@ == "last"@);
    let kept = policy::contains(&remaining,"kept");
    let last = policy::contains(&remaining,"last");
    expect_true(kept && last);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn release_requires_closed_users_and_keeps_original_size_boundary() {
    broadcast use vstd::array::group_array_axioms;
    proof {
        reveal_strlit("recovery"); reveal_strlit("preserved"); reveal_strlit("");
        vstd::string::is_ascii_spec_bytes("recovery");
    }
    let roles = vec!["recovery".to_owned()];
    proof {
        assert(roles[0]@ == "recovery"@);
        assert forall|i: int| 0 <= i < "recovery"@.len()
            implies ("recovery"@[i] as u32) < 128 by {
            assert(i == 0 || i == 1 || i == 2 || i == 3
                || i == 4 || i == 5 || i == 6 || i == 7);
        }
        assert(vstd::string::is_ascii("recovery"));
        vstd::string::is_ascii_spec_bytes("recovery");
        assert(vstd::utf8::encode_utf8("recovery"@).len() == 8);
        assert(policy::bounded_roles(roles@));
    }
    let accepted = policy::valid_request(&roles, 1, "preserved", 4096, true, 1024, 1024);
    let live = policy::valid_request(&roles, 1, "preserved", 4096, false, 1024, 1024);
    let empty = policy::valid_request(&roles, 1, "", 0, true, 1024, 1024);
    let reason = policy::valid_request(&roles, 1, "preserved", 4097, true, 1024, 1024);
    let size = policy::valid_request(&roles, 1, "preserved", 4096, true, 1025, 1024);
    let bad_roles = vec!["".to_owned()];
    let bad = policy::valid_request(&bad_roles, 1, "preserved", 9, true, 0, 0);
    expect_true(accepted); expect_true(!live); expect_true(!empty); expect_true(!reason); expect_true(!size); expect_true(!bad);
    let global = policy::global_maintenance(None);
    let selected = vec!["output-a".to_owned()];
    let scoped = policy::global_maintenance(Some(&selected));
    expect_true(global && !scoped);
}


#[cfg_attr(not(verus_keep_ghost), test)]
fn sweep_observation_capacity_accepts_exact_fit_and_rejects_overflow() {
    expect_true(policy::observation_bytes_fit(0, 0, 0));
    expect_true(policy::observation_bytes_fit(0, 65536, 65536));
    expect_true(policy::observation_bytes_fit(65535, 1, 65536));
    expect_true(!policy::observation_bytes_fit(65536, 1, 65536));
    expect_true(!policy::observation_bytes_fit(1, 0, 0));
    expect_true(policy::observation_bytes_fit(u64::MAX, 0, u64::MAX));
    expect_true(!policy::observation_bytes_fit(u64::MAX, 1, u64::MAX));
    expect_true(!policy::observation_bytes_fit(1, u64::MAX, u64::MAX));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn object_order_and_returned_command_have_independent_boundaries() {
    let reserve = policy::observation_owner(false,true,true);
    let ordinary = policy::observation_owner(false,false,true);
    let unsealed = policy::observation_result(true,false,true,true,true);
    let replaced = policy::observation_result(true,true,true,true,false);
    let live = policy::settled_pr_command(true,false,true);
    let create = policy::settled_pr_command(false,true,true);
    let storage = policy::settled_pr_command(false,false,false);
    let failed_read = policy::settled_pr_command(false,false,true);
    expect_true(reserve && !ordinary && !unsealed && !replaced);
    expect_true(!live && !create && !storage && failed_read);
    let first = policy::next_observation(0);
    let last = policy::next_observation(u64::MAX);
    expect_true(first.is_some()); expect_true(first.unwrap() == 1); expect_true(last.is_none());
    let pending_wrong_epoch = policy::observation_epoch(true,false,1,3);
    let completed_new_epoch = policy::observation_epoch(false,false,9,1);
    expect_true(!pending_wrong_epoch && completed_new_epoch);
    let intact = policy::observation_order(64,3,2,true,true);
    let future = policy::observation_order(64,3,4,true,true);
    let absent = policy::observation_order(64,3,2,true,false);
    expect_true(intact && !future && !absent);
    let ordinary_edit = policy::observation_edit(true,false,false);
    let owner_edit = policy::observation_edit(true,true,false);
    expect_true(!ordinary_edit && owner_edit);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn actual_pr_facts_do_not_certify_a_failed_or_unverified_command() {
    proof { reveal_strlit("OPEN"); reveal_strlit("CLOSED"); reveal_strlit("MERGED"); reveal_strlit("unknown"); reveal_strlit("pr"); reveal_strlit("create"); reveal_strlit("checks"); }
    proof {
        assert("OPEN"@[0] != "CLOSED"@[0] && "OPEN"@[0] != "MERGED"@[0]);
        assert("unknown"@[0] != "OPEN"@[0] && "unknown"@[0] != "CLOSED"@[0] && "unknown"@[0] != "MERGED"@[0]);
        assert("create"@[1] != "checks"@[1]);
    }
    let open = policy::open_pr("OPEN");
    let closed = policy::observed_pr("CLOSED",false);
    let merged = policy::observed_pr("MERGED",false);
    let wrong = policy::observed_pr("unknown",false);
    let contradictory = policy::observed_pr("OPEN",false);
    expect_true(open && closed && merged && !wrong && !contradictory);
    let create = policy::creating_pr("pr","create");
    let query = policy::creating_pr("pr","checks");
    expect_true(create && !query);
    let failed = policy::continued_pr_handoff(true,false,true,true);
    let stale = policy::continued_pr_handoff(true,true,true,false);
    let passed = policy::continued_pr_handoff(true,true,true,true);
    expect_true(!failed && !stale && passed);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn older_observation_preserves_current_and_every_unrelated_pending_member() {
    broadcast use vstd::array::group_array_axioms;
    proof { reveal_strlit("unknown"); reveal_strlit("current"); reveal_strlit("request"); }
    let members = vec!["unknown".to_owned(),"current".to_owned(),"request".to_owned()];
    let id = "request".to_owned();
    let current = Some("current".to_owned());
    let old = policy::settle_observation_members(&members,&id,&current,9,Some((8,true)));
    let terminal = policy::settle_observation_members(&members,&id,&current,9,Some((10,false)));
    let active = policy::settle_observation_members(&members,&id,&current,9,Some((10,true)));
    proof {
        reveal_with_fuel(policy::removed,5);
        assert(members[0]@ == "unknown"@ && members[1]@ == "current"@ && members[2]@ == "request"@);
        assert("unknown"@[0] != "request"@[0] && "current"@[0] != "request"@[0]);
        assert("unknown"@[0] != "current"@[0] && "request"@[0] != "current"@[0]);
    }
    assert(old.accepted == 9 && policy::current_value(old.current) == Some("current"@));
    assert(terminal.accepted == 10 && terminal.current.is_none());
    assert(active.accepted == 10 && policy::current_value(active.current) == Some("request"@));
    expect_true(old.accepted == 9 && old.current.is_some());
    expect_true(terminal.accepted == 10 && terminal.current.is_none());
    expect_true(active.accepted == 10 && active.current.is_some());
    proof {
        assert(old.members@ =~= seq![members[0], members[1]]);
        assert(terminal.members@ =~= seq![members[0]]);
        assert(active.members@ =~= seq![members[0], id]);
    }
    let old_unknown = policy::contains(&old.members,"unknown");
    let old_current = policy::contains(&old.members,"current");
    let old_request = policy::contains(&old.members,"request");
    let closed_unknown = policy::contains(&terminal.members,"unknown");
    let closed_current = policy::contains(&terminal.members,"current");
    let active_request = policy::contains(&active.members,"request");
    expect_true(old_unknown && old_current && !old_request);
    expect_true(closed_unknown && !closed_current && active_request);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn observation_binding_requires_the_actual_role_output_epoch_and_nonzero_attempt() {
    proof {
        reveal_strlit("role"); reveal_strlit("else"); reveal_strlit("");
        assert("role"@[0] != "else"@[0]); assert("role"@.len() == 4 && ""@.len() == 0);
    }
    let request = policy::ObservationBinding { role:"role", epoch:"role", id:"role",
        path:"role", physical:"role", serial:9, attempt:3 };
    let correct = policy::ObservationBinding { role:"role", epoch:"role", id:"role",
        path:"role", physical:"role", serial:8, attempt:2 };
    let other_role = policy::ObservationBinding { role:"else", ..correct };
    let other_epoch = policy::ObservationBinding { epoch:"else", ..correct };
    let other_id = policy::ObservationBinding { id:"else", ..correct };
    let other_path = policy::ObservationBinding { path:"else", ..correct };
    let replaced = policy::ObservationBinding { physical:"else", ..correct };
    let future = policy::ObservationBinding { serial:10, ..correct };
    let zero = policy::ObservationBinding { attempt:0, ..correct };
    let match_ok = policy::matching_observation(&request,&correct);
    let role = policy::matching_observation(&request,&other_role);
    let epoch = policy::matching_observation(&request,&other_epoch);
    let id = policy::matching_observation(&request,&other_id);
    let path = policy::matching_observation(&request,&other_path);
    let physical = policy::matching_observation(&request,&replaced);
    let newer = policy::matching_observation(&request,&future);
    let invalid_attempt = policy::matching_observation(&request,&zero);
    expect_true(match_ok && !role && !epoch && !id && !path && !physical && !newer && !invalid_attempt);
    let maximum = policy::observation_role("role",256,1);
    let too_long = policy::observation_role("role",257,1);
    let blank = policy::observation_role("",0,1);
    let no_attempt = policy::observation_token("role",4,1,0);
    let no_serial = policy::observation_token("role",4,0,1);
    expect_true(maximum && !too_long && !blank && !no_attempt && !no_serial);
    let continued = policy::continued_observation(&correct,"role","role");
    let rebound = policy::continued_observation(&correct,"role","else");
    expect_true(continued && !rebound);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn absent_or_conflicting_native_exit_does_not_complete_the_operation() {
    let success = policy::process_exit_known(0,None);
    let failure = policy::process_exit_known(17,None);
    let largest_unix_exit = policy::process_exit_known(255,None);
    let killed = policy::process_exit_known(-1,Some(9));
    let terminated = policy::process_exit_known(-1,Some(15));
    let missing = policy::process_exit_known(-1,None);
    let invalid_code = policy::process_exit_known(-2,None);
    let conflicting = policy::process_exit_known(0,Some(9));
    let zero_signal = policy::process_exit_known(-1,Some(0));
    let invalid_signal = policy::process_exit_known(-1,Some(-1));
    expect_true(success && failure && largest_unix_exit && killed && terminated);
    expect_true(!missing && !invalid_code && !conflicting && !zero_signal && !invalid_signal);
    let unknown = policy::unknown_pr_operation(!missing,false);
    let settled = policy::settled_pr_command(!missing,false,true);
    expect_true(unknown && !settled);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn direct_exit_requires_joined_owned_work_before_recovery_can_release() {
    let success = policy::process_completion_known(0,None,true);
    let failure = policy::process_completion_known(17,None,true);
    let killed = policy::process_completion_known(-1,Some(9),true);
    let success_unjoined = policy::process_completion_known(0,None,false);
    let failure_unjoined = policy::process_completion_known(17,None,false);
    let killed_unjoined = policy::process_completion_known(-1,Some(9),false);
    let missing = policy::process_completion_known(-1,None,true);
    let conflicting = policy::process_completion_known(0,Some(9),true);
    let invalid_signal = policy::process_completion_known(-1,Some(0),true);
    expect_true(success && failure && killed);
    expect_true(!success_unjoined && !failure_unjoined && !killed_unjoined);
    expect_true(!missing && !conflicting && !invalid_signal);
    let unknown = policy::unknown_pr_operation(!success_unjoined,false);
    let settled = policy::settled_pr_command(!success_unjoined,false,true);
    expect_true(unknown && !settled);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn replay_or_missing_observation_keeps_the_current_original() {
    broadcast use vstd::array::group_array_axioms;
    proof { reveal_strlit("current"); reveal_strlit("unknown"); assert("current"@[0] != "unknown"@[0]); }
    let id = "current".to_owned();
    let members = vec!["unknown".to_owned(),id.clone()];
    let current = Some(id.clone());
    let replay = policy::settle_observation_members(&members,&id,&current,9,Some((9,false)));
    let failed = policy::settle_observation_members(&members,&id,&current,9,None);
    assert(replay.members@ == members@ && failed.members@ == members@);
    expect_true(replay.accepted == 9 && failed.accepted == 9);
    assert(policy::current_value(replay.current) == Some(id@));
    assert(policy::current_value(failed.current) == Some(id@));
    let replay_unknown = policy::contains(&replay.members,"unknown");
    let replay_current = policy::contains(&replay.members,"current");
    let failed_unknown = policy::contains(&failed.members,"unknown");
    let failed_current = policy::contains(&failed.members,"current");
    expect_true(replay.members.len() == 2 && replay_unknown && replay_current);
    expect_true(failed.members.len() == 2 && failed_unknown && failed_current);
    expect_true(replay.current.is_some() && failed.current.is_some());
    let producer = policy::observation_producer(true,true,true);
    let unreturned = policy::unknown_pr_operation(true,false);
    let unverified_create = policy::unknown_pr_operation(false,true);
    let complete = policy::unknown_pr_operation(false,false);
    expect_true(producer && unreturned && unverified_create && !complete);
    let valid_state = policy::object_state("chrono-pr-object-state/v1",false);
    let unknown_write = policy::object_state("chrono-pr-object-state/v1",true);
    expect_true(valid_state && !unknown_write);
}

}
