//! 发布入口的实际字段判断。适配器只供应外部解码的原值，不供应预先算好的通过标志。
use super::identifiers;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

verus! {

pub open spec fn text_pair(pair: (Option<&str>, &str)) -> bool {
    match pair.0 { Some(actual) => actual@ == pair.1@, None => false }
}

/// 每个输入保留缺失／错型观察；缺字段不能与空文本混同。
pub fn text_fields(fields: &[(Option<&str>, &str)]) -> (valid: bool)
    ensures valid <==> forall|i: int| 0 <= i < fields.len() ==> text_pair(fields@[i]),
{
    let mut i = 0;
    while i < fields.len()
        invariant i <= fields.len(),
            forall|j: int| 0 <= j < i ==> text_pair(fields@[j]),
        decreases fields.len() - i,
    {
        match fields[i].0 {
            Some(actual) => {
                if !identifiers::equal_text(actual, fields[i].1) { return false; }
            },
            None => { return false; },
        }
        i += 1;
    }
    true
}

pub fn number_fields(fields: &[(Option<u64>, u64)]) -> (valid: bool)
    ensures valid <==> forall|i: int| 0 <= i < fields.len() ==> fields@[i].0 == Some(fields@[i].1),
{
    let mut i = 0;
    while i < fields.len()
        invariant i <= fields.len(),
            forall|j: int| 0 <= j < i ==> fields@[j].0 == Some(fields@[j].1),
        decreases fields.len() - i,
    {
        if fields[i].0 != Some(fields[i].1) { return false; }
        i += 1;
    }
    true
}

pub fn boolean_fields(fields: &[(Option<bool>, bool)]) -> (valid: bool)
    ensures valid <==> forall|i: int| 0 <= i < fields.len() ==> fields@[i].0 == Some(fields@[i].1),
{
    let mut i = 0;
    while i < fields.len()
        invariant i <= fields.len(),
            forall|j: int| 0 <= j < i ==> fields@[j].0 == Some(fields@[j].1),
        decreases fields.len() - i,
    {
        if fields[i].0 != Some(fields[i].1) { return false; }
        i += 1;
    }
    true
}

/// 0=成功，1=允许缺失的原生 404，2=失败。失败不能因 absent 标志变成成功。
pub fn api_outcome(exit: i32, status: Option<&str>, message: Option<&str>, absent: bool) -> (out: u8)
    ensures out == if exit == 0 { 0u8 }
        else if exit == 1 && absent && text_pair((status,"404")) && text_pair((message,"Not Found")) { 1u8 }
        else { 2u8 },
{
    if exit == 0 { 0 }
    else if exit == 1 && absent && text_fields(&[(status,"404"),(message,"Not Found")]) { 1 }
    else { 2 }
}

pub open spec fn decimal(bytes: Seq<u8>, zero: bool) -> bool {
    bytes.len() > 0
        && (bytes[0] != 48 || (zero && bytes.len() == 1))
        && forall|i: int| 0 <= i < bytes.len() ==> 48 <= #[trigger] bytes[i] <= 57
}

pub fn decimal_text(bytes: &[u8], zero: bool) -> (valid: bool)
    ensures valid == decimal(bytes@, zero),
{
    if bytes.len() == 0 || (bytes[0] == 48 && (!zero || bytes.len() != 1)) { return false; }
    let mut i = 0;
    while i < bytes.len()
        invariant i <= bytes.len(), bytes.len() > 0,
            bytes[0] != 48 || (zero && bytes.len() == 1),
            forall|j: int| 0 <= j < i ==> 48 <= #[trigger] bytes[j] <= 57,
        decreases bytes.len() - i,
    {
        if bytes[i] < 48 || bytes[i] > 57 { return false; }
        i += 1;
    }
    true
}

/// 标准 split 只解码原分量；三段和规范十进制的要求由这里拥有。
pub fn version_numbers(parts: &[&str]) -> (valid: bool)
    ensures valid == (parts.len() == 3
        && forall|i: int| 0 <= i < parts.len() ==> decimal(parts@[i].spec_bytes(), true)),
{
    if parts.len() != 3 { return false; }
    let mut i = 0;
    while i < parts.len()
        invariant parts.len() == 3, i <= parts.len(),
            forall|j: int| 0 <= j < i ==> decimal(parts@[j].spec_bytes(), true),
        decreases parts.len() - i,
    {
        if !decimal_text(parts[i].as_bytes(), true) { return false; }
        i += 1;
    }
    true
}

/// channel 的数字仅为适配器对既有枚举的无状态投影；3 表示拒绝。
pub fn channel(text: &str) -> (out: u8)
    ensures out == if text@ == ""@ || text@ == "build"@ { 0u8 }
        else if text@ == "preview"@ { 1u8 }
        else if text@ == "stable"@ { 2u8 }
        else { 3u8 },
{
    if identifiers::equal_text(text, "") || identifiers::equal_text(text, "build") { 0 }
    else if identifiers::equal_text(text, "preview") { 1 }
    else if identifiers::equal_text(text, "stable") { 2 }
    else { 3 }
}

pub fn requested_version(channel: u8, requested: &str, core: &str, planned: &str) -> (valid: bool)
    ensures valid == (if channel == 1 { requested@.len() == 0 }
        else if channel == 2 { core@ == planned@ && requested@ == planned@ }
        else { false }),
{
    if channel == 1 { requested.is_empty() }
    else if channel == 2 {
        identifiers::equal_text(core, planned) && identifiers::equal_text(requested, planned)
    } else { false }
}

pub open spec fn name_byte(byte: u8) -> bool {
    (65 <= byte <= 90) || (97 <= byte <= 122) || (48 <= byte <= 57)
        || byte == 46 || byte == 95 || byte == 45
}

pub fn name(bytes: &[u8]) -> (valid: bool)
    ensures valid == (bytes.len() > 0 && bytes@ != seq![46u8] && bytes@ != seq![46u8,46]
        && forall|i: int| 0 <= i < bytes.len() ==> name_byte(#[trigger] bytes[i])),
{
    if bytes.len() == 0 || identifiers::equal_bytes(bytes, &[46u8])
        || identifiers::equal_bytes(bytes, &[46u8,46]) { return false; }
    let mut i = 0;
    while i < bytes.len()
        invariant i <= bytes.len(), bytes.len() > 0,
            bytes@ != seq![46u8], bytes@ != seq![46u8,46],
            forall|j: int| 0 <= j < i ==> name_byte(#[trigger] bytes[j]),
        decreases bytes.len() - i,
    {
        let b = bytes[i];
        if !((65 <= b && b <= 90) || (97 <= b && b <= 122) || (48 <= b && b <= 57)
            || b == 46 || b == 95 || b == 45) { return false; }
        i += 1;
    }
    true
}

pub fn positive(value: Option<u64>) -> (valid: bool)
    ensures valid == (value is Some && value.unwrap() > 0),
{
    match value { Some(value) => value > 0, None => false }
}

pub fn draft_state(draft: Option<bool>, missing: usize) -> (action: u8)
    ensures action == match draft {
        Some(true) => 1u8,
        Some(false) if missing == 0 => 0u8,
        _ => 2u8,
    },
{
    match draft { Some(true) => 1, Some(false) if missing == 0 => 0, _ => 2 }
}

}
