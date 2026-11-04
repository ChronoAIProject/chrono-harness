//! The executable admission predicate and its Verus specification share this source.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

/// One record binds the finding identity to the decision which produced it.
pub struct Obligation {
    pub identity: String,
    pub accepted: bool,
}

/// The identity sequence preserves every rejected record, including duplicates.
pub open spec fn rejected_identities(entries: Seq<Obligation>) -> Seq<String>
    decreases entries.len(),
{
    if entries.len() == 0 {
        Seq::empty()
    } else {
        let earlier = rejected_identities(entries.drop_last());
        if entries.last().accepted {
            earlier
        } else {
            earlier.push(entries.last().identity)
        }
    }
}

pub struct Coverage {
    pub obligations: usize,
    pub complete: bool,
    pub missing: Vec<String>,
}

impl Coverage {
    pub fn first_missing(&self) -> (first: Option<&str>)
        ensures
            first.is_none() <==> self.missing.len() == 0,
            first.is_some() ==> first.unwrap()@ == self.missing[0]@,
    {
        if self.missing.len() == 0 {
            None
        } else {
            Some(self.missing[0].as_str())
        }
    }
}

/// The private vector is the sole owner of coverage registration state.
/// Finishing consumes the ledger, so no later record can change that result.
pub struct Ledger {
    entries: Vec<Obligation>,
}

impl Ledger {
    pub closed spec fn view(&self) -> Seq<Obligation> {
        self.entries@
    }

    pub fn new() -> (ledger: Ledger)
        ensures ledger.view() == Seq::<Obligation>::empty(),
    {
        Ledger { entries: Vec::new() }
    }

    pub fn record(&mut self, identity: String, accepted: bool)
        ensures self.view() == old(self).view().push(Obligation { identity, accepted }),
    {
        self.entries.push(Obligation { identity, accepted });
    }

    pub fn finish(self) -> (coverage: Coverage)
        ensures
            coverage.obligations == self.view().len(),
            coverage.missing@ == rejected_identities(self.view()),
            coverage.complete <==> (
                self.view().len() > 0
                && forall|i: int| 0 <= i < self.view().len() ==> #[trigger] self.view()[i].accepted
            ),
            coverage.missing.len() == 0 <==> (
                forall|i: int| 0 <= i < self.view().len() ==> #[trigger] self.view()[i].accepted
            ),
    {
        let mut missing = Vec::new();
        let mut i = 0;
        while i < self.entries.len()
            invariant
                0 <= i <= self.entries.len(),
                missing@ == rejected_identities(self.entries@.take(i as int)),
                missing.len() == 0 <==> (
                    forall|j: int| 0 <= j < i ==> #[trigger] self.entries[j].accepted
                ),
            decreases self.entries.len() - i,
        {
            if !self.entries[i].accepted {
                missing.push(self.entries[i].identity.clone());
            }
            proof {
                let prefix = self.entries@.take(i + 1);
                assert(prefix.drop_last() =~= self.entries@.take(i as int));
                assert(prefix.last() == self.entries@[i as int]);
            }
            i += 1;
        }
        assert(self.entries@.take(i as int) =~= self.entries@);
        let complete = self.entries.len() > 0 && missing.len() == 0;
        Coverage { obligations: self.entries.len(), complete, missing }
    }
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
