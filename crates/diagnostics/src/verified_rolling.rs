//! 日志文件数、单记录容量和轮转阈值的同源判断。
#[cfg(not(verus_keep_ghost))]
use crate::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

pub const MAX_FILES: usize = 64;

pub fn valid_bounds(max_bytes: u64, max_files: usize) -> (valid: bool)
    ensures valid == (max_bytes > 0 && 1 <= max_files && max_files <= MAX_FILES),
{
    max_bytes > 0 && 1 <= max_files && max_files <= MAX_FILES
}

pub fn record_fits(record_bytes: u64, max_bytes: u64) -> (fits: bool)
    ensures fits == (record_bytes <= max_bytes),
{
    record_bytes <= max_bytes
}

/// 用减法实现数学整数的完整长度和比较，避免实际 u64 加法溢出。
pub fn needs_rotation(length: u64, max_bytes: u64, record_bytes: u64) -> (rotate: bool)
    requires record_bytes <= max_bytes,
    ensures rotate == (length as int + record_bytes as int > max_bytes as int),
{
    length > max_bytes - record_bytes
}

pub open spec fn regular_result<E, M, I>(
    observed: Result<bool, E>, missing: M, invalid: I, out: Result<bool, E>,
) -> bool
    where M: Fn(&E) -> bool, I: FnOnce() -> E,
{
    match observed {
        Ok(true) => out == Ok::<bool, E>(true),
        Ok(false) => match out {
            Err(error) => invalid.ensures((), error),
            _ => false,
        },
        Err(error) => match out {
            Ok(false) => missing.ensures((&error,), true),
            Err(original) => original == error && missing.ensures((&error,), false),
            _ => false,
        },
    }
}

/// metadata 的类型观察由标准接口提供；分类及原错误的移交归本所有者。
pub fn regular_member<E, M, I>(observed: Result<bool, E>, missing: M, invalid: I)
    -> (out: Result<bool, E>)
    where M: Fn(&E) -> bool, I: FnOnce() -> E,
    requires forall|error: &E| missing.requires((error,)), invalid.requires(()),
    ensures regular_result(observed, missing, invalid, out),
{
    match observed {
        Ok(true) => Ok(true),
        Ok(false) => Err(invalid()),
        Err(error) => if missing(&error) { Ok(false) } else { Err(error) },
    }
}

/// 状态轨迹只用于证明；运行时不收集、克隆或缓存成员观察。
pub open spec fn phase_prefix<S, E>(
    states: Seq<S>, initial: S, first: int,
    step: spec_fn(S, usize, (S, Result<(), E>)) -> bool,
) -> bool {
    states.len() > 0 && states.first() == initial
        && forall|i: int| 0 <= i < states.len() - 1 ==>
            #[trigger] step(states[i], (first + i) as usize,
                (states[i + 1], Ok::<(), E>(())))
}

pub open spec fn phase_result<S, E>(
    initial: S, first: usize, end: usize,
    step: spec_fn(S, usize, (S, Result<(), E>)) -> bool,
    out: (S, Result<(), E>),
) -> bool {
    exists|states: Seq<S>| phase_prefix(states, initial, first as int, step)
        && match out.1 {
            Ok(()) => states.len() == end as int - first as int + 1
                && out.0 == states.last(),
            Err(error) => first as int + states.len() - 1 < end as int
                && step(states.last(), (first as int + states.len() - 1) as usize,
                    (out.0, Err::<(), E>(error))),
        }
}

