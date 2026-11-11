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
use vstd::vpanic as panic;

verus! {

fn expect_true(value: bool)
    requires value,
{ if !value { panic!(); } }

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

}
