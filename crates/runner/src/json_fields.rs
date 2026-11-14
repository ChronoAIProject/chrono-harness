//! JSON 字符串字段读取；生产适配器和严格预期直接消费同一程序。
use crate::json_value_specs as json;
use serde_json::Value;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

pub fn field<'a>(value: &'a Value, key: &str) -> (out: Option<&'a Value>)
    ensures out == json::field(value, key@),
{
    match value.as_object() {
        None => None,
        Some(map) => {
            proof { json::str_lookup(map, key); }
            map.get(key)
        },
    }
}

pub fn field_text<'a>(value: &'a Value, key: &str) -> (out: Option<&'a str>)
    ensures out == json::field_text(value, key@),
{
    match field(value, key) {
        None => None,
        Some(member) => member.as_str(),
    }
}

} // verus!
