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

spec fn parent_fixture_kind(mode: u8, length: nat) -> u8 {
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
    let (component, built, observation) = choose|component: u8, built: Vec<u8>, observation: Result<bool, String>|
        parent_fixture_next.ensures((before.0,), (after.0, Some(component)))
        && parent_fixture_append.ensures((before.1, component), built)
        && metadata.ensures((built,), (after.1, observation))
        && policy::parent_path_continues(observation, parent_fixture_missing);
    assert(parent_fixture_observation(mode, built@, observation));
    assert(after.1 == built);
    if parent_fixture_kind(mode, built@.len()) == 3 {
        let error = observation->Err_0;
        assert(error@ == "io: denied é\nend"@);
        assert(error@.len() != "missing: é\0end"@.len());
        assert(error@ != "missing: é\0end"@);
        assert(parent_fixture_missing.ensures((&error,), true));
        assert(false);
    }
}

proof fn parent_fixture_prefix_at<M>(states: Seq<(u8, Vec<u8>)>, initial: (u8, Vec<u8>), metadata: M, index: int)
    where M: Fn(Vec<u8>) -> (Vec<u8>, Result<bool, String>),
    requires policy::parent_path_prefix::<u8, Vec<u8>, u8, String, _, _, M, _>(states, initial,
        parent_fixture_next, parent_fixture_append, metadata, parent_fixture_missing),
        0 <= index < states.len() - 1,
    ensures policy::parent_path_step::<u8, Vec<u8>, u8, String, _, _, M, _>(
        states[index], states[index + 1], parent_fixture_next, parent_fixture_append, metadata, parent_fixture_missing),
{ }

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
        parent_fixture_prefix_at(states, (count, root), metadata, 0);
        parent_fixture_step(states[0], states[1], mode, metadata);
        assert(states[1].1@ == parent_fixture_path(count, 1));
    }
    if states.len() > 2 {
        parent_fixture_prefix_at(states, (count, root), metadata, 1);
        parent_fixture_step(states[1], states[2], mode, metadata);
        assert(states[2].1@ == parent_fixture_path(count, 2));
    }
    if states.len() > 3 {
        parent_fixture_prefix_at(states, (count, root), metadata, 2);
        parent_fixture_step(states[2], states[3], mode, metadata);
        assert(states[3].0 == 0);
        assert(states[3].1@ == parent_fixture_path(count, 3));
    }
    if states.len() > 4 {
        parent_fixture_prefix_at(states, (count, root), metadata, 3);
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
    assert(parent_fixture_path(3, 3) =~= seq![77u8, 0, 255, 3, 2, 1]);
    assert(parent_fixture_path(1, 1) =~= seq![77u8, 0, 255, 1]);
    assert(parent_fixture_path(3, 1) =~= seq![77u8, 0, 255, 3]);
    assert(parent_fixture_path(3, 2) =~= seq![77u8, 0, 255, 3, 2]);
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

// 0–3：directory+file／directory／file／neither；4–7：四种 symlink 组合；8：missing；9：native。
// 这是独立的有限观察表，不从生产分类谓词生成预期。
pub open spec fn publication_expected(count: u8, first: u8, second: u8) -> Result<(), (u8, u8)> {
    match (count, first, second) {
        (0, _, _) => Ok(()),
        (1, 0, _) | (1, 2, _) | (1, 8, _) => Ok(()),
        (1, 1, _) | (1, 3, _) => Err((3, 1)),
        (1, 4 | 5 | 6 | 7, _) => Err((1, 1)),
        (1, 9, _) => Err((4, 61)),
        (2, 2, _) | (2, 3, _) => Err((2, 1)),
        (2, 4 | 5 | 6 | 7, _) => Err((1, 1)),
        (2, 8, _) => Ok(()),
        (2, 9, _) => Err((4, 61)),
        (2, 0, 0) | (2, 0, 2) | (2, 0, 8)
            | (2, 1, 0) | (2, 1, 2) | (2, 1, 8) => Ok(()),
        (2, 0, 1) | (2, 0, 3) | (2, 1, 1) | (2, 1, 3) => Err((3, 2)),
        (2, 0 | 1, 4 | 5 | 6 | 7) => Err((1, 2)),
        (2, 0, 9) | (2, 1, 9) => Err((4, 62)),
        _ => Err((5, 0)),
    }
}

fn publication_expected_value(count: u8, first: u8, second: u8) -> (out: Result<(), (u8, u8)>)
    ensures out == publication_expected(count, first, second),
{
    match (count, first, second) {
        (0, _, _) => Ok(()),
        (1, 0, _) | (1, 2, _) | (1, 8, _) => Ok(()),
        (1, 1, _) | (1, 3, _) => Err((3, 1)),
        (1, 4 | 5 | 6 | 7, _) => Err((1, 1)),
        (1, 9, _) => Err((4, 61)),
        (2, 2, _) | (2, 3, _) => Err((2, 1)),
        (2, 4 | 5 | 6 | 7, _) => Err((1, 1)),
        (2, 8, _) => Ok(()),
        (2, 9, _) => Err((4, 61)),
        (2, 0, 0) | (2, 0, 2) | (2, 0, 8)
            | (2, 1, 0) | (2, 1, 2) | (2, 1, 8) => Ok(()),
        (2, 0, 1) | (2, 0, 3) | (2, 1, 1) | (2, 1, 3) => Err((3, 2)),
        (2, 0 | 1, 4 | 5 | 6 | 7) => Err((1, 2)),
        (2, 0, 9) | (2, 1, 9) => Err((4, 62)),
        _ => Err((5, 0)),
    }
}

pub open spec fn publication_observed(mode: u8, path: u8, out: Result<policy::PublicationMetadata, u8>) -> bool {
    match out {
        Ok(value) => mode < 8 && value.symlink == (mode >= 4)
            && value.directory == (mode == 0 || mode == 1 || mode == 5 || mode == 7)
            && value.file == (mode == 0 || mode == 2 || mode == 6 || mode == 7),
        Err(error) => (mode == 8 && error == 1)
            || (mode == 9 && error == if path == 1 { 61u8 } else { 62u8 }),
    }
}

fn publication_observation(mode: u8, path: u8) -> (out: Result<policy::PublicationMetadata, u8>)
    requires mode <= 9,
    ensures publication_observed(mode, path, out),
{
    if mode == 8 { Err(1) }
    else if mode == 9 { Err(if path == 1 { 61 } else { 62 }) }
    else { Ok(policy::PublicationMetadata {
        symlink: mode >= 4, directory: mode == 0 || mode == 1 || mode == 5 || mode == 7,
        file: mode == 0 || mode == 2 || mode == 6 || mode == 7,
    }) }
}

fn publication_append(_path: u8, component: u8) -> (out: u8)
    ensures out == component,
{ component }

fn publication_missing(error: &u8) -> (out: bool)
    ensures out == (*error == 1),
{ *error == 1 }

pub open spec fn publication_failure_code(error: policy::PublicationPathFailure<u8, u8, u8>) -> (u8, u8) {
    match error {
        policy::PublicationPathFailure::Symlink(path) => (1, path),
        policy::PublicationPathFailure::Parent(path) => (2, path),
        policy::PublicationPathFailure::Destination(path) => (3, path),
        policy::PublicationPathFailure::Native(error) => (4, error),
        policy::PublicationPathFailure::Followup(error) => (5, error),
    }
}

fn publication_code<T>(out: Result<T, policy::PublicationPathFailure<u8, u8, u8>>)
    -> (result: Result<T, (u8, u8)>)
    ensures result == match out { Ok(value) => Ok(value), Err(error) => Err(publication_failure_code(error)) },
{
    match out {
        Ok(value) => Ok(value),
        Err(error) => Err(error.render(
            |path: u8| -> (out: (u8, u8)) ensures out == (1u8, path), { (1, path) },
            |path: u8| -> (out: (u8, u8)) ensures out == (2u8, path), { (2, path) },
            |path: u8| -> (out: (u8, u8)) ensures out == (3u8, path), { (3, path) },
            |error: u8| -> (out: (u8, u8)) ensures out == (4u8, error), { (4, error) },
            |error: u8| -> (out: (u8, u8)) ensures out == (5u8, error), { (5, error) })),
    }
}

proof fn publication_fixture_result<A, M, N>(count: u8, first: u8, second: u8, parts: Seq<u8>, append: A, metadata: M, missing: N,
    out: Result<(), policy::PublicationPathFailure<u8, u8, u8>>)
    where A: Fn(u8, u8) -> u8, M: Fn(u8) -> (u8, Result<policy::PublicationMetadata, u8>), N: Fn(&u8) -> bool,
    requires count <= 2, first <= 9, second <= 9,
        parts == seq![1u8, 2].take(count as int),
        forall|path: u8, component: u8, built: u8| #[trigger] append.ensures((path, component), built) ==> built == component,
        forall|error: &u8, matched: bool| #[trigger] missing.ensures((error,), matched) ==> matched == (*error == 1),
        forall|path: u8, observed: (u8, Result<policy::PublicationMetadata, u8>)|
            #[trigger] metadata.ensures((path,), observed) ==> observed.0 == path
                && publication_observed(if path == 1 { first } else { second }, path, observed.1),
        policy::publication_preflight_result(parts, 0, 0u8, append, metadata, missing, out),
    ensures match out { Ok(()) => Ok::<(), (u8, u8)>(()), Err(error) => Err::<(), (u8, u8)>(publication_failure_code(error)) }
        == publication_expected(count, first, second),
{
    assert(parts.len() == count as int);
    reveal_with_fuel(policy::publication_preflight_result, 4);
    if count > 0 {
        assert(parts[0] == 1u8);
        let (built, observed) = choose|built: u8, observed: (u8, Result<policy::PublicationMetadata, u8>)|
            append.ensures((0u8, parts[0]), built)
            && metadata.ensures((built,), observed)
            && policy::publication_observation_result(observed, 1 == parts.len(), missing,
                policy::publication_preflight_result(parts, 1, observed.0, append, metadata, missing, out), out);
        assert(built == 1 && observed.0 == 1);
        assert(publication_observed(first, 1, observed.1));
        if count == 2 && (first == 0 || first == 1) {
            assert(parts[1] == 2u8);
            assert(policy::publication_preflight_result(parts, 1, 1u8, append, metadata, missing, out));
            let (built, observed) = choose|built: u8, observed: (u8, Result<policy::PublicationMetadata, u8>)|
                append.ensures((1u8, parts[1]), built)
                && metadata.ensures((built,), observed)
                && policy::publication_observation_result(observed, true, missing,
                    policy::publication_preflight_result(parts, 2, observed.0, append, metadata, missing, out), out);
            assert(built == 2 && observed.0 == 2);
            assert(publication_observed(second, 2, observed.1));
        }
    }
}

fn publication_fixture(count: u8, first: u8, second: u8) -> (out: Result<(), (u8, u8)>)
    requires count <= 2, first <= 9, second <= 9,
    ensures out == publication_expected(count, first, second),
{
    let mut parts = Vec::new();
    if count > 0 { parts.push(1u8); }
    if count > 1 { parts.push(2u8); }
    assert(parts@ == seq![1u8, 2].take(count as int));
    let metadata = |path: u8| -> (out: (u8, Result<policy::PublicationMetadata, u8>))
        ensures out.0 == path,
            publication_observed(if path == 1 { first } else { second }, path, out.1),
    { (path, publication_observation(if path == 1 { first } else { second }, path)) };
    let result = policy::publication_preflight(parts.as_slice(), 0u8, publication_append, metadata, publication_missing);
    proof { publication_fixture_result(count, first, second, parts@, publication_append, metadata, publication_missing, result); }
    publication_code(result)
}

fn expect_publication_result(actual: Result<(), (u8, u8)>, expected: Result<(), (u8, u8)>)
    requires actual == expected,
{
    match (actual, expected) {
        (Ok(()), Ok(())) => {},
        (Err(actual), Err(expected)) => {
            expect_path_true(actual.0 == expected.0);
            expect_path_true(actual.1 == expected.1);
        },
        _ => panic!(),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn publication_preflight_obeys_independent_first_error_and_missing_table() {
    let mut count = 0u8;
    while count < 3
        invariant count <= 3,
        decreases 3 - count,
    {
        let mut first = 0u8;
        while first < 10
            invariant first <= 10, count < 3,
            decreases 10 - first,
        {
            let mut second = 0u8;
            while second < 10
                invariant second <= 10, first < 10, count < 3,
                decreases 10 - second,
            {
                expect_publication_result(publication_fixture(count, first, second),
                    publication_expected_value(count, first, second));
                second += 1;
            }
            first += 1;
        }
        count += 1;
    }
}

pub open spec fn publication_completed_expected(count: u8, first: u8, second: u8,
    has_parent: bool, create_error: bool, finish_error: bool) -> Result<u8, (u8, u8)>
{
    match publication_expected(count, first, second) {
        Err(error) => Err(error),
        Ok(()) => if has_parent && create_error { Err((4, 71)) }
            else if finish_error { Err((5, 81)) }
            else { Ok(91) },
    }
}

fn publication_completed_expected_value(count: u8, first: u8, second: u8,
    has_parent: bool, create_error: bool, finish_error: bool) -> (out: Result<u8, (u8, u8)>)
    ensures out == publication_completed_expected(count, first, second, has_parent, create_error, finish_error),
{
    match publication_expected_value(count, first, second) {
        Err(error) => Err(error),
        Ok(()) => if has_parent && create_error { Err((4, 71)) }
            else if finish_error { Err((5, 81)) }
            else { Ok(91) },
    }
}

proof fn publication_completion_result<'a, R, D, Z>(target: &'a u8, parent: R, create: D, finish: Z,
    has_parent: bool, create_error: bool, finish_error: bool,
    out: Result<u8, policy::PublicationPathFailure<u8, u8, u8>>)
    where R: Fn(&'a u8) -> Option<u8>, D: Fn(u8) -> Result<(), u8>, Z: Fn() -> Result<u8, u8>,
    requires forall|observed: Option<u8>| #[trigger] parent.ensures((target,), observed)
            ==> observed == if has_parent { Some(*target) } else { None },
        forall|path: u8, observed: Result<(), u8>| #[trigger] create.ensures((path,), observed)
            ==> observed == if create_error { Err::<(), u8>(71) } else { Ok::<(), u8>(()) },
        forall|observed: Result<u8, u8>| #[trigger] finish.ensures((), observed)
            ==> observed == if finish_error { Err::<u8, u8>(81) } else { Ok::<u8, u8>(91) },
        policy::publication_prepared(target, parent, create, finish, out),
    ensures match out { Ok(value) => Ok(value), Err(error) => Err(publication_failure_code(error)) }
        == if has_parent && create_error { Err::<u8, (u8, u8)>((4, 71)) }
            else if finish_error { Err::<u8, (u8, u8)>((5, 81)) }
            else { Ok::<u8, (u8, u8)>(91) },
{
    reveal(policy::publication_prepared);
}

fn publication_completed_fixture(count: u8, first: u8, second: u8,
    has_parent: bool, create_error: bool, finish_error: bool) -> (out: Result<u8, (u8, u8)>)
    requires count <= 2, first <= 9, second <= 9,
    ensures out == publication_completed_expected(count, first, second, has_parent, create_error, finish_error),
{
    let mut parts = Vec::new();
    if count > 0 { parts.push(1u8); }
    if count > 1 { parts.push(2u8); }
    assert(parts@ == seq![1u8, 2].take(count as int));
    let metadata = |path: u8| -> (out: (u8, Result<policy::PublicationMetadata, u8>))
        ensures out.0 == path,
            publication_observed(if path == 1 { first } else { second }, path, out.1),
    { (path, publication_observation(if path == 1 { first } else { second }, path)) };
    let parent = |target: &u8| -> (out: Option<u8>)
        ensures out == if has_parent { Some(*target) } else { None },
    { if has_parent { Some(*target) } else { None } };
    let create = |_path: u8| -> (out: Result<(), u8>)
        ensures out == if create_error { Err::<(), u8>(71) } else { Ok::<(), u8>(()) },
    { if create_error { Err(71) } else { Ok(()) } };
    let finish = || -> (out: Result<u8, u8>)
        ensures out == if finish_error { Err::<u8, u8>(81) } else { Ok::<u8, u8>(91) },
    { if finish_error { Err(81) } else { Ok(91) } };
    let target = 99u8;
    let result = policy::prepare_publication(parts.as_slice(), 0u8, &target,
        publication_append, metadata, publication_missing, parent, create, finish);
    proof {
        if exists|error: policy::PublicationPathFailure<u8, u8, u8>|
            policy::publication_preflight_result(parts@, 0, 0u8, publication_append, metadata, publication_missing, Err(error))
            && result == Err::<u8, policy::PublicationPathFailure<u8, u8, u8>>(error)
        {
            let error = choose|error: policy::PublicationPathFailure<u8, u8, u8>|
                policy::publication_preflight_result(parts@, 0, 0u8, publication_append, metadata, publication_missing, Err(error))
                && result == Err::<u8, policy::PublicationPathFailure<u8, u8, u8>>(error);
            publication_fixture_result(count, first, second, parts@, publication_append, metadata, publication_missing, Err(error));
        } else {
            publication_fixture_result(count, first, second, parts@, publication_append, metadata, publication_missing,
                Ok::<(), policy::PublicationPathFailure<u8, u8, u8>>(()));
            publication_completion_result(&target, parent, create, finish, has_parent, create_error, finish_error, result);
        }
    }
    publication_code(result)
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn publication_combination_preserves_preflight_create_and_finish_priority() {
    let mut count = 0u8;
    while count < 3
        invariant count <= 3,
        decreases 3 - count,
    {
        let mut first = 0u8;
        while first < 10
            invariant first <= 10, count < 3,
            decreases 10 - first,
        {
            let mut second = 0u8;
            while second < 10
                invariant second <= 10, first < 10, count < 3,
                decreases 10 - second,
            {
                let mut phases = 0u8;
                while phases < 8
                    invariant phases <= 8, second < 10, first < 10, count < 3,
                    decreases 8 - phases,
                {
                    let has_parent = phases >= 4;
                    let create_error = phases % 4 >= 2;
                    let finish_error = phases % 2 == 1;
                    let actual = publication_completed_fixture(count, first, second, has_parent, create_error, finish_error);
                    let expected = publication_completed_expected_value(count, first, second, has_parent, create_error, finish_error);
                    match (actual, expected) {
                        (Ok(actual), Ok(expected)) => expect_path_true(actual == expected),
                        (Err(actual), Err(expected)) => {
                            expect_path_true(actual.0 == expected.0);
                            expect_path_true(actual.1 == expected.1);
                        },
                        _ => panic!(),
                    }
                    phases += 1;
                }
                second += 1;
            }
            first += 1;
        }
        count += 1;
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn publication_failure_render_preserves_complete_path_and_both_error_payloads() {
    let mut variant = 0u8;
    while variant < 5
        invariant variant <= 5,
        decreases 5 - variant,
    {
        let original = vec![0u8, 10, 255, 47, variant];
        let error: policy::PublicationPathFailure<Vec<u8>, Vec<u8>, Vec<u8>> = match variant {
            0 => policy::PublicationPathFailure::Symlink(original),
            1 => policy::PublicationPathFailure::Parent(original),
            2 => policy::PublicationPathFailure::Destination(original),
            3 => policy::PublicationPathFailure::Native(original),
            _ => policy::PublicationPathFailure::Followup(original),
        };
        let (kind, bytes) = error.render(
            |value: Vec<u8>| -> (out: (u8, Vec<u8>)) ensures out == (0u8, value), { (0, value) },
            |value: Vec<u8>| -> (out: (u8, Vec<u8>)) ensures out == (1u8, value), { (1, value) },
            |value: Vec<u8>| -> (out: (u8, Vec<u8>)) ensures out == (2u8, value), { (2, value) },
            |value: Vec<u8>| -> (out: (u8, Vec<u8>)) ensures out == (3u8, value), { (3, value) },
            |value: Vec<u8>| -> (out: (u8, Vec<u8>)) ensures out == (4u8, value), { (4, value) });
        expect_path_true(kind == variant);
        expect_parent_bytes(bytes.as_slice(), &[0u8, 10, 255, 47, variant]);
        variant += 1;
    }
}

}
