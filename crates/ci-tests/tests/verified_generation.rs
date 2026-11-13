//! Independent complete call traces and original errors, in Rust and Verus.
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../ci/src/verified_generation.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_ci::verified_generation as policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
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

fn case(prepare_fails: bool, apply_fails: bool) {
    let prepare = |mut trace: Vec<u8>| -> (out: (Vec<u8>, Result<u64, (u64, u64)>))
        ensures out.0@ == trace@.push(1),
            out.1 == if prepare_fails { Err::<u64, (u64, u64)>((u64::MAX, 0)) } else { Ok::<u64, (u64, u64)>(u64::MAX) },
    {
        trace.push(1);
        (trace, if prepare_fails { Err((u64::MAX, 0)) } else { Ok(u64::MAX) })
    };
    let apply = |mut trace: Vec<u8>, plan: u64| -> (out: (Vec<u8>, Result<u64, (u64, u64)>))
        ensures out.0@ == trace@.push(2),
            out.1 == if apply_fails { Err::<u64, (u64, u64)>((plan, 1)) } else { Ok::<u64, (u64, u64)>(plan) },
    {
        trace.push(2);
        (trace, if apply_fails { Err((plan, 1)) } else { Ok(plan) })
    };
    let initial: Vec<u8> = Vec::new();
    let ghost before = initial;
    let (trace, result) = policy::prepare_then_apply(initial, prepare, apply);
    proof {
        let (middle, prepared) = choose|state: Vec<u8>, prepared: Result<u64, (u64, u64)>|
            #[trigger] prepare.ensures((before,), (state, prepared))
            && match prepared {
                Err(error) => (trace, result) == (state, Err::<u64, (u64, u64)>(error)),
                Ok(plan) => apply.ensures((state, plan), (trace, result)),
            };
        assert(middle@ == seq![1u8]);
        if !prepare_fails { assert(prepared == Ok::<u64, (u64, u64)>(u64::MAX)); }
    }
    expect(trace.len() == if prepare_fails { 1 } else { 2 });
    expect(trace[0] == 1);
    if !prepare_fails { expect(trace[1] == 2); }
    match result {
        Err((code, source)) => {
            expect(prepare_fails || apply_fails);
            expect(code == u64::MAX);
            expect(source == if prepare_fails { 0 } else { 1 });
        },
        Ok(value) => { expect(!prepare_fails && !apply_fails); expect(value == u64::MAX); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn preparation_success_and_failure_keep_order_and_original_result() {
    case(false, false);
    case(false, true);
    case(true, false);
    case(true, true);
}

#[cfg(not(verus_keep_ghost))]
#[test]
#[should_panic]
fn runtime_expectation_keeps_its_failure_branch() { expect(false); }
}
