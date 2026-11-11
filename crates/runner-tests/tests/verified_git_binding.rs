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

}
