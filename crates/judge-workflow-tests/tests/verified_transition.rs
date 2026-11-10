//! Independent boundary expectations for schema migration admission.
#![allow(unexpected_cfgs)]

#[path = "../../judge-workflow/src/verified_transition.rs"]
mod policy;

#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn no_script_requires_no_actual_conversion() {
    let admitted = policy::admit_migration(
        true, false, true, false, false, false, false, false, false, false, false,
    );
    let conversion = policy::admit_migration(
        true, false, false, false, true, false, false, false, false, false, false,
    );
    assert(admitted && !conversion);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn exactly_one_declaration_is_required() {
    let none = policy::admit_migration(
        false, false, true, false, false, false, false, false, false, false, false,
    );
    let many = policy::admit_migration(
        false, true, true, true, true, true, true, true, true, true, true,
    );
    assert(!none && !many);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn every_scripted_fact_is_required() {
    let admitted = policy::admit_migration(
        true, true, true, true, true, true, true, true, true, true, true,
    );
    let replacement = policy::admit_migration(
        true, true, true, false, true, true, true, true, true, true, true,
    );
    let conversion = policy::admit_migration(
        true, true, true, true, false, true, true, true, true, true, true,
    );
    let pair = policy::admit_migration(
        true, true, true, true, true, false, true, true, true, true, true,
    );
    let process = policy::admit_migration(
        true, true, true, true, true, true, false, true, true, true, true,
    );
    let input = policy::admit_migration(
        true, true, true, true, true, true, true, false, true, true, true,
    );
    let output = policy::admit_migration(
        true, true, true, true, true, true, true, true, false, true, true,
    );
    let decoded = policy::admit_migration(
        true, true, true, true, true, true, true, true, true, false, true,
    );
    let mappings = policy::admit_migration(
        true, true, true, true, true, true, true, true, true, true, false,
    );
    assert(admitted);
    assert(!replacement && !conversion && !pair && !process);
    assert(!input && !output && !decoded && !mappings);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn all_invalid_facts_are_rejected() {
    let admitted = policy::admit_migration(
        false, false, false, false, false, false, false, false, false, false, false,
    );
    assert(!admitted);
}

}
