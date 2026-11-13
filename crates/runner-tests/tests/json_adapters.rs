//! 独立 JSON／文本边界预期，直接消费 runner 的真实适配器。
#![allow(unexpected_cfgs)]

use chrono_harness::{
    CapturedRegistry, JsonBindingReferences, JsonGuardReferences, JsonSelectorRows, JsonTargetRows,
    SeenSelectorText, json_value_specs as json_spec, verified_git_binding as policy,
};
use policy::{
    BindingReferenceRows, GitTargetRows, RegistryMembership, SelectorRegistryRows, SelectorRows,
    SelectorSeen,
};
use serde_json::Value;
use std::collections::BTreeMap;
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

spec fn observed_text(value: Option<&str>) -> Option<Seq<char>> {
    match value { None => None, Some(text) => Some(text@) }
}

fn expect_text(actual: Option<&str>, expected: Option<&str>)
    requires observed_text(actual) == observed_text(expected),
{
    match (actual, expected) {
        (None, None) => {},
        (Some(actual), Some(expected)) =>
            expect_true(policy::matches_expected(actual, Some(expected))),
        _ => { panic!(); },
    }
}

fn reference_observations(rows: &[Value], expected: &[Option<&str>])
    requires rows@.len() == expected@.len(),
        forall|i: int| 0 <= i < rows@.len() ==>
            observed_text(json_spec::field_text(&rows@[i], "id"@))
                == observed_text(expected@[i]),
{
    let binding = JsonBindingReferences(rows);
    let guard = JsonGuardReferences(rows);
    expect_true(binding.len() == expected.len());
    expect_true(guard.len() == expected.len());
    let mut i = 0;
    while i < expected.len()
        invariant i <= expected@.len(), rows@.len() == expected@.len(),
            binding.0 == rows, guard.0 == rows,
            forall|j: int| 0 <= j < rows@.len() ==>
                observed_text(json_spec::field_text(&rows@[j], "id"@))
                    == observed_text(expected@[j]),
        decreases expected.len() - i,
    {
        expect_text(binding.id(i), expected[i]);
        expect_text(guard.id(i), expected[i]);
        i += 1;
    }
}

fn selector_observations<'a>(rows: &'a [Value], expected: &[policy::SelectorRow<'_>],
    registry: &[Option<&str>])
    requires rows@.len() == expected@.len(), rows@.len() == registry@.len(),
        forall|i: int| 0 <= i < rows@.len() ==> {
            &&& json_spec::fields(&rows@[i]) == expected@[i].fields
            &&& observed_text(json_spec::field_text(&rows@[i], "sha256"@))
                == observed_text(expected@[i].digest)
            &&& json_spec::field(&rows@[i], "path"@).is_some() == expected@[i].path_present
            &&& observed_text(json_spec::field_text(&rows@[i], "path"@))
                == observed_text(expected@[i].path)
            &&& json_spec::field(&rows@[i], "registry_path"@).is_some()
                == expected@[i].registry_path_present
            &&& observed_text(json_spec::field_text(&rows@[i], "registry_path"@))
                == observed_text(registry@[i])
        },
{
    let adapter = JsonSelectorRows(rows);
    expect_true(SelectorRows::len(&adapter) == expected.len());
    expect_true(SelectorRegistryRows::len(&adapter) == registry.len());
    let mut i = 0;
    while i < expected.len()
        invariant i <= expected@.len(), rows@.len() == expected@.len(),
            rows@.len() == registry@.len(), adapter.0 == rows,
            forall|j: int| 0 <= j < rows@.len() ==> {
                &&& json_spec::fields(&rows@[j]) == expected@[j].fields
                &&& observed_text(json_spec::field_text(&rows@[j], "sha256"@))
                    == observed_text(expected@[j].digest)
                &&& json_spec::field(&rows@[j], "path"@).is_some() == expected@[j].path_present
                &&& observed_text(json_spec::field_text(&rows@[j], "path"@))
                    == observed_text(expected@[j].path)
                &&& json_spec::field(&rows@[j], "registry_path"@).is_some()
                    == expected@[j].registry_path_present
                &&& observed_text(json_spec::field_text(&rows@[j], "registry_path"@))
                    == observed_text(registry@[j])
            },
        decreases expected.len() - i,
    {
        let actual = adapter.row(i);
        expect_true(actual.fields == expected[i].fields);
        expect_text(actual.digest, expected[i].digest);
        expect_true(actual.path_present == expected[i].path_present);
        expect_text(actual.path, expected[i].path);
        expect_true(actual.registry_path_present == expected[i].registry_path_present);
        expect_text(adapter.registry_path(i), registry[i]);
        i += 1;
    }
}

