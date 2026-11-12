//! 容量和成员控制的独立边界预期；同一驱动用于 Verus 和普通行为测试。
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

#[cfg_attr(not(verus_keep_ghost), test)]
fn regular_members_keep_missing_invalid_and_original_errors_distinct() {
    let missing = |error: &u64| -> (out: bool)
        ensures out == (*error == 404),
    { *error == 404 };
    let invalid = || -> (out: u64) ensures out == 400,
    { 400 };
    let regular = policy::regular_member(Ok::<bool, u64>(true), missing, invalid);
    match regular { Ok(value) => expect(value), Err(_) => expect(false) }
    let absent = policy::regular_member(Err::<bool, u64>(404), missing, invalid);
    match absent { Ok(value) => expect(!value), Err(_) => expect(false) }
    let wrong_type = policy::regular_member(Ok::<bool, u64>(false), missing, invalid);
    match wrong_type { Err(code) => expect(code == 400), Ok(_) => expect(false) }
    let original = policy::regular_member(Err::<bool, u64>(u64::MAX), missing, invalid);
    match original { Err(code) => expect(code == u64::MAX), Ok(_) => expect(false) }
}

type MemberFault = (u64, usize, usize);

spec fn next_count(count: u64) -> u64 {
    if count < u64::MAX { (count + 1) as u64 } else { count }
}

fn counted(count: u64) -> (out: u64)
    ensures out == next_count(count),
{
    if count < u64::MAX { count + 1 } else { count }
}

/// 从被证明的实际轨迹取出计数；不另运行一份轮转实现来产生预期。
proof fn phase_counter_at<E>(
    states: Seq<u64>, initial: u64, first: usize,
    step: spec_fn(u64, usize, (u64, Result<(), E>)) -> bool,
    width: u64, at: int,
)
    requires policy::phase_prefix(states, initial, first as int, step),
        0 <= at < states.len(), 1 <= width <= 2,
        first as int + at <= usize::MAX,
        initial as int + width as int * at <= u64::MAX,
        forall|before: u64, index: usize, after: u64|
            before <= u64::MAX - width
                && #[trigger] step(before, index, (after, Ok::<(), E>(())))
                ==> after as int == before as int + width as int,
    ensures states[at] as int == initial as int + width as int * at,
    decreases at,
{
    if at > 0 {
        if width == 1 {
            assert(initial as int + width as int * (at - 1) <= u64::MAX);
        } else {
            assert(width == 2);
            assert(initial as int + width as int * (at - 1) <= u64::MAX);
        }
        phase_counter_at(states, initial, first, step, width, at - 1);
        if width == 1 {
            assert(states[at - 1] <= u64::MAX - width);
        } else {
            assert(width == 2);
            assert(states[at - 1] <= u64::MAX - width);
        }
        assert(0 <= at - 1 < states.len() - 1);
        let previous = at - 1;
        assert(step(states[previous], (first as int + previous) as usize,
            (states[previous + 1], Ok::<(), E>(()))) );
        assert(states[previous + 1] == states[at]);
        if width == 1 {
            assert(states[at] as int == initial as int + width as int * at);
        } else {
            assert(width == 2);
            assert(states[at] as int == initial as int + width as int * at);
        }
    }
}

