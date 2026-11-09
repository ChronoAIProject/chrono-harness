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

verus! {

fn expect(actual: bool)
    requires actual,
{
    if !actual { panic!(); }
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
    expect(policy::reusable_release(true, 7, None));
    expect(policy::reusable_release(true, 8, Some(7)));
    expect(policy::reusable_release(false, u64::MAX, Some(u64::MAX)));
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