fn target_observations(rows: &[Value], digests: &[Option<&str>], paths: &[Option<&str>])
    requires rows@.len() == digests@.len(), rows@.len() == paths@.len(),
        forall|i: int| 0 <= i < rows@.len() ==>
            observed_text(json_spec::field_text(&rows@[i], "sha256"@)) == observed_text(digests@[i])
            && observed_text(json_spec::field_text(&rows@[i], "path"@)) == observed_text(paths@[i]),
{
    let adapter = JsonTargetRows(rows);
    expect_true(adapter.len() == paths.len());
    let mut i = 0;
    while i < paths.len()
        invariant i <= paths@.len(), rows@.len() == paths@.len(),
            rows@.len() == digests@.len(), adapter.0 == rows,
            forall|j: int| 0 <= j < rows@.len() ==>
                observed_text(json_spec::field_text(&rows@[j], "sha256"@)) == observed_text(digests@[j])
                && observed_text(json_spec::field_text(&rows@[j], "path"@)) == observed_text(paths@[j]),
        decreases paths.len() - i,
    {
        expect_text(adapter.text(i, &policy::GitTargetField::Digest), digests[i]);
        expect_text(adapter.text(i, &policy::GitTargetField::Path), paths[i]);
        i += 1;
    }
}

fn captured_membership(values: &BTreeMap<String, Value>, path: &str, expected: bool)
    requires expected == (exists|stored: String|
        #[trigger] values@.contains_key(stored) && stored@ == path@),
{
    expect_true(CapturedRegistry(values).contains_path(path) == expected);
}

fn record_text<'a>(seen: &mut SeenSelectorText<'a>, text: &'a str, expected_fresh: bool)
    requires expected_fresh == !old(seen).seen().contains(text@),
    ensures seen.seen() == old(seen).seen().insert(text@),
{
    expect_true(seen.record(text) == expected_fresh);
}

} // verus!

// 这些仅负责提供外部 JSON／标准集合的实际值。期望和断言在上面的同源驱动中；
// 严格证明核驱动的输入合同，Rust 测试另核固定解析器产生的这些真实边界值。
#[test]
fn reference_rows_keep_absence_types_order_empty_and_complete_text() {
    let rows = vec![
        Value::Null,
        serde_json::json!({}),
        serde_json::json!({"id": null}),
        serde_json::json!({"id": 7}),
        serde_json::json!({"id": ""}),
        serde_json::json!({"id": "目录é\0\n尾部"}),
        serde_json::json!({"id": "目录é\0\n尾部"}),
    ];
    reference_observations(
        &rows,
        &[
            None,
            None,
            None,
            None,
            Some(""),
            Some("目录é\0\n尾部"),
            Some("目录é\0\n尾部"),
        ],
    );
    reference_observations(&[], &[]);
}

#[test]
fn selector_rows_distinguish_missing_null_wrong_type_and_full_fields() {
    let rows = vec![
        Value::Null,
        serde_json::json!({}),
        serde_json::json!({"sha256": null, "path": null, "registry_path": null}),
        serde_json::json!({"sha256": 7, "path": false, "registry_path": []}),
        serde_json::json!({"sha256": "摘要\0", "path": "目录é\n", "registry_path": "完整λ", "extra": true}),
    ];
    selector_observations(
        &rows,
        &[
            policy::SelectorRow {
                fields: None,
                digest: None,
                path_present: false,
                path: None,
                registry_path_present: false,
            },
            policy::SelectorRow {
                fields: Some(0),
                digest: None,
                path_present: false,
                path: None,
                registry_path_present: false,
            },
            policy::SelectorRow {
                fields: Some(3),
                digest: None,
                path_present: true,
                path: None,
                registry_path_present: true,
            },
            policy::SelectorRow {
                fields: Some(3),
                digest: None,
                path_present: true,
                path: None,
                registry_path_present: true,
            },
            policy::SelectorRow {
                fields: Some(4),
                digest: Some("摘要\0"),
                path_present: true,
                path: Some("目录é\n"),
                registry_path_present: true,
            },
        ],
        &[None, None, None, None, Some("完整λ")],
    );
    selector_observations(&[], &[], &[]);
    target_observations(
        &rows,
        &[None, None, None, None, Some("摘要\0")],
        &[None, None, None, None, Some("目录é\n")],
    );
    target_observations(&[], &[], &[]);
}

#[test]
fn captured_map_and_seen_set_compare_full_text_from_different_storage() {
    let path = String::from("目录é\0\n尾部");
    let same = String::from("目录é\0\n尾部");
    let prefix = String::from("目录é");
    let mut values = BTreeMap::new();
    captured_membership(&values, &path, false);
    values.insert(path.clone(), Value::Null);
    values.insert(String::new(), Value::Null);
    captured_membership(&values, &same, true);
    captured_membership(&values, "", true);
    captured_membership(&values, &prefix, false);
    captured_membership(&values, "目录é\0\n尾部", false);
    let mut seen = SeenSelectorText::default();
    record_text(&mut seen, "prior", true);
    record_text(&mut seen, &path, true);
    record_text(&mut seen, &same, false);
    record_text(&mut seen, &prefix, true);
    record_text(&mut seen, "", true);
    record_text(&mut seen, "", false);
    record_text(&mut seen, "prior", false);
}