proof fn successful_phase_count<E>(
    initial: u64, first: usize, end: usize,
    step: spec_fn(u64, usize, (u64, Result<(), E>)) -> bool,
    width: u64, out: (u64, Result<(), E>),
)
    requires first <= end, 1 <= width <= 2,
        initial as int + width as int * (end as int - first as int) <= u64::MAX,
        policy::phase_result(initial, first, end, step, out),
        forall|before: u64, index: usize, after: u64, error: E|
            #[trigger] step(before, index, (after, Err::<(), E>(error))) ==> false,
        forall|before: u64, index: usize, after: u64|
            before <= u64::MAX - width
                && #[trigger] step(before, index, (after, Ok::<(), E>(())))
                ==> after as int == before as int + width as int,
    ensures out.1.is_ok(),
        out.0 as int == initial as int + width as int * (end as int - first as int),
{
    let states = choose|states: Seq<u64>|
        policy::phase_prefix(states, initial, first as int, step)
            && match out.1 {
                Ok(()) => states.len() == end as int - first as int + 1 && out.0 == states.last(),
                Err(error) => first as int + states.len() - 1 < end as int
                    && step(states.last(), (first as int + states.len() - 1) as usize,
                        (out.0, Err::<(), E>(error))),
            };
    assert(out.1.is_ok());
    assert(states.len() == end as int - first as int + 1);
    phase_counter_at(states, initial, first, step, width, states.len() - 1);
}

fn expect_member_fault(out: (u64, Result<(), MemberFault>), calls: u64,
    at: u64, source: usize, target: usize)
    requires out.0 == calls,
        match out.1 { Err(error) => error == (at, source, target), Ok(()) => false },
{
    expect(out.0 == calls);
    match out.1 {
        Err(error) => {
            expect(error.0 == at);
            expect(error.1 == source);
            expect(error.2 == target);
        },
        Ok(()) => expect(false),
    }
}

fn first_remove_failure_case(count: usize)
    requires 1 <= count <= 64,
{
    let check = |state: u64, _index: usize| -> (out: (u64, Result<bool, MemberFault>))
        ensures out == (next_count(state), Ok::<bool, MemberFault>(true)),
    { (counted(state), Ok(true)) };
    let remove = |state: u64, index: usize| -> (out: (u64, Result<(), MemberFault>))
        ensures out == (next_count(state), Err::<(), MemberFault>((state, index, index))),
    { (counted(state), Err((state, index, index))) };
    let rename = |state: u64, source: usize, target: usize| -> (out: (u64, Result<(), MemberFault>))
        ensures out == (next_count(state), Err::<(), MemberFault>((state, source, target))),
    { (counted(state), Err((state, source, target))) };
    let out = policy::rotate_members(0, count, check, remove, rename);
    proof {
        let (after_check, checked) = choose|state: u64, observed: Result<(), MemberFault>|
            #[trigger] policy::checked_phase(0, count, check, (state, observed))
                && match observed {
                    Err(error) => out == (state, Err::<(), MemberFault>(error)),
                    Ok(()) => exists|after: u64, removed: Result<(), MemberFault>|
                        #[trigger] policy::remove_result(state, (count - 1) as usize,
                            check, remove, (after, removed))
                            && match removed {
                                Err(error) => out == (after, Err::<(), MemberFault>(error)),
                                Ok(()) => policy::phase_result(after, 0, (count - 1) as usize,
                                    |before: u64, offset: usize, result: (u64, Result<(), MemberFault>)|
                                        policy::move_result(before, (count - 1 - offset) as usize,
                                            check, rename, result), out),
                            },
                };
        let step = |before: u64, index: usize, result: (u64, Result<(), MemberFault>)|
            policy::check_result(before, index, check, result);
        successful_phase_count(0, 0, count, step, 1, (after_check, checked));
        let (after_remove, removed) = choose|after: u64, removed: Result<(), MemberFault>|
            #[trigger] policy::remove_result(after_check, (count - 1) as usize,
                check, remove, (after, removed))
                && match removed {
                    Err(error) => out == (after, Err::<(), MemberFault>(error)),
                    Ok(()) => policy::phase_result(after, 0, (count - 1) as usize,
                        |before: u64, offset: usize, result: (u64, Result<(), MemberFault>)|
                            policy::move_result(before, (count - 1 - offset) as usize,
                                check, rename, result), out),
                };
        let (middle, present) = choose|middle: u64, observed: Result<bool, MemberFault>|
            #[trigger] check.ensures((after_check, (count - 1) as usize), (middle, observed))
                && match observed {
                    Err(error) => (after_remove, removed) == (middle, Err::<(), MemberFault>(error)),
                    Ok(false) => (after_remove, removed) == (middle, Ok::<(), MemberFault>(())),
                    Ok(true) => remove.ensures((middle, (count - 1) as usize), (after_remove, removed)),
                };
        assert(present == Ok::<bool, MemberFault>(true));
        assert(after_check == count as u64);
        assert(middle == count as u64 + 1);
    }
    expect_member_fault(out, count as u64 + 2, count as u64 + 1, count - 1, count - 1);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn all_reserved_members_are_checked_before_the_first_remove() {
    first_remove_failure_case(1);
    first_remove_failure_case(3);
    first_remove_failure_case(64);
}

