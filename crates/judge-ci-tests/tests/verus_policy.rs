//! Verified success, rejection and boundary expectations for the actual predicate.
#![allow(unexpected_cfgs)]
#[path = "../../judge-ci/src/verus_policy.rs"]
mod policy;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn complete_inventory_is_accepted() {
    let obligations = vec![true, true, true];
    let result = policy::complete(&obligations);
    assert(result);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn empty_inventory_is_rejected() {
    let obligations: Vec<bool> = Vec::new();
    let result = policy::complete(&obligations);
    assert(!result);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn failure_identity_is_the_first_missing_obligation() {
    let obligations = vec![true, false, false];
    let result = policy::complete(&obligations);
    assert(!result);
    let first = policy::first_unverified(&obligations);
    assert(first == Some(1usize));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn first_and_last_obligations_cannot_be_skipped() {
    let first = vec![false, true];
    let last = vec![true, false];
    let a = policy::first_unverified(&first);
    assert(a == Some(0usize));
    let b = policy::first_unverified(&last);
    assert(b == Some(1usize));
}

}
