//! Independent success/error boundaries for the shared executable path decision.
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../runner/src/verified_paths.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_paths as policy;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
verus! {

fn expect(path: &[u8], expected: bool)
    requires expected == policy::relative_path(path@),
{
    let actual = policy::valid_relative_path(path);
    assert(actual == expected);
}

fn empty_and_absolute_paths_fail() {
    expect(&[], false);
    expect(&[47u8], false);
    expect(&[47u8, 97], false);
    expect(&[47u8, 47, 97], false);
}

fn whole_parent_components_fail_at_every_position() {
    expect(&[46u8, 46], false);
    expect(&[46u8, 46, 47, 97], false);
    expect(&[97u8, 47, 46, 46], false);
    expect(&[97u8, 47, 46, 46, 47, 98], false);
    expect(&[97u8, 47, 47, 46, 46, 47, 98], false);
    expect(&[46u8, 46, 97], true);
    expect(&[97u8, 47, 46, 46, 46, 47, 98], true);
}

fn current_directory_and_separator_boundaries() {
    expect(&[46u8], false);
    expect(&[46u8, 47, 97], false);
    expect(&[46u8, 47, 47, 97], false);
    expect(&[97u8, 47, 46], false);
    expect(&[97u8, 47, 46, 47, 98], false);
    expect(&[97u8, 47, 47, 98, 47], true);
    expect(&[46u8, 97], true);
}

fn forbidden_bytes_and_permitted_posix_names() {
    expect(&[97u8, 0, 98], false);
    expect(&[97u8, 10, 98], false);
    expect(&[97u8, 13, 98], false);
    expect(&[97u8, 92, 98], false);
    expect(&[97u8, 9, 32, 127], true);
    expect(&[67u8, 58, 97], true);
    expect(&[42u8, 63], true);
}

fn unicode_bytes_preserve_component_boundaries() {
    // U+4E2D, '/', U+6587. The caller's str owns UTF-8 validity.
    expect(&[228u8, 184, 173, 47, 230, 150, 135], true);
    expect(&[228u8, 184, 173, 47, 46, 46], false);
    expect(&[228u8, 184, 173, 46, 46], true);
}

}

#[cfg(not(verus_keep_ghost))]
mod runtime {
    use super::policy;

    fn expect(path: &[u8], expected: bool) {
        assert_eq!(policy::valid_relative_path(path), expected);
    }

    #[test]
    fn empty_and_absolute_paths_fail() {
        expect(&[], false);
        expect(&[47u8], false);
        expect(&[47u8, 97], false);
        expect(&[47u8, 47, 97], false);
    }

    #[test]
    fn whole_parent_components_fail_at_every_position() {
        expect(&[46u8, 46], false);
        expect(&[46u8, 46, 47, 97], false);
        expect(&[97u8, 47, 46, 46], false);
        expect(&[97u8, 47, 46, 46, 47, 98], false);
        expect(&[97u8, 47, 47, 46, 46, 47, 98], false);
        expect(&[46u8, 46, 97], true);
        expect(&[97u8, 47, 46, 46, 46, 47, 98], true);
    }

    #[test]
    fn current_directory_and_separator_boundaries() {
        expect(&[46u8], false);
        expect(&[46u8, 47, 97], false);
        expect(&[46u8, 47, 47, 97], false);
        expect(&[97u8, 47, 46], false);
        expect(&[97u8, 47, 46, 47, 98], false);
        expect(&[97u8, 47, 47, 98, 47], true);
        expect(&[46u8, 97], true);
    }

    #[test]
    fn forbidden_bytes_and_permitted_posix_names() {
        expect(&[97u8, 0, 98], false);
        expect(&[97u8, 10, 98], false);
        expect(&[97u8, 13, 98], false);
        expect(&[97u8, 92, 98], false);
        expect(&[97u8, 9, 32, 127], true);
        expect(&[67u8, 58, 97], true);
        expect(&[42u8, 63], true);
    }

    #[test]
    fn unicode_bytes_preserve_component_boundaries() {
        expect(&[228u8, 184, 173, 47, 230, 150, 135], true);
        expect(&[228u8, 184, 173, 47, 46, 46], false);
        expect(&[228u8, 184, 173, 46, 46], true);
    }
}
