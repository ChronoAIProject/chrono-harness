//! Independent boundary expectations for workflow execution admission.
#![allow(unexpected_cfgs)]

#[path = "../../judge-workflow/src/verified_execution.rs"]
mod policy;

#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn collected_execution_requires_every_fact() {
    let admitted = policy::admit_collected(true, true, true, true, true, true, true, true, true);
    assert(admitted);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn each_collected_execution_fact_is_required() {
    let schema = policy::admit_collected(false, true, true, true, true, true, true, true, true);
    let status = policy::admit_collected(true, false, true, true, true, true, true, true, true);
    let scope = policy::admit_collected(true, true, false, true, true, true, true, true, true);
    let selected = policy::admit_collected(true, true, true, false, true, true, true, true, true);
    let executed = policy::admit_collected(true, true, true, true, false, true, true, true, true);
    let blocked = policy::admit_collected(true, true, true, true, true, false, true, true, true);
    let plan = policy::admit_collected(true, true, true, true, true, true, false, true, true);
    let result_selected = policy::admit_collected(true, true, true, true, true, true, true, false, true);
    let tests = policy::admit_collected(true, true, true, true, true, true, true, true, false);
    assert(!schema && !status && !scope && !selected && !executed);
    assert(!blocked && !plan && !result_selected && !tests);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn ordinary_execution_requires_every_fact() {
    let admitted = policy::admit_execution(true, true, true, true, true, true);
    let plan = policy::admit_execution(false, true, true, true, true, true);
    let scope = policy::admit_execution(true, false, true, true, true, true);
    let selected = policy::admit_execution(true, true, false, true, true, true);
    let tests = policy::admit_execution(true, true, true, false, true, true);
    let passed = policy::admit_execution(true, true, true, true, false, true);
    let count = policy::admit_execution(true, true, true, true, true, false);
    assert(admitted && !plan && !scope && !selected && !tests && !passed && !count);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn all_false_execution_facts_are_rejected() {
    let collected = policy::admit_collected(false, false, false, false, false, false, false, false, false);
    let ordinary = policy::admit_execution(false, false, false, false, false, false);
    assert(!collected && !ordinary);
}

}
