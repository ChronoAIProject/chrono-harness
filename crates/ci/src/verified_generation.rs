//! Preparation must return its durable plan before any projection effect is dispatched.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;

verus! {
#[cfg_attr(not(verus_keep_ghost), derive(Clone, Debug, serde::Serialize, serde::Deserialize))]
pub struct Object {
    pub identity: String,
    pub sha256: String,
    pub bytes: u64,
}

pub open spec fn same_object_spec(first: Option<&Object>, second: Option<&Object>) -> bool {
    match (first, second) {
        (None, None) => true,
        (Some(left), Some(right)) => left.identity@ == right.identity@
            && left.sha256@ == right.sha256@ && left.bytes == right.bytes,
        _ => false,
    }
}

/// A missing original and an observed file never identify the same object.
pub fn same_object(first: Option<&Object>, second: Option<&Object>) -> (same: bool)
    ensures same == same_object_spec(first, second),
{
    match (first, second) {
        (None, None) => true,
        (Some(left), Some(right)) => identifiers::equal_text(&left.identity, &right.identity)
            && identifiers::equal_text(&left.sha256, &right.sha256) && left.bytes == right.bytes,
        _ => false,
    }
}

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
