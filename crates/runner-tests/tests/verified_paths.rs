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

fn parent_fixture_next(source: u8) -> (out: (u8, Option<u8>))
    ensures if source == 0 { out == (0u8, None::<u8>) }
        else { out == ((source - 1) as u8, Some(source)) },
{
    if source == 0 { (0, None) } else { (source - 1, Some(source)) }
}

fn parent_fixture_append(mut path: Vec<u8>, component: u8) -> (out: Vec<u8>)
    ensures out@ == path@ + seq![component],
{ path.push(component); path }

spec fn parent_fixture_kind(mode: u8, length: int) -> u8 {
    if mode == 0 { 0 }
    else if mode == 1 { if length == 4 { 2 } else { 0 } }
    else if mode == 2 { 1 }
    else if mode == 3 { if length == 4 { 0 } else { 1 } }
    else if mode == 4 { 3 }
    else if mode == 5 { if length == 4 { 2 } else { 3 } }
    else { 2 }
}

spec fn parent_fixture_observation(mode: u8, path: Seq<u8>, observed: Result<bool, String>) -> bool {
    match parent_fixture_kind(mode, path.len()) {
        0 => observed == Ok::<bool, String>(false),
        1 => observed == Ok::<bool, String>(true),
        2 => match observed { Err(error) => error@ == "missing: é\0end"@, _ => false },
        _ => match observed { Err(error) => error@ == "io: denied é\nend"@, _ => false },
    }
}

fn parent_fixture_metadata(mode: u8, path: Vec<u8>) -> (out: (Vec<u8>, Result<bool, String>))
    ensures out.0 == path, parent_fixture_observation(mode, path@, out.1),
{
    let kind = if mode == 0 { 0u8 }
        else if mode == 1 { if path.len() == 4 { 2 } else { 0 } }
        else if mode == 2 { 1 }
        else if mode == 3 { if path.len() == 4 { 0 } else { 1 } }
        else if mode == 4 { 3 }
        else if mode == 5 { if path.len() == 4 { 2 } else { 3 } }
        else { 2 };
    let observed = match kind {
        0 => Ok(false), 1 => Ok(true),
        2 => Err("missing: é\0end".to_owned()),
        _ => Err("io: denied é\nend".to_owned()),
    };
    (path, observed)
}

fn parent_fixture_missing(error: &String) -> (out: bool)
    ensures out == (error@ == "missing: é\0end"@),
{ identifiers::equal_text(error.as_str(), "missing: é\0end") }

spec fn parent_fixture_path(count: u8, consumed: int) -> Seq<u8> {
    seq![77u8, 0, 255] + if consumed == 0 { seq![] }
    else if consumed == 1 { seq![count] }
    else if consumed == 2 { seq![count, (count - 1) as u8] }
    else { seq![count, (count - 1) as u8, (count - 2) as u8] }
}

// 预期直接列出七种外部观察和 0–3 个组件的结果，不调用生产规格作判定。
spec fn parent_fixture_expected(count: u8, mode: u8,
    out: Result<Vec<u8>, policy::ParentPathFailure<Vec<u8>, String>>) -> bool
{
    if count == 0 || mode == 0 || mode == 1 || mode == 6
        || (count == 1 && (mode == 3 || mode == 5)) {
        match out { Ok(path) => path@ == parent_fixture_path(count, count as int), _ => false }
    } else if mode == 2 || mode == 3 {
        match out {
            Err(policy::ParentPathFailure::Symlink(path)) => path@ == parent_fixture_path(count,
                if mode == 2 { 1int } else { 2int }),
            _ => false,
        }
    } else {
        match out { Err(policy::ParentPathFailure::Native(error)) => error@ == "io: denied é\nend"@, _ => false }
    }
}

proof fn parent_fixture_step<M>(before: (u8, Vec<u8>), after: (u8, Vec<u8>), mode: u8, metadata: M)
    where M: Fn(Vec<u8>) -> (Vec<u8>, Result<bool, String>),
    requires policy::parent_path_step::<u8, Vec<u8>, u8, String, _, _, M, _>(
        before, after, parent_fixture_next, parent_fixture_append, metadata, parent_fixture_missing),
        forall|path: Vec<u8>, observed: (Vec<u8>, Result<bool, String>)|
            #[trigger] metadata.ensures((path,), observed) ==>
                observed.0 == path && parent_fixture_observation(mode, path@, observed.1),
    ensures before.0 > 0, after.0 as int == before.0 as int - 1,
        after.1@ == before.1@ + seq![before.0],
        parent_fixture_kind(mode, after.1@.len()) != 1,
        parent_fixture_kind(mode, after.1@.len()) != 3,
{
    reveal(policy::parent_path_step);
    reveal_strlit("missing: é\0end");
    reveal_strlit("io: denied é\nend");
}

