//! 登记启用与端点快照准备的独立边界预期，直接调用生产判断。
#![allow(unexpected_cfgs)]
#[path = "../../judge-registration/src/verified_readiness.rs"]
mod readiness;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn only_exact_active_registry_is_ready() {
    proof {
        reveal_strlit("active"); reveal_strlit("proposed"); reveal_strlit("");
        reveal_strlit("Active"); reveal_strlit("active\0"); reveal_strlit("启用");
        assert("active"@.len() != "proposed"@.len());
        assert("active"@.len() != ""@.len());
        assert("active"@[0] != "Active"@[0]);
        assert("active"@.len() != "active\0"@.len());
        assert("active"@.len() != "启用"@.len());
    }
    let ready = readiness::registry_active(Some("active"));
    assert(ready);
    let missing = readiness::registry_active(None);
    assert(!missing);
    let proposed = readiness::registry_active(Some("proposed"));
    assert(!proposed);
    let empty = readiness::registry_active(Some(""));
    assert(!empty);
    let case = readiness::registry_active(Some("Active"));
    assert(!case);
    let nul = readiness::registry_active(Some("active\0"));
    assert(!nul);
    let unicode = readiness::registry_active(Some("启用"));
    assert(!unicode);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn activation_requires_all_three_current_input_conditions() {
    proof {
        reveal_strlit("enabled"); reveal_strlit("not-implemented");
        reveal_strlit("declared-complete"); reveal_strlit("incomplete");
        assert("enabled"@.len() != "not-implemented"@.len());
        assert("declared-complete"@.len() != "incomplete"@.len());
    }
    let empty: Vec<u8> = Vec::new();
    let ready = readiness::input_ready(Some("enabled"), Some("declared-complete"), Some(&empty));
    assert(ready);
    let disabled = readiness::input_ready(Some("not-implemented"), Some("declared-complete"), Some(&empty));
    assert(!disabled);
    let incomplete = readiness::input_ready(Some("enabled"), Some("incomplete"), Some(&empty));
    assert(!incomplete);
    let unresolved = vec![0u8];
    let pending = readiness::input_ready(Some("enabled"), Some("declared-complete"), Some(&unresolved));
    assert(!pending);
    let multiple = vec![0u8, 255u8];
    let pending_multiple = readiness::input_ready(Some("enabled"), Some("declared-complete"), Some(&multiple));
    assert(!pending_multiple);
    let absent_enforcement = readiness::input_ready(None, Some("declared-complete"), Some(&empty));
    assert(!absent_enforcement);
    let absent_closure = readiness::input_ready(Some("enabled"), None, Some(&empty));
    assert(!absent_closure);
    let absent_unresolved = readiness::input_ready::<u8>(Some("enabled"), Some("declared-complete"), None);
    assert(!absent_unresolved);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn empty_collection_does_not_make_mismatched_text_ready() {
    proof {
        reveal_strlit("enabled"); reveal_strlit("Enabled"); reveal_strlit("enabled\0");
        reveal_strlit("declared-complete"); reveal_strlit("declared-complete\0");
        reveal_strlit(""); reveal_strlit("完整");
        assert("enabled"@[0] != "Enabled"@[0]);
        assert("enabled"@.len() != "enabled\0"@.len());
        assert("declared-complete"@.len() != "declared-complete\0"@.len());
        assert("declared-complete"@.len() != ""@.len());
        assert("declared-complete"@.len() != "完整"@.len());
    }
    let empty: Vec<&str> = Vec::new();
    let case = readiness::input_ready(Some("Enabled"), Some("declared-complete"), Some(&empty));
    assert(!case);
    let enforcement_nul = readiness::input_ready(Some("enabled\0"), Some("declared-complete"), Some(&empty));
    assert(!enforcement_nul);
    let closure_nul = readiness::input_ready(Some("enabled"), Some("declared-complete\0"), Some(&empty));
    assert(!closure_nul);
    let missing_text = readiness::input_ready(Some("enabled"), Some(""), Some(&empty));
    assert(!missing_text);
    let unicode = readiness::input_ready(Some("enabled"), Some("完整"), Some(&empty));
    assert(!unicode);
}

#[cfg(verus_keep_ghost)]
fn every_declared_snapshot_dependency_requires_its_exact_endpoint(
    schema_present: bool, files: usize, strict_environment: bool, inherited: usize,
    actual: &str, expected: &str,
)
    requires schema_present || files > 0 || (strict_environment && inherited > 0),
{
    let same = readiness::endpoint_snapshot_matches(
        schema_present, files, strict_environment, inherited, Some(expected), expected,
    );
    assert(same);
    let missing = readiness::endpoint_snapshot_matches(
        schema_present, files, strict_environment, inherited, None, expected,
    );
    assert(!missing);
    let observed = readiness::endpoint_snapshot_matches(
        schema_present, files, strict_environment, inherited, Some(actual), expected,
    );
    assert(observed == (actual@ == expected@));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn snapshot_schema_files_and_strict_inheritance_each_require_a_commit() {
    let schema = readiness::endpoint_snapshot_matches(true, 0, false, 0, None, "base");
    assert(!schema);
    let file = readiness::endpoint_snapshot_matches(false, 1, false, 0, None, "base");
    assert(!file);
    let environment = readiness::endpoint_snapshot_matches(false, 0, true, 1, None, "base");
    assert(!environment);
    let many_files = readiness::endpoint_snapshot_matches(false, usize::MAX, false, 0, None, "base");
    assert(!many_files);
    let many_variables = readiness::endpoint_snapshot_matches(false, 0, true, usize::MAX, None, "base");
    assert(!many_variables);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn legacy_without_snapshot_dependencies_keeps_its_existing_optional_binding() {
    let no_snapshot = readiness::endpoint_snapshot_matches(false, 0, false, 0, None, "base");
    assert(no_snapshot);
    let inherited_legacy = readiness::endpoint_snapshot_matches(false, 0, false, usize::MAX, None, "base");
    assert(inherited_legacy);
    let strict_empty = readiness::endpoint_snapshot_matches(false, 0, true, 0, None, "base");
    assert(strict_empty);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn snapshot_endpoint_text_is_exact_without_adding_identifier_restrictions() {
    proof {
        reveal_strlit("base"); reveal_strlit("base\0"); reveal_strlit("b");
        reveal_strlit("é"); reveal_strlit("e\u{301}");
        assert("base"@.len() != "base\0"@.len());
        assert("base"@.len() != "b"@.len());
        assert("é"@.len() != "e\u{301}"@.len());
    }
    let exact = readiness::endpoint_snapshot_matches(true, 0, false, 0, Some("base"), "base");
    assert(exact);
    let nul = readiness::endpoint_snapshot_matches(true, 0, false, 0, Some("base\0"), "base");
    assert(!nul);
    let prefix = readiness::endpoint_snapshot_matches(true, 0, false, 0, Some("b"), "base");
    assert(!prefix);
    let unicode = readiness::endpoint_snapshot_matches(true, 0, false, 0, Some("é"), "e\u{301}");
    assert(!unicode);
    let empty = readiness::endpoint_snapshot_matches(true, 0, false, 0, Some(""), "");
    assert(empty);
}

}
