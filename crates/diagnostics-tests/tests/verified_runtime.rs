//! 双输出的独立结果表、完整错误载荷和实际状态变化。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../diagnostics/src/verified_runtime.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_diagnostics::{verified_runtime as policy, verus};
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

struct Sink {
    fail_write: bool,
    fail_flush: bool,
    code: u64,
    trace: Vec<u8>,
}

struct Fault {
    primary: u64,
    related: Option<u64>,
}

spec fn same_sink(a: Sink, b: Sink) -> bool {
    a.fail_write == b.fail_write && a.fail_flush == b.fail_flush && a.code == b.code
}

spec fn single_failure(code: u64, result: Result<(), Fault>) -> bool {
    match result {
        Err(error) => error.primary == code && error.related.is_none(),
        Ok(()) => false,
    }
}

spec fn wrote(sink: Sink, out: (Sink, Result<(), Fault>)) -> bool {
    same_sink(sink, out.0) && out.0.trace@ == sink.trace@.push(1)
        && if sink.fail_write { single_failure(sink.code, out.1) } else { out.1.is_ok() }
}

spec fn flushed(sink: Sink, out: (Sink, Result<(), Fault>)) -> bool {
    same_sink(sink, out.0) && out.0.trace@ == sink.trace@.push(2)
        && if sink.fail_flush { single_failure(sink.code, out.1) } else { out.1.is_ok() }
}

spec fn mirrored(sink: Sink, out: (Sink, Result<(), Fault>)) -> bool {
    same_sink(sink, out.0)
        && if sink.fail_write {
            out.0.trace@ == sink.trace@.push(1) && single_failure(sink.code, out.1)
        } else {
            out.0.trace@ == sink.trace@.push(1).push(2)
                && if sink.fail_flush { single_failure(sink.code, out.1) } else { out.1.is_ok() }
        }
}

fn write(mut sink: Sink) -> (out: (Sink, Result<(), Fault>))
    ensures wrote(sink, out),
{
    sink.trace.push(1);
    let result = if sink.fail_write {
        Err(Fault { primary: sink.code, related: None })
    } else { Ok(()) };
    (sink, result)
}

fn flush(mut sink: Sink) -> (out: (Sink, Result<(), Fault>))
    ensures flushed(sink, out),
{
    sink.trace.push(2);
    let result = if sink.fail_flush {
        Err(Fault { primary: sink.code, related: None })
    } else { Ok(()) };
    (sink, result)
}

fn write_mirror(sink: Sink) -> (out: (Sink, Result<(), Fault>))
    ensures mirrored(sink, out),
{
    let ghost initial = sink;
    let write = |sink: Sink| -> (out: (Sink, Result<(), Fault>))
        ensures wrote(sink, out),
    { write(sink) };
    let flush = |sink: Sink| -> (out: (Sink, Result<(), Fault>))
        ensures flushed(sink, out),
    { flush(sink) };
    let out = policy::write_and_flush(sink, write, flush);
    proof {
        let (middle, first) = choose|middle: Sink, first: Result<(), Fault>|
            #[trigger] write.ensures((initial,), (middle, first))
            && match first {
                Err(error) => out == (middle, Err::<(), Fault>(error)),
                Ok(()) => flush.ensures((middle,), out),
            };
        assert(wrote(initial, (middle, first)));
        if first.is_ok() { assert(flushed(middle, out)); }
    }
    out
}

fn expect(condition: bool)
    requires condition,
{
    if !condition { panic!(); }
}

fn fresh(fail_write: bool, fail_flush: bool, code: u64) -> (sink: Sink)
    ensures sink.fail_write == fail_write, sink.fail_flush == fail_flush,
        sink.code == code, sink.trace@ == Seq::<u8>::empty(),
{
    Sink { fail_write, fail_flush, code, trace: Vec::new() }
}

