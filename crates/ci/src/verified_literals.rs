//! CI 配置的字面单行判断及原生缓存登记准入，供现役投影使用。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;

verus! {

pub fn cache_save_event_guard(push_only: bool) -> (guard: &'static str)
    ensures guard@ == if push_only { "github.event_name == 'push' && "@ } else { ""@ },
{
    if push_only { "github.event_name == 'push' && " } else { "" }
}

pub fn valid_download_config(schema: &str, entries: usize, environment: usize) -> (valid: bool)
    ensures valid == (schema@ == "chrono-github-downloads/v1"@ && entries > 0 && environment > 0),
{
    identifiers::equal_text(schema, "chrono-github-downloads/v1") && entries > 0 && environment > 0
}

pub fn valid_download_environment(key: &str) -> (valid: bool)
    ensures valid == (key@ == "CARGO_HOME"@ || key@ == "RUSTUP_HOME"@),
{
    identifiers::equal_text(key, "CARGO_HOME") || identifiers::equal_text(key, "RUSTUP_HOME")
}

pub open spec fn download_entry(
    paths: nat, compatibility: nat, sources: nat, consumers: nat,
    unique: bool, routed: bool, fallback_compatible: bool, purpose: bool,
) -> bool {
    paths > 0 && compatibility > 0 && sources > 0 && consumers > 0
        && unique && routed && fallback_compatible && purpose
}

pub fn valid_download_entry(
    paths: usize, compatibility: usize, sources: usize, consumers: usize,
    unique: bool, routed: bool, fallback_compatible: bool, purpose: bool,
) -> (valid: bool)
    ensures valid == download_entry(paths as nat, compatibility as nat, sources as nat,
        consumers as nat, unique, routed, fallback_compatible, purpose),
{
    paths > 0 && compatibility > 0 && sources > 0 && consumers > 0
        && unique && routed && fallback_compatible && purpose
}

pub fn single_download_binding(count: usize) -> (valid: bool)
    ensures valid == (count == 1),
{ count == 1 }

pub open spec fn download_artifact(
    path: Option<&str>, expected_path: &str, owner: Option<&str>, expected_owner: &str,
    is_tracked: Option<bool>, kind: Option<&str>,
) -> bool {
    match (path, owner, is_tracked, kind) {
        (Some(path), Some(owner), Some(false), Some(kind)) =>
            path@ == expected_path@ && owner@ == expected_owner@ && kind@ != "execution-evidence"@,
        _ => false,
    }
}

pub fn valid_download_artifact(
    path: Option<&str>, expected_path: &str, owner: Option<&str>, expected_owner: &str,
    is_tracked: Option<bool>, kind: Option<&str>,
) -> (valid: bool)
    ensures valid == download_artifact(path, expected_path, owner, expected_owner, is_tracked, kind),
{
    match (path, owner, is_tracked, kind) {
        (Some(path), Some(owner), Some(false), Some(kind)) =>
            identifiers::equal_text(path, expected_path) && identifiers::equal_text(owner, expected_owner)
                && !identifiers::equal_text(kind, "execution-evidence"),
        _ => false,
    }
}

pub open spec fn download_path_byte(b: u8) -> bool {
    (b >= 65 && b <= 90) || (b >= 97 && b <= 122) || (b >= 48 && b <= 57)
        || b == 45 || b == 95 || b == 46 || b == 47
}

pub fn valid_download_path(bytes: &[u8]) -> (valid: bool)
    ensures valid == (bytes@.len() > 0
        && forall|i: int| 0 <= i < bytes@.len() ==> download_path_byte(#[trigger] bytes@[i])),
{
    if bytes.len() == 0 { return false; }
    let mut i = 0;
    while i < bytes.len()
        invariant bytes.len() > 0, 0 <= i <= bytes.len(),
            forall|j: int| 0 <= j < i ==> download_path_byte(#[trigger] bytes@[j]),
        decreases bytes.len() - i,
    {
        let b = bytes[i];
        if !((b >= 65 && b <= 90) || (b >= 97 && b <= 122) || (b >= 48 && b <= 57)
            || b == 45 || b == 95 || b == 46 || b == 47) { return false; }
        i += 1;
    }
    true
}

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
