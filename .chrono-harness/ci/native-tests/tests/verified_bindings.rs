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

fn executable(path: &str, hash: &str, version: &str) -> (out: Executable)
    ensures out.path@ == path@, out.sha256@ == hash@, out.version@ == version@,
{
    Executable { path: path.to_string(), sha256: hash.to_string(), version: version.to_string() }
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
    assert(accepted);
    assert(!short_hash);
    assert(!long_hash);
    assert(!upper_hash);
    assert(!nul_hash);
}

}
