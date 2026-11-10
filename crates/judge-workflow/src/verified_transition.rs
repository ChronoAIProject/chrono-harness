//! Schema migration admission owned by a proved predicate.
#![allow(unexpected_cfgs)]

#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

/// A schema transition is admissible only when exactly one declaration exists.
/// A declaration without a script forbids an observed conversion.  A scripted
/// declaration requires every fact collected by the external adapters.
pub open spec fn migration_admissible(
    exactly_one_declaration: bool,
    has_script: bool,
    no_conversion_without_script: bool,
    replacement_test_passed: bool,
    conversion_present: bool,
    script_test_matches: bool,
    process_succeeded: bool,
    input_digest_matches: bool,
    output_digest_matches: bool,
    decoded_output_matches: bool,
    mappings_match: bool,
) -> bool {
    exactly_one_declaration
        && if has_script {
            replacement_test_passed
                && conversion_present
                && script_test_matches
                && process_succeeded
                && input_digest_matches
                && output_digest_matches
                && decoded_output_matches
                && mappings_match
        } else {
            no_conversion_without_script
        }
}

/// Consume adapted migration facts through the proved schema-transition rule.
pub fn admit_migration(
    exactly_one_declaration: bool,
    has_script: bool,
    no_conversion_without_script: bool,
    replacement_test_passed: bool,
    conversion_present: bool,
    script_test_matches: bool,
    process_succeeded: bool,
    input_digest_matches: bool,
    output_digest_matches: bool,
    decoded_output_matches: bool,
    mappings_match: bool,
) -> (admitted: bool)
    ensures admitted == migration_admissible(
        exactly_one_declaration,
        has_script,
        no_conversion_without_script,
        replacement_test_passed,
        conversion_present,
        script_test_matches,
        process_succeeded,
        input_digest_matches,
        output_digest_matches,
        decoded_output_matches,
        mappings_match,
    ),
{
    exactly_one_declaration
        && if has_script {
            replacement_test_passed
                && conversion_present
                && script_test_matches
                && process_succeeded
                && input_digest_matches
                && output_digest_matches
                && decoded_output_matches
                && mappings_match
        } else {
            no_conversion_without_script
        }
}

}
