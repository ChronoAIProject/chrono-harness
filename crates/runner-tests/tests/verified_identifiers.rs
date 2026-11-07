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
fn digest_lengths_and_allowed_bytes() {
    let empty: [u8; 0] = [];
    let missing = policy::valid_digest(&empty);
    assert(!missing);
    let oid_length = [97u8; 40];
    let only_oid = policy::valid_digest(&oid_length);
    assert(!only_oid);
    let short = [48u8; 63];
    let too_short = policy::valid_digest(&short);
    assert(!too_short);
    let long = [48u8; 65];
    let too_long = policy::valid_digest(&long);
    assert(!too_long);
    let zero = [48u8; 64];
    let zero_digest = policy::valid_digest(&zero);
    assert(zero_digest);
    let nine = [57u8; 64];
    let digit_end = policy::valid_digest(&nine);
    assert(digit_end);
    let a = [97u8; 64];
    let lower_start = policy::valid_digest(&a);
    assert(lower_start);
    let f = [102u8; 64];
    let lower_end = policy::valid_digest(&f);
    assert(lower_end);
}

fn reject_digest_byte_at(at: usize, byte: u8)
    requires at < 64,
        byte < b'0' || (b'9' < byte && byte < b'a') || b'f' < byte,
{
    let mut bytes = [48u8; 64];
    bytes[at] = byte;
    let accepted = policy::valid_digest(&bytes);
    assert(bytes@[at as int] == byte);
    assert(!policy::oid_byte(bytes@[at as int]));
    assert(!accepted);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn digest_rejects_bad_bytes_throughout_the_full_input() {
    reject_digest_byte_at(0, 47);
    reject_digest_byte_at(1, 58);
    reject_digest_byte_at(9, 64);
    reject_digest_byte_at(31, 65);
    reject_digest_byte_at(32, 96);
    reject_digest_byte_at(63, 103);
    reject_digest_byte_at(7, 0);
    reject_digest_byte_at(42, 10);
    reject_digest_byte_at(62, 128);
    reject_digest_byte_at(17, 195);
    reject_digest_byte_at(61, 255);
    let mut utf8 = [48u8; 64];
    utf8[0] = 195;
    utf8[1] = 169;
    let unicode = policy::valid_digest(&utf8);
    assert(!unicode);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn digest_text_uses_the_complete_utf8_bytes() {
    let text = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let accepted = policy::valid_digest_text(text);
    assert(accepted);
    let empty = policy::valid_digest_text("");
    assert(!empty);
}

}