proof fn parent_fixture_result<M>(count: u8, mode: u8, root: Vec<u8>, metadata: M,
    out: Result<Vec<u8>, policy::ParentPathFailure<Vec<u8>, String>>)
    where M: Fn(Vec<u8>) -> (Vec<u8>, Result<bool, String>),
    requires count <= 3, mode <= 6, root@ == seq![77u8, 0, 255],
        forall|path: Vec<u8>, observed: (Vec<u8>, Result<bool, String>)|
            #[trigger] metadata.ensures((path,), observed) ==>
                observed.0 == path && parent_fixture_observation(mode, path@, observed.1),
        policy::parent_path_result(count, root, parent_fixture_next, parent_fixture_append,
            metadata, parent_fixture_missing, out),
    ensures parent_fixture_expected(count, mode, out),
{
    reveal_strlit("missing: é\0end");
    reveal_strlit("io: denied é\nend");
    let (states, following, built, observed) = choose|states: Seq<(u8, Vec<u8>)>, following: (u8, Option<u8>),
        built: Option<Vec<u8>>, observed: Option<(Vec<u8>, Result<bool, String>)>|
        policy::parent_path_prefix::<u8, Vec<u8>, u8, String, _, _, M, _>(states, (count, root),
            parent_fixture_next, parent_fixture_append, metadata, parent_fixture_missing)
        && policy::parent_path_terminal(states.last(), following, built, observed,
            parent_fixture_next, parent_fixture_append, metadata, parent_fixture_missing, out);
    if states.len() > 1 {
        parent_fixture_step(states[0], states[1], mode, metadata);
        assert(states[1].1@ == parent_fixture_path(count, 1));
    }
    if states.len() > 2 {
        parent_fixture_step(states[1], states[2], mode, metadata);
        assert(states[2].1@ == parent_fixture_path(count, 2));
    }
    if states.len() > 3 {
        parent_fixture_step(states[2], states[3], mode, metadata);
        assert(states[3].0 == 0);
        assert(states[3].1@ == parent_fixture_path(count, 3));
    }
    if states.len() > 4 {
        parent_fixture_step(states[3], states[4], mode, metadata);
        assert(false);
    }
    assert(1 <= states.len() <= 4);
    let last = states.last();
    if states.len() == 1 { assert(last == states[0]); }
    else if states.len() == 2 { assert(last == states[1]); }
    else if states.len() == 3 { assert(last == states[2]); }
    else { assert(last == states[3]); }
    assert(last.0 as int == count as int + 1 - states.len());
    assert(last.1@ == parent_fixture_path(count, states.len() - 1));
    match following.1 {
        None => {
            assert(last.0 == 0);
            assert(states.len() == count as int + 1);
            assert(parent_fixture_expected(count, mode, out));
        },
        Some(component) => {
            assert(component == last.0 && last.0 > 0);
            let built = built->Some_0;
            let observed = observed->Some_0;
            assert(built@ == last.1@ + seq![component]);
            assert(observed.0 == built);
            assert(parent_fixture_observation(mode, built@, observed.1));
            if states.len() == 1 { assert(built@ == parent_fixture_path(count, 1)); }
            else if states.len() == 2 { assert(built@ == parent_fixture_path(count, 2)); }
            else { assert(built@ == parent_fixture_path(count, 3)); }
            match observed.1 {
                Ok(true) => { assert(parent_fixture_expected(count, mode, out)); },
                Err(error) => {
                    assert(parent_fixture_kind(mode, built@.len()) == 3);
                    assert(parent_fixture_expected(count, mode, out));
                },
                _ => { assert(false); },
            }
        },
    }
}

fn parent_fixture_run(count: u8, mode: u8)
    -> (out: Result<Vec<u8>, policy::ParentPathFailure<Vec<u8>, String>>)
    requires count <= 3, mode <= 6,
    ensures parent_fixture_expected(count, mode, out),
{
    let root = vec![77u8, 0, 255];
    let ghost original_root = root;
    let metadata = |path: Vec<u8>| -> (out: (Vec<u8>, Result<bool, String>))
        ensures out.0 == path, parent_fixture_observation(mode, path@, out.1),
    { parent_fixture_metadata(mode, path) };
    assert(policy::component_steps_finite::<u8, u8, _>(parent_fixture_next, |source: u8| source as nat));
    assert(exists|rank: spec_fn(u8) -> nat| policy::component_steps_finite::<u8, u8, _>(parent_fixture_next, rank));
    let out = policy::no_symlink_path(count, root, parent_fixture_next, parent_fixture_append,
        metadata, parent_fixture_missing);
    proof { parent_fixture_result(count, mode, original_root, metadata, out); }
    out
}

