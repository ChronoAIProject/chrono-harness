//! Bootstrap 工具绑定判断；文件观察和解码归调用方。
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "../../../../crates/runner/src/verified_identifiers.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;
use identifiers::equal_text;

verus! {

pub enum DeclarationSource { Registry, Direct, Unbound, Reject }

pub fn declaration_source(
    registry: bool, direct: bool, native: bool, required: bool, native_matches: bool,
) -> (out: DeclarationSource)
    ensures
        (out is Registry) <==> (registry && !direct && !native),
        (out is Direct) <==> (!registry && direct
            && (!required || (native && native_matches))),
        (out is Unbound) <==> (!registry && !direct && !required),
        (out is Reject) <==> ((registry && (direct || native))
            || (!registry && required && (!direct || !native || !native_matches))),
{
    if registry {
        if direct || native { DeclarationSource::Reject }
        else { DeclarationSource::Registry }
    } else if required && (!direct || !native || !native_matches) {
        DeclarationSource::Reject
    } else if direct { DeclarationSource::Direct }
    else { DeclarationSource::Unbound }
}

pub open spec fn hash_byte(b: u8) -> bool {
    (48 <= b <= 57) || (97 <= b <= 102)
}

pub fn valid_sha256(bytes: &[u8]) -> (valid: bool)
    ensures valid <==> (bytes@.len() == 64
        && forall|i: int| 0 <= i < bytes@.len() ==> hash_byte(bytes@[i])),
{
    if bytes.len() != 64 { return false; }
    let mut i = 0;
    while i < bytes.len()
        invariant
            bytes.len() == 64,
            0 <= i <= bytes.len(),
            forall|j: int| 0 <= j < i ==> hash_byte(bytes@[j]),
        decreases bytes.len() - i,
    {
        let b = bytes[i];
        if !((48 <= b && b <= 57) || (97 <= b && b <= 102)) { return false; }
        i += 1;
    }
    true
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Deserialize, serde::Serialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct Executable {
    pub path: String,
    pub sha256: String,
    pub version: String,
}

pub open spec fn same_payload(a: &Executable, b: &Executable) -> bool {
    a.sha256@ == b.sha256@ && a.version@ == b.version@
}

pub fn payload_equal(actual: &Executable, expected: &Executable) -> (equal: bool)
    ensures equal <==> same_payload(actual, expected),
{
    equal_text(actual.sha256.as_str(), expected.sha256.as_str())
        && equal_text(actual.version.as_str(), expected.version.as_str())
}

// expected.path 由调用方按宿主根解析；可重定位只放开路径，不放开字节和版本。
pub fn selected_matches(
    selected: &Executable, expected: &Executable, relocatable: bool,
) -> (valid: bool)
    ensures valid <==> (same_payload(selected, expected)
        && (relocatable || selected.path@ == expected.path@)),
{
    payload_equal(selected, expected)
        && (relocatable || equal_text(selected.path.as_str(), expected.path.as_str()))
}

pub enum ConfiguredAction { Observe, Retired, Missing, NotDeclared }

pub fn configured_action(
    configured: Option<&str>, declared_path: Option<&str>, relocatable: bool,
    registry: bool,
) -> (out: ConfiguredAction)
    ensures
        (out is Retired) <==> (configured is Some && declared_path is Some
            && relocatable && configured.unwrap()@ == declared_path.unwrap()@),
        (out is Observe) <==> (configured is Some
            && !(declared_path is Some && relocatable
                && configured.unwrap()@ == declared_path.unwrap()@)),
        (out is Missing) <==> (configured is None && !registry),
        (out is NotDeclared) <==> (configured is None && registry),
{
    match configured {
        None => {
            if registry { ConfiguredAction::NotDeclared }
            else { ConfiguredAction::Missing }
        },
        Some(actual) => {
            match declared_path {
                Some(expected) => {
                    if relocatable && equal_text(actual, expected) {
                        return ConfiguredAction::Retired;
                    }
                },
                None => {},
            }
            ConfiguredAction::Observe
        },
    }
}

// 调用方先按现行合同解析 selected／非空 inherited 的物理路径；空值原样保留。
pub fn compiler_selection(
    selected: &str, inherited: Option<&str>, wrapper: Option<&str>,
    workspace_wrapper: Option<&str>,
) -> (valid: bool)
    ensures valid <==> (
        (inherited is None || inherited.unwrap()@ == ""@
            || inherited.unwrap()@ == selected@)
        && (wrapper is None || wrapper.unwrap()@ == ""@)
        && (workspace_wrapper is None || workspace_wrapper.unwrap()@ == ""@)),
{
    match inherited {
        Some(path) => {
            if !equal_text(path, "") && !equal_text(path, selected) { return false; }
        },
        None => {},
    }
    match wrapper {
        Some(value) => { if !equal_text(value, "") { return false; } },
        None => {},
    }
    match workspace_wrapper {
        Some(value) => { if !equal_text(value, "") { return false; } },
        None => {},
    }
    true
}

}
