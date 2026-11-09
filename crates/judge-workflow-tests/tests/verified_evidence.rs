//! Independent boundary expectations for workflow evidence admission.
#![allow(unexpected_cfgs)]

#[path = "../../judge-workflow/src/verified_evidence.rs"]
mod policy;

#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn every_required_fact_admits_the_record() {
    let admitted = policy::admit_record(true, true, true, true, true, true, true);
    assert(admitted);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn each_missing_fact_rejects_the_record() {
    let state = policy::admit_record(false, true, true, true, true, true, true);
    let judge = policy::admit_record(true, false, true, true, true, true, true);
    let request = policy::admit_record(true, true, false, true, true, true, true);
    let digest = policy::admit_record(true, true, true, false, true, true, true);
    let exit = policy::admit_record(true, true, true, true, false, true, true);
    let status = policy::admit_record(true, true, true, true, true, false, true);
    let response = policy::admit_record(true, true, true, true, true, true, false);
    assert(!state && !judge && !request && !digest && !exit && !status && !response);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn all_false_and_single_boundary_facts_remain_rejected() {
    let all_false = policy::admit_record(false, false, false, false, false, false, false);
    let first_only = policy::admit_record(true, false, false, false, false, false, false);
    let state_missing = policy::admit_record(false, true, true, true, true, true, true);
    assert(!all_false && !first_only && !state_missing);
}

}
