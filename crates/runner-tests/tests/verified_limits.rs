//! 直接调用真实实现的独立资源边界预期；相同测试体也参与整 crate 证明。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../runner/src/verified_limits.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_limits as policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

#[cfg(verus_keep_ghost)]
#[path = "maintenance_signal.rs"]
mod maintenance_boundaries;

verus! {

// 前置条件由独立预期证明，普通 Rust 同时执行失败分支。
fn expect_limit_true(actual: bool)
    requires actual,
{
    if !actual {
        panic!();
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn positive_timeout_accepts_the_complete_inclusive_output_boundary() {
    let minimum = policy::valid_execution_bounds(1, 1);
    let below_maximum = policy::valid_execution_bounds(30, 67_108_863);
    let maximum = policy::valid_execution_bounds(1, 67_108_864);
    let maximum_timeout = policy::valid_execution_bounds(u64::MAX, 67_108_864);
    expect_limit_true(minimum && below_maximum && maximum && maximum_timeout);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn zero_timeout_and_invalid_output_are_rejected_independently() {
    let zero_pair = policy::valid_execution_bounds(0, 0);
    let zero_timeout = policy::valid_execution_bounds(0, 1);
    let zero_timeout_at_maximum = policy::valid_execution_bounds(0, 67_108_864);
    let zero_output = policy::valid_execution_bounds(u64::MAX, 0);
    let above_maximum = policy::valid_execution_bounds(1, 67_108_865);
    let largest_output = policy::valid_execution_bounds(u64::MAX, usize::MAX);
    expect_limit_true(!zero_pair && !zero_timeout && !zero_timeout_at_maximum && !zero_output);
    expect_limit_true(!above_maximum && !largest_output);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn transport_output_uses_the_same_inclusive_byte_boundary() {
    let zero = policy::valid_output_limit(0);
    let minimum = policy::valid_output_limit(1);
    let maximum = policy::valid_output_limit(67_108_864);
    let above_maximum = policy::valid_output_limit(67_108_865);
    let largest = policy::valid_output_limit(usize::MAX);
    expect_limit_true(!zero && minimum && maximum && !above_maximum && !largest);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn launch_events_preserve_the_complete_identity_at_both_encoding_boundaries() {
    let first = policy::launch_cookie(0, 1);
    let last_slot = policy::launch_cookie(4095, 1);
    let last_identity = policy::launch_cookie(4095, policy::MAX_LAUNCH_GENERATION);
    expect_limit_true(first == Some(8192));
    expect_limit_true(last_slot == Some(16382));
    expect_limit_true(last_identity == Some(usize::MAX - 1));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn unallocated_and_unrepresentable_launch_identities_are_rejected() {
    let empty_generation = policy::launch_cookie(0, 0);
    let first_bad_slot = policy::launch_cookie(4096, 1);
    let largest_slot = policy::launch_cookie(usize::MAX, 1);
    let first_bad_generation = policy::launch_cookie(0, policy::MAX_LAUNCH_GENERATION + 1);
    let largest_generation = policy::launch_cookie(0, usize::MAX);
    expect_limit_true(empty_generation.is_none() && first_bad_slot.is_none());
    expect_limit_true(largest_slot.is_none() && first_bad_generation.is_none());
    expect_limit_true(largest_generation.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn generation_exhaustion_never_wraps_back_to_an_earlier_identity() {
    let first = policy::next_launch_generation(1);
    let last = policy::next_launch_generation(policy::MAX_LAUNCH_GENERATION);
    let exhausted = policy::next_launch_generation(policy::MAX_LAUNCH_GENERATION + 1);
    let empty = policy::next_launch_generation(0);
    let largest = policy::next_launch_generation(usize::MAX);
    expect_limit_true(first == Some(2));
    expect_limit_true(last == Some(policy::MAX_LAUNCH_GENERATION + 1));
    expect_limit_true(exhausted.is_none() && empty.is_none() && largest.is_none());
}

}
