//! 实际身份实现的边界证明。
#![allow(unexpected_cfgs)]
#[path = "../../runner/src/verified_identifiers.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

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

#[cfg_attr(not(verus_keep_ghost), test)]
fn merge_parent_identity_boundaries() {
    let base = [97u8; 40];
    let head = [98u8; 40];
    let wrong = [99u8; 40];
    let zero = [48u8; 40];
    assert(base@[0] == 97 && head@[0] == 98);
    let accepted = policy::valid_merge_parents(2, &base, &head, &head);
    assert(accepted);
    let missing_parent = policy::valid_merge_parents(1, &base, &head, &head);
    assert(!missing_parent);
    let extra_parent = policy::valid_merge_parents(3, &base, &head, &head);
    assert(!extra_parent);
    let unrelated_head = policy::valid_merge_parents(2, &base, &wrong, &head);
    assert(!unrelated_head);
    let invalid_base = policy::valid_merge_parents(2, &zero, &head, &head);
    assert(!invalid_base);
    let invalid_head = policy::valid_merge_parents(2, &base, &zero, &zero);
    assert(!invalid_head);
    let empty: [u8; 0] = [];
    let missing_head = policy::valid_merge_parents(2, &base, &empty, &head);
    assert(!missing_head);
    let base64 = [100u8; 64];
    let head64 = [101u8; 64];
    assert(base64@[0] == 100 && head64@[0] == 101);
    let accepted64 = policy::valid_merge_parents(2, &base64, &head64, &head64);
    assert(accepted64);
}

}