fn expect_parent_bytes(actual: &[u8], expected: &[u8])
    requires actual@ == expected@,
{
    expect_path_true(actual.len() == expected.len());
    let mut i = 0;
    while i < actual.len()
        invariant i <= actual.len(), actual@ == expected@,
        decreases actual.len() - i,
    { expect_path_true(actual[i] == expected[i]); i += 1; }
}

fn expect_parent_success(out: Result<Vec<u8>, policy::ParentPathFailure<Vec<u8>, String>>, expected: &[u8])
    requires out matches Ok(path) && path@ == expected@,
{
    match out { Ok(path) => expect_parent_bytes(path.as_slice(), expected), _ => panic!() }
}

fn expect_parent_symlink(out: Result<Vec<u8>, policy::ParentPathFailure<Vec<u8>, String>>, expected: &[u8])
    requires out matches Err(policy::ParentPathFailure::Symlink(path)) && path@ == expected@,
{
    match out { Err(policy::ParentPathFailure::Symlink(path)) => expect_parent_bytes(path.as_slice(), expected), _ => panic!() }
}

fn expect_parent_native(out: Result<Vec<u8>, policy::ParentPathFailure<Vec<u8>, String>>)
    requires out matches Err(policy::ParentPathFailure::Native(error)) && error@ == "io: denied é\nend"@,
{
    match out { Err(policy::ParentPathFailure::Native(error)) => expect_path_text(error.as_str(), "io: denied é\nend"), _ => panic!() }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn parent_walk_empty_never_observes_metadata() {
    let mut mode = 0u8;
    while mode < 7
        invariant mode <= 7,
        decreases 7 - mode,
    { expect_parent_success(parent_fixture_run(0, mode), &[77u8, 0, 255]); mode += 1; }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn parent_walk_checks_final_component_and_continues_after_missing() {
    expect_parent_success(parent_fixture_run(3, 0), &[77u8, 0, 255, 3, 2, 1]);
    expect_parent_success(parent_fixture_run(3, 1), &[77u8, 0, 255, 3, 2, 1]);
    expect_parent_success(parent_fixture_run(3, 6), &[77u8, 0, 255, 3, 2, 1]);
    expect_parent_success(parent_fixture_run(1, 3), &[77u8, 0, 255, 1]);
    expect_parent_success(parent_fixture_run(1, 5), &[77u8, 0, 255, 1]);
    expect_parent_symlink(parent_fixture_run(1, 2), &[77u8, 0, 255, 1]);
    expect_parent_symlink(parent_fixture_run(3, 2), &[77u8, 0, 255, 3]);
    expect_parent_symlink(parent_fixture_run(3, 3), &[77u8, 0, 255, 3, 2]);
    expect_parent_native(parent_fixture_run(3, 4));
    expect_parent_native(parent_fixture_run(3, 5));
    expect_parent_native(parent_fixture_run(1, 4));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn parent_failure_renderer_preserves_complete_value_and_selects_one_branch() {
    let symlink = policy::ParentPathFailure::<Vec<u8>, Vec<u8>>::Symlink(vec![77u8, 0, 255]);
    let (kind, bytes) = symlink.render(
        |path: Vec<u8>| -> (out: (bool, Vec<u8>)) ensures out == (true, path), { (true, path) },
        |error: Vec<u8>| -> (out: (bool, Vec<u8>)) ensures out == (false, error), { (false, error) });
    expect_path_true(kind);
    expect_parent_bytes(bytes.as_slice(), &[77u8, 0, 255]);
    let native = policy::ParentPathFailure::<Vec<u8>, Vec<u8>>::Native(vec![0u8, 10, 255, 99]);
    let (kind, bytes) = native.render(
        |path: Vec<u8>| -> (out: (bool, Vec<u8>)) ensures out == (true, path), { (true, path) },
        |error: Vec<u8>| -> (out: (bool, Vec<u8>)) ensures out == (false, error), { (false, error) });
    expect_path_true(!kind);
    expect_parent_bytes(bytes.as_slice(), &[0u8, 10, 255, 99]);
}

}
