//! 当前 Python 工具选择合同的独立边界；原生适配器仍须验证真实进程和文件。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../native/src/verified_bindings.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(not(verus_keep_ghost))]
use chrono_host_bootstrap::verified_bindings as policy;
use policy::Executable;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

fn source_observation(commit: &str, tree: &str, dirty: bool, error: Option<String>) -> (out: policy::SourceObservation)
    ensures
        out.commit is Some, out.commit.unwrap()@ == commit@,
        out.tree is Some, out.tree.unwrap()@ == tree@,
        out.dirty == Some(dirty), out.error == error,
{
    policy::SourceObservation {
        commit: Some(commit.to_string()), tree: Some(tree.to_string()), dirty: Some(dirty), error,
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn source_identity_only_exposes_commit_and_tree_for_unchanged_clean_observations() {
    proof { reveal_strlit("c"); reveal_strlit("cc"); reveal_strlit("t"); reveal_strlit("tt"); }
    let clean = policy::source_identity(source_observation("c", "t", false, None), source_observation("c", "t", false, None));
    let dirty = policy::source_identity(source_observation("c", "t", true, None), source_observation("c", "t", true, None));
    let commit = policy::source_identity(source_observation("c", "t", false, None), source_observation("cc", "t", false, None));
    let tree = policy::source_identity(source_observation("c", "t", false, None), source_observation("c", "tt", false, None));
    let status = policy::source_identity(source_observation("c", "t", false, None), source_observation("c", "t", true, None));
    let failed_before = policy::source_identity(source_observation("c", "t", true, Some("failed".to_string())), source_observation("cc", "tt", false, None));
    let failed_after = policy::source_identity(source_observation("c", "t", false, None), source_observation("cc", "tt", true, Some("failed".to_string())));
    assert(clean.state is Clean && clean.commit is Some && clean.tree is Some);
    assert(clean.commit.unwrap()@ == "c"@ && clean.tree.unwrap()@ == "t"@);
    assert(dirty.state is Dirty && dirty.commit is None && dirty.tree is None);
    proof {
        assert(commit.before.commit.unwrap()@.len() == 1);
        assert(commit.after.commit.unwrap()@.len() == 2);
        assert(!policy::source_observations_same(&commit.before, &commit.after));
        assert(tree.before.tree.unwrap()@.len() == 1);
        assert(tree.after.tree.unwrap()@.len() == 2);
        assert(!policy::source_observations_same(&tree.before, &tree.after));
    }
    assert(commit.state is Changed && commit.commit is None && commit.tree is None);
    assert(tree.state is Changed && tree.commit is None && tree.tree is None);
    assert(status.state is Changed && status.commit is None && status.tree is None);
    assert(failed_before.state is Unavailable && failed_before.commit is None && failed_before.tree is None);
    assert(failed_after.state is Unavailable && failed_after.commit is None && failed_after.tree is None);
    assert(failed_before.before.error is Some && failed_after.after.error is Some);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn exit_status_keeps_original_codes_and_only_negates_positive_signals() {
    let success = policy::process_exit(Some(0), None);
    let failure = policy::process_exit(Some(17), None);
    let precedence = policy::process_exit(Some(17), Some(9));
    let signal = policy::process_exit(None, Some(9));
    let missing = policy::process_exit(None, None);
    let zero_signal = policy::process_exit(None, Some(0));
    let negative_signal = policy::process_exit(None, Some(-1));
    let minimum_code = policy::process_exit(Some(-2147483648i32), Some(9));
    let maximum_code = policy::process_exit(Some(2147483647i32), None);
    let maximum_signal = policy::process_exit(None, Some(2147483647i32));
    let minimum_signal = policy::process_exit(None, Some(-2147483648i32));
    assert(success == Some(0i32) && failure == Some(17i32) && precedence == Some(17i32));
    assert(signal == Some(-9i32) && missing is None);
    assert(zero_signal is None && negative_signal is None && minimum_signal is None);
    assert(minimum_code == Some(-2147483648i32) && maximum_code == Some(2147483647i32));
    assert(maximum_signal == Some(-2147483647i32));
}

fn executable(path: &str, hash: &str, version: &str) -> (out: Executable)
    ensures out.path@ == path@, out.sha256@ == hash@, out.version@ == version@,
{
    Executable { path: path.to_string(), sha256: hash.to_string(), version: version.to_string() }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn environment_names_reject_only_the_reserved_prefix_equals_and_nul() {
    let empty: [u8; 0] = [];
    let normal = policy::environment_key(&[65u8]);
    let missing = policy::environment_key(&empty);
    let equals = policy::environment_key(&[65u8, 61]);
    let nul = policy::environment_key(&[65u8, 0]);
    let reserved = policy::environment_key(&[67u8, 72, 82, 79, 78, 79, 95]);
    let shorter = policy::environment_key(&[67u8, 72, 82, 79, 78, 79]);
    let value_empty = policy::environment_value(&empty);
    let value_lines = policy::environment_value(&[10u8, 13, 61]);
    let value_nul = policy::environment_value(&[10u8, 0]);
    assert(normal && !missing && !equals && !nul && !reserved && shorter);
    assert(value_empty && value_lines && !value_nul);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn host_paths_require_the_declared_prefix_and_exact_normal_components() {
    let prefix = [97u8, 47];
    let good_bytes = [97u8, 47, 98];
    let hidden_bytes = [97u8, 47, 46, 98];
    let repeated_bytes = [97u8, 47, 47, 98];
    let current_bytes = [97u8, 47, 46, 47, 98];
    let parent_bytes = [97u8, 47, 46, 46];
    let trailing_bytes = [97u8, 47, 98, 47];
    let nul_bytes = [97u8, 47, 0];
    proof {
        assert forall|i: int| 0 <= i < good_bytes@.len() implies !policy::bad_path_byte(good_bytes@, i) by {
            assert(i == 0 || i == 1 || i == 2);
        }
        assert forall|i: int| 0 <= i < hidden_bytes@.len() implies !policy::bad_path_byte(hidden_bytes@, i) by {
            assert(i == 0 || i == 1 || i == 2 || i == 3);
        }
        assert(policy::bad_path_byte(repeated_bytes@, 2));
        assert(policy::bad_path_byte(current_bytes@, 2));
        assert(policy::bad_path_byte(parent_bytes@, 2));
        assert(policy::bad_path_byte(trailing_bytes@, 3));
        assert(policy::bad_path_byte(nul_bytes@, 2));
    }
    let good = policy::host_path(&good_bytes, &prefix);
    let hidden = policy::host_path(&hidden_bytes, &prefix);
    let repeated = policy::host_path(&repeated_bytes, &prefix);
    let current = policy::host_path(&current_bytes, &prefix);
    let parent = policy::host_path(&parent_bytes, &prefix);
    let trailing = policy::host_path(&trailing_bytes, &prefix);
    let nul = policy::host_path(&nul_bytes, &prefix);
    let wrong_prefix = policy::host_path(&[98u8, 47, 97], &prefix);
    assert(good && hidden && !repeated && !current && !parent && !trailing && !nul && !wrong_prefix);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn private_homes_compare_full_components_and_default_profile_is_exact() {
    proof {
        reveal_strlit("default"); reveal_strlit("minimal");
        reveal_strlit("HOME"); reveal_strlit("TMPDIR"); reveal_strlit("CARGO_HOME"); reveal_strlit("RUSTUP_HOME");
        assert("default"@[0] != "minimal"@[0]);
        assert("RUSTUP_HOME"@.len() != "HOME"@.len());
        assert("RUSTUP_HOME"@.len() != "TMPDIR"@.len());
        assert("RUSTUP_HOME"@.len() != "CARGO_HOME"@.len());
    }
    let separate = policy::disjoint_homes(&[97u8], &[98u8]);
    let neighbor = policy::disjoint_homes(&[97u8], &[97u8, 98]);
    let same = policy::disjoint_homes(&[97u8], &[97u8]);
    let child = policy::disjoint_homes(&[97u8], &[97u8, 47, 98]);
    let parent = policy::disjoint_homes(&[97u8, 47, 98], &[97u8]);
    let profile = policy::default_profile("default");
    let minimal = policy::default_profile("minimal");
    let home = policy::host_variable("HOME");
    let cargo = policy::host_variable("CARGO_HOME");
    let temp = policy::host_variable("TMPDIR");
    let rustup = policy::host_variable("RUSTUP_HOME");
    assert(separate && neighbor && !same && !child && !parent);
    assert(profile && !minimal && home && cargo && temp && !rustup);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn private_probe_failures_refuse_and_only_ordinary_probes_choose_repairs() {
    let private_cargo = policy::probe_action(true, false, true);
    let private_format = policy::probe_action(true, false, false);
    let ordinary_cargo = policy::probe_action(false, false, true);
    let ordinary_format = policy::probe_action(false, false, false);
    let ready_pc = policy::probe_action(true, true, true);
    let ready_pf = policy::probe_action(true, true, false);
    let ready_oc = policy::probe_action(false, true, true);
    let ready_of = policy::probe_action(false, true, false);
    assert(private_cargo is Refuse && private_format is Refuse);
    assert(ordinary_cargo is InstallMinimal && ordinary_format is AddFormat);
    assert(ready_pc is Ready && ready_pf is Ready && ready_oc is Ready && ready_of is Ready);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn native_binding_requires_both_fields_and_an_explicit_platform() {
    proof {
        reveal_strlit("Darwin"); reveal_strlit("Linux");
        reveal_strlit("arm64"); reveal_strlit("x86_64");
        assert("Darwin"@.len() != "Linux"@.len());
        assert("arm64"@.len() != "x86_64"@.len());
    }
    let native = policy::NativePlatform { system: "Darwin".to_string(), machine: "arm64".to_string() };
    let system = policy::NativePlatform { system: "Linux".to_string(), machine: "arm64".to_string() };
    let machine = policy::NativePlatform { system: "Darwin".to_string(), machine: "x86_64".to_string() };
    let exact = policy::platform_matches(Some(&native), &native);
    let absent = policy::platform_matches(None, &native);
    let wrong_system = policy::platform_matches(Some(&system), &native);
    let wrong_machine = policy::platform_matches(Some(&machine), &native);
    assert(exact && !absent && !wrong_system && !wrong_machine);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_identity_binds_schema_and_toolchain_independently() {
    proof {
        reveal_strlit("chrono-bootstrap-toolchains/v1");
        reveal_strlit("chrono-bootstrap-toolchains/v2");
        reveal_strlit("1.95.0"); reveal_strlit("1.95.0x");
        assert("chrono-bootstrap-toolchains/v1"@[29] != "chrono-bootstrap-toolchains/v2"@[29]);
        assert("1.95.0"@.len() != "1.95.0x"@.len());
    }
    let exact = policy::registry_matches("chrono-bootstrap-toolchains/v1", "1.95.0", "1.95.0");
    let schema = policy::registry_matches("chrono-bootstrap-toolchains/v2", "1.95.0", "1.95.0");
    let version = policy::registry_matches("chrono-bootstrap-toolchains/v1", "1.95.0x", "1.95.0");
    assert(exact && !schema && !version);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn version_prefix_requires_a_delimiter_and_only_cargo_allows_an_exact_version() {
    let prefix = [97u8];
    let exact = [97u8];
    let spaced = [97u8, 32, 98];
    let glued = [97u8, 98];
    let wrong = [98u8, 32];
    let cargo = policy::version_matches(&exact, &prefix, true);
    let rustc = policy::version_matches(&exact, &prefix, false);
    let with_suffix = policy::version_matches(&spaced, &prefix, false);
    let without_delimiter = policy::version_matches(&glued, &prefix, true);
    let wrong_prefix = policy::version_matches(&wrong, &prefix, true);
    let short = policy::version_matches(&prefix, &spaced, true);
    assert(cargo && !rustc && with_suffix && !without_delimiter && !wrong_prefix && !short);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_rejects_direct_or_native_overrides_even_when_bindings_are_optional() {
    let normal = policy::declaration_source(true, false, false, true, false);
    let direct = policy::declaration_source(true, true, false, false, true);
    let native = policy::declaration_source(true, false, true, false, true);
    let both = policy::declaration_source(true, true, true, true, true);
    assert(normal is Registry);
    assert(direct is Reject && native is Reject && both is Reject);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn required_direct_bindings_need_a_matching_platform_and_declaration() {
    let accepted = policy::declaration_source(false, true, true, true, true);
    let wrong = policy::declaration_source(false, true, true, true, false);
    let missing = policy::declaration_source(false, false, true, true, true);
    let missing_platform = policy::declaration_source(false, true, false, true, true);
    let optional = policy::declaration_source(false, true, false, false, false);
    let unbound = policy::declaration_source(false, false, false, false, false);
    assert(accepted is Direct && optional is Direct);
    assert(wrong is Reject && missing is Reject && missing_platform is Reject);
    assert(unbound is Unbound);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn relocation_keeps_digest_and_version_and_only_ignores_the_old_path() {
    proof {
        reveal_strlit("old"); reveal_strlit("new"); reveal_strlit("hash");
        reveal_strlit("other"); reveal_strlit("version");
        assert("old"@[0] != "new"@[0]);
        assert("hash"@.len() != "other"@.len());
        assert("version"@.len() != "other"@.len());
    }
    let expected = executable("old", "hash", "version");
    let relocated = executable("new", "hash", "version");
    let accepted = policy::selected_matches(&relocated, &expected, true);
    let exact = policy::selected_matches(&relocated, &expected, false);
    let hash = executable("new", "other", "version");
    let version = executable("new", "hash", "other");
    let wrong_hash = policy::selected_matches(&hash, &expected, true);
    let wrong_version = policy::selected_matches(&version, &expected, true);
    assert(accepted && !exact && !wrong_hash && !wrong_version);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn only_the_repeated_relocatable_declared_path_is_retired() {
    proof {
        reveal_strlit("old"); reveal_strlit("current");
        assert("old"@.len() != "current"@.len());
    }
    let retired = policy::configured_action(Some("old"), Some("old"), true, false);
    let exact = policy::configured_action(Some("old"), Some("old"), false, false);
    let current = policy::configured_action(Some("current"), Some("old"), true, false);
    let missing = policy::configured_action(None, Some("old"), true, false);
    let registry = policy::configured_action(None, None, true, true);
    let configured_registry = policy::configured_action(Some("current"), None, true, true);
    assert(retired is Retired);
    assert(exact is Observe && current is Observe && configured_registry is Observe);
    assert(missing is Missing && registry is NotDeclared);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn inherited_compiler_and_both_wrapper_slots_are_independent_constraints() {
    proof {
        reveal_strlit("selected"); reveal_strlit("other"); reveal_strlit("");
        assert("selected"@.len() != "other"@.len());
        assert("other"@.len() != 0);
    }
    let absent = policy::compiler_selection("selected", None, None, None);
    let empty = policy::compiler_selection("selected", Some(""), Some(""), Some(""));
    let exact = policy::compiler_selection("selected", Some("selected"), None, None);
    let wrong = policy::compiler_selection("selected", Some("other"), None, None);
    let wrapper = policy::compiler_selection("selected", None, Some("other"), None);
    let workspace = policy::compiler_selection("selected", None, None, Some("other"));
    assert(absent && empty && exact && !wrong && !wrapper && !workspace);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn sha256_allows_zero_but_requires_exact_lowercase_hex_length() {
    broadcast use vstd::array::group_array_axioms;
    let zeros = [48u8; 64];
    let short = [48u8; 63];
    let long = [48u8; 65];
    let mut upper = [48u8; 64];
    let mut nul = [48u8; 64];
    upper[63] = 65;
    nul[0] = 0;
    proof {
        assert(zeros@.len() == 64);
        assert forall|i: int| 0 <= i < zeros@.len()
            implies policy::hash_byte(zeros@[i]) by {
            assert(zeros@[i] == 48);
        }
        assert(short@.len() == 63);
        assert(long@.len() == 65);
        assert(upper@[63] == 65);
        assert(nul@[0] == 0);
    }
    let accepted = policy::valid_sha256(&zeros);
    let short_hash = policy::valid_sha256(&short);
    let long_hash = policy::valid_sha256(&long);
    let upper_hash = policy::valid_sha256(&upper);
    let nul_hash = policy::valid_sha256(&nul);
    let direct = policy::binding_valid(&zeros, false, true);
    let missing = policy::binding_valid(&zeros, false, false);
    let registry = policy::binding_valid(&zeros, true, false);
    let invalid_registry = policy::binding_valid(&upper, true, true);
    assert(accepted);
    assert(!short_hash);
    assert(!long_hash);
    assert(!upper_hash);
    assert(!nul_hash);
    assert(direct && !missing && registry && !invalid_registry);
}

}