fn write_case(file_failed: bool, present: bool, mirror_write_failed: bool,
    mirror_flush_failed: bool, wanted: u8, length: usize)
    requires
        wanted <= 3,
        (wanted == 0) == (!file_failed && (!present || (!mirror_write_failed && !mirror_flush_failed))),
        (wanted == 1) == (file_failed && (!present || (!mirror_write_failed && !mirror_flush_failed))),
        (wanted == 2) == (!file_failed && present && (mirror_write_failed || mirror_flush_failed)),
        (wanted == 3) == (file_failed && present && (mirror_write_failed || mirror_flush_failed)),
{
    let file = fresh(file_failed, false, u64::MAX);
    let mirror = if present { Some(fresh(mirror_write_failed, mirror_flush_failed, 0)) } else { None };
    let ghost initial_file = file;
    let ghost initial_mirror = mirror;
    let write_file = |sink: Sink| -> (out: (Sink, Result<(), Fault>))
        ensures wrote(sink, out),
    { write(sink) };
    let write_mirror = |sink: Sink| -> (out: (Sink, Result<(), Fault>))
        ensures mirrored(sink, out),
    { write_mirror(sink) };
    let combine = |file: Fault, mirror: Fault| -> (out: Fault)
        ensures out.primary == file.primary, out.related == Some(mirror.primary),
    { Fault { primary: file.primary, related: Some(mirror.primary) } };
    let (file, mirror, result) = policy::write_mirrors(
        file, mirror, length, write_file, write_mirror, combine,
    );
    let ghost observed_result = result;
    proof {
        let (first, second) = choose|first: Result<(), Fault>, second: Result<(), Fault>|
            write_file.ensures((initial_file,), (file, first))
            && policy::optional_result(initial_mirror, write_mirror, (mirror, second))
            && #[trigger] policy::combined_result(first, second, length, combine, result);
        assert(wrote(initial_file, (file, first)));
        if let Some(initial) = initial_mirror {
            let after = choose|after: Sink|
                #[trigger] write_mirror.ensures((initial,), (after, second)) && mirror == Some(after);
            assert(mirrored(initial, (after, second)));
        }
        if let (Err(file_error), Err(mirror_error)) = (first, second) {
            match observed_result {
                Err(error) => assert(combine.ensures((file_error, mirror_error), error)),
                Ok(_) => assert(false),
            }
        }
    }
    expect(file.fail_write == file_failed);
    expect(!file.fail_flush);
    expect(file.code == u64::MAX);
    expect(file.trace.len() == 1);
    expect(file.trace[0] == 1);
    match mirror {
        None => expect(!present),
        Some(mirror) => {
            expect(present);
            expect(mirror.fail_write == mirror_write_failed);
            expect(mirror.fail_flush == mirror_flush_failed);
            expect(mirror.code == 0);
            expect(mirror.trace.len() == if mirror_write_failed { 1 } else { 2 });
            expect(mirror.trace[0] == 1);
            if !mirror_write_failed { expect(mirror.trace[1] == 2); }
        },
    }
    match result {
        Ok(actual_length) => { expect(wanted == 0); expect(actual_length == length); },
        Err(error) => {
            expect(wanted != 0);
            expect(error.primary == if wanted == 2 { 0 } else { u64::MAX });
            expect(error.related == if wanted == 3 { Some(0) } else { None });
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn write_outcome_table_keeps_every_attempt_error_and_success_length() {
    write_case(false, false, false, false, 0, 0);
    write_case(false, false, false, true, 0, usize::MAX);
    write_case(false, false, true, false, 0, 1);
    write_case(false, false, true, true, 0, 17);
    write_case(true, false, false, false, 1, 0);
    write_case(true, false, false, true, 1, usize::MAX);
    write_case(true, false, true, false, 1, 1);
    write_case(true, false, true, true, 1, 17);
    write_case(false, true, false, false, 0, usize::MAX);
    write_case(false, true, false, true, 2, 0);
    write_case(false, true, true, false, 2, 1);
    write_case(false, true, true, true, 2, 17);
    write_case(true, true, false, false, 1, usize::MAX);
    write_case(true, true, false, true, 3, 0);
    write_case(true, true, true, false, 3, 1);
    write_case(true, true, true, true, 3, 17);
}

fn flush_case(file_failed: bool, present: bool, mirror_failed: bool, wanted: u8)
    requires wanted <= 2,
        (wanted == 0) == (!file_failed && (!present || !mirror_failed)),
        (wanted == 1) == file_failed,
        (wanted == 2) == (!file_failed && present && mirror_failed),
{
    let file = fresh(false, file_failed, 0);
    let mirror = if present { Some(fresh(false, mirror_failed, u64::MAX)) } else { None };
    let ghost initial_file = file;
    let ghost initial_mirror = mirror;
    let flush_file = |sink: Sink| -> (out: (Sink, Result<(), Fault>))
        ensures flushed(sink, out),
    { flush(sink) };
    let flush_mirror = |sink: Sink| -> (out: (Sink, Result<(), Fault>))
        ensures flushed(sink, out),
    { flush(sink) };
    let (file, mirror, result) = policy::flush_mirrors(file, mirror, flush_file, flush_mirror);
    proof {
        let first = choose|first: Result<(), Fault>|
            #[trigger] flush_file.ensures((initial_file,), (file, first))
            && match first {
                Err(error) => mirror == initial_mirror && result == Err::<(), Fault>(error),
                Ok(()) => policy::optional_result(initial_mirror, flush_mirror, (mirror, result)),
            };
        assert(flushed(initial_file, (file, first)));
        if first.is_ok() {
            if let Some(initial) = initial_mirror {
                let after = choose|after: Sink|
                    #[trigger] flush_mirror.ensures((initial,), (after, result)) && mirror == Some(after);
                assert(flushed(initial, (after, result)));
            }
        }
    }
    expect(file.code == 0);
    expect(file.fail_flush == file_failed);
    expect(!file.fail_write);
    expect(file.trace.len() == 1);
    expect(file.trace[0] == 2);
    match mirror {
        None => expect(!present),
        Some(mirror) => {
            expect(present);
            expect(mirror.code == u64::MAX);
            expect(mirror.fail_flush == mirror_failed);
            expect(!mirror.fail_write);
            expect(mirror.trace.len() == if file_failed { 0 } else { 1 });
            if !file_failed { expect(mirror.trace[0] == 2); }
        },
    }
    match result {
        Ok(()) => expect(wanted == 0),
        Err(error) => {
            expect(wanted != 0);
            expect(error.primary == if wanted == 1 { 0 } else { u64::MAX });
            expect(error.related.is_none());
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn flush_outcome_table_stops_after_the_first_failure() {
    flush_case(false, false, false, 0);
    flush_case(false, false, true, 0);
    flush_case(false, true, false, 0);
    flush_case(false, true, true, 2);
    flush_case(true, false, false, 1);
    flush_case(true, false, true, 1);
    flush_case(true, true, false, 1);
    flush_case(true, true, true, 1);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn write_then_flush_appends_to_the_actual_previous_state() {
    let mut sink = fresh(false, true, 17);
    sink.trace.push(99);
    let (sink, result) = write_mirror(sink);
    expect(sink.trace.len() == 3);
    expect(sink.trace[0] == 99);
    expect(sink.trace[1] == 1);
    expect(sink.trace[2] == 2);
    match result {
        Err(error) => { expect(error.primary == 17); expect(error.related.is_none()); },
        Ok(()) => { proof { assert(false); } panic!(); },
    }
}

}
