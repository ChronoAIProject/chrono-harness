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

verus! {

fn every_scalar_pair_has_exact_admission(timeout: u64, output: usize) {
    let observed = policy::valid_execution_bounds(timeout, output);
    assert(observed == (timeout != 0 && output >= 1 && output <= 67_108_864));
    let observed_output = policy::valid_output_limit(output);
    assert(observed_output == (output >= 1 && output <= 67_108_864));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn positive_timeout_accepts_the_complete_inclusive_output_boundary() {
    let minimum = policy::valid_execution_bounds(1, 1);
    let below_maximum = policy::valid_execution_bounds(30, 67_108_863);
    let maximum = policy::valid_execution_bounds(1, 67_108_864);
    let maximum_timeout = policy::valid_execution_bounds(u64::MAX, 67_108_864);
    assert(minimum && below_maximum && maximum && maximum_timeout);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn zero_timeout_and_invalid_output_are_rejected_independently() {
    let zero_pair = policy::valid_execution_bounds(0, 0);
    let zero_timeout = policy::valid_execution_bounds(0, 1);
    let zero_timeout_at_maximum = policy::valid_execution_bounds(0, 67_108_864);
    let zero_output = policy::valid_execution_bounds(u64::MAX, 0);
    let above_maximum = policy::valid_execution_bounds(1, 67_108_865);
    let largest_output = policy::valid_execution_bounds(u64::MAX, usize::MAX);
    assert(!zero_pair && !zero_timeout && !zero_timeout_at_maximum && !zero_output);
    assert(!above_maximum && !largest_output);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn transport_output_uses_the_same_inclusive_byte_boundary() {
    let zero = policy::valid_output_limit(0);
    let minimum = policy::valid_output_limit(1);
    let maximum = policy::valid_output_limit(67_108_864);
    let above_maximum = policy::valid_output_limit(67_108_865);
    let largest = policy::valid_output_limit(usize::MAX);
    assert(!zero && minimum && maximum && !above_maximum && !largest);
}

}
