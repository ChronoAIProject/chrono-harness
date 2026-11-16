//! 独立边界预期同时由 Verus 和真实生产实现核验。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../worktree/src/verified_lifecycle.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(not(verus_keep_ghost))]
use chrono_worktree::verified_lifecycle as policy;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

#[cfg(verus_keep_ghost)]
#[path = "automatic.rs"]
mod automatic_boundaries;

#[cfg(verus_keep_ghost)]
#[path = "complete_inventory.rs"]
mod inventory_boundaries;
#[cfg(verus_keep_ghost)]
#[path = "unified_maintenance.rs"]
mod maintenance_boundaries;

#[cfg(verus_keep_ghost)]
#[path = "temporary.rs"]
#[rustfmt::skip]
mod rolling_boundaries;

#[cfg(verus_keep_ghost)]
#[path = "age.rs"]
#[rustfmt::skip]
mod source_boundaries_0;

#[cfg(verus_keep_ghost)]
#[path = "bounded_cleanup.rs"]
#[rustfmt::skip]
mod source_boundaries_1;

#[cfg(verus_keep_ghost)]
#[path = "creation.rs"]
#[rustfmt::skip]
mod source_boundaries_2;

#[cfg(verus_keep_ghost)]
#[path = "interrupted_cleanup.rs"]
#[rustfmt::skip]
mod source_boundaries_3;

#[cfg(verus_keep_ghost)]
#[path = "policy_migration.rs"]
#[rustfmt::skip]
mod source_boundaries_4;

#[cfg(verus_keep_ghost)]
#[path = "../../ci-tests/tests/behavior.rs"]
#[rustfmt::skip]
mod source_boundaries_5;

#[cfg(verus_keep_ghost)]
#[path = "fixture_body.rs"]
#[rustfmt::skip]
mod fixture_body_boundaries;

#[cfg(verus_keep_ghost)]
#[path = "support/git_fault.rs"]
#[rustfmt::skip]
mod inventory_failure_owner;

verus! {

use policy::ProducerDeclarations;

pub struct DeclarationRows<'a> { pub rows: Vec<(&'a str, bool)> }

