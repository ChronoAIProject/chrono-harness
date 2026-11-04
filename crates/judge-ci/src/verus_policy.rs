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

/// Return every failed obligation exactly once, in inventory order.
pub fn missing_obligations(obligations: &Vec<bool>) -> (missing: Vec<usize>)
    ensures
        forall|j: int| 0 <= j < missing.len() ==> (
            0 <= #[trigger] missing[j] < obligations.len() && !obligations[missing[j] as int]
        ),
        forall|i: int| 0 <= i < obligations.len() ==> (
            !obligations[i] <==> missing@.contains(i as usize)
        ),
        forall|a: int, b: int| 0 <= a < b < missing.len() ==> missing[a] < missing[b],
        missing.len() == 0 <==> (forall|i: int| 0 <= i < obligations.len() ==> obligations[i]),
{
    let mut missing = Vec::new();
    let mut i = 0;
    while i < obligations.len()
        invariant
            0 <= i <= obligations.len(),
            forall|j: int| 0 <= j < missing.len() ==> (
                0 <= #[trigger] missing[j] < i && !obligations[missing[j] as int]
            ),
            forall|j: int| 0 <= j < i ==> (
                !obligations[j] <==> missing@.contains(j as usize)
            ),
            forall|a: int, b: int| 0 <= a < b < missing.len() ==> missing[a] < missing[b],
            missing.len() == 0 <==> (forall|j: int| 0 <= j < i ==> obligations[j]),
        decreases obligations.len() - i,
    {
        proof {
            assert(!missing@.contains(i)) by {
                if missing@.contains(i) {
                    let j = choose|j: int| 0 <= j < missing.len() && missing[j] == i;
                    assert(missing[j] < i);
                }
            }
        }
        let ghost before = missing@;
        if !obligations[i] {
            missing.push(i);
        }
        proof {
            assert forall|j: int| 0 <= j < i + 1 implies (
                !obligations[j] <==> missing@.contains(j as usize)
            ) by {
                if !obligations[i as int] {
                    vstd::seq_lib::lemma_seq_contains_after_push(before, i, j as usize);
                }
                if j < i {
                    assert(!obligations[j] <==> before.contains(j as usize));
                }
            }
        }
        i += 1;
    }
    missing
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

/// External JSON decoding supplies optional typed fields. Missing/wrongly
/// typed fields stay absent; the repository owns and proves their admission.
pub struct CompilerResult<'a> {
    pub exit_code: Option<i64>,
    pub success: Option<bool>,
    pub encountered_error: Option<bool>,
    pub encountered_vir_error: Option<bool>,
    pub errors: Option<u64>,
    pub verified: Option<u64>,
    pub entire_crate: Option<bool>,
    pub version: Option<&'a [u8]>,
    pub commit: Option<&'a [u8]>,
}

pub open spec fn compiler_accepted(
    result: &CompilerResult,
    version: Seq<u8>,
    commit: Seq<u8>,
) -> bool {
    version.len() > 0 && commit.len() > 0
        && result.exit_code == Some(0i64)
        && result.success == Some(true)
        && result.encountered_error == Some(false)
        && result.encountered_vir_error == Some(false)
        && result.errors == Some(0u64)
        && result.verified.is_some() && result.verified.unwrap() > 0
        && result.entire_crate == Some(true)
        && result.version.is_some() && result.version.unwrap()@ == version
        && result.commit.is_some() && result.commit.unwrap()@ == commit
}

pub fn accept_compiler_result(
    result: &CompilerResult,
    version: &[u8],
    commit: &[u8],
) -> (accepted: bool)
    ensures accepted <==> compiler_accepted(result, version@, commit@),
{
    if version.len() == 0 || commit.len() == 0
        || result.exit_code != Some(0i64)
        || result.success != Some(true)
        || result.encountered_error != Some(false)
        || result.encountered_vir_error != Some(false)
        || result.errors != Some(0u64)
        || result.entire_crate != Some(true)
    {
        return false;
    }
    match result.verified {
        Some(count) => { if count == 0 { return false; } },
        None => { return false; },
    }
    match (result.version, result.commit) {
        (Some(actual_version), Some(actual_commit)) => {
            equal_bytes(actual_version, version) && equal_bytes(actual_commit, commit)
        },
        _ => false,
    }
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
