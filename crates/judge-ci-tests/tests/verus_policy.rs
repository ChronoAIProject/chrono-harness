//! Verified success, rejection and boundary expectations for the actual predicate.
#![allow(unexpected_cfgs)]
#[path = "../../judge-ci/src/verus_policy.rs"]
mod policy;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;

verus! {

broadcast use vstd::string::group_string_axioms;

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
fn registered_text_requires_exact_identity() {
    proof {
        reveal_strlit("verus");
        reveal_strlit("git");
        reveal_strlit("Verus");
        reveal_strlit("ver");
        reveal_strlit("");
        reveal_strlit("接口");
    }
    let registered = vec!["verus", "git", "verus"];
    let present = policy::contains_text(&registered, "verus");
    let absent = policy::contains_text(&registered, "Verus");
    let prefix = policy::contains_text(&registered, "ver");
    let empty: Vec<&str> = Vec::new();
    let no_entries = policy::contains_text(&empty, "");
    assert(registered[0]@ == "verus"@);
    assert("verus"@[0] != "Verus"@[0]);
    assert("git"@[0] != "Verus"@[0]);
    assert("verus"@.len() != "ver"@.len());
    assert("git"@[0] != "ver"@[0]);
    assert(present);
    assert(!absent);
    assert(!prefix);
    assert(!no_entries);
    let unicode = vec!["接口", ""];
    let exact_unicode = policy::contains_text(&unicode, "接口");
    let exact_empty = policy::contains_text(&unicode, "");
    assert(exact_unicode && exact_empty);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn requirements_need_selected_tool_and_explicit_nonempty_interfaces() {
    proof {
        reveal_strlit("verus");
        reveal_strlit("unregistered");
        reveal_strlit("chrono-verus/v1");
        reveal_strlit("chrono-verus/v2");
        reveal_strlit("pinned");
        reveal_strlit("identity");
        reveal_strlit("os");
        reveal_strlit("library");
        reveal_strlit("registered bytes");
        reveal_strlit("pinned contract");
        reveal_strlit("");
    }
    let tools = vec!["verus"];
    let mut fields = policy::RequirementFields {
        schema: "chrono-verus/v1",
        tool: "verus",
        version: "pinned",
        commit: "identity",
        interfaces: vec![("os", "registered bytes"), ("library", "pinned contract")],
    };
    let accepted = policy::valid_requirement_fields(&fields, &tools);
    assert(accepted);
    fields.tool = "unregistered";
    let wrong_tool = policy::valid_requirement_fields(&fields, &tools);
    assert("unregistered"@.len() != "verus"@.len());
    assert(!wrong_tool);
    fields.tool = "verus";
    fields.schema = "chrono-verus/v2";
    let wrong_schema = policy::valid_requirement_fields(&fields, &tools);
    assert("chrono-verus/v1"@[14] != "chrono-verus/v2"@[14]);
    assert(!wrong_schema);
    fields.schema = "chrono-verus/v1";
    fields.version = "";
    let no_version = policy::valid_requirement_fields(&fields, &tools);
    assert(!no_version);
    fields.version = "pinned";
    fields.commit = "";
    let no_commit = policy::valid_requirement_fields(&fields, &tools);
    assert(!no_commit);
    fields.commit = "identity";
    fields.interfaces[1] = ("library", "");
    let later_empty_assumption = policy::valid_requirement_fields(&fields, &tools);
    assert(fields.interfaces[1].1@.len() == 0);
    assert(!later_empty_assumption);
    fields.interfaces[1] = ("", "pinned contract");
    let later_empty_name = policy::valid_requirement_fields(&fields, &tools);
    assert(fields.interfaces[1].0@.len() == 0);
    assert(!later_empty_name);
    fields.interfaces = Vec::new();
    let no_interfaces = policy::valid_requirement_fields(&fields, &tools);
    assert(!no_interfaces);
}

fn canonical_arguments() -> (args: Vec<String>)
    ensures policy::verifier_arguments(args@),
{
    vec![
        "--crate-type=lib".to_owned(), "--no-cheating".to_owned(),
        "--no-external-by-default".to_owned(), "--num-threads".to_owned(),
        "2".to_owned(), "--output-json".to_owned(),
        "--compile".to_owned(), "--emit=link,dep-info".to_owned(), "root.rs".to_owned(),
        "-o".to_owned(), "proof.rlib".to_owned(),
    ]
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn verifier_arguments_cannot_disable_checks_or_select_partial_functions() {
    proof {
        reveal_strlit("--no-cheating");
        reveal_strlit("--no-cheating=false");
        reveal_strlit("--no-external-by-default");
        reveal_strlit("--external-by-default");
        reveal_strlit("--verify-function");
        reveal_strlit("-o");
    }
    let mut args = canonical_arguments();
    let accepted = policy::canonical_verifier_arguments(&args);
    assert(accepted);
    args[1] = "--no-cheating=false".to_owned();
    let cheating = policy::canonical_verifier_arguments(&args);
    assert("--no-cheating=false"@.len() != "--no-cheating"@.len());
    assert(!cheating);
    args[1] = "--no-cheating".to_owned();
    args[2] = "--external-by-default".to_owned();
    let external = policy::canonical_verifier_arguments(&args);
    assert("--external-by-default"@[2] != "--no-external-by-default"@[2]);
    assert(!external);
    args[2] = "--no-external-by-default".to_owned();
    args[9] = "--verify-function".to_owned();
    let partial = policy::canonical_verifier_arguments(&args);
    assert("--verify-function"@.len() != "-o"@.len());
    assert(!partial);
    let mut extended = canonical_arguments();
    extended.push("--verify-function=one".to_owned());
    let extra = policy::canonical_verifier_arguments(&extended);
    assert(!extra);
    let empty: Vec<String> = Vec::new();
    let absent = policy::canonical_verifier_arguments(&empty);
    assert(!absent);
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