/// 按原顺序调用每个槽位；返回当前失败的完整状态和错误，不执行后续槽位。
fn run_phase<S, E, F>(
    initial: S, first: usize, end: usize, operation: F,
    Ghost(step): Ghost<spec_fn(S, usize, (S, Result<(), E>)) -> bool>,
) -> (out: (S, Result<(), E>))
    where F: Fn(S, usize) -> (S, Result<(), E>),
    requires first <= end,
        forall|state: S, index: usize| first <= index < end ==>
            #[trigger] operation.requires((state, index)),
        forall|state: S, index: usize, observed: (S, Result<(), E>)|
            first <= index < end && #[trigger] operation.ensures((state, index), observed)
                ==> step(state, index, observed),
    ensures phase_result(initial, first, end, step, out),
{
    let mut state = initial;
    let mut index = first;
    let ghost states = seq![initial];
    while index < end
        invariant first <= index <= end,
            states.len() == index as int - first as int + 1,
            states.last() == state,
            phase_prefix(states, initial, first as int, step),
            forall|state: S, index: usize| first <= index < end ==>
                #[trigger] operation.requires((state, index)),
            forall|state: S, index: usize, observed: (S, Result<(), E>)|
                first <= index < end && #[trigger] operation.ensures((state, index), observed)
                    ==> step(state, index, observed),
        decreases end - index,
    {
        let ghost before = state;
        let observed = operation(state, index);
        let ghost original = observed;
        let (after, result) = observed;
        assert(step(before, index, original));
        match result {
            Err(error) => {
                let out = (after, Err(error));
                assert(phase_result(initial, first, end, step, out));
                return out;
            },
            Ok(()) => {},
        }
        assert(original == (after, Ok::<(), E>(())));
        proof {
            let previous = states;
            states = states.push(after);
            assert forall|i: int| 0 <= i < states.len() - 1 implies
                #[trigger] step(states[i], (first as int + i) as usize,
                    (states[i + 1], Ok::<(), E>(()))) by {
                if i < previous.len() - 1 {
                    assert(step(previous[i], (first as int + i) as usize,
                        (previous[i + 1], Ok::<(), E>(()))) );
                } else {
                    assert(i == previous.len() - 1);
                    assert(states[i] == before);
                    assert(states[i + 1] == after);
                    assert((first as int + i) as usize == index);
                    assert(step(before, index, (after, Ok::<(), E>(()))) );
                }
            }
        }
        state = after;
        index += 1;
    }
    let out = (state, Ok(()));
    assert(phase_result(initial, first, end, step, out));
    out
}

pub open spec fn check_result<S, E, C>(
    state: S, index: usize, check: C, out: (S, Result<(), E>),
) -> bool
    where C: Fn(S, usize) -> (S, Result<bool, E>),
{
    match out.1 {
        Ok(()) => exists|present: bool| #[trigger] check.ensures((state, index), (out.0, Ok::<bool, E>(present))),
        Err(error) => check.ensures((state, index), (out.0, Err::<bool, E>(error))),
    }
}

fn check_slot<S, E, C>(state: S, index: usize, check: &C)
    -> (out: (S, Result<(), E>))
    where C: Fn(S, usize) -> (S, Result<bool, E>),
    requires check.requires((state, index)),
    ensures check_result(state, index, *check, out),
{
    let ghost initial = state;
    let (state, observed) = check(state, index);
    let ghost original = observed;
    let ghost after = state;
    let out = match observed {
        Ok(present) => {
            assert(check.ensures((initial, index), (after, Ok::<bool, E>(present))));
            (state, Ok(()))
        },
        Err(error) => (state, Err(error)),
    };
    assert(check.ensures((initial, index), (after, original)));
    out
}

pub open spec fn remove_result<S, E, C, D>(
    initial: S, index: usize, check: C, remove: D, out: (S, Result<(), E>),
) -> bool
    where C: Fn(S, usize) -> (S, Result<bool, E>),
          D: Fn(S, usize) -> (S, Result<(), E>),
{
    exists|state: S, observed: Result<bool, E>|
        #[trigger] check.ensures((initial, index), (state, observed)) && match observed {
            Err(error) => out == (state, Err::<(), E>(error)),
            Ok(false) => out == (state, Ok::<(), E>(())),
            Ok(true) => remove.ensures((state, index), out),
        }
}

