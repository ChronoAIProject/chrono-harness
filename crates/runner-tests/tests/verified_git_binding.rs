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