impl<'a> policy::ProducerDeclarations for DeclarationRows<'a> {
    open spec fn count(&self) -> nat { self.rows@.len() }
    open spec fn name_at(&self, index: int) -> &str { self.rows@[index].0 }
    open spec fn temporary_at(&self, index: int) -> bool { self.rows@[index].1 }
    fn len(&self) -> (out: usize)
        ensures out == self.count(),
    { self.rows.len() }
    fn name(&self, index: usize) -> (out: &str)
        ensures out == self.name_at(index as int),
    { self.rows[index].0 }
    fn temporary(&self, index: usize) -> (out: bool)
        ensures out == self.temporary_at(index as int),
    { self.rows[index].1 }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn producer_lookup_preserves_first_match_and_temporary_admission() {
    proof {
        reveal_strlit("fixture λ\0\n"); reveal_strlit("");
        assert("fixture λ\0\n"@.len() != ""@.len());
    }
    let mut entries = Vec::new();
    entries.push(("fixture λ\0\n", false));
    entries.push(("fixture λ\0\n", true));
    entries.push(("", true));
    let rows = DeclarationRows { rows: entries };
    assert(rows.count() == 3);
    assert(rows.name_at(0)@ == "fixture λ\0\n"@ && !rows.temporary_at(0));
    match policy::find_producer_declaration(&rows, "fixture λ\0\n", false) {
        Ok(index) => expect(index == 0), _ => expect(false),
    }
    match policy::find_producer_declaration(&rows, "fixture λ\0\n", true) {
        Err(policy::ProducerLookupError::NotTemporary(index)) => {
            expect(index == 0);
            expect(policy::equal_text(policy::ProducerLookupError::NotTemporary(index).message(),
                "temporary producer is not prospectively adopted"));
        },
        _ => expect(false),
    }
    match policy::find_producer_declaration(&rows, "", true) {
        Ok(index) => expect(index == 2),
        Err(policy::ProducerLookupError::Missing) => {
            assert(rows.name_at(2)@ == ""@);
            expect(false);
        },
        Err(policy::ProducerLookupError::NotTemporary(index)) => {
            assert(index < 3);
            if index < 2 { assert(rows.name_at(index as int)@ != ""@); }
            else { assert(rows.temporary_at(2)); }
            expect(false);
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn producer_lookup_rejects_absence_and_nearby_full_names() {
    proof {
        reveal_strlit("fixture"); reveal_strlit("fixture-extra"); reveal_strlit("Fixture");
        assert("fixture"@.len() != "fixture-extra"@.len());
        assert("fixture"@[0] != "Fixture"@[0]);
    }
    let rows = DeclarationRows { rows: Vec::new() };
    match policy::find_producer_declaration(&rows, "", false) {
        Err(policy::ProducerLookupError::Missing) => expect(policy::equal_text(
            policy::ProducerLookupError::Missing.message(), "retained producer store is not declared by its owner")),
        _ => expect(false),
    }
    let mut entries = Vec::new();
    entries.push(("fixture-extra", true));
    entries.push(("Fixture", true));
    let rows = DeclarationRows { rows: entries };
    match policy::find_producer_declaration(&rows, "fixture", true) {
        Err(policy::ProducerLookupError::Missing) => expect(policy::equal_text(
            policy::ProducerLookupError::Missing.message(), "retained producer store is not declared by its owner")),
        _ => expect(false),
    }
}

proof fn producer_trace_prefix_step<S, T, E, F>(rows: Seq<T>, states: Seq<S>, initial: S,
    visit: &F, index: int)
    where F: Fn(S, usize, &T) -> (S, Result<bool, E>),
    requires policy::producer_visit_prefix(rows, states, initial, visit),
        0 <= index < states.len() - 1,
    ensures visit.ensures((states[index], index as usize, &rows[index]),
        (states[index + 1], Ok::<bool, E>(true))),
{
    reveal(policy::producer_visit_prefix);
}

fn producer_visit_case(stop: usize, failure: bool)
    requires stop <= 2,
{
    let mut rows = Vec::new();
    rows.push(17u8); rows.push(239u8);
    let visit = |state: Vec<(usize, u8)>, index: usize, row: &u8|
        -> (out: (Vec<(usize, u8)>, Result<bool, (u32, u64)>))
        ensures out.0@ == state@.push((index, *row)),
            out.1 == if index == stop {
                if failure { Err::<bool, (u32, u64)>((u32::MAX, u64::MAX)) }
                else { Ok::<bool, (u32, u64)>(false) }
            } else { Ok::<bool, (u32, u64)>(true) },
    {
        let mut next = state;
        next.push((index, *row));
        let result = if index == stop {
            if failure { Err((u32::MAX, u64::MAX)) } else { Ok(false) }
        } else { Ok(true) };
        (next, result)
    };
    let initial = Vec::new();
    let ghost original = initial;
    let (trace, result) = policy::visit_producer_stores(initial, &rows, &visit);
    proof {
        reveal(policy::producer_visit_prefix);
        let states = choose|states: Seq<Vec<(usize, u8)>>|
            policy::producer_visit_prefix(rows@, states, original, &visit)
                && match result {
                    Ok(true) => states.len() == rows.len() + 1 && trace == states.last(),
                    _ => states.len() <= rows.len() && visit.ensures(
                        (states.last(), (states.len() - 1) as usize,
                            &rows@[states.len() - 1]), (trace, result)),
                };
        assert(policy::producer_visit_prefix(rows@, states, original, &visit));
        assert(forall|j: int| 0 <= j < states.len() - 1 ==> #[trigger]
            visit.ensures((states[j], j as usize, &rows@[j]),
                (states[j + 1], Ok::<bool, (u32, u64)>(true))));
        assert(1 <= states.len() <= 3);
        assert(states[0]@ == Seq::<(usize, u8)>::empty());
        if states.len() >= 2 {
            producer_trace_prefix_step(rows@, states, original, &visit, 0);
            assert((&visit).ensures((states[0], 0usize, &rows@[0]),
                (states[1], Ok::<bool, (u32, u64)>(true))));
            assert(stop != 0);
            assert(states[1]@ == seq![(0usize, 17u8)]);
        }
        if states.len() == 3 {
            producer_trace_prefix_step(rows@, states, original, &visit, 1);
            assert((&visit).ensures((states[1], 1usize, &rows@[1]),
                (states[2], Ok::<bool, (u32, u64)>(true))));
            assert(stop == 2);
            assert(states[2]@ == seq![(0usize, 17u8), (1usize, 239u8)]);
        }
        match result {
            Ok(true) => {
                assert(states.len() == 3 && stop == 2);
                assert(trace@ == seq![(0usize, 17u8), (1usize, 239u8)]);
            },
            _ => {
                let index = states.len() - 1;
                assert(visit.ensures((states[index], index as usize, &rows@[index]),
                    (trace, result)));
                assert(index == stop);
                if index == 0 { assert(trace@ == seq![(0usize, 17u8)]); }
                else { assert(trace@ == seq![(0usize, 17u8), (1usize, 239u8)]); }
            },
        }
    }
    expect(trace.len() == if stop == 0 { 1 } else { 2 });
    expect(trace[0].0 == 0 && trace[0].1 == 17);
    if stop != 0 { expect(trace[1].0 == 1 && trace[1].1 == 239); }
    match result {
        Ok(value) => { expect(!failure || stop == 2); expect(value == (stop == 2)); },
        Err(error) => { expect(failure && stop < 2); expect(error.0 == u32::MAX && error.1 == u64::MAX); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn producer_visits_stop_at_the_actual_false_or_original_error() {
    producer_visit_case(0, false); producer_visit_case(1, false);
    producer_visit_case(0, true); producer_visit_case(1, true);
    producer_visit_case(2, false); producer_visit_case(2, true);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn empty_producer_visit_does_not_observe_any_store() {
    let rows: Vec<u8> = Vec::new();
    let visit = |state: u8, _index: usize, _row: &u8| -> (out: (u8, Result<bool, u8>))
        ensures out == (255u8, Err::<bool, u8>(state)),
    { (255, Err(state)) };
    let (state, result) = policy::visit_producer_stores(17u8, &rows, &visit);
    expect(state == 17);
    match result { Ok(value) => expect(value), _ => expect(false), }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn single_store_adapter_preserves_value_and_full_missing_error() {
    let mut stores = Vec::new();
    stores.push((u32::MAX, u64::MAX));
    match policy::take_resolved_store(stores) {
        Ok(value) => expect(value.0 == u32::MAX && value.1 == u64::MAX), _ => expect(false),
    }
    let stores: Vec<u8> = Vec::new();
    match policy::take_resolved_store(stores) {
        Err(error) => expect(policy::equal_text(error, "retained producer store was not resolved")),
        _ => expect(false),
    }
}

fn report_scope_case(operation: &str, publication: Option<u8>,
    result: Result<(u8, u8), (u32, u64)>, expected: u8)
    requires expected <= 3,
        expected == 0 ==> operation@ == "maintain"@ && publication.is_none(),
        expected == 1 ==> operation@ == "maintain"@ && publication.is_some(),
        expected == 2 ==> operation@ != "maintain"@ && publication.is_some(),
        expected == 3 ==> operation@ != "maintain"@ && publication.is_none(),
{
    let outputs = |state: (u8, u8, u8), publication: u8|
        -> (out: ((u8, u8, u8), Result<(u8, u8), (u32, u64)>))
        ensures out.0 == (state.0, 1u8, publication), out.1 == result,
    { ((state.0, 1u8, publication), result) };
    let inputs = |state: (u8, u8, u8), publication: u8|
        -> (out: ((u8, u8, u8), Result<(u8, u8), (u32, u64)>))
        ensures out.0 == (state.0, 2u8, publication), out.1 == result,
    { ((state.0, 2u8, publication), result) };
    let caller = |state: (u8, u8, u8)|
        -> (out: ((u8, u8, u8), Result<(u8, u8), (u32, u64)>))
        ensures out.0 == (state.0, 3u8, 0u8), out.1 == result,
    { ((state.0, 3u8, 0u8), result) };
    let (state, actual) = policy::select_report_input_scope(
        (91u8, 0u8, 0u8), operation, publication, outputs, inputs, caller);
    expect(state.0 == 91);
    expect(state.1 == expected);
    match publication {
        Some(publication) => expect(state.2 == publication),
        None => expect(state.2 == 0),
    }
    if expected == 0 {
        match actual {
            Ok(None) => {},
            _ => expect(false),
        }
    } else {
        match (result, actual) {
            (Ok(scope), Ok(Some(actual))) => {
                expect(actual.0 == scope.0);
                expect(actual.1 == scope.1);
            },
            (Err(error), Err(actual)) => {
                expect(actual.0 == error.0);
                expect(actual.1 == error.1);
            },
            _ => expect(false),
        }
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn maintenance_retains_published_outputs_and_preserves_their_original_result() {
    report_scope_case("maintain", Some(0), Ok((17, 39)), 1);
    report_scope_case("maintain", Some(255), Err((u32::MAX, u64::MAX)), 1);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn unpublished_maintenance_calls_no_input_or_output_scope_even_on_callback_error() {
    report_scope_case("maintain", None, Ok((17, 39)), 0);
    report_scope_case("maintain", None, Err((u32::MAX, u64::MAX)), 0);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn ordinary_published_activity_retains_only_its_publication_inputs() {
    proof {
        reveal_strlit("use"); reveal_strlit("bootstrap"); reveal_strlit("maintain");
        assert("use"@.len() != "maintain"@.len());
        assert("bootstrap"@.len() != "maintain"@.len());
    }
    report_scope_case("use", Some(0), Ok((17, 39)), 2);
    report_scope_case("bootstrap", Some(255), Err((u32::MAX, u64::MAX)), 2);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn ordinary_unpublished_activity_retains_the_original_caller_inputs() {
    proof {
        reveal_strlit("maintain"); reveal_strlit(""); reveal_strlit("maintain\0");
        reveal_strlit("Maintain"); reveal_strlit("维护");
        assert(""@.len() != "maintain"@.len());
        assert("maintain\0"@.len() != "maintain"@.len());
        assert("Maintain"@[0] != "maintain"@[0]);
        assert("维护"@.len() != "maintain"@.len());
    }
    report_scope_case("", None, Ok((17, 39)), 3);
    report_scope_case("maintain\0", None, Err((u32::MAX, u64::MAX)), 3);
    report_scope_case("Maintain", None, Ok((255, 0)), 3);
    report_scope_case("维护", None, Err((0, 0)), 3);
}




pub(crate) fn expect(actual: bool)
    requires actual,
{
    if !actual { panic!(); }
}

fn expect_source(actual: policy::AdoptedPolicySource, expected: policy::AdoptedPolicySource)
    requires actual == expected,
{
    use policy::AdoptedPolicySource::*;
    expect(match (actual, expected) {
        (ImportedOriginal, ImportedOriginal) | (Current, Current)
        | (RetainedTarget, RetainedTarget) => true,
        _ => false,
    });
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn imported_original_precedes_target_binding_and_explicit_migration_supersedes_it() {
    broadcast use vstd::string::group_string_axioms;
    proof {
        reveal_strlit("import"); reveal_strlit("birth"); reveal_strlit("import\0");
        reveal_strlit("old"); reveal_strlit("new");
        assert("old"@[0] != "new"@[0]);
        assert("import"@.len() == 6 && "birth"@.len() == 5 && "import\0"@.len() == 7);
    }
    expect(policy::imported_policy_required("import", false));
    expect(!policy::imported_policy_required("import", true));
    expect(!policy::imported_policy_required("birth", false));
    expect(!policy::imported_policy_required("import\0", false));
    use policy::AdoptedPolicySource::*;
    expect_source(policy::adopted_policy_source("import", false, "old", "old", "old", "old"), ImportedOriginal);
    expect_source(policy::adopted_policy_source("import", false, "old", "new", "old", "new"), ImportedOriginal);
    expect_source(policy::adopted_policy_source("import", true, "old", "old", "old", "old"), Current);
    expect_source(policy::adopted_policy_source("import", true, "old", "new", "old", "old"), RetainedTarget);
    expect_source(policy::adopted_policy_source("import", true, "old", "old", "old", "new"), RetainedTarget);
    expect_source(policy::adopted_policy_source("birth", false, "old", "old", "old", "old"), Current);
    expect_source(policy::adopted_policy_source("birth", false, "old", "new", "old", "old"), RetainedTarget);
    expect_source(policy::adopted_policy_source("birth", true, "old", "old", "old", "new"), RetainedTarget);
    expect_source(policy::adopted_policy_source("import\0", false, "old", "old", "old", "old"), Current);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn present_import_restoration_requires_explicit_maintenance_and_managed_originals() {
    expect(policy::restore_present_import(true, true, true));
    expect(!policy::restore_present_import(false, true, true));
    expect(!policy::restore_present_import(true, false, true));
    expect(!policy::restore_present_import(true, true, false));
    expect(!policy::restore_present_import(false, false, false));
}




#[cfg_attr(not(verus_keep_ghost), test)]
fn terminal_linked_work_releases_reusable_outputs_without_a_main_release_request() {
    expect(policy::reusable_release(true, false, 0, None));
    expect(policy::reusable_release(true, false, 7, None));
    expect(policy::reusable_release(true, false, u64::MAX, None));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn qualified_expiry_releases_reusable_outputs_without_finishing_work() {
    expect(policy::reusable_release(false, true, 0, None));
    expect(policy::reusable_release(false, true, 7, None));
    expect(policy::reusable_release(false, true, 8, Some(7)));
    expect(policy::reusable_release(false, true, u64::MAX, None));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn retained_reference_recovery_queries_only_permitted_advances() {
    match policy::retained_reference_ancestor_required(true, false) {
        Ok(query) => expect(!query),
        Err(_) => expect(false),
    }
    match policy::retained_reference_ancestor_required(true, true) {
        Ok(query) => expect(!query),
        Err(_) => expect(false),
    }
    match policy::retained_reference_ancestor_required(false, true) {
        Ok(query) => expect(query),
        Err(_) => expect(false),
    }
    expect(policy::retained_reference_ancestor_required(false, false).is_err());
}

pub(crate) fn expect_stage(actual: policy::CleanupStage, expected: policy::CleanupStage)
    requires actual == expected,
{
    use policy::CleanupStage::*;
    let equal = match (actual, expected) {
        (Pending, Pending) | (InProgress, InProgress)
        | (WaitingForConsumers, WaitingForConsumers) | (Retained, Retained)
        | (Failed, Failed) | (CachesReleased, CachesReleased) | (Disposed, Disposed) => true,
        _ => false,
    };
    expect(equal);
}

fn expect_cleanup_outcome(out: policy::CleanupOutcome, cache: bool, cleaned: bool,
    stage: policy::CleanupStage)
    requires out.cache == cache, out.cleaned == cleaned, out.stage == stage,
{
    expect(out.cache == cache);
    expect(out.cleaned == cleaned);
    expect_stage(out.stage, stage);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn terminal_cache_results_preserve_completion_and_actual_removal_priority() {
    use policy::CleanupStage::*;
    broadcast use vstd::string::group_string_axioms;
    proof {
        reveal_strlit("active"); reveal_strlit("terminal"); reveal_strlit("cleaned");
        reveal_strlit("failed"); reveal_strlit("verified-absent"); reveal_strlit("already-absent");
        reveal_strlit("preserved"); reveal_strlit("");
        reveal_strlit("reusable-source-preserved");
        reveal_strlit("no-disposable-output-retained-checkout");
        reveal_strlit("expired-checkout-verified-absent-source-branches-retained");
        assert("active"@.len() != "terminal"@.len());
        assert("failed"@.len() != "cleaned"@.len());
        assert("preserved"@.len() != "verified-absent"@.len());
        assert("preserved"@.len() != "already-absent"@.len());
        assert("reusable-source-preserved"@.len()
            != "expired-checkout-verified-absent-source-branches-retained"@.len());
    }
    let terminal = policy::observed_cleanup_result(false, "terminal",
        Some("reusable-source-preserved"), Some("cleaned"), Some("preserved"));
    expect_cleanup_outcome(terminal, true, true, Retained);
    let no_effect = policy::observed_cleanup_result(false, "active",
        Some("no-disposable-output-retained-checkout"), Some("cleaned"), Some("preserved"));
    expect_cleanup_outcome(no_effect, true, true, CachesReleased);
    let no_effect_terminal = policy::observed_cleanup_result(false, "terminal",
        Some("no-disposable-output-retained-checkout"), Some("cleaned"), Some("preserved"));
    expect_cleanup_outcome(no_effect_terminal, true, true, Retained);
    let pending = policy::observed_cleanup_result(true, "active",
        Some("reusable-source-preserved"), Some("cleaned"), Some("verified-absent"));
    expect_cleanup_outcome(pending, true, true, CachesReleased);
    let removed = policy::observed_cleanup_result(true, "terminal",
        Some("reusable-source-preserved"), Some("cleaned"), Some("verified-absent"));
    expect_cleanup_outcome(removed, true, true, Disposed);
    let already_absent = policy::observed_cleanup_result(false, "terminal",
        Some("reusable-source-preserved"), Some("cleaned"), Some("already-absent"));
    expect_cleanup_outcome(already_absent, true, true, Disposed);
    let failed = policy::observed_cleanup_result(true, "terminal",
        Some("reusable-source-preserved"), Some("failed"), Some("verified-absent"));
    expect_cleanup_outcome(failed, true, false, Failed);
    let expired = policy::observed_cleanup_result(true, "active",
        Some("expired-checkout-verified-absent-source-branches-retained"), Some("cleaned"),
        Some("verified-absent"));
    expect_cleanup_outcome(expired, false, true, Disposed);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn cleanup_observations_keep_missing_unknown_and_complete_text_distinct() {
    use policy::CleanupStage::*;
    broadcast use vstd::string::group_string_axioms;
    proof {
        reveal_strlit(""); reveal_strlit("terminal"); reveal_strlit("cleaned");
        reveal_strlit("verified-absent"); reveal_strlit("already-absent");
        reveal_strlit("reusable-source-preserved");
        reveal_strlit("no-disposable-output-retained-checkout");
        reveal_strlit("expired-checkout-verified-absent-source-branches-retained");
        reveal_strlit("unknown λ\0\n"); reveal_strlit("terminal\0");
        reveal_strlit("cleaned\0"); reveal_strlit("verified-absent\n");
        reveal_strlit("reusable-source-preserved\0");
        reveal_strlit("expired-checkout-verified-absent-source-branches-retained\n");
        assert(""@.len() != "reusable-source-preserved"@.len());
        assert(""@.len() != "expired-checkout-verified-absent-source-branches-retained"@.len());
        assert(""@.len() != "cleaned"@.len());
        assert(""@.len() != "verified-absent"@.len());
        assert(""@.len() != "already-absent"@.len());
        assert("unknown λ\0\n"@.len() != "terminal"@.len());
        assert("terminal\0"@.len() != "terminal"@.len());
        assert("cleaned\0"@.len() != "cleaned"@.len());
        assert("verified-absent\n"@.len() != "verified-absent"@.len());
        assert("verified-absent\n"@.len() != "already-absent"@.len());
        assert("reusable-source-preserved\0"@.len() != "reusable-source-preserved"@.len());
        assert("expired-checkout-verified-absent-source-branches-retained\n"@.len()
            != "expired-checkout-verified-absent-source-branches-retained"@.len());
    }
    let missing = policy::observed_cleanup_result(false, "terminal", None, None, None);
    expect_cleanup_outcome(missing, false, false, Failed);
    let pending = policy::observed_cleanup_result(true, "unknown λ\0\n", None,
        Some("cleaned"), None);
    expect_cleanup_outcome(pending, true, true, CachesReleased);
    let terminal = policy::observed_cleanup_result(true, "terminal", None,
        Some("cleaned"), None);
    expect_cleanup_outcome(terminal, true, true, Retained);
    let empty = policy::observed_cleanup_result(false, "", Some(""), Some("cleaned"), Some(""));
    expect_cleanup_outcome(empty, false, true, Retained);
    let age_suffix = policy::observed_cleanup_result(false, "terminal",
        Some("reusable-source-preserved\0"), Some("cleaned"), Some("verified-absent\n"));
    expect_cleanup_outcome(age_suffix, false, true, Retained);
    let expired_suffix = policy::observed_cleanup_result(true, "terminal\0",
        Some("expired-checkout-verified-absent-source-branches-retained\n"),
        Some("cleaned"), Some("verified-absent"));
    expect_cleanup_outcome(expired_suffix, true, true, CachesReleased);
    let status_suffix = policy::observed_cleanup_result(true, "terminal", None,
        Some("cleaned\0"), Some("already-absent"));
    expect_cleanup_outcome(status_suffix, true, false, Failed);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn terminal_and_released_retained_work_remain_discoverable() {
    broadcast use vstd::string::group_string_axioms;
    proof {
        reveal_strlit("terminal"); reveal_strlit("retained");
        assert("terminal"@[0] != "retained"@[0]);
    }
    let terminal = policy::select_cleanup("terminal", false, false, false);
    let retry = policy::select_cleanup("retained", false, true, false);
    let ordinary = policy::select_cleanup("retained", false, false, false);
    let limited = policy::select_cleanup("retained", false, true, true);
    expect(terminal && retry && !ordinary && !limited);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn active_unknown_and_disposed_work_never_gain_terminal_authority() {
    broadcast use vstd::string::group_string_axioms;
    proof {
        reveal_strlit("active"); reveal_strlit("unknown");
        reveal_strlit("disposed"); reveal_strlit("terminal"); reveal_strlit("retained");
        assert("active"@.len() == 6 && "unknown"@.len() == 7);
        assert("disposed"@.len() == 8 && "terminal"@.len() == 8 && "retained"@.len() == 8);
        assert("disposed"@[0] != "terminal"@[0]);
        assert("disposed"@[0] != "retained"@[0]);
    }
    let active = policy::select_cleanup("active", false, true, false);
    let cache = policy::select_cleanup("active", true, false, false);
    let unknown = policy::select_cleanup("unknown", true, true, false);
    let disposed = policy::select_cleanup("disposed", true, true, false);
    expect(!active && cache && !unknown && !disposed);
}







#[cfg_attr(not(verus_keep_ghost), test)]
fn explicit_import_preserves_external_branch_names_and_birth_keeps_its_rule() {
    broadcast use vstd::string::group_string_axioms;
    proof {
        reveal_strlit("birth"); reveal_strlit("import"); reveal_strlit("unknown");
        assert("birth"@.len() == 5 && "import"@.len() == 6 && "unknown"@.len() == 7);
    }
    let named_birth = policy::accepts_enrollment_branch("birth", true);
    let other_birth = policy::accepts_enrollment_branch("birth", false);
    let named_import = policy::accepts_enrollment_branch("import", true);
    let other_import = policy::accepts_enrollment_branch("import", false);
    let unknown_named = policy::accepts_enrollment_branch("unknown", true);
    let unknown_other = policy::accepts_enrollment_branch("unknown", false);
    expect(named_birth && !other_birth && named_import && other_import
        && !unknown_named && !unknown_other);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn cleanup_of_an_import_uses_its_enrollment_and_explicit_cleanup_keeps_the_prefix_rule() {
    broadcast use vstd::string::group_string_axioms;
    proof {
        reveal_strlit("import"); reveal_strlit("birth"); reveal_strlit("explicit");
        assert("import"@.len() == 6 && "birth"@.len() == 5 && "explicit"@.len() == 8);
    }
    let imported = policy::accepts_cleanup_branch("import", false);
    let named_import = policy::accepts_cleanup_branch("import", true);
    let other_birth = policy::accepts_cleanup_branch("birth", false);
    let other_explicit = policy::accepts_cleanup_branch("explicit", false);
    let named_explicit = policy::accepts_cleanup_branch("explicit", true);
    expect(imported && named_import && !other_birth && !other_explicit && named_explicit);
}


}