fn remove_member<S, E, C, D>(state: S, index: usize, check: &C, remove: &D)
    -> (out: (S, Result<(), E>))
    where C: Fn(S, usize) -> (S, Result<bool, E>),
          D: Fn(S, usize) -> (S, Result<(), E>),
    requires check.requires((state, index)),
        forall|state: S| remove.requires((state, index)),
    ensures remove_result(state, index, *check, *remove, out),
{
    let ghost initial = state;
    let (state, observed) = check(state, index);
    let ghost original = observed;
    let ghost after_check = state;
    let out = match observed {
        Err(error) => (state, Err(error)),
        Ok(false) => (state, Ok(())),
        Ok(true) => remove(state, index),
    };
    assert(check.ensures((initial, index), (after_check, original)));
    out
}

pub open spec fn move_result<S, E, C, R>(
    initial: S, target: usize, check: C, rename: R, out: (S, Result<(), E>),
) -> bool
    where C: Fn(S, usize) -> (S, Result<bool, E>),
          R: Fn(S, usize, usize) -> (S, Result<(), E>),
{
    exists|state: S, observed: Result<bool, E>|
        #[trigger] check.ensures((initial, (target - 1) as usize), (state, observed)) && match observed {
            Err(error) => out == (state, Err::<(), E>(error)),
            Ok(false) => out == (state, Ok::<(), E>(())),
            Ok(true) => rename.ensures((state, (target - 1) as usize, target), out),
        }
}

fn move_member<S, E, C, R>(state: S, target: usize, check: &C, rename: &R)
    -> (out: (S, Result<(), E>))
    where C: Fn(S, usize) -> (S, Result<bool, E>),
          R: Fn(S, usize, usize) -> (S, Result<(), E>),
    requires target > 0, check.requires((state, (target - 1) as usize)),
        forall|state: S| #[trigger] rename.requires((state, (target - 1) as usize, target)),
    ensures move_result(state, target, *check, *rename, out),
{
    let ghost initial = state;
    let (state, observed) = check(state, target - 1);
    let ghost original = observed;
    let ghost after_check = state;
    let out = match observed {
        Err(error) => (state, Err(error)),
        Ok(false) => (state, Ok(())),
        Ok(true) => rename(state, target - 1, target),
    };
    assert(check.ensures((initial, (target - 1) as usize), (after_check, original)));
    out
}

pub open spec fn checked_phase<S, E, C>(
    initial: S, count: usize, check: C, out: (S, Result<(), E>),
) -> bool
    where C: Fn(S, usize) -> (S, Result<bool, E>),
{
    phase_result(initial, 0, count,
        |state: S, index: usize, observed: (S, Result<(), E>)|
            check_result(state, index, check, observed), out)
}

fn validate_members<S, E, C>(state: S, count: usize, check: &C)
    -> (out: (S, Result<(), E>))
    where C: Fn(S, usize) -> (S, Result<bool, E>),
    requires forall|state: S, index: usize| check.requires((state, index)),
    ensures checked_phase(state, count, *check, out),
{
    run_phase(state, 0, count,
        |state: S, index: usize| -> (out: (S, Result<(), E>))
            ensures check_result(state, index, *check, out),
        { check_slot(state, index, check) },
        Ghost(|state: S, index: usize, observed: (S, Result<(), E>)|
            check_result(state, index, *check, observed)))
}

pub open spec fn adoption_result<S, E, C, D>(
    initial: S, count: usize, check: C, remove: D, out: (S, Result<(), E>),
) -> bool
    where C: Fn(S, usize) -> (S, Result<bool, E>),
          D: Fn(S, usize) -> (S, Result<(), E>),
{
    exists|state: S, observed: Result<(), E>| #[trigger] checked_phase(initial, MAX_FILES, check, (state, observed))
        && match observed {
            Err(error) => out == (state, Err::<(), E>(error)),
            Ok(()) => phase_result(state, count, MAX_FILES,
                |before: S, index: usize, result: (S, Result<(), E>)|
                    remove_result(before, index, check, remove, result), out),
        }
}

