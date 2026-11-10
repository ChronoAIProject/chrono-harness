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
use vstd::string::StringSliceAdditionalSpecFns;
#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

fn expect_path_true(actual: bool)
    requires actual,
{
    if !actual { panic!(); }
}

fn expect_path_text(actual: &str, expected: &str)
    requires actual@ == expected@,
{
    let matched = identifiers::equal_text(actual, expected);
    expect_path_true(matched);
}

use policy::ArtifactDeclarationError;

fn declared_paths<'a>(rows: Option<&'a [Option<&'a str>]>)
    -> (out: Result<Vec<&'a str>, ArtifactDeclarationError>)
    ensures match (rows, out) {
        (None, Err(ArtifactDeclarationError::Missing)) => true,
        (Some(rows), Ok(paths)) => paths.len() == rows.len()
            && forall|i: int| 0 <= i < rows.len() ==> #[trigger] rows@[i] == Some(paths@[i]),
        (Some(rows), Err(ArtifactDeclarationError::Invalid { row })) => row < rows.len()
            && rows@[row as int].is_none()
            && forall|j: int| 0 <= j < row ==> #[trigger] rows@[j].is_some(),
        _ => false,
    },
{
    let observe = |row: &'a Option<&'a str>| -> (path: Option<&'a str>)
        ensures path == *row,
    { *row };
    let result = policy::collect_artifact_declarations(rows, observe);
    proof {
        match (rows, &result) {
            (Some(rows), Ok(paths)) => {
                assert forall|i: int| 0 <= i < rows.len()
                    implies #[trigger] rows@[i] == Some(paths@[i]) by {
                    assert(observe.ensures((&rows@[i],), Some(paths@[i])));
                }
            },
            (Some(rows), Err(ArtifactDeclarationError::Invalid { row })) => {
                assert forall|j: int| 0 <= j < *row implies #[trigger] rows@[j].is_some() by {
                    assert(policy::artifact_path_observed(observe, &rows@[j]));
                    let path = choose|path: &'a str| #[trigger] observe.ensures((&rows@[j],), Some(path));
                    assert(rows@[j] == Some(path));
                }
            },
            _ => {},
        }
    }
    result
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn artifact_declaration_read_preserves_all_strings_in_original_order() {
    let rows = [Some(""), Some("目录/"), Some("../a\0"), Some("目录/")];
    match declared_paths(Some(&rows)) {
        Ok(paths) => {
            expect_path_true(paths.len() == 4);
            expect_path_text(paths[0], "");
            expect_path_text(paths[1], "目录/");
            expect_path_text(paths[2], "../a\0");
            expect_path_text(paths[3], "目录/");
        },
        Err(_) => { proof { assert(false); } panic!(); },
    }
    match declared_paths(Some(&[])) {
        Ok(paths) => expect_path_true(paths.len() == 0),
        Err(_) => { proof { assert(false); } panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn artifact_declaration_read_distinguishes_missing_list_and_missing_path() {
    match declared_paths(None) {
        Err(error) => expect_path_text(error.message(), "missing artifact declarations"),
        Ok(_) => { proof { assert(false); } panic!(); },
    }
    let rows = [Some("a/"), None, Some("b/"), None];
    match declared_paths(Some(&rows)) {
        Err(error @ ArtifactDeclarationError::Invalid { row }) => {
            proof {
                assert(rows@[1].is_none());
                if row == 0 { assert(rows@[0].is_some()); }
                if row > 1 { assert(rows@[1].is_some()); }
            }
            expect_path_true(row == 1);
            expect_path_text(error.message(), "invalid artifact path");
        },
        Err(ArtifactDeclarationError::Missing) => { proof { assert(false); } panic!(); },
        Ok(paths) => {
            proof { assert(rows@[1] == Some(paths@[1])); assert(false); }
            panic!();
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn literal_exclusions_filter_components_and_keep_order_and_duplicates() {
    proof {
        reveal_strlit(""); reveal_strlit("/"); reveal_strlit("file");
        reveal_strlit("./"); reveal_strlit("../"); reveal_strlit("/absolute/");
        reveal_strlit("a//b/"); reveal_strlit("a/./b/"); reveal_strlit("a/../b/");
        reveal_strlit("out/"); reveal_strlit("a..b/");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("") && vstd::string::is_ascii("/")
            && vstd::string::is_ascii("file") && vstd::string::is_ascii("./")
            && vstd::string::is_ascii("../") && vstd::string::is_ascii("/absolute/")
            && vstd::string::is_ascii("a//b/") && vstd::string::is_ascii("a/./b/")
            && vstd::string::is_ascii("a/../b/") && vstd::string::is_ascii("out/")
            && vstd::string::is_ascii("a..b/"));
        assert(!policy::artifact_directory("".spec_bytes()));
        assert(!policy::artifact_component_ok("/".spec_bytes(), 0));
        assert(!policy::artifact_directory("/".spec_bytes()));
        assert(!policy::artifact_directory("file".spec_bytes()));
        assert(!policy::artifact_component_ok("./".spec_bytes(), 0));
        assert(!policy::artifact_directory("./".spec_bytes()));
        assert(!policy::artifact_component_ok("../".spec_bytes(), 0));
        assert(!policy::artifact_directory("../".spec_bytes()));
        assert(!policy::artifact_component_ok("/absolute/".spec_bytes(), 0));
        assert(!policy::artifact_directory("/absolute/".spec_bytes()));
        assert(!policy::artifact_component_ok("a//b/".spec_bytes(), 2));
        assert(!policy::artifact_directory("a//b/".spec_bytes()));
        assert(!policy::artifact_component_ok("a/./b/".spec_bytes(), 2));
        assert(!policy::artifact_directory("a/./b/".spec_bytes()));
        assert(!policy::artifact_component_ok("a/../b/".spec_bytes(), 2));
        assert(!policy::artifact_directory("a/../b/".spec_bytes()));
        assert(policy::artifact_directory("out/".spec_bytes()));
        assert(policy::artifact_directory("a..b/".spec_bytes()));
        reveal_with_fuel(Seq::<_>::filter_map, 14);
    }
    let selected = policy::literal_artifact_directories(&[
        "", "/", "file", "./", "../", "/absolute/", "a//b/", "a/./b/", "a/../b/",
        "out/", "a..b/", "out/",
    ]);
    expect_path_true(selected.len() == 3);
    expect_path_text(selected[0], "out/");
    expect_path_text(selected[1], "a..b/");
    expect_path_text(selected[2], "out/");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn literal_exclusions_preserve_globs_unicode_nul_and_control_text() {
    proof {
        reveal_strlit("目录/é/"); reveal_strlit("a*b?[x]/"); reveal_strlit("a\0b/");
        reveal_strlit("a\nb/"); reveal_strlit("a\\b/"); reveal_strlit(".hidden/");
        reveal_strlit(".../");
        unicode_literal_directory_bytes();
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("a*b?[x]/") && vstd::string::is_ascii("a\0b/")
            && vstd::string::is_ascii("a\nb/") && vstd::string::is_ascii("a\\b/")
            && vstd::string::is_ascii(".hidden/") && vstd::string::is_ascii(".../"));
        assert(policy::artifact_directory("目录/é/".spec_bytes()));
        assert(policy::artifact_directory("a*b?[x]/".spec_bytes()));
        assert(policy::artifact_directory("a\0b/".spec_bytes()));
        assert(policy::artifact_directory("a\nb/".spec_bytes()));
        assert(policy::artifact_directory("a\\b/".spec_bytes()));
        assert(policy::artifact_directory(".hidden/".spec_bytes()));
        assert(policy::artifact_directory(".../".spec_bytes()));
        reveal_with_fuel(Seq::<_>::filter_map, 10);
    }
    let selected = policy::literal_artifact_directories(&[
        "目录/é/", "a*b?[x]/", "a\0b/", "a\nb/", "a\\b/", ".hidden/", ".../",
    ]);
    expect_path_true(selected.len() == 7);
    expect_path_text(selected[0], "目录/é/");
    expect_path_text(selected[1], "a*b?[x]/");
    expect_path_text(selected[2], "a\0b/");
    expect_path_text(selected[3], "a\nb/");
    expect_path_text(selected[4], "a\\b/");
    expect_path_text(selected[5], ".hidden/");
    expect_path_text(selected[6], ".../");
    let empty = policy::literal_artifact_directories(&[]);
    expect_path_true(empty.len() == 0);
}

proof fn unicode_literal_directory_bytes()
    ensures "目录/é/".spec_bytes() == seq![231u8, 155, 174, 229, 189, 149, 47, 195, 169, 47],
{
    reveal_strlit("目录/é/");
    assert("目录/é/"@ =~= seq!['目', '录', '/', 'é', '/']);
    assert('目' as u32 == 30446 && '录' as u32 == 24405
        && 'é' as u32 == 233 && '/' as u32 == 47);
    assert(vstd::utf8::encode_scalar(30446) == seq![231u8, 155, 174]) by (compute);
    assert(vstd::utf8::encode_scalar(24405) == seq![229u8, 189, 149]) by (compute);
    assert(vstd::utf8::encode_scalar(233) == seq![195u8, 169]) by (compute);
    assert(vstd::utf8::encode_scalar(47) == seq![47u8]) by (compute);
    reveal_with_fuel(vstd::utf8::encode_utf8, 6);
    assert("目录/é/".spec_bytes() =~= seq![231u8, 155, 174, 229, 189, 149, 47, 195, 169, 47]);
}

fn expect(path: &[u8], expected: bool)
    requires expected == policy::relative_path(path@),
{
    let actual = policy::valid_relative_path(path);
    expect_path_true(actual == expected);
}

fn expect_state(path: &[u8], expected: bool)
    requires expected == policy::host_state_path(path@),
{
    let actual = policy::valid_host_state_path(path);
    expect_path_true(actual == expected);
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
