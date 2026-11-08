//! 实际路径判断的独立成功／失败边界。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../runner/src/verified_paths.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_paths as policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

fn expect(path: &[u8], expected: bool)
    requires expected == policy::relative_path(path@),
{
    let actual = policy::valid_relative_path(path);
    if actual != expected {
        panic!();
    }
}

fn expect_state(path: &[u8], expected: bool)
    requires expected == policy::host_state_path(path@),
{
    let actual = policy::valid_host_state_path(path);
    if actual != expected {
        panic!();
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn state_prefix_requires_an_exact_directory_and_nonempty_suffix() {
    expect_state(&[], false);
    expect_state(&[46u8, 99, 104, 114, 111, 110, 111, 45, 104, 97, 114, 110, 101, 115, 115, 47, 115, 116, 97, 116, 101], false);
    expect_state(&[46u8, 99, 104, 114, 111, 110, 111, 45, 104, 97, 114, 110, 101, 115, 115, 47, 115, 116, 97, 116, 101, 47], false);
    expect_state(&[46u8, 99, 104, 114, 111, 110, 111, 45, 104, 97, 114, 110, 101, 115, 115, 47, 115, 116, 97, 116, 101, 115, 47, 97], false);
    expect_state(&[46u8, 99, 104, 114, 111, 110, 111, 45, 104, 97, 114, 110, 101, 115, 115, 47, 115, 116, 97, 116, 101, 47, 97], true);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn state_suffix_keeps_relative_path_boundaries() {
    expect_state(&[46u8, 99, 104, 114, 111, 110, 111, 45, 104, 97, 114, 110, 101, 115, 115, 47, 115, 116, 97, 116, 101, 47, 228, 184, 173], true);
    expect_state(&[46u8, 99, 104, 114, 111, 110, 111, 45, 104, 97, 114, 110, 101, 115, 115, 47, 115, 116, 97, 116, 101, 47, 97, 47, 46, 47, 98], true);
    expect_state(&[46u8, 99, 104, 114, 111, 110, 111, 45, 104, 97, 114, 110, 101, 115, 115, 47, 115, 116, 97, 116, 101, 47, 47, 97, 47], true);
    expect_state(&[46u8, 99, 104, 114, 111, 110, 111, 45, 104, 97, 114, 110, 101, 115, 115, 47, 115, 116, 97, 116, 101, 47, 46, 46, 47, 97], false);
    expect_state(&[46u8, 99, 104, 114, 111, 110, 111, 45, 104, 97, 114, 110, 101, 115, 115, 47, 115, 116, 97, 116, 101, 47, 97, 0], false);
    expect_state(&[46u8, 99, 104, 114, 111, 110, 111, 45, 104, 97, 114, 110, 101, 115, 115, 47, 115, 116, 97, 116, 101, 47, 97, 10], false);
    expect_state(&[46u8, 99, 104, 114, 111, 110, 111, 45, 104, 97, 114, 110, 101, 115, 115, 47, 115, 116, 97, 116, 101, 47, 97, 13], false);
    expect_state(&[46u8, 99, 104, 114, 111, 110, 111, 45, 104, 97, 114, 110, 101, 115, 115, 47, 115, 116, 97, 116, 101, 47, 97, 92], false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn empty_and_absolute_paths_fail() {
    expect(&[], false);
    expect(&[47u8], false);
    expect(&[47u8, 97], false);
    expect(&[47u8, 47, 97], false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn whole_parent_components_fail_at_every_position() {
    expect(&[46u8, 46], false);
    expect(&[46u8, 46, 47, 97], false);
    expect(&[97u8, 47, 46, 46], false);
    expect(&[97u8, 47, 46, 46, 47, 98], false);
    expect(&[97u8, 47, 47, 46, 46, 47, 98], false);
    expect(&[46u8, 46, 97], true);
    expect(&[97u8, 47, 46, 46, 46, 47, 98], true);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn current_directory_and_separator_boundaries() {
    expect(&[46u8], false);
    expect(&[46u8, 47, 97], false);
    expect(&[46u8, 47, 47, 97], false);
    expect(&[97u8, 47, 46], true);
    expect(&[97u8, 47, 46, 47, 98], true);
    expect(&[97u8, 47, 47, 98, 47], true);
    expect(&[46u8, 97], true);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn forbidden_bytes_and_permitted_posix_names() {
    expect(&[97u8, 0, 98], false);
    expect(&[97u8, 10, 98], false);
    expect(&[97u8, 13, 98], false);
    expect(&[97u8, 92, 98], false);
    expect(&[97u8, 9, 32, 127], true);
    expect(&[67u8, 58, 97], true);
    expect(&[42u8, 63], true);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn unicode_bytes_preserve_component_boundaries() {
    // U+4E2D、'/'、U+6587。调用方的 str 保证 UTF-8 有效性。
    expect(&[228u8, 184, 173, 47, 230, 150, 135], true);
    expect(&[228u8, 184, 173, 47, 46, 46], false);
    expect(&[228u8, 184, 173, 46, 46], true);
}

}
