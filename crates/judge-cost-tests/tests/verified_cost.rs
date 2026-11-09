//! Independent boundary expectations for the declared-cost Verus policy.
#![allow(unexpected_cfgs)]
#[path = "../../judge-cost/src/verified_cost.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn no_missing_coordinate_is_known() {
    let known = policy::has_unknown_coordinate(false, false, false, false);
    assert(!known);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn each_coordinate_independently_makes_the_declaration_unknown() {
    let cpu = policy::has_unknown_coordinate(true, false, false, false);
    let wall = policy::has_unknown_coordinate(false, true, false, false);
    let peak_rss = policy::has_unknown_coordinate(false, false, true, false);
    let io = policy::has_unknown_coordinate(false, false, false, true);
    assert(cpu && wall && peak_rss && io);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn combinations_are_monotone_in_missing_coordinates() {
    let none = policy::has_unknown_coordinate(false, false, false, false);
    let cpu_wall = policy::has_unknown_coordinate(true, true, false, false);
    let peak_io = policy::has_unknown_coordinate(false, false, true, true);
    let all = policy::has_unknown_coordinate(true, true, true, true);
    assert(!none && cpu_wall && peak_io && all);
}

}
