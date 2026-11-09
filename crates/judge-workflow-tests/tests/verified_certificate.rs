//! Independent boundary expectations for certificate binding admission.
#![allow(unexpected_cfgs)]

#[path = "../../judge-workflow/src/verified_certificate.rs"]
mod policy;

#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn complete_binding_is_admitted() {
    let admitted = policy::admit_binding(true, true);
    assert(admitted);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn missing_or_duplicate_binding_fields_are_rejected() {
    let missing = policy::admit_binding(false, true);
    let duplicate = policy::admit_binding(true, false);
    assert(!missing && !duplicate);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn all_invalid_binding_facts_are_rejected() {
    let admitted = policy::admit_binding(false, false);
    assert(!admitted);
}

}
