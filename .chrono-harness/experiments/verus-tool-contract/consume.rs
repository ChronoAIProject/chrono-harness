//! 工具合同的严格使用实验；不提供 runner 或 CI generation 的证明资格。
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use vstd::json_value_specs;
use vstd::json_value_specs as json;
use vstd::prelude::*;

// 与生产适配器同源；仓内分支不进入工具库的受信边界。
#[path = "../../../crates/runner/src/json_fields.rs"]
mod json_fields;
#[path = "../../../crates/runner/src/verified_collections.rs"]
mod collections;

verus! {

// 没有 JSON 模型前提：实际外部调用必须取得同一值的完整文本观察。
pub fn observe_text(value: &Value) -> (out: Option<&str>)
    ensures out == json::text(value),
{
    value.as_str()
}

pub fn observe_field<'a>(value: &'a Value, key: &str) -> (out: Option<&'a Value>)
    ensures out == json::field(value, key@),
{
    json_fields::field(value, key)
}

// 保留原 Index 的完整文本观察，由生产共用的公开对象查询路径实现。
pub fn observe_index<'a>(value: &'a Value, key: &str) -> (out: Option<&'a str>)
    ensures out == json::field_text(value, key@),
{
    json_fields::field_text(value, key)
}

pub fn observe_object_size(value: &Value) -> (out: Option<usize>)
    ensures out == json::fields(value),
{
    match value.as_object() {
        None => None,
        Some(map) => Some(map.len()),
    }
}

// 独立边界：缺键与存在但无文本的值不同，get 不能把后者变成 None。
pub fn missing_key(value: &Value, key: &str)
    requires json::field(value, key@).is_none(),
{
    let field = observe_field(value, key);
    let text = observe_index(value, key);
    assert(field.is_none());
    assert(text.is_none());
}

pub fn present_nontext(value: &Value, key: &str)
    requires json::field(value, key@).is_some(),
        json::field_text(value, key@).is_none(),
{
    let field = observe_field(value, key);
    let text = observe_index(value, key);
    assert(field.is_some());
    assert(text.is_none());
}

pub fn borrowed_fulltext(map: &BTreeMap<String, Value>, key: &str) -> (out: bool)
    ensures out <==> exists|stored: String|
        #[trigger] map@.contains_key(stored) && stored@ == key@,
{
    broadcast use json::standard_string_key_order;
    broadcast use json::standard_str_key_order;
    collections::contains_string_key(map, key)
}

// 本实验自己证明插入的结果；没有把状态更新当成工具假设。
pub fn fulltext_insert<'a>(set: &mut BTreeSet<&'a str>, text: &'a str) -> (fresh: bool)
    ensures
        final(set).deep_view() == old(set).deep_view().insert(text@),
        fresh == !old(set).deep_view().contains(text@),
        !fresh ==> final(set)@ == old(set)@,
{
    broadcast use json::standard_str_key_order;
    collections::insert_str_key(set, text)
}

// 全文相等适用于不同存储及任意 Unicode、NUL、换行；不采用前缀身份。
pub proof fn same_text_membership<'a>(set: &BTreeSet<&'a str>, left: &'a str, right: &'a str)
    requires left@ == right@,
    ensures set.deep_view().contains(left@) == set.deep_view().contains(right@),
{
}

// 独立预期：不同存储的重复文字不替换已保存键，也不改变任何其他成员。
pub fn duplicate_text_keeps_original_keys<'a>(set: &mut BTreeSet<&'a str>, left: &'a str,
    right: &'a str)
    requires left@ == right@, old(set).deep_view().contains(left@),
    ensures final(set)@ == old(set)@,
{
    let fresh = fulltext_insert(set, right);
    assert(!fresh);
}

} // verus!
