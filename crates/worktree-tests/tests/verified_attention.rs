//! 提醒分类的独立边界预期；Cargo 和 Verus 消费同一生产实现。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../worktree/src/verified_attention.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(not(verus_keep_ghost))]
use chrono_worktree::verified_attention as policy;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn active_and_retained_work_still_need_attention() {
    let active = policy::classify("active", false, true, true, true, true, true, false);
    let retained = policy::classify("retained", false, true, true, true, true, true, true);
    let pending = policy::classify("terminal", false, true, true, true, true, true, true);
    proof {
        reveal_strlit("active"); reveal_strlit("retained");
        reveal_strlit("terminal"); reveal_strlit("disposed");
        assert("terminal"@[0] != "retained"@[0]);
    }
    assert(active is Active);
    assert(retained is Retained);
    assert(pending is TerminalPending);
    let a = policy::needs_attention(&active);
    let r = policy::needs_attention(&retained);
    let p = policy::needs_attention(&pending);
    assert(a && r && p);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn removal_requires_both_absences_and_a_terminal_handoff() {
    let removed = policy::classify("disposed", false, true, true, false, false, false, true);
    let physical = policy::classify("disposed", false, true, true, true, false, false, true);
    let git = policy::classify("disposed", false, true, true, false, true, false, true);
    let unfinished = policy::classify("disposed", false, true, true, false, false, false, false);
    assert(removed is Disposed);
    assert(physical is Unknown);
    assert(git is Unknown);
    assert(unfinished is Unknown);
    let quiet = policy::needs_attention(&removed);
    let alert = policy::needs_attention(&physical);
    assert(!quiet && alert);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn unknown_unregistered_and_coordinator_are_distinct() {
    let unknown = policy::classify("active", false, false, true, true, true, true, false);
    let unattached = policy::classify("active", false, true, true, true, true, false, false);
    let unregistered = policy::classify("", false, true, false, true, true, false, false);
    let main = policy::classify("active", true, true, true, true, true, true, false);
    assert(unknown is Unknown);
    assert(unattached is Unknown);
    assert(unregistered is Unregistered);
    assert(main is Coordinator);
    let a = policy::needs_attention(&unknown);
    let b = policy::needs_attention(&unregistered);
    let c = policy::needs_attention(&main);
    assert(a && b && !c);
}

}
