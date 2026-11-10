//! Independent expectations also checked by the ordinary Rust test executable.
#![allow(unexpected_cfgs)]

#[path = "../../judge-workflow/src/verified_transition.rs"]
mod policy;

use policy::{ConversionFacts, MigrationError, MigrationFacts};
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

fn expect(out: Result<(), MigrationError>, expected: Option<MigrationError>)
    requires match expected {
        None => out.is_ok(),
        Some(error) => out == Err(error),
    },
{
    match (out, expected) {
        (Ok(()), None) => {},
        (Err(actual), Some(expected)) => match (actual, expected) {
            (MigrationError::DeclarationCount, MigrationError::DeclarationCount) => {},
            (MigrationError::UnexpectedConversion, MigrationError::UnexpectedConversion) => {},
            (MigrationError::MissingExecution, MigrationError::MissingExecution) => {},
            (MigrationError::PairMismatch, MigrationError::PairMismatch) => {},
            (MigrationError::ConversionMismatch, MigrationError::ConversionMismatch) => {},
            _ => { panic!(); },
        },
        _ => { panic!(); },
    }
}

fn good_conversion() -> (facts: ConversionFacts)
    ensures policy::conversion_admissible(facts),
{
    ConversionFacts {
        process_succeeded: true,
        declared_input_digest_matches: true,
        actual_input_digest_matches: Some(true),
        output_digest_matches: true,
        decoded_output_matches: Some(true),
        mappings_match: true,
    }
}

fn good_migration() -> (facts: MigrationFacts)
    ensures
        facts.declaration_count == 1,
        facts.has_script,
        facts.conversion_present,
        facts.replacement_test_passed,
        facts.script_matches,
        facts.test_matches,
        match facts.conversion {
            Some(conversion) => policy::conversion_admissible(conversion),
            None => false,
        },
{
    MigrationFacts {
        declaration_count: 1,
        has_script: true,
        conversion_present: true,
        replacement_test_passed: true,
        script_matches: true,
        test_matches: true,
        conversion: Some(good_conversion()),
    }
}

fn expect_evidence_error(conversion: Option<ConversionFacts>)
    requires !(match conversion {
        Some(facts) => policy::conversion_admissible(facts),
        None => false,
    }),
{
    let mut facts = good_migration();
    facts.conversion = conversion;
    expect(policy::admit_migration(facts), Some(MigrationError::ConversionMismatch));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn no_script_requires_no_actual_conversion() {
    let facts = MigrationFacts {
        declaration_count: 1,
        has_script: false,
        conversion_present: false,
        replacement_test_passed: false,
        script_matches: false,
        test_matches: false,
        conversion: None,
    };
    expect(policy::admit_migration(facts), None);
    let mut facts = good_migration();
    facts.has_script = false;
    expect(policy::admit_migration(facts), Some(MigrationError::UnexpectedConversion));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn exactly_one_declaration_is_required() {
    expect(policy::admit_declaration(1), None);
    expect(policy::admit_declaration(0), Some(MigrationError::DeclarationCount));
    expect(policy::admit_declaration(2), Some(MigrationError::DeclarationCount));
    expect(policy::admit_declaration(usize::MAX), Some(MigrationError::DeclarationCount));
    let mut facts = good_migration();
    facts.declaration_count = 0;
    expect(policy::admit_migration(facts), Some(MigrationError::DeclarationCount));
    let mut facts = good_migration();
    facts.declaration_count = 2;
    expect(policy::admit_migration(facts), Some(MigrationError::DeclarationCount));
    let mut facts = good_migration();
    facts.declaration_count = usize::MAX;
    expect(policy::admit_migration(facts), Some(MigrationError::DeclarationCount));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn every_scripted_fact_is_required() {
    expect(policy::admit_migration(good_migration()), None);
    let mut facts = good_migration();
    facts.replacement_test_passed = false;
    expect(policy::admit_migration(facts), Some(MigrationError::MissingExecution));
    let mut facts = good_migration();
    facts.conversion_present = false;
    expect(policy::admit_migration(facts), Some(MigrationError::MissingExecution));
    let mut facts = good_migration();
    facts.script_matches = false;
    expect(policy::admit_migration(facts), Some(MigrationError::PairMismatch));
    let mut facts = good_migration();
    facts.test_matches = false;
    expect(policy::admit_migration(facts), Some(MigrationError::PairMismatch));
    expect_evidence_error(None);
    let mut conversion = good_conversion();
    conversion.process_succeeded = false;
    expect_evidence_error(Some(conversion));
    let mut conversion = good_conversion();
    conversion.declared_input_digest_matches = false;
    expect_evidence_error(Some(conversion));
    let mut conversion = good_conversion();
    conversion.actual_input_digest_matches = Some(false);
    expect_evidence_error(Some(conversion));
    let mut conversion = good_conversion();
    conversion.actual_input_digest_matches = None;
    expect_evidence_error(Some(conversion));
    let mut conversion = good_conversion();
    conversion.output_digest_matches = false;
    expect_evidence_error(Some(conversion));
    let mut conversion = good_conversion();
    conversion.decoded_output_matches = Some(false);
    expect_evidence_error(Some(conversion));
    let mut conversion = good_conversion();
    conversion.decoded_output_matches = None;
    expect_evidence_error(Some(conversion));
    let mut conversion = good_conversion();
    conversion.mappings_match = false;
    expect_evidence_error(Some(conversion));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn all_invalid_facts_are_rejected() {
    let facts = MigrationFacts {
        declaration_count: 0,
        has_script: true,
        conversion_present: false,
        replacement_test_passed: false,
        script_matches: false,
        test_matches: false,
        conversion: None,
    };
    expect(policy::admit_migration(facts), Some(MigrationError::DeclarationCount));
    let mut facts = good_migration();
    facts.replacement_test_passed = false;
    facts.script_matches = false;
    facts.conversion = None;
    expect(policy::admit_migration(facts), Some(MigrationError::MissingExecution));
    let mut facts = good_migration();
    facts.test_matches = false;
    facts.conversion = None;
    expect(policy::admit_migration(facts), Some(MigrationError::PairMismatch));
}

}
