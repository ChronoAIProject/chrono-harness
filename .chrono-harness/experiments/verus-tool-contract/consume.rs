//! 工具合同的严格使用实验；不提供 runner 或 CI generation 的证明资格。
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use vstd::json_value_specs as json;
use vstd::prelude::*;

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
    proof { json::str_lookup(value, key); }
    value.get(key)
}

// 实际调用泛型 Index，而不是只导入未用的代理。
pub fn observe_index<'a>(value: &'a Value, key: &str) -> (out: Option<&'a str>)
    ensures out == json::field_text(value, key@),
{
    proof { json::str_lookup(value, key); }
    value[key].as_str()
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
    proof {
        json::standard_string_key_order();
        json::string_map_borrow(map, key);
    }
    map.contains_key(key)
}

// 本实验自己证明插入的结果；没有把状态更新当成工具假设。
pub fn fulltext_insert<'a>(set: &mut BTreeSet<&'a str>, text: &'a str) -> (fresh: bool)
    ensures
        set@ == old(set)@.insert(text),
        fresh == !old(set)@.map(|stored: &str| stored@).contains(text@),
{
    proof {
        json::standard_str_key_order();
        json::str_set_membership(set, text);
    }
    set.insert(text)
}

// 全文相等适用于不同存储及任意 Unicode、NUL、换行；不采用前缀身份。
pub proof fn same_text_membership<'a>(set: &BTreeSet<&'a str>, left: &'a str, right: &'a str)
    requires left@ == right@,
    ensures set@.contains(left) == set@.contains(right),
{
    json::str_set_membership(set, left);
    json::str_set_membership(set, right);
}

} // verus!
