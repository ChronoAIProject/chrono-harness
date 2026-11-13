//! Preparation must return its durable plan before any projection effect is dispatched.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub open spec fn evaluation<S, P, T, E, Prepare, Apply>(
    initial: S, prepare: Prepare, apply: Apply, out: (S, Result<T, E>),
) -> bool
    where Prepare: FnOnce(S) -> (S, Result<P, E>),
          Apply: FnOnce(S, P) -> (S, Result<T, E>),
{
    exists|state: S, prepared: Result<P, E>|
        #[trigger] prepare.ensures((initial,), (state, prepared))
        && match prepared {
            Err(error) => out == (state, Err::<T, E>(error)),
            Ok(plan) => apply.ensures((state, plan), out),
        }
}

pub fn prepare_then_apply<S, P, T, E, Prepare, Apply>(
    initial: S, prepare: Prepare, apply: Apply,
) -> (out: (S, Result<T, E>))
    where Prepare: FnOnce(S) -> (S, Result<P, E>),
          Apply: FnOnce(S, P) -> (S, Result<T, E>),
    requires prepare.requires((initial,)),
        forall|state: S, plan: P| apply.requires((state, plan)),
    ensures evaluation(initial, prepare, apply, out),
{
    let ghost before = initial;
    let (state, prepared) = prepare(initial);
    let ghost middle = state;
    let ghost original = prepared;
    let out = match prepared {
        Err(error) => (state, Err(error)),
        Ok(plan) => apply(state, plan),
    };
    assert(prepare.ensures((before,), (middle, original)));
    out
}
}