fn first_rename_failure_case(count: usize)
    requires 2 <= count <= 64,
{
    let check = |state: u64, _index: usize| -> (out: (u64, Result<bool, MemberFault>))
        ensures out == (next_count(state), Ok::<bool, MemberFault>(true)),
    { (counted(state), Ok(true)) };
    let remove = |state: u64, _index: usize| -> (out: (u64, Result<(), MemberFault>))
        ensures out == (next_count(state), Ok::<(), MemberFault>(())),
    { (counted(state), Ok(())) };
    let rename = |state: u64, source: usize, target: usize| -> (out: (u64, Result<(), MemberFault>))
        ensures out == (next_count(state), Err::<(), MemberFault>((state, source, target))),
    { (counted(state), Err((state, source, target))) };
    let out = policy::rotate_members(0, count, check, remove, rename);
    proof {
        let (after_check, checked) = choose|state: u64, observed: Result<(), MemberFault>|
            #[trigger] policy::checked_phase(0, count, check, (state, observed))
                && match observed {
                    Err(error) => out == (state, Err::<(), MemberFault>(error)),
                    Ok(()) => exists|after: u64, removed: Result<(), MemberFault>|
                        #[trigger] policy::remove_result(state, (count - 1) as usize,
                            check, remove, (after, removed))
                            && match removed {
                                Err(error) => out == (after, Err::<(), MemberFault>(error)),
                                Ok(()) => policy::phase_result(after, 0, (count - 1) as usize,
                                    |before: u64, offset: usize, result: (u64, Result<(), MemberFault>)|
                                        policy::move_result(before, (count - 1 - offset) as usize,
                                            check, rename, result), out),
                            },
                };
        let checked_step = |before: u64, index: usize, result: (u64, Result<(), MemberFault>)|
            policy::check_result(before, index, check, result);
        successful_phase_count(0, 0, count, checked_step, 1, (after_check, checked));
        let (after_remove, removed) = choose|after: u64, removed: Result<(), MemberFault>|
            #[trigger] policy::remove_result(after_check, (count - 1) as usize,
                check, remove, (after, removed))
                && match removed {
                    Err(error) => out == (after, Err::<(), MemberFault>(error)),
                    Ok(()) => policy::phase_result(after, 0, (count - 1) as usize,
                        |before: u64, offset: usize, result: (u64, Result<(), MemberFault>)|
                            policy::move_result(before, (count - 1 - offset) as usize,
                                check, rename, result), out),
                };
        let (middle, present) = choose|middle: u64, observed: Result<bool, MemberFault>|
            #[trigger] check.ensures((after_check, (count - 1) as usize), (middle, observed))
                && match observed {
                    Err(error) => (after_remove, removed) == (middle, Err::<(), MemberFault>(error)),
                    Ok(false) => (after_remove, removed) == (middle, Ok::<(), MemberFault>(())),
                    Ok(true) => remove.ensures((middle, (count - 1) as usize), (after_remove, removed)),
                };
        assert(present == Ok::<bool, MemberFault>(true));
        assert(removed == Ok::<(), MemberFault>(()));
        assert(after_remove == count as u64 + 2);
        let step = |before: u64, offset: usize, result: (u64, Result<(), MemberFault>)|
            policy::move_result(before, (count - 1 - offset) as usize, check, rename, result);
        let states = choose|states: Seq<u64>|
            policy::phase_prefix(states, after_remove, 0, step)
                && match out.1 {
                    Ok(()) => states.len() == count && out.0 == states.last(),
                    Err(error) => states.len() - 1 < count - 1
                        && step(states.last(), (states.len() - 1) as usize,
                            (out.0, Err::<(), MemberFault>(error))),
                };
        assert forall|before: u64, offset: usize, after: u64|
            #[trigger] step(before, offset, (after, Ok::<(), MemberFault>(()))) implies false by {}
        assert(policy::phase_prefix(states, after_remove, 0, step));
        if states.len() > 1 {
            let first: int = 0;
            let index: int = 0;
            assert(step(states[index], (first + index) as usize,
                (states[index + 1], Ok::<(), MemberFault>(()))) );
            assert(false);
        }
        assert(states.len() == 1);
        assert(out.1.is_err());
        assert(step(after_remove, 0, out));
        let (before_rename, source) = choose|middle: u64, observed: Result<bool, MemberFault>|
            #[trigger] check.ensures((after_remove, (count - 2) as usize), (middle, observed))
                && match observed {
                    Err(error) => out == (middle, Err::<(), MemberFault>(error)),
                    Ok(false) => out == (middle, Ok::<(), MemberFault>(())),
                    Ok(true) => rename.ensures((middle, (count - 2) as usize, (count - 1) as usize), out),
                };
        assert(source == Ok::<bool, MemberFault>(true));
        assert(before_rename == count as u64 + 3);
    }
    expect_member_fault(out, count as u64 + 4, count as u64 + 3, count - 2, count - 1);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn rename_starts_with_the_last_source_and_keeps_its_complete_error() {
    first_rename_failure_case(2);
    first_rename_failure_case(3);
    first_rename_failure_case(64);
}

