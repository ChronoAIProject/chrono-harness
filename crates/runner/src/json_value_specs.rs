//! 借用 JSON 和完整文本树集合的外部接口合同；不包含仓内选择或状态逻辑。
#![allow(unexpected_cfgs)]

#[cfg(verus_keep_ghost)]
use serde_json::{Map as JsonMap, Value};
#[cfg(verus_keep_ghost)]
use std::collections::{BTreeMap, BTreeSet};
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

// 只声明外部类型。适配器及其投影函数均不在受信边界内。
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExValue(Value);

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::accept_recursive_types(K)]
#[verifier::accept_recursive_types(V)]
pub struct ExJsonMap<K, V>(JsonMap<K, V>);

#[verifier::external_trait_specification]
pub trait ExJsonIndex {
    type ExternalTraitSpecificationFor: serde_json::value::Index;
}

/// 这些观察属于同一个不可变原值；文本借用保留全部 Unicode 字符。
pub uninterp spec fn text(value: &Value) -> Option<&str>;
pub uninterp spec fn object(value: &Value) -> Option<&JsonMap<String, Value>>;
pub uninterp spec fn members(map: &JsonMap<String, Value>) -> Map<Seq<char>, &Value>;

pub open spec fn field<'a>(value: &'a Value, key: Seq<char>) -> Option<&'a Value> {
    match object(value) {
        None => None,
        Some(map) => if members(map).contains_key(key) { Some(members(map)[key]) } else { None },
    }
}

pub open spec fn field_text<'a>(value: &'a Value, key: Seq<char>) -> Option<&'a str> {
    match field(value, key) { None => None, Some(value) => text(value) }
}

pub open spec fn fields(value: &Value) -> Option<usize> {
    match object(value) { None => None, Some(map) => Some(members(map).len() as usize) }
}

// 保留外部方法本身的泛型，不为同一方法制造单态重复规格。
// 本类只解释 &str 索引；其他 Index 实例的观察保持不透明。
pub uninterp spec fn lookup<'a, I: serde_json::value::Index>(value: &'a Value, index: I)
    -> Option<&'a Value>;

pub broadcast axiom fn str_lookup(value: &Value, key: &str)
    ensures #[trigger] lookup(value, key) == field(value, key@);

// serde_json 1.0.151 value/mod.rs::as_str/as_object/get、value/index.rs::index。
pub assume_specification[ Value::as_str ](value: &Value) -> (out: Option<&str>)
    ensures out == text(value);

pub assume_specification[ Value::as_object ](value: &Value)
    -> (out: Option<&JsonMap<String, Value>>)
    ensures out == object(value);

pub assume_specification<'a, I: serde_json::value::Index>[ Value::get::<I> ](
    value: &'a Value, index: I,
) -> (out: Option<&'a Value>)
    ensures out == lookup(value, index);

pub assume_specification<'a, I: serde_json::value::Index>[ <Value as std::ops::Index<I>>::index ](
    value: &'a Value, index: I,
) -> (out: &'a Value)
    ensures
        match lookup(value, index) {
            Some(original) => out == original,
            // 缺键／非对象的不可变索引返回 Null；本类只使用其文本观察。
            None => text(out).is_none(),
        },
        text(out) == match lookup(value, index) {
            None => None, Some(original) => text(original),
        };

// serde_json 1.0.151 map.rs::len：有限原成员数，不把非对象当作空对象。
pub assume_specification[ JsonMap::<String, Value>::len ](
    map: &JsonMap<String, Value>,
) -> (out: usize)
    ensures members(map).dom().finite(), out == members(map).len();

// 固定 vstd 的通用树集合合同以 obeys_cmp 为前提，且借用键模型未解释
// String -> str。以下仅补固定 Rust 标准文本键的接口模型；不假设仓内状态。
pub broadcast axiom fn standard_string_key_order()
    ensures #[trigger] vstd::laws_cmp::obeys_cmp::<String>();

pub broadcast axiom fn standard_str_key_order()
    ensures #[trigger] vstd::laws_cmp::obeys_cmp::<&str>();

pub broadcast axiom fn string_map_borrow(map: &BTreeMap<String, Value>, key: &str)
    ensures
        #[trigger] vstd::std_specs::btree::contains_borrowed_key(map@, key)
            <==> exists|stored: String| #[trigger] map@.contains_key(stored) && stored@ == key@;

pub broadcast axiom fn str_set_membership<'a>(set: &BTreeSet<&'a str>, text: &'a str)
    ensures
        #[trigger] set@.contains(text)
            <==> set@.map(|stored: &str| stored@).contains(text@);

} // verus!
