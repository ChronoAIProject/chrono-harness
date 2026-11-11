//! scoped 观察与显式预期的独立成功／拒绝边界。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../runner/src/verified_git_binding.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_git_binding as policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

fn expect_true(value: bool)
    requires value,
{ if !value { panic!(); } }

fn expect_selector_binding_result(actual: Result<(), &str>, expected: Result<(), &str>)
    requires actual == expected,
{
    match (actual, expected) {
        (Ok(()), Ok(())) => (),
        (Err(actual), Err(expected)) => {
            expect_true(policy::matches_expected(actual, Some(expected)));
        },
        _ => { panic!(); },
    }
}

struct RegistryPathsFixture<'a>(Vec<Option<&'a str>>);

impl<'a> policy::SelectorRegistryRows<'a> for RegistryPathsFixture<'a> {
    open spec fn registry_paths(&self) -> Seq<Option<&'a str>> { self.0@ }
    fn len(&self) -> usize { self.0.len() }
    fn registry_path(&self, index: usize) -> Option<&'a str> { self.0[index] }
}

fn expect_selector_path_success(actual: Result<(), policy::SelectorPathError>)
    requires actual is Ok,
{
    match actual { Ok(()) => {}, Err(_) => { panic!(); } }
}

fn expect_selector_path_error(actual: Result<(), policy::SelectorPathError>,
    invalid_relative: bool, expected: &str)
    requires match actual {
        Ok(()) => false,
        Err(policy::SelectorPathError::InvalidRelative(path)) => invalid_relative && path@ == expected@,
        Err(policy::SelectorPathError::Message(message)) => !invalid_relative && message@ == expected@,
    },
{
    match actual {
        Ok(()) => { panic!(); },
        Err(policy::SelectorPathError::InvalidRelative(path)) => {
            expect_true(invalid_relative);
            expect_true(policy::matches_expected(path, Some(expected)));
        },
        Err(policy::SelectorPathError::Message(message)) => {
            expect_true(!invalid_relative);
            expect_true(policy::matches_expected(message, Some(expected)));
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn selector_registry_paths_skip_non_v3_and_accept_empty_v3() {
    let rows = RegistryPathsFixture(vec![None, Some("../bad\0\n"), Some("other/missing")]);
    expect_selector_path_success(policy::validate_identity_registry_paths(false, &rows, &[]));
    let empty = RegistryPathsFixture(Vec::new());
    expect_selector_path_success(policy::validate_identity_registry_paths(true, &empty, &[]));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn selector_registry_paths_accept_all_registered_rows_and_repeated_targets() {
    let first = ".chrono-harness/a";
    let second = ".chrono-harness/b";
    let rows = RegistryPathsFixture(vec![Some(first), Some(second), Some(first)]);
    expect_selector_path_success(policy::validate_identity_registry_paths(true, &rows, &[first, second]));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn selector_registry_paths_return_first_missing_or_invalid_row() {
    let first = ".chrono-harness/a";
    let invalid = ".chrono-harness/目录é\0\n尾部";
    let missing_first = RegistryPathsFixture(vec![Some(first), None, Some(invalid)]);
    expect_selector_path_error(policy::validate_identity_registry_paths(true, &missing_first, &[first]),
        false, "Git selector registry_path required");
    let invalid_first = RegistryPathsFixture(vec![Some(first), Some(invalid), None]);
    expect_selector_path_error(policy::validate_identity_registry_paths(true, &invalid_first, &[first]),
        true, invalid);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn selector_registry_paths_check_host_scope_before_exact_membership() {
    let outside = RegistryPathsFixture(vec![Some("other/policy")]);
    expect_selector_path_error(policy::validate_identity_registry_paths(true, &outside, &[]),
        false, "Git config selector paths must reside in .chrono-harness/");
    let different = RegistryPathsFixture(vec![Some(".chrono-harness/A")]);
    expect_selector_path_error(policy::validate_identity_registry_paths(true, &different, &[".chrono-harness/a"]),
        false, "Git selector registry_path must name a registered policy target");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn selector_complete_host_path_preserves_relative_host_and_self_error_priority() {
    let same = ".chrono-harness/a";
    expect_selector_path_error(policy::admit_complete_selector_host_path(same, Some(same)),
        false, "Git config selector cannot select itself");
    let invalid = ".chrono-harness/../a";
    expect_selector_path_error(policy::admit_complete_selector_host_path(invalid, Some(invalid)),
        true, invalid);
    expect_selector_path_success(policy::admit_complete_selector_host_path(same, Some(".chrono-harness/A")));
}

fn selector_invalid_path_echo(path: &str) -> (out: String)
    ensures out@ == path@,
{ path.to_owned() }

fn selector_unused_path_formatter(_unused: &str) -> String
    requires false,
{ panic!() }

#[cfg_attr(not(verus_keep_ghost), test)]
fn selector_path_error_render_keeps_full_text_and_skips_unused_formatter() {
    let original = "../目录é\0\n尾部";
    let rendered = policy::SelectorPathError::InvalidRelative(original).render(selector_invalid_path_echo);
    expect_true(policy::matches_expected(rendered.as_str(), Some(original)));
    let message = "完整错误é\0\n尾部";
    let rendered = policy::SelectorPathError::Message(message).render(selector_unused_path_formatter);
    expect_true(policy::matches_expected(rendered.as_str(), Some(message)));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn selector_binding_requires_an_input_before_checking_the_three_text_fields() {
    let program = "/git/版本";
    let wrong_program = "/git/版本\0";
    let digest = "hash";
    let wrong_digest = "HASH";
    let missing = "Git identity selector requires a declared executable input";
    let mismatch = "Git config selector identity differs from selected policy binding";
    proof {
        reveal_strlit("/git/版本"); reveal_strlit("/git/版本\0");
        reveal_strlit("hash"); reveal_strlit("HASH");
        assert(program@.len() != wrong_program@.len());
        assert(digest@[0] != wrong_digest@[0]);
    }
    expect_selector_binding_result(
        policy::admit_selector_binding(program, digest, Some(program), Some((Some(program), Some(digest)))),
        Ok(()),
    );
    expect_selector_binding_result(
        policy::admit_selector_binding(program, digest, Some(wrong_program), None), Err(missing),
    );
    expect_selector_binding_result(
        policy::admit_selector_binding(program, digest, Some(wrong_program), Some((Some(program), Some(digest)))),
        Err(mismatch),
    );
    expect_selector_binding_result(
        policy::admit_selector_binding(program, digest, Some(program), Some((Some(wrong_program), Some(digest)))),
        Err(mismatch),
    );
    expect_selector_binding_result(
        policy::admit_selector_binding(program, digest, Some(program), Some((Some(program), Some(wrong_digest)))),
        Err(mismatch),
    );
    expect_selector_binding_result(
        policy::admit_selector_binding(program, digest, Some(program), Some((Some(program), None))),
        Err(mismatch),
    );
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn input_is_optional_only_for_scoped_v4_and_other_link_constraints_remain() {
    expect_true(policy::valid_link(4, true, false, false));
    expect_true(policy::valid_link(4, true, true, false));
    expect_true(policy::valid_link(3, true, true, false));
    expect_true(!policy::valid_link(3, true, false, false));
    expect_true(!policy::valid_link(4, false, true, false));
    expect_true(!policy::valid_link(4, true, false, true));
    expect_true(!policy::valid_link(3, true, true, true));
    expect_true(!policy::valid_link(2, true, true, false));
    expect_true(!policy::valid_link(0, true, true, false));
}

pub struct CapturedPath<'a>(pub &'a str);

pub struct ReferenceIds<'a>(pub Vec<Option<&'a str>>);

impl policy::BindingReferenceRows for ReferenceIds<'_> {
    open spec fn count(&self) -> nat { self.0@.len() }
    open spec fn id_at(&self, index: int) -> Option<&str> { self.0@[index] }
    fn len(&self) -> usize { self.0.len() }
    fn id(&self, index: usize) -> Option<&str> { self.0[index] }
}

fn expect_reference_index<R: policy::BindingReferenceRows>(rows: &R, id: &str, expected: Option<usize>)
    requires match expected {
        Some(index) => policy::unique_binding_reference_index(rows, id, index as int),
        None => forall|i: int| !#[trigger] policy::unique_binding_reference_index(rows, id, i),
    },
{
    let result = policy::unique_binding_reference(Some(rows), id);
    match expected {
        Some(index) => {
            proof {
                assert(policy::unique_binding_reference_index(rows, id, index as int));
                assert(result == Ok::<Option<usize>, &str>(Some(index)));
            }
            match result {
                Ok(Some(actual)) => { expect_true(actual == index); },
                _ => { panic!(); },
            }
        },
        None => match result {
            Ok(None) => {},
            Ok(Some(actual)) => {
                assert(!policy::unique_binding_reference_index(rows, id, actual as int));
                panic!();
            },
            Err(_) => { panic!(); },
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn binding_reference_requires_an_array_even_when_the_requested_id_is_empty() {
    expect_identity_error(policy::unique_binding_reference::<ReferenceIds>(None, ""),
        "binding collection must be array");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn binding_reference_keeps_empty_and_nonmatching_collections_distinct_from_missing_arrays() {
    let mut rows = ReferenceIds(Vec::new());
    expect_reference_index(&rows, "id", None);
    rows.0.push(None);
    rows.0.push(Some("ID"));
    rows.0.push(Some("id\0"));
    rows.0.push(Some("idx"));
    proof {
        reveal_strlit("id"); reveal_strlit("ID"); reveal_strlit("id\0"); reveal_strlit("idx");
        assert("id"@[0] != "ID"@[0]);
        assert("id"@.len() != "id\0"@.len() && "id"@.len() != "idx"@.len());
        assert forall|i: int| !#[trigger] policy::unique_binding_reference_index(&rows, "id", i) by {}
    }
    expect_reference_index(&rows, "id", None);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn binding_reference_returns_the_original_last_index_after_missing_and_other_ids() {
    let mut rows = ReferenceIds(Vec::new());
    rows.0.push(None);
    rows.0.push(Some("other"));
    rows.0.push(Some("git"));
    proof {
        reveal_strlit("other"); reveal_strlit("git");
        assert("other"@.len() != "git"@.len());
        assert(policy::unique_binding_reference_index(&rows, "git", 2));
    }
    expect_reference_index(&rows, "git", Some(2));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn binding_reference_rejects_nonadjacent_duplicates_and_keeps_another_unique_id() {
    let mut rows = ReferenceIds(Vec::new());
    rows.0.push(Some("git"));
    rows.0.push(Some("other"));
    rows.0.push(Some("git"));
    proof {
        reveal_strlit("other"); reveal_strlit("git");
        assert("other"@.len() != "git"@.len());
        assert forall|i: int| !#[trigger] policy::unique_binding_reference_index(&rows, "git", i) by {
            if i == 0 { assert(policy::binding_reference_matches(&rows, 2, "git")); }
            else { assert(policy::binding_reference_matches(&rows, 0, "git")); }
        }
        assert(policy::unique_binding_reference_index(&rows, "other", 1));
    }
    expect_reference_index(&rows, "git", None);
    expect_reference_index(&rows, "other", Some(1));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn binding_reference_preserves_first_index_and_full_unicode_nul_and_empty_text() {
    let mut rows = ReferenceIds(Vec::new());
    rows.0.push(Some("λ\0"));
    rows.0.push(Some("λ"));
    rows.0.push(Some(""));
    proof {
        reveal_strlit("λ\0"); reveal_strlit("λ"); reveal_strlit("");
        assert("λ\0"@.len() != "λ"@.len());
        assert("λ\0"@.len() != ""@.len() && "λ"@.len() != ""@.len());
        assert(policy::unique_binding_reference_index(&rows, "λ\0", 0));
        assert(policy::unique_binding_reference_index(&rows, "", 2));
    }
    expect_reference_index(&rows, "λ\0", Some(0));
    expect_reference_index(&rows, "", Some(2));
}

impl policy::RegistryMembership for CapturedPath<'_> {
    open spec fn has_path(&self, path: &str) -> bool { path@ == self.0@ }

    fn contains_path(&self, path: &str) -> bool {
        policy::matches_expected(path, Some(self.0))
    }
}

fn expect_identity_error<T>(result: Result<T, &str>, expected: &str)
    requires match result { Ok(_) => false, Err(error) => error@ == expected@ },
{
    match result {
        Ok(_) => { panic!(); },
        Err(error) => { expect_true(policy::matches_expected(error, Some(expected))); },
    }
}

fn expect_guard_error<T>(result: Result<T, policy::GitGuardError<'_>>, prefix: &str, id: &str, suffix: &str)
    requires match result {
        Ok(_) => false,
        Err(error) => error.prefix@ == prefix@ && error.id@ == id@ && error.suffix@ == suffix@,
    },
{
    match result {
        Ok(_) => { panic!(); },
        Err(error) => {
            expect_true(policy::matches_expected(error.prefix, Some(prefix)));
            expect_true(policy::matches_expected(error.id, Some(id)));
            expect_true(policy::matches_expected(error.suffix, Some(suffix)));
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn guard_header_keeps_object_shape_schema_and_array_boundaries() {
    let shape = "Git input guard requires only schema chrono-git-inputs/v1 and inputs";
    proof {
        reveal_strlit("chrono-git-inputs/v1"); reveal_strlit("chrono-git-inputs/v1 ");
        assert("chrono-git-inputs/v1"@.len() != "chrono-git-inputs/v1 "@.len());
    }
    expect_identity_error(policy::admit_git_guard_object(None, None, false),
        "Git input guard must be an object");
    expect_identity_error(policy::admit_git_guard_object(Some(3), Some("chrono-git-inputs/v1"), true), shape);
    expect_identity_error(policy::admit_git_guard_object(Some(2), None, true), shape);
    expect_identity_error(policy::admit_git_guard_object(Some(2), Some("chrono-git-inputs/v1 "), true), shape);
    expect_identity_error(policy::admit_git_guard_object(Some(2), Some("chrono-git-inputs/v1"), false), shape);
    expect_true(policy::admit_git_guard_object(Some(2), Some("chrono-git-inputs/v1"), true).is_ok());
    expect_identity_error(policy::git_guard_ids::<u8>(None), "Git input guard requires nonempty inputs");
    expect_identity_error(policy::git_guard_ids(Some((&7u8, 0))), "Git input guard requires nonempty inputs");
    match policy::git_guard_ids(Some((&7u8, 2))) {
        Ok(original) => { expect_true(*original == 7); }, Err(_) => { panic!(); },
    }
    expect_identity_error(policy::git_guard_files::<u8>(None), "Git input guard requires file inputs");
    match policy::git_guard_files(Some(&9u8)) {
        Ok(original) => { expect_true(*original == 9); }, Err(_) => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn guard_ids_and_locations_preserve_complete_nonempty_literals() {
    proof { reveal_strlit(""); reveal_strlit(" λ\0 "); }
    expect_identity_error(policy::git_guard_id(None), "invalid Git input ID");
    expect_identity_error(policy::git_guard_id(Some("")), "invalid Git input ID");
    expect_identity_error(policy::git_guard_location(None), "Git input location required");
    expect_identity_error(policy::git_guard_location(Some("")), "Git input location required");
    match policy::git_guard_id(Some(" λ\0 ")) {
        Ok(original) => { expect_true(policy::matches_expected(original, Some(" λ\0 "))); },
        Err(_) => { panic!(); },
    }
    match policy::git_guard_location(Some(" λ\0 ")) {
        Ok(original) => { expect_true(policy::matches_expected(original, Some(" λ\0 "))); },
        Err(_) => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn guard_duplicate_precedes_lookup_and_unique_reference_preserves_original_index() {
    let mut rows = ReferenceIds(Vec::new());
    expect_guard_error(policy::git_guard_reference(&rows, " λ\0 ", false), "duplicate Git input ", " λ\0 ", "");
    proof {
        assert forall|i: int| !#[trigger] policy::unique_binding_reference_index(&rows, " λ\0 ", i) by {}
    }
    expect_guard_error(policy::git_guard_reference(&rows, " λ\0 ", true), "missing/ambiguous Git input ", " λ\0 ", "");
    rows.0.push(None);
    rows.0.push(Some("other"));
    rows.0.push(Some(" λ\0 "));
    proof {
        reveal_strlit("other"); reveal_strlit(" λ\0 ");
        assert("other"@[0] != " λ\0 "@[0]);
        assert(policy::unique_binding_reference_index(&rows, " λ\0 ", 2));
    }
    match policy::git_guard_reference(&rows, " λ\0 ", true) {
        Ok(index) => { expect_true(index == 2); }, Err(_) => { panic!(); },
    }
    rows.0.push(Some(" λ\0 "));
    proof {
        assert forall|i: int| !#[trigger] policy::unique_binding_reference_index(&rows, " λ\0 ", i) by {
            if i == 2 { assert(policy::binding_reference_matches(&rows, 3, " λ\0 ")); }
            else { assert(policy::binding_reference_matches(&rows, 2, " λ\0 ")); }
        }
    }
    expect_guard_error(policy::git_guard_reference(&rows, " λ\0 ", true), "missing/ambiguous Git input ", " λ\0 ", "");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn guard_presence_distinguishes_absent_digest_keys_and_requires_full_present_digest() {
    broadcast use vstd::string::is_ascii_spec_bytes;
    let digest = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let suffix = " requires explicit absence or a present digest";
    proof {
        reveal_strlit("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        reveal_strlit("present"); reveal_strlit("present "); reveal_strlit("ABSENT"); reveal_strlit("absent");
        reveal_strlit("");
        assert(vstd::string::is_ascii(digest));
        assert(policy::identifiers::digest_format(digest.spec_bytes()));
        assert("present"@.len() != "present "@.len());
        assert("ABSENT"@[0] != "absent"@[0] && "ABSENT"@.len() != "present"@.len());
    }
    expect_true(policy::admit_git_guard_presence(" λ\0 ", Some("absent"), false, None).is_ok());
    expect_guard_error(policy::admit_git_guard_presence(" λ\0 ", Some("absent"), true, None), "Git input ", " λ\0 ", suffix);
    expect_guard_error(policy::admit_git_guard_presence(" λ\0 ", Some("absent"), true, Some(digest)), "Git input ", " λ\0 ", suffix);
    expect_guard_error(policy::admit_git_guard_presence(" λ\0 ", None, false, None), "Git input ", " λ\0 ", suffix);
    expect_guard_error(policy::admit_git_guard_presence(" λ\0 ", Some("ABSENT"), false, None), "Git input ", " λ\0 ", suffix);
    expect_guard_error(policy::admit_git_guard_presence(" λ\0 ", Some("present"), true, None), "Git input ", " λ\0 ", suffix);
    expect_guard_error(policy::admit_git_guard_presence(" λ\0 ", Some("present"), true, Some("")), "Git input ", " λ\0 ", suffix);
    expect_guard_error(policy::admit_git_guard_presence(" λ\0 ", Some("present "), true, Some(digest)), "Git input ", " λ\0 ", suffix);
    expect_true(policy::admit_git_guard_presence(" λ\0 ", Some("present"), true, Some(digest)).is_ok());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn identity_header_requires_exact_fields_then_nonempty_configurations() {
    let fields = "Git selector identity requires only program and configurations";
    expect_identity_error(policy::admit_identity_object(None, false, false),
        "Git selector identity object required");
    expect_identity_error(policy::admit_identity_object(Some(3), true, true), fields);
    expect_identity_error(policy::admit_identity_object(Some(2), false, true), fields);
    expect_identity_error(policy::admit_identity_object(Some(2), true, false), fields);
    expect_true(policy::admit_identity_object(Some(2), true, true).is_ok());
    expect_identity_error(policy::admit_identity_configurations::<u8>(None),
        "Git selector requires nonempty configurations");
    expect_identity_error(policy::admit_identity_configurations(Some((&7u8, 0))),
        "Git selector requires nonempty configurations");
    match policy::admit_identity_configurations(Some((&7u8, 1))) {
        Ok(rows) => { expect_true(*rows == 7); },
        Err(_) => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn identity_program_keeps_missing_empty_absolute_and_control_boundaries() {
    broadcast use vstd::string::is_ascii_spec_bytes;
    proof {
        reveal_strlit(""); reveal_strlit("/git/tool"); reveal_strlit("/git/tool ");
        reveal_strlit("/git/\0"); reveal_strlit("/git/\n"); reveal_strlit("/git/\r");
        assert(vstd::string::is_ascii("/git/tool") && vstd::string::is_ascii("/git/tool "));
        assert(vstd::string::is_ascii("/git/\0") && vstd::string::is_ascii("/git/\n")
            && vstd::string::is_ascii("/git/\r"));
        assert("/git/\0".spec_bytes()[5] == 0 && "/git/\n".spec_bytes()[5] == 10
            && "/git/\r".spec_bytes()[5] == 13);
        assert(!policy::literal_program("/git/\0", true));
        assert(!policy::literal_program("/git/\n", true));
        assert(!policy::literal_program("/git/\r", true));
    }
    let missing = "Git selector program required";
    let invalid = "Git selector program must be an absolute literal path";
    expect_identity_error(policy::admit_identity_program(None, true), missing);
    expect_identity_error(policy::admit_identity_program(Some(""), false), missing);
    expect_identity_error(policy::admit_identity_program(Some("/git/tool"), false), invalid);
    expect_identity_error(policy::admit_identity_program(Some("/git/\0"), true), invalid);
    expect_identity_error(policy::admit_identity_program(Some("/git/\n"), true), invalid);
    expect_identity_error(policy::admit_identity_program(Some("/git/\r"), true), invalid);
    match policy::admit_identity_program(Some("/git/tool "), true) {
        Ok(program) => { expect_true(policy::matches_expected(program, Some("/git/tool "))); },
        Err(_) => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn identity_registry_target_requires_text_and_exact_membership() {
    proof {
        reveal_strlit("a"); reveal_strlit("a\0");
        assert("a"@.len() != "a\0"@.len());
    }
    expect_identity_error(policy::identity_registry_path(None), "Git selector registry_path required");
    match policy::identity_registry_path(Some("a")) {
        Ok(path) => { expect_true(policy::matches_expected(path, Some("a"))); },
        Err(_) => { panic!(); },
    }
    expect_true(policy::admit_identity_registry_target("a", &["a", "a"]).is_ok());
    expect_identity_error(policy::admit_identity_registry_target("a", &[]),
        "Git selector registry_path must name a registered policy target");
    expect_identity_error(policy::admit_identity_registry_target("a", &["a\0"]),
        "Git selector registry_path must name a registered policy target");
}

fn expect_captured_path(paths: &[&str], values: &CapturedPath, expected: &str, index: usize)
    requires policy::unique_registry_index(paths@, values, index as int),
        paths@[index as int]@ == expected@,
{
    match policy::select_captured_target(paths, values) {
        Ok(actual) => {
            proof {
                use policy::RegistryMembership;

                let actual_index = choose|i: int| policy::unique_registry_index(paths@, values, i)
                    && paths@[i]@ == actual@;
                assert(values.has_path(paths@[actual_index]));
                assert(values.has_path(paths@[index as int]));
                assert(actual_index == index);
            }
            expect_true(policy::matches_expected(actual, Some(expected)));
        },
        Err(_) => { panic!(); },
    }
}

fn expect_no_captured_target(paths: &[&str], values: &CapturedPath)
    requires forall|i: int| !policy::unique_registry_index(paths@, values, i),
{
    match policy::select_captured_target(paths, values) {
        Ok(_) => { panic!(); },
        Err(error) => {
            expect_true(policy::matches_expected(
                error, Some("registry values require exactly one registered effective policy")));
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn captured_registry_requires_one_matching_slot_and_preserves_the_declared_path() {
    let values = CapturedPath("目录/é\0");
    let paths = ["", "目录/é\0", "目录/é"];
    let misses = ["", "目录/é"];
    let duplicate = ["目录/é\0", "", "目录/é\0"];
    proof {
        reveal_strlit(""); reveal_strlit("目录/é\0"); reveal_strlit("目录/é");
        assert(""@.len() == 0 && "目录/é\0"@.len() == 5 && "目录/é"@.len() == 4);
    }
    expect_captured_path(&paths, &values, "目录/é\0", 1);
    expect_no_captured_target(&[], &values);
    expect_no_captured_target(&misses, &values);
    expect_no_captured_target(&duplicate, &values);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn null_version_is_scoped_only_and_missing_empty_or_wrong_types_are_rejected() {
    proof { reveal_strlit(""); reveal_strlit("version"); }
    expect_true(policy::valid_version(4, true, true, None));
    expect_true(!policy::valid_version(3, true, true, None));
    expect_true(!policy::valid_version(4, false, true, None));
    expect_true(!policy::valid_version(4, true, false, None));
    expect_true(!policy::valid_version(4, true, false, Some("")));
    expect_true(policy::valid_version(4, true, false, Some("version")));
    expect_true(policy::valid_version(3, true, false, Some("version")));
}

fn expect_missing_binding_text(result: Result<&str, policy::MissingBindingText>, expected_field: &str)
    requires match result {
        Err(message) => message.prefix@ == "missing "@ && message.field@ == expected_field@,
        Ok(_) => false,
    },
{
    match result {
        Err(message) => {
            expect_true(policy::matches_expected(message.prefix, Some("missing ")));
            expect_true(policy::matches_expected(message.field, Some(expected_field)));
        },
        Ok(_) => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn required_binding_text_preserves_original_and_complete_missing_field_diagnostics() {
    proof {
        reveal_strlit(""); reveal_strlit(" \0工具 ");
    }
    expect_missing_binding_text(policy::required_binding_text(None, "program"), "program");
    expect_missing_binding_text(policy::required_binding_text(Some(""), "工具\0"), "工具\0");
    expect_missing_binding_text(policy::required_binding_text(None, ""), "");
    match policy::required_binding_text(Some(" \0工具 "), "program") {
        Ok(text) => { expect_true(policy::matches_expected(text, Some(" \0工具 "))); },
        Err(_) => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn binding_tool_checks_program_before_all_other_invalid_fields() {
    proof { reveal_strlit(""); reveal_strlit("git"); }
    expect_identity_error(policy::admit_binding_tool(3, None, (false, false, None), None,
        Some((None, None, None))), "missing program");
    expect_identity_error(policy::admit_binding_tool(4, Some(""), (false, false, None), None,
        Some((None, None, None))), "missing program");
    expect_identity_error(policy::admit_binding_tool(4, Some("git"), (false, false, None), None,
        Some((None, None, None))), "facts_git requires an expected version, or null in v4");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn binding_tool_keeps_v3_v4_null_and_nonempty_version_boundaries() {
    proof { reveal_strlit("git"); reveal_strlit(""); reveal_strlit(" \0v "); }
    let error = "facts_git requires an expected version, or null in v4";
    expect_identity_error(policy::admit_binding_tool(3, Some("git"), (true, true, None),
        Some("PATH-once"), None), error);
    expect_identity_error(policy::admit_binding_tool(4, Some("git"), (false, true, None),
        Some("PATH-once"), None), error);
    expect_identity_error(policy::admit_binding_tool(4, Some("git"), (true, false, None),
        Some("PATH-once"), None), error);
    expect_identity_error(policy::admit_binding_tool(4, Some("git"), (true, false, Some("")),
        Some("PATH-once"), None), error);
    expect_true(policy::admit_binding_tool(4, Some("git"), (true, true, None), Some("PATH-once"), None).is_ok());
    expect_true(policy::admit_binding_tool(3, Some("git"), (true, false, Some(" \0v ")),
        Some("PATH-once"), None).is_ok());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn binding_tool_requires_exact_resolution_before_examining_optional_input() {
    proof {
        reveal_strlit(""); reveal_strlit("git"); reveal_strlit("PATH-once"); reveal_strlit("path-once");
        reveal_strlit("PATH-once\0");
        assert("PATH-once"@[0] != "path-once"@[0]);
        assert("PATH-once"@.len() != "PATH-once\0"@.len());
    }
    let error = "facts_git requires a PATH-once tool";
    let input = Some((None, None, None));
    expect_identity_error(policy::admit_binding_tool(4, Some("git"), (true, true, None), None, input), error);
    expect_identity_error(policy::admit_binding_tool(4, Some("git"), (true, true, None),
        Some("path-once"), input), error);
    expect_identity_error(policy::admit_binding_tool(4, Some("git"), (true, true, None),
        Some("PATH-once\0"), input), error);
    expect_identity_error(policy::admit_binding_tool(4, Some("git"), (true, true, None),
        Some("PATH-once"), input), "missing location");
    expect_identity_error(policy::admit_binding_tool(4, Some("git"), (true, true, None),
        Some("PATH-once"), Some((Some(""), None, None))), "missing location");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn binding_tool_requires_present_digest_input_and_preserves_nonempty_literals() {
    let digest = "0000000000000000000000000000000000000000000000000000000000000000";
    let uppercase = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    let short = "000000000000000000000000000000000000000000000000000000000000000";
    proof {
        reveal_strlit("0000000000000000000000000000000000000000000000000000000000000000");
        reveal_strlit("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
        reveal_strlit("000000000000000000000000000000000000000000000000000000000000000");
        reveal_strlit(" \0工具 "); reveal_strlit("present"); reveal_strlit("present\0");
        assert(vstd::string::is_ascii(digest) && vstd::string::is_ascii(uppercase) && vstd::string::is_ascii(short));
        vstd::string::is_ascii_spec_bytes(digest);
        vstd::string::is_ascii_spec_bytes(uppercase);
        vstd::string::is_ascii_spec_bytes(short);
        assert(policy::identifiers::digest_format(digest.spec_bytes()));
        assert(uppercase.spec_bytes()[0] == 65);
        assert(!policy::identifiers::digest_format(uppercase.spec_bytes()));
        assert(short.spec_bytes().len() == 63);
        assert(!policy::identifiers::digest_format(short.spec_bytes()));
        assert("present"@.len() != "present\0"@.len());
    }
    let error = "facts_git requires a present digest-bound input when declared";
    expect_identity_error(policy::admit_binding_tool(4, Some(" \0工具 "), (true, true, None),
        Some("PATH-once"), Some((Some(" \0工具 "), None, Some(digest)))), error);
    expect_identity_error(policy::admit_binding_tool(4, Some(" \0工具 "), (true, true, None),
        Some("PATH-once"), Some((Some(" \0工具 "), Some("present\0"), Some(digest)))), error);
    expect_identity_error(policy::admit_binding_tool(4, Some(" \0工具 "), (true, true, None),
        Some("PATH-once"), Some((Some(" \0工具 "), Some("present"), None))), error);
    expect_identity_error(policy::admit_binding_tool(4, Some(" \0工具 "), (true, true, None),
        Some("PATH-once"), Some((Some(" \0工具 "), Some("present"), Some(uppercase)))), error);
    expect_identity_error(policy::admit_binding_tool(4, Some(" \0工具 "), (true, true, None),
        Some("PATH-once"), Some((Some(" \0工具 "), Some("present"), Some(short)))), error);
    expect_true(policy::admit_binding_tool(4, Some(" \0工具 "), (true, true, None),
        Some("PATH-once"), Some((Some(" \0工具 "), Some("present"), Some(digest)))).is_ok());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn binding_version_arguments_reject_only_the_empty_decoded_array() {
    expect_identity_error(policy::admit_binding_version_argv(0), "facts_git requires version_argv");
    expect_true(policy::admit_binding_version_argv(1).is_ok());
    expect_true(policy::admit_binding_version_argv(usize::MAX).is_ok());
}

pub struct TargetRows<'a>(pub &'a [(Option<&'a str>, Option<&'a str>)]);

impl policy::GitTargetRows for TargetRows<'_> {
    open spec fn count(&self) -> nat { self.0@.len() }

    open spec fn text_at(&self, index: int, field: policy::GitTargetField) -> Option<&str> {
        match field {
            policy::GitTargetField::Digest => self.0@[index].0,
            policy::GitTargetField::Path => self.0@[index].1,
        }
    }

    fn len(&self) -> usize { self.0.len() }

    fn text(&self, index: usize, field: &policy::GitTargetField) -> Option<&str> {
        let (digest, path) = self.0[index];
        match field {
            policy::GitTargetField::Digest => digest,
            policy::GitTargetField::Path => path,
        }
    }
}

proof fn first_target_unique(
    rows: &TargetRows, field: policy::GitTargetField, text: Option<&str>,
    a: Option<usize>, b: Option<usize>,
)
    requires policy::first_target(rows, field, text, a), policy::first_target(rows, field, text, b),
    ensures a == b,
{
    match (a, b) {
        (Some(a), Some(b)) => {
            assert(policy::target_matches(rows, a as int, field, text));
            assert(policy::target_matches(rows, b as int, field, text));
        },
        (Some(a), None) => { assert(policy::target_matches(rows, a as int, field, text)); },
        (None, Some(b)) => { assert(policy::target_matches(rows, b as int, field, text)); },
        (None, None) => {},
    }
}

fn expect_target(rows: &TargetRows, digest: Option<&str>, path: Option<&str>, expected: Option<usize>)
    requires policy::preferred_target(rows, digest, path, expected),
{
    let actual = policy::select_git_target(rows, digest, path);
    proof {
        use policy::GitTargetField::{Digest, Path};
        if actual.is_some() && policy::first_target(rows, Digest, digest, actual) {
            if policy::first_target(rows, Digest, digest, None) {
                first_target_unique(rows, Digest, digest, actual, None);
            }
            first_target_unique(rows, Digest, digest, actual, expected);
        } else {
            if expected.is_some() && policy::first_target(rows, Digest, digest, expected) {
                first_target_unique(rows, Digest, digest, expected, None);
            }
            first_target_unique(rows, Path, path, actual, expected);
        }
    }
    match (actual, expected) {
        (Some(a), Some(e)) => expect_true(a == e),
        (None, None) => {},
        _ => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn git_target_prefers_digest_then_first_path_and_never_guesses() {
    let input = [(Some("old"), Some("a")), (Some("actual"), Some("b")),
        (Some("actual"), Some("a")), (None, None)];
    let rows = TargetRows(&input);
    use policy::GitTargetRows;
    use policy::GitTargetField::{Digest, Path};
    proof {
        reveal_strlit("old"); reveal_strlit("actual"); reveal_strlit("a");
        reveal_strlit("b"); reveal_strlit("none"); reveal_strlit("a\0");
        assert("old"@.len() != "actual"@.len() && "old"@.len() != "none"@.len());
        assert("actual"@.len() != "none"@.len());
        assert("a"@[0] != "b"@[0]);
        assert("a\0"@.len() != "a"@.len() && "a\0"@.len() != "b"@.len());
        assert(rows.count() == 4);
        assert(rows.text_at(0, Digest) == Some("old"));
        assert(rows.text_at(1, Digest) == Some("actual"));
        assert(rows.text_at(2, Digest) == Some("actual"));
        assert(rows.text_at(3, Digest).is_none());
        assert(rows.text_at(0, Path) == Some("a"));
        assert(rows.text_at(1, Path) == Some("b"));
        assert(rows.text_at(2, Path) == Some("a"));
        assert(rows.text_at(3, Path).is_none());
        assert forall|i: int| 0 <= i < 4 implies
            !policy::target_matches(&rows, i, Digest, Some("none")) by {
            assert(i == 0 || i == 1 || i == 2 || i == 3);
        }
        assert forall|i: int| 0 <= i < 4 implies
            !policy::target_matches(&rows, i, Path, Some("a\0")) by {
            assert(i == 0 || i == 1 || i == 2 || i == 3);
        }
    }
    expect_target(&rows, Some("actual"), Some("a"), Some(1));
    expect_target(&rows, Some("none"), Some("a"), Some(0));
    expect_target(&rows, None, Some("b"), Some(1));
    expect_target(&rows, Some("actual"), None, Some(1));
    expect_target(&rows, Some("none"), None, None);
    expect_target(&rows, None, Some("a\0"), None);
    expect_target(&rows, None, None, None);
    expect_target(&TargetRows(&[]), Some("actual"), Some("a"), None);
}

fn an_observation_is_not_an_implicit_allowlist(actual: &str) {
    expect_true(policy::matches_expected(actual, None));
    expect_true(policy::matches_expected(actual, Some(actual)));
}

fn a_declared_expectation_rejects_any_different_text(actual: &str, expected: &str)
    requires actual@ != expected@,
{
    expect_true(!policy::matches_expected(actual, Some(expected)));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn observations_keep_empty_unicode_and_control_bytes_and_expectations_are_exact() {
    proof {
        reveal_strlit(""); reveal_strlit("git"); reveal_strlit("git ");
        reveal_strlit("GIT"); reveal_strlit("git\0");
        assert("git"@.len() != "git "@.len());
        assert("git"@.len() != "git\0"@.len());
        assert("git"@[0] != "GIT"@[0]);
    }
    an_observation_is_not_an_implicit_allowlist("");
    an_observation_is_not_an_implicit_allowlist("Git 版本 λ\0\n");
    a_declared_expectation_rejects_any_different_text("git", "git ");
    a_declared_expectation_rejects_any_different_text("git", "git\0");
    a_declared_expectation_rejects_any_different_text("git", "GIT");
}

fn expect_selector_error<T>(actual: Result<T, &str>, expected: &str)
    requires match actual { Ok(_) => false, Err(error) => error@ == expected@ },
{
    match actual {
        Ok(_) => { panic!(); },
        Err(error) => { expect_true(policy::matches_expected(error, Some(expected))); },
    }
}

fn expect_platform_name(name: &str, trimmed: &str, expected: bool)
    requires expected == (name@.len() > 0 && name@ == trimmed@),
{
    match policy::admit_selector_platform_name(name, trimmed) {
        Ok(()) => { expect_true(expected); },
        Err(error) => {
            expect_true(!expected);
            expect_true(policy::matches_expected(error, Some(
                "Git config selector platform must be nonempty and literal")));
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn platform_names_require_nonempty_original_text_equal_to_the_trim_observation() {
    proof {
        reveal_strlit(""); reveal_strlit("linux"); reveal_strlit(" linux");
        reveal_strlit("linux\n"); reveal_strlit("\u{2003}λ"); reveal_strlit("λ");
        reveal_strlit("\0"); reveal_strlit("lin\nux"); reveal_strlit("\u{200b}");
        assert("linux"@.len() != " linux"@.len());
        assert("linux"@.len() != "linux\n"@.len());
        assert("\u{2003}λ"@.len() != "λ"@.len());
    }
    expect_platform_name("", "", false);
    expect_platform_name("linux", "linux", true);
    expect_platform_name(" linux", "linux", false);
    expect_platform_name("linux\n", "linux", false);
    expect_platform_name("\u{2003}λ", "λ", false);
    expect_platform_name("λ", "λ", true);
    expect_platform_name("\0", "\0", true);
    expect_platform_name("lin\nux", "lin\nux", true);
    expect_platform_name("\u{200b}", "\u{200b}", true);
}

fn expect_selector_bytes(actual: &[u8], captured: &[u8], unchanged: bool)
    requires unchanged == (actual@ == captured@),
{
    match policy::admit_selector_bytes(actual, captured) {
        Ok(()) => { expect_true(unchanged); },
        Err(error) => {
            expect_true(!unchanged);
            expect_true(policy::matches_expected(error, Some("Git config selector changed")));
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn selector_stability_compares_every_original_byte_including_non_utf8() {
    let captured: &[u8] = &[0u8, 255, 10];
    let shorter: &[u8] = &[0u8, 255];
    let longer: &[u8] = &[0u8, 255, 10, 0];
    let empty: &[u8] = &[];
    let nul: &[u8] = &[0u8];
    proof {
        assert(captured@.len() == 3);
        assert(shorter@.len() == 2);
        assert(longer@.len() == 4);
        assert(empty@.len() == 0 && nul@.len() == 1);
    }
    expect_selector_bytes(&[], &[], true);
    expect_selector_bytes(&[0u8, 255, 10], &[0u8, 255, 10], true);
    expect_selector_bytes(&[0u8, 254, 10], &[0u8, 255, 10], false);
    expect_selector_bytes(shorter, captured, false);
    expect_selector_bytes(longer, captured, false);
    expect_selector_bytes(empty, nul, false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn selector_envelope_requires_exact_fields_and_a_nonempty_platform_object() {
    let shape = "Git config selector requires only schema and platforms";
    let platforms = "Git config selector requires nonempty platforms";
    expect_selector_error(policy::admit_selector_object(None, false), "Git config selector object required");
    expect_selector_error(policy::admit_selector_object(Some(2), false), shape);
    expect_selector_error(policy::admit_selector_object(Some(3), true), shape);
    expect_true(policy::admit_selector_object(Some(2), true).is_ok());
    expect_selector_error(policy::admit_selector_platforms::<u8>(None), platforms);
    expect_selector_error(policy::admit_selector_platforms(Some((&7u8, 0))), platforms);
    match policy::admit_selector_platforms(Some((&7u8, 1))) {
        Ok(value) => { expect_true(*value == 7); },
        Err(_) => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn selector_path_rejects_neighbor_prefix_and_self_after_host_admission() {
    broadcast use vstd::string::is_ascii_spec_bytes;
    let root = ".chrono-harness/";
    let target = ".chrono-harness/a";
    let neighbor = ".chrono-harness-x";
    proof {
        reveal_strlit(".chrono-harness/");
        reveal_strlit(".chrono-harness/a");
        reveal_strlit(".chrono-harness-x");
        assert(vstd::string::is_ascii(root) && vstd::string::is_ascii(target)
            && vstd::string::is_ascii(neighbor));
        assert(root.spec_bytes().len() == 16);
        assert(target.spec_bytes().take(16) =~= root.spec_bytes());
        assert(neighbor.spec_bytes()[15] != root.spec_bytes()[15]);
    }
    expect_true(policy::admit_selector_host_path(target, None).is_ok());
    expect_true(policy::admit_selector_host_path(target, Some(root)).is_ok());
    expect_selector_error(
        policy::admit_selector_host_path(target, Some(target)),
        "Git config selector cannot select itself",
    );
    expect_selector_error(
        policy::admit_selector_host_path(neighbor, Some(neighbor)),
        "Git config selector paths must reside in .chrono-harness/",
    );
}

fn expect_schema(schema: Option<&str>, expected: policy::GitConfigSchema)
    requires policy::git_config_schema(schema) == expected,
{
    use policy::GitConfigSchema::*;
    let actual = policy::classify_git_config_schema(schema);
    let same = match (actual, expected) {
        (Direct, Direct) | (V1, V1) | (V2, V2) | (V3, V3) | (Unsupported, Unsupported) => true,
        _ => false,
    };
    expect_true(same);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn git_selector_schema_distinguishes_exact_versions_from_direct_and_unknown() {
    use policy::GitConfigSchema::*;
    broadcast use vstd::string::is_ascii_spec_bytes;
    proof {
        reveal_strlit(""); reveal_strlit("chrono-git-configs");
        reveal_strlit("Chrono-git-configs/v1"); reveal_strlit("chrono-git-configs/v1");
        reveal_strlit("chrono-git-configs/v2"); reveal_strlit("chrono-git-configs/v3");
        reveal_strlit("chrono-git-configs/"); reveal_strlit("chrono-git-configs/v4");
        reveal_strlit("chrono-git-configs/v1\0");
        assert(vstd::string::is_ascii("") && vstd::string::is_ascii("chrono-git-configs"));
        assert(vstd::string::is_ascii("Chrono-git-configs/v1")
            && vstd::string::is_ascii("chrono-git-configs/"));
        assert(vstd::string::is_ascii("chrono-git-configs/v4")
            && vstd::string::is_ascii("chrono-git-configs/v1\0"));
        assert("".spec_bytes().len() == 0 && "chrono-git-configs".spec_bytes().len() == 18);
        assert("chrono-git-configs/".spec_bytes().len() == 19);
        assert("chrono-git-configs/"@.len() == 19);
        assert("chrono-git-configs/".spec_bytes().take(19) =~= "chrono-git-configs/".spec_bytes());
        assert("chrono-git-configs/v1"@[20] == '1' && "chrono-git-configs/v2"@[20] == '2'
            && "chrono-git-configs/v3"@[20] == '3' && "chrono-git-configs/v4"@[20] == '4');
        assert("chrono-git-configs/v1\0"@.len() == 22 && "chrono-git-configs/v1"@.len() == 21
            && "chrono-git-configs/v2"@.len() == 21 && "chrono-git-configs/v3"@.len() == 21);
        assert("Chrono-git-configs/v1".spec_bytes()[0] != "chrono-git-configs/".spec_bytes()[0]);
        assert("chrono-git-configs/v4".spec_bytes().take(19) =~= "chrono-git-configs/".spec_bytes());
        assert("chrono-git-configs/v1\0".spec_bytes().take(19) =~= "chrono-git-configs/".spec_bytes());
    }
    expect_schema(None, Direct);
    expect_schema(Some(""), Direct);
    expect_schema(Some("chrono-git-configs"), Direct);
    expect_schema(Some("Chrono-git-configs/v1"), Direct);
    expect_schema(Some("chrono-git-configs/v1"), V1);
    expect_schema(Some("chrono-git-configs/v2"), V2);
    expect_schema(Some("chrono-git-configs/v3"), V3);
    expect_schema(Some("chrono-git-configs/"), Unsupported);
    expect_schema(Some("chrono-git-configs/v4"), Unsupported);
    expect_schema(Some("chrono-git-configs/v1\0"), Unsupported);
}

pub struct SeenRows<'a>(pub Vec<&'a str>);

impl<'a> policy::SelectorSeen<'a> for SeenRows<'a> {
    open spec fn seen(&self) -> Set<Seq<char>> {
        self.0@.map_values(|text: &str| text@).to_set()
    }

    fn record(&mut self, text: &'a str) -> bool {
        let ghost keys = self.0@.map_values(|value: &str| value@);
        let mut i = 0;
        while i < self.0.len()
            invariant 0 <= i <= self.0.len(), keys == self.0@.map_values(|value: &str| value@),
                forall|j: int| 0 <= j < i ==> #[trigger] keys[j] != text@,
            decreases self.0.len() - i,
        {
            if policy::matches_expected(self.0[i], Some(text)) {
                assert(keys[i as int] == text@);
                assert(keys.contains(text@));
                assert(keys.to_set().insert(text@) =~= keys.to_set());
                return false;
            }
            i += 1;
        }
        proof {
            assert(!keys.contains(text@));
            keys.lemma_push_to_set_commute(text@);
            self.0@.lemma_push_map_commute(|value: &str| value@, text);
        }
        self.0.push(text);
        true
    }
}

fn expect_row_error(actual: Result<&str, &str>, expected: &str)
    requires match actual { Ok(_) => false, Err(error) => error@ == expected@ },
{
    match actual {
        Ok(_) => { panic!(); },
        Err(error) => { expect_true(policy::matches_expected(error, Some(expected))); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn selector_identity_rows_keep_shape_digest_and_path_error_order() {
    use policy::SelectorSeen;

    let mut hashes = SeenRows(Vec::new());
    let mut paths = SeenRows(Vec::new());
    let zero = "0000000000000000000000000000000000000000000000000000000000000000";
    let one = "1111111111111111111111111111111111111111111111111111111111111111";
    let target = ".chrono-harness/git.json";
    let malformed = "Git selector configuration requires a unique digest/path and the declared v3 registry_path only";
    proof {
        reveal_strlit("0000000000000000000000000000000000000000000000000000000000000000");
        reveal_strlit("1111111111111111111111111111111111111111111111111111111111111111");
        assert(vstd::string::is_ascii(zero) && vstd::string::is_ascii(one));
        vstd::string::is_ascii_spec_bytes(zero);
        vstd::string::is_ascii_spec_bytes(one);
        assert(zero.spec_bytes().len() == 64 && one.spec_bytes().len() == 64);
        assert(policy::identifiers::digest_format(zero.spec_bytes()));
        assert(policy::identifiers::digest_format(one.spec_bytes()));
        assert(zero@[0] != one@[0]);
        assert(hashes.seen() =~= Set::empty() && paths.seen() =~= Set::empty());
    }
    expect_row_error(policy::admit_identity_row(policy::SelectorRow {
        fields: None, digest: None, path_present: false, path: None, registry_path_present: false,
    }, true, &mut hashes, &mut paths), "Git selector configuration object required");
    expect_row_error(policy::admit_identity_row(policy::SelectorRow {
        fields: Some(1), digest: None, path_present: false, path: None, registry_path_present: false,
    }, true, &mut hashes, &mut paths), "Git selector configuration digest required");
    expect_row_error(policy::admit_identity_row(policy::SelectorRow {
        fields: Some(3), digest: Some(zero), path_present: true, path: Some(target), registry_path_present: false,
    }, true, &mut hashes, &mut paths), malformed);
    let accepted = policy::admit_identity_row(policy::SelectorRow {
        fields: Some(3), digest: Some(zero), path_present: true, path: Some(target), registry_path_present: true,
    }, true, &mut hashes, &mut paths);
    match accepted {
        Ok(path) => { expect_true(policy::matches_expected(path, Some(target))); },
        Err(_) => { panic!(); },
    }
    expect_row_error(policy::admit_identity_row(policy::SelectorRow {
        fields: Some(3), digest: Some(zero), path_present: true, path: None, registry_path_present: true,
    }, true, &mut hashes, &mut paths), malformed);
    expect_row_error(policy::admit_identity_row(policy::SelectorRow {
        fields: Some(2), digest: Some(one), path_present: true, path: Some(target), registry_path_present: false,
    }, false, &mut hashes, &mut paths), "Git selector configuration paths must be unique");
}

pub struct ObservedRows<'a>(pub Vec<policy::SelectorRow<'a>>);

impl<'a> policy::SelectorRows<'a> for ObservedRows<'a> {
    open spec fn rows(&self) -> Seq<policy::SelectorRow<'a>> { self.0@ }

    fn len(&self) -> usize { self.0.len() }

    fn row(&self, index: usize) -> policy::SelectorRow<'a> {
        let row = &self.0[index];
        policy::SelectorRow {
            fields: row.fields, digest: row.digest, path_present: row.path_present,
            path: row.path, registry_path_present: row.registry_path_present,
        }
    }
}

fn observed_row<'a>(digest: &'a str, path: &'a str) -> (row: policy::SelectorRow<'a>)
    ensures row.fields == Some(2usize), row.digest == Some(digest), row.path_present,
        row.path == Some(path), !row.registry_path_present,
        row.digest.unwrap().spec_bytes() == digest.spec_bytes(),
{
    policy::SelectorRow {
        fields: Some(2), digest: Some(digest), path_present: true,
        path: Some(path), registry_path_present: false,
    }
}

proof fn valid_walk_digest_literals()
    ensures policy::identifiers::digest_format(
        "0000000000000000000000000000000000000000000000000000000000000000".spec_bytes()),
        policy::identifiers::digest_format(
        "1111111111111111111111111111111111111111111111111111111111111111".spec_bytes()),
        "0000000000000000000000000000000000000000000000000000000000000000"@
            != "1111111111111111111111111111111111111111111111111111111111111111"@,
{
    let zero = "0000000000000000000000000000000000000000000000000000000000000000";
    let one = "1111111111111111111111111111111111111111111111111111111111111111";
    reveal_strlit("0000000000000000000000000000000000000000000000000000000000000000");
    reveal_strlit("1111111111111111111111111111111111111111111111111111111111111111");
    assert(vstd::string::is_ascii(zero) && vstd::string::is_ascii(one));
    vstd::string::is_ascii_spec_bytes(zero);
    vstd::string::is_ascii_spec_bytes(one);
    assert(zero.spec_bytes().len() == 64 && one.spec_bytes().len() == 64);
    assert(policy::identifiers::digest_format(zero.spec_bytes()));
    assert(policy::identifiers::digest_format(one.spec_bytes()));
    assert(zero@[0] != one@[0]);
}

fn expect_walk_error(
    rows: &ObservedRows, v3: bool, index: usize, expected: &str,
    result: Result<Vec<&str>, &str>,
)
    requires policy::identity_failure_at(rows.0@, v3, index as int, expected@),
        match result {
            Ok(_) => policy::identity_prefix_valid(rows.0@, v3, rows.0@.len() as int),
            Err(error) => exists|i: int| policy::identity_failure_at(rows.0@, v3, i, error@),
        },
{
    match result {
        Ok(_) => { panic!(); },
        Err(error) => {
            proof {
                let first = choose|i: int| policy::identity_failure_at(rows.0@, v3, i, error@);
                if first < index {
                    assert(policy::identity_row_error(&rows.0@[first], v3,
                        policy::identity_prefix_hashes(rows.0@, first),
                        policy::identity_prefix_paths(rows.0@, first)).is_none());
                } else if first > index {
                    assert(policy::identity_row_error(&rows.0@[index as int], v3,
                        policy::identity_prefix_hashes(rows.0@, index as int),
                        policy::identity_prefix_paths(rows.0@, index as int)).is_none());
                }
                assert(first == index);
                assert(error@ == expected@);
            }
            expect_true(policy::matches_expected(error, Some(expected)));
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn identity_row_walk_preserves_empty_and_ordered_original_paths() {
    use policy::SelectorSeen;
    let mut hashes = SeenRows(Vec::new());
    let mut paths = SeenRows(Vec::new());
    let empty = ObservedRows(Vec::new());
    proof { assert(hashes.seen() =~= Set::empty() && paths.seen() =~= Set::empty()); }
    match policy::collect_identity_paths(&empty, false, &mut hashes, &mut paths) {
        Ok(collected) => { expect_true(collected.len() == 0); },
        Err(_) => { panic!(); },
    }
    let mut rows = ObservedRows(Vec::new());
    rows.0.push(observed_row(
        "0000000000000000000000000000000000000000000000000000000000000000", ""));
    rows.0.push(observed_row(
        "1111111111111111111111111111111111111111111111111111111111111111", "λ\0"));
    proof {
        valid_walk_digest_literals();
        reveal_strlit(""); reveal_strlit("λ\0");
        assert(""@.len() != "λ\0"@.len());
        assert(policy::identifiers::digest_format(rows.0@[0].digest.unwrap().spec_bytes()));
        assert(policy::identifiers::digest_format(rows.0@[1].digest.unwrap().spec_bytes()));
        assert(policy::identity_prefix_hashes(rows.0@, 1) =~=
            Set::empty().insert(rows.0@[0].digest.unwrap()@));
        assert(policy::identity_prefix_paths(rows.0@, 1) =~= Set::empty().insert(""@));
        assert(policy::identity_row_error(&rows.0@[0], false,
            policy::identity_prefix_hashes(rows.0@, 0), policy::identity_prefix_paths(rows.0@, 0)) == None::<Seq<char>>);
        assert(policy::identity_row_error(&rows.0@[1], false,
            policy::identity_prefix_hashes(rows.0@, 1), policy::identity_prefix_paths(rows.0@, 1)) == None::<Seq<char>>);
        assert(policy::identity_prefix_valid(rows.0@, false, 2));
    }
    match policy::collect_identity_paths(&rows, false, &mut hashes, &mut paths) {
        Ok(collected) => {
            expect_true(collected.len() == 2);
            expect_true(policy::matches_expected(collected[0], Some("")));
            expect_true(policy::matches_expected(collected[1], Some("λ\0")));
        },
        Err(_) => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn identity_row_walk_stops_at_the_first_malformed_row() {
    use policy::SelectorSeen;
    let mut hashes = SeenRows(Vec::new());
    let mut paths = SeenRows(Vec::new());
    let mut rows = ObservedRows(Vec::new());
    rows.0.push(policy::SelectorRow {
        fields: None, digest: None, path_present: false, path: None, registry_path_present: false,
    });
    rows.0.push(observed_row(
        "0000000000000000000000000000000000000000000000000000000000000000", "later"));
    proof { assert(hashes.seen() =~= Set::empty() && paths.seen() =~= Set::empty()); }
    let result = policy::collect_identity_paths(&rows, false, &mut hashes, &mut paths);
    expect_walk_error(&rows, false, 0, "Git selector configuration object required", result);
    proof { assert(hashes.seen() =~= Set::empty() && paths.seen() =~= Set::empty()); }
    expect_true(hashes.record(
        "0000000000000000000000000000000000000000000000000000000000000000"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn identity_row_walk_rejects_nonadjacent_digest_reuse_before_collecting_its_path() {
    use policy::SelectorSeen;
    let mut hashes = SeenRows(Vec::new());
    let mut paths = SeenRows(Vec::new());
    let mut rows = ObservedRows(Vec::new());
    let zero = "0000000000000000000000000000000000000000000000000000000000000000";
    let one = "1111111111111111111111111111111111111111111111111111111111111111";
    rows.0.push(observed_row(zero, "a"));
    rows.0.push(observed_row(one, "b"));
    rows.0.push(observed_row(zero, "later"));
    let expected = "Git selector configuration requires a unique digest/path and the declared v3 registry_path only";
    proof {
        valid_walk_digest_literals();
        reveal_strlit("a"); reveal_strlit("b"); reveal_strlit("later");
        assert("a"@[0] != "b"@[0]);
        assert(hashes.seen() =~= Set::empty() && paths.seen() =~= Set::empty());
        assert(policy::identity_row_error(&rows.0@[0], false,
            policy::identity_prefix_hashes(rows.0@, 0), policy::identity_prefix_paths(rows.0@, 0)) == None::<Seq<char>>);
        assert(policy::identity_row_error(&rows.0@[1], false,
            policy::identity_prefix_hashes(rows.0@, 1), policy::identity_prefix_paths(rows.0@, 1)) == None::<Seq<char>>);
        assert(policy::identity_prefix_valid(rows.0@, false, 2));
        assert(policy::identity_failure_at(rows.0@, false, 2, expected@));
    }
    let result = policy::collect_identity_paths(&rows, false, &mut hashes, &mut paths);
    expect_walk_error(&rows, false, 2, expected, result);
    expect_true(paths.record("later"));
    expect_true(!hashes.record(zero));
    expect_true(!hashes.record(one));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn identity_row_walk_keeps_the_second_digest_when_its_path_is_a_duplicate() {
    use policy::SelectorSeen;
    let mut hashes = SeenRows(Vec::new());
    let mut paths = SeenRows(Vec::new());
    let mut rows = ObservedRows(Vec::new());
    let zero = "0000000000000000000000000000000000000000000000000000000000000000";
    let one = "1111111111111111111111111111111111111111111111111111111111111111";
    rows.0.push(observed_row(zero, "a"));
    rows.0.push(observed_row(one, "a"));
    let expected = "Git selector configuration paths must be unique";
    proof {
        valid_walk_digest_literals();
        assert(hashes.seen() =~= Set::empty() && paths.seen() =~= Set::empty());
        assert(policy::identity_row_error(&rows.0@[0], false,
            policy::identity_prefix_hashes(rows.0@, 0), policy::identity_prefix_paths(rows.0@, 0)) == None::<Seq<char>>);
        assert(policy::identity_prefix_valid(rows.0@, false, 1));
        assert(policy::identity_failure_at(rows.0@, false, 1, expected@));
    }
    let result = policy::collect_identity_paths(&rows, false, &mut hashes, &mut paths);
    expect_walk_error(&rows, false, 1, expected, result);
    expect_true(!hashes.record(zero));
    expect_true(!hashes.record(one));
    expect_true(!paths.record("a"));
}

fn expect_target_schema(version: Option<u64>, schema_present: bool, expected: bool)
    requires expected == (!schema_present && (version == Some(3u64) || version == Some(4u64))),
{
    match policy::admit_git_selector_target(version, schema_present) {
        Ok(()) => { expect_true(expected); },
        Err(message) => {
            expect_true(!expected);
            expect_true(policy::matches_expected(message, Some(
                "Git config selector requires a direct full-v3 policy or v4 policy, never another selector")));
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn git_selector_target_accepts_only_direct_v3_or_v4_without_a_schema_key() {
    expect_target_schema(None, false, false);
    expect_target_schema(Some(2), false, false);
    expect_target_schema(Some(3), false, true);
    expect_target_schema(Some(4), false, true);
    expect_target_schema(Some(5), false, false);
    expect_target_schema(Some(u64::MAX), false, false);
    expect_target_schema(Some(3), true, false);
    expect_target_schema(Some(4), true, false);
}

fn expect_absent_observation(result: Result<Option<(&str, u64)>, (&str, &str, &str)>)
    requires match result { Ok(value) => value.is_none(), Err(_) => false },
{
    match result {
        Ok(None) => (),
        _ => { panic!(); },
    }
}

fn expect_present_observation(
    digest: &str, length: u64, result: Result<Option<(&str, u64)>, (&str, &str, &str)>,
)
    requires match result {
        Ok(Some((actual_digest, actual_length))) => actual_digest@ == digest@ && actual_length == length,
        _ => false,
    },
{
    match result {
        Ok(Some((actual_digest, actual_length))) => {
            expect_true(policy::matches_expected(actual_digest, Some(digest)));
            expect_true(actual_length == length);
        },
        _ => { panic!(); },
    }
}

fn expect_observation_mismatch(id: &str, result: Result<Option<(&str, u64)>, (&str, &str, &str)>)
    requires match result {
        Err((prefix, actual_id, suffix)) => prefix@ == "Git input "@ && actual_id@ == id@
            && suffix@ == " digest/presence mismatch"@,
        _ => false,
    },
{
    match result {
        Err((prefix, actual_id, suffix)) => {
            expect_true(policy::matches_expected(prefix, Some("Git input ")));
            expect_true(policy::matches_expected(actual_id, Some(id)));
            expect_true(policy::matches_expected(suffix, Some(" digest/presence mismatch")));
        },
        _ => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn guard_absence_requires_exact_presence_and_does_not_check_a_digest() {
    proof {
        reveal_strlit("absent"); reveal_strlit("present");
        reveal_strlit(""); reveal_strlit("absent\0");
        assert("absent"@.len() != "present"@.len());
        assert("absent"@.len() != ""@.len());
        assert("absent"@.len() != "absent\0"@.len());
    }
    expect_absent_observation(policy::admit_git_guard_observation("id", Some("absent"), None, None));
    expect_absent_observation(policy::admit_git_guard_observation("id", Some("absent"), Some("arbitrary"), None));
    expect_observation_mismatch("id", policy::admit_git_guard_observation("id", None, None, None));
    expect_observation_mismatch("id", policy::admit_git_guard_observation("id", Some("present"), Some("hash"), None));
    expect_observation_mismatch("id", policy::admit_git_guard_observation("id", Some(""), None, None));
    expect_observation_mismatch("id", policy::admit_git_guard_observation("id", Some("absent\0"), None, None));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn guard_present_observation_preserves_the_original_digest_and_boundary_lengths() {
    expect_present_observation("hash", 0, policy::admit_git_guard_observation(
        "id", Some("present"), Some("hash"), Some(("hash", 0))));
    expect_present_observation("hash", u64::MAX, policy::admit_git_guard_observation(
        "id", Some("present"), Some("hash"), Some(("hash", u64::MAX))));
    expect_present_observation("", 1, policy::admit_git_guard_observation(
        "id", Some("present"), Some(""), Some(("", 1))));
    expect_present_observation("摘要\0λ", 7, policy::admit_git_guard_observation(
        "id", Some("present"), Some("摘要\0λ"), Some(("摘要\0λ", 7))));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn guard_digest_comparison_keeps_case_unicode_nul_and_empty_text() {
    proof {
        reveal_strlit("hash"); reveal_strlit("HASH"); reveal_strlit("");
        reveal_strlit("hash\0"); reveal_strlit("摘要"); reveal_strlit("摘要λ");
        assert("hash"@[0] != "HASH"@[0]);
        assert("hash"@.len() != ""@.len());
        assert("hash"@.len() != "hash\0"@.len());
        assert("摘要"@.len() != "摘要λ"@.len());
    }
    expect_observation_mismatch("id", policy::admit_git_guard_observation(
        "id", Some("present"), None, Some(("hash", 0))));
    expect_observation_mismatch("id", policy::admit_git_guard_observation(
        "id", Some("present"), Some("HASH"), Some(("hash", 0))));
    expect_observation_mismatch("id", policy::admit_git_guard_observation(
        "id", Some("present"), Some(""), Some(("hash", 0))));
    expect_observation_mismatch("id", policy::admit_git_guard_observation(
        "id", Some("present"), Some("hash\0"), Some(("hash", 0))));
    expect_observation_mismatch("id", policy::admit_git_guard_observation(
        "id", Some("present"), Some("摘要λ"), Some(("摘要", 0))));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn guard_presence_mismatch_keeps_the_complete_original_id() {
    proof {
        reveal_strlit("absent"); reveal_strlit("present");
        reveal_strlit("present\0"); reveal_strlit("PRESENT");
        assert("absent"@.len() != "present"@.len());
        assert("present"@.len() != "present\0"@.len());
        assert("present"@[0] != "PRESENT"@[0]);
    }
    let id = "输入 λ\0 ";
    expect_observation_mismatch(id, policy::admit_git_guard_observation(
        id, Some("absent"), Some("hash"), Some(("hash", 9))));
    expect_observation_mismatch(id, policy::admit_git_guard_observation(
        id, None, Some("hash"), Some(("hash", 9))));
    expect_observation_mismatch(id, policy::admit_git_guard_observation(
        id, Some("present\0"), Some("hash"), Some(("hash", 9))));
    expect_observation_mismatch("", policy::admit_git_guard_observation(
        "", Some("PRESENT"), Some("hash"), Some(("hash", u64::MAX))));
}

}
