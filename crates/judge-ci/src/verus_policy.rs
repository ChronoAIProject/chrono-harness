//! The executable admission predicate and its Verus specification share this source.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

/// Empty inventories cannot establish coverage. For a nonempty inventory,
/// return the first unsatisfied obligation, retaining its caller-owned identity.
pub fn first_unverified(obligations: &Vec<bool>) -> (result: Option<usize>)
    ensures
        result.is_none() <==> (forall|i: int| 0 <= i < obligations.len() ==> obligations[i]),
        result.is_some() ==> (
            0 <= result.unwrap() < obligations.len()
            && !obligations[result.unwrap() as int]
            && forall|i: int| 0 <= i < result.unwrap() ==> obligations[i]
        ),
{
    let mut i = 0;
    while i < obligations.len()
        invariant
            0 <= i <= obligations.len(),
            forall|j: int| 0 <= j < i ==> obligations[j],
        decreases obligations.len() - i,
    {
        if !obligations[i] {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Absence of obligations is a registration failure, not a passing proof.
pub fn complete(obligations: &Vec<bool>) -> (result: bool)
    ensures result <==> (
        obligations.len() > 0
        && forall|i: int| 0 <= i < obligations.len() ==> obligations[i]
    ),
{
    !obligations.is_empty() && first_unverified(obligations).is_none()
}

#[verifier::inline]
pub open spec fn oid_byte(byte: u8) -> bool {
    48 <= byte <= 57 || 97 <= byte <= 102
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
