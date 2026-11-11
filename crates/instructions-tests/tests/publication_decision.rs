//! 与 IO 实现独立的恢复、释放边界预期；同源 Verus 和 Rust 均执行。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../instructions/src/publication_decision.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_instructions::publication_decision as policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {
fn expect_true(actual: bool)
    requires actual,
{
    if !actual { panic!(); }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn sealed_and_interrupted_envelopes_keep_the_original_intent() {
    let raw = policy::intent_layer(true, false, false);
    let interrupted = policy::intent_layer(false, true, false);
    let sealed = policy::intent_layer(false, false, true);
    let nested = policy::intent_layer(false, true, true);
    let invalid = policy::intent_layer(false, false, false);
    expect_true(matches!(raw, policy::IntentLayer::Done));
    expect_true(matches!(interrupted, policy::IntentLayer::Original));
    expect_true(matches!(sealed, policy::IntentLayer::Intent));
    expect_true(matches!(nested, policy::IntentLayer::Original));
    expect_true(matches!(invalid, policy::IntentLayer::Invalid));
    let bound = policy::record_bound(true, true, true);
    let wrong_host = policy::record_bound(false, true, true);
    let wrong_attempt = policy::record_bound(true, false, true);
    let wrong_plan = policy::record_bound(true, true, false);
    expect_true(bound); expect_true(!wrong_host); expect_true(!wrong_attempt); expect_true(!wrong_plan);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn rollback_only_replaces_the_exact_installed_object() {
    let untouched = policy::repair(true, false, false, false);
    let fresh = policy::repair(false, true, true, false);
    let lost_original = policy::repair(false, true, false, false);
    let drift = policy::repair(false, false, true, false);
    let missing = policy::repair(false, false, false, false);
    expect_true(matches!(untouched, policy::Repair::Leave));
    expect_true(matches!(fresh, policy::Repair::Restore));
    expect_true(matches!(lost_original, policy::Repair::Preserve));
    expect_true(matches!(drift, policy::Repair::Preserve));
    expect_true(matches!(missing, policy::Repair::Preserve));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn immutable_commit_never_rolls_back_and_requires_installed_identity() {
    let installed = policy::repair(false, true, true, true);
    let original = policy::repair(true, false, true, true);
    let missing = policy::repair(false, false, false, true);
    expect_true(matches!(installed, policy::Repair::Leave));
    expect_true(matches!(original, policy::Repair::Preserve));
    expect_true(matches!(missing, policy::Repair::Preserve));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn all_three_independent_release_obligations_are_required() {
    let yes = policy::can_release(true, true, true);
    let unsealed = policy::can_release(false, true, true);
    let unbound = policy::can_release(true, false, true);
    let unresolved = policy::can_release(true, true, false);
    let none = policy::can_release(false, false, false);
    expect_true(yes); expect_true(!unsealed); expect_true(!unbound);
    expect_true(!unresolved); expect_true(!none);
}

#[cfg(not(verus_keep_ghost))]
#[test]
#[should_panic]
fn expectations_have_a_real_failure_branch() { expect_true(false); }
}