/// 所有保留槽位先检查完毕，随后才依次重查和删除超出当前政策的成员。
pub fn adopt_members<S, E, C, D>(state: S, count: usize, check: C, remove: D)
    -> (out: (S, Result<(), E>))
    where C: Fn(S, usize) -> (S, Result<bool, E>),
          D: Fn(S, usize) -> (S, Result<(), E>),
    requires 1 <= count <= MAX_FILES,
        forall|state: S, index: usize| check.requires((state, index)),
        forall|state: S, index: usize| remove.requires((state, index)),
    ensures adoption_result(state, count, check, remove, out),
{
    let ghost initial = state;
    let (state, observed) = validate_members(state, MAX_FILES, &check);
    let ghost after_check = state;
    let ghost original = observed;
    let out = match observed {
        Err(error) => (state, Err(error)),
        Ok(()) => run_phase(state, count, MAX_FILES,
            |state: S, index: usize| -> (out: (S, Result<(), E>))
                ensures remove_result(state, index, check, remove, out),
            { remove_member(state, index, &check, &remove) },
            Ghost(|state: S, index: usize, result: (S, Result<(), E>)|
                remove_result(state, index, check, remove, result))),
    };
    assert(checked_phase(initial, MAX_FILES, check, (after_check, original)));
    out
}

pub open spec fn rotation_result<S, E, C, D, R>(
    initial: S, count: usize, check: C, remove: D, rename: R, out: (S, Result<(), E>),
) -> bool
    where C: Fn(S, usize) -> (S, Result<bool, E>),
          D: Fn(S, usize) -> (S, Result<(), E>),
          R: Fn(S, usize, usize) -> (S, Result<(), E>),
{
    exists|state: S, observed: Result<(), E>| #[trigger] checked_phase(initial, count, check, (state, observed))
        && match observed {
            Err(error) => out == (state, Err::<(), E>(error)),
            Ok(()) => exists|after: S, removed: Result<(), E>|
                #[trigger] remove_result(state, (count - 1) as usize, check, remove, (after, removed))
                && match removed {
                    Err(error) => out == (after, Err::<(), E>(error)),
                    Ok(()) => phase_result(after, 0, (count - 1) as usize,
                        |before: S, offset: usize, result: (S, Result<(), E>)|
                            move_result(before, (count - 1 - offset) as usize, check, rename, result), out),
                },
        }
}

/// 全部成员检查通过后才删除最旧成员；再按降序重查来源并搬移，保留首错。
pub fn rotate_members<S, E, C, D, R>(state: S, count: usize, check: C, remove: D, rename: R)
    -> (out: (S, Result<(), E>))
    where C: Fn(S, usize) -> (S, Result<bool, E>),
          D: Fn(S, usize) -> (S, Result<(), E>),
          R: Fn(S, usize, usize) -> (S, Result<(), E>),
    requires 1 <= count <= MAX_FILES,
        forall|state: S, index: usize| check.requires((state, index)),
        forall|state: S, index: usize| remove.requires((state, index)),
        forall|state: S, source: usize, target: usize| rename.requires((state, source, target)),
    ensures rotation_result(state, count, check, remove, rename, out),
{
    let ghost initial = state;
    let (state, observed) = validate_members(state, count, &check);
    let ghost after_check = state;
    let ghost original = observed;
    let out = match observed {
        Err(error) => (state, Err(error)),
        Ok(()) => {
            let (state, observed) = remove_member(state, count - 1, &check, &remove);
            let ghost after_remove = state;
            let ghost removed = observed;
            let out = match observed {
                Err(error) => (state, Err(error)),
                Ok(()) => run_phase(state, 0, count - 1,
                    |state: S, offset: usize| -> (out: (S, Result<(), E>))
                        requires offset < count - 1,
                        ensures move_result(state, (count - 1 - offset) as usize, check, rename, out),
                    { move_member(state, count - 1 - offset, &check, &rename) },
                    Ghost(|state: S, offset: usize, result: (S, Result<(), E>)|
                        move_result(state, (count - 1 - offset) as usize, check, rename, result))),
            };
            assert(remove_result(after_check, (count - 1) as usize, check, remove,
                (after_remove, removed)));
            out
        },
    };
    assert(checked_phase(initial, count, check, (after_check, original)));
    out
}

}
