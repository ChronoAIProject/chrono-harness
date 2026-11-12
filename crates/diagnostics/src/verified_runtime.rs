//! 双输出的真实调用顺序和结果组合；具体 Write 接口只转交原始结果。
#[cfg(not(verus_keep_ghost))]
use crate::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

pub open spec fn sequence_result<S, E, W, F>(
    initial: S, write: W, flush: F, out: (S, Result<(), E>),
) -> bool
    where W: FnOnce(S) -> (S, Result<(), E>), F: FnOnce(S) -> (S, Result<(), E>),
{
    exists|middle: S, first: Result<(), E>|
        #[trigger] write.ensures((initial,), (middle, first))
        && match first {
            Err(error) => out == (middle, Err::<(), E>(error)),
            Ok(()) => flush.ensures((middle,), out),
        }
}

/// 写入失败时不刷新；成功时只把真实返回的状态交给刷新接口。
pub fn write_and_flush<S, E, W, F>(state: S, write: W, flush: F)
    -> (out: (S, Result<(), E>))
    where W: FnOnce(S) -> (S, Result<(), E>), F: FnOnce(S) -> (S, Result<(), E>),
    requires write.requires((state,)), forall|state: S| flush.requires((state,)),
    ensures sequence_result(state, write, flush, out),
{
    let ghost initial = state;
    let (state, result) = write(state);
    let ghost middle = state;
    let ghost first = result;
    let out = match result {
        Err(error) => (state, Err(error)),
        Ok(success) => {
            assert(success == ());
            flush(state)
        },
    };
    assert(write.ensures((initial,), (middle, first)));
    out
}

pub open spec fn optional_result<S, E, F>(
    initial: Option<S>, operation: F, out: (Option<S>, Result<(), E>),
) -> bool
    where F: FnOnce(S) -> (S, Result<(), E>),
{
    match initial {
        None => out == (None::<S>, Ok::<(), E>(())),
        Some(state) => exists|after: S|
            #[trigger] operation.ensures((state,), (after, out.1)) && out.0 == Some(after),
    }
}

pub fn run_optional<S, E, F>(state: Option<S>, operation: F)
    -> (out: (Option<S>, Result<(), E>))
    where F: FnOnce(S) -> (S, Result<(), E>),
    requires forall|state: S| operation.requires((state,)),
    ensures optional_result(state, operation, out),
{
    match state {
        None => (None, Ok(())),
        Some(state) => {
            let ghost before = state;
            let (state, result) = operation(state);
            let ghost after = state;
            let out = (Some(state), result);
            assert(operation.ensures((before,), (after, out.1)));
            assert(out.0 == Some(after));
            out
        },
    }
}

pub open spec fn combined_result<E, C>(
    file: Result<(), E>, mirror: Result<(), E>, length: usize, combine: C,
    out: Result<usize, E>,
) -> bool
    where C: FnOnce(E, E) -> E,
{
    match (file, mirror) {
        (Ok(()), Ok(())) => out == Ok::<usize, E>(length),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => out == Err::<usize, E>(error),
        (Err(file), Err(mirror)) => match out {
            Err(error) => combine.ensures((file, mirror), error),
            Ok(_) => false,
        },
    }
}

pub fn combine_results<E, C>(
    file: Result<(), E>, mirror: Result<(), E>, length: usize, combine: C,
) -> (out: Result<usize, E>)
    where C: FnOnce(E, E) -> E,
    requires forall|file: E, mirror: E| combine.requires((file, mirror)),
    ensures combined_result(file, mirror, length, combine, out),
{
    match (file, mirror) {
        (Ok(()), Ok(())) => Ok(length),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(file), Err(mirror)) => Err(combine(file, mirror)),
    }
}

pub open spec fn mirrored_write_result<A, B, E, W, M, C>(
    file: A, mirror: Option<B>, length: usize, write_file: W, write_mirror: M,
    combine: C, out: (A, Option<B>, Result<usize, E>),
) -> bool
    where W: FnOnce(A) -> (A, Result<(), E>),
        M: FnOnce(B) -> (B, Result<(), E>), C: FnOnce(E, E) -> E,
{
    exists|file_result: Result<(), E>, mirror_result: Result<(), E>|
        write_file.ensures((file,), (out.0, file_result))
        && optional_result(mirror, write_mirror, (out.1, mirror_result))
        && #[trigger] combined_result(file_result, mirror_result, length, combine, out.2)
}

/// 两个输出都尝试；文件写入失败仍尝试镜像。两个错误才调用组合所有者。
pub fn write_mirrors<A, B, E, W, M, C>(
    file: A, mirror: Option<B>, length: usize, write_file: W, write_mirror: M, combine: C,
) -> (out: (A, Option<B>, Result<usize, E>))
    where W: FnOnce(A) -> (A, Result<(), E>),
        M: FnOnce(B) -> (B, Result<(), E>), C: FnOnce(E, E) -> E,
    requires write_file.requires((file,)),
        forall|mirror: B| write_mirror.requires((mirror,)),
        forall|file: E, mirror: E| combine.requires((file, mirror)),
    ensures mirrored_write_result(file, mirror, length, write_file, write_mirror, combine, out),
{
    let ghost initial_file = file;
    let ghost initial_mirror = mirror;
    let (file, file_result) = write_file(file);
    let ghost observed_file = file_result;
    let (mirror, mirror_result) = run_optional(mirror, write_mirror);
    let ghost observed_mirror = mirror_result;
    let result = combine_results(file_result, mirror_result, length, combine);
    let out = (file, mirror, result);
    assert(write_file.ensures((initial_file,), (out.0, observed_file)));
    assert(optional_result(initial_mirror, write_mirror, (out.1, observed_mirror)));
    assert(combined_result(observed_file, observed_mirror, length, combine, out.2));
    out
}

pub open spec fn mirrored_flush_result<A, B, E, F, M>(
    file: A, mirror: Option<B>, flush_file: F, flush_mirror: M,
    out: (A, Option<B>, Result<(), E>),
) -> bool
    where F: FnOnce(A) -> (A, Result<(), E>), M: FnOnce(B) -> (B, Result<(), E>),
{
    exists|first: Result<(), E>|
        #[trigger] flush_file.ensures((file,), (out.0, first))
        && match first {
            Err(error) => out.1 == mirror && out.2 == Err::<(), E>(error),
            Ok(()) => optional_result(mirror, flush_mirror, (out.1, out.2)),
        }
}

/// 独立 flush 保留首错：文件失败时保留镜像状态且不调用它。
pub fn flush_mirrors<A, B, E, F, M>(
    file: A, mirror: Option<B>, flush_file: F, flush_mirror: M,
) -> (out: (A, Option<B>, Result<(), E>))
    where F: FnOnce(A) -> (A, Result<(), E>), M: FnOnce(B) -> (B, Result<(), E>),
    requires flush_file.requires((file,)),
        forall|mirror: B| flush_mirror.requires((mirror,)),
    ensures mirrored_flush_result(file, mirror, flush_file, flush_mirror, out),
{
    let ghost initial_file = file;
    let (file, first) = flush_file(file);
    let ghost observed_first = first;
    let out = match first {
        Err(error) => (file, mirror, Err(error)),
        Ok(success) => {
            assert(success == ());
            let (mirror, result) = run_optional(mirror, flush_mirror);
            (file, mirror, result)
        },
    };
    assert(flush_file.ensures((initial_file,), (out.0, observed_first)));
    out
}

}
