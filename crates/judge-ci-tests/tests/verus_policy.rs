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
    let mut ledger = policy::Ledger::new();
    ledger.record("first".to_owned(), true);
    ledger.record("last".to_owned(), true);
    let coverage = ledger.finish();
    assert(coverage.obligations == 2);
    assert(coverage.complete);
    assert(coverage.missing.len() == 0);
    let first = coverage.first_missing();
    assert(first.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn empty_inventory_is_rejected() {
    let ledger = policy::Ledger::new();
    let coverage = ledger.finish();
    assert(coverage.obligations == 0);
    assert(!coverage.complete);
    assert(coverage.missing.len() == 0);
    let first = coverage.first_missing();
    assert(first.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn every_failed_identity_is_retained_in_order() {
    let mut ledger = policy::Ledger::new();
    ledger.record("first".to_owned(), false);
    ledger.record("accepted".to_owned(), true);
    ledger.record("middle".to_owned(), false);
    ledger.record("last".to_owned(), false);
    let coverage = ledger.finish();
    reveal_with_fuel(policy::rejected_identities, 5);
    assert(coverage.obligations == 4);
    assert(!coverage.complete);
    assert(coverage.missing.len() == 3);
    assert(coverage.missing[0]@ == "first"@);
    assert(coverage.missing[1]@ == "middle"@);
    assert(coverage.missing[2]@ == "last"@);
    let first = coverage.first_missing();
    assert(first.is_some() && first.unwrap()@ == "first"@);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn duplicate_and_empty_identities_are_not_conflated() {
    let mut ledger = policy::Ledger::new();
    ledger.record("same".to_owned(), false);
    ledger.record("".to_owned(), false);
    ledger.record("same".to_owned(), true);
    ledger.record("same".to_owned(), false);
    let coverage = ledger.finish();
    reveal_with_fuel(policy::rejected_identities, 5);
    assert(coverage.obligations == 4);
    assert(!coverage.complete);
    assert(coverage.missing.len() == 3);
    assert(coverage.missing[0]@ == "same"@);
    assert(coverage.missing[1]@ == ""@);
    assert(coverage.missing[2]@ == "same"@);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn appending_records_preserves_prior_decisions() {
    let mut ledger = policy::Ledger::new();
    ledger.record("before".to_owned(), false);
    let ghost before = ledger.view();
    ledger.record("after".to_owned(), true);
    assert(ledger.view().take(1) =~= before);
    let coverage = ledger.finish();
    reveal_with_fuel(policy::rejected_identities, 3);
    assert(coverage.obligations == 2);
    assert(!coverage.complete);
    assert(coverage.missing.len() == 1);
    assert(coverage.missing[0]@ == "before"@);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn proof_identity_requires_equal_bytes_and_lengths() {
    let first = [0u8, 255u8, 7u8];
    let same = [0u8, 255u8, 7u8];
    let changed = [0u8, 255u8, 8u8];
    let shorter = [0u8, 255u8];
    let empty: [u8; 0] = [];
    let a = policy::equal_bytes(&first, &same);
    let b = policy::equal_bytes(&first, &changed);
    let c = policy::equal_bytes(&first, &shorter);
    let d = policy::equal_bytes(&empty, &empty);
    assert(first@ =~= same@);
    assert(first@[2] != changed@[2]);
    assert(first@.len() == 3 && shorter@.len() == 2);
    assert(a);
    assert(!b);
    assert(!c);
    assert(d);
}

fn successful_compiler_result<'a>(version: &'a [u8], commit: &'a [u8])
    -> (result: policy::CompilerResult<'a>)
    ensures
        result.exit_code == Some(0i64),
        result.success == Some(true),
        result.encountered_error == Some(false),
        result.encountered_vir_error == Some(false),
        result.errors == Some(0u64),
        result.verified == Some(1u64),
        result.entire_crate == Some(true),
        result.version == Some(version),
        result.commit == Some(commit),
{
    policy::CompilerResult {
        exit_code: Some(0i64),
        success: Some(true),
        encountered_error: Some(false),
        encountered_vir_error: Some(false),
        errors: Some(0u64),
        verified: Some(1u64),
        entire_crate: Some(true),
        version: Some(version),
        commit: Some(commit),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn whole_crate_proof_with_exact_nonempty_identity_is_accepted() {
    let version = [118u8];
    let commit = [99u8];
    let mut result = successful_compiler_result(&version, &commit);
    let accepted = policy::accept_compiler_result(&result, &version, &commit);
    assert(accepted);
    result.verified = Some(u64::MAX);
    let largest_count = policy::accept_compiler_result(&result, &version, &commit);
    assert(largest_count);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn partial_failed_empty_or_different_proofs_are_rejected() {
    let version = [118u8];
    let commit = [99u8];
    let other = [100u8];
    let empty: [u8; 0] = [];
    let mut result = successful_compiler_result(&version, &commit);
    let stale_version = policy::accept_compiler_result(&result, &other, &commit);
    let stale_commit = policy::accept_compiler_result(&result, &version, &other);
    let no_pin = policy::accept_compiler_result(&result, &empty, &commit);
    assert(!stale_version && !stale_commit && !no_pin);
    result.entire_crate = Some(false);
    let partial = policy::accept_compiler_result(&result, &version, &commit);
    assert(!partial);
    result.entire_crate = Some(true);
    result.verified = Some(0u64);
    let zero = policy::accept_compiler_result(&result, &version, &commit);
    assert(!zero);
    result.verified = Some(1u64);
    result.errors = Some(1u64);
    let error = policy::accept_compiler_result(&result, &version, &commit);
    assert(!error);
    result.errors = Some(0u64);
    result.exit_code = Some(1i64);
    let failed = policy::accept_compiler_result(&result, &version, &commit);
    assert(!failed);
    result.exit_code = Some(0i64);
    result.encountered_error = Some(true);
    let encountered = policy::accept_compiler_result(&result, &version, &commit);
    assert(!encountered);
    result.encountered_error = Some(false);
    result.encountered_vir_error = Some(true);
    let vir = policy::accept_compiler_result(&result, &version, &commit);
    assert(!vir);
    result.encountered_vir_error = Some(false);
    result.success = Some(false);
    let unsuccessful = policy::accept_compiler_result(&result, &version, &commit);
    assert(!unsuccessful);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn absent_or_wrongly_typed_compiler_fields_cannot_become_success() {
    let version = [118u8];
    let commit = [99u8];
    let mut result = successful_compiler_result(&version, &commit);
    result.verified = None;
    let a = policy::accept_compiler_result(&result, &version, &commit);
    assert(!a);
    result.verified = Some(1u64);
    result.version = None;
    let b = policy::accept_compiler_result(&result, &version, &commit);
    assert(!b);
    result.version = Some(&version);
    result.commit = None;
    let c = policy::accept_compiler_result(&result, &version, &commit);
    assert(!c);
    result.commit = Some(&commit);
    result.errors = None;
    let d = policy::accept_compiler_result(&result, &version, &commit);
    assert(!d);
    result.errors = Some(0u64);
    result.exit_code = None;
    let e = policy::accept_compiler_result(&result, &version, &commit);
    assert(!e);
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
