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
