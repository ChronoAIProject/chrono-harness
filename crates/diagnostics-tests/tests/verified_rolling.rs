//! 容量的独立边界表；同一预期与驱动用于 Verus 和普通行为测试。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../diagnostics/src/verified_rolling.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_diagnostics::{verified_rolling as policy, verus};
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

fn expect(condition: bool)
    requires condition,
{
    if !condition { panic!(); }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn file_and_byte_bounds_are_positive_and_include_the_last_slot() {
    expect(policy::MAX_FILES == 64);
    expect(!policy::valid_bounds(0, 0));
    expect(!policy::valid_bounds(0, 1));
    expect(!policy::valid_bounds(0, 64));
    expect(!policy::valid_bounds(1, 0));
    expect(policy::valid_bounds(1, 1));
    expect(policy::valid_bounds(1, 64));
    expect(!policy::valid_bounds(1, 65));
    expect(policy::valid_bounds(u64::MAX, 64));
    expect(!policy::valid_bounds(u64::MAX, usize::MAX));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn records_keep_empty_exact_limit_and_maximum_lengths() {
    expect(policy::record_fits(0, 0));
    expect(policy::record_fits(0, 1));
    expect(!policy::record_fits(1, 0));
    expect(policy::record_fits(1, 1));
    expect(!policy::record_fits(2, 1));
    expect(policy::record_fits(u64::MAX - 1, u64::MAX));
    expect(policy::record_fits(u64::MAX, u64::MAX));
    expect(!policy::record_fits(u64::MAX, u64::MAX - 1));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn rotation_is_strict_at_the_sum_limit_without_integer_overflow() {
    expect(!policy::needs_rotation(0, 1, 1));
    expect(policy::needs_rotation(1, 1, 1));
    expect(!policy::needs_rotation(0, 0, 0));
    expect(!policy::needs_rotation(0, u64::MAX, 0));
    expect(!policy::needs_rotation(u64::MAX, u64::MAX, 0));
    expect(policy::needs_rotation(u64::MAX, u64::MAX, 1));
    expect(!policy::needs_rotation(u64::MAX - 1, u64::MAX, 1));
    expect(policy::needs_rotation(u64::MAX - 1, u64::MAX, 2));
    expect(!policy::needs_rotation(0, u64::MAX, u64::MAX));
    expect(policy::needs_rotation(1, u64::MAX, u64::MAX));
    expect(policy::needs_rotation(u64::MAX, 1, 0));
    expect(policy::needs_rotation(u64::MAX, u64::MAX, u64::MAX));
}

}
