//! CI 配置的字面单行判断及原生缓存登记准入，供现役投影使用。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "../../runner/src/verified_identifiers.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;

verus! {

pub open spec fn cache_registry(schema: Option<&str>, primary_checkout: bool) -> bool {
    match schema {
        Some(schema) => schema@ == "chrono-cache/v2"@ && primary_checkout,
        None => false,
    }
}

pub fn valid_cache_registry(schema: Option<&str>, primary_checkout: bool) -> (valid: bool)
    ensures valid <==> cache_registry(schema, primary_checkout),
{
    match schema {
        Some(schema) => identifiers::equal_text(schema, "chrono-cache/v2") && primary_checkout,
        None => false,
    }
}

#[verifier::inline]
pub open spec fn literal_byte(byte: u8) -> bool {
    byte != 0 && byte != 10 && byte != 13
}

#[verifier::inline]
pub open spec fn template_at(bytes: Seq<u8>, i: int) -> bool {
    0 <= i && i + 2 < bytes.len()
        && bytes[i] == 36
        && bytes[i + 1] == 123
        && bytes[i + 2] == 123
}

#[verifier::inline]
pub open spec fn literal(bytes: Seq<u8>) -> bool {
    forall|i: int| 0 <= i < bytes.len() ==> (
        literal_byte(#[trigger] bytes[i]) && !template_at(bytes, i)
    )
}

pub fn valid_literal(bytes: &[u8]) -> (valid: bool)
    ensures valid == literal(bytes@),
{
    let mut i = 0;
    while i < bytes.len()
        invariant
            0 <= i <= bytes.len(),
            forall|j: int| 0 <= j < i ==> (
                literal_byte(#[trigger] bytes[j]) && !template_at(bytes@, j)
            ),
        decreases bytes.len() - i,
    {
        let byte = bytes[i];
        if byte == 0 || byte == 10 || byte == 13 {
            return false;
        }
        if byte == 36 && bytes.len() >= 3 && i <= bytes.len() - 3
            && bytes[i + 1] == 123 && bytes[i + 2] == 123
        {
            return false;
        }
        i += 1;
    }
    true
}

}
