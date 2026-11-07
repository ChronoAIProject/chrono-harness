//! 实际身份判断及其精确 Verus 合同。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

verus! {

/// 只检查请求头；judge 名称沿用非空合同，不套用 CI 单元的 ASCII ID 限制。
pub fn valid_request_header(
    protocol: &str, mode: &str, judge_id: &str,
    expected_protocol: &str, expected_mode: &str,
) -> (valid: bool)
    ensures valid <==> protocol@ == expected_protocol@ && mode@ == expected_mode@
        && judge_id@.len() > 0,
{
    equal_text(protocol, expected_protocol) && equal_text(mode, expected_mode)
        && !judge_id.is_empty()
}

pub fn equal_text(actual: &str, expected: &str) -> (equal: bool)
    ensures equal <==> actual@ == expected@,
{
    let equal = equal_bytes(actual.as_bytes(), expected.as_bytes());
    proof {
        vstd::utf8::encode_utf8_decode_utf8(actual@);
        vstd::utf8::encode_utf8_decode_utf8(expected@);
    }
    equal
}

pub fn equal_bytes(actual: &[u8], expected: &[u8]) -> (equal: bool)
    ensures equal <==> actual@ == expected@,
{
    if actual.len() != expected.len() {
        return false;
    }
    let mut i = 0;
    while i < actual.len()
        invariant
            actual.len() == expected.len(),
            0 <= i <= actual.len(),
            forall|j: int| 0 <= j < i ==> actual[j] == expected[j],
        decreases actual.len() - i,
    {
        if actual[i] != expected[i] {
            return false;
        }
        i += 1;
    }
    assert(actual@ =~= expected@);
    true
}



#[verifier::inline]
pub open spec fn oid_byte(byte: u8) -> bool {
    48 <= byte <= 57 || 97 <= byte <= 102
}

#[verifier::inline]
pub open spec fn digest_format(bytes: Seq<u8>) -> bool {
    bytes.len() == 64
        && forall|i: int| 0 <= i < bytes.len() ==> oid_byte(#[trigger] bytes[i])
}

pub fn valid_digest(bytes: &[u8]) -> (valid: bool)
    ensures valid == digest_format(bytes@),
{
    if bytes.len() != 64 {
        return false;
    }
    let mut i = 0;
    while i < bytes.len()
        invariant
            0 <= i <= bytes.len(),
            bytes.len() == 64,
            forall|j: int| 0 <= j < i ==> oid_byte(#[trigger] bytes[j]),
        decreases bytes.len() - i,
    {
        let byte = bytes[i];
        if !((48 <= byte && byte <= 57) || (97 <= byte && byte <= 102)) {
            return false;
        }
        i += 1;
    }
    true
}

pub fn valid_digest_text(text: &str) -> (valid: bool)
    ensures valid == digest_format(text.spec_bytes()),
{
    valid_digest(text.as_bytes())
}

#[verifier::inline]
pub open spec fn oid(bytes: Seq<u8>) -> bool {
    (bytes.len() == 40 || bytes.len() == 64)
        && (forall|i: int| 0 <= i < bytes.len() ==> oid_byte(#[trigger] bytes[i]))
        && (exists|i: int| 0 <= i < bytes.len() && bytes[i] != 48)
}

pub fn valid_oid(bytes: &[u8]) -> (valid: bool)
    ensures valid == oid(bytes@),
{
    if bytes.len() != 40 && bytes.len() != 64 {
        return false;
    }
    let mut i = 0;
    let mut nonzero = false;
    while i < bytes.len()
        invariant
            0 <= i <= bytes.len(),
            bytes.len() == 40 || bytes.len() == 64,
            forall|j: int| 0 <= j < i ==> oid_byte(#[trigger] bytes[j]),
            nonzero == (exists|j: int| 0 <= j < i && bytes[j] != 48),
        decreases bytes.len() - i,
    {
        let byte = bytes[i];
        if !((48 <= byte && byte <= 57) || (97 <= byte && byte <= 102)) {
            return false;
        }
        nonzero = nonzero || byte != 48;
        i += 1;
    }
    nonzero
}

pub fn valid_merge_parents(
    count: usize, base: &[u8], second: &[u8], head: &[u8],
) -> (valid: bool)
    ensures valid <==> count == 2 && oid(base@) && oid(head@) && second@ == head@,
{
    count == 2 && valid_oid(base) && valid_oid(head) && equal_bytes(second, head)
}


#[verifier::inline]
pub open spec fn identifier_byte(byte: u8) -> bool {
    48 <= byte <= 57 || 65 <= byte <= 90 || 97 <= byte <= 122
        || byte == 46 || byte == 95 || byte == 45
}

#[verifier::inline]
pub open spec fn identifier(bytes: Seq<u8>) -> bool {
    bytes.len() > 0
        && forall|i: int| 0 <= i < bytes.len() ==> identifier_byte(#[trigger] bytes[i])
}

pub fn valid_id(bytes: &[u8]) -> (valid: bool)
    ensures valid == identifier(bytes@),
{
    if bytes.len() == 0 {
        return false;
    }
    let mut i = 0;
    while i < bytes.len()
        invariant
            0 <= i <= bytes.len(),
            bytes.len() > 0,
            forall|j: int| 0 <= j < i ==> identifier_byte(#[trigger] bytes[j]),
        decreases bytes.len() - i,
    {
        let byte = bytes[i];
        if !((48 <= byte && byte <= 57) || (65 <= byte && byte <= 90)
            || (97 <= byte && byte <= 122) || byte == 46 || byte == 95 || byte == 45) {
            return false;
        }
        i += 1;
    }
    true
}


}