fn absent_adoption_case(count: usize)
    requires 1 <= count <= 64,
{
    let check = |state: u64, _index: usize| -> (out: (u64, Result<bool, MemberFault>))
        ensures out == (next_count(state), Ok::<bool, MemberFault>(false)),
    { (counted(state), Ok(false)) };
    let remove = |state: u64, index: usize| -> (out: (u64, Result<(), MemberFault>))
        ensures out == (next_count(state), Err::<(), MemberFault>((state, index, index))),
    { (counted(state), Err((state, index, index))) };
    let out = policy::adopt_members(0, count, check, remove);
    proof {
        let (after_check, checked) = choose|state: u64, observed: Result<(), MemberFault>|
            #[trigger] policy::checked_phase(0, policy::MAX_FILES, check, (state, observed))
                && match observed {
                    Err(error) => out == (state, Err::<(), MemberFault>(error)),
                    Ok(()) => policy::phase_result(state, count, policy::MAX_FILES,
                        |before: u64, index: usize, result: (u64, Result<(), MemberFault>)|
                            policy::remove_result(before, index, check, remove, result), out),
                };
        let checked_step = |before: u64, index: usize, result: (u64, Result<(), MemberFault>)|
            policy::check_result(before, index, check, result);
        successful_phase_count(0, 0, policy::MAX_FILES, checked_step, 1, (after_check, checked));
        let removed_step = |before: u64, index: usize, result: (u64, Result<(), MemberFault>)|
            policy::remove_result(before, index, check, remove, result);
        successful_phase_count(after_check, count, policy::MAX_FILES, removed_step, 1, out);
    }
    expect(out.0 == 128 - count as u64);
    match out.1 { Ok(()) => (), Err(_) => expect(false) }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn adoption_rechecks_only_excess_slots_and_skips_absent_members() {
    absent_adoption_case(1);
    absent_adoption_case(63);
    absent_adoption_case(64);
}

}
