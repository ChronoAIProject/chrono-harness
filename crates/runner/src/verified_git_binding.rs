//! Git 事实读取的声明与可选预期；实际文件／进程观察归适配器。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
#[path = "verified_identifiers.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use crate::verified_identifiers as identifiers;

verus! {

pub fn valid_link(schema: u64, tool: bool, input: bool, unknown: bool) -> (valid: bool)
    ensures valid <==> ((schema == 3 || schema == 4) && tool && !unknown
        && (schema == 4 || input)),
{
    (schema == 3 || schema == 4) && tool && !unknown && (schema == 4 || input)
}

pub fn valid_version(schema: u64, present: bool, null: bool, text: Option<&str>) -> (valid: bool)
    ensures valid <==> (present && match text {
        Some(value) => value@.len() > 0,
        None => schema == 4 && null,
    }),
{
    present && match text {
        Some(value) => !value.is_empty(),
        None => schema == 4 && null,
    }
}

/// 无预期时保留真实观察；给出预期时必须完整相等。
pub fn matches_expected(actual: &str, expected: Option<&str>) -> (valid: bool)
    ensures valid <==> match expected {
        None => true,
        Some(value) => actual@ == value@,
    },
{
    match expected {
        None => true,
        Some(value) => identifiers::equal_text(actual, value),
    }
}

}
