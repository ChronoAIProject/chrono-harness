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
fn terminal_and_released_retained_work_remain_discoverable() {
    proof { reveal_strlit("terminal"); reveal_strlit("retained"); }
    let terminal = policy::select_cleanup("terminal", false, false);
    let retry = policy::select_cleanup("retained", false, true);
    let ordinary = policy::select_cleanup("retained", false, false);
    expect(terminal && retry && !ordinary);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn active_unknown_and_disposed_work_never_gain_terminal_authority() {
    proof {
        reveal_strlit("active"); reveal_strlit("unknown");
        reveal_strlit("disposed"); reveal_strlit("terminal"); reveal_strlit("retained");
    }
    let active = policy::select_cleanup("active", false, true);
    let cache = policy::select_cleanup("active", true, false);
    let unknown = policy::select_cleanup("unknown", true, true);
    let disposed = policy::select_cleanup("disposed", true, true);
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
fn only_a_successful_original_with_absence_verification_establishes_disposal() {
    proof {
        reveal_strlit("cleaned"); reveal_strlit("failed");
        reveal_strlit("verified-absent"); reveal_strlit("already-absent");
        reveal_strlit("preserved"); reveal_strlit("");
    }
    let removed = policy::cleanup_result(false, "cleaned", "verified-absent");
    let recovered = policy::cleanup_result(false, "cleaned", "already-absent");
    let failed = policy::cleanup_result(false, "failed", "verified-absent");
    let retained = policy::cleanup_result(false, "cleaned", "preserved");
    let missing = policy::cleanup_result(false, "cleaned", "");
    let cache = policy::cleanup_result(true, "cleaned", "verified-absent");
    expect(removed == policy::CleanupStage::Disposed);
    expect(recovered == policy::CleanupStage::Disposed);
    expect(failed == policy::CleanupStage::Failed);
    expect(retained == policy::CleanupStage::Retained);
    expect(missing == policy::CleanupStage::Retained);
    expect(cache == policy::CleanupStage::CachesReleased);
}

}
