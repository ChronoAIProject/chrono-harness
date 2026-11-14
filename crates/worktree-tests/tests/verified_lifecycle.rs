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

#[cfg_attr(not(verus_keep_ghost), test)]
fn empty_admission_has_no_policy_consumer_and_nonempty_enrollments_keep_the_check() {
    expect(!policy::enrolled_policy_required(0));
    expect(policy::enrolled_policy_required(1));
    expect(policy::enrolled_policy_required(usize::MAX));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn producer_namespace_members_require_complete_literal_identity() {
    proof {
        reveal_strlit("objects.json"); reveal_strlit("producer-records");
        reveal_strlit("rolling-metadata"); reveal_strlit("rolling-disposals");
        reveal_strlit(""); reveal_strlit("unfinished-birth"); reveal_strlit("objects.json\0");
        assert("unfinished-birth"@[0] != "producer-records"@[0]);
        assert("unfinished-birth"@[0] != "rolling-metadata"@[0]);
        assert(""@.len() != "objects.json"@.len());
        assert(""@.len() != "producer-records"@.len());
        assert(""@.len() != "rolling-metadata"@.len());
        assert(""@.len() != "rolling-disposals"@.len());
        assert("objects.json\0"@.len() != "objects.json"@.len());
        assert("objects.json\0"@.len() != "producer-records"@.len());
        assert("objects.json\0"@.len() != "rolling-metadata"@.len());
        assert("objects.json\0"@.len() != "rolling-disposals"@.len());
    }
    use policy::ProducerNamespaceMember::*;
    expect(match policy::producer_namespace_member("objects.json") { Owner => true, _ => false });
    expect(match policy::producer_namespace_member("producer-records") { Records => true, _ => false });
    expect(match policy::producer_namespace_member("rolling-metadata") { Metadata => true, _ => false });
    expect(match policy::producer_namespace_member("rolling-disposals") { Disposals => true, _ => false });
    expect(match policy::producer_namespace_member("") { Unknown => true, _ => false });
    expect(match policy::producer_namespace_member("unfinished-birth") { Unknown => true, _ => false });
    expect(match policy::producer_namespace_member("objects.json\0") { Unknown => true, _ => false });
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn enclosing_fixture_requires_retired_objects_metadata_and_known_members() {
    expect(policy::producer_namespace_retired(true, true, true));
    expect(!policy::producer_namespace_retired(false, true, true));
    expect(!policy::producer_namespace_retired(true, false, true));
    expect(!policy::producer_namespace_retired(true, true, false));
    expect(!policy::producer_namespace_retired(false, false, false));
}

fn expect(actual: bool)
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
fn imported_policy_requires_published_original_and_exact_receipt_fields() {
    broadcast use vstd::string::group_string_axioms;
    proof {
        reveal_strlit("chrono-worktree-report/v1"); reveal_strlit("chrono-worktree-report/v2");
        reveal_strlit("import"); reveal_strlit("migrate");
        reveal_strlit("imported"); reveal_strlit("failed");
        reveal_strlit("/owner"); reveal_strlit("/owner\0");
        reveal_strlit("report"); reveal_strlit("report/other");
        reveal_strlit("config"); reveal_strlit("config\0");
        assert("chrono-worktree-report/v1"@[24] != "chrono-worktree-report/v2"@[24]);
        assert("import"@.len() == 6 && "migrate"@.len() == 7);
        assert("imported"@.len() == 8 && "failed"@.len() == 6);
        assert("/owner"@.len() == 6 && "/owner\0"@.len() == 7);
        assert("report"@.len() == 6 && "report/other"@.len() == 12);
        assert("config"@.len() == 6 && "config\0"@.len() == 7);
    }
    let mut r = policy::ImportedPolicyReceipt {
        schema: Some("chrono-worktree-report/v1"), operation: Some("import"),
        status: Some("imported"), source_root: Some("/owner"),
        report_path: Some("report"), config_path: Some("config"),
    };
    expect(policy::imported_policy_receipt_valid(&r, true, "/owner", "report", "config"));
    expect(!policy::imported_policy_receipt_valid(&r, false, "/owner", "report", "config"));
    r.schema = None;
    expect(!policy::imported_policy_receipt_valid(&r, true, "/owner", "report", "config"));
    r.schema = Some("chrono-worktree-report/v2");
    expect(!policy::imported_policy_receipt_valid(&r, true, "/owner", "report", "config"));
    r.schema = Some("chrono-worktree-report/v1");
    r.operation = Some("migrate");
    expect(!policy::imported_policy_receipt_valid(&r, true, "/owner", "report", "config"));
    r.operation = Some("import"); r.status = Some("failed");
    expect(!policy::imported_policy_receipt_valid(&r, true, "/owner", "report", "config"));
    r.status = Some("imported"); r.source_root = Some("/owner\0");
    expect(!policy::imported_policy_receipt_valid(&r, true, "/owner", "report", "config"));
    r.source_root = Some("/owner"); r.report_path = Some("report/other");
    expect(!policy::imported_policy_receipt_valid(&r, true, "/owner", "report", "config"));
    r.report_path = Some("report"); r.config_path = Some("config\0");
    expect(!policy::imported_policy_receipt_valid(&r, true, "/owner", "report", "config"));
    r.config_path = None;
    expect(!policy::imported_policy_receipt_valid(&r, true, "/owner", "report", "config"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn reusable_outputs_wait_for_release_and_evidence_keeps_its_own_permission() {
    use policy::Disposition::*;
    expect(policy::select_artifact(&Dispose, false, false));
    expect(policy::select_artifact(&Dispose, true, true));
    expect(!policy::select_artifact(&FinishDispose, false, false));
    expect(!policy::select_artifact(&FinishDispose, false, true));
    expect(policy::select_artifact(&FinishDispose, true, false));
    expect(policy::select_artifact(&FinishDispose, true, true));
    expect(!policy::select_artifact(&Retain, false, false));
    expect(!policy::select_artifact(&Retain, true, true));
    expect(!policy::select_artifact(&EvidenceRetain, true, false));
    expect(policy::select_artifact(&EvidenceRetain, false, true));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn release_requests_bind_only_the_requested_cache_generation() {
    expect(!policy::reusable_release(false, 7, None));
    expect(policy::reusable_release(false, 7, Some(7)));
    expect(!policy::reusable_release(false, 8, Some(7)));
    expect(!policy::reusable_release(false, 7, Some(8)));
    expect(!policy::reusable_release(false, u64::MAX, None));
    expect(!policy::reusable_release(false, u64::MAX, Some(0)));
    expect(policy::reusable_release(false, u64::MAX, Some(u64::MAX)));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn terminal_linked_work_releases_reusable_outputs_without_a_main_release_request() {
    expect(policy::reusable_release(true, 0, None));
    expect(policy::reusable_release(true, 7, None));
    expect(policy::reusable_release(true, u64::MAX, None));
}

fn expect_stage(actual: policy::CleanupStage, expected: policy::CleanupStage)
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
fn checkout_adoption_requires_active_joined_nonterminal_work() {
    broadcast use vstd::string::group_string_axioms;
    proof {
        reveal_strlit("active"); reveal_strlit("terminal");
        reveal_strlit("retained"); reveal_strlit("Active");
        reveal_strlit("active\0"); reveal_strlit("");
        assert("active"@[0] != "Active"@[0]);
        assert("active"@.len() == 6 && "active\0"@.len() == 7 && ""@.len() == 0);
    }
    expect(policy::checkout_migration_ready("active", false, false, false));
    expect(!policy::checkout_migration_ready("active", true, false, false));
    expect(!policy::checkout_migration_ready("active", false, true, false));
    expect(!policy::checkout_migration_ready("active", false, false, true));
    expect(!policy::checkout_migration_ready("active", true, true, true));
    expect(!policy::checkout_migration_ready("terminal", false, false, false));
    expect(!policy::checkout_migration_ready("retained", false, false, false));
    expect(!policy::checkout_migration_ready("Active", false, false, false));
    expect(!policy::checkout_migration_ready("active\0", false, false, false));
    expect(!policy::checkout_migration_ready("", false, false, false));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn only_actual_unreleased_evidence_blocks_checkout_removal() {
    let needed = policy::retain_evidence(false, true);
    let absent = policy::retain_evidence(false, false);
    let released = policy::retain_evidence(true, true);
    let released_absent = policy::retain_evidence(true, false);
    expect(needed && !absent && !released && !released_absent);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn terminal_history_keeps_unknown_future_live_and_unexpired_records() {
    let boundary = policy::retire_history(true, true, Some(100), 110, 10, false);
    let young = policy::retire_history(true, true, Some(100), 109, 10, false);
    let capacity = policy::retire_history(true, true, Some(100), 100, 10, true);
    let unknown = policy::retire_history(true, true, None, 110, 0, true);
    let future = policy::retire_history(true, true, Some(111), 110, 0, true);
    let active = policy::retire_history(false, true, Some(100), 110, 0, true);
    let busy = policy::retire_history(true, false, Some(100), 110, 0, true);
    let maximum = policy::retire_history(true, true, Some(u64::MAX), u64::MAX, 1, false);
    expect(boundary && !young && capacity && !unknown && !future && !active && !busy && !maximum);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn cache_history_preserves_current_generations_and_unknown_completion() {
    let expired = policy::retire_cache_history(true, 1, 2, true, Some(100), 110, 10, false);
    let capacity = policy::retire_cache_history(true, 1, 2, true, Some(100), 100, 10, true);
    let current = policy::retire_cache_history(true, 2, 2, true, Some(100), 110, 0, true);
    let future = policy::retire_cache_history(true, 3, 2, true, Some(100), 110, 0, true);
    let unknown = policy::retire_cache_history(true, 1, 2, true, None, 110, 0, true);
    let incomplete = policy::retire_cache_history(false, 1, 2, true, Some(100), 110, 0, true);
    let busy = policy::retire_cache_history(true, 1, 2, false, Some(100), 110, 0, true);
    expect(expired && capacity && !current && !future && !unknown && !incomplete && !busy);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn whole_directory_requires_owned_lifetime_and_consumer_exclusion() {
    expect(policy::rolling_disposal(true, true, true, true));
    expect(!policy::rolling_disposal(false, true, true, true));
    expect(!policy::rolling_disposal(true, false, true, true));
    expect(!policy::rolling_disposal(true, true, false, true));
    expect(!policy::rolling_disposal(true, true, true, false));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn rolling_windows_distinguish_failure_diagnostics_and_unknown_outcomes() {
    expect(policy::rolling_keep(true, false, false, 0, 86400, 3600) == 0);
    expect(policy::rolling_keep(true, true, false, 0, 86400, 3600) == 86400);
    expect(policy::rolling_keep(true, false, true, 0, 86400, 3600) == 86400);
    expect(policy::rolling_keep(false, false, false, 0, 86400, 3600) == 3600);
    expect(policy::rolling_keep(false, true, true, 0, 86400, 3600) == 3600);
    expect(policy::rolling_keep(true, true, true, u64::MAX, 0, u64::MAX) == 0);
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

#[cfg_attr(not(verus_keep_ghost), test)]
fn only_a_successful_original_with_absence_verification_establishes_disposal() {
    broadcast use vstd::string::group_string_axioms;
    proof {
        reveal_strlit("cleaned"); reveal_strlit("failed");
        reveal_strlit("verified-absent"); reveal_strlit("already-absent");
        reveal_strlit("preserved"); reveal_strlit("");
        assert("cleaned"@.len() == 7 && "failed"@.len() == 6);
        assert("preserved"@.len() == 9 && "verified-absent"@.len() == 15
            && "already-absent"@.len() == 14 && ""@.len() == 0);
    }
    let removed = policy::cleanup_result(false, "cleaned", "verified-absent");
    let recovered = policy::cleanup_result(false, "cleaned", "already-absent");
    let failed = policy::cleanup_result(false, "failed", "verified-absent");
    let retained = policy::cleanup_result(false, "cleaned", "preserved");
    let missing = policy::cleanup_result(false, "cleaned", "");
    let cache = policy::cleanup_result(true, "cleaned", "verified-absent");
    expect_stage(removed, policy::CleanupStage::Disposed);
    expect_stage(recovered, policy::CleanupStage::Disposed);
    expect_stage(failed, policy::CleanupStage::Failed);
    expect_stage(retained, policy::CleanupStage::Retained);
    expect_stage(missing, policy::CleanupStage::Retained);
    expect_stage(cache, policy::CleanupStage::CachesReleased);
}

}
