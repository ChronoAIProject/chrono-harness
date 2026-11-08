//! 独立列出既有发布配方的成功、失败和边界，不以新实现计算预期。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../native/src/verified_recipe.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_host_release::verified_recipe as policy;
use policy::{DependencyResult, Invocation};
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn non_rust_verification_and_collection_do_not_require_rust() {
    let verification = policy::requires_rust(&Invocation::Verify { rust_toolchain: false });
    let collection = policy::requires_rust(&Invocation::Collect);
    assert(!verification && !collection);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn build_and_explicit_verification_require_rust() {
    let build = policy::requires_rust(&Invocation::Build);
    let verification = policy::requires_rust(&Invocation::Verify { rust_toolchain: true });
    assert(build && verification);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn child_failure_wins_over_forwarding_retention_and_receipt_failures() {
    let ordinary = policy::result_exit(Some(41), false);
    let auxiliary = policy::result_exit(Some(41), true);
    let lowest = policy::result_exit(Some(1), true);
    let highest = policy::result_exit(Some(255), true);
    assert(ordinary == 41 && auxiliary == 41 && lowest == 1 && highest == 255);
    let success = policy::passed(Some(41), true);
    assert(!success);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn successful_child_fails_when_evidence_cannot_be_published() {
    let success = policy::result_exit(Some(0), false);
    let failure = policy::result_exit(Some(0), true);
    let passed = policy::passed(Some(0), false);
    let rejected = policy::passed(Some(0), true);
    assert(success == 0 && failure == 2 && passed && !rejected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn missing_child_status_is_never_success() {
    let no_error = policy::result_exit(None, false);
    let with_error = policy::result_exit(None, true);
    let passed = policy::passed(None, false);
    assert(no_error == 2 && with_error == 2 && !passed);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn child_signal_survives_auxiliary_io_failures() {
    let killed = policy::result_exit(Some(-9), false);
    let killed_with_io_failure = policy::result_exit(Some(-9), true);
    let first = policy::result_exit(Some(-1), true);
    let last = policy::result_exit(Some(-127), true);
    assert(killed == 137 && killed_with_io_failure == 137 && first == 129 && last == 255);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn old_success_cannot_hide_current_failed_cancelled_or_skipped_needs() {
    let failed = policy::selected_producer(&DependencyResult::Failure, "run", "run", 1, 2, "artifact");
    let cancelled = policy::selected_producer(&DependencyResult::Cancelled, "run", "run", 1, 2, "artifact");
    let skipped = policy::selected_producer(&DependencyResult::Skipped, "run", "run", 1, 2, "artifact");
    assert(!failed && !cancelled && !skipped);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn successful_current_needs_can_select_the_original_earlier_attempt() {
    proof { reveal_strlit("run"); reveal_strlit("artifact"); }
    let carried = policy::selected_producer(&DependencyResult::Success, "run", "run", 1, 2, "artifact");
    let same = policy::selected_producer(&DependencyResult::Success, "run", "run", 2, 2, "artifact");
    let large = policy::selected_producer(&DependencyResult::Success, "run", "run", u64::MAX, u64::MAX, "artifact");
    assert(carried && same && large);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn missing_future_or_foreign_producers_are_rejected() {
    proof {
        reveal_strlit("run"); reveal_strlit("foreign");
        assert("run"@.len() != "foreign"@.len());
    }
    let zero = policy::selected_producer(&DependencyResult::Success, "run", "run", 0, 2, "artifact");
    let future = policy::selected_producer(&DependencyResult::Success, "run", "run", 3, 2, "artifact");
    let foreign = policy::selected_producer(&DependencyResult::Success, "foreign", "run", 1, 2, "artifact");
    let absent_run = policy::selected_producer(&DependencyResult::Success, "", "", 1, 2, "artifact");
    let absent_artifact = policy::selected_producer(&DependencyResult::Success, "run", "run", 1, 2, "");
    assert(!zero && !future && !foreign && !absent_run && !absent_artifact);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn receipt_job_run_and_attempt_must_match_the_selected_producer() {
    proof {
        reveal_strlit("job"); reveal_strlit("other"); reveal_strlit("run"); reveal_strlit("changed");
        assert("job"@.len() != "other"@.len());
        assert("run"@.len() != "changed"@.len());
    }
    let valid = policy::producer_lineage("job", "job", "run", "run", 1, 1);
    let job = policy::producer_lineage("other", "job", "run", "run", 1, 1);
    let run = policy::producer_lineage("job", "job", "changed", "run", 1, 1);
    let attempt = policy::producer_lineage("job", "job", "run", "run", 1, 2);
    let zero = policy::producer_lineage("job", "job", "run", "run", 0, 0);
    let empty_job = policy::producer_lineage("", "", "run", "run", 1, 1);
    let empty_run = policy::producer_lineage("job", "job", "", "", 1, 1);
    assert(valid && !job && !run && !attempt && !zero && !empty_job && !empty_run);
}

}
