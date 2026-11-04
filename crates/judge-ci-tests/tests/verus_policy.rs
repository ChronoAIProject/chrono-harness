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

#[cfg_attr(not(verus_keep_ghost), test)]
fn oid_boundaries() {
    let zero_bytes = [48u8; 40];
    let zero = policy::valid_oid(&zero_bytes);
    assert(!zero);
    let nonzero_bytes = [97u8; 40];
    let nonzero = policy::valid_oid(&nonzero_bytes);
    assert(nonzero_bytes@[0] == 97);
    assert(nonzero);
    let uppercase_bytes = [65u8; 40];
    let uppercase = policy::valid_oid(&uppercase_bytes);
    assert(!uppercase);
    let empty: [u8; 0] = [];
    let too_short = policy::valid_oid(&empty);
    assert(!too_short);
    let long_bytes = [102u8; 64];
    let long = policy::valid_oid(&long_bytes);
    assert(long_bytes@[0] == 102);
    assert(long);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn identifier_ascii_boundaries() {
    let bytes = [48u8, 57u8, 65u8, 90u8, 97u8, 122u8, 46u8, 95u8, 45u8];
    let accepted = policy::valid_id(&bytes);
    assert(accepted);
    let empty: [u8; 0] = [];
    let missing = policy::valid_id(&empty);
    assert(!missing);
    let slash = [47u8];
    let invalid_separator = policy::valid_id(&slash);
    assert(!invalid_separator);
    let non_ascii = [128u8];
    let invalid_unicode = policy::valid_id(&non_ascii);
    assert(!invalid_unicode);
}

}
