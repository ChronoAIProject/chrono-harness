//! Independent boundary expectations for the declared-cost Verus policy.
#![allow(unexpected_cfgs)]
#[path = "../../judge-cost/src/verified_cost.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

fn expect(actual: bool)
    requires actual,
{
    if !actual { panic!(); }
}

fn expect_name(actual: &str, expected: &str)
    requires actual@ == expected@,
{
    let equal = policy::identifiers::equal_text(actual, expected);
    expect(equal);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn no_missing_coordinate_is_known() {
    let known = policy::has_unknown_coordinate(false, false, false, false);
    expect(!known);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn each_coordinate_independently_makes_the_declaration_unknown() {
    let cpu = policy::has_unknown_coordinate(true, false, false, false);
    let wall = policy::has_unknown_coordinate(false, true, false, false);
    let peak_rss = policy::has_unknown_coordinate(false, false, true, false);
    let io = policy::has_unknown_coordinate(false, false, false, true);
    expect(cpu && wall && peak_rss && io);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn combinations_are_monotone_in_missing_coordinates() {
    let none = policy::has_unknown_coordinate(false, false, false, false);
    let cpu_wall = policy::has_unknown_coordinate(true, true, false, false);
    let peak_io = policy::has_unknown_coordinate(false, false, true, true);
    let all = policy::has_unknown_coordinate(true, true, true, true);
    expect(!none && cpu_wall && peak_io && all);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn missing_names_preserve_each_coordinate_and_fixed_order() {
    let known = policy::missing_coordinate_names(false, false, false, false);
    expect(known.len() == 0);
    let cpu = policy::missing_coordinate_names(true, false, false, false);
    expect(cpu.len() == 1);
    expect_name(cpu[0], "cpu_ms");
    let wall = policy::missing_coordinate_names(false, true, false, false);
    expect(wall.len() == 1);
    expect_name(wall[0], "wall_ms");
    let peak = policy::missing_coordinate_names(false, false, true, false);
    expect(peak.len() == 1);
    expect_name(peak[0], "peak_rss_bytes");
    let io = policy::missing_coordinate_names(false, false, false, true);
    expect(io.len() == 1);
    expect_name(io[0], "io_bytes");
    let mixed = policy::missing_coordinate_names(true, false, true, true);
    expect(mixed.len() == 3);
    expect_name(mixed[0], "cpu_ms");
    expect_name(mixed[1], "peak_rss_bytes");
    expect_name(mixed[2], "io_bytes");
    let all = policy::missing_coordinate_names(true, true, true, true);
    expect(all.len() == 4);
    expect_name(all[0], "cpu_ms");
    expect_name(all[1], "wall_ms");
    expect_name(all[2], "peak_rss_bytes");
    expect_name(all[3], "io_bytes");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn absent_endpoints_preserve_subject_and_existing_removals() {
    let mut removed = vec!["previous".to_string()];
    let result = policy::record_subject_presence(false, false, "成本\0尾".to_string(), &mut removed);
    expect(result.is_err());
    let error = result.unwrap_err();
    expect_name(error.message, "E_COST_REFERENCE: no declaration at either endpoint for ");
    expect_name(error.subject.as_str(), "成本\0尾");
    expect(removed.len() == 1);
    expect_name(removed[0].as_str(), "previous");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn only_removed_subjects_append_without_losing_existing_entries() {
    let mut removed = vec!["previous".to_string()];
    let added = policy::record_subject_presence(false, true, "new".to_string(), &mut removed);
    expect(added.is_ok());
    expect(removed.len() == 1);
    let retained = policy::record_subject_presence(true, true, "same".to_string(), &mut removed);
    expect(retained.is_ok());
    expect(removed.len() == 1);
    let deleted = policy::record_subject_presence(true, false, "成本\0尾".to_string(), &mut removed);
    expect(deleted.is_ok());
    expect(removed.len() == 2);
    expect_name(removed[0].as_str(), "previous");
    expect_name(removed[1].as_str(), "成本\0尾");
}

}
